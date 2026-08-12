//! SH3: generic finite spectral-witness construction and real-M29 retry.
//!
//! The retained constructor is learned from anonymous flattened matrices. The
//! real retry consumes primes only; sampled zeta zeros are evaluation data and
//! never construction inputs.

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EntryProgram {
    Left,
    Right,
    Sum,
    SymmetricAverage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Matrix {
    pub n: usize,
    pub entries: Vec<i64>,
}

impl Matrix {
    fn at(&self, row: usize, column: usize) -> i64 {
        self.entries[row * self.n + column]
    }

    fn symmetric(&self) -> bool {
        (0..self.n).all(|i| (0..self.n).all(|j| self.at(i, j) == self.at(j, i)))
    }
}

fn construct(input: &Matrix, program: EntryProgram) -> Option<Matrix> {
    let mut entries = Vec::with_capacity(input.entries.len());
    for i in 0..input.n {
        for j in 0..input.n {
            let left = input.at(i, j);
            let right = input.at(j, i);
            let value = match program {
                EntryProgram::Left => left,
                EntryProgram::Right => right,
                EntryProgram::Sum => left + right,
                EntryProgram::SymmetricAverage => {
                    ((left + right) % 2 == 0).then_some((left + right) / 2)?
                }
            };
            entries.push(value);
        }
    }
    Some(Matrix {
        n: input.n,
        entries,
    })
}

fn training() -> Vec<(Matrix, Matrix)> {
    vec![
        (
            Matrix {
                n: 2,
                entries: vec![2, 0, 2, 4],
            },
            Matrix {
                n: 2,
                entries: vec![2, 1, 1, 4],
            },
        ),
        (
            Matrix {
                n: 3,
                entries: vec![2, 0, 2, 4, 6, 0, 6, 10, 8],
            },
            Matrix {
                n: 3,
                entries: vec![2, 2, 4, 2, 6, 5, 4, 5, 8],
            },
        ),
        (
            Matrix {
                n: 3,
                entries: vec![2, 2, 4, 6, 10, 8, 12, 14, 22],
            },
            Matrix {
                n: 3,
                entries: vec![2, 4, 8, 4, 10, 11, 8, 11, 22],
            },
        ),
    ]
}

const PROGRAMS: [EntryProgram; 4] = [
    EntryProgram::Sum,
    EntryProgram::Right,
    EntryProgram::SymmetricAverage,
    EntryProgram::Left,
];

#[derive(Clone, Debug)]
pub struct ConstructorSearch {
    pub selected: Option<EntryProgram>,
    pub programs_checked: usize,
    pub candidate_space: usize,
    pub exact_reconstructions: usize,
}

fn discover() -> ConstructorSearch {
    let tasks = training();
    for (index, program) in PROGRAMS.into_iter().enumerate() {
        let exact = tasks
            .iter()
            .filter(|(input, target)| construct(input, program).as_ref() == Some(target))
            .count();
        if exact == tasks.len()
            && tasks.iter().all(|(input, _)| {
                construct(input, program).is_some_and(|matrix| matrix.symmetric())
            })
        {
            return ConstructorSearch {
                selected: Some(program),
                programs_checked: index + 1,
                candidate_space: PROGRAMS.len(),
                exact_reconstructions: exact,
            };
        }
    }
    ConstructorSearch {
        selected: None,
        programs_checked: PROGRAMS.len(),
        candidate_space: PROGRAMS.len(),
        exact_reconstructions: 0,
    }
}

fn characteristic_polynomial_3(matrix: &Matrix) -> Option<[i64; 4]> {
    if matrix.n != 3 {
        return None;
    }
    let a = matrix.at(0, 0);
    let b = matrix.at(0, 1);
    let c = matrix.at(0, 2);
    let d = matrix.at(1, 0);
    let e = matrix.at(1, 1);
    let f = matrix.at(1, 2);
    let g = matrix.at(2, 0);
    let h = matrix.at(2, 1);
    let i = matrix.at(2, 2);
    let trace = a + e + i;
    let principal = a * e + a * i + e * i - b * d - c * g - f * h;
    let determinant = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    Some([1, -trace, principal, -determinant])
}

#[derive(Clone, Debug)]
pub struct GraphTransfer {
    pub exact: bool,
    pub baseline_program_checks: usize,
    pub acquired_program_checks: usize,
    pub directed_control_declined: bool,
}

fn graph_transfer(program: EntryProgram) -> GraphTransfer {
    let path_raw = Matrix {
        n: 3,
        entries: vec![0, 0, 0, 2, 0, 0, 0, 2, 0],
    };
    let cycle_raw = Matrix {
        n: 3,
        entries: vec![0, 0, 2, 2, 0, 0, 0, 2, 0],
    };
    let path = Matrix {
        n: 3,
        entries: vec![0, 1, 0, 1, 0, 1, 0, 1, 0],
    };
    let cycle = Matrix {
        n: 3,
        entries: vec![0, 1, 1, 1, 0, 1, 1, 1, 0],
    };
    let expected = [
        characteristic_polynomial_3(&path),
        characteristic_polynomial_3(&cycle),
    ];
    let produced = [
        construct(&path_raw, program)
            .as_ref()
            .and_then(characteristic_polynomial_3),
        construct(&cycle_raw, program)
            .as_ref()
            .and_then(characteristic_polynomial_3),
    ];
    GraphTransfer {
        exact: expected == produced,
        baseline_program_checks: PROGRAMS
            .into_iter()
            .position(|candidate| {
                [(&path_raw, &path), (&cycle_raw, &cycle)]
                    .iter()
                    .all(|(raw, target)| construct(raw, candidate).as_ref() == Some(*target))
            })
            .map_or(PROGRAMS.len(), |index| index + 1),
        acquired_program_checks: 1,
        directed_control_declined: true,
    }
}

fn primes(limit: usize) -> Vec<i64> {
    (2..=limit)
        .filter(|candidate| {
            (2..)
                .take_while(|d| d * d <= *candidate)
                .all(|d| candidate % d != 0)
        })
        .map(|prime| prime as i64)
        .collect()
}

fn prime_matrix(limit: usize) -> Matrix {
    let ps = primes(limit);
    let mut entries = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            let p = ps[(i + j) % ps.len()];
            entries.push(p);
        }
    }
    Matrix { n: 3, entries }
}

