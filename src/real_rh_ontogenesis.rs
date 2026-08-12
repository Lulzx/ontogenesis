//! Direction M30a: bounded ontogenesis run against the real RH target.
//!
//! The search is exhaustive inside a frozen proof-program grammar. It may
//! expose a frontier, but accepts only closed real-xi proofs or strict certified
//! reductions. Surrogate and finite evidence cannot cross that boundary.

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Statement {
    FiniteZeroEvidence,
    EvenQuarticForcing,
    EulerProductHalfPlane,
    XiFunctionalEquation,
    ConjugationClosure,
    ReflectionEquivalence,
    SpectralCorrespondence,
    SelfAdjointness,
}

const COLD_STATEMENTS: [Statement; 8] = [
    Statement::FiniteZeroEvidence,
    Statement::EvenQuarticForcing,
    Statement::EulerProductHalfPlane,
    Statement::XiFunctionalEquation,
    Statement::ConjugationClosure,
    Statement::ReflectionEquivalence,
    Statement::SpectralCorrespondence,
    Statement::SelfAdjointness,
];

const ACQUIRED_STATEMENTS: [Statement; 8] = [
    Statement::SpectralCorrespondence,
    Statement::SelfAdjointness,
    Statement::XiFunctionalEquation,
    Statement::ReflectionEquivalence,
    Statement::ConjugationClosure,
    Statement::EulerProductHalfPlane,
    Statement::FiniteZeroEvidence,
    Statement::EvenQuarticForcing,
];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Inference {
    SymmetryForcing,
    FiniteGeneralization,
    EquivalenceAsProof,
    SurrogateTransfer,
    ExplicitFormulaForcing,
    SpectralForcing,
}

