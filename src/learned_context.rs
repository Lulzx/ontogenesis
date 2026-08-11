//! Regret-selected representations of raw, pre-search task observations.
//!
//! This layer learns *which observable distinctions should condition search*.
//! It enumerates bounded projections of raw fields, freezes each candidate,
//! and selects the projection with the lowest leave-group-out calibration
//! regret.  Encoder work is deliberately not converted into lambda proposals
//! or behavior-bank constructions.

use crate::{
    contextual_allocation::{
        evidence_utility, ConceptSet, ContextualEvidence, ContextualLedger, EvidenceDerivation,
        FreezeSpec, FrozenPolicy, TaskContext,
    },
    search_accounting::{RunAccounting, SearchEngine},
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FieldOrigin {
    PreSearchObservable,
    ProtectedOutput,
    HeldoutIdentity,
    SolutionDerived,
    TargetAncestry,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawField {
    pub value: i64,
    pub origin: FieldOrigin,
    pub observed_epoch: u64,
}

impl RawField {
    pub fn observable(value: i64, observed_epoch: u64) -> Self {
        Self {
            value,
            origin: FieldOrigin::PreSearchObservable,
            observed_epoch,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawTaskObservation {
    pub task_id: String,
    pub duplicate_group_id: String,
    /// Generic numeric measurements, not target-family labels.
    pub fields: BTreeMap<String, RawField>,
}

#[derive(Clone, Debug)]
pub struct RawUtilityEvidence {
    pub observation: RawTaskObservation,
    pub concept_ids: Vec<String>,
    pub without: RunAccounting,
    pub with: RunAccounting,
    pub age: u32,
    pub recorded_epoch: u64,
    pub derivation: EvidenceDerivation,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EncoderKind {
    Collapsed,
    Projection(Vec<String>),
    /// Controls only. Production enumeration never proposes metadata identity.
    IdentityMemorizer,
    ShuffledProjection(Vec<String>),
    UnstableProjection(Vec<String>),
}

impl EncoderKind {
    fn complexity(&self) -> usize {
        match self {
            Self::Collapsed => 0,
            Self::Projection(fields)
            | Self::ShuffledProjection(fields)
            | Self::UnstableProjection(fields) => fields.len(),
            Self::IdentityMemorizer => usize::MAX,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrozenContextEncoder {
    pub kind: EncoderKind,
    pub freeze_epoch: u64,
    pub calibration_regret: u64,
    pub collapsed_regret: u64,
    pub retained: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EncodeError {
    ForbiddenIdentity,
    MissingField(String),
    ForbiddenOrigin { field: String, origin: FieldOrigin },
    PostFreezeField(String),
}

impl FrozenContextEncoder {
    pub fn encode(&self, raw: &RawTaskObservation) -> Result<TaskContext, EncodeError> {
        let selected = match &self.kind {
            EncoderKind::Collapsed => Vec::new(),
            EncoderKind::IdentityMemorizer => return Err(EncodeError::ForbiddenIdentity),
            EncoderKind::Projection(fields)
            | EncoderKind::ShuffledProjection(fields)
            | EncoderKind::UnstableProjection(fields) => fields.clone(),
        };
        let mut features = BTreeMap::new();
        for (index, name) in selected.iter().enumerate() {
            let field = raw
                .fields
                .get(name)
                .ok_or_else(|| EncodeError::MissingField(name.clone()))?;
            if field.origin != FieldOrigin::PreSearchObservable {
                return Err(EncodeError::ForbiddenOrigin {
                    field: name.clone(),
                    origin: field.origin,
                });
            }
            if field.observed_epoch > self.freeze_epoch {
                return Err(EncodeError::PostFreezeField(name.clone()));
            }
            let mut value = field.value;
            if matches!(self.kind, EncoderKind::ShuffledProjection(_)) {
                // Diagnostic only: deliberately scramble assignments between
                // task groups. Production enumeration never proposes this.
                let task_hash = raw.task_id.bytes().fold(0u64, |hash, byte| {
                    hash.wrapping_mul(131).wrapping_add(byte.into())
                });
                value ^= (task_hash & 1) as i64;
            }
            if matches!(self.kind, EncoderKind::UnstableProjection(_)) {
                value = value.saturating_add(raw.task_id.len() as i64);
            }
            // Hide raw vocabulary from the contextual policy. Only the learned
            // coordinates and their values cross this boundary.
            features.insert(format!("z{index}"), value.to_string());
        }
        Ok(TaskContext {
            task_id: raw.task_id.clone(),
            family_id: "learned-z".into(),
            duplicate_group_id: raw.duplicate_group_id.clone(),
            features,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EncoderAccounting {
    pub candidates_evaluated: u64,
    pub validation_predictions: u64,
    pub raw_fields_inspected: u64,
    pub max_projection_width: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateEvaluation {
    pub kind: EncoderKind,
    pub regret: u64,
    pub predictions: usize,
    pub rejected: Option<EncodeError>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LearnedRepresentation {
    pub encoder: FrozenContextEncoder,
    pub evaluations: Vec<CandidateEvaluation>,
    pub accounting: EncoderAccounting,
}

#[derive(Clone, Debug)]
pub struct RepresentationSpec {
    pub engine: SearchEngine,
    pub freeze_epoch: u64,
    pub decay_per_mille: u16,
    pub interactions: bool,
    pub max_interaction_width: usize,
    pub max_projection_width: usize,
}

pub fn enumerate_projection_candidates(
    observations: &[RawTaskObservation],
    max_width: usize,
) -> Vec<EncoderKind> {
    let mut common: Option<BTreeSet<String>> = None;
    for observation in observations {
        let safe = observation
            .fields
            .iter()
            .filter(|(_, field)| field.origin == FieldOrigin::PreSearchObservable)
            .map(|(name, _)| name.clone())
            .collect::<BTreeSet<_>>();
        common = Some(match common {
            None => safe,
            Some(previous) => previous.intersection(&safe).cloned().collect(),
        });
    }
    let fields = common.unwrap_or_default().into_iter().collect::<Vec<_>>();
    let mut result = vec![EncoderKind::Collapsed];
    fn combinations(
        fields: &[String],
        start: usize,
        remaining: usize,
        current: &mut Vec<String>,
        out: &mut Vec<EncoderKind>,
    ) {
        if remaining == 0 {
            out.push(EncoderKind::Projection(current.clone()));
            return;
        }
        for index in start..=fields.len().saturating_sub(remaining) {
            current.push(fields[index].clone());
            combinations(fields, index + 1, remaining - 1, current, out);
            current.pop();
        }
    }
    for width in 1..=max_width.min(fields.len()) {
        combinations(&fields, 0, width, &mut Vec::new(), &mut result);
    }
    result
}

fn contextual_evidence(
    raw: &RawUtilityEvidence,
    encoder: &FrozenContextEncoder,
) -> Result<ContextualEvidence, EncodeError> {
    Ok(ContextualEvidence {
        context: encoder.encode(&raw.observation)?,
        concept_ids: raw.concept_ids.clone(),
        without: raw.without.clone(),
        with: raw.with.clone(),
        age: raw.age,
        recorded_epoch: raw.recorded_epoch,
        derivation: raw.derivation.clone(),
    })
}

pub fn evaluate_encoder(
    kind: EncoderKind,
    training: &[RawUtilityEvidence],
    calibration: &[RawUtilityEvidence],
    concepts: &[ConceptSet],
    spec: &RepresentationSpec,
) -> CandidateEvaluation {
    let encoder = FrozenContextEncoder {
        kind: kind.clone(),
        freeze_epoch: spec.freeze_epoch,
        calibration_regret: 0,
        collapsed_regret: 0,
        retained: false,
    };
    let mut ledger = ContextualLedger::default();
    for raw in training {
        match contextual_evidence(raw, &encoder) {
            Ok(evidence) => ledger.record(evidence),
            Err(error) => {
                return CandidateEvaluation {
                    kind,
                    regret: u64::MAX,
                    predictions: 0,
                    rejected: Some(error),
                }
            }
        }
    }
    let task_ids = calibration
        .iter()
        .map(|record| record.observation.task_id.clone())
        .collect::<BTreeSet<_>>();
    let mut regret = 0u64;
    let mut predictions = 0usize;
    for task_id in &task_ids {
        let records = calibration
            .iter()
            .filter(|record| record.observation.task_id == *task_id)
            .collect::<Vec<_>>();
        let Some(first) = records.first() else {
            continue;
        };
        let target = match encoder.encode(&first.observation) {
            Ok(target) => target,
            Err(error) => {
                return CandidateEvaluation {
                    kind,
                    regret: u64::MAX,
                    predictions,
                    rejected: Some(error),
                }
            }
        };
        let policy = ledger.learn(
            concepts,
            &FreezeSpec {
                target,
                engine: spec.engine,
                freeze_epoch: spec.freeze_epoch,
                decay_per_mille: spec.decay_per_mille,
                contextual: true,
                interactions: spec.interactions,
                max_interaction_width: spec.max_interaction_width,
            },
        );
        let predicted = policy.ranked.first().map(|weight| &weight.concepts);
        let oracle = records
            .iter()
            .map(|record| evidence_utility(&contextual_evidence(record, &encoder).unwrap()))
            .max()
            .unwrap_or(0);
        let selected = predicted
            .and_then(|set| {
                records
                    .iter()
                    .find(|record| ConceptSet::new(record.concept_ids.clone()) == *set)
            })
            .map(|record| evidence_utility(&contextual_evidence(record, &encoder).unwrap()))
            // A concept set without a calibration intervention has no earned
            // utility; do not manufacture an enormous pseudo-cost.
            .unwrap_or(0);
        regret = regret.saturating_add(oracle.saturating_sub(selected).max(0) as u64);
        predictions += 1;
    }
    CandidateEvaluation {
        kind,
        regret,
        predictions,
        rejected: None,
    }
}

pub fn learn_representation(
    training: &[RawUtilityEvidence],
    calibration: &[RawUtilityEvidence],
    concepts: &[ConceptSet],
    spec: &RepresentationSpec,
) -> LearnedRepresentation {
    let observations = training
        .iter()
        .chain(calibration)
        .filter(|record| {
            record.recorded_epoch <= spec.freeze_epoch
                && !record.derivation.target_program_derived
                && !record.derivation.output_derived
        })
        .map(|record| record.observation.clone())
        .collect::<Vec<_>>();
    let candidates = enumerate_projection_candidates(&observations, spec.max_projection_width);
    let mut accounting = EncoderAccounting {
        max_projection_width: spec.max_projection_width,
        ..Default::default()
    };
    let mut evaluations = candidates
        .into_iter()
        .map(|kind| {
            accounting.candidates_evaluated += 1;
            accounting.raw_fields_inspected +=
                kind.complexity().min(1_000) as u64 * (training.len() + calibration.len()) as u64;
            let result = evaluate_encoder(kind, training, calibration, concepts, spec);
            accounting.validation_predictions += result.predictions as u64;
            result
        })
        .collect::<Vec<_>>();
    evaluations.sort_by(|a, b| {
        a.regret
            .cmp(&b.regret)
            .then_with(|| a.kind.complexity().cmp(&b.kind.complexity()))
            .then_with(|| a.kind.cmp(&b.kind))
    });
    let collapsed_regret = evaluations
        .iter()
        .find(|evaluation| evaluation.kind == EncoderKind::Collapsed)
        .expect("collapsed baseline")
        .regret;
    let winner = evaluations
        .iter()
        .find(|evaluation| evaluation.rejected.is_none())
        .expect("at least collapsed encoder");
    LearnedRepresentation {
        encoder: FrozenContextEncoder {
            kind: winner.kind.clone(),
            freeze_epoch: spec.freeze_epoch,
            calibration_regret: winner.regret,
            collapsed_regret,
            retained: winner.regret < collapsed_regret,
        },
        evaluations,
        accounting,
    }
}

pub fn freeze_policy(
    encoder: &FrozenContextEncoder,
    training: &[RawUtilityEvidence],
    target: &RawTaskObservation,
    concepts: &[ConceptSet],
    spec: &RepresentationSpec,
) -> Result<FrozenPolicy, EncodeError> {
    let mut ledger = ContextualLedger::default();
    for raw in training {
        ledger.record(contextual_evidence(raw, encoder)?);
    }
    Ok(ledger.learn(
        concepts,
        &FreezeSpec {
            target: encoder.encode(target)?,
            engine: spec.engine,
            freeze_epoch: spec.freeze_epoch,
            decay_per_mille: spec.decay_per_mille,
            contextual: true,
            interactions: spec.interactions,
            max_interaction_width: spec.max_interaction_width,
        },
    ))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkDomain {
    EncoderCandidates,
    UniversalLambda,
    BehaviorBank,
}

pub fn aggregate_same_domain(
    samples: &[(WorkDomain, u64)],
) -> Result<(WorkDomain, u64), &'static str> {
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
        universal::{InterleavedDovetail, ResourceLane},
    };

    fn raw(task: &str, group: &str, signal: i64, nuisance: i64) -> RawTaskObservation {
        RawTaskObservation {
            task_id: task.into(),
            duplicate_group_id: group.into(),
            fields: BTreeMap::from([
                ("measure-0".into(), RawField::observable(signal, 1)),
                ("measure-1".into(), RawField::observable(nuisance, 1)),
            ]),
        }
    }

    fn run(task: &str, work: u64, solved: bool) -> RunAccounting {
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
                task_id: task.into(),
                family_id: "raw".into(),
                duplicate_group_id: task.into(),
                context_features: BTreeMap::new(),
                concept_ids: Vec::new(),
                phase: EvidencePhase::Training,
                observed_epoch: 1,
            },
        }
    }

    fn record(
        observation: RawTaskObservation,
        concept: &[&str],
        useful: bool,
    ) -> RawUtilityEvidence {
        let task = observation.task_id.clone();
        RawUtilityEvidence {
            observation,
            concept_ids: concept.iter().map(|id| (*id).into()).collect(),
            without: run(&task, 100, true),
            with: run(&task, if useful { 10 } else { 100 }, true),
            age: 0,
            recorded_epoch: 1,
            derivation: EvidenceDerivation::default(),
        }
    }

    fn add_task(out: &mut Vec<RawUtilityEvidence>, observation: RawTaskObservation, useful: &str) {
        out.push(record(observation.clone(), &["A"], useful == "A"));
        out.push(record(observation, &["B"], useful == "B"));
    }

    fn fixture() -> (Vec<RawUtilityEvidence>, Vec<RawUtilityEvidence>) {
        let mut training = Vec::new();
        add_task(&mut training, raw("train-a1", "a1", 0, 10), "A");
        add_task(&mut training, raw("train-a2", "a2", 0, 20), "A");
        add_task(&mut training, raw("train-b1", "b1", 1, 30), "B");
        add_task(&mut training, raw("train-b2", "b2", 1, 40), "B");
        let mut calibration = Vec::new();
        add_task(&mut calibration, raw("cal-a", "ca", 0, 99), "A");
        add_task(&mut calibration, raw("cal-b", "cb", 1, 98), "B");
        (training, calibration)
    }

    fn spec() -> RepresentationSpec {
        RepresentationSpec {
            engine: SearchEngine::UniversalLambda,
            freeze_epoch: 1,
            decay_per_mille: 900,
            interactions: true,
            max_interaction_width: 2,
            max_projection_width: 2,
        }
    }

    fn concepts() -> Vec<ConceptSet> {
        vec![ConceptSet::singleton("A"), ConceptSet::singleton("B")]
    }

    #[test]
    fn regret_learning_separates_needed_contexts_and_merges_surface_variants() {
        let (training, calibration) = fixture();
        let learned = learn_representation(&training, &calibration, &concepts(), &spec());
        assert_eq!(
            learned.encoder.kind,
            EncoderKind::Projection(vec!["measure-0".into()])
        );
        assert!(learned.encoder.retained);
        assert_eq!(learned.encoder.calibration_regret, 0);
        assert!(learned.encoder.collapsed_regret > 0);

        let target_a = raw("held-a", "ha", 0, -7_000);
        let target_b = raw("held-b", "hb", 1, 7_000);
        let policy_a =
            freeze_policy(&learned.encoder, &training, &target_a, &concepts(), &spec()).unwrap();
        let policy_b =
            freeze_policy(&learned.encoder, &training, &target_b, &concepts(), &spec()).unwrap();
        assert_eq!(policy_a.ranked[0].concepts, ConceptSet::singleton("A"));
        assert_eq!(policy_b.ranked[0].concepts, ConceptSet::singleton("B"));

        let alternate_surface = raw("held-a2", "ha2", 0, 123_456);
        assert_eq!(
            learned.encoder.encode(&target_a).unwrap().features,
            learned.encoder.encode(&alternate_surface).unwrap().features
        );
    }

    #[test]
    fn ablations_and_adversarial_encoders_do_not_match_learned_regret() {
        let (training, calibration) = fixture();
        let candidates = concepts();
        let learned = learn_representation(&training, &calibration, &candidates, &spec());
        let collapsed = evaluate_encoder(
            EncoderKind::Collapsed,
            &training,
            &calibration,
            &candidates,
            &spec(),
        );
        let irrelevant = evaluate_encoder(
            EncoderKind::Projection(vec!["measure-1".into()]),
            &training,
            &calibration,
            &candidates,
            &spec(),
        );
        let unstable = evaluate_encoder(
            EncoderKind::UnstableProjection(vec!["measure-0".into()]),
            &training,
            &calibration,
            &candidates,
            &spec(),
        );
        let shuffled = evaluate_encoder(
            EncoderKind::ShuffledProjection(vec!["measure-0".into()]),
            &training,
            &calibration,
            &candidates,
            &spec(),
        );
        let identity = evaluate_encoder(
            EncoderKind::IdentityMemorizer,
            &training,
            &calibration,
            &candidates,
            &spec(),
        );
        assert_eq!(learned.encoder.calibration_regret, 0);
        assert!(collapsed.regret > 0);
        assert!(irrelevant.regret > 0);
        assert!(unstable.regret > 0);
        assert!(shuffled.regret > 0);
        assert_eq!(identity.rejected, Some(EncodeError::ForbiddenIdentity));
        assert_eq!(identity.regret, u64::MAX);
        assert!(learned
            .evaluations
            .iter()
            .all(|evaluation| !matches!(evaluation.kind, EncoderKind::IdentityMemorizer)));
    }

    #[test]
    fn protected_late_and_solution_fields_are_excluded_and_cannot_change_policy() {
        let (training, calibration) = fixture();
        let learned = learn_representation(&training, &calibration, &concepts(), &spec());
        let mut poisoned_training = training.clone();
        let mut poison = poisoned_training[0].clone();
        poison.observation.task_id = "poison".into();
        poison.observation.duplicate_group_id = "poison".into();
        poison
            .observation
            .fields
            .insert("perfect-target-label".into(), RawField::observable(999, 1));
        poison.derivation.target_program_derived = true;
        poisoned_training.push(poison);
        let poisoned = learn_representation(&poisoned_training, &calibration, &concepts(), &spec());
        assert_eq!(poisoned.encoder, learned.encoder);
        assert_eq!(poisoned.evaluations, learned.evaluations);

        let mut target = raw("held", "held", 0, 5);
        target.fields.insert(
            "protected".into(),
            RawField {
                value: 1,
                origin: FieldOrigin::ProtectedOutput,
                observed_epoch: 1,
            },
        );
        target.fields.insert(
            "solution".into(),
            RawField {
                value: 2,
                origin: FieldOrigin::SolutionDerived,
                observed_epoch: 1,
            },
        );
        target.fields.insert(
            "identity".into(),
            RawField {
                value: 3,
                origin: FieldOrigin::HeldoutIdentity,
                observed_epoch: 1,
            },
        );
        target.fields.insert(
            "ancestry".into(),
            RawField {
                value: 4,
                origin: FieldOrigin::TargetAncestry,
                observed_epoch: 1,
            },
        );
        target
            .fields
            .insert("late".into(), RawField::observable(3, 2));
        let before = learned.encoder.encode(&target).unwrap();
        target.fields.get_mut("protected").unwrap().value = 999_999;
        target.fields.get_mut("solution").unwrap().value = -999_999;
        target.fields.get_mut("late").unwrap().value = 42;
        let after = learned.encoder.encode(&target).unwrap();
        assert_eq!(before, after);

        for (field, expected) in [
            ("protected", FieldOrigin::ProtectedOutput),
            ("solution", FieldOrigin::SolutionDerived),
            ("identity", FieldOrigin::HeldoutIdentity),
            ("ancestry", FieldOrigin::TargetAncestry),
        ] {
            let encoder = FrozenContextEncoder {
                kind: EncoderKind::Projection(vec![field.into()]),
                freeze_epoch: 1,
                calibration_regret: 0,
                collapsed_regret: 0,
                retained: false,
            };
            assert!(matches!(
                encoder.encode(&target),
                Err(EncodeError::ForbiddenOrigin { origin, .. }) if origin == expected
            ));
        }
        let late = FrozenContextEncoder {
            kind: EncoderKind::Projection(vec!["late".into()]),
            freeze_epoch: 1,
            calibration_regret: 0,
            collapsed_regret: 0,
            retained: false,
        };
        assert_eq!(
            late.encode(&target),
            Err(EncodeError::PostFreezeField("late".into()))
        );
    }

    #[test]
    fn selection_uses_allocation_regret_not_richer_reconstruction() {
        let (training, calibration) = fixture();
        let learned = learn_representation(&training, &calibration, &concepts(), &spec());
        assert_eq!(learned.encoder.kind.complexity(), 1);
        let richer = learned
            .evaluations
            .iter()
            .find(|evaluation| {
                evaluation.kind
                    == EncoderKind::Projection(vec!["measure-0".into(), "measure-1".into()])
            })
            .unwrap();
        assert_eq!(richer.regret, learned.encoder.calibration_regret);
        assert!(richer.kind.complexity() > learned.encoder.kind.complexity());
        assert_ne!(richer.kind, learned.encoder.kind);
    }

    #[test]
    fn interaction_can_be_conditioned_by_a_learned_representation() {
        let (mut training, mut calibration) = fixture();
        let pair = ConceptSet::new(["A".into(), "B".into()]);
        for records in [&mut training, &mut calibration] {
            let observations = records
                .iter()
                .filter(|record| record.observation.fields["measure-0"].value == 2)
                .map(|record| record.observation.clone())
                .collect::<Vec<_>>();
            assert!(observations.is_empty());
        }
        for (task, nuisance) in [("train-p1", 50), ("train-p2", 60)] {
            let observation = raw(task, task, 2, nuisance);
            training.push(record(observation.clone(), &["A"], false));
            training.push(record(observation.clone(), &["B"], false));
            training.push(record(observation, &["A", "B"], true));
        }
        let observation = raw("cal-p", "cal-p", 2, 70);
        calibration.push(record(observation.clone(), &["A"], false));
        calibration.push(record(observation.clone(), &["B"], false));
        calibration.push(record(observation, &["A", "B"], true));
        let all = vec![
            ConceptSet::singleton("A"),
            ConceptSet::singleton("B"),
            pair.clone(),
        ];
        let learned = learn_representation(&training, &calibration, &all, &spec());
        let target = raw("held-p", "held-p", 2, 999);
        let policy = freeze_policy(&learned.encoder, &training, &target, &all, &spec()).unwrap();
        assert_eq!(policy.ranked[0].concepts, pair);
        assert!(policy.ranked[0].interaction_residual > 0);
        let mut ablated = spec();
        ablated.interactions = false;
        let no_interaction =
            freeze_policy(&learned.encoder, &training, &target, &all, &ablated).unwrap();
        assert!(no_interaction
            .ranked
            .iter()
            .all(|weight| weight.concepts.len() == 1));
        assert!(no_interaction.ranked[0].score <= 0);
    }

    #[test]
    fn learned_context_composes_with_decay_under_shift_without_context_collapse() {
        let (training, calibration) = fixture();
        let learned = learn_representation(&training, &calibration, &concepts(), &spec());
        let mut old_a = record(raw("shift-old-a", "soa", 0, 1), &["A"], true);
        old_a.age = 8;
        if let EngineWork::UniversalLambda { proposals, .. } = &mut old_a.without.work {
            *proposals = 1_000;
        }
        let mut fresh_b = record(raw("shift-new-b", "snb", 0, 2), &["B"], true);
        if let EngineWork::UniversalLambda { proposals, .. } = &mut fresh_b.without.work {
            *proposals = 500;
        }
        let old_context_a = record(raw("old-context-a", "oca", 1, 3), &["A"], true);
        let shifted = vec![old_a, fresh_b, old_context_a];

        let new_target = raw("new-target", "nt", 0, 999);
        let decayed = freeze_policy(
            &learned.encoder,
            &shifted,
            &new_target,
            &concepts(),
            &spec(),
        )
        .unwrap();
        assert_eq!(decayed.ranked[0].concepts, ConceptSet::singleton("B"));

        let mut stale_spec = spec();
        stale_spec.decay_per_mille = 1_000;
        let stale = freeze_policy(
            &learned.encoder,
            &shifted,
            &new_target,
            &concepts(),
            &stale_spec,
        )
        .unwrap();
        assert_eq!(stale.ranked[0].concepts, ConceptSet::singleton("A"));

        let old_replay = freeze_policy(
            &learned.encoder,
            &shifted,
            &raw("old-replay", "or", 1, -999),
            &concepts(),
            &spec(),
        )
        .unwrap();
        assert_eq!(old_replay.ranked[0].concepts, ConceptSet::singleton("A"));
    }

    #[test]
    fn accounting_units_are_exact_and_cannot_be_mixed() {
        let (training, calibration) = fixture();
        let first = learn_representation(&training, &calibration, &concepts(), &spec());
        let second = learn_representation(&training, &calibration, &concepts(), &spec());
        assert_eq!(first, second);
        assert!(first.accounting.candidates_evaluated > 0);
        assert_eq!(
            aggregate_same_domain(&[
                (WorkDomain::EncoderCandidates, 3),
                (WorkDomain::EncoderCandidates, 4),
            ]),
            Ok((WorkDomain::EncoderCandidates, 7))
        );
        assert_eq!(
            aggregate_same_domain(&[
                (WorkDomain::EncoderCandidates, 3),
                (WorkDomain::UniversalLambda, 4),
            ]),
            Err("unlike work units")
        );
    }

    #[test]
    fn learned_context_never_changes_the_universal_projection() {
        let (training, calibration) = fixture();
        let learned = learn_representation(&training, &calibration, &concepts(), &spec());
        let learned_points = (0..learned.accounting.candidates_evaluated)
            .map(|index| ((index % 5 + 1) as u32, index + 1));
        let mut interleaved = InterleavedDovetail::new(learned_points);
        let mut universal = Vec::new();
        while universal.len() < 128 {
            let point = interleaved.next_labeled().unwrap();
            if point.lane == ResourceLane::Universal {
                universal.push((point.syntax_size, point.evaluation_fuel));
            }
        }
        let original = crate::universal::Dovetail::default()
            .take(128)
            .collect::<Vec<_>>();
        assert_eq!(universal, original);
    }
}
