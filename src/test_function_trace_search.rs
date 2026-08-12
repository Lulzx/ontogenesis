//! SH8: exact even-polynomial trace identities for prime Jacobi truncations.

use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Monomial(Vec<u8>);

#[derive(Clone, Debug, Eq, PartialEq)]
struct Polynomial(BTreeMap<Monomial, u64>);

impl Polynomial {
    fn zero() -> Self {
        Self(BTreeMap::new())
    }

    fn one(variable_count: usize) -> Self {
        Self(BTreeMap::from([(Monomial(vec![0; variable_count]), 1)]))
    }

    fn variable(variable_count: usize, index: usize) -> Self {
        let mut powers = vec![0; variable_count];
        powers[index] = 1;
        Self(BTreeMap::from([(Monomial(powers), 1)]))
    }

    fn add_assign(&mut self, other: &Self) {
        for (monomial, coefficient) in &other.0 {
            *self.0.entry(monomial.clone()).or_insert(0) += coefficient;
        }
    }

    fn multiply(&self, other: &Self) -> Self {
        let mut product = Self::zero();
        for (left, left_coefficient) in &self.0 {
            for (right, right_coefficient) in &other.0 {
                let powers = left
                    .0
                    .iter()
                    .zip(&right.0)
                    .map(|(left, right)| left + right)
                    .collect();
                *product.0.entry(Monomial(powers)).or_insert(0) +=
                    left_coefficient * right_coefficient;
            }
        }
        product
    }
}

fn jacobi_matrix(size: usize) -> Vec<Vec<Polynomial>> {
    let variable_count = 2 * size - 1;
    (0..size)
        .map(|row| {
            (0..size)
                .map(|column| {
                    if row == column {
                        Polynomial::variable(variable_count, row)
                    } else if row.abs_diff(column) == 1 {
                        Polynomial::variable(variable_count, size + row.min(column))
                    } else {
                        Polynomial::zero()
                    }
                })
                .collect()
        })
        .collect()
}

fn matrix_multiply(left: &[Vec<Polynomial>], right: &[Vec<Polynomial>]) -> Vec<Vec<Polynomial>> {
    let size = left.len();
    (0..size)
        .map(|row| {
            (0..size)
                .map(|column| {
                    let mut value = Polynomial::zero();
                    for middle in 0..size {
                        value.add_assign(&left[row][middle].multiply(&right[middle][column]));
                    }
                    value
                })
                .collect()
        })
        .collect()
}

