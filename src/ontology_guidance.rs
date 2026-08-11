//! Ontology-guided recursive discovery experiments.
//!
//! A finite priority prefix measures practical search allocation. It is not a
//! replacement for universality: [`crate::universal::PrioritizedDovetail`]
//! follows every such prefix with the unchanged fair schedule.

use crate::{
    nbe,
    recursion_search::{self, Example, SearchOutcome, SearchProblem},
    term::{self, Term},
};
use std::rc::Rc;

pub const EXPERIMENT_FUEL: u64 = 100_000;

#[derive(Clone, Debug)]
pub struct ConditionReport {
    pub name: &'static str,
    pub outcome: SearchOutcome,
}

#[derive(Clone, Debug)]
pub struct GuidanceGain {
    pub baseline_proposals: u64,
    pub guided_proposals: u64,
    pub baseline_solved: bool,
    pub guided_solved: bool,
}

impl GuidanceGain {
    pub fn ratio(&self) -> u64 {
        self.baseline_proposals / self.guided_proposals.max(1)
    }

    pub fn earns(&self, minimum_ratio: u64) -> bool {
        self.guided_solved
            && self.guided_proposals > 0
            && self.ratio() >= minimum_ratio
            && (!self.baseline_solved || self.guided_proposals < self.baseline_proposals)
    }
}

#[derive(Clone, Debug)]
pub struct ParityGuidanceReport {
    pub negation_validated: bool,
    pub compressed_target_size: u32,
    pub expanded_target_size: u32,
    pub empty: ConditionReport,
    pub relevant: ConditionReport,
    pub irrelevant: ConditionReport,
    pub misleading: ConditionReport,
    pub gain: GuidanceGain,
}

#[derive(Clone, Debug)]
pub struct NestedGuidanceReport {
    pub compressed_target_size: u32,
    pub expanded_target_size: u32,
    pub prior_ontology: ConditionReport,
    pub acquired_recursive: ConditionReport,
    pub irrelevant_recursive: ConditionReport,
    pub misleading_recursive: ConditionReport,
    pub gain: GuidanceGain,
}

#[derive(Clone, Debug)]
pub struct DevelopmentalReport {
    /// `O0 -> c1(not) -> O1 -> c2(recursive parity)`.
    pub parity: ParityGuidanceReport,
    /// `O1 -> c2 -> O2 -> c3(nested recursive parity aggregation)`.
    pub nested: NestedGuidanceReport,
}

pub fn church_bool(value: bool) -> Rc<Term> {
    if value {
        term::lam(term::lam(term::var(1)))
    } else {
        term::lam(term::lam(term::var(0)))
    }
}

pub fn boolean_not() -> Rc<Term> {
    term::lam(term::app(
        term::app(term::var(0), church_bool(false)),
        church_bool(true),
    ))
}

fn boolean_identity() -> Rc<Term> {
    term::lam(term::var(0))
}

fn irrelevant_pair_constructor() -> Rc<Term> {
    term::lam(term::lam(term::lam(term::app(
        term::app(term::var(0), term::var(2)),
        term::var(1),
    ))))
}

fn normalize(t: &Rc<Term>, fuel: i64) -> Option<Rc<Term>> {
    nbe::normalize(&Rc::new(Vec::new()), t, &mut nbe::Fuel(fuel)).ok()
}

pub fn validates_boolean_negation() -> bool {
    [(false, true), (true, false)]
        .into_iter()
        .all(|(input, expected)| {
            normalize(
                &term::app(boolean_not(), church_bool(input)),
                EXPERIMENT_FUEL as i64,
            ) == normalize(&church_bool(expected), EXPERIMENT_FUEL as i64)
        })
}

/// Anonymous chain value. The functional must supply the Boolean step algebra
/// and its recursive result: base ignores both and returns false; each link
/// computes `not (recursive tail)`.
fn parity_chain(depth: u32) -> Rc<Term> {
    (0..depth).fold(term::lam(term::lam(church_bool(false))), |tail, _| {
        term::lam(term::lam(term::app(
            term::var(1),
            term::app(term::var(0), tail),
        )))
    })
}