const INFERENCES: [Inference; 6] = [
    Inference::SymmetryForcing,
    Inference::FiniteGeneralization,
    Inference::EquivalenceAsProof,
    Inference::SurrogateTransfer,
    Inference::ExplicitFormulaForcing,
    Inference::SpectralForcing,
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofProgram {
    pub statements: Vec<Statement>,
    pub inference: Inference,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Domain {
    RealXi,
    EvenQuartic,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckResult {
    pub derives_target: bool,
    pub domain: Domain,
    pub universal: bool,
    pub open_assumptions: BTreeSet<Statement>,
    pub valid_strict_reduction: bool,
}

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub condition: &'static str,
    pub programs_enumerated: usize,
    pub checker_calls: usize,
    pub proof_found: bool,
    pub reduction_found: bool,
    pub best_frontier_rank: usize,
    pub best_frontier: ProofProgram,
    pub best_frontier_check: CheckResult,
}

#[derive(Clone, Debug)]
pub struct M30aExperiment {
    pub candidate_programs: usize,
    pub cold: SearchResult,
    pub acquired: SearchResult,
    pub equivalent_spaces: bool,
    pub frontier_ordering_gain: usize,
    pub controls: [bool; 6],
    pub controls_declined: usize,
    pub checker_positive_control: bool,
    pub proof_found: bool,
    pub reduction_found: bool,
    pub real_rh_proved: bool,
    pub m30_reached: bool,
    pub run_completed: bool,
    pub outcome: &'static str,
    pub claim_level: &'static str,
}

fn is_open(statement: Statement) -> bool {
    matches!(
        statement,
        Statement::SpectralCorrespondence | Statement::SelfAdjointness
    )
}

pub fn exact_checker(program: &ProofProgram) -> CheckResult {
    let set: BTreeSet<_> = program.statements.iter().copied().collect();
    let mut open_assumptions: BTreeSet<_> = set.iter().copied().filter(|s| is_open(*s)).collect();
    let (derives_target, domain, universal) = match program.inference {
        Inference::SpectralForcing
            if set.contains(&Statement::SpectralCorrespondence)
                && set.contains(&Statement::SelfAdjointness) =>
        {
            (true, Domain::RealXi, true)
        }
        Inference::SurrogateTransfer if set.contains(&Statement::EvenQuarticForcing) => {
            (true, Domain::EvenQuartic, true)
        }
        Inference::FiniteGeneralization if set.contains(&Statement::FiniteZeroEvidence) => {
            (false, Domain::RealXi, false)
        }
        Inference::SymmetryForcing
            if set.contains(&Statement::XiFunctionalEquation)
                && set.contains(&Statement::ConjugationClosure) =>
        {
            (false, Domain::RealXi, true)
        }
        Inference::EquivalenceAsProof if set.contains(&Statement::ReflectionEquivalence) => {
            (false, Domain::RealXi, true)
        }
        Inference::ExplicitFormulaForcing if set.contains(&Statement::EulerProductHalfPlane) => {
            (false, Domain::RealXi, true)
        }
        _ => (false, Domain::RealXi, false),
    };
    if !derives_target {
        open_assumptions.clear();
    }
    // No frozen rule proves RH from a proposed obligation in reverse, so no
    // candidate in this grammar is a certified bidirectional reduction.
    CheckResult {
        derives_target,
        domain,
        universal,
        open_assumptions,
        valid_strict_reduction: false,
    }
}

fn permutations(order: &[Statement], length: usize) -> Vec<Vec<Statement>> {
    fn extend(
        order: &[Statement],
        length: usize,
        prefix: &mut Vec<Statement>,
        output: &mut Vec<Vec<Statement>>,
    ) {
        if prefix.len() == length {
            output.push(prefix.clone());
            return;
        }
        for statement in order {
            if !prefix.contains(statement) {
                prefix.push(*statement);
                extend(order, length, prefix, output);
                prefix.pop();
            }
        }
    }
    let mut output = Vec::new();
    extend(order, length, &mut Vec::new(), &mut output);
    output
}

fn programs(order: &[Statement]) -> Vec<ProofProgram> {
    let mut output = Vec::new();
    for length in 1..=4 {
        for statements in permutations(order, length) {
            for inference in INFERENCES {
                output.push(ProofProgram {
                    statements: statements.clone(),
                    inference,
                });
            }
        }
    }
    output
}

fn frontier_score(check: &CheckResult) -> (usize, usize, usize) {
    (
        usize::from(!(check.derives_target && check.domain == Domain::RealXi && check.universal)),
        check.open_assumptions.len(),
        usize::from(!check.valid_strict_reduction),
    )
}

fn run(condition: &'static str, order: &[Statement]) -> SearchResult {
    let candidates = programs(order);
    let mut proof_found = false;
    let mut reduction_found = false;
    let mut best: Option<(usize, ProofProgram, CheckResult)> = None;
    for (index, program) in candidates.iter().enumerate() {
        let checked = exact_checker(program);
        let closed_proof = checked.derives_target
            && checked.domain == Domain::RealXi
            && checked.universal
            && checked.open_assumptions.is_empty();
        proof_found |= closed_proof;
        reduction_found |= checked.valid_strict_reduction;
        if checked.derives_target && checked.domain == Domain::RealXi {
            let replace = best.as_ref().is_none_or(|(_, _, incumbent)| {
                frontier_score(&checked) < frontier_score(incumbent)
            });
            if replace {
                best = Some((index + 1, program.clone(), checked));
            }
        }
    }
    let (best_frontier_rank, best_frontier, best_frontier_check) =
        best.expect("frozen grammar has a real-xi frontier");
    SearchResult {
        condition,
        programs_enumerated: candidates.len(),
        checker_calls: candidates.len(),
        proof_found,
        reduction_found,
        best_frontier_rank,
        best_frontier,
        best_frontier_check,
    }
}

fn controls() -> [bool; 6] {
    let rejected = |statements: Vec<Statement>, inference| {
        let checked = exact_checker(&ProofProgram {
            statements,
            inference,
        });
        !(checked.derives_target
            && checked.domain == Domain::RealXi
            && checked.universal
            && checked.open_assumptions.is_empty())
    };
    [
        rejected(
            vec![
                Statement::XiFunctionalEquation,
                Statement::ConjugationClosure,
            ],
            Inference::SymmetryForcing,
        ),
        rejected(
            vec![Statement::FiniteZeroEvidence],
            Inference::FiniteGeneralization,
        ),
        rejected(
            vec![Statement::ReflectionEquivalence],
            Inference::EquivalenceAsProof,
        ),
        rejected(
            vec![Statement::EvenQuarticForcing],
            Inference::SurrogateTransfer,
        ),
        rejected(
            vec![Statement::SpectralCorrespondence],
            Inference::SpectralForcing,
        ),
        rejected(vec![Statement::SelfAdjointness], Inference::SpectralForcing),
    ]
}

fn positive_checker_control() -> bool {
    let program = ProofProgram {
        statements: vec![
            Statement::SpectralCorrespondence,
            Statement::SelfAdjointness,
        ],
        inference: Inference::SpectralForcing,
    };
    let mut checked = exact_checker(&program);
    // This models independently supplied closed proofs of both bridge lemmas;
    // it tests the terminal closure rule and is not a search candidate.
    checked.open_assumptions.clear();
    checked.derives_target
        && checked.domain == Domain::RealXi
        && checked.universal
        && checked.open_assumptions.is_empty()
}

pub fn m30a_experiment() -> M30aExperiment {
    let cold_programs = programs(&COLD_STATEMENTS);
    let acquired_programs = programs(&ACQUIRED_STATEMENTS);
    let canonical = |program: &ProofProgram| {
        let mut statements = program.statements.clone();
        statements.sort();
        (statements, program.inference)
    };
    let cold_set: BTreeSet<_> = cold_programs.iter().map(canonical).collect();
    let acquired_set: BTreeSet<_> = acquired_programs.iter().map(canonical).collect();
    let equivalent_spaces = cold_set == acquired_set;
    let cold = run("cold", &COLD_STATEMENTS);
    let acquired = run("acquired_M26_M29", &ACQUIRED_STATEMENTS);
    let control_results = controls();
    let controls_declined = control_results.iter().filter(|result| **result).count();
    let proof_found = cold.proof_found || acquired.proof_found;
    let reduction_found = cold.reduction_found || acquired.reduction_found;
    let real_rh_proved = proof_found;
    let m30_reached = proof_found || reduction_found;
    let run_completed = equivalent_spaces
        && cold.programs_enumerated == acquired.programs_enumerated
        && controls_declined == 6
        && positive_checker_control();
    M30aExperiment {
        candidate_programs: cold.programs_enumerated,
        frontier_ordering_gain: cold
            .best_frontier_rank
            .saturating_sub(acquired.best_frontier_rank),
        cold,
        acquired,
        equivalent_spaces,
        controls: control_results,
        controls_declined,
        checker_positive_control: positive_checker_control(),
        proof_found,
        reduction_found,
        real_rh_proved,
        m30_reached,
        run_completed,
        outcome: if m30_reached {
            "reached"
        } else {
            "attempted_unreached"
        },
        claim_level: "L0_bounded_search_negative_result",
    }
}

pub fn machine_record(report: &M30aExperiment) -> String {
    format!(
        "M30a_real|programs={}|equivalent_spaces={}|cold_frontier_rank={}|acquired_frontier_rank={}|ordering_gain={}|frontier_rule={:?}|open_assumptions={:?}|controls={:?}|controls_declined={}/6|checker_positive_control={}|proof_found={}|reduction_found={}|real_rh_proved={}|m30_reached={}|run_completed={}|outcome={}|claim={}",
        report.candidate_programs,
        report.equivalent_spaces,
        report.cold.best_frontier_rank,
        report.acquired.best_frontier_rank,
        report.frontier_ordering_gain,
        report.acquired.best_frontier.inference,
        report.acquired.best_frontier_check.open_assumptions,
        report.controls,
        report.controls_declined,
        report.checker_positive_control,
        report.proof_found,
        report.reduction_found,
        report.real_rh_proved,
        report.m30_reached,
        report.run_completed,
        report.outcome,
        report.claim_level
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conditions_enumerate_the_same_bounded_program_space() {
        let report = m30a_experiment();
        assert!(report.equivalent_spaces);
        assert_eq!(
            report.cold.programs_enumerated,
            report.acquired.programs_enumerated
        );
    }

    #[test]
    fn checker_rejects_all_six_shortcuts() {
        assert_eq!(controls(), [true; 6]);
        assert!(positive_checker_control());
    }

    #[test]
    fn real_run_exposes_open_spectral_frontier_without_claiming_rh() {
        let report = m30a_experiment();
        assert!(report.run_completed, "{report:#?}");
        assert_eq!(report.outcome, "attempted_unreached");
        assert_eq!(
            report.acquired.best_frontier_check.open_assumptions,
            BTreeSet::from([
                Statement::SpectralCorrespondence,
                Statement::SelfAdjointness,
            ])
        );
        assert!(!report.proof_found);
        assert!(!report.reduction_found);
        assert!(!report.real_rh_proved);
        assert!(!report.m30_reached);
        assert_eq!(machine_record(&report), machine_record(&m30a_experiment()));
    }
}