fn independent_trace_power(size: usize, power: usize) -> Polynomial {
    let matrix = jacobi_matrix(size);
    let variable_count = 2 * size - 1;
    let mut product = (0..size)
        .map(|row| {
            (0..size)
                .map(|column| {
                    if row == column {
                        Polynomial::one(variable_count)
                    } else {
                        Polynomial::zero()
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for _ in 0..power {
        product = matrix_multiply(&product, &matrix);
    }
    let mut trace = Polynomial::zero();
    for (index, row) in product.iter().enumerate() {
        trace.add_assign(&row[index]);
    }
    trace
}

fn walk_trace(size: usize, power: usize, omit_last_start: bool) -> Polynomial {
    let matrix = jacobi_matrix(size);
    let mut trace = Polynomial::zero();
    let start_limit = if omit_last_start { size - 1 } else { size };
    for start in 0..start_limit {
        let mut states = vec![(start, Polynomial::one(2 * size - 1))];
        for _ in 0..power {
            let mut next = Vec::new();
            for (vertex, weight) in states {
                for target in vertex.saturating_sub(1)..=(vertex + 1).min(size - 1) {
                    next.push((target, weight.multiply(&matrix[vertex][target])));
                }
            }
            states = next;
        }
        for (_, weight) in states.into_iter().filter(|(vertex, _)| *vertex == start) {
            trace.add_assign(&weight);
        }
    }
    trace
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Provenance {
    ArithmeticOnly,
    ZeroDerived,
}

fn normalization_allowed(provenance: Provenance) -> bool {
    provenance == Provenance::ArithmeticOnly
}

fn finite_class_promotes_measure(class_size: usize) -> bool {
    class_size == usize::MAX
}

#[derive(Clone, Debug)]
pub struct MomentResult {
    pub degree: usize,
    pub training_exact: bool,
    pub held_out_exact: bool,
    pub monomial_count_at_holdout: usize,
    pub proposed_limit: String,
    pub limit_certified: bool,
}

#[derive(Clone, Debug)]
pub struct Sh8Experiment {
    pub moments: Vec<MomentResult>,
    pub controls: [bool; 4],
    pub controls_declined: usize,
    pub common_normalization: &'static str,
    pub finite_distribution_certified: bool,
    pub sh8_completed: bool,
    pub m29_reached: bool,
    pub outcome: &'static str,
}

pub fn sh8_experiment() -> Sh8Experiment {
    let moments = [2, 4, 6]
        .into_iter()
        .map(|degree| {
            let training_exact = (2..=4).all(|size| {
                walk_trace(size, degree, false) == independent_trace_power(size, degree)
            });
            let held_out = independent_trace_power(5, degree);
            let held_out_exact = walk_trace(5, degree, false) == held_out;
            MomentResult {
                degree,
                training_exact,
                held_out_exact,
                monomial_count_at_holdout: held_out.0.len(),
                proposed_limit: format!("1/{}", degree + 1),
                // SH8 has no prime-asymptotic or perturbation theorem.
                limit_certified: false,
            }
        })
        .collect::<Vec<_>>();
    let controls = [
        walk_trace(4, 4, false) != {
            let mut corrupted = independent_trace_power(4, 4);
            if let Some(coefficient) = corrupted.0.values_mut().next() {
                *coefficient += 1;
            }
            corrupted
        },
        walk_trace(4, 4, true) != independent_trace_power(4, 4),
        !normalization_allowed(Provenance::ZeroDerived),
        !finite_class_promotes_measure(moments.len()),
    ];
    let controls_declined = controls.iter().filter(|control| **control).count();
    let finite_distribution_certified = moments.iter().all(|moment| moment.limit_certified);
    Sh8Experiment {
        sh8_completed: moments
            .iter()
            .all(|moment| moment.training_exact && moment.held_out_exact)
            && controls_declined == controls.len(),
        moments,
        controls,
        controls_declined,
        common_normalization: "empirical spectrum of J_N / p_N",
        finite_distribution_certified,
        m29_reached: false,
        outcome: "exact_finite_test_function_identities_limit_certificate_missing",
    }
}

pub fn machine_record(report: &Sh8Experiment) -> String {
    let moments = report
        .moments
        .iter()
        .map(|moment| {
            format!(
                "degree{}:train={}:holdout={}:monomials={}:proposed_limit={}:certified={}",
                moment.degree,
                moment.training_exact,
                moment.held_out_exact,
                moment.monomial_count_at_holdout,
                moment.proposed_limit,
                moment.limit_certified,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "SH8|moments=[{}]|normalization={}|finite_distribution_certified={}|controls={:?}|controls_declined={}/4|passed={}|m29_reached={}|outcome={}",
        moments,
        report.common_normalization,
        report.finite_distribution_certified,
        report.controls,
        report.controls_declined,
        report.sh8_completed,
        report.m29_reached,
        report.outcome,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_exact_even_test_function_identities() {
        let report = sh8_experiment();
        assert!(report.sh8_completed, "{report:#?}");
        assert_eq!(report.moments.len(), 3);
        assert!(report
            .moments
            .iter()
            .all(|moment| moment.training_exact && moment.held_out_exact));
        assert!(!report.finite_distribution_certified);
        assert!(!report.m29_reached);
        assert_eq!(machine_record(&report), machine_record(&sh8_experiment()));
    }
}
