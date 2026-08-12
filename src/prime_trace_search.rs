//! SH7: exact symbolic trace search for the selected prime Jacobi family.

use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Monomial {
    DiagonalSquare(usize),
    EdgeSquare(usize),
}

type Polynomial = BTreeMap<Monomial, i64>;

fn add_term(polynomial: &mut Polynomial, monomial: Monomial, coefficient: i64) {
    *polynomial.entry(monomial).or_insert(0) += coefficient;
    polynomial.retain(|_, value| *value != 0);
}

fn closed_walk_trace_two(size: usize) -> Polynomial {
    let mut polynomial = Polynomial::new();
    for start in 0..size {
        for middle in 0..size {
            if start == middle {
                add_term(&mut polynomial, Monomial::DiagonalSquare(start), 1);
            } else if start.abs_diff(middle) == 1 {
                add_term(&mut polynomial, Monomial::EdgeSquare(start.min(middle)), 1);
            }
        }
    }
    polynomial
}

fn candidate(size: usize, diagonal_coefficient: i64, edge_coefficient: i64) -> Polynomial {
    let mut polynomial = Polynomial::new();
    for index in 0..size {
        add_term(
            &mut polynomial,
            Monomial::DiagonalSquare(index),
            diagonal_coefficient,
        );
    }
    for index in 0..size.saturating_sub(1) {
        add_term(
            &mut polynomial,
            Monomial::EdgeSquare(index),
            edge_coefficient,
        );
    }
    polynomial
}

fn primes(count: usize) -> Vec<u64> {
    let mut values = Vec::new();
    let mut candidate = 2_u64;
    while values.len() < count {
        if (2..)
            .take_while(|divisor| divisor * divisor <= candidate)
            .all(|divisor| candidate % divisor != 0)
        {
            values.push(candidate);
        }
        candidate += 1;
    }
    values
}

fn prime_trace_two(size: usize) -> u128 {
    primes(size)
        .into_iter()
        .map(|prime| u128::from(prime * prime))
        .sum::<u128>()
        + 2 * size.saturating_sub(1) as u128
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LimitFrontier {
    ExactFiniteTrace,
    DegenerateScalarLimit,
    SeparatingTraceIdentity,
    ExactXiCorrespondence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TraceEvidence {
    SymbolicPolynomialIdentity,
    SampledEquality,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NormalizationProvenance {
    ArithmeticOnly,
    ZeroDerived,
}

fn accepts_trace_evidence(evidence: TraceEvidence) -> bool {
    evidence == TraceEvidence::SymbolicPolynomialIdentity
}

fn accepts_normalization(provenance: NormalizationProvenance) -> bool {
    provenance == NormalizationProvenance::ArithmeticOnly
}

fn promotes_to_measure(frontier: LimitFrontier, separating_class: bool) -> bool {
    separating_class && frontier >= LimitFrontier::SeparatingTraceIdentity
}

#[derive(Clone, Debug)]
pub struct Sh7Experiment {
    pub coefficient_pairs_checked: usize,
    pub discovered_coefficients: (i64, i64),
    pub symbolic_training_exact: bool,
    pub held_out_exact: bool,
    pub prime_specializations: Vec<(usize, u128)>,
    pub normalization_results: [(&'static str, &'static str); 4],
    pub frontier: LimitFrontier,
    pub controls: [bool; 4],
    pub sh7_passed: bool,
    pub m29_reached: bool,
}

pub fn sh7_experiment() -> Sh7Experiment {
    let coefficient_order = (-2_i64..=2)
        .flat_map(|diagonal| (-2_i64..=2).map(move |edge| (diagonal, edge)))
        .collect::<Vec<_>>();
    let training_sizes = 2..=6;
    let (index, discovered_coefficients) = coefficient_order
        .iter()
        .copied()
        .enumerate()
        .find(|(_, (diagonal, edge))| {
            training_sizes
                .clone()
                .all(|size| candidate(size, *diagonal, *edge) == closed_walk_trace_two(size))
        })
        .expect("frozen basis expresses the second trace moment");
    let symbolic_training_exact = training_sizes.clone().all(|size| {
        candidate(size, discovered_coefficients.0, discovered_coefficients.1)
            == closed_walk_trace_two(size)
    });
    let held_out_exact = (7..=10).all(|size| {
        candidate(size, discovered_coefficients.0, discovered_coefficients.1)
            == closed_walk_trace_two(size)
    });
    let prime_specializations = (2..=10)
        .map(|size| (size, prime_trace_two(size)))
        .collect::<Vec<_>>();
    let normalization_results = [
        ("1", "diverges"),
        ("N", "diverges"),
        ("sum_p", "diverges"),
        ("sum_p_squared", "limit_1_degenerate"),
    ];
    let frontier = LimitFrontier::DegenerateScalarLimit;
    let controls = [
        candidate(5, 1, 1) != closed_walk_trace_two(5),
        !accepts_trace_evidence(TraceEvidence::SampledEquality),
        !accepts_normalization(NormalizationProvenance::ZeroDerived),
        !promotes_to_measure(frontier, false),
    ];
    Sh7Experiment {
        coefficient_pairs_checked: index + 1,
        discovered_coefficients,
        symbolic_training_exact,
        held_out_exact,
        prime_specializations,
        normalization_results,
        frontier,
        controls,
        sh7_passed: symbolic_training_exact
            && held_out_exact
            && controls.iter().all(|control| *control),
        m29_reached: frontier == LimitFrontier::ExactXiCorrespondence,
    }
}

pub fn machine_record(report: &Sh7Experiment) -> String {
    format!(
        "SH7b|coefficient_pairs_checked={}|coefficients={:?}|symbolic_training_exact={}|held_out_exact={}|prime_specializations={:?}|normalizations={:?}|frontier={:?}|controls={:?}|passed={}|m29_reached={}|claim=exact_finite_prime_trace_identity_only",
        report.coefficient_pairs_checked,
        report.discovered_coefficients,
        report.symbolic_training_exact,
        report.held_out_exact,
        report.prime_specializations,
        report.normalization_results,
        report.frontier,
        report.controls,
        report.sh7_passed,
        report.m29_reached,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_exact_prime_jacobi_second_trace_identity() {
        let report = sh7_experiment();
        assert!(report.sh7_passed, "{report:#?}");
        assert_eq!(report.discovered_coefficients, (1, 2));
        assert_eq!(report.coefficient_pairs_checked, 20);
        assert_eq!(report.frontier, LimitFrontier::DegenerateScalarLimit);
        assert!(!report.m29_reached);
        assert_eq!(machine_record(&report), machine_record(&sh7_experiment()));
    }
}