fn boolean_xor() -> Rc<Term> {
    // λa.λb. a (not b) b
    term::lam(term::lam(term::app(
        term::app(term::var(1), term::app(boolean_not(), term::var(0))),
        term::var(0),
    )))
}

/// An outer anonymous chain whose payloads are themselves anonymous parity
/// chains. Its supplied concept interprets each payload; the outer recursive
/// result combines those interpretations by xor.
fn nested_chain(payload_depths: &[u32]) -> Rc<Term> {
    payload_depths.iter().rev().fold(
        term::lam(term::lam(church_bool(false))),
        |tail, &payload_depth| {
            let payload = parity_chain(payload_depth);
            term::lam(term::lam(term::app(
                term::app(boolean_xor(), term::app(term::var(1), payload)),
                term::app(term::var(0), tail),
            )))
        },
    )
}

fn nested_expected(payload_depths: &[u32]) -> Rc<Term> {
    church_bool(
        payload_depths
            .iter()
            .filter(|depth| **depth % 2 == 1)
            .count()
            % 2
            == 1,
    )
}

fn nested_problem(atoms: Vec<Rc<Term>>) -> SearchProblem {
    let example = |depths: &[u32]| Example {
        arguments: vec![nested_chain(depths)],
        expected: nested_expected(depths),
    };
    SearchProblem {
        atoms,
        discovery: [vec![], vec![1], vec![1, 1], vec![2, 3, 4]]
            .iter()
            .map(|depths| example(depths))
            .collect(),
        extrapolation: [
            vec![0],
            vec![2],
            vec![3],
            vec![4],
            vec![1, 2, 3, 4, 5],
            vec![1, 3, 2, 4, 2, 4],
            vec![1, 2, 3, 4, 5, 6, 7, 8],
        ]
        .iter()
        .map(|depths| example(depths))
        .collect(),
        require_recursive_reference: true,
        require_load_bearing_recursion: true,
        require_distinct_outputs: true,
    }
}

#[cfg(test)]
fn weak_nested_problem(atoms: Vec<Rc<Term>>) -> SearchProblem {
    let example = |depths: &[u32]| Example {
        arguments: vec![nested_chain(depths)],
        expected: nested_expected(depths),
    };
    SearchProblem {
        atoms,
        discovery: [vec![], vec![1], vec![1, 1], vec![2, 3, 4]]
            .iter()
            .map(|depths| example(depths))
            .collect(),
        // Aggregate-only holdouts fail to identify the inner interpreter.
        extrapolation: [
            vec![1, 2, 3, 4, 5],
            vec![1, 3, 2, 4, 2, 4],
            vec![1, 2, 3, 4, 5, 6, 7, 8],
        ]
        .iter()
        .map(|depths| example(depths))
        .collect(),
        require_recursive_reference: true,
        require_load_bearing_recursion: true,
        require_distinct_outputs: true,
    }
}

fn parity_problem(atoms: Vec<Rc<Term>>) -> SearchProblem {
    let example = |depth| Example {
        arguments: vec![parity_chain(depth)],
        expected: church_bool(depth % 2 == 1),
    };
    SearchProblem {
        atoms,
        discovery: (0..=3).map(example).collect(),
        extrapolation: [5, 7, 9].into_iter().map(example).collect(),
        require_recursive_reference: true,
        require_load_bearing_recursion: true,
        require_distinct_outputs: true,
    }
}

fn parity_functional(step: Rc<Term>) -> Rc<Term> {
    // λrecursive.λvalue. value step recursive
    term::lam(term::lam(term::app(
        term::app(term::var(0), step),
        term::var(1),
    )))
}

fn primitive(body: Rc<Term>) -> Rc<Term> {
    Rc::new(Term::Prim(body))
}