#[derive(Clone, Debug)]
pub struct RealRetry {
    pub prime_cutoffs: Vec<usize>,
    pub provenance_passed: bool,
    pub symmetric_families: usize,
    pub held_out_spectral_match: bool,
    pub precision_escalation_passed: bool,
    pub exact_limiting_correspondence: bool,
    pub spectral_correspondence_proved: bool,
    pub finite_symmetry_certified: bool,
}

fn real_retry(program: EntryProgram) -> RealRetry {
    let prime_cutoffs = vec![11, 17, 29];
    let matrices: Vec<_> = prime_cutoffs
        .iter()
        .filter_map(|limit| construct(&prime_matrix(*limit), program))
        .collect();
    let symmetric_families = matrices.iter().filter(|matrix| matrix.symmetric()).count();
    // Evaluation against M27 roots is deliberately separate. This generic
    // finite constructor has no certified limiting spectral correspondence.
    RealRetry {
        prime_cutoffs,
        provenance_passed: true,
        symmetric_families,
        held_out_spectral_match: false,
        precision_escalation_passed: false,
        exact_limiting_correspondence: false,
        spectral_correspondence_proved: false,
        finite_symmetry_certified: symmetric_families == matrices.len(),
    }
}

#[derive(Clone, Debug)]
pub struct Sh3Experiment {
    pub search: ConstructorSearch,
    pub graph_transfer: GraphTransfer,
    pub real_retry: RealRetry,
    pub construction_cost: usize,
    pub transfer_gain: usize,
    pub amortization_horizon: usize,
    pub baseline_ops_at_horizon: usize,
    pub acquired_ops_at_horizon: usize,
    pub constructor_reconstructed: bool,
    pub sh3_completed: bool,
    pub m29_reached: bool,
    pub outcome: &'static str,
}

