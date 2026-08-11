//! Bounded discovery of a product-like universal factorization in pure lambda calculus.
//!
//! The discovery grammar is the ordinary closed untyped lambda language. It
//! contains no constructor, projection, tuple, product, cone, or mediator
//! production. An external finite observational harness asks only whether
//! independently enumerated programs make two families of equations commute
//! and whether all bounded solutions are extensionally equal.

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

const BIT: u32 = 0;
const CARRIER: u32 = 1;
const SOURCE: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceInterface {
    Bit,
    Ternary,
    ChurchList,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EvidenceDerivation {
    pub target_derived: bool,
    pub output_derived: bool,
    pub ancestor_ids: BTreeSet<String>,
}

#[derive(Clone, Debug)]
pub struct ConeEvidence {
    pub id: String,
    pub duplicate_group: String,
    pub source: SourceInterface,
    pub probes: Vec<Rc<Term>>,
    pub channel_a: Rc<Term>,
    pub channel_b: Rc<Term>,
    pub recorded_epoch: u64,
    pub derivation: EvidenceDerivation,
    /// Verification-only annotation. No discovery or replay function reads it.
    pub protected_annotation: i64,
}

#[derive(Clone, Debug)]
pub struct DiscoverySpec {
    pub freeze_epoch: u64,
    pub max_carrier_size: u32,
    pub max_observer_size: u32,
    pub max_generator_size: u32,
    pub max_mediator_size: u32,
    pub typed_cell_cap: usize,
    pub fuel: i64,
    pub complexity_price: u64,
    pub execution_price: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct U1Accounting {
    pub carrier_terms: u64,
    pub observer_terms: u64,
    pub factorization_candidates: u64,
    pub generator_terms: u64,
    pub mediator_terms: u64,
    pub normalization_checks: u64,
    pub equivalence_checks: u64,
    pub rejected_unsafe: u64,
    pub rejected_nonunique: u64,
    pub max_carrier_size: u32,
    pub max_observer_size: u32,
    pub max_generator_size: u32,
    pub max_mediator_size: u32,
    pub fuel: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum U1WorkDomain {
    LambdaObservations,
    TypedProposals,
    UniversalResources,
    BehaviorBank,
}

pub fn aggregate_work(
    samples: &[(U1WorkDomain, u64)],
) -> Result<(U1WorkDomain, u64), &'static str> {
    let Some((domain, _)) = samples.first() else {
        return Err("empty work set");
    };
    if samples.iter().any(|(next, _)| next != domain) {
        return Err("unlike work units");
    }
    Ok((*domain, samples.iter().map(|(_, work)| *work).sum()))
}

#[derive(Clone, Debug)]
pub struct UniversalStructure {
    /// Anonymous two-input carrier constructor discovered as a closed lambda term.
    pub carrier: Rc<Term>,
    pub observe_a: Rc<Term>,
    pub observe_b: Rc<Term>,
    /// Higher-order program mapping two arrows to their shared mediator.
    pub generator: Rc<Term>,
    pub freeze_epoch: u64,
    pub observational_fuel: i64,
    pub mediator_boundary: u32,
}

#[derive(Clone, Debug)]
pub struct DiscoveryReport {
    pub structure: Option<UniversalStructure>,
    pub accounting: U1Accounting,
    pub calibration_commutes: bool,
    pub calibration_unique: bool,
    pub syntax_baseline_found: bool,
    pub charged_discovery_cost: u64,
    pub termination: U1Termination,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum U1Termination {
    Discovered,
    ExhaustedBoundary,
    InvalidEvidence,
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
    Swap,
    MapBoth,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchMeasurement {
    pub solved: bool,
    pub size: Option<u32>,
    pub proposals: u64,
    pub generated_candidates: u64,
    pub observation_checks: u64,
    pub max_size: u32,
    pub termination: U1Termination,
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

pub fn decide_acquisition(geometry: &CostGeometry) -> AcquisitionDecision {
    let retained = geometry.net_gain > 0 && geometry.triangle_holds;
    AcquisitionDecision {
        retained,
        utility: geometry.net_gain,
        learned_budget_units: if retained { 1 } else { 0 },
        ranking: if retained {
            vec!["invented-universal-structure"]
        } else {
            Vec::new()
        },
    }
}

fn bool_term(value: bool) -> Rc<Term> {
    if value {
        term::lam(term::lam(term::var(1)))
    } else {
        term::lam(term::lam(term::var(0)))
    }
}

fn normalize(value: &Rc<Term>, fuel: i64) -> Option<Rc<Term>> {
    nbe::normalize(&Rc::new(Vec::new()), value, &mut nbe::Fuel(fuel)).ok()
}

fn apply(function: Rc<Term>, arguments: impl IntoIterator<Item = Rc<Term>>) -> Rc<Term> {
    arguments.into_iter().fold(function, term::app)
}

fn observationally_equal(a: &Rc<Term>, b: &Rc<Term>, fuel: i64) -> bool {
    normalize(a, fuel)
        .zip(normalize(b, fuel))
        .is_some_and(|(left, right)| left == right)
}

fn field_probes() -> Vec<(Rc<Term>, Rc<Term>)> {
    let values = [church_numeral(0), church_numeral(1), church_numeral(2)];
    values
        .iter()
        .flat_map(|a| values.iter().map(move |b| (a.clone(), b.clone())))
        .collect()
}

fn visible_evidence<'a>(
    records: &'a [ConeEvidence],
    freeze_epoch: u64,
    protected_ids: &BTreeSet<String>,
) -> Vec<&'a ConeEvidence> {
    records
        .iter()
        .filter(|record| {
            record.recorded_epoch <= freeze_epoch
                && !record.derivation.target_derived
                && !record.derivation.output_derived
                && record.derivation.ancestor_ids.is_empty()
                && !protected_ids.contains(&record.id)
                && protected_ids.iter().all(|id| record.duplicate_group != *id)
        })
        .collect()
}

fn closed_terms(max_size: u32, leading_lambdas: usize) -> Vec<Rc<Term>> {
    let mut result = Vec::new();
    for size in 1..=max_size {
        for candidate in universal::terms_exact(size, 0, &[]) {
            let mut cursor = candidate.as_ref();
            let mut lambdas = 0;
            while let Term::Lam(body) = cursor {
                lambdas += 1;
                cursor = body;
            }
            if lambdas >= leading_lambdas {
                result.push(candidate);
            }
        }
    }
    result
}

fn separates_probe_values(
    carrier: &Rc<Term>,
    probes: &[(Rc<Term>, Rc<Term>)],
    fuel: i64,
    checks: &mut u64,
) -> bool {
    let mut normal = HashSet::new();
    for (a, b) in probes {
        *checks += 1;
        let Some(value) = normalize(&apply(carrier.clone(), [a.clone(), b.clone()]), fuel) else {
            return false;
        };
        normal.insert((*value).clone());
    }
    let distinct_inputs = probes
        .iter()
        .map(|(a, b)| (a.as_ref().clone(), b.as_ref().clone()))
        .collect::<HashSet<_>>()
        .len();
    normal.len() == distinct_inputs
}

fn observer_role(
    carrier: &Rc<Term>,
    observer: &Rc<Term>,
    probes: &[(Rc<Term>, Rc<Term>)],
    fuel: i64,
    checks: &mut u64,
) -> (bool, bool) {
    let mut is_a = true;
    let mut is_b = true;
    for (a, b) in probes {
        let encoded = apply(carrier.clone(), [a.clone(), b.clone()]);
        let observed = term::app(observer.clone(), encoded);
        *checks += 2;
        is_a &= observationally_equal(&observed, &a, fuel);
        is_b &= observationally_equal(&observed, &b, fuel);
    }
    (is_a, is_b)
}

fn observed_factorization_pairs(
    cones: &[&ConeEvidence],
    fuel: i64,
    checks: &mut u64,
) -> Option<Vec<(Rc<Term>, Rc<Term>)>> {
    let mut pairs = Vec::new();
    for cone in cones {
        for probe in &cone.probes {
            *checks += 2;
            let a = normalize(&term::app(cone.channel_a.clone(), probe.clone()), fuel)?;
            let b = normalize(&term::app(cone.channel_b.clone(), probe.clone()), fuel)?;
            if !pairs
                .iter()
                .any(|(old_a, old_b)| old_a == &a && old_b == &b)
            {
                pairs.push((a, b));
            }
        }
    }
    Some(pairs)
}

fn generator_type() -> Type {
    let bit = Type::Atom(BIT);
    let source = Type::Atom(SOURCE);
    let carrier = Type::Atom(CARRIER);
    Type::arrow(
        Type::arrow(source.clone(), bit.clone()),
        Type::arrow(
            Type::arrow(source.clone(), bit),
            Type::arrow(source, carrier),
        ),
    )
}

fn carrier_atom(carrier: &Rc<Term>, concrete: bool) -> Atom {
    let bit = Type::Atom(BIT);
    let carrier_type = if concrete {
        Type::arrow(
            Type::arrow(bit.clone(), Type::arrow(bit.clone(), bit.clone())),
            bit.clone(),
        )
    } else {
        Type::Atom(CARRIER)
    };
    Atom {
        body: carrier.clone(),
        ty: Type::arrow(bit.clone(), Type::arrow(bit, carrier_type)),
    }
}

fn generated_mediator(generator: &Rc<Term>, cone: &ConeEvidence) -> Rc<Term> {
    apply(
        generator.clone(),
        [cone.channel_a.clone(), cone.channel_b.clone()],
    )
}

pub fn commutes(structure: &UniversalStructure, cone: &ConeEvidence, checks: &mut u64) -> bool {
    if cone.probes.is_empty()
        || ![&cone.channel_a, &cone.channel_b]
            .into_iter()
            .all(|term| transform::is_closed(term))
    {
        return false;
    }
    let mediator = generated_mediator(&structure.generator, cone);
    cone.probes.iter().all(|probe| {
        let value = term::app(mediator.clone(), probe.clone());
        let left = term::app(structure.observe_a.clone(), value.clone());
        let right = term::app(structure.observe_b.clone(), value);
        let expected_a = term::app(cone.channel_a.clone(), probe.clone());
        let expected_b = term::app(cone.channel_b.clone(), probe.clone());
        *checks += 2;
        observationally_equal(&left, &expected_a, structure.observational_fuel)
            && observationally_equal(&right, &expected_b, structure.observational_fuel)
    })
}

fn mediator_observations(
    mediator: &Rc<Term>,
    cone: &ConeEvidence,
    fuel: i64,
) -> Option<Vec<Rc<Term>>> {
    cone.probes
        .iter()
        .map(|probe| normalize(&term::app(mediator.clone(), probe.clone()), fuel))
        .collect()
}

pub fn bounded_uniqueness(
    structure: &UniversalStructure,
    cone: &ConeEvidence,
    spec: &DiscoverySpec,
) -> UniquenessReport {
    let bit = Type::Atom(BIT);
    let source = Type::Atom(SOURCE);
    let carrier = Type::Atom(CARRIER);
    let atoms = vec![
        carrier_atom(&structure.carrier, false),
        Atom {
            body: cone.channel_a.clone(),
            ty: Type::arrow(source.clone(), bit.clone()),
        },
        Atom {
            body: cone.channel_b.clone(),
            ty: Type::arrow(source.clone(), bit),
        },
    ];
    let enumeration = typed::enumerate_closed(
        &Type::arrow(source, carrier),
        &atoms,
        spec.max_mediator_size,
        spec.typed_cell_cap,
    );
    let mut checks = 0;
    let mut classes = HashSet::new();
    let mut valid = 0;
    for mediator in &enumeration.terms {
        let equations_hold = cone.probes.iter().all(|probe| {
            let value = term::app(mediator.clone(), probe.clone());
            let expected_a = term::app(cone.channel_a.clone(), probe.clone());
            let expected_b = term::app(cone.channel_b.clone(), probe.clone());
            checks += 2;
            observationally_equal(
                &term::app(structure.observe_a.clone(), value.clone()),
                &expected_a,
                spec.fuel,
            ) && observationally_equal(
                &term::app(structure.observe_b.clone(), value),
                &expected_b,
                spec.fuel,
            )
        });
        if equations_hold {
            valid += 1;
            if let Some(observations) = mediator_observations(mediator, cone, spec.fuel) {
                classes.insert(
                    observations
                        .into_iter()
                        .map(|t| (*t).clone())
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

fn find_generator(
    carrier: &Rc<Term>,
    cones: &[&ConeEvidence],
    observe_a: &Rc<Term>,
    observe_b: &Rc<Term>,
    spec: &DiscoverySpec,
) -> (Option<typed::Found>, u64) {
    let mut checks = 0;
    let found = typed::find_closed(
        &generator_type(),
        &[carrier_atom(carrier, false)],
        spec.max_generator_size,
        spec.typed_cell_cap,
        |candidate| {
            let structure = UniversalStructure {
                carrier: carrier.clone(),
                observe_a: observe_a.clone(),
                observe_b: observe_b.clone(),
                generator: candidate.clone(),
                freeze_epoch: spec.freeze_epoch,
                observational_fuel: spec.fuel,
                mediator_boundary: spec.max_mediator_size,
            };
            cones
                .iter()
                .all(|cone| commutes(&structure, cone, &mut checks))
        },
    );
    (found, checks)
}

fn nontrivial_closed_subterms(term: &Rc<Term>, out: &mut HashSet<Term>) {
    if term.size() >= 4 && transform::is_closed(term) {
        out.insert(term.as_ref().clone());
    }
    match term.as_ref() {
        Term::Lam(body) => nontrivial_closed_subterms(body, out),
        Term::App(function, argument) => {
            nontrivial_closed_subterms(function, out);
            nontrivial_closed_subterms(argument, out);
        }
        Term::Var(_) | Term::Free(_) | Term::Prim(_) => {}
    }
}

pub fn syntax_factorization_baseline(records: &[ConeEvidence]) -> bool {
    let mut intersection: Option<HashSet<Term>> = None;
    for record in records {
        for morphism in [&record.channel_a, &record.channel_b] {
            let Some(normal) = normalize(morphism, 100_000) else {
                return false;
            };
            let mut subterms = HashSet::new();
            nontrivial_closed_subterms(&normal, &mut subterms);
            intersection = Some(match intersection {
                None => subterms,
                Some(current) => current.intersection(&subterms).cloned().collect(),
            });
        }
    }
    intersection.is_some_and(|terms| !terms.is_empty())
}

pub fn discover(
    training: &[ConeEvidence],
    calibration: &[ConeEvidence],
    protected_ids: &BTreeSet<String>,
    spec: &DiscoverySpec,
) -> DiscoveryReport {
    let training = visible_evidence(training, spec.freeze_epoch, protected_ids);
    let calibration = visible_evidence(calibration, spec.freeze_epoch, protected_ids);
    let mut accounting = U1Accounting {
        max_carrier_size: spec.max_carrier_size,
        max_observer_size: spec.max_observer_size,
        max_generator_size: spec.max_generator_size,
        max_mediator_size: spec.max_mediator_size,
        fuel: spec.fuel,
        ..Default::default()
    };
    let carriers = closed_terms(spec.max_carrier_size, 2);
    let observers = closed_terms(spec.max_observer_size, 1);
    accounting.carrier_terms = carriers.len() as u64;
    accounting.observer_terms = observers.len() as u64;
    let source_kinds = training
        .iter()
        .map(|cone| cone.source)
        .collect::<BTreeSet<_>>();
    let all_evidence = training
        .iter()
        .chain(calibration.iter())
        .copied()
        .collect::<Vec<_>>();
    let observed_pairs = observed_factorization_pairs(
        &all_evidence,
        spec.fuel,
        &mut accounting.normalization_checks,
    )
    .unwrap_or_default();
    if training.is_empty()
        || calibration.is_empty()
        || source_kinds.len() < 2
        || observed_pairs.len() < 4
    {
        return DiscoveryReport {
            structure: None,
            accounting,
            calibration_commutes: false,
            calibration_unique: false,
            syntax_baseline_found: false,
            charged_discovery_cost: u64::MAX,
            termination: U1Termination::InvalidEvidence,
        };
    }
    let independent_pairs = field_probes();
    let mut best: Option<(u64, UniversalStructure, u64)> = None;
    for carrier in carriers {
        if !separates_probe_values(
            &carrier,
            &observed_pairs,
            spec.fuel,
            &mut accounting.normalization_checks,
        ) || !separates_probe_values(
            &carrier,
            &independent_pairs,
            spec.fuel,
            &mut accounting.normalization_checks,
        ) {
            continue;
        }
        let mut left = Vec::new();
        let mut right = Vec::new();
        for observer in &observers {
            let (a, b) = observer_role(
                &carrier,
                observer,
                &observed_pairs,
                spec.fuel,
                &mut accounting.normalization_checks,
            );
            let (independent_a, independent_b) = observer_role(
                &carrier,
                observer,
                &independent_pairs,
                spec.fuel,
                &mut accounting.normalization_checks,
            );
            if a && independent_a {
                left.push(observer.clone());
            }
            if b && independent_b {
                right.push(observer.clone());
            }
        }
        for observe_a in &left {
            for observe_b in &right {
                accounting.factorization_candidates += 1;
                let (found, generator_checks) =
                    find_generator(&carrier, &all_evidence, observe_a, observe_b, spec);
                accounting.normalization_checks = accounting
                    .normalization_checks
                    .saturating_add(generator_checks);
                let Some(found) = found else {
                    continue;
                };
                accounting.generator_terms =
                    accounting.generator_terms.saturating_add(found.generated);
                let structure = UniversalStructure {
                    carrier: carrier.clone(),
                    observe_a: observe_a.clone(),
                    observe_b: observe_b.clone(),
                    generator: found.term,
                    freeze_epoch: spec.freeze_epoch,
                    observational_fuel: spec.fuel,
                    mediator_boundary: spec.max_mediator_size,
                };
                let mut unique = true;
                let mut mediator_work = 0u64;
                for cone in &calibration {
                    let result = bounded_uniqueness(&structure, cone, spec);
                    accounting.mediator_terms =
                        accounting.mediator_terms.saturating_add(result.generated);
                    accounting.equivalence_checks =
                        accounting.equivalence_checks.saturating_add(result.checks);
                    mediator_work = mediator_work.saturating_add(result.checks);
                    if !result.unique {
                        accounting.rejected_nonunique += 1;
                        unique = false;
                        break;
                    }
                }
                if !unique {
                    continue;
                }
                let complexity = u64::from(
                    structure.carrier.size()
                        + structure.observe_a.size()
                        + structure.observe_b.size()
                        + structure.generator.size(),
                );
                let charge = complexity
                    .saturating_mul(spec.complexity_price)
                    .saturating_add(
                        accounting
                            .normalization_checks
                            .saturating_add(mediator_work)
                            .saturating_mul(spec.execution_price),
                    );
                if best.as_ref().is_none_or(|(cost, _, _)| charge < *cost) {
                    best = Some((charge, structure, found.generated));
                }
            }
        }
    }
    let syntax_baseline_found = syntax_factorization_baseline(
        &training
            .iter()
            .chain(calibration.iter())
            .map(|record| (*record).clone())
            .collect::<Vec<_>>(),
    );
    let structure = best.map(|(_, structure, _)| structure);
    let mut calibration_checks = 0;
    let calibration_commutes = structure.as_ref().is_some_and(|structure| {
        calibration
            .iter()
            .all(|cone| commutes(structure, cone, &mut calibration_checks))
    });
    accounting.normalization_checks = accounting
        .normalization_checks
        .saturating_add(calibration_checks);
    let calibration_unique = structure.as_ref().is_some_and(|structure| {
        calibration.iter().all(|cone| {
            let result = bounded_uniqueness(structure, cone, spec);
            accounting.mediator_terms = accounting.mediator_terms.saturating_add(result.generated);
            accounting.equivalence_checks =
                accounting.equivalence_checks.saturating_add(result.checks);
            result.unique
        })
    });
    let charged_discovery_cost = structure.as_ref().map_or(u64::MAX, |structure| {
        u64::from(
            structure.carrier.size()
                + structure.observe_a.size()
                + structure.observe_b.size()
                + structure.generator.size(),
        )
        .saturating_mul(spec.complexity_price)
        .saturating_add(
            accounting
                .normalization_checks
                .saturating_add(accounting.equivalence_checks)
                .saturating_mul(spec.execution_price),
        )
    });
    let termination = if structure.is_some() {
        U1Termination::Discovered
    } else {
        U1Termination::ExhaustedBoundary
    };
    DiscoveryReport {
        structure,
        accounting,
        calibration_commutes,
        calibration_unique,
        syntax_baseline_found,
        charged_discovery_cost,
        termination,
    }
}

fn concrete_types() -> (Type, Type) {
    let bit = Type::Atom(BIT);
    let handler = Type::arrow(bit.clone(), Type::arrow(bit.clone(), bit.clone()));
    let carrier = Type::arrow(handler, bit.clone());
    (bit, carrier)
}

fn downstream_target(task: DownstreamTask) -> Type {
    let (bit, carrier_type) = concrete_types();
    match task {
        DownstreamTask::Swap => Type::arrow(carrier_type.clone(), carrier_type),
        DownstreamTask::MapBoth => Type::arrow(
            Type::arrow(bit.clone(), bit),
            Type::arrow(carrier_type.clone(), carrier_type),
        ),
    }
}

fn structure_atoms(structure: &UniversalStructure) -> Vec<Atom> {
    let (bit, carrier_type) = concrete_types();
    vec![
        carrier_atom(&structure.carrier, true),
        Atom {
            body: structure.observe_a.clone(),
            ty: Type::arrow(carrier_type.clone(), bit.clone()),
        },
        Atom {
            body: structure.observe_b.clone(),
            ty: Type::arrow(carrier_type, bit),
        },
    ]
}

fn downstream_holds(
    candidate: &Rc<Term>,
    task: DownstreamTask,
    structure: &UniversalStructure,
    fuel: i64,
    checks: &mut u64,
) -> bool {
    for (a, b) in field_probes() {
        let input = apply(structure.carrier.clone(), [a.clone(), b.clone()]);
        let output = match task {
            DownstreamTask::Swap => term::app(candidate.clone(), input),
            DownstreamTask::MapBoth => apply(candidate.clone(), [church_successor(), input]),
        };
        let expected_a = match task {
            DownstreamTask::Swap => b.clone(),
            DownstreamTask::MapBoth => term::app(church_successor(), a.clone()),
        };
        let expected_b = match task {
            DownstreamTask::Swap => a.clone(),
            DownstreamTask::MapBoth => term::app(church_successor(), b.clone()),
        };
        let expected = apply(structure.carrier.clone(), [expected_a, expected_b]);
        *checks += 1;
        if !observationally_equal(&output, &expected, fuel) {
            return false;
        }
    }
    true
}

pub fn measure_downstream(
    task: DownstreamTask,
    structure: &UniversalStructure,
    acquired: bool,
    max_size: u32,
    cell_cap: usize,
) -> SearchMeasurement {
    let target = downstream_target(task);
    let atoms = if acquired {
        structure_atoms(structure)
    } else {
        Vec::new()
    };
    let universal_enumeration = typed::enumerate_closed(&target, &[], max_size, cell_cap);
    let mut generated_candidates = universal_enumeration.generated;
    let universal_terms = universal_enumeration.terms;
    let mut learned = if acquired {
        let enumeration = typed::enumerate_closed(&target, &atoms, max_size, cell_cap);
        generated_candidates = generated_candidates.saturating_add(enumeration.generated);
        enumeration
            .terms
            .into_iter()
            .filter(|candidate| primitive_count(candidate) > 0)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    // This is task-independent ontology compression priority: programs using
    // more acquired structure per syntax node enter the learned lane first.
    learned.sort_by_key(|candidate| {
        (
            Reverse(distinct_primitive_count(candidate)),
            candidate
                .size()
                .saturating_sub(primitive_count(candidate) * 2),
            candidate.size(),
            term::show(candidate),
        )
    });
    let mut learned = learned.into_iter();
    let mut universal = universal_terms.into_iter();
    let mut observation_checks = 0;
    let mut proposals = 0;
    let mut found = None;
    loop {
        let mut progressed = false;
        if let Some(candidate) = learned.next() {
            progressed = true;
            proposals += 1;
            if downstream_holds(
                &candidate,
                task,
                structure,
                structure.observational_fuel,
                &mut observation_checks,
            ) {
                found = Some(candidate);
                break;
            }
        }
        if let Some(candidate) = universal.next() {
            progressed = true;
            proposals += 1;
            if downstream_holds(
                &candidate,
                task,
                structure,
                structure.observational_fuel,
                &mut observation_checks,
            ) {
                found = Some(candidate);
                break;
            }
        }
        if !progressed {
            break;
        }
    }
    SearchMeasurement {
        solved: found.is_some(),
        size: found.as_ref().map(|found| found.size()),
        proposals,
        generated_candidates,
        observation_checks,
        max_size,
        termination: if found.is_some() {
            U1Termination::Discovered
        } else {
            U1Termination::ExhaustedBoundary
        },
    }
}

pub fn measure_universal_downstream(
    task: DownstreamTask,
    structure: &UniversalStructure,
    max_size: u32,
) -> SearchMeasurement {
    let mut proposals = 0;
    let mut checks = 0;
    for size in 1..=max_size {
        for candidate in universal::terms_exact(size, 0, &[]) {
            proposals += 1;
            if downstream_holds(
                &candidate,
                task,
                structure,
                structure.observational_fuel,
                &mut checks,
            ) {
                return SearchMeasurement {
                    solved: true,
                    size: Some(size),
                    proposals,
                    generated_candidates: proposals,
                    observation_checks: checks,
                    max_size,
                    termination: U1Termination::Discovered,
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
        termination: U1Termination::ExhaustedBoundary,
    }
}

pub fn measure_universal_with_structure(
    task: DownstreamTask,
    structure: &UniversalStructure,
    max_size: u32,
) -> SearchMeasurement {
    let learned_enumeration = typed::enumerate_closed(
        &downstream_target(task),
        &structure_atoms(structure),
        max_size,
        50_000,
    );
    let generated_candidates = learned_enumeration.generated;
    let mut learned = learned_enumeration
        .terms
        .into_iter()
        .filter(|candidate| primitive_count(candidate) > 0)
        .collect::<Vec<_>>();
    learned.sort_by_key(|candidate| {
        (
            Reverse(distinct_primitive_count(candidate)),
            candidate
                .size()
                .saturating_sub(primitive_count(candidate) * 2),
            candidate.size(),
            term::show(candidate),
        )
    });
    let mut checks = 0;
    let mut proposals = 0;
    for candidate in &learned {
        proposals += 1;
        if downstream_holds(
            candidate,
            task,
            structure,
            structure.observational_fuel,
            &mut checks,
        ) {
            return SearchMeasurement {
                solved: true,
                size: Some(candidate.size()),
                proposals,
                generated_candidates,
                observation_checks: checks,
                max_size,
                termination: U1Termination::Discovered,
            };
        }
    }
    // Finite learned prefix exhausted: resume the original empty-alphabet
    // universal stream in its exact size/term order.
    for size in 1..=max_size {
        for candidate in universal::terms_exact(size, 0, &[]) {
            proposals += 1;
            if downstream_holds(
                &candidate,
                task,
                structure,
                structure.observational_fuel,
                &mut checks,
            ) {
                return SearchMeasurement {
                    solved: true,
                    size: Some(size),
                    proposals,
                    generated_candidates: generated_candidates.saturating_add(proposals),
                    observation_checks: checks,
                    max_size,
                    termination: U1Termination::Discovered,
                };
            }
        }
    }
    SearchMeasurement {
        solved: false,
        size: None,
        proposals,
        generated_candidates: generated_candidates.saturating_add(proposals),
        observation_checks: checks,
        max_size,
        termination: U1Termination::ExhaustedBoundary,
    }
}

pub fn measure_uniform_downstream(
    task: DownstreamTask,
    structure: &UniversalStructure,
    max_size: u32,
    cell_cap: usize,
) -> SearchMeasurement {
    let enumeration = typed::enumerate_closed(
        &downstream_target(task),
        &structure_atoms(structure),
        max_size,
        cell_cap,
    );
    let mut observation_checks = 0;
    for (index, candidate) in enumeration.terms.iter().enumerate() {
        if downstream_holds(
            candidate,
            task,
            structure,
            structure.observational_fuel,
            &mut observation_checks,
        ) {
            return SearchMeasurement {
                solved: true,
                size: Some(candidate.size()),
                proposals: index as u64 + 1,
                generated_candidates: enumeration.generated,
                observation_checks,
                max_size,
                termination: U1Termination::Discovered,
            };
        }
    }
    SearchMeasurement {
        solved: false,
        size: None,
        proposals: enumeration.terms.len() as u64,
        generated_candidates: enumeration.generated,
        observation_checks,
        max_size,
        termination: U1Termination::ExhaustedBoundary,
    }
}

pub fn measure_irrelevant_downstream(
    task: DownstreamTask,
    structure: &UniversalStructure,
    max_size: u32,
    cell_cap: usize,
) -> SearchMeasurement {
    let (bit, _) = concrete_types();
    let irrelevant = Atom {
        body: identity(),
        ty: Type::arrow(bit.clone(), bit),
    };
    let enumeration =
        typed::enumerate_closed(&downstream_target(task), &[irrelevant], max_size, cell_cap);
    let mut observation_checks = 0;
    for (index, candidate) in enumeration.terms.iter().enumerate() {
        if downstream_holds(
            candidate,
            task,
            structure,
            structure.observational_fuel,
            &mut observation_checks,
        ) {
            return SearchMeasurement {
                solved: true,
                size: Some(candidate.size()),
                proposals: index as u64 + 1,
                generated_candidates: enumeration.generated,
                observation_checks,
                max_size,
                termination: U1Termination::Discovered,
            };
        }
    }
    SearchMeasurement {
        solved: false,
        size: None,
        proposals: enumeration.terms.len() as u64,
        generated_candidates: enumeration.generated,
        observation_checks,
        max_size,
        termination: U1Termination::ExhaustedBoundary,
    }
}

fn primitive(body: &Rc<Term>) -> Rc<Term> {
    Rc::new(Term::Prim(body.clone()))
}

pub fn measure_oracle_downstream(
    task: DownstreamTask,
    structure: &UniversalStructure,
) -> SearchMeasurement {
    let candidate = match task {
        DownstreamTask::Swap => term::lam(apply(
            primitive(&structure.carrier),
            [
                term::app(primitive(&structure.observe_b), term::var(0)),
                term::app(primitive(&structure.observe_a), term::var(0)),
            ],
        )),
        DownstreamTask::MapBoth => term::lam(term::lam(apply(
            primitive(&structure.carrier),
            [
                term::app(
                    term::var(1),
                    term::app(primitive(&structure.observe_a), term::var(0)),
                ),
                term::app(
                    term::var(1),
                    term::app(primitive(&structure.observe_b), term::var(0)),
                ),
            ],
        ))),
    };
    let mut checks = 0;
    let solved = downstream_holds(
        &candidate,
        task,
        structure,
        structure.observational_fuel,
        &mut checks,
    );
    SearchMeasurement {
        solved,
        size: solved.then_some(candidate.size()),
        proposals: 1,
        generated_candidates: 1,
        observation_checks: checks,
        max_size: candidate.size(),
        termination: if solved {
            U1Termination::Discovered
        } else {
            U1Termination::ExhaustedBoundary
        },
    }
}

fn primitive_count(term: &Rc<Term>) -> u32 {
    match term.as_ref() {
        Term::Prim(_) => 1,
        Term::Lam(body) => primitive_count(body),
        Term::App(function, argument) => primitive_count(function) + primitive_count(argument),
        Term::Var(_) | Term::Free(_) => 0,
    }
}

fn distinct_primitive_count(term: &Rc<Term>) -> usize {
    fn visit(term: &Rc<Term>, bodies: &mut HashSet<Term>) {
        match term.as_ref() {
            Term::Prim(body) => {
                bodies.insert(body.as_ref().clone());
            }
            Term::Lam(body) => visit(body, bodies),
            Term::App(function, argument) => {
                visit(function, bodies);
                visit(argument, bodies);
            }
            Term::Var(_) | Term::Free(_) => {}
        }
    }
    let mut bodies = HashSet::new();
    visit(term, &mut bodies);
    bodies.len()
}

fn church_successor() -> Rc<Term> {
    // λn.λf.λx. f (n f x)
    term::lam(term::lam(term::lam(term::app(
        term::var(1),
        apply(term::var(2), [term::var(1), term::var(0)]),
    ))))
}

pub fn cost_geometry(
    discovery_charge: u64,
    before: &[SearchMeasurement],
    after: &[SearchMeasurement],
    protected_uses: u64,
) -> CostGeometry {
    let before_work = before.iter().map(|run| run.observation_checks).sum::<u64>();
    let after_work = after.iter().map(|run| run.observation_checks).sum::<u64>();
    let before_total = before_work.saturating_mul(protected_uses);
    let after_total = after_work.saturating_mul(protected_uses);
    let composition_overhead = 1;
    let samples = sampled_composition_costs();
    let triangle_holds = samples
        .iter()
        .all(|(ab, bc, ac)| *ac <= ab.saturating_add(*bc).saturating_add(composition_overhead));
    CostGeometry {
        before: before_total,
        after: after_total,
        discovery_charge,
        protected_uses,
        net_gain: i128::from(before_total) - i128::from(after_total) - i128::from(discovery_charge),
        composition_overhead,
        triangle_samples: samples.len(),
        triangle_holds,
    }
}

fn compose_morphisms(after: Rc<Term>, before: Rc<Term>) -> Rc<Term> {
    term::lam(term::app(after, term::app(before, term::var(0))))
}

fn morphism_distance(target: &Rc<Term>) -> u64 {
    let mut checks = 0;
    for size in 1..=12 {
        for candidate in universal::terms_exact(size, 0, &[]) {
            let agrees = [bool_term(false), bool_term(true)].iter().all(|probe| {
                checks += 1;
                observationally_equal(
                    &term::app(candidate.clone(), probe.clone()),
                    &term::app(target.clone(), probe.clone()),
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
    let id = identity();
    let not = boolean_not();
    let id_cost = morphism_distance(&id);
    let not_cost = morphism_distance(&not);
    debug_assert!([bool_term(false), bool_term(true)].iter().all(|probe| {
        observationally_equal(
            &term::app(compose_morphisms(not.clone(), not.clone()), probe.clone()),
            &term::app(id.clone(), probe.clone()),
            100_000,
        )
    }));
    vec![
        (id_cost, not_cost, not_cost),
        (not_cost, id_cost, not_cost),
        (not_cost, not_cost, id_cost),
    ]
}

pub fn protected_view(
    cone: &ConeEvidence,
) -> (
    String,
    String,
    SourceInterface,
    Vec<Rc<Term>>,
    Rc<Term>,
    Rc<Term>,
) {
    (
        cone.id.clone(),
        cone.duplicate_group.clone(),
        cone.source,
        cone.probes.clone(),
        cone.channel_a.clone(),
        cone.channel_b.clone(),
    )
}

pub fn default_spec() -> DiscoverySpec {
    DiscoverySpec {
        freeze_epoch: 1,
        max_carrier_size: 8,
        max_observer_size: 6,
        max_generator_size: 12,
        max_mediator_size: 10,
        typed_cell_cap: 50_000,
        fuel: 100_000,
        complexity_price: 10,
        execution_price: 1,
    }
}

pub fn identity() -> Rc<Term> {
    term::lam(term::var(0))
}

pub fn boolean_not() -> Rc<Term> {
    term::lam(term::app(
        term::app(term::var(0), bool_term(false)),
        bool_term(true),
    ))
}

pub fn church_numeral(n: u32) -> Rc<Term> {
    let body = (0..n).fold(term::var(0), |body, _| term::app(term::var(1), body));
    term::lam(term::lam(body))
}

pub fn is_zero() -> Rc<Term> {
    let ignore_false = term::lam(bool_term(false));
    term::lam(apply(term::var(0), [ignore_false, bool_term(true)]))
}

pub fn numeral_parity() -> Rc<Term> {
    let toggle = boolean_not();
    term::lam(apply(term::var(0), [toggle, bool_term(false)]))
}

pub fn church_list(values: &[u32]) -> Rc<Term> {
    let body = values.iter().rev().fold(term::var(0), |tail, value| {
        apply(term::var(1), [church_numeral(*value), tail])
    });
    term::lam(term::lam(body))
}

pub fn list_is_empty() -> Rc<Term> {
    // λxs. xs (λhead.λtail.false) true
    term::lam(apply(
        term::var(0),
        [term::lam(term::lam(bool_term(false))), bool_term(true)],
    ))
}

pub fn list_length_parity() -> Rc<Term> {
    // λxs. xs (λhead.λacc.not(acc)) false
    term::lam(apply(
        term::var(0),
        [
            term::lam(term::lam(term::app(boolean_not(), term::var(0)))),
            bool_term(false),
        ],
    ))
}

pub fn sample_cones() -> (Vec<ConeEvidence>, Vec<ConeEvidence>, Vec<ConeEvidence>) {
    let clean =
        |id: &str, source, probes: Vec<Rc<Term>>, channel_a: Rc<Term>, channel_b: Rc<Term>| {
            ConeEvidence {
                id: id.into(),
                duplicate_group: id.into(),
                source,
                probes,
                channel_a,
                channel_b,
                recorded_epoch: 1,
                derivation: EvidenceDerivation::default(),
                protected_annotation: 0,
            }
        };
    let training = vec![
        clean(
            "train-bit",
            SourceInterface::Bit,
            vec![bool_term(false), bool_term(true)],
            identity(),
            boolean_not(),
        ),
        clean(
            "train-trit",
            SourceInterface::Ternary,
            vec![church_numeral(0), church_numeral(1), church_numeral(2)],
            is_zero(),
            numeral_parity(),
        ),
    ];
    let calibration = vec![clean(
        "cal-four",
        SourceInterface::Ternary,
        vec![
            church_numeral(0),
            church_numeral(1),
            church_numeral(2),
            church_numeral(3),
        ],
        numeral_parity(),
        compose_morphisms(boolean_not(), is_zero()),
    )];
    let protected = vec![
        clean(
            "held-bit-composed",
            SourceInterface::Bit,
            vec![bool_term(false), bool_term(true)],
            boolean_not(),
            identity(),
        ),
        clean(
            "held-list-representation",
            SourceInterface::ChurchList,
            vec![
                church_list(&[]),
                church_list(&[7]),
                church_list(&[2, 8]),
                church_list(&[1, 3, 9]),
            ],
            list_is_empty(),
            list_length_parity(),
        ),
    ];
    (training, calibration, protected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universal::{Dovetail, InterleavedDovetail, ResourceLane};

    fn hidden_carrier() -> Rc<Term> {
        // λa.λb.λtag.λk. k a b tag
        let body = apply(term::var(0), [term::var(3), term::var(2), term::var(1)]);
        (0..4).fold(body, |body, _| term::lam(body))
    }

    fn hidden_observer(index: u32) -> Rc<Term> {
        // λp. p (λa.λb.λtag. selected-field)
        term::lam(term::app(
            term::var(0),
            (0..3).fold(term::var(index), |body, _| term::lam(body)),
        ))
    }

    fn direct_hidden_mediator(cone: &ConeEvidence, tag: bool) -> Rc<Term> {
        term::lam(apply(
            hidden_carrier(),
            [
                term::app(cone.channel_a.clone(), term::var(0)),
                term::app(cone.channel_b.clone(), term::var(0)),
                bool_term(tag),
            ],
        ))
    }

    #[test]
    fn discovers_and_reuses_a_bounded_universal_factorization() {
        let (training, calibration, protected) = sample_cones();
        let protected_ids = protected
            .iter()
            .map(|cone| cone.id.clone())
            .collect::<BTreeSet<_>>();
        let spec = default_spec();
        let report = discover(&training, &calibration, &protected_ids, &spec);
        let structure = report.structure.as_ref().expect("U1 must be discovered");

        assert!(report.calibration_commutes);
        assert!(report.calibration_unique);
        assert!(!report.syntax_baseline_found);
        assert!(transform::is_closed(&structure.carrier));
        assert!(transform::is_closed(&structure.observe_a));
        assert!(transform::is_closed(&structure.observe_b));
        assert!(universal::in_language(&structure.carrier, 0, &[]));
        assert!(universal::in_language(&structure.observe_a, 0, &[]));
        assert!(universal::in_language(&structure.observe_b, 0, &[]));
        assert_eq!(structure.carrier.size(), 8);
        assert_eq!(structure.observe_a.size(), 6);
        assert_eq!(structure.observe_b.size(), 6);
        let richer_carrier = term::lam(term::lam(term::app(
            identity(),
            apply(structure.carrier.clone(), [term::var(1), term::var(0)]),
        )));
        assert!(richer_carrier.size() > structure.carrier.size());
        assert!(field_probes().iter().all(|(a, b)| {
            observationally_equal(
                &apply(structure.carrier.clone(), [a.clone(), b.clone()]),
                &apply(richer_carrier.clone(), [a.clone(), b.clone()]),
                spec.fuel,
            )
        }));

        for cone in &protected {
            let mut checks = 0;
            assert!(commutes(structure, cone, &mut checks));
            let uniqueness = bounded_uniqueness(structure, cone, &spec);
            assert!(uniqueness.exhaustive_within_size);
            assert!(uniqueness.unique);
            assert!(uniqueness.valid_mediators > 0);

            let before_view = protected_view(cone);
            let mut mutated = cone.clone();
            mutated.protected_annotation = i64::MAX;
            assert_eq!(protected_view(&mutated), before_view);
            let mut replay_checks = 0;
            assert!(commutes(structure, &mutated, &mut replay_checks));
            assert_eq!(replay_checks, checks);
            assert_eq!(bounded_uniqueness(structure, &mutated, &spec), uniqueness);
        }

        let base_swap = measure_downstream(DownstreamTask::Swap, structure, false, 14, 50_000);
        let learned_swap = measure_downstream(DownstreamTask::Swap, structure, true, 14, 50_000);
        assert!(base_swap.solved && learned_swap.solved);
        assert!(learned_swap.proposals < base_swap.proposals);
        assert!(learned_swap.observation_checks < base_swap.observation_checks);
        let uniform = measure_uniform_downstream(DownstreamTask::Swap, structure, 14, 50_000);
        let irrelevant = measure_irrelevant_downstream(DownstreamTask::Swap, structure, 14, 50_000);
        let oracle = measure_oracle_downstream(DownstreamTask::Swap, structure);
        assert!(uniform.solved && irrelevant.solved && oracle.solved);
        assert_eq!(learned_swap.proposals, oracle.proposals);
        assert!(learned_swap.proposals < uniform.proposals);
        assert!(learned_swap.proposals < irrelevant.proposals);
        let universal = measure_universal_downstream(DownstreamTask::Swap, structure, 10);
        assert!(!universal.solved);
        assert!(universal.proposals > learned_swap.proposals);
        let universal_learned =
            measure_universal_with_structure(DownstreamTask::Swap, structure, 10);
        assert!(universal_learned.solved);
        assert_eq!(universal_learned.proposals, 1);
        let universal_discovery = report
            .accounting
            .carrier_terms
            .saturating_add(report.accounting.observer_terms);
        assert!(universal.proposals > universal_learned.proposals + universal_discovery);
        let geometry = cost_geometry(
            report.charged_discovery_cost,
            std::slice::from_ref(&base_swap),
            std::slice::from_ref(&learned_swap),
            1_000,
        );
        assert!(geometry.net_gain > 0);
        assert!(geometry.before > geometry.after);
        assert!(geometry.triangle_holds);
        let typed_discovery = report
            .accounting
            .generator_terms
            .saturating_add(report.accounting.mediator_terms);
        assert!(
            base_swap
                .proposals
                .saturating_sub(learned_swap.proposals)
                .saturating_mul(geometry.protected_uses)
                > typed_discovery
        );
        let decision = decide_acquisition(&geometry);
        assert!(decision.retained);
        assert_eq!(decision.learned_budget_units, 1);

        let base_map = measure_downstream(DownstreamTask::MapBoth, structure, false, 20, 50_000);
        let learned_map = measure_downstream(DownstreamTask::MapBoth, structure, true, 20, 50_000);
        assert!(base_map.solved && learned_map.solved);
        assert!(learned_map.proposals >= base_map.proposals);
        let useless = cost_geometry(100, &[base_map.clone()], &[base_map], 1_000);
        assert!(useless.net_gain < 0);
        let rejected = decide_acquisition(&useless);
        assert!(!rejected.retained);
        assert_eq!(rejected.learned_budget_units, 0);

        assert_eq!(
            aggregate_work(&[
                (U1WorkDomain::LambdaObservations, 3),
                (U1WorkDomain::LambdaObservations, 4),
            ]),
            Ok((U1WorkDomain::LambdaObservations, 7))
        );
        assert_eq!(
            aggregate_work(&[
                (U1WorkDomain::LambdaObservations, 3),
                (U1WorkDomain::TypedProposals, 4),
            ]),
            Err("unlike work units")
        );

        let learned_resources = (0..128).map(|index| ((index % 9 + 1) as u32, index + 1));
        let mut schedule = InterleavedDovetail::new(learned_resources);
        let mut projection = Vec::new();
        while projection.len() < 256 {
            let point = schedule.next_labeled().unwrap();
            if point.lane == ResourceLane::Universal {
                projection.push((point.syntax_size, point.evaluation_fuel));
            }
        }
        assert_eq!(
            projection,
            Dovetail::default().take(256).collect::<Vec<_>>()
        );
    }

    #[test]
    fn existence_without_uniqueness_and_unsafe_controls_are_rejected() {
        let (_, _, protected) = sample_cones();
        let cone = &protected[0];
        let carrier = hidden_carrier();
        let observe_a = hidden_observer(2);
        let observe_b = hidden_observer(1);
        let mediators = [
            direct_hidden_mediator(cone, false),
            direct_hidden_mediator(cone, true),
        ];
        let mut classes = HashSet::new();
        for mediator in &mediators {
            let mut observations = Vec::new();
            for probe in &cone.probes {
                let value = term::app(mediator.clone(), probe.clone());
                assert!(observationally_equal(
                    &term::app(observe_a.clone(), value.clone()),
                    &term::app(cone.channel_a.clone(), probe.clone()),
                    100_000,
                ));
                assert!(observationally_equal(
                    &term::app(observe_b.clone(), value.clone()),
                    &term::app(cone.channel_b.clone(), probe.clone()),
                    100_000,
                ));
                observations.push(normalize(&value, 100_000).unwrap().as_ref().clone());
            }
            classes.insert(observations);
        }
        assert_eq!(classes.len(), 2);

        let valid = UniversalStructure {
            carrier,
            observe_a,
            observe_b,
            generator: term::lam(term::lam(term::lam(term::var(0)))),
            freeze_epoch: 1,
            observational_fuel: 100_000,
            mediator_boundary: 10,
        };
        let swapped = UniversalStructure {
            observe_a: valid.observe_b.clone(),
            observe_b: valid.observe_a.clone(),
            ..valid.clone()
        };
        let mut checks = 0;
        assert!(!commutes(&swapped, cone, &mut checks));

        let wrong_generator = UniversalStructure {
            generator: term::lam(term::lam(term::lam(term::var(0)))),
            ..valid.clone()
        };
        let mut checks = 0;
        assert!(!commutes(&wrong_generator, cone, &mut checks));

        let collapsed = term::lam(term::lam(term::var(1)));
        let mut checks = 0;
        assert!(!separates_probe_values(
            &collapsed,
            &field_probes(),
            100_000,
            &mut checks,
        ));

        let omega_abs = term::lam(term::app(term::var(0), term::var(0)));
        let omega = term::app(omega_abs.clone(), omega_abs);
        let divergent = UniversalStructure {
            carrier: term::lam(term::lam(omega)),
            ..valid
        };
        let mut checks = 0;
        assert!(!commutes(&divergent, cone, &mut checks));

        let (training, calibration, protected) = sample_cones();
        let protected_ids = protected
            .iter()
            .map(|cone| cone.id.clone())
            .collect::<BTreeSet<_>>();
        let mut undersized = default_spec();
        undersized.max_carrier_size = 7;
        assert!(
            discover(&training, &calibration, &protected_ids, &undersized)
                .structure
                .is_none()
        );
        assert!(discover(&[], &calibration, &protected_ids, &default_spec())
            .structure
            .is_none());
    }

    #[test]
    fn leakage_is_removed_before_candidate_generation_and_accounting() {
        let (training, calibration, protected) = sample_cones();
        let protected_ids = protected
            .iter()
            .map(|cone| cone.id.clone())
            .collect::<BTreeSet<_>>();
        let spec = default_spec();
        let clean = discover(&training, &calibration, &protected_ids, &spec);
        let mut poisoned = training.clone();
        let mut target = protected[0].clone();
        target.derivation.target_derived = true;
        poisoned.push(target);
        let mut output = protected[0].clone();
        output.id = "output-poison".into();
        output.derivation.output_derived = true;
        poisoned.push(output);
        let mut late = protected[0].clone();
        late.id = "late-poison".into();
        late.recorded_epoch = 2;
        poisoned.push(late);
        let mut ancestry = protected[0].clone();
        ancestry.id = "ancestry-poison".into();
        ancestry
            .derivation
            .ancestor_ids
            .insert("held-bit-composed".into());
        poisoned.push(ancestry);
        let mut duplicate = training[0].clone();
        duplicate.id = "duplicate-poison".into();
        duplicate.duplicate_group = "held-bit-composed".into();
        poisoned.push(duplicate);
        let contaminated = discover(&poisoned, &calibration, &protected_ids, &spec);
        let clean_structure = clean.structure.unwrap();
        let contaminated_structure = contaminated.structure.unwrap();
        assert_eq!(contaminated_structure.carrier, clean_structure.carrier);
        assert_eq!(contaminated_structure.observe_a, clean_structure.observe_a);
        assert_eq!(contaminated_structure.observe_b, clean_structure.observe_b);
        assert_eq!(contaminated_structure.generator, clean_structure.generator);
        assert_eq!(contaminated.accounting, clean.accounting);
        assert_eq!(
            contaminated.charged_discovery_cost,
            clean.charged_discovery_cost
        );
    }
}