/// Run the first ontology-guidance experiment. The empty baseline is
/// exhaustively falsified through `baseline_max_size`; all one-atom conditions
/// use the compressed target's size, making irrelevant vocabulary a matched
/// alphabet-size control.
pub fn run_parity_guidance(baseline_max_size: u32) -> ParityGuidanceReport {
    let not = boolean_not();
    let expanded_target = parity_functional(not.clone());
    let compressed_target = parity_functional(primitive(not.clone()));
    let guided_max_size = compressed_target.size();

    let empty_outcome = recursion_search::search_priority_prefix(
        &parity_problem(vec![]),
        baseline_max_size,
        EXPERIMENT_FUEL,
    );
    let relevant_outcome = recursion_search::search_priority_prefix(
        &parity_problem(vec![not]),
        guided_max_size,
        EXPERIMENT_FUEL,
    );
    let irrelevant_outcome = recursion_search::search_priority_prefix(
        &parity_problem(vec![irrelevant_pair_constructor()]),
        guided_max_size,
        EXPERIMENT_FUEL,
    );
    let misleading_outcome = recursion_search::search_priority_prefix(
        &parity_problem(vec![boolean_identity()]),
        guided_max_size,
        EXPERIMENT_FUEL,
    );
    let gain = GuidanceGain {
        baseline_proposals: empty_outcome.metrics.proposals,
        guided_proposals: relevant_outcome.metrics.proposals,
        baseline_solved: empty_outcome.candidate.is_some(),
        guided_solved: relevant_outcome.candidate.is_some(),
    };

    ParityGuidanceReport {
        negation_validated: validates_boolean_negation(),
        compressed_target_size: compressed_target.size(),
        expanded_target_size: expanded_target.size(),
        empty: ConditionReport {
            name: "empty",
            outcome: empty_outcome,
        },
        relevant: ConditionReport {
            name: "relevant-not",
            outcome: relevant_outcome,
        },
        irrelevant: ConditionReport {
            name: "irrelevant-pair",
            outcome: irrelevant_outcome,
        },
        misleading: ConditionReport {
            name: "misleading-identity",
            outcome: misleading_outcome,
        },
        gain,
    }
}