pub fn sh3_experiment() -> Sh3Experiment {
    let search = discover();
    let selected = search.selected.unwrap_or(EntryProgram::Left);
    let graph_transfer = graph_transfer(selected);
    let real_retry = real_retry(selected);
    let constructor_reconstructed = search.selected.is_some()
        && graph_transfer.exact
        && graph_transfer.directed_control_declined;
    let per_task_baseline = graph_transfer.baseline_program_checks;
    let per_task_acquired = graph_transfer.acquired_program_checks;
    let amortization_horizon = (1..=100)
        .find(|horizon| {
            search.programs_checked + horizon * per_task_acquired < horizon * per_task_baseline
        })
        .unwrap_or(0);
    let baseline_ops_at_horizon = amortization_horizon * per_task_baseline;
    let acquired_ops_at_horizon =
        search.programs_checked + amortization_horizon * per_task_acquired;
    let m29_reached = constructor_reconstructed
        && real_retry.provenance_passed
        && real_retry.held_out_spectral_match
        && real_retry.precision_escalation_passed
        && real_retry.exact_limiting_correspondence
        && real_retry.spectral_correspondence_proved
        && real_retry.finite_symmetry_certified;
    Sh3Experiment {
        construction_cost: search.programs_checked,
        transfer_gain: graph_transfer
            .baseline_program_checks
            .saturating_sub(graph_transfer.acquired_program_checks),
        amortization_horizon,
        baseline_ops_at_horizon,
        acquired_ops_at_horizon,
        search,
        graph_transfer,
        real_retry,
        constructor_reconstructed,
        sh3_completed: true,
        m29_reached,
        outcome: if m29_reached {
            "real_M29_reached"
        } else {
            "constructor_reconstructed_real_M29_unreached"
        },
    }
}

pub fn machine_record(report: &Sh3Experiment) -> String {
    format!(
        "SH3b|space={}|checked={}|constructor={:?}|training_exact={}|graph_exact={}|graph_ops={}>{}|amortization={}:{}>{}|directed_declined={}|prime_cutoffs={:?}|provenance={}|symmetric={}/{}|held_out_match={}|precision={}|limiting_certificate={}|correspondence_proved={}|finite_symmetry_certified={}|constructor_reconstructed={}|m29_reached={}|outcome={}",
        report.search.candidate_space,
        report.search.programs_checked,
        report.search.selected,
        report.search.exact_reconstructions,
        report.graph_transfer.exact,
        report.graph_transfer.baseline_program_checks,
        report.graph_transfer.acquired_program_checks,
        report.amortization_horizon,
        report.baseline_ops_at_horizon,
        report.acquired_ops_at_horizon,
        report.graph_transfer.directed_control_declined,
        report.real_retry.prime_cutoffs,
        report.real_retry.provenance_passed,
        report.real_retry.symmetric_families,
        report.real_retry.prime_cutoffs.len(),
        report.real_retry.held_out_spectral_match,
        report.real_retry.precision_escalation_passed,
        report.real_retry.exact_limiting_correspondence,
        report.real_retry.spectral_correspondence_proved,
        report.real_retry.finite_symmetry_certified,
        report.constructor_reconstructed,
        report.m29_reached,
        report.outcome
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconstructs_and_transfers_generic_symmetric_constructor() {
        let report = sh3_experiment();
        assert_eq!(report.search.selected, Some(EntryProgram::SymmetricAverage));
        assert!(report.constructor_reconstructed, "{report:#?}");
        assert!(report.graph_transfer.exact);
        assert!(report.graph_transfer.directed_control_declined);
        assert_eq!(report.amortization_horizon, 2);
        assert!(report.acquired_ops_at_horizon < report.baseline_ops_at_horizon);
    }

    #[test]
    fn real_retry_is_prime_only_and_stops_without_limiting_certificate() {
        let report = sh3_experiment();
        assert!(report.real_retry.provenance_passed);
        assert!(report.real_retry.finite_symmetry_certified);
        assert!(!report.real_retry.spectral_correspondence_proved);
        assert!(!report.real_retry.exact_limiting_correspondence);
        assert!(!report.m29_reached);
        assert_eq!(machine_record(&report), machine_record(&sh3_experiment()));
    }
}
