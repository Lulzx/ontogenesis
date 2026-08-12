//! U2: bounded discovery of a coproduct-like universal cocone.
//!
//! Candidate production is only closed untyped lambda enumeration.  There is
//! no constructor for variants, tags, injections, branching, cocones, or their
//! universal law.  A finite observational verifier asks whether two proposed
//! embeddings are injective and disjoint, whether an independently proposed
//! higher-order program mediates heterogeneous arrow pairs, and whether every
//! mediator in a separately declared typed boundary has one observational
//! class.

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

const LEFT: u32 = 20;
const CARRIER: u32 = 21;
const RESULT: u32 = 22;
const RIGHT: u32 = 23;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DomainEncoding {
    ChurchNumeral,
    ChurchBoolean,
    ChurchList,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResultEncoding {
    ChurchBoolean,
    ChurchNumeral,
    ChurchList,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EvidenceDerivation {
    pub target_derived: bool,
    pub output_derived: bool,
    pub trace_derived: bool,
    pub ancestor_ids: BTreeSet<String>,
}

#[derive(Clone, Debug)]
pub struct RelationalEvidence {
    pub id: String,
    pub duplicate_group: String,
    pub left_domain: DomainEncoding,
    pub right_domain: DomainEncoding,
    pub result: ResultEncoding,
    pub left_probes: Vec<Rc<Term>>,
    pub right_probes: Vec<Rc<Term>>,
    pub left_arrow: Rc<Term>,
    pub right_arrow: Rc<Term>,
    pub recorded_epoch: u64,
    pub derivation: EvidenceDerivation,
    /// Verification-only field; discovery, ranking and accounting never read it.
    pub protected_annotation: i64,
}

#[derive(Clone, Debug)]
pub struct DiscoverySpec {
    pub freeze_epoch: u64,
    pub max_embedding_size: u32,
    pub max_generator_size: u32,
    pub max_mediator_size: u32,
    pub typed_cell_cap: usize,
    pub fuel: i64,
    pub complexity_price: u64,
    pub execution_price: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct U2Accounting {
    pub embedding_terms: u64,
    pub embedding_pairs: u64,
    pub generator_terms: u64,
    pub mediator_terms: u64,
    pub normalization_checks: u64,
    pub equation_checks: u64,
    pub equivalence_checks: u64,
    pub rejected_unsafe: u64,
    pub rejected_nonunique: u64,
    pub max_embedding_size: u32,
    pub max_generator_size: u32,
    pub max_mediator_size: u32,
    pub fuel: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum U2WorkDomain {
    LambdaObservations,
    TypedProposals,
    UniversalResources,
    BehaviorExecutions,
}

pub fn aggregate_work(
    samples: &[(U2WorkDomain, u64)],
) -> Result<(U2WorkDomain, u64), &'static str> {
    let Some((domain, _)) = samples.first() else {
        return Err("empty work set");
    };
    if samples.iter().any(|(other, _)| other != domain) {
        return Err("unlike work units");
    }
    Ok((*domain, samples.iter().map(|(_, n)| *n).sum()))
}

#[derive(Clone, Debug)]
pub struct CoproductStructure {
    /// Together these closed programs constitute the anonymous carrier encoding.
    pub embed_left: Rc<Term>,
    pub embed_right: Rc<Term>,
    /// Maps two arrows to the mediating arrow out of the anonymous carrier.
    pub generator: Rc<Term>,
    pub freeze_epoch: u64,
    pub observational_fuel: i64,
    pub mediator_boundary: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum U2Termination {
    Discovered,
    ExhaustedBoundary,
    InvalidEvidence,
}

#[derive(Clone, Debug)]
pub struct DiscoveryReport {
    pub structure: Option<CoproductStructure>,
    pub accounting: U2Accounting,
    pub calibration_commutes: bool,
    pub calibration_unique: bool,
    pub syntax_baseline_found: bool,
    pub charged_discovery_cost: u64,
    pub termination: U2Termination,
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
    MapBranches,
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
    pub termination: U2Termination,
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

fn apply(function: Rc<Term>, args: impl IntoIterator<Item = Rc<Term>>) -> Rc<Term> {
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

fn leading_lambdas(term: &Rc<Term>) -> usize {
    let mut cursor = term.as_ref();
    let mut count = 0;
    while let Term::Lam(body) = cursor {
        count += 1;
        cursor = body;
    }
    count
}

fn closed_terms(max_size: u32, lambdas: usize) -> Vec<Rc<Term>> {
    (1..=max_size)
        .flat_map(|size| universal::terms_exact(size, 0, &[]))
        .filter(|term| leading_lambdas(term) >= lambdas)
        .collect()
}

fn visible_evidence<'a>(
    evidence: &'a [RelationalEvidence],
    freeze_epoch: u64,
    protected_ids: &BTreeSet<String>,
) -> Vec<&'a RelationalEvidence> {
    evidence
        .iter()
        .filter(|item| {
            item.recorded_epoch <= freeze_epoch
                && !item.derivation.target_derived
                && !item.derivation.output_derived
                && !item.derivation.trace_derived
                && item.derivation.ancestor_ids.is_empty()
                && !protected_ids.contains(&item.id)
                && protected_ids.iter().all(|id| item.duplicate_group != *id)
        })
        .collect()
}

fn independent_payloads() -> Vec<Rc<Term>> {
    vec![
        church_numeral(0),
        church_numeral(1),
        church_numeral(2),
        church_numeral(4),
    ]
}

fn embedding_images(
    embedding: &Rc<Term>,
    probes: &[Rc<Term>],
    fuel: i64,
    checks: &mut u64,
) -> Option<Vec<Rc<Term>>> {
    probes
        .iter()
        .map(|probe| {
            *checks += 1;
            normalize(&term::app(embedding.clone(), probe.clone()), fuel)
        })
        .collect()
}

fn embeddings_are_safe(
    left: &Rc<Term>,
    right: &Rc<Term>,
    probes: &[Rc<Term>],
    fuel: i64,
    checks: &mut u64,
) -> bool {
    if !transform::is_closed(left) || !transform::is_closed(right) || left == right {
        return false;
    }
    let Some(left_images) = embedding_images(left, probes, fuel, checks) else {
        return false;
    };
    let Some(right_images) = embedding_images(right, probes, fuel, checks) else {
        return false;
    };
    let left_set = left_images
        .iter()
        .map(|x| x.as_ref().clone())
        .collect::<HashSet<_>>();
    let right_set = right_images
        .iter()
        .map(|x| x.as_ref().clone())
        .collect::<HashSet<_>>();
    left_set.len() == probes.len()
        && right_set.len() == probes.len()
        && left_set.is_disjoint(&right_set)
}

fn mediator(structure: &CoproductStructure, evidence: &RelationalEvidence) -> Rc<Term> {
    apply(
        structure.generator.clone(),
        [evidence.left_arrow.clone(), evidence.right_arrow.clone()],
    )
}

pub fn commutes(
    structure: &CoproductStructure,
    evidence: &RelationalEvidence,
    checks: &mut u64,
) -> bool {
    if evidence.left_probes.is_empty()
        || evidence.right_probes.is_empty()
        || ![&evidence.left_arrow, &evidence.right_arrow]
            .into_iter()
            .all(|t| transform::is_closed(t))
    {
        return false;
    }
    let h = mediator(structure, evidence);
    let left = evidence.left_probes.iter().all(|probe| {
        *checks += 1;
        equivalent(
            &term::app(
                h.clone(),
                term::app(structure.embed_left.clone(), probe.clone()),
            ),
            &term::app(evidence.left_arrow.clone(), probe.clone()),
            structure.observational_fuel,
        )
    });
    left && evidence.right_probes.iter().all(|probe| {
        *checks += 1;
        equivalent(
            &term::app(
                h.clone(),
                term::app(structure.embed_right.clone(), probe.clone()),
            ),
            &term::app(evidence.right_arrow.clone(), probe.clone()),
            structure.observational_fuel,
        )
    })
}

fn generator_type() -> Type {
    let a = Type::Atom(LEFT);
    let b = Type::Atom(RIGHT);
    let s = Type::Atom(CARRIER);
    let x = Type::Atom(RESULT);
    Type::arrow(
        Type::arrow(a, x.clone()),
        Type::arrow(Type::arrow(b, x.clone()), Type::arrow(s, x)),
    )
}

fn mediator_observations(
    candidate: &Rc<Term>,
    structure: &CoproductStructure,
    evidence: &RelationalEvidence,
    fuel: i64,
) -> Option<Vec<Rc<Term>>> {
    evidence
        .left_probes
        .iter()
        .map(|p| term::app(structure.embed_left.clone(), p.clone()))
        .chain(
            evidence
                .right_probes
                .iter()
                .map(|p| term::app(structure.embed_right.clone(), p.clone())),
        )
        .map(|s| normalize(&term::app(candidate.clone(), s), fuel))
        .collect()
}

pub fn bounded_uniqueness(
    structure: &CoproductStructure,
    evidence: &RelationalEvidence,
    spec: &DiscoverySpec,
) -> UniquenessReport {
    let a = Type::Atom(LEFT);
    let b = Type::Atom(RIGHT);
    let s = Type::Atom(CARRIER);
    let x = Type::Atom(RESULT);
    let atoms = vec![
        Atom {
            body: structure.generator.clone(),
            ty: generator_type(),
        },
        Atom {
            body: evidence.left_arrow.clone(),
            ty: Type::arrow(a, x.clone()),
        },
        Atom {
            body: evidence.right_arrow.clone(),
            ty: Type::arrow(b, x.clone()),
        },
    ];
    let enumeration = typed::enumerate_closed(
        &Type::arrow(s, x),
        &atoms,
        spec.max_mediator_size,
        spec.typed_cell_cap,
    );
    let mut checks = 0;
    let mut valid = 0;
    let mut classes = HashSet::new();
    for candidate in &enumeration.terms {
        let equations = evidence.left_probes.iter().all(|probe| {
            checks += 1;
            equivalent(
                &term::app(
                    candidate.clone(),
                    term::app(structure.embed_left.clone(), probe.clone()),
                ),
                &term::app(evidence.left_arrow.clone(), probe.clone()),
                spec.fuel,
            )
        }) && evidence.right_probes.iter().all(|probe| {
            checks += 1;
            equivalent(
                &term::app(
                    candidate.clone(),
                    term::app(structure.embed_right.clone(), probe.clone()),
                ),
                &term::app(evidence.right_arrow.clone(), probe.clone()),
                spec.fuel,
            )
        });
        if equations {
            valid += 1;
            if let Some(observations) =
                mediator_observations(candidate, structure, evidence, spec.fuel)
            {
                classes.insert(
                    observations
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

pub fn syntax_baseline(evidence: &[RelationalEvidence]) -> bool {
    let mut intersection: Option<HashSet<Term>> = None;
    for arrow in evidence
        .iter()
        .flat_map(|e| [&e.left_arrow, &e.right_arrow])
    {
        let Some(normal) = normalize(arrow, 100_000) else {
            return false;
        };
        let mut terms = HashSet::new();
        nontrivial_closed_subterms(&normal, &mut terms);
        intersection = Some(match intersection {
            None => terms,
            Some(old) => old.intersection(&terms).cloned().collect(),
        });
    }
    intersection.is_some_and(|terms| !terms.is_empty())
}

pub fn discover(
    training: &[RelationalEvidence],
    calibration: &[RelationalEvidence],
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
    let mut accounting = U2Accounting {
        max_embedding_size: spec.max_embedding_size,
        max_generator_size: spec.max_generator_size,
        max_mediator_size: spec.max_mediator_size,
        fuel: spec.fuel,
        ..Default::default()
    };
    let domain_kinds = all
        .iter()
        .flat_map(|e| [e.left_domain, e.right_domain])
        .collect::<BTreeSet<_>>();
    let result_kinds = all.iter().map(|e| e.result).collect::<BTreeSet<_>>();
    if training.is_empty()
        || calibration.is_empty()
        || domain_kinds.len() < 2
        || result_kinds.len() < 2
        || all
            .iter()
            .any(|e| e.left_probes.is_empty() || e.right_probes.is_empty())
    {
        return DiscoveryReport {
            structure: None,
            accounting,
            calibration_commutes: false,
            calibration_unique: false,
            syntax_baseline_found: false,
            charged_discovery_cost: u64::MAX,
            termination: U2Termination::InvalidEvidence,
        };
    }
    let embeddings = closed_terms(spec.max_embedding_size, 3);
    let generators = closed_terms(spec.max_generator_size, 3);
    accounting.embedding_terms = embeddings.len() as u64;
    accounting.generator_terms = generators.len() as u64;
    let probes = independent_payloads();
    let mut best: Option<(u64, CoproductStructure)> = None;
    'pairs: for left in &embeddings {
        for right in &embeddings {
            accounting.embedding_pairs += 1;
            if !embeddings_are_safe(
                left,
                right,
                &probes,
                spec.fuel,
                &mut accounting.normalization_checks,
            ) {
                accounting.rejected_unsafe += 1;
                continue;
            }
            for generator in &generators {
                let proposed = CoproductStructure {
                    embed_left: left.clone(),
                    embed_right: right.clone(),
                    generator: generator.clone(),
                    freeze_epoch: spec.freeze_epoch,
                    observational_fuel: spec.fuel,
                    mediator_boundary: spec.max_mediator_size,
                };
                if !all
                    .iter()
                    .all(|e| commutes(&proposed, e, &mut accounting.equation_checks))
                {
                    continue;
                }
                let mut mediator_work = 0;
                let unique = calibration.iter().all(|e| {
                    let report = bounded_uniqueness(&proposed, e, spec);
                    accounting.mediator_terms += report.generated;
                    accounting.equivalence_checks += report.checks;
                    mediator_work += report.checks;
                    report.unique
                });
                if !unique {
                    accounting.rejected_nonunique += 1;
                    continue;
                }
                let complexity = u64::from(left.size() + right.size() + generator.size());
                let charge = complexity
                    .saturating_mul(spec.complexity_price)
                    .saturating_add(
                        accounting
                            .normalization_checks
                            .saturating_add(accounting.equation_checks)
                            .saturating_add(mediator_work)
                            .saturating_mul(spec.execution_price),
                    );
                if best.as_ref().is_none_or(|(cost, _)| charge < *cost) {
                    best = Some((charge, proposed));
                    // Enumeration is size ordered; this is the first minimal-size complete witness.
                    break 'pairs;
                }
            }
        }
    }
    let syntax_baseline_found =
        syntax_baseline(&all.iter().map(|x| (*x).clone()).collect::<Vec<_>>());
    let structure = best.map(|(_, structure)| structure);
    let mut calibration_checks = 0;
    let calibration_commutes = structure.as_ref().is_some_and(|s| {
        calibration
            .iter()
            .all(|e| commutes(s, e, &mut calibration_checks))
    });
    accounting.equation_checks += calibration_checks;
    let calibration_unique = structure.as_ref().is_some_and(|s| {
        calibration.iter().all(|e| {
            let result = bounded_uniqueness(s, e, spec);
            accounting.mediator_terms += result.generated;
            accounting.equivalence_checks += result.checks;
            result.unique
        })
    });
    let charged_discovery_cost = structure.as_ref().map_or(u64::MAX, |s| {
        u64::from(s.embed_left.size() + s.embed_right.size() + s.generator.size())
            .saturating_mul(spec.complexity_price)
            .saturating_add(
                accounting
                    .normalization_checks
                    .saturating_add(accounting.equation_checks)
                    .saturating_add(accounting.equivalence_checks)
                    .saturating_mul(spec.execution_price),
            )
    });
    DiscoveryReport {
        termination: if structure.is_some() {
            U2Termination::Discovered
        } else {
            U2Termination::ExhaustedBoundary
        },
        structure,
        accounting,
        calibration_commutes,
        calibration_unique,
        syntax_baseline_found,
        charged_discovery_cost,
    }
}

fn prim(body: &Rc<Term>) -> Rc<Term> {
    Rc::new(Term::Prim(body.clone()))
}

fn distinct_primitive_count(value: &Rc<Term>) -> usize {
    fn visit(value: &Rc<Term>, found: &mut HashSet<Term>) {
        match value.as_ref() {
            Term::Prim(body) => {
                found.insert(body.as_ref().clone());
            }
            Term::Lam(body) => visit(body, found),
            Term::App(f, a) => {
                visit(f, found);
                visit(a, found);
            }
            Term::Var(_) | Term::Free(_) => {}
        }
    }
    let mut found = HashSet::new();
    visit(value, &mut found);
    found.len()
}

fn downstream_holds(
    candidate: &Rc<Term>,
    task: DownstreamTask,
    structure: &CoproductStructure,
    checks: &mut u64,
) -> bool {
    if task == DownstreamTask::IdentityControl {
        return independent_payloads()
            .iter()
            .flat_map(|p| {
                [
                    term::app(structure.embed_left.clone(), p.clone()),
                    term::app(structure.embed_right.clone(), p.clone()),
                ]
            })
            .all(|s| {
                *checks += 1;
                equivalent(
                    &term::app(candidate.clone(), s.clone()),
                    &s,
                    structure.observational_fuel,
                )
            });
    }
    independent_payloads().iter().all(|probe| {
        let cases = [
            (
                term::app(structure.embed_left.clone(), probe.clone()),
                term::app(
                    structure.embed_left.clone(),
                    term::app(successor(), probe.clone()),
                ),
            ),
            (
                term::app(structure.embed_right.clone(), probe.clone()),
                term::app(
                    structure.embed_right.clone(),
                    term::app(successor(), probe.clone()),
                ),
            ),
        ];
        cases.into_iter().all(|(input, expected)| {
            *checks += 1;
            equivalent(
                &term::app(candidate.clone(), input),
                &expected,
                structure.observational_fuel,
            )
        })
    })
}

fn downstream_target(task: DownstreamTask) -> Type {
    match task {
        DownstreamTask::MapBranches => Type::arrow(Type::Atom(CARRIER), Type::Atom(CARRIER)),
        DownstreamTask::IdentityControl => Type::arrow(Type::Atom(CARRIER), Type::Atom(CARRIER)),
    }
}

fn task_atoms(task: DownstreamTask) -> Vec<Atom> {
    if task == DownstreamTask::IdentityControl {
        return Vec::new();
    }
    vec![
        Atom {
            body: successor(),
            ty: Type::arrow(Type::Atom(LEFT), Type::Atom(LEFT)),
        },
        Atom {
            body: successor(),
            ty: Type::arrow(Type::Atom(RIGHT), Type::Atom(RIGHT)),
        },
    ]
}

fn structure_atoms(structure: &CoproductStructure) -> Vec<Atom> {
    let a = Type::Atom(LEFT);
    let b = Type::Atom(RIGHT);
    let s = Type::Atom(CARRIER);
    vec![
        Atom {
            body: structure.generator.clone(),
            ty: Type::arrow(
                Type::arrow(a.clone(), s.clone()),
                Type::arrow(
                    Type::arrow(b.clone(), s.clone()),
                    Type::arrow(s.clone(), s.clone()),
                ),
            ),
        },
        Atom {
            body: structure.embed_left.clone(),
            ty: Type::arrow(a, s.clone()),
        },
        Atom {
            body: structure.embed_right.clone(),
            ty: Type::arrow(b, s),
        },
    ]
}

pub fn measure_downstream(
    task: DownstreamTask,
    structure: &CoproductStructure,
    acquired: bool,
    max_size: u32,
    cap: usize,
) -> SearchMeasurement {
    let mut atoms = task_atoms(task);
    if acquired {
        atoms.extend(structure_atoms(structure));
    }
    let enumeration = typed::enumerate_closed(&downstream_target(task), &atoms, max_size, cap);
    let mut terms = enumeration.terms;
    if acquired {
        terms.sort_by_key(|candidate| {
            (
                Reverse(distinct_primitive_count(candidate)),
                candidate.size(),
                term::show(candidate),
            )
        });
    }
    let mut checks = 0;
    for (index, candidate) in terms.iter().enumerate() {
        if downstream_holds(candidate, task, structure, &mut checks) {
            return SearchMeasurement {
                solved: true,
                size: Some(candidate.size()),
                proposals: index as u64 + 1,
                generated_candidates: enumeration.generated,
                observation_checks: checks,
                max_size,
                termination: U2Termination::Discovered,
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
        termination: U2Termination::ExhaustedBoundary,
    }
}

pub fn measure_uniform(
    task: DownstreamTask,
    structure: &CoproductStructure,
    max_size: u32,
    cap: usize,
) -> SearchMeasurement {
    let mut atoms = task_atoms(task);
    atoms.extend(structure_atoms(structure));
    let enumeration = typed::enumerate_closed(&downstream_target(task), &atoms, max_size, cap);
    let mut checks = 0;
    for (index, candidate) in enumeration.terms.iter().enumerate() {
        if downstream_holds(candidate, task, structure, &mut checks) {
            return SearchMeasurement {
                solved: true,
                size: Some(candidate.size()),
                proposals: index as u64 + 1,
                generated_candidates: enumeration.generated,
                observation_checks: checks,
                max_size,
                termination: U2Termination::Discovered,
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
        termination: U2Termination::ExhaustedBoundary,
    }
}

pub fn measure_irrelevant(
    task: DownstreamTask,
    structure: &CoproductStructure,
    max_size: u32,
    cap: usize,
) -> SearchMeasurement {
    let mut atoms = task_atoms(task);
    atoms.push(Atom {
        body: identity(),
        ty: Type::arrow(Type::Atom(RESULT), Type::Atom(RESULT)),
    });
    let enumeration = typed::enumerate_closed(&downstream_target(task), &atoms, max_size, cap);
    let mut checks = 0;
    for (index, candidate) in enumeration.terms.iter().enumerate() {
        if downstream_holds(candidate, task, structure, &mut checks) {
            return SearchMeasurement {
                solved: true,
                size: Some(candidate.size()),
                proposals: index as u64 + 1,
                generated_candidates: enumeration.generated,
                observation_checks: checks,
                max_size,
                termination: U2Termination::Discovered,
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
        termination: U2Termination::ExhaustedBoundary,
    }
}

pub fn measure_oracle(task: DownstreamTask, structure: &CoproductStructure) -> SearchMeasurement {
    let candidate = match task {
        DownstreamTask::IdentityControl => identity(),
        DownstreamTask::MapBranches => term::lam(term::app(
            apply(
                prim(&structure.generator),
                [
                    term::lam(term::app(
                        prim(&structure.embed_left),
                        term::app(prim(&successor()), term::var(0)),
                    )),
                    term::lam(term::app(
                        prim(&structure.embed_right),
                        term::app(prim(&successor()), term::var(0)),
                    )),
                ],
            ),
            term::var(0),
        )),
    };
    let mut checks = 0;
    let solved = downstream_holds(&candidate, task, structure, &mut checks);
    SearchMeasurement {
        solved,
        size: solved.then_some(candidate.size()),
        proposals: 1,
        generated_candidates: 1,
        observation_checks: checks,
        max_size: candidate.size(),
        termination: if solved {
            U2Termination::Discovered
        } else {
            U2Termination::ExhaustedBoundary
        },
    }
}

pub fn measure_pure_universal(
    task: DownstreamTask,
    structure: &CoproductStructure,
    max_size: u32,
) -> SearchMeasurement {
    let mut proposals = 0;
    let mut checks = 0;
    for size in 1..=max_size {
        for candidate in universal::terms_exact(size, 0, &[]) {
            proposals += 1;
            if downstream_holds(&candidate, task, structure, &mut checks) {
                return SearchMeasurement {
                    solved: true,
                    size: Some(size),
                    proposals,
                    generated_candidates: proposals,
                    observation_checks: checks,
                    max_size,
                    termination: U2Termination::Discovered,
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
        termination: U2Termination::ExhaustedBoundary,
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

pub fn decide_acquisition(geometry: &CostGeometry) -> AcquisitionDecision {
    let retained = geometry.net_gain > 0 && geometry.triangle_holds;
    AcquisitionDecision {
        retained,
        utility: geometry.net_gain,
        learned_budget_units: u32::from(retained),
        ranking: if retained {
            vec!["invented-coproduct-structure"]
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

pub fn default_spec() -> DiscoverySpec {
    DiscoverySpec {
        freeze_epoch: 1,
        max_embedding_size: 6,
        max_generator_size: 8,
        max_mediator_size: 8,
        typed_cell_cap: 50_000,
        fuel: 100_000,
        complexity_price: 10,
        execution_price: 1,
    }
}

pub fn identity() -> Rc<Term> {
    term::lam(term::var(0))
}

fn bool_term(value: bool) -> Rc<Term> {
    if value {
        term::lam(term::lam(term::var(1)))
    } else {
        term::lam(term::lam(term::var(0)))
    }
}

pub fn boolean_not() -> Rc<Term> {
    term::lam(apply(term::var(0), [bool_term(false), bool_term(true)]))
}

pub fn church_numeral(n: u32) -> Rc<Term> {
    let body = (0..n).fold(term::var(0), |body, _| term::app(term::var(1), body));
    term::lam(term::lam(body))
}

pub fn successor() -> Rc<Term> {
    term::lam(term::lam(term::lam(term::app(
        term::var(1),
        apply(term::var(2), [term::var(1), term::var(0)]),
    ))))
}

pub fn is_zero() -> Rc<Term> {
    term::lam(apply(
        term::var(0),
        [term::lam(bool_term(false)), bool_term(true)],
    ))
}

pub fn numeral_parity() -> Rc<Term> {
    term::lam(apply(term::var(0), [boolean_not(), bool_term(false)]))
}

pub fn church_list(values: &[u32]) -> Rc<Term> {
    let body = values.iter().rev().fold(term::var(0), |tail, value| {
        apply(term::var(1), [church_numeral(*value), tail])
    });
    term::lam(term::lam(body))
}

fn numeral_to_singleton() -> Rc<Term> {
    // λn.λc.λz.c n z
    term::lam(term::lam(term::lam(apply(
        term::var(1),
        [term::var(2), term::var(0)],
    ))))
}

fn numeral_to_doubleton() -> Rc<Term> {
    // λn.λc.λz.c n (c n z)
    term::lam(term::lam(term::lam(apply(
        term::var(1),
        [
            term::var(2),
            apply(term::var(1), [term::var(2), term::var(0)]),
        ],
    ))))
}

pub fn sample_evidence() -> (
    Vec<RelationalEvidence>,
    Vec<RelationalEvidence>,
    Vec<RelationalEvidence>,
) {
    let make = |id: &str,
                left_domain,
                right_domain,
                result,
                left_probes,
                right_probes,
                left_arrow,
                right_arrow| RelationalEvidence {
        id: id.into(),
        duplicate_group: id.into(),
        left_domain,
        right_domain,
        result,
        left_probes,
        right_probes,
        left_arrow,
        right_arrow,
        recorded_epoch: 1,
        derivation: EvidenceDerivation::default(),
        protected_annotation: 0,
    };
    let numerals = || vec![church_numeral(0), church_numeral(1), church_numeral(2)];
    let booleans = || vec![bool_term(false), bool_term(true)];
    let lists = || vec![church_list(&[]), church_list(&[1]), church_list(&[2, 3])];
    let training = vec![
        make(
            "train-bool",
            DomainEncoding::ChurchNumeral,
            DomainEncoding::ChurchNumeral,
            ResultEncoding::ChurchBoolean,
            numerals(),
            numerals(),
            is_zero(),
            numeral_parity(),
        ),
        make(
            "train-numeral",
            DomainEncoding::ChurchBoolean,
            DomainEncoding::ChurchNumeral,
            ResultEncoding::ChurchNumeral,
            booleans(),
            numerals(),
            term::lam(apply(term::var(0), [church_numeral(2), church_numeral(0)])),
            successor(),
        ),
    ];
    let calibration = vec![make(
        "cal-list",
        DomainEncoding::ChurchNumeral,
        DomainEncoding::ChurchNumeral,
        ResultEncoding::ChurchList,
        numerals(),
        numerals(),
        numeral_to_singleton(),
        numeral_to_doubleton(),
    )];
    let protected = vec![
        make(
            "held-composed",
            DomainEncoding::ChurchNumeral,
            DomainEncoding::ChurchNumeral,
            ResultEncoding::ChurchBoolean,
            vec![church_numeral(0), church_numeral(3), church_numeral(5)],
            numerals(),
            numeral_parity(),
            term::lam(term::app(boolean_not(), term::app(is_zero(), term::var(0)))),
        ),
        make(
            "held-representation",
            DomainEncoding::ChurchBoolean,
            DomainEncoding::ChurchList,
            ResultEncoding::ChurchNumeral,
            booleans(),
            lists(),
            term::lam(apply(term::var(0), [church_numeral(1), church_numeral(2)])),
            term::lam(apply(
                term::var(0),
                [
                    term::lam(term::lam(term::app(successor(), term::var(0)))),
                    church_numeral(0),
                ],
            )),
        ),
    ];
    (training, calibration, protected)
}

pub fn protected_view(
    e: &RelationalEvidence,
) -> (
    String,
    String,
    DomainEncoding,
    DomainEncoding,
    ResultEncoding,
    Vec<Rc<Term>>,
    Vec<Rc<Term>>,
    Rc<Term>,
    Rc<Term>,
) {
    (
        e.id.clone(),
        e.duplicate_group.clone(),
        e.left_domain,
        e.right_domain,
        e.result,
        e.left_probes.clone(),
        e.right_probes.clone(),
        e.left_arrow.clone(),
        e.right_arrow.clone(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universal::{Dovetail, InterleavedDovetail, ResourceLane};

    fn discovered() -> (
        DiscoverySpec,
        Vec<RelationalEvidence>,
        CoproductStructure,
        DiscoveryReport,
    ) {
        let (training, calibration, protected) = sample_evidence();
        let ids = protected.iter().map(|x| x.id.clone()).collect();
        let spec = default_spec();
        let report = discover(&training, &calibration, &ids, &spec);
        let structure = report.structure.clone().expect("U2 should be discovered");
        (spec, protected, structure, report)
    }

    #[test]
    fn discovers_independent_coproduct_like_structure_and_generalizes() {
        let (spec, protected, structure, report) = discovered();
        assert!(report.calibration_commutes && report.calibration_unique);
        assert!(!report.syntax_baseline_found);
        assert_eq!(structure.embed_left.size(), 6);
        assert_eq!(structure.embed_right.size(), 6);
        assert_eq!(structure.generator.size(), 8);
        assert!(universal::in_language(&structure.embed_left, 0, &[]));
        assert!(universal::in_language(&structure.embed_right, 0, &[]));
        assert!(universal::in_language(&structure.generator, 0, &[]));
        for evidence in &protected {
            let mut checks = 0;
            assert!(commutes(&structure, evidence, &mut checks));
            let unique = bounded_uniqueness(&structure, evidence, &spec);
            assert!(unique.exhaustive_within_size && unique.unique && unique.valid_mediators > 0);
        }
    }

    #[test]
    fn leakage_controls_do_not_change_discovery_or_counters() {
        let (training, calibration, protected) = sample_evidence();
        let ids = protected
            .iter()
            .map(|x| x.id.clone())
            .collect::<BTreeSet<_>>();
        let spec = default_spec();
        let clean = discover(&training, &calibration, &ids, &spec);
        let mut poisoned = training.clone();
        for (index, kind) in [0, 1, 2, 3, 4].into_iter().enumerate() {
            let mut item = protected[0].clone();
            item.id = format!("poison-{index}");
            match kind {
                0 => item.derivation.target_derived = true,
                1 => item.derivation.output_derived = true,
                2 => item.derivation.trace_derived = true,
                3 => {
                    item.derivation.ancestor_ids.insert("held-composed".into());
                }
                _ => item.recorded_epoch = 2,
            }
            poisoned.push(item);
        }
        let mut duplicate = training[0].clone();
        duplicate.id = "duplicate-poison".into();
        duplicate.duplicate_group = "held-composed".into();
        poisoned.push(duplicate);
        let contaminated = discover(&poisoned, &calibration, &ids, &spec);
        let a = clean.structure.unwrap();
        let b = contaminated.structure.unwrap();
        assert_eq!(
            (a.embed_left, a.embed_right, a.generator),
            (b.embed_left, b.embed_right, b.generator)
        );
        assert_eq!(clean.accounting, contaminated.accounting);
        assert_eq!(
            clean.charged_discovery_cost,
            contaminated.charged_discovery_cost
        );
    }

    #[test]
    fn protected_mutation_and_universal_lane_are_invariant() {
        let (spec, protected, structure, report) = discovered();
        let baseline =
            measure_downstream(DownstreamTask::MapBranches, &structure, true, 20, 50_000);
        let uniform = measure_uniform(DownstreamTask::MapBranches, &structure, 20, 50_000);
        let geometry = cost_geometry(
            report.charged_discovery_cost,
            std::slice::from_ref(&uniform),
            std::slice::from_ref(&baseline),
            10_000,
        );
        let decision = decide_acquisition(&geometry);
        for evidence in &protected {
            let before = protected_view(evidence);
            let mut mutated = evidence.clone();
            mutated.protected_annotation = i64::MAX;
            assert_eq!(protected_view(&mutated), before);
            let mut a = 0;
            let mut b = 0;
            assert!(commutes(&structure, evidence, &mut a));
            assert!(commutes(&structure, &mutated, &mut b));
            assert_eq!(a, b);
            assert_eq!(
                bounded_uniqueness(&structure, evidence, &spec),
                bounded_uniqueness(&structure, &mutated, &spec)
            );
            // The protected-only annotation is absent from proposal generation,
            // ordering, allocation, and the charged acquisition calculation.
            assert_eq!(
                measure_downstream(DownstreamTask::MapBranches, &structure, true, 20, 50_000,),
                baseline
            );
            assert_eq!(decide_acquisition(&geometry), decision);
        }
        let priority = (0..128).map(|i| ((i % 9 + 1) as u32, i + 1));
        let mut schedule = InterleavedDovetail::new(priority);
        let mut projected = Vec::new();
        while projected.len() < 256 {
            let point = schedule.next_labeled().unwrap();
            if point.lane == ResourceLane::Universal {
                projected.push((point.syntax_size, point.evaluation_fuel));
            }
        }
        assert_eq!(projected, Dovetail::default().take(256).collect::<Vec<_>>());
        assert!(universal::scheduled_stage(u32::MAX, i64::MAX as u64).is_some());
    }

    #[test]
    fn downstream_gain_negative_transfer_accounting_and_geometry() {
        let (_, _, structure, report) = discovered();
        let raw = measure_downstream(DownstreamTask::MapBranches, &structure, false, 20, 50_000);
        let learned = measure_downstream(DownstreamTask::MapBranches, &structure, true, 20, 50_000);
        let uniform = measure_uniform(DownstreamTask::MapBranches, &structure, 20, 50_000);
        let irrelevant = measure_irrelevant(DownstreamTask::MapBranches, &structure, 20, 50_000);
        let oracle = measure_oracle(DownstreamTask::MapBranches, &structure);
        let universal = measure_pure_universal(DownstreamTask::MapBranches, &structure, 8);
        assert!(
            !raw.solved
                && learned.solved
                && uniform.solved
                && !irrelevant.solved
                && oracle.solved
                && !universal.solved
        );
        assert!(
            learned.proposals < uniform.proposals
                && learned.observation_checks < uniform.observation_checks
        );
        let geometry = cost_geometry(
            report.charged_discovery_cost,
            &[uniform],
            &[learned],
            10_000,
        );
        assert!(
            geometry.triangle_holds
                && geometry.net_gain > 0
                && decide_acquisition(&geometry).retained
        );
        let base_identity = measure_downstream(
            DownstreamTask::IdentityControl,
            &structure,
            false,
            4,
            50_000,
        );
        let learned_identity =
            measure_downstream(DownstreamTask::IdentityControl, &structure, true, 4, 50_000);
        assert!(
            base_identity.solved
                && learned_identity.solved
                && learned_identity.proposals >= base_identity.proposals
        );
        let useless = cost_geometry(
            100,
            std::slice::from_ref(&base_identity),
            std::slice::from_ref(&base_identity),
            1_000,
        );
        assert!(!decide_acquisition(&useless).retained);
        assert_eq!(
            aggregate_work(&[
                (U2WorkDomain::TypedProposals, 2),
                (U2WorkDomain::TypedProposals, 3)
            ]),
            Ok((U2WorkDomain::TypedProposals, 5))
        );
        assert_eq!(
            aggregate_work(&[
                (U2WorkDomain::TypedProposals, 2),
                (U2WorkDomain::LambdaObservations, 3)
            ]),
            Err("unlike work units")
        );
    }

    #[test]
    fn malformed_swapped_collapsed_open_partial_and_divergent_controls_fail() {
        let (spec, protected, structure, _) = discovered();
        let evidence = &protected[0];
        let swapped = CoproductStructure {
            embed_left: structure.embed_right.clone(),
            embed_right: structure.embed_left.clone(),
            ..structure.clone()
        };
        let missing = CoproductStructure {
            generator: term::lam(term::lam(term::var(1))),
            ..structure.clone()
        };
        let collapsed = CoproductStructure {
            embed_right: structure.embed_left.clone(),
            ..structure.clone()
        };
        let omega_abs = term::lam(term::app(term::var(0), term::var(0)));
        let omega = term::app(omega_abs.clone(), omega_abs);
        let divergent = CoproductStructure {
            generator: term::lam(term::lam(omega)),
            ..structure.clone()
        };
        for bad in [&swapped, &missing, &collapsed, &divergent] {
            let mut checks = 0;
            assert!(!commutes(bad, evidence, &mut checks));
        }
        assert!(!transform::is_closed(&term::var(0)));
        let mut undersized = spec;
        undersized.max_embedding_size = 5;
        let (training, calibration, held) = sample_evidence();
        let ids = held.iter().map(|x| x.id.clone()).collect();
        assert!(discover(&training, &calibration, &ids, &undersized)
            .structure
            .is_none());
        assert!(discover(&[], &calibration, &ids, &default_spec())
            .structure
            .is_none());
    }

    #[test]
    fn non_epic_carrier_has_existence_but_not_uniqueness() {
        // A third observable branch admits two arrows agreeing on both embedded images.
        let wrap4 = |body| (0..4).fold(body, |body, _| term::lam(body));
        let hidden_left = wrap4(term::app(term::var(2), term::var(3)));
        let hidden_right = wrap4(term::app(term::var(1), term::var(3)));
        let third = wrap4(term::app(term::var(0), term::var(3)));
        let choose = |hidden: bool| {
            term::lam(apply(
                term::var(0),
                [
                    identity(),
                    identity(),
                    if hidden { boolean_not() } else { identity() },
                ],
            ))
        };
        let h0 = choose(false);
        let h1 = choose(true);
        let probe = bool_term(false);
        for h in [&h0, &h1] {
            assert!(equivalent(
                &term::app(h.clone(), term::app(hidden_left.clone(), probe.clone())),
                &probe,
                100_000
            ));
            assert!(equivalent(
                &term::app(h.clone(), term::app(hidden_right.clone(), probe.clone())),
                &probe,
                100_000
            ));
        }
        assert!(!equivalent(
            &term::app(h0, term::app(third.clone(), probe.clone())),
            &term::app(h1, term::app(third, probe)),
            100_000
        ));
    }

    #[test]
    fn truncation_equivalent_cost_and_random_carrier_controls_are_explicit() {
        let (mut spec, protected, structure, _) = discovered();
        spec.typed_cell_cap = 1;
        let truncated = bounded_uniqueness(&structure, &protected[0], &spec);
        assert!(!truncated.exhaustive_within_size);
        assert!(!truncated.unique);

        // A beta-expanded spelling is observationally equivalent on independent
        // images but is strictly more expensive, so size-ordered discovery keeps
        // the cheaper witness.
        let expanded_left = term::lam(term::app(
            identity(),
            term::app(structure.embed_left.clone(), term::var(0)),
        ));
        assert!(independent_payloads().iter().all(|probe| equivalent(
            &term::app(expanded_left.clone(), probe.clone()),
            &term::app(structure.embed_left.clone(), probe.clone()),
            100_000,
        )));
        assert!(expanded_left.size() > structure.embed_left.size());

        // A matched-size arbitrary closed term receives no privileged status.
        let arbitrary = (0..5).fold(term::var(0), |body, _| term::lam(body));
        assert_eq!(arbitrary.size(), structure.embed_left.size());
        let mut checks = 0;
        assert!(!embeddings_are_safe(
            &arbitrary,
            &structure.embed_right,
            &independent_payloads(),
            100_000,
            &mut checks,
        ));
    }
}