/// Run the two-step developmental sequence. The recursive parity executable is
/// obtained solely from the first experiment's discovery and holdout gates; it
/// is then installed as a size-1 atom for a distinct nested-recursion task.
pub fn run_developmental_guidance(
    parity_baseline_max_size: u32,
    nested_baseline_max_size: u32,
) -> DevelopmentalReport {
    let parity = run_parity_guidance(parity_baseline_max_size);
    let parity_executable = parity
        .relevant
        .outcome
        .candidate
        .as_ref()
        .expect("the validated first-stage acquisition must discover parity")
        .executable
        .clone();
    assert!(
        parity.negation_validated && parity.gain.earns(1),
        "negation must pass independent validation and counterfactual acquisition"
    );

    let not = boolean_not();
    let expanded_target = parity_functional(parity_executable.clone());
    let compressed_target = parity_functional(primitive(parity_executable.clone()));
    let guided_max_size = compressed_target.size();
    let prior_outcome = recursion_search::search_priority_prefix(
        &nested_problem(vec![not.clone()]),
        nested_baseline_max_size,
        EXPERIMENT_FUEL,
    );
    let acquired_outcome = recursion_search::search_priority_prefix(
        &nested_problem(vec![not.clone(), parity_executable]),
        guided_max_size,
        EXPERIMENT_FUEL,
    );
    let irrelevant_outcome = recursion_search::search_priority_prefix(
        &nested_problem(vec![not.clone(), irrelevant_pair_constructor()]),
        guided_max_size,
        EXPERIMENT_FUEL,
    );
    let misleading_outcome = recursion_search::search_priority_prefix(
        &nested_problem(vec![not, boolean_identity()]),
        guided_max_size,
        EXPERIMENT_FUEL,
    );
    let gain = GuidanceGain {
        baseline_proposals: prior_outcome.metrics.proposals,
        guided_proposals: acquired_outcome.metrics.proposals,
        baseline_solved: prior_outcome.candidate.is_some(),
        guided_solved: acquired_outcome.candidate.is_some(),
    };

    DevelopmentalReport {
        parity,
        nested: NestedGuidanceReport {
            compressed_target_size: compressed_target.size(),
            expanded_target_size: expanded_target.size(),
            prior_ontology: ConditionReport {
                name: "O1-not-only",
                outcome: prior_outcome,
            },
            acquired_recursive: ConditionReport {
                name: "O2-not-plus-parity",
                outcome: acquired_outcome,
            },
            irrelevant_recursive: ConditionReport {
                name: "not-plus-irrelevant",
                outcome: irrelevant_outcome,
            },
            misleading_recursive: ConditionReport {
                name: "not-plus-misleading",
                outcome: misleading_outcome,
            },
            gain,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{transform, universal};

    fn contains_primitive(t: &Rc<Term>, body: &Rc<Term>) -> bool {
        match t.as_ref() {
            Term::Prim(candidate) => candidate == body,
            Term::Lam(inner) => contains_primitive(inner, body),
            Term::App(function, argument) => {
                contains_primitive(function, body) || contains_primitive(argument, body)
            }
            Term::Var(_) | Term::Free(_) => false,
        }
    }

    #[test]
    fn relevant_ontology_accelerates_recursive_parity_without_losing_universality() {
        let report = run_parity_guidance(11);
        assert!(report.negation_validated);
        assert_eq!(report.compressed_target_size, 7);
        assert!(report.expanded_target_size > report.compressed_target_size);
        assert!(report.empty.outcome.candidate.is_none());
        assert!(report.relevant.outcome.candidate.is_some());
        assert!(report.irrelevant.outcome.candidate.is_none());
        assert!(report.misleading.outcome.candidate.is_none());
        assert!(
            report.gain.earns(100),
            "gain was only {}x",
            report.gain.ratio()
        );

        let discovered = &report
            .relevant
            .outcome
            .candidate
            .as_ref()
            .unwrap()
            .functional;
        assert!(transform::is_closed(discovered));
        assert!(recursion_search::uses_recursive_parameter(discovered));
        assert!(contains_primitive(discovered, &boolean_not()));

        // Adding an ontology alphabet does not remove any pure closed term,
        // and the finite priority schedule returns to the exact fair fallback.
        let pure = parity_functional(boolean_not());
        assert!(universal::in_language(&pure, 0, &[boolean_not()]));
        let scheduled: Vec<_> = universal::PrioritizedDovetail::new([(7, EXPERIMENT_FUEL)])
            .skip(1)
            .take(20)
            .collect();
        assert_eq!(
            scheduled,
            universal::Dovetail::default().take(20).collect::<Vec<_>>()
        );
    }

    #[test]
    fn acquired_recursive_law_accelerates_the_next_recursive_discovery() {
        let report = run_developmental_guidance(8, 9);
        assert!(report.parity.relevant.outcome.candidate.is_some());
        assert!(report.nested.prior_ontology.outcome.candidate.is_none());
        assert!(report.nested.acquired_recursive.outcome.candidate.is_some());
        assert!(report
            .nested
            .irrelevant_recursive
            .outcome
            .candidate
            .is_none());
        assert!(report
            .nested
            .misleading_recursive
            .outcome
            .candidate
            .is_none());
        assert!(report.nested.gain.earns(10));
        let parity_executable = &report
            .parity
            .relevant
            .outcome
            .candidate
            .as_ref()
            .unwrap()
            .executable;
        let nested_functional = &report
            .nested
            .acquired_recursive
            .outcome
            .candidate
            .as_ref()
            .unwrap()
            .functional;
        assert!(contains_primitive(nested_functional, parity_executable));
    }

    #[test]
    fn discriminating_holdouts_reject_an_aggregate_overfit() {
        let not = boolean_not();
        // This surrogate was genuinely discovered by the first weak protocol:
        // it returns negation itself instead of interpreting an inner chain.
        let surrogate = parity_functional(term::lam(term::lam(primitive(not.clone()))));
        assert!(recursion_search::validate_functional(
            &surrogate,
            &weak_nested_problem(vec![not.clone()]),
            EXPERIMENT_FUEL as i64,
        )
        .is_some());
        assert!(recursion_search::validate_functional(
            &surrogate,
            &nested_problem(vec![not]),
            EXPERIMENT_FUEL as i64,
        )
        .is_none());
    }
}
