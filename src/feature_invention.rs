//! Executable context-feature invention over a lower-level raw tree substrate.
//!
//! Programs cannot inspect task metadata.  They see only published examples,
//! are evaluated with explicit fuel, and are selected by downstream allocation
//! regret on a nested calibration split.  Feature work remains its own unit.

use crate::{
    contextual_allocation::{ConceptSet, EvidenceDerivation, FrozenPolicy},
    learned_context::{
        evaluate_encoder, freeze_policy as freeze_projected_policy, EncodeError, EncoderKind,
        FrozenContextEncoder, RawField, RawTaskObservation, RawUtilityEvidence, RepresentationSpec,
    },
    search_accounting::{RunAccounting, SearchEngine},
    term::Term,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawNode {
    pub label: i64,
    pub children: Vec<RawNode>,
}

impl RawNode {
    pub fn leaf(label: i64) -> Self {
        Self {
            label,
            children: Vec::new(),
        }
    }

    pub fn branch(label: i64, children: Vec<Self>) -> Self {
        Self { label, children }
    }

    pub fn grid(rows: &[Vec<u32>]) -> Self {
        Self::branch(
            -1,
            rows.iter()
                .map(|row| {
                    Self::branch(
                        -2,
                        row.iter()
                            .map(|value| Self::leaf(i64::from(*value)))
                            .collect(),
                    )
                })
                .collect(),
        )
    }

    pub fn lambda(term: &Term) -> Self {
        match term {
            Term::Var(index) => Self::branch(0, vec![Self::leaf(i64::from(*index))]),
            Term::Lam(body) => Self::branch(1, vec![Self::lambda(body)]),
            Term::App(function, argument) => {
                Self::branch(2, vec![Self::lambda(function), Self::lambda(argument)])
            }
            Term::Free(index) => Self::branch(3, vec![Self::leaf(i64::from(*index))]),
            Term::Prim(body) => Self::branch(4, vec![Self::lambda(body)]),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawExample {
    pub inputs: Vec<RawNode>,
    /// Published training output only. Protected outputs are never represented.
    pub published_output: Option<RawNode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawTask {
    /// Metadata is used only by split/leakage gates, never by feature programs.
    pub task_id: String,
    pub duplicate_group_id: String,
    pub examples: Vec<RawExample>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Primitive {
    ExampleCount,
    InputNodeCount,
    InputHeight,
    InputMaxArity,
    InputDistinctLabels,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnaryOp {
    Mod2,
    IsZero,
    Abs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BinaryOp {
    Add,
    Subtract,
    Equal,
    Greater,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransformProgram {
    Identity,
    ReverseChildren,
    MapChildren(Box<TransformProgram>),
    Compose(Box<TransformProgram>, Box<TransformProgram>),
}

impl TransformProgram {
    pub fn size(&self) -> u32 {
        match self {
            Self::Identity | Self::ReverseChildren => 1,
            Self::MapChildren(inner) => 1 + inner.size(),
            Self::Compose(first, second) => 1 + first.size() + second.size(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FeatureProgram {
    Primitive(Primitive),
    Constant(i64),
    Unary(UnaryOp, Box<FeatureProgram>),
    Binary(BinaryOp, Box<FeatureProgram>, Box<FeatureProgram>),
    /// True iff one generic tree transformation maps every first input to its
    /// published output. No transformation is named by domain semantics.
    Relation(TransformProgram),
    /// Adversarial controls. These are never emitted by the production grammar.
    Diverge,
    Partial,
    Unstable,
    Noise,
    Shuffled,
    IdentityProbe,
    ProtectedOutputProbe,
}

impl FeatureProgram {
    pub fn size(&self) -> u32 {
        match self {
            Self::Primitive(_) | Self::Constant(_) => 1,
            Self::Unary(_, inner) => 1 + inner.size(),
            Self::Binary(_, left, right) => 1 + left.size() + right.size(),
            Self::Relation(transform) => 1 + transform.size(),
            Self::Diverge
            | Self::Partial
            | Self::Unstable
            | Self::Noise
            | Self::Shuffled
            | Self::IdentityProbe
            | Self::ProtectedOutputProbe => 1,
        }
    }

    pub fn is_compositional(&self) -> bool {
        self.size() >= 2
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FeatureError {
    OutOfFuel,
    MissingInput,
    MissingPublishedOutput,
    Partial,
    Unstable,
    Nondeterministic,
    Shuffled,
    MetadataAccess,
    ProtectedOutput,
    ArithmeticOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureExecution {
    pub value: i64,
    pub steps: u64,
}

#[derive(Clone, Debug)]
struct Fuel {
    remaining: u64,
    initial: u64,
}

impl Fuel {
    fn new(amount: u64) -> Self {
        Self {
            remaining: amount,
            initial: amount,
        }
    }

    fn spend(&mut self) -> Result<(), FeatureError> {
        if self.remaining == 0 {
            Err(FeatureError::OutOfFuel)
        } else {
            self.remaining -= 1;
            Ok(())
        }
    }

    fn used(&self) -> u64 {
        self.initial - self.remaining
    }
}

fn node_count(node: &RawNode, fuel: &mut Fuel) -> Result<i64, FeatureError> {
    fuel.spend()?;
    node.children.iter().try_fold(1i64, |total, child| {
        total
            .checked_add(node_count(child, fuel)?)
            .ok_or(FeatureError::ArithmeticOverflow)
    })
}

fn node_height(node: &RawNode, fuel: &mut Fuel) -> Result<i64, FeatureError> {
    fuel.spend()?;
    let child = node
        .children
        .iter()
        .map(|child| node_height(child, fuel))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .unwrap_or(0);
    child.checked_add(1).ok_or(FeatureError::ArithmeticOverflow)
}

fn max_arity(node: &RawNode, fuel: &mut Fuel) -> Result<i64, FeatureError> {
    fuel.spend()?;
    let child = node
        .children
        .iter()
        .map(|child| max_arity(child, fuel))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .unwrap_or(0);
    Ok(child.max(node.children.len() as i64))
}

fn collect_labels(
    node: &RawNode,
    labels: &mut BTreeSet<i64>,
    fuel: &mut Fuel,
) -> Result<(), FeatureError> {
    fuel.spend()?;
    labels.insert(node.label);
    for child in &node.children {
        collect_labels(child, labels, fuel)?;
    }
    Ok(())
}

fn transform_node(
    transform: &TransformProgram,
    node: &RawNode,
    fuel: &mut Fuel,
) -> Result<RawNode, FeatureError> {
    fuel.spend()?;
    match transform {
        TransformProgram::Identity => Ok(node.clone()),
        TransformProgram::ReverseChildren => {
            let mut result = node.clone();
            result.children.reverse();
            Ok(result)
        }
        TransformProgram::MapChildren(inner) => Ok(RawNode {
            label: node.label,
            children: node
                .children
                .iter()
                .map(|child| transform_node(inner, child, fuel))
                .collect::<Result<Vec<_>, _>>()?,
        }),
        TransformProgram::Compose(first, second) => {
            let intermediate = transform_node(first, node, fuel)?;
            transform_node(second, &intermediate, fuel)
        }
    }
}

fn primitive_value(
    primitive: Primitive,
    task: &RawTask,
    fuel: &mut Fuel,
) -> Result<i64, FeatureError> {
    fuel.spend()?;
    match primitive {
        Primitive::ExampleCount => Ok(task.examples.len() as i64),
        Primitive::InputNodeCount => task.examples.iter().try_fold(0i64, |total, example| {
            let input = example.inputs.first().ok_or(FeatureError::MissingInput)?;
            total
                .checked_add(node_count(input, fuel)?)
                .ok_or(FeatureError::ArithmeticOverflow)
        }),
        Primitive::InputHeight => task.examples.iter().try_fold(0i64, |maximum, example| {
            let input = example.inputs.first().ok_or(FeatureError::MissingInput)?;
            Ok(maximum.max(node_height(input, fuel)?))
        }),
        Primitive::InputMaxArity => task.examples.iter().try_fold(0i64, |maximum, example| {
            let input = example.inputs.first().ok_or(FeatureError::MissingInput)?;
            Ok(maximum.max(max_arity(input, fuel)?))
        }),
        Primitive::InputDistinctLabels => {
            let mut labels = BTreeSet::new();
            for example in &task.examples {
                let input = example.inputs.first().ok_or(FeatureError::MissingInput)?;
                collect_labels(input, &mut labels, fuel)?;
            }
            Ok(labels.len() as i64)
        }
    }
}

fn eval_inner(
    program: &FeatureProgram,
    task: &RawTask,
    fuel: &mut Fuel,
) -> Result<i64, FeatureError> {
    fuel.spend()?;
    match program {
        FeatureProgram::Primitive(primitive) => primitive_value(*primitive, task, fuel),
        FeatureProgram::Constant(value) => Ok(*value),
        FeatureProgram::Unary(operator, inner) => {
            let value = eval_inner(inner, task, fuel)?;
            match operator {
                UnaryOp::Mod2 => Ok(value.rem_euclid(2)),
                UnaryOp::IsZero => Ok(i64::from(value == 0)),
                UnaryOp::Abs => value.checked_abs().ok_or(FeatureError::ArithmeticOverflow),
            }
        }
        FeatureProgram::Binary(operator, left, right) => {
            let left = eval_inner(left, task, fuel)?;
            let right = eval_inner(right, task, fuel)?;
            match operator {
                BinaryOp::Add => left
                    .checked_add(right)
                    .ok_or(FeatureError::ArithmeticOverflow),
                BinaryOp::Subtract => left
                    .checked_sub(right)
                    .ok_or(FeatureError::ArithmeticOverflow),
                BinaryOp::Equal => Ok(i64::from(left == right)),
                BinaryOp::Greater => Ok(i64::from(left > right)),
            }
        }
        FeatureProgram::Relation(transform) => {
            for example in &task.examples {
                let input = example.inputs.first().ok_or(FeatureError::MissingInput)?;
                let output = example
                    .published_output
                    .as_ref()
                    .ok_or(FeatureError::MissingPublishedOutput)?;
                if transform_node(transform, input, fuel)? != *output {
                    return Ok(0);
                }
            }
            Ok(1)
        }
        FeatureProgram::Diverge => loop {
            fuel.spend()?;
        },
        FeatureProgram::Partial => Err(FeatureError::Partial),
        FeatureProgram::Unstable => Err(FeatureError::Unstable),
        FeatureProgram::Noise => Err(FeatureError::Nondeterministic),
        FeatureProgram::Shuffled => Err(FeatureError::Shuffled),
        FeatureProgram::IdentityProbe => Err(FeatureError::MetadataAccess),
        FeatureProgram::ProtectedOutputProbe => Err(FeatureError::ProtectedOutput),
    }
}

pub fn execute_feature(
    program: &FeatureProgram,
    task: &RawTask,
    fuel: u64,
) -> Result<FeatureExecution, FeatureError> {
    let mut fuel = Fuel::new(fuel);
    let value = eval_inner(program, task, &mut fuel)?;
    Ok(FeatureExecution {
        value,
        steps: fuel.used(),
    })
}

fn transform_programs(max_size: u32) -> BTreeMap<u32, Vec<TransformProgram>> {
    let mut exact = BTreeMap::<u32, Vec<TransformProgram>>::new();
    exact.insert(
        1,
        vec![
            TransformProgram::Identity,
            TransformProgram::ReverseChildren,
        ],
    );
    for size in 2..=max_size {
        let mut values = BTreeSet::new();
        if let Some(inner) = exact.get(&(size - 1)) {
            values.extend(
                inner
                    .iter()
                    .cloned()
                    .map(|value| TransformProgram::MapChildren(Box::new(value))),
            );
        }
        for left_size in 1..size.saturating_sub(1) {
            let right_size = size - 1 - left_size;
            if let (Some(left), Some(right)) = (exact.get(&left_size), exact.get(&right_size)) {
                for first in left {
                    for second in right {
                        values.insert(TransformProgram::Compose(
                            Box::new(first.clone()),
                            Box::new(second.clone()),
                        ));
                    }
                }
            }
        }
        exact.insert(size, values.into_iter().collect());
    }
    exact
}

/// Finite exact-size enumeration. Increasing `max_size` is a fair structural
/// widening; controls with metadata, nondeterminism, or divergence are absent.
pub fn enumerate_features(max_size: u32, max_programs: usize) -> Vec<FeatureProgram> {
    let transforms = transform_programs(max_size.saturating_sub(1));
    let mut exact = BTreeMap::<u32, Vec<FeatureProgram>>::new();
    exact.insert(
        1,
        Primitive::all()
            .into_iter()
            .map(FeatureProgram::Primitive)
            .chain([FeatureProgram::Constant(0), FeatureProgram::Constant(1)])
            .collect(),
    );
    let mut result = Vec::new();
    for size in 1..=max_size {
        if size > 1 {
            let mut values = BTreeSet::new();
            if let Some(inner) = exact.get(&(size - 1)) {
                for program in inner {
                    for operator in [UnaryOp::Mod2, UnaryOp::IsZero, UnaryOp::Abs] {
                        values.insert(FeatureProgram::Unary(operator, Box::new(program.clone())));
                    }
                }
            }
            for left_size in 1..size.saturating_sub(1) {
                let right_size = size - 1 - left_size;
                if let (Some(left), Some(right)) = (exact.get(&left_size), exact.get(&right_size)) {
                    for first in left {
                        for second in right {
                            for operator in [
                                BinaryOp::Add,
                                BinaryOp::Subtract,
                                BinaryOp::Equal,
                                BinaryOp::Greater,
                            ] {
                                values.insert(FeatureProgram::Binary(
                                    operator,
                                    Box::new(first.clone()),
                                    Box::new(second.clone()),
                                ));
                            }
                        }
                    }
                }
            }
            if let Some(exact_transforms) = transforms.get(&(size - 1)) {
                values.extend(
                    exact_transforms
                        .iter()
                        .cloned()
                        .map(FeatureProgram::Relation),
                );
            }
            exact.insert(size, values.into_iter().collect());
        }
        if let Some(programs) = exact.get(&size) {
            for program in programs {
                if result.len() == max_programs {
                    return result;
                }
                result.push(program.clone());
            }
        }
    }
    result
}

impl Primitive {
    fn all() -> [Self; 5] {
        [
            Self::ExampleCount,
            Self::InputNodeCount,
            Self::InputHeight,
            Self::InputMaxArity,
            Self::InputDistinctLabels,
        ]
    }
}

#[derive(Clone, Debug)]
pub struct FeatureUtilityEvidence {
    pub task: RawTask,
    pub concept_ids: Vec<String>,
    pub without: RunAccounting,
    pub with: RunAccounting,
    pub age: u32,
    pub recorded_epoch: u64,
    pub derivation: EvidenceDerivation,
}

#[derive(Clone, Debug)]
pub struct FeatureSelectionSpec {
    pub engine: SearchEngine,
    pub freeze_epoch: u64,
    pub decay_per_mille: u16,
    pub interactions: bool,
    pub max_interaction_width: usize,
    pub max_program_size: u32,
    pub max_programs: usize,
    pub max_feature_width: usize,
    pub feature_pool_limit: usize,
    pub execution_fuel: u64,
    pub complexity_cost: u64,
    pub execution_cost: u64,
}

impl FeatureSelectionSpec {
    fn representation_spec(&self, width: usize) -> RepresentationSpec {
        RepresentationSpec {
            engine: self.engine,
            freeze_epoch: self.freeze_epoch,
            decay_per_mille: self.decay_per_mille,
            interactions: self.interactions,
            max_interaction_width: self.max_interaction_width,
            max_projection_width: width.max(1),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FeatureAccounting {
    pub programs_enumerated: u64,
    pub feature_sets_evaluated: u64,
    pub task_executions: u64,
    pub execution_steps: u64,
    pub rejected_programs: u64,
    pub max_program_size: u32,
    pub execution_fuel: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureSetEvaluation {
    pub programs: Vec<FeatureProgram>,
    pub regret: u64,
    pub charged_objective: u64,
    pub execution_steps: u64,
    pub rejected: Option<FeatureError>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrozenFeatureEncoder {
    pub programs: Vec<FeatureProgram>,
    pub freeze_epoch: u64,
    pub calibration_regret: u64,
    pub collapsed_regret: u64,
    pub charged_objective: u64,
    pub collapsed_objective: u64,
    pub retained: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InventedFeatureRepresentation {
    pub encoder: FrozenFeatureEncoder,
    pub evaluations: Vec<FeatureSetEvaluation>,
    pub accounting: FeatureAccounting,
    pub primitive_projection_regret: u64,
}

fn feature_name(index: usize) -> String {
    format!("phi-{index}")
}

fn materialize_observation(
    encoder: &[FeatureProgram],
    task: &RawTask,
    spec: &FeatureSelectionSpec,
) -> Result<(RawTaskObservation, u64), FeatureError> {
    let mut fields = BTreeMap::new();
    let mut steps = 0u64;
    for (index, program) in encoder.iter().enumerate() {
        let execution = execute_feature(program, task, spec.execution_fuel)?;
        steps = steps.saturating_add(execution.steps);
        fields.insert(
            feature_name(index),
            RawField::observable(execution.value, spec.freeze_epoch),
        );
    }
    Ok((
        RawTaskObservation {
            task_id: task.task_id.clone(),
            duplicate_group_id: task.duplicate_group_id.clone(),
            fields,
        },
        steps,
    ))
}

fn materialize_evidence(
    programs: &[FeatureProgram],
    records: &[FeatureUtilityEvidence],
    spec: &FeatureSelectionSpec,
) -> Result<(Vec<RawUtilityEvidence>, u64), FeatureError> {
    let mut result = Vec::new();
    let mut steps = 0u64;
    for record in records {
        let (observation, used) = materialize_observation(programs, &record.task, spec)?;
        steps = steps.saturating_add(used);
        result.push(RawUtilityEvidence {
            observation,
            concept_ids: record.concept_ids.clone(),
            without: record.without.clone(),
            with: record.with.clone(),
            age: record.age,
            recorded_epoch: record.recorded_epoch,
            derivation: record.derivation.clone(),
        });
    }
    Ok((result, steps))
}

fn evaluate_feature_set(
    programs: Vec<FeatureProgram>,
    training: &[FeatureUtilityEvidence],
    calibration: &[FeatureUtilityEvidence],
    concepts: &[ConceptSet],
    spec: &FeatureSelectionSpec,
) -> FeatureSetEvaluation {
    let materialized = materialize_evidence(&programs, training, spec).and_then(|(train, a)| {
        materialize_evidence(&programs, calibration, spec)
            .map(|(calibration, b)| (train, calibration, a.saturating_add(b)))
    });
    let (training, calibration, execution_steps) = match materialized {
        Ok(value) => value,
        Err(error) => {
            return FeatureSetEvaluation {
                programs,
                regret: u64::MAX,
                charged_objective: u64::MAX,
                execution_steps: 0,
                rejected: Some(error),
            }
        }
    };
    let names = (0..programs.len()).map(feature_name).collect::<Vec<_>>();
    let evaluation = evaluate_encoder(
        if names.is_empty() {
            EncoderKind::Collapsed
        } else {
            EncoderKind::Projection(names)
        },
        &training,
        &calibration,
        concepts,
        &spec.representation_spec(programs.len()),
    );
    let complexity = programs
        .iter()
        .map(FeatureProgram::size)
        .map(u64::from)
        .sum::<u64>();
    let charged_objective = evaluation
        .regret
        .saturating_mul(1_000_000)
        .saturating_add(complexity.saturating_mul(spec.complexity_cost))
        .saturating_add(execution_steps.saturating_mul(spec.execution_cost));
    FeatureSetEvaluation {
        programs,
        regret: evaluation.regret,
        charged_objective,
        execution_steps,
        rejected: evaluation.rejected.map(|_| FeatureError::Partial),
    }
}

fn combinations<T: Clone>(items: &[T], width: usize) -> Vec<Vec<T>> {
    fn visit<T: Clone>(
        items: &[T],
        start: usize,
        remaining: usize,
        current: &mut Vec<T>,
        result: &mut Vec<Vec<T>>,
    ) {
        if remaining == 0 {
            result.push(current.clone());
            return;
        }
        for index in start..=items.len().saturating_sub(remaining) {
            current.push(items[index].clone());
            visit(items, index + 1, remaining - 1, current, result);
            current.pop();
        }
    }
    let mut result = Vec::new();
    visit(items, 0, width, &mut Vec::new(), &mut result);
    result
}

pub fn invent_features(
    training: &[FeatureUtilityEvidence],
    calibration: &[FeatureUtilityEvidence],
    concepts: &[ConceptSet],
    spec: &FeatureSelectionSpec,
) -> InventedFeatureRepresentation {
    let safe = |record: &&FeatureUtilityEvidence| {
        record.recorded_epoch <= spec.freeze_epoch
            && !record.derivation.target_program_derived
            && !record.derivation.output_derived
            && record.derivation.ancestor_task_ids.is_empty()
    };
    let training = training.iter().filter(safe).cloned().collect::<Vec<_>>();
    let calibration = calibration.iter().filter(safe).cloned().collect::<Vec<_>>();
    let training = training.as_slice();
    let calibration = calibration.as_slice();
    let programs = enumerate_features(spec.max_program_size, spec.max_programs);
    let collapsed = evaluate_feature_set(Vec::new(), training, calibration, concepts, spec);
    let mut accounting = FeatureAccounting {
        programs_enumerated: programs.len() as u64,
        max_program_size: spec.max_program_size,
        execution_fuel: spec.execution_fuel,
        ..Default::default()
    };
    let mut singleton = programs
        .iter()
        .cloned()
        .map(|program| evaluate_feature_set(vec![program], training, calibration, concepts, spec))
        .collect::<Vec<_>>();
    singleton.sort_by(|a, b| {
        a.charged_objective
            .cmp(&b.charged_objective)
            .then_with(|| a.programs.cmp(&b.programs))
    });
    let primitive_projection_regret = singleton
        .iter()
        .filter(|evaluation| {
            matches!(
                evaluation.programs.as_slice(),
                [FeatureProgram::Primitive(_)]
            )
        })
        .map(|evaluation| evaluation.regret)
        .min()
        .unwrap_or(u64::MAX);
    let pool = singleton
        .iter()
        .filter(|evaluation| evaluation.rejected.is_none())
        .take(spec.feature_pool_limit)
        .map(|evaluation| evaluation.programs[0].clone())
        .collect::<Vec<_>>();
    let mut evaluations = vec![collapsed.clone()];
    evaluations.extend(singleton);
    for width in 2..=spec.max_feature_width.min(pool.len()) {
        evaluations.extend(
            combinations(&pool, width)
                .into_iter()
                .map(|set| evaluate_feature_set(set, training, calibration, concepts, spec)),
        );
    }
    accounting.feature_sets_evaluated = evaluations.len() as u64;
    accounting.task_executions = evaluations
        .iter()
        .map(|evaluation| {
            (training.len() + calibration.len()) as u64 * evaluation.programs.len() as u64
        })
        .sum();
    accounting.execution_steps = evaluations
        .iter()
        .map(|evaluation| evaluation.execution_steps)
        .sum();
    accounting.rejected_programs = evaluations
        .iter()
        .filter(|evaluation| evaluation.rejected.is_some())
        .count() as u64;
    evaluations.sort_by(|a, b| {
        a.charged_objective
            .cmp(&b.charged_objective)
            .then_with(|| a.programs.cmp(&b.programs))
    });
    let winner = evaluations
        .iter()
        .find(|evaluation| evaluation.rejected.is_none())
        .expect("collapsed feature set is total");
    InventedFeatureRepresentation {
        encoder: FrozenFeatureEncoder {
            programs: winner.programs.clone(),
            freeze_epoch: spec.freeze_epoch,
            calibration_regret: winner.regret,
            collapsed_regret: collapsed.regret,
            charged_objective: winner.charged_objective,
            collapsed_objective: collapsed.charged_objective,
            retained: !winner.programs.is_empty()
                && winner.charged_objective < collapsed.charged_objective,
        },
        evaluations,
        accounting,
        primitive_projection_regret,
    }
}

impl FrozenFeatureEncoder {
    pub fn encode(
        &self,
        task: &RawTask,
        execution_fuel: u64,
    ) -> Result<(BTreeMap<String, String>, u64), FeatureError> {
        let spec = FeatureSelectionSpec {
            engine: SearchEngine::UniversalLambda,
            freeze_epoch: self.freeze_epoch,
            decay_per_mille: 1_000,
            interactions: true,
            max_interaction_width: 2,
            max_program_size: 0,
            max_programs: 0,
            max_feature_width: self.programs.len(),
            feature_pool_limit: 0,
            execution_fuel,
            complexity_cost: 0,
            execution_cost: 0,
        };
        let (observation, steps) = materialize_observation(&self.programs, task, &spec)?;
        Ok((
            observation
                .fields
                .into_iter()
                .map(|(name, field)| (name, field.value.to_string()))
                .collect(),
            steps,
        ))
    }
}

pub fn freeze_feature_policy(
    encoder: &FrozenFeatureEncoder,
    training: &[FeatureUtilityEvidence],
    target: &RawTask,
    concepts: &[ConceptSet],
    spec: &FeatureSelectionSpec,
) -> Result<FrozenPolicy, FeatureError> {
    let (training, _) = materialize_evidence(&encoder.programs, training, spec)?;
    let (target, _) = materialize_observation(&encoder.programs, target, spec)?;
    let names = (0..encoder.programs.len())
        .map(feature_name)
        .collect::<Vec<_>>();
    let projected = FrozenContextEncoder {
        kind: if names.is_empty() {
            EncoderKind::Collapsed
        } else {
            EncoderKind::Projection(names)
        },
        freeze_epoch: spec.freeze_epoch,
        calibration_regret: encoder.calibration_regret,
        collapsed_regret: encoder.collapsed_regret,
        retained: encoder.retained,
    };
    freeze_projected_policy(
        &projected,
        &training,
        &target,
        concepts,
        &spec.representation_spec(encoder.programs.len()),
    )
    .map_err(|error| match error {
        EncodeError::PostFreezeField(_) => FeatureError::ProtectedOutput,
        _ => FeatureError::Partial,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeatureWorkDomain {
    ProgramEnumeration,
    ProgramExecution,
    UniversalLambda,
    BehaviorBank,
}

pub fn aggregate_feature_work(
    samples: &[(FeatureWorkDomain, u64)],
) -> Result<(FeatureWorkDomain, u64), &'static str> {
    let Some((domain, _)) = samples.first() else {
        return Err("empty");
    };
    if samples.iter().any(|(next, _)| next != domain) {
        return Err("unlike work units");
    }
    Ok((*domain, samples.iter().map(|(_, work)| *work).sum()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        search_accounting::{
            EngineWork, EvaluatorBudget, EvidencePhase, RunProvenance, TerminationStatus,
        },
        universal::{Dovetail, InterleavedDovetail, ResourceLane},
    };

    fn chain(nodes: usize, label: i64) -> RawNode {
        (1..nodes).fold(RawNode::leaf(label), |tail, _| {
            RawNode::branch(label, vec![tail])
        })
    }

    fn task(id: &str, nodes: usize, nuisance_label: i64) -> RawTask {
        RawTask {
            task_id: id.into(),
            duplicate_group_id: id.into(),
            examples: vec![RawExample {
                inputs: vec![chain(nodes, nuisance_label)],
                published_output: None,
            }],
        }
    }

    fn distinct_task(id: &str, nodes: usize, distinct: usize) -> RawTask {
        let tree = (1..nodes).fold(RawNode::leaf(0), |tail, index| {
            RawNode::branch((index % distinct) as i64, vec![tail])
        });
        RawTask {
            task_id: id.into(),
            duplicate_group_id: id.into(),
            examples: vec![RawExample {
                inputs: vec![tree],
                published_output: None,
            }],
        }
    }

    fn run(task: &RawTask, work: u64, solved: bool) -> RunAccounting {
        RunAccounting {
            work: EngineWork::UniversalLambda {
                proposals: work,
                evaluated_candidates: 0,
                resource_points: 1,
            },
            max_structural_size: 7,
            evaluator_budget: EvaluatorBudget::LambdaFuel(100),
            solution_rank: solved.then_some(work),
            termination: if solved {
                TerminationStatus::Solved
            } else {
                TerminationStatus::ExhaustedFiniteBoundary
            },
            provenance: RunProvenance {
                task_id: task.task_id.clone(),
                family_id: "raw-tree".into(),
                duplicate_group_id: task.duplicate_group_id.clone(),
                context_features: BTreeMap::new(),
                concept_ids: Vec::new(),
                phase: EvidencePhase::Training,
                observed_epoch: 1,
            },
        }
    }

    fn record(task: RawTask, concept: &[&str], useful: bool) -> FeatureUtilityEvidence {
        let without = run(&task, 100, true);
        let with = run(&task, if useful { 10 } else { 100 }, true);
        FeatureUtilityEvidence {
            task,
            concept_ids: concept.iter().map(|value| (*value).into()).collect(),
            without,
            with,
            age: 0,
            recorded_epoch: 1,
            derivation: EvidenceDerivation::default(),
        }
    }

    fn add_binary_task(records: &mut Vec<FeatureUtilityEvidence>, raw: RawTask, useful: &str) {
        records.push(record(raw.clone(), &["A"], useful == "A"));
        records.push(record(raw, &["B"], useful == "B"));
    }

    fn fixture() -> (Vec<FeatureUtilityEvidence>, Vec<FeatureUtilityEvidence>) {
        let mut training = Vec::new();
        add_binary_task(&mut training, task("train-a2", 2, 10), "A");
        add_binary_task(&mut training, task("train-a4", 4, 20), "A");
        add_binary_task(&mut training, task("train-b3", 3, 30), "B");
        add_binary_task(&mut training, task("train-b5", 5, 40), "B");
        let mut calibration = Vec::new();
        add_binary_task(&mut calibration, task("cal-a6", 6, 50), "A");
        add_binary_task(&mut calibration, task("cal-b7", 7, 60), "B");
        (training, calibration)
    }

    fn concepts() -> Vec<ConceptSet> {
        vec![ConceptSet::singleton("A"), ConceptSet::singleton("B")]
    }

    fn spec() -> FeatureSelectionSpec {
        FeatureSelectionSpec {
            engine: SearchEngine::UniversalLambda,
            freeze_epoch: 1,
            decay_per_mille: 850,
            interactions: true,
            max_interaction_width: 2,
            max_program_size: 2,
            max_programs: 64,
            max_feature_width: 1,
            feature_pool_limit: 12,
            execution_fuel: 1_000,
            complexity_cost: 10,
            execution_cost: 1,
        }
    }

    fn parity_program() -> FeatureProgram {
        FeatureProgram::Unary(
            UnaryOp::Mod2,
            Box::new(FeatureProgram::Primitive(Primitive::InputNodeCount)),
        )
    }

    #[test]
    fn invents_a_compositional_feature_that_fixed_projections_cannot_match() {
        let (training, calibration) = fixture();
        let invented = invent_features(&training, &calibration, &concepts(), &spec());
        assert!(invented.encoder.retained);
        assert_eq!(invented.encoder.programs, vec![parity_program()]);
        assert_eq!(invented.encoder.calibration_regret, 0);
        assert!(invented.encoder.collapsed_regret > 0);
        assert!(invented.primitive_projection_regret > 0);
        assert!(invented.encoder.programs[0].is_compositional());

        let even = task("held-even-100", 100, 999);
        let odd = task("held-odd-101", 101, -999);
        let even_policy =
            freeze_feature_policy(&invented.encoder, &training, &even, &concepts(), &spec())
                .unwrap();
        let odd_policy =
            freeze_feature_policy(&invented.encoder, &training, &odd, &concepts(), &spec())
                .unwrap();
        assert_eq!(even_policy.ranked[0].concepts, ConceptSet::singleton("A"));
        assert_eq!(odd_policy.ranked[0].concepts, ConceptSet::singleton("B"));

        let first = invented.encoder.encode(&even, 1_000).unwrap();
        let replay = invented.encoder.encode(&even, 1_000).unwrap();
        assert_eq!(first, replay);
        assert_eq!(
            invented
                .encoder
                .encode(&task("surface-variant", 100, 123_456), 1_000),
            Ok(first)
        );
    }

    #[test]
    fn primitive_depth_ablation_and_reconstruction_richness_do_not_qualify() {
        let (training, calibration) = fixture();
        let mut shallow = spec();
        shallow.max_program_size = 1;
        let ablated = invent_features(&training, &calibration, &concepts(), &shallow);
        assert!(!ablated.encoder.retained);
        assert!(ablated.encoder.calibration_regret > 0);

        let full = invent_features(&training, &calibration, &concepts(), &spec());
        let richer = FeatureProgram::Binary(
            BinaryOp::Add,
            Box::new(parity_program()),
            Box::new(FeatureProgram::Constant(0)),
        );
        let richer_eval = evaluate_feature_set(
            vec![richer.clone()],
            &training,
            &calibration,
            &concepts(),
            &FeatureSelectionSpec {
                max_program_size: richer.size(),
                ..spec()
            },
        );
        assert_eq!(richer_eval.regret, full.encoder.calibration_regret);
        assert!(richer_eval.charged_objective > full.encoder.charged_objective);
    }

    #[test]
    fn leakage_is_filtered_before_enumeration_scoring_and_accounting() {
        let (training, calibration) = fixture();
        let clean = invent_features(&training, &calibration, &concepts(), &spec());
        let mut poisoned_training = training.clone();
        let mut poison = record(task("target-derived", 42, 42), &["B"], true);
        poison.derivation.target_program_derived = true;
        poisoned_training.push(poison);
        let mut output_poison = record(task("output-derived", 44, 44), &["B"], true);
        output_poison.derivation.output_derived = true;
        poisoned_training.push(output_poison);
        let mut ancestry = record(task("ancestry", 45, 45), &["B"], true);
        ancestry
            .derivation
            .ancestor_task_ids
            .insert("cal-a6".into());
        poisoned_training.push(ancestry);
        let mut late = record(task("post-freeze", 43, 43), &["B"], true);
        late.recorded_epoch = 2;
        poisoned_training.push(late);
        let poisoned = invent_features(&poisoned_training, &calibration, &concepts(), &spec());
        assert_eq!(poisoned, clean);

        let mut duplicate_training = training.clone();
        for record in &mut duplicate_training {
            if record.task.task_id == "train-a2" {
                record.task.duplicate_group_id = "cal-a6".into();
            }
        }
        let duplicate = invent_features(&duplicate_training, &calibration, &concepts(), &spec());
        assert_eq!(duplicate.encoder.programs, clean.encoder.programs);
    }

    #[test]
    fn unsafe_partial_divergent_and_metadata_like_controls_are_rejected() {
        let raw = task("safe", 8, 1);
        assert_eq!(
            execute_feature(&FeatureProgram::Diverge, &raw, 5),
            Err(FeatureError::OutOfFuel)
        );
        assert_eq!(
            execute_feature(
                &FeatureProgram::Primitive(Primitive::InputNodeCount),
                &raw,
                1,
            ),
            Err(FeatureError::OutOfFuel)
        );
        assert_eq!(
            execute_feature(&FeatureProgram::Partial, &raw, 5),
            Err(FeatureError::Partial)
        );
        assert_eq!(
            execute_feature(&FeatureProgram::Unstable, &raw, 5),
            Err(FeatureError::Unstable)
        );
        assert_eq!(
            execute_feature(&FeatureProgram::Noise, &raw, 5),
            Err(FeatureError::Nondeterministic)
        );
        assert_eq!(
            execute_feature(&FeatureProgram::Shuffled, &raw, 5),
            Err(FeatureError::Shuffled)
        );
        assert_eq!(
            execute_feature(&FeatureProgram::IdentityProbe, &raw, 5),
            Err(FeatureError::MetadataAccess)
        );
        assert_eq!(
            execute_feature(&FeatureProgram::ProtectedOutputProbe, &raw, 5),
            Err(FeatureError::ProtectedOutput)
        );
        let enumerated = enumerate_features(4, 10_000);
        assert!(!enumerated.iter().any(|program| matches!(
            program,
            FeatureProgram::Diverge
                | FeatureProgram::Partial
                | FeatureProgram::Unstable
                | FeatureProgram::Noise
                | FeatureProgram::Shuffled
                | FeatureProgram::IdentityProbe
                | FeatureProgram::ProtectedOutputProbe
        )));
        let constant = FeatureProgram::Constant(0);
        let (training, calibration) = fixture();
        let constant_eval = evaluate_feature_set(
            vec![constant],
            &training,
            &calibration,
            &concepts(),
            &spec(),
        );
        assert!(constant_eval.regret > 0);
    }

    #[test]
    fn exact_size_enumeration_is_deterministic_and_reaches_required_programs() {
        let first = enumerate_features(3, 10_000);
        let second = enumerate_features(3, 10_000);
        assert_eq!(first, second);
        assert!(first
            .windows(2)
            .all(|pair| pair[0].size() <= pair[1].size()));
        assert!(first.contains(&parity_program()));
        assert!(
            first.contains(&FeatureProgram::Relation(TransformProgram::MapChildren(
                Box::new(TransformProgram::ReverseChildren)
            )))
        );
    }

    #[test]
    fn lambda_and_grid_inputs_share_the_same_lower_level_tree_language() {
        let lambda = crate::term::lam(crate::term::app(crate::term::var(0), crate::term::var(0)));
        let lambda_task = RawTask {
            task_id: "lambda".into(),
            duplicate_group_id: "lambda".into(),
            examples: vec![RawExample {
                inputs: vec![RawNode::lambda(&lambda)],
                published_output: None,
            }],
        };
        let grid_input = vec![vec![1, 2], vec![3, 4]];
        let grid_output = vec![vec![2, 1], vec![4, 3]];
        let grid_task = RawTask {
            task_id: "grid".into(),
            duplicate_group_id: "grid".into(),
            examples: vec![RawExample {
                inputs: vec![RawNode::grid(&grid_input)],
                published_output: Some(RawNode::grid(&grid_output)),
            }],
        };
        let count = FeatureProgram::Primitive(Primitive::InputNodeCount);
        assert!(execute_feature(&count, &lambda_task, 100).unwrap().value > 0);
        assert!(execute_feature(&count, &grid_task, 100).unwrap().value > 0);
        let mirror = FeatureProgram::Relation(TransformProgram::MapChildren(Box::new(
            TransformProgram::ReverseChildren,
        )));
        assert_eq!(execute_feature(&mirror, &grid_task, 100).unwrap().value, 1);
        assert_eq!(
            execute_feature(&mirror, &lambda_task, 100),
            Err(FeatureError::MissingPublishedOutput)
        );
    }

    #[test]
    fn invented_context_supports_interactions_and_shift_adaptation() {
        let mut training = Vec::new();
        for (id, nodes) in [("pair-even-2", 2), ("pair-even-4", 4)] {
            let raw = task(id, nodes, nodes as i64);
            training.push(record(raw.clone(), &["A"], false));
            training.push(record(raw.clone(), &["B"], false));
            training.push(record(raw, &["A", "B"], true));
        }
        for (id, nodes) in [("single-odd-3", 3), ("single-odd-5", 5)] {
            let raw = task(id, nodes, nodes as i64);
            training.push(record(raw.clone(), &["A"], true));
            training.push(record(raw.clone(), &["B"], false));
            training.push(record(raw, &["A", "B"], false));
        }
        let mut calibration = Vec::new();
        for (id, nodes, pair) in [("cal-even", 6, true), ("cal-odd", 7, false)] {
            let raw = task(id, nodes, 100 + nodes as i64);
            calibration.push(record(raw.clone(), &["A"], !pair));
            calibration.push(record(raw.clone(), &["B"], false));
            calibration.push(record(raw, &["A", "B"], pair));
        }
        let sets = vec![
            ConceptSet::singleton("A"),
            ConceptSet::singleton("B"),
            ConceptSet::new(["A".into(), "B".into()]),
        ];
        let invented = invent_features(&training, &calibration, &sets, &spec());
        assert_eq!(invented.encoder.programs, vec![parity_program()]);
        let held = task("held-pair", 100, -1);
        let policy =
            freeze_feature_policy(&invented.encoder, &training, &held, &sets, &spec()).unwrap();
        assert_eq!(
            policy.ranked[0].concepts,
            ConceptSet::new(["A".into(), "B".into()])
        );
        assert!(policy.ranked[0].interaction_residual > 0);
        let mut no_interactions = spec();
        no_interactions.interactions = false;
        let ablated =
            freeze_feature_policy(&invented.encoder, &training, &held, &sets, &no_interactions)
                .unwrap();
        assert!(ablated
            .ranked
            .iter()
            .all(|weight| weight.concepts.len() == 1));
        assert!(ablated.ranked[0].score <= 0);

        let (base_training, base_calibration) = fixture();
        let base = invent_features(&base_training, &base_calibration, &concepts(), &spec());
        let mut old_a = record(task("old-a", 100, 1), &["A"], true);
        old_a.age = 8;
        if let EngineWork::UniversalLambda { proposals, .. } = &mut old_a.without.work {
            *proposals = 1_000;
        }
        let mut fresh_b = record(task("fresh-b", 100, 2), &["B"], true);
        if let EngineWork::UniversalLambda { proposals, .. } = &mut fresh_b.without.work {
            *proposals = 500;
        }
        let old_context = record(task("old-context", 101, 3), &["A"], true);
        let shifted = vec![old_a, fresh_b, old_context];
        let decayed = freeze_feature_policy(
            &base.encoder,
            &shifted,
            &task("new", 100, 999),
            &concepts(),
            &spec(),
        )
        .unwrap();
        assert_eq!(decayed.ranked[0].concepts, ConceptSet::singleton("B"));
        let mut stale_spec = spec();
        stale_spec.decay_per_mille = 1_000;
        let stale = freeze_feature_policy(
            &base.encoder,
            &shifted,
            &task("new", 100, 999),
            &concepts(),
            &stale_spec,
        )
        .unwrap();
        assert_eq!(stale.ranked[0].concepts, ConceptSet::singleton("A"));
        let replay = freeze_feature_policy(
            &base.encoder,
            &shifted,
            &task("old-replay", 101, -999),
            &concepts(),
            &spec(),
        )
        .unwrap();
        assert_eq!(replay.ranked[0].concepts, ConceptSet::singleton("A"));
    }

    #[test]
    fn stale_feature_loses_selection_priority_after_a_contextual_shift() {
        let mut training = Vec::new();
        let mut old_a = record(distinct_task("old-a", 2, 1), &["A"], true);
        let mut old_b = record(distinct_task("old-b", 3, 2), &["B"], true);
        for old in [&mut old_a, &mut old_b] {
            old.age = 8;
            if let EngineWork::UniversalLambda { proposals, .. } = &mut old.without.work {
                *proposals = 1_000;
            }
        }
        training.extend([old_a, old_b]);
        let mut fresh_a = record(distinct_task("fresh-a", 3, 2), &["A"], true);
        let mut fresh_b = record(distinct_task("fresh-b", 2, 1), &["B"], true);
        for fresh in [&mut fresh_a, &mut fresh_b] {
            if let EngineWork::UniversalLambda { proposals, .. } = &mut fresh.without.work {
                *proposals = 500;
            }
        }
        training.extend([fresh_a, fresh_b]);
        let calibration = vec![
            record(distinct_task("cal-a", 5, 2), &["A"], true),
            record(distinct_task("cal-a", 5, 2), &["B"], false),
            record(distinct_task("cal-b", 4, 1), &["A"], false),
            record(distinct_task("cal-b", 4, 1), &["B"], true),
        ];
        let decayed = invent_features(&training, &calibration, &concepts(), &spec());
        assert_eq!(
            decayed.encoder.programs,
            vec![FeatureProgram::Primitive(Primitive::InputDistinctLabels)]
        );
        let mut no_decay = spec();
        no_decay.decay_per_mille = 1_000;
        let stale = invent_features(&training, &calibration, &concepts(), &no_decay);
        assert!(stale.encoder.programs.is_empty());
        assert!(!stale.encoder.retained);
        assert!(decayed.encoder.calibration_regret < stale.encoder.calibration_regret);
    }

    #[test]
    fn feature_accounting_is_exact_unmixable_and_universal_lane_is_unchanged() {
        let (training, calibration) = fixture();
        let first = invent_features(&training, &calibration, &concepts(), &spec());
        let replay = invent_features(&training, &calibration, &concepts(), &spec());
        assert_eq!(first, replay);
        assert!(first.accounting.programs_enumerated > 0);
        assert!(first.accounting.task_executions > 0);
        assert_eq!(
            aggregate_feature_work(&[
                (FeatureWorkDomain::ProgramExecution, 3),
                (FeatureWorkDomain::ProgramExecution, 4),
            ]),
            Ok((FeatureWorkDomain::ProgramExecution, 7))
        );
        assert_eq!(
            aggregate_feature_work(&[
                (FeatureWorkDomain::ProgramExecution, 3),
                (FeatureWorkDomain::UniversalLambda, 4),
            ]),
            Err("unlike work units")
        );
        let learned = (0..first.accounting.feature_sets_evaluated)
            .map(|index| ((index % 7 + 1) as u32, index + 1));
        let mut schedule = InterleavedDovetail::new(learned);
        let mut projected = Vec::new();
        while projected.len() < 256 {
            let point = schedule.next_labeled().unwrap();
            if point.lane == ResourceLane::Universal {
                projected.push((point.syntax_size, point.evaluation_fuel));
            }
        }
        assert_eq!(projected, Dovetail::default().take(256).collect::<Vec<_>>());
    }
}
