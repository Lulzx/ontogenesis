//! U3: bounded discovery of an initial-algebra-like recursive structure.
//!
//! The candidate language is ordinary closed lambda calculus plus the generic
//! typed normal-form enumerator. It has no production for recursion, folding,
//! fixed points, algebras, or their universal equation. The verifier declares
//! only the finite action F(X)=1+X and tests relational equations externally.

use crate::{
    nbe,
    term::{self, Term},
    transform,
    typed::{self, Atom, Type},
    universal,
};
use std::cmp::Reverse;
use std::collections::{BTreeSet, HashSet};
use std::rc::Rc;

const M: u32 = 30;
const A: u32 = 31;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResultEncoding {
    ChurchBoolean,
    ChurchNumeral,
    ChurchList,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlgebraRole {
    EvenParity,
    Count,
    Reconstruct,
    OddParity,
    DoubleCount,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EvidenceDerivation {
    pub target_derived: bool,
    pub output_derived: bool,
    pub trace_derived: bool,
    pub ancestor_ids: BTreeSet<String>,
}

#[derive(Clone, Debug)]
pub struct AlgebraEvidence {
    pub id: String,
    pub duplicate_group: String,
    pub role: AlgebraRole,
    pub result: ResultEncoding,
    pub base: Rc<Term>,
    pub step: Rc<Term>,
    pub depths: Vec<u32>,
    pub recorded_epoch: u64,
    pub derivation: EvidenceDerivation,
    pub protected_annotation: i64,
}

#[derive(Clone, Debug)]
pub struct DiscoverySpec {
    pub freeze_epoch: u64,
    pub max_base_size: u32,
    pub max_step_size: u32,
    pub max_constructor_size: u32,
    pub max_generator_size: u32,
    pub max_mediator_size: u32,
    pub typed_cell_cap: usize,
    pub fuel: i64,
    pub complexity_price: u64,
    pub execution_price: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct U3Accounting {
    pub base_terms: u64,
    pub step_terms: u64,
    pub carrier_pairs: u64,
    pub constructor_terms: u64,
    pub generator_terms: u64,
    pub mediator_terms: u64,
    pub generated_candidates: u64,
    pub evaluated_candidates: u64,
    pub observation_checks: u64,
    pub equation_checks: u64,
    pub equivalence_checks: u64,
    pub rejected_unsafe: u64,
    pub rejected_nonunique: u64,
    pub max_base_size: u32,
    pub max_step_size: u32,
    pub max_constructor_size: u32,
    pub max_generator_size: u32,
    pub max_mediator_size: u32,
    pub fuel: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum U3WorkDomain {
    LambdaObservations,
    TypedProposals,
    UniversalResources,
    BehaviorExecutions,
}

pub fn aggregate_work(
    samples: &[(U3WorkDomain, u64)],
) -> Result<(U3WorkDomain, u64), &'static str> {
    let Some((domain, _)) = samples.first() else {
        return Err("empty work set");
    };
    if samples.iter().any(|(other, _)| other != domain) {
        return Err("unlike work units");
    }
    Ok((*domain, samples.iter().map(|(_, n)| *n).sum()))
}

#[derive(Clone, Debug)]
pub struct InitialStructure {
    /// A discovered inhabitant witnessing the anonymous carrier representation.
    pub carrier_witness: Rc<Term>,
    pub carrier_step: Rc<Term>,
    /// Discovered `F(M)->M`; primitive wrappers have been fully expanded.
    pub constructor: Rc<Term>,
    /// Discovered program mapping an algebra's base/step to its mediator.
    pub generator: Rc<Term>,
    /// Declared executable action F(h):F(M)->F(A), frozen before protection.
    pub f_action: Rc<Term>,
    pub freeze_epoch: u64,
    pub observational_fuel: i64,
    pub mediator_boundary: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum U3Termination {
    Discovered,
    ExhaustedBoundary,
    InvalidEvidence,
}

#[derive(Clone, Debug)]
pub struct DiscoveryReport {
    pub structure: Option<InitialStructure>,
    pub accounting: U3Accounting,
    pub calibration_commutes: bool,
    pub calibration_unique: bool,
    pub syntax_baseline_found: bool,
    pub recurrence_subtree_found: bool,
    pub charged_discovery_cost: u64,
    pub termination: U3Termination,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UniquenessReport {
    pub valid_mediators: usize,
    pub equivalence_classes: usize,
    pub unique: bool,
    pub generated: u64,
    pub checks: u64,
    pub exhaustive_within_size: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DownstreamTask {
    DoubleCarrier,
    IdentityControl,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchMeasurement {
    pub solved: bool,
    pub size: Option<u32>,
    pub proposals: u64,
    pub generated_candidates: u64,
    pub observation_checks: u64,
    pub max_size: u32,
    pub termination: U3Termination,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostGeometry {
    pub before: u64,
    pub after: u64,
    pub discovery_charge: u64,
    pub protected_uses: u64,
    pub net_gain: i128,
    pub composition_overhead: u64,
    pub triangle_samples: usize,
    pub triangle_holds: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcquisitionDecision {
    pub retained: bool,
    pub utility: i128,
    pub learned_budget_units: u32,
    pub ranking: Vec<&'static str>,
}

fn app(function: Rc<Term>, args: impl IntoIterator<Item = Rc<Term>>) -> Rc<Term> {
    args.into_iter().fold(function, term::app)
}

fn normalize(value: &Rc<Term>, fuel: i64) -> Option<Rc<Term>> {
    nbe::normalize(&Rc::new(Vec::new()), value, &mut nbe::Fuel(fuel)).ok()
}

fn equivalent(a: &Rc<Term>, b: &Rc<Term>, fuel: i64) -> bool {
    normalize(a, fuel)
        .zip(normalize(b, fuel))
        .is_some_and(|(a, b)| a == b)
}

fn leading_lambdas(value: &Rc<Term>) -> usize {
    let mut cursor = value.as_ref();
    let mut count = 0;
    while let Term::Lam(body) = cursor {
        count += 1;
        cursor = body;
    }
    count
}

fn closed_terms(max_size: u32, minimum_lambdas: usize) -> Vec<Rc<Term>> {
    (1..=max_size)
        .flat_map(|size| universal::terms_exact(size, 0, &[]))
        .filter(|candidate| leading_lambdas(candidate) >= minimum_lambdas)
        .collect()
}

fn expand_primitives(value: &Rc<Term>) -> Rc<Term> {
    match value.as_ref() {
        Term::Prim(body) => expand_primitives(body),
        Term::Lam(body) => term::lam(expand_primitives(body)),
        Term::App(f, a) => term::app(expand_primitives(f), expand_primitives(a)),
        Term::Var(_) | Term::Free(_) => value.clone(),
    }
}

fn layer_base() -> Rc<Term> {
    term::lam(term::lam(term::var(1)))
}
fn layer_step(value: Rc<Term>) -> Rc<Term> {
    term::lam(term::lam(term::app(
        term::var(0),
        transform::shift(&value, 2, 0),
    )))
}

pub fn declared_f_action() -> Rc<Term> {
    // λh.λlayer.λleft.λright. layer left (λm. right (h m))
    let mapped = term::lam(term::app(
        term::var(1),
        term::app(term::var(4), term::var(0)),
    ));
    let body = app(term::var(2), [term::var(1), mapped]);
    (0..4).fold(body, |body, _| term::lam(body))
}

fn algebra_term(evidence: &AlgebraEvidence) -> Rc<Term> {
    term::lam(app(
        term::var(0),
        [evidence.base.clone(), evidence.step.clone()],
    ))
}

fn carrier_value(structure: &InitialStructure, depth: u32) -> Rc<Term> {
    (0..depth).fold(structure.carrier_witness.clone(), |value, _| {
        term::app(structure.carrier_step.clone(), value)
    })
}

fn generated_mediator(structure: &InitialStructure, evidence: &AlgebraEvidence) -> Rc<Term> {
    app(
        structure.generator.clone(),
        [evidence.base.clone(), evidence.step.clone()],
    )
}

fn expected(role: AlgebraRole, depth: u32) -> Rc<Term> {
    match role {
        AlgebraRole::EvenParity => bool_term(depth % 2 == 0),
        AlgebraRole::OddParity => bool_term(depth % 2 == 1),
        AlgebraRole::Count => church_numeral(depth),
        AlgebraRole::DoubleCount => church_numeral(depth * 2),
        AlgebraRole::Reconstruct => church_list(&vec![1; depth as usize]),
    }
}

pub fn commutes(
    structure: &InitialStructure,
    evidence: &AlgebraEvidence,
    checks: &mut u64,
) -> bool {
    if evidence.depths.is_empty()
        || !transform::is_closed(&evidence.base)
        || !transform::is_closed(&evidence.step)
    {
        return false;
    }
    let h = generated_mediator(structure, evidence);
    let alpha = algebra_term(evidence);
    evidence.depths.iter().all(|depth| {
        let predecessor = carrier_value(structure, depth.saturating_sub(1));
        let layer = if *depth == 0 {
            layer_base()
        } else {
            layer_step(predecessor)
        };
        let lhs = term::app(
            h.clone(),
            term::app(structure.constructor.clone(), layer.clone()),
        );
        let rhs = term::app(
            alpha.clone(),
            app(structure.f_action.clone(), [h.clone(), layer]),
        );
        *checks += 2;
        equivalent(&lhs, &rhs, structure.observational_fuel)
            && equivalent(
                &lhs,
                &expected(evidence.role, *depth),
                structure.observational_fuel,
            )
    })
}

fn visible_evidence<'a>(
    records: &'a [AlgebraEvidence],
    freeze: u64,
    protected: &BTreeSet<String>,
) -> Vec<&'a AlgebraEvidence> {
    records
        .iter()
        .filter(|record| {
            record.recorded_epoch <= freeze
                && !record.derivation.target_derived
                && !record.derivation.output_derived
                && !record.derivation.trace_derived
                && record.derivation.ancestor_ids.is_empty()
                && !protected.contains(&record.id)
                && protected.iter().all(|id| record.duplicate_group != *id)
        })
        .collect()
}

fn mediator_type() -> Type {
    let a = Type::Atom(A);
    let carrier = Type::arrow(
        Type::arrow(a.clone(), a.clone()),
        Type::arrow(a.clone(), a.clone()),
    );
    Type::arrow(
        a.clone(),
        Type::arrow(Type::arrow(a.clone(), a.clone()), Type::arrow(carrier, a)),
    )
}

fn constructor_type() -> Type {
    let m = Type::Atom(M);
    let layer = Type::arrow(
        m.clone(),
        Type::arrow(Type::arrow(m.clone(), m.clone()), m.clone()),
    );
    Type::arrow(layer, m)
}

fn constructor_for(
    base: &Rc<Term>,
    step: &Rc<Term>,
    spec: &DiscoverySpec,
) -> Option<(Rc<Term>, u64)> {
    let m = Type::Atom(M);
    let atoms = [
        Atom {
            body: base.clone(),
            ty: m.clone(),
        },
        Atom {
            body: step.clone(),
            ty: Type::arrow(m.clone(), m),
        },
    ];
    typed::find_closed(
        &constructor_type(),
        &atoms,
        spec.max_constructor_size,
        spec.typed_cell_cap,
        |candidate| {
            let base_case =
                equivalent(&term::app(candidate.clone(), layer_base()), base, spec.fuel);
            let mut value = base.clone();
            base_case
                && (0..=4).all(|_| {
                    let observed = term::app(candidate.clone(), layer_step(value.clone()));
                    let expected = term::app(step.clone(), value.clone());
                    let holds = equivalent(&observed, &expected, spec.fuel);
                    value = expected;
                    holds
                })
        },
    )
    .map(|found| (expand_primitives(&found.term), found.generated))
}

fn carrier_separates(base: &Rc<Term>, step: &Rc<Term>, fuel: i64, checks: &mut u64) -> bool {
    if !transform::is_closed(base) || !transform::is_closed(step) || base == step {
        return false;
    }
    let mut classes = HashSet::new();
    let mut value = base.clone();
    for _ in 0..=9 {
        *checks += 1;
        let Some(normal) = normalize(&value, fuel) else {
            return false;
        };
        classes.insert(normal.as_ref().clone());
        value = term::app(step.clone(), value);
    }
    classes.len() == 10
}

fn generator_candidates(spec: &DiscoverySpec) -> typed::Enumeration {
    typed::enumerate_closed(
        &mediator_type(),
        &[],
        spec.max_generator_size,
        spec.typed_cell_cap,
    )
}

fn candidate_structure(
    base: Rc<Term>,
    step: Rc<Term>,
    constructor: Rc<Term>,
    generator: Rc<Term>,
    spec: &DiscoverySpec,
) -> InitialStructure {
    InitialStructure {
        carrier_witness: base,
        carrier_step: step,
        constructor,
        generator,
        f_action: declared_f_action(),
        freeze_epoch: spec.freeze_epoch,
        observational_fuel: spec.fuel,
        mediator_boundary: spec.max_mediator_size,
    }
}

pub fn bounded_uniqueness(
    structure: &InitialStructure,
    evidence: &AlgebraEvidence,
    spec: &DiscoverySpec,
) -> UniquenessReport {
    let a = Type::Atom(A);
    let carrier = Type::arrow(
        Type::arrow(a.clone(), a.clone()),
        Type::arrow(a.clone(), a.clone()),
    );
    let atoms = [
        Atom {
            body: evidence.base.clone(),
            ty: a.clone(),
        },
        Atom {
            body: evidence.step.clone(),
            ty: Type::arrow(a.clone(), a.clone()),
        },
    ];
    let enumeration = typed::enumerate_closed(
        &Type::arrow(carrier, a),
        &atoms,
        spec.max_mediator_size,
        spec.typed_cell_cap,
    );
    let mut valid = 0;
    let mut checks = 0;
    let mut classes = HashSet::new();
    for candidate in &enumeration.terms {
        let equations = (0u32..=9).all(|depth| {
            let predecessor = carrier_value(structure, depth.saturating_sub(1));
            let layer = if depth == 0 {
                layer_base()
            } else {
                layer_step(predecessor)
            };
            let lhs = term::app(
                candidate.clone(),
                term::app(structure.constructor.clone(), layer.clone()),
            );
            let rhs = term::app(
                algebra_term(evidence),
                app(structure.f_action.clone(), [candidate.clone(), layer]),
            );
            checks += 1;
            equivalent(&lhs, &rhs, spec.fuel)
        });
        if equations {
            valid += 1;
            let observations = (0u32..=9)
                .map(|depth| {
                    normalize(
                        &term::app(candidate.clone(), carrier_value(structure, depth)),
                        spec.fuel,
                    )
                })
                .collect::<Option<Vec<_>>>();
            if let Some(values) = observations {
                classes.insert(
                    values
                        .into_iter()
                        .map(|x| x.as_ref().clone())
                        .collect::<Vec<_>>(),
                );
            }
        }
    }
    UniquenessReport {
        valid_mediators: valid,
        equivalence_classes: classes.len(),
        unique: valid > 0 && classes.len() == 1 && !enumeration.truncated,
        generated: enumeration.generated,
        checks,
        exhaustive_within_size: !enumeration.truncated,
    }
}

fn nontrivial_closed_subterms(value: &Rc<Term>, out: &mut HashSet<Term>) {
    if value.size() >= 4 && transform::is_closed(value) {
        out.insert(value.as_ref().clone());
    }
    match value.as_ref() {
        Term::Lam(body) => nontrivial_closed_subterms(body, out),
        Term::App(f, a) => {
            nontrivial_closed_subterms(f, out);
            nontrivial_closed_subterms(a, out);
        }
        Term::Var(_) | Term::Free(_) | Term::Prim(_) => {}
    }
}

pub fn syntax_baseline(records: &[AlgebraEvidence]) -> bool {
    let mut intersection: Option<HashSet<Term>> = None;
    for program in records.iter().flat_map(|e| [&e.base, &e.step]) {
        let Some(normal) = normalize(program, 100_000) else {
            return false;
        };
        let mut terms = HashSet::new();
        nontrivial_closed_subterms(&normal, &mut terms);
        intersection = Some(match intersection {
            None => terms,
            Some(old) => old.intersection(&terms).cloned().collect(),
        });
    }
    intersection.is_some_and(|x| !x.is_empty())
}

pub fn recurrence_subtree_baseline(structure: &InitialStructure) -> bool {
    let values = (0..=4)
        .map(|n| normalize(&carrier_value(structure, n), structure.observational_fuel).unwrap())
        .collect::<Vec<_>>();
    values
        .windows(2)
        .all(|window| transform::subterms(&window[1]).contains(&window[0]))
}

pub fn discover(
    training: &[AlgebraEvidence],
    calibration: &[AlgebraEvidence],
    protected_ids: &BTreeSet<String>,
    spec: &DiscoverySpec,
) -> DiscoveryReport {
    let training = visible_evidence(training, spec.freeze_epoch, protected_ids);
    let calibration = visible_evidence(calibration, spec.freeze_epoch, protected_ids);
    let all = training
        .iter()
        .chain(calibration.iter())
        .copied()
        .collect::<Vec<_>>();
    let mut accounting = U3Accounting {
        max_base_size: spec.max_base_size,
        max_step_size: spec.max_step_size,
        max_constructor_size: spec.max_constructor_size,
        max_generator_size: spec.max_generator_size,
        max_mediator_size: spec.max_mediator_size,
        fuel: spec.fuel,
        ..Default::default()
    };
    let encodings = all.iter().map(|e| e.result).collect::<BTreeSet<_>>();
    if training.is_empty()
        || calibration.is_empty()
        || encodings.len() < 3
        || all.iter().any(|e| e.depths.is_empty())
    {
        return DiscoveryReport {
            structure: None,
            accounting,
            calibration_commutes: false,
            calibration_unique: false,
            syntax_baseline_found: false,
            recurrence_subtree_found: false,
            charged_discovery_cost: u64::MAX,
            termination: U3Termination::InvalidEvidence,
        };
    }
    let bases = closed_terms(spec.max_base_size, 2);
    let steps = closed_terms(spec.max_step_size, 3);
    let generators = generator_candidates(spec);
    accounting.base_terms = bases.len() as u64;
    accounting.step_terms = steps.len() as u64;
    accounting.generator_terms = generators.generated;
    accounting.generated_candidates = generators.generated;
    let mut found = None;
    'search: for base in &bases {
        for step in &steps {
            accounting.carrier_pairs += 1;
            if !carrier_separates(base, step, spec.fuel, &mut accounting.observation_checks) {
                accounting.rejected_unsafe += 1;
                continue;
            }
            let Some((constructor, generated)) = constructor_for(base, step, spec) else {
                continue;
            };
            accounting.constructor_terms += generated;
            for generator in &generators.terms {
                accounting.evaluated_candidates += 1;
                let proposed = candidate_structure(
                    base.clone(),
                    step.clone(),
                    constructor.clone(),
                    generator.clone(),
                    spec,
                );
                if all
                    .iter()
                    .all(|e| commutes(&proposed, e, &mut accounting.equation_checks))
                {
                    let unique = calibration.iter().all(|e| {
                        let report = bounded_uniqueness(&proposed, e, spec);
                        accounting.mediator_terms += report.generated;
                        accounting.equivalence_checks += report.checks;
                        report.unique
                    });
                    if unique {
                        found = Some(proposed);
                        break 'search;
                    }
                    accounting.rejected_nonunique += 1;
                }
            }
        }
    }
    let syntax_baseline_found =
        syntax_baseline(&all.iter().map(|e| (*e).clone()).collect::<Vec<_>>());
    let recurrence_subtree_found = found.as_ref().is_some_and(recurrence_subtree_baseline);
    let mut calibration_checks = 0;
    let calibration_commutes = found.as_ref().is_some_and(|s| {
        calibration
            .iter()
            .all(|e| commutes(s, e, &mut calibration_checks))
    });
    accounting.equation_checks += calibration_checks;
    let calibration_unique = found.as_ref().is_some_and(|s| {
        calibration.iter().all(|e| {
            let result = bounded_uniqueness(s, e, spec);
            accounting.mediator_terms += result.generated;
            accounting.equivalence_checks += result.checks;
            result.unique
        })
    });
    let charged_discovery_cost = found.as_ref().map_or(u64::MAX, |s| {
        u64::from(
            s.carrier_witness.size()
                + s.carrier_step.size()
                + s.constructor.size()
                + s.generator.size()
                + s.f_action.size(),
        )
        .saturating_mul(spec.complexity_price)
        .saturating_add(
            accounting
                .observation_checks
                .saturating_add(accounting.equation_checks)
                .saturating_add(accounting.equivalence_checks)
                .saturating_mul(spec.execution_price),
        )
    });
    DiscoveryReport {
        termination: if found.is_some() {
            U3Termination::Discovered
        } else {
            U3Termination::ExhaustedBoundary
        },
        structure: found,
        accounting,
        calibration_commutes,
        calibration_unique,
        syntax_baseline_found,
        recurrence_subtree_found,
        charged_discovery_cost,
    }
}

pub fn default_spec() -> DiscoverySpec {
    DiscoverySpec {
        freeze_epoch: 1,
        max_base_size: 3,
        max_step_size: 10,
        max_constructor_size: 6,
        max_generator_size: 8,
        max_mediator_size: 8,
        typed_cell_cap: 50_000,
        fuel: 2_000_000,
        complexity_price: 10,
        execution_price: 1,
    }
}

fn bool_term(value: bool) -> Rc<Term> {
    if value {
        term::lam(term::lam(term::var(1)))
    } else {
        term::lam(term::lam(term::var(0)))
    }
}
pub fn boolean_not() -> Rc<Term> {
    term::lam(app(term::var(0), [bool_term(false), bool_term(true)]))
}
pub fn church_numeral(n: u32) -> Rc<Term> {
    let body = (0..n).fold(term::var(0), |b, _| term::app(term::var(1), b));
    term::lam(term::lam(body))
}
pub fn numeral_successor() -> Rc<Term> {
    term::lam(term::lam(term::lam(term::app(
        term::var(1),
        app(term::var(2), [term::var(1), term::var(0)]),
    ))))
}
pub fn church_list(values: &[u32]) -> Rc<Term> {
    let body = values.iter().rev().fold(term::var(0), |tail, v| {
        app(term::var(1), [church_numeral(*v), tail])
    });
    term::lam(term::lam(body))
}
fn list_prepend_one() -> Rc<Term> {
    term::lam(term::lam(term::lam(app(
        term::var(1),
        [
            church_numeral(1),
            app(term::var(2), [term::var(1), term::var(0)]),
        ],
    ))))
}

pub fn sample_algebras() -> (
    Vec<AlgebraEvidence>,
    Vec<AlgebraEvidence>,
    Vec<AlgebraEvidence>,
) {
    let make = |id: &str, role, result, base, step, depths| AlgebraEvidence {
        id: id.into(),
        duplicate_group: id.into(),
        role,
        result,
        base,
        step,
        depths,
        recorded_epoch: 1,
        derivation: EvidenceDerivation::default(),
        protected_annotation: 0,
    };
    let training = vec![
        make(
            "train-parity",
            AlgebraRole::EvenParity,
            ResultEncoding::ChurchBoolean,
            bool_term(true),
            boolean_not(),
            vec![0, 1, 2, 3],
        ),
        make(
            "train-count",
            AlgebraRole::Count,
            ResultEncoding::ChurchNumeral,
            church_numeral(0),
            numeral_successor(),
            vec![0, 1, 2, 3],
        ),
    ];
    let calibration = vec![make(
        "cal-reconstruct",
        AlgebraRole::Reconstruct,
        ResultEncoding::ChurchList,
        church_list(&[]),
        list_prepend_one(),
        vec![0, 1, 2, 3, 4],
    )];
    let protected = vec![
        make(
            "held-odd-composed",
            AlgebraRole::OddParity,
            ResultEncoding::ChurchBoolean,
            bool_term(false),
            boolean_not(),
            vec![5, 7, 9],
        ),
        make(
            "held-double-count",
            AlgebraRole::DoubleCount,
            ResultEncoding::ChurchNumeral,
            church_numeral(0),
            term::lam(term::app(
                numeral_successor(),
                term::app(numeral_successor(), term::var(0)),
            )),
            vec![5, 7, 9],
        ),
    ];
    (training, calibration, protected)
}

pub fn protected_view(
    e: &AlgebraEvidence,
) -> (
    String,
    String,
    AlgebraRole,
    ResultEncoding,
    Rc<Term>,
    Rc<Term>,
    Vec<u32>,
) {
    (
        e.id.clone(),
        e.duplicate_group.clone(),
        e.role,
        e.result,
        e.base.clone(),
        e.step.clone(),
        e.depths.clone(),
    )
}

fn prim(body: &Rc<Term>) -> Rc<Term> {
    Rc::new(Term::Prim(body.clone()))
}
fn distinct_primitive_count(t: &Rc<Term>) -> usize {
    fn go(t: &Rc<Term>, s: &mut HashSet<Term>) {
        match t.as_ref() {
            Term::Prim(b) => {
                s.insert(b.as_ref().clone());
            }
            Term::Lam(b) => go(b, s),
            Term::App(f, a) => {
                go(f, s);
                go(a, s)
            }
            Term::Var(_) | Term::Free(_) => {}
        }
    }
    let mut s = HashSet::new();
    go(t, &mut s);
    s.len()
}

fn contains_primitive_body(t: &Rc<Term>, wanted: &Rc<Term>) -> bool {
    match t.as_ref() {
        Term::Prim(body) => body == wanted,
        Term::Lam(body) => contains_primitive_body(body, wanted),
        Term::App(f, a) => contains_primitive_body(f, wanted) || contains_primitive_body(a, wanted),
        Term::Var(_) | Term::Free(_) => false,
    }
}

fn instantiated_generator_type() -> Type {
    let m = Type::Atom(M);
    Type::arrow(
        m.clone(),
        Type::arrow(Type::arrow(m.clone(), m.clone()), Type::arrow(m.clone(), m)),
    )
}
fn structure_atoms(s: &InitialStructure) -> Vec<Atom> {
    let m = Type::Atom(M);
    vec![
        Atom {
            body: s.generator.clone(),
            ty: instantiated_generator_type(),
        },
        Atom {
            body: s.carrier_witness.clone(),
            ty: m.clone(),
        },
        Atom {
            body: s.carrier_step.clone(),
            ty: Type::arrow(m.clone(), m),
        },
    ]
}

fn downstream_holds(
    candidate: &Rc<Term>,
    task: DownstreamTask,
    s: &InitialStructure,
    checks: &mut u64,
) -> bool {
    (0..=9).all(|depth| {
        let input = carrier_value(s, depth);
        let expected = match task {
            DownstreamTask::DoubleCarrier => carrier_value(s, depth * 2),
            DownstreamTask::IdentityControl => input.clone(),
        };
        *checks += 1;
        equivalent(
            &term::app(candidate.clone(), input),
            &expected,
            s.observational_fuel,
        )
    })
}

pub fn measure_downstream(
    task: DownstreamTask,
    s: &InitialStructure,
    acquired: bool,
    max_size: u32,
    cap: usize,
) -> SearchMeasurement {
    let target = Type::arrow(Type::Atom(M), Type::Atom(M));
    let atoms = if acquired {
        structure_atoms(s)
    } else {
        Vec::new()
    };
    let enumeration = typed::enumerate_closed(&target, &atoms, max_size, cap);
    let mut terms = enumeration.terms;
    if acquired {
        terms.sort_by_key(|t| {
            (
                Reverse(contains_primitive_body(t, &s.generator)),
                t.size(),
                Reverse(distinct_primitive_count(t)),
                term::show(t),
            )
        });
    }
    let mut checks = 0;
    for (index, candidate) in terms.iter().enumerate() {
        if downstream_holds(candidate, task, s, &mut checks) {
            return SearchMeasurement {
                solved: true,
                size: Some(candidate.size()),
                proposals: index as u64 + 1,
                generated_candidates: enumeration.generated,
                observation_checks: checks,
                max_size,
                termination: U3Termination::Discovered,
            };
        }
    }
    SearchMeasurement {
        solved: false,
        size: None,
        proposals: terms.len() as u64,
        generated_candidates: enumeration.generated,
        observation_checks: checks,
        max_size,
        termination: U3Termination::ExhaustedBoundary,
    }
}

pub fn measure_uniform(
    task: DownstreamTask,
    s: &InitialStructure,
    max_size: u32,
    cap: usize,
) -> SearchMeasurement {
    let enumeration = typed::enumerate_closed(
        &Type::arrow(Type::Atom(M), Type::Atom(M)),
        &structure_atoms(s),
        max_size,
        cap,
    );
    let mut checks = 0;
    for (index, candidate) in enumeration.terms.iter().enumerate() {
        if downstream_holds(candidate, task, s, &mut checks) {
            return SearchMeasurement {
                solved: true,
                size: Some(candidate.size()),
                proposals: index as u64 + 1,
                generated_candidates: enumeration.generated,
                observation_checks: checks,
                max_size,
                termination: U3Termination::Discovered,
            };
        }
    }
    SearchMeasurement {
        solved: false,
        size: None,
        proposals: enumeration.terms.len() as u64,
        generated_candidates: enumeration.generated,
        observation_checks: checks,
        max_size,
        termination: U3Termination::ExhaustedBoundary,
    }
}

pub fn measure_irrelevant(
    task: DownstreamTask,
    s: &InitialStructure,
    max_size: u32,
    cap: usize,
) -> SearchMeasurement {
    let m = Type::Atom(M);
    let atom = Atom {
        body: identity(),
        ty: Type::arrow(m.clone(), m),
    };
    let enumeration = typed::enumerate_closed(
        &Type::arrow(Type::Atom(M), Type::Atom(M)),
        &[atom],
        max_size,
        cap,
    );
    let mut checks = 0;
    for (index, candidate) in enumeration.terms.iter().enumerate() {
        if downstream_holds(candidate, task, s, &mut checks) {
            return SearchMeasurement {
                solved: true,
                size: Some(candidate.size()),
                proposals: index as u64 + 1,
                generated_candidates: enumeration.generated,
                observation_checks: checks,
                max_size,
                termination: U3Termination::Discovered,
            };
        }
    }
    SearchMeasurement {
        solved: false,
        size: None,
        proposals: enumeration.terms.len() as u64,
        generated_candidates: enumeration.generated,
        observation_checks: checks,
        max_size,
        termination: U3Termination::ExhaustedBoundary,
    }
}

pub fn measure_oracle(task: DownstreamTask, s: &InitialStructure) -> SearchMeasurement {
    let candidate = match task {
        DownstreamTask::IdentityControl => identity(),
        DownstreamTask::DoubleCarrier => {
            let step = term::lam(term::app(
                prim(&s.carrier_step),
                term::app(prim(&s.carrier_step), term::var(0)),
            ));
            app(prim(&s.generator), [prim(&s.carrier_witness), step])
        }
    };
    let mut checks = 0;
    let solved = downstream_holds(&candidate, task, s, &mut checks);
    SearchMeasurement {
        solved,
        size: solved.then_some(candidate.size()),
        proposals: 1,
        generated_candidates: 1,
        observation_checks: checks,
        max_size: candidate.size(),
        termination: if solved {
            U3Termination::Discovered
        } else {
            U3Termination::ExhaustedBoundary
        },
    }
}

pub fn measure_pure_universal(
    task: DownstreamTask,
    s: &InitialStructure,
    max_size: u32,
) -> SearchMeasurement {
    let mut proposals = 0;
    let mut checks = 0;
    for size in 1..=max_size {
        for candidate in universal::terms_exact(size, 0, &[]) {
            proposals += 1;
            if downstream_holds(&candidate, task, s, &mut checks) {
                return SearchMeasurement {
                    solved: true,
                    size: Some(size),
                    proposals,
                    generated_candidates: proposals,
                    observation_checks: checks,
                    max_size,
                    termination: U3Termination::Discovered,
                };
            }
        }
    }
    SearchMeasurement {
        solved: false,
        size: None,
        proposals,
        generated_candidates: proposals,
        observation_checks: checks,
        max_size,
        termination: U3Termination::ExhaustedBoundary,
    }
}

pub fn measure_recurrence_only(
    structure: &InitialStructure,
    protected: &[AlgebraEvidence],
) -> SearchMeasurement {
    // Identity is the exact recurrence-only fit for the discovered count
    // representation. It is then subjected to the heterogeneous frozen law.
    let candidate = identity();
    let recurrence_structure = InitialStructure {
        generator: term::lam(term::lam(candidate)),
        ..structure.clone()
    };
    let mut checks = 0;
    let solved = !protected.is_empty()
        && protected
            .iter()
            .all(|e| commutes(&recurrence_structure, e, &mut checks));
    SearchMeasurement {
        solved,
        size: solved.then_some(2),
        proposals: 1,
        generated_candidates: 1,
        observation_checks: checks,
        max_size: 2,
        termination: if solved {
            U3Termination::Discovered
        } else {
            U3Termination::ExhaustedBoundary
        },
    }
}

pub fn cost_geometry(
    discovery_charge: u64,
    before: &[SearchMeasurement],
    after: &[SearchMeasurement],
    protected_uses: u64,
) -> CostGeometry {
    let before = before
        .iter()
        .map(|x| x.observation_checks)
        .sum::<u64>()
        .saturating_mul(protected_uses);
    let after = after
        .iter()
        .map(|x| x.observation_checks)
        .sum::<u64>()
        .saturating_mul(protected_uses);
    let composition_overhead = 1;
    let samples = sampled_composition_costs();
    let triangle_holds = samples
        .iter()
        .all(|(ab, bc, ac)| *ac <= ab.saturating_add(*bc).saturating_add(composition_overhead));
    CostGeometry {
        before,
        after,
        discovery_charge,
        protected_uses,
        net_gain: i128::from(before) - i128::from(after) - i128::from(discovery_charge),
        composition_overhead,
        triangle_samples: samples.len(),
        triangle_holds,
    }
}
pub fn decide_acquisition(g: &CostGeometry) -> AcquisitionDecision {
    let retained = g.net_gain > 0 && g.triangle_holds;
    AcquisitionDecision {
        retained,
        utility: g.net_gain,
        learned_budget_units: u32::from(retained),
        ranking: if retained {
            vec!["invented-initial-structure"]
        } else {
            Vec::new()
        },
    }
}
fn morphism_distance(target: &Rc<Term>) -> u64 {
    let probes = [bool_term(false), bool_term(true)];
    let mut checks = 0;
    for size in 1..=12 {
        for candidate in universal::terms_exact(size, 0, &[]) {
            let agrees = probes.iter().all(|p| {
                checks += 1;
                equivalent(
                    &term::app(candidate.clone(), p.clone()),
                    &term::app(target.clone(), p.clone()),
                    100_000,
                )
            });
            if agrees {
                return checks;
            }
        }
    }
    u64::MAX / 4
}
pub fn sampled_composition_costs() -> Vec<(u64, u64, u64)> {
    let id = morphism_distance(&identity());
    let not = morphism_distance(&boolean_not());
    vec![(id, not, not), (not, id, not), (not, not, id)]
}
pub fn identity() -> Rc<Term> {
    term::lam(term::var(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universal::{Dovetail, InterleavedDovetail, ResourceLane};

    fn discovered() -> (
        DiscoverySpec,
        Vec<AlgebraEvidence>,
        InitialStructure,
        DiscoveryReport,
    ) {
        let (training, calibration, protected) = sample_algebras();
        let ids = protected.iter().map(|e| e.id.clone()).collect();
        let spec = default_spec();
        let report = discover(&training, &calibration, &ids, &spec);
        let structure = report.structure.clone().expect("U3 discovery");
        (spec, protected, structure, report)
    }

    #[test]
    fn discovers_initial_like_structure_and_extrapolates() {
        let (spec, protected, structure, report) = discovered();
        assert!(report.calibration_commutes && report.calibration_unique);
        assert!(!report.syntax_baseline_found && !report.recurrence_subtree_found);
        assert!([
            &structure.carrier_witness,
            &structure.carrier_step,
            &structure.constructor,
            &structure.generator
        ]
        .into_iter()
        .all(|t| transform::is_closed(t) && universal::in_language(t, 0, &[])));
        for evidence in &protected {
            let mut checks = 0;
            assert!(commutes(&structure, evidence, &mut checks));
            let unique = bounded_uniqueness(&structure, evidence, &spec);
            assert!(unique.unique && unique.exhaustive_within_size);
            assert!(unique.valid_mediators > 0);
        }
    }

    #[test]
    fn recurrence_only_fit_fails_the_relational_law() {
        let (spec, protected, structure, _) = discovered();
        for depth in 0..=4 {
            let value = carrier_value(&structure, depth);
            assert!(equivalent(
                &term::app(identity(), value.clone()),
                &value,
                spec.fuel
            ));
        }
        let bad = InitialStructure {
            generator: term::lam(term::lam(identity())),
            ..structure
        };
        let mut checks = 0;
        assert!(!commutes(&bad, &protected[0], &mut checks));
    }

    #[test]
    fn hidden_state_has_existence_without_uniqueness() {
        let pair = |n: Rc<Term>, tag: Rc<Term>| {
            term::lam(term::app(
                term::app(term::var(0), transform::shift(&n, 1, 0)),
                transform::shift(&tag, 1, 0),
            ))
        };
        let first = term::lam(term::app(term::var(0), term::lam(term::lam(term::var(1)))));
        let second = term::lam(term::app(term::var(0), term::lam(term::lam(term::var(0)))));
        let zero = pair(church_numeral(0), bool_term(false));
        let extra = pair(church_numeral(0), bool_term(true));
        let step = term::lam(pair(
            term::app(numeral_successor(), term::app(first.clone(), term::var(0))),
            term::app(second.clone(), term::var(0)),
        ));
        let h0 = first.clone();
        let h1 = term::lam(app(
            term::app(second.clone(), term::var(0)),
            [
                term::app(numeral_successor(), term::app(first.clone(), term::var(0))),
                term::app(first.clone(), term::var(0)),
            ],
        ));
        for h in [&h0, &h1] {
            let mut value = zero.clone();
            for depth in 0..=5 {
                assert!(equivalent(
                    &term::app(h.clone(), value.clone()),
                    &church_numeral(depth),
                    500_000
                ));
                value = term::app(step.clone(), value);
            }
            let extra_step = term::app(step.clone(), extra.clone());
            assert!(equivalent(
                &term::app(h.clone(), extra_step),
                &term::app(numeral_successor(), term::app(h.clone(), extra.clone())),
                500_000,
            ));
        }
        assert!(!equivalent(
            &term::app(h0, extra.clone()),
            &term::app(h1, extra),
            100_000
        ));
    }

    #[test]
    fn leakage_mutation_and_fallback_are_invariant() {
        let (training, calibration, protected) = sample_algebras();
        let ids = protected
            .iter()
            .map(|e| e.id.clone())
            .collect::<BTreeSet<_>>();
        let spec = default_spec();
        let clean = discover(&training, &calibration, &ids, &spec);
        let mut poisoned = training.clone();
        for (kind, index) in (0..5).zip(0..) {
            let mut item = protected[0].clone();
            item.id = format!("poison-{index}");
            match kind {
                0 => item.derivation.target_derived = true,
                1 => item.derivation.output_derived = true,
                2 => item.derivation.trace_derived = true,
                3 => {
                    item.derivation
                        .ancestor_ids
                        .insert("held-odd-composed".into());
                }
                _ => item.recorded_epoch = 2,
            }
            poisoned.push(item);
        }
        let mut duplicate = training[0].clone();
        duplicate.id = "duplicate".into();
        duplicate.duplicate_group = "held-odd-composed".into();
        poisoned.push(duplicate);
        let contaminated = discover(&poisoned, &calibration, &ids, &spec);
        let a = clean.structure.as_ref().unwrap();
        let b = contaminated.structure.as_ref().unwrap();
        assert_eq!(
            (
                &a.carrier_witness,
                &a.carrier_step,
                &a.constructor,
                &a.generator
            ),
            (
                &b.carrier_witness,
                &b.carrier_step,
                &b.constructor,
                &b.generator
            )
        );
        assert_eq!(clean.accounting, contaminated.accounting);
        let mut mutated_protected = protected.clone();
        mutated_protected[0].base = church_numeral(99);
        mutated_protected[0].step = identity();
        mutated_protected[0].depths = vec![99];
        let mutated_ids = mutated_protected
            .iter()
            .map(|e| e.id.clone())
            .collect::<BTreeSet<_>>();
        let output_mutated = discover(&training, &calibration, &mutated_ids, &spec);
        let output_structure = output_mutated.structure.as_ref().unwrap();
        assert_eq!(
            (
                &a.carrier_witness,
                &a.carrier_step,
                &a.constructor,
                &a.generator
            ),
            (
                &output_structure.carrier_witness,
                &output_structure.carrier_step,
                &output_structure.constructor,
                &output_structure.generator
            )
        );
        assert_eq!(clean.accounting, output_mutated.accounting);
        let baseline = measure_downstream(DownstreamTask::DoubleCarrier, a, true, 16, 50_000);
        for e in &protected {
            let before = protected_view(e);
            let mut changed = e.clone();
            changed.protected_annotation = i64::MAX;
            assert_eq!(protected_view(&changed), before);
            assert_eq!(
                measure_downstream(DownstreamTask::DoubleCarrier, a, true, 16, 50_000),
                baseline
            );
            assert_eq!(
                bounded_uniqueness(a, e, &spec),
                bounded_uniqueness(a, &changed, &spec)
            );
        }
        let mut schedule = InterleavedDovetail::new((0..128).map(|i| ((i % 9 + 1) as u32, i + 1)));
        let mut projection = Vec::new();
        while projection.len() < 256 {
            let p = schedule.next_labeled().unwrap();
            if p.lane == ResourceLane::Universal {
                projection.push((p.syntax_size, p.evaluation_fuel));
            }
        }
        assert_eq!(
            projection,
            Dovetail::default().take(256).collect::<Vec<_>>()
        );
        assert!(universal::scheduled_stage(u32::MAX, i64::MAX as u64).is_some());
    }

    #[test]
    fn truncation_economics_controls_and_units_are_explicit() {
        let (mut spec, protected, structure, report) = discovered();
        spec.typed_cell_cap = 1;
        let truncated = bounded_uniqueness(&structure, &protected[0], &spec);
        assert!(!truncated.exhaustive_within_size && !truncated.unique);
        let raw = measure_downstream(DownstreamTask::DoubleCarrier, &structure, false, 16, 50_000);
        let learned =
            measure_downstream(DownstreamTask::DoubleCarrier, &structure, true, 16, 50_000);
        let uniform = measure_uniform(DownstreamTask::DoubleCarrier, &structure, 16, 50_000);
        let irrelevant = measure_irrelevant(DownstreamTask::DoubleCarrier, &structure, 16, 50_000);
        let oracle = measure_oracle(DownstreamTask::DoubleCarrier, &structure);
        let universal_run = measure_pure_universal(DownstreamTask::DoubleCarrier, &structure, 8);
        assert!(
            !raw.solved
                && learned.solved
                && uniform.solved
                && !irrelevant.solved
                && oracle.solved
                && !universal_run.solved
        );
        assert!(
            learned.proposals < uniform.proposals
                && learned.observation_checks < uniform.observation_checks
        );
        let geometry = cost_geometry(
            report.charged_discovery_cost,
            &[uniform],
            &[learned],
            100_000,
        );
        assert!(
            geometry.net_gain > 0
                && geometry.triangle_holds
                && decide_acquisition(&geometry).retained
        );
        let base = measure_downstream(
            DownstreamTask::IdentityControl,
            &structure,
            false,
            4,
            50_000,
        );
        let acquired =
            measure_downstream(DownstreamTask::IdentityControl, &structure, true, 4, 50_000);
        assert!(base.solved && acquired.solved && acquired.proposals >= base.proposals);
        let useless = cost_geometry(
            100,
            std::slice::from_ref(&base),
            std::slice::from_ref(&base),
            1_000,
        );
        assert!(!decide_acquisition(&useless).retained);
        assert_eq!(
            aggregate_work(&[
                (U3WorkDomain::TypedProposals, 2),
                (U3WorkDomain::TypedProposals, 3)
            ]),
            Ok((U3WorkDomain::TypedProposals, 5))
        );
        assert_eq!(
            aggregate_work(&[
                (U3WorkDomain::TypedProposals, 2),
                (U3WorkDomain::LambdaObservations, 3)
            ]),
            Err("unlike work units")
        );
        let expanded = term::lam(term::app(
            identity(),
            term::app(structure.carrier_step.clone(), term::var(0)),
        ));
        assert!(expanded.size() > structure.carrier_step.size());
        for depth in 0..=9 {
            let value = carrier_value(&structure, depth);
            assert!(equivalent(
                &term::app(expanded.clone(), value.clone()),
                &term::app(structure.carrier_step.clone(), value),
                spec.fuel,
            ));
        }
        let mut under = default_spec();
        under.max_step_size = 9;
        let (t, c, p) = sample_algebras();
        let ids = p.iter().map(|e| e.id.clone()).collect();
        assert!(discover(&t, &c, &ids, &under).structure.is_none());
    }

    #[test]
    fn wrong_constant_open_divergent_and_incomplete_controls_fail() {
        let (spec, protected, structure, _) = discovered();
        let evidence = &protected[0];
        let wrong_constructor = InitialStructure {
            constructor: term::lam(structure.carrier_witness.clone()),
            ..structure.clone()
        };
        let missing_step = InitialStructure {
            constructor: term::lam(term::app(term::var(0), structure.carrier_witness.clone())),
            ..structure.clone()
        };
        let constant_generator = InitialStructure {
            generator: term::lam(term::lam(term::lam(term::var(2)))),
            ..structure.clone()
        };
        let omega_half = term::lam(term::app(term::var(0), term::var(0)));
        let omega = term::app(omega_half.clone(), omega_half);
        let divergent = InitialStructure {
            generator: term::lam(term::lam(omega)),
            ..structure.clone()
        };
        for bad in [
            &wrong_constructor,
            &missing_step,
            &constant_generator,
            &divergent,
        ] {
            let mut checks = 0;
            assert!(!commutes(bad, evidence, &mut checks));
        }
        // The step parameter is syntactically present but computationally dead.
        let dead_reference = InitialStructure {
            generator: term::lam(term::lam(term::lam(term::var(2)))),
            ..structure.clone()
        };
        let mut dead_checks = 0;
        assert!(!commutes(&dead_reference, evidence, &mut dead_checks));
        assert!(!transform::is_closed(&term::var(0)));
        let mut incomplete = evidence.clone();
        incomplete.depths.clear();
        let mut checks = 0;
        assert!(!commutes(&structure, &incomplete, &mut checks));
        let mut collision_checks = 0;
        assert!(!carrier_separates(
            &structure.carrier_witness,
            &identity(),
            spec.fuel,
            &mut collision_checks
        ));
        let arbitrary_base = term::lam(term::lam(term::var(1)));
        let arbitrary_step = term::lam(term::app(
            identity(),
            term::app(identity(), arbitrary_base.clone()),
        ));
        assert_eq!(arbitrary_step.size(), structure.carrier_step.size());
        let mut arbitrary_checks = 0;
        assert!(!carrier_separates(
            &arbitrary_base,
            &arbitrary_step,
            spec.fuel,
            &mut arbitrary_checks,
        ));
        let (training, mut calibration, held) = sample_algebras();
        calibration[0].result = ResultEncoding::ChurchNumeral;
        let ids = held.iter().map(|e| e.id.clone()).collect();
        assert!(
            discover(&training[..1], &calibration, &ids, &default_spec())
                .structure
                .is_none()
        );
    }
}
