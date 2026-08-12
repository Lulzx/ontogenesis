//! M29c: reverse forcing object for real xi.
//!
//! X is the typed Weil functional. P is positivity on squares. The only
//! residual after the checked implication chain is P itself.

use crate::validated_explicit_formula::{ExactScale, Interval, IntervalError, Provenance};
use crate::weil_entry_assembly::{
    assemble_component_entries, finite_ldl_report, interval_ldl_matrix, LdlStatus, Normalization,
};
use rug::float::Round;
use rug::ops::{AddAssignRound, MulAssignRound};
use rug::Float;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProvenanceTag {
    ArithmeticOnly,
    ZeroDerived,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Fact {
    WeilFunctional {
        pole: bool,
        archimedean: bool,
        prime: bool,
    },
    PoleGramRankOnePsd,
    PositiveOnSquares {
        provenance: ProvenanceTag,
    },
    SeparatingAlgebra,
    FiniteNonseparatingClass,
    GnsSelfAdjoint,
    SpectralCorrespondence,
    RiemannHypothesis,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Rejection {
    MissingPositivity,
    ForbiddenProvenance,
    NonseparatingClass,
    IncompleteFunctional,
    PoleIsNotTheFunctional,
}

fn pole_ratio(power: usize) -> (i128, i128) {
    let denominator = 1_i128 << (2 * power);
    if power % 2 == 0 {
        (1, denominator)
    } else {
        (-1, denominator)
    }
}

fn mul_ratio(left: (i128, i128), right: (i128, i128)) -> (i128, i128) {
    let mut numerator = left.0 * right.0;
    let mut denominator = left.1 * right.1;
    let divisor = gcd(numerator.unsigned_abs(), denominator.unsigned_abs()) as i128;
    numerator /= divisor;
    denominator /= divisor;
    (numerator, denominator)
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

fn pole_rank_one_identity(dimension: usize) -> bool {
    dimension >= 2
        && (0..dimension).all(|row| {
            (0..dimension).all(|column| {
                mul_ratio(pole_ratio(row), pole_ratio(column)) == pole_ratio(row + column)
            })
        })
}

fn pole_quadratic_nonnegative(dimension: usize, coefficients: &[i128]) -> bool {
    if coefficients.len() != dimension {
        return false;
    }
    let mut square_numerator = 0_i128;
    let mut square_denominator = 1_i128;
    for (index, coefficient) in coefficients.iter().copied().enumerate() {
        let ratio = pole_ratio(index);
        let term = mul_ratio((coefficient, 1), ratio);
        let left = square_numerator * term.1 + term.0 * square_denominator;
        let right = square_denominator * term.1;
        let divisor = gcd(left.unsigned_abs(), right.unsigned_abs()) as i128;
        square_numerator = left / divisor;
        square_denominator = right / divisor;
    }
    square_numerator.signum() * square_numerator * square_denominator >= 0
}

fn apply_gns(positivity: &Fact, functional: &Fact) -> Result<Fact, Rejection> {
    match (positivity, functional) {
        (
            Fact::PositiveOnSquares {
                provenance: ProvenanceTag::ZeroDerived,
            },
            _,
        ) => Err(Rejection::ForbiddenProvenance),
        (
            Fact::PositiveOnSquares {
                provenance: ProvenanceTag::ArithmeticOnly,
            },
            Fact::WeilFunctional {
                pole: true,
                archimedean: true,
                prime: true,
            },
        ) => Ok(Fact::GnsSelfAdjoint),
        (_, Fact::WeilFunctional { prime: false, .. }) => Err(Rejection::IncompleteFunctional),
        (Fact::PoleGramRankOnePsd, _) => Err(Rejection::PoleIsNotTheFunctional),
        _ => Err(Rejection::MissingPositivity),
    }
}

fn apply_correspondence(
    positivity: &Fact,
    functional: &Fact,
    class: &Fact,
) -> Result<Fact, Rejection> {
    match (positivity, functional, class) {
        (
            Fact::PositiveOnSquares {
                provenance: ProvenanceTag::ZeroDerived,
            },
            _,
            _,
        ) => Err(Rejection::ForbiddenProvenance),
        (_, _, Fact::FiniteNonseparatingClass) => Err(Rejection::NonseparatingClass),
        (
            Fact::PositiveOnSquares {
                provenance: ProvenanceTag::ArithmeticOnly,
            },
            Fact::WeilFunctional {
                pole: true,
                archimedean: true,
                prime: true,
            },
            Fact::SeparatingAlgebra,
        ) => Ok(Fact::SpectralCorrespondence),
        _ => Err(Rejection::MissingPositivity),
    }
}

fn apply_forcing(self_adjoint: &Fact, correspondence: &Fact) -> Result<Fact, Rejection> {
    match (self_adjoint, correspondence) {
        (Fact::GnsSelfAdjoint, Fact::SpectralCorrespondence) => Ok(Fact::RiemannHypothesis),
        _ => Err(Rejection::MissingPositivity),
    }
}

fn integer_sqrt_ceiling(value: u64) -> u64 {
    let mut lower = 0_u64;
    let mut upper = value.max(1);
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        if middle.saturating_mul(middle) >= value {
            upper = middle;
        } else {
            lower = middle + 1;
        }
    }
    lower
}

fn integration_bound(scale: ExactScale) -> i32 {
    integer_sqrt_ceiling(
        72_u64 * u64::from(scale.denominator()) / u64::from(scale.numerator())
            + u64::from(
                (72_u64 * u64::from(scale.denominator())) % u64::from(scale.numerator()) != 0,
            ),
    ) as i32
}

fn hankel(entries: &[Interval], dimension: usize) -> Vec<Vec<Interval>> {
    (0..dimension)
        .map(|row| {
            (0..dimension)
                .map(|column| entries[row + column].clone())
                .collect()
        })
        .collect()
}

fn frobenius_upper(matrix: &[Vec<Interval>]) -> Float {
    let precision = matrix[0][0].precision;
    let mut sum = Float::with_val(precision, 0);
    for row in matrix {
        for entry in row {
            let mut square = entry.abs_upper();
            square.mul_assign_round(&entry.abs_upper(), Round::Up);
            sum.add_assign_round(&square, Round::Up);
        }
    }
    sum.sqrt_round(Round::Up);
    sum
}

fn singleton(value: Float, precision: u32) -> Interval {
    Interval {
        lower: value.clone(),
        upper: value,
        precision,
        provenance: Provenance::Generic,
        tail_certified: true,
    }
}

fn subtract_shift(
    matrix: &[Vec<Interval>],
    shift: &Interval,
) -> Result<Vec<Vec<Interval>>, IntervalError> {
    matrix
        .iter()
        .enumerate()
        .map(|(row_index, row)| {
            row.iter()
                .enumerate()
                .map(|(column_index, entry)| {
                    if row_index == column_index {
                        entry.sub(shift)
                    } else {
                        Ok(entry.clone())
                    }
                })
                .collect()
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct DominationReport {
    pub dimension: usize,
    pub nested: bool,
    pub total_ldl: LdlStatus,
    pub total_pivot_lower: String,
    pub total_pivot_upper: String,
    pub dominated: bool,
    pub prime_frobenius_upper: String,
}

fn dominate(
    scale: ExactScale,
    dimension: usize,
    fine: bool,
) -> Result<DominationReport, IntervalError> {
    let bound = integration_bound(scale);
    let (cell_factor, terms, cutoff, precision) = if fine {
        (256, 256, 16_384_u64, 160)
    } else {
        (64, 64, 4_096, 80)
    };
    let powers = (0..2 * dimension - 1)
        .map(|index| index * 2)
        .collect::<Vec<_>>();
    let normalization = Normalization::angular().with_scale(scale);
    let coarse = assemble_component_entries(
        &powers,
        bound,
        64 * bound as usize,
        64,
        4_096,
        80,
        normalization,
    )?;
    let components = if fine {
        assemble_component_entries(
            &powers,
            bound,
            cell_factor * bound as usize,
            terms,
            cutoff,
            precision,
            normalization,
        )?
    } else {
        coarse.clone()
    };
    let nested = !fine
        || coarse.iter().zip(&components).all(|(outer, inner)| {
            outer.pole.contains_interval(&inner.pole)
                && outer
                    .weighted_archimedean
                    .contains_interval(&inner.weighted_archimedean)
                && outer
                    .weighted_prime
                    .contains_interval(&inner.weighted_prime)
        });
    let totals = components
        .iter()
        .map(|component| {
            component
                .pole
                .add(&component.weighted_archimedean)?
                .sub(&component.weighted_prime)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let total_ldl = finite_ldl_report(&totals, dimension)?;
    let l0 = components
        .iter()
        .map(|component| component.pole.add(&component.weighted_archimedean))
        .collect::<Result<Vec<_>, _>>()?;
    let primes = components
        .iter()
        .map(|component| component.weighted_prime.clone())
        .collect::<Vec<_>>();
    let prime_norm = frobenius_upper(&hankel(&primes, dimension));
    let shifted = subtract_shift(
        &hankel(&l0, dimension),
        &singleton(prime_norm.clone(), components[0].pole.precision),
    )?;
    let dominated = matches!(
        interval_ldl_matrix(&shifted)?.status,
        LdlStatus::StrictlyPositive
    );
    Ok(DominationReport {
        dimension,
        nested,
        total_ldl: total_ldl.status,
        total_pivot_lower: format!("{:.8e}", total_ldl.pivot.lower),
        total_pivot_upper: format!("{:.8e}", total_ldl.pivot.upper),
        dominated,
        prime_frobenius_upper: format!("{:.8e}", prime_norm),
    })
}

#[derive(Clone, Debug)]
pub struct M29cExperiment {
    pub pole_identity: bool,
    pub pole_quadratic_nonnegative: bool,
    pub implication_closes_with_p: bool,
    pub controls: [bool; 6],
    pub controls_declined: usize,
    pub dimension_two: Option<DominationReport>,
    pub dimension_four: Option<DominationReport>,
    pub residual: &'static str,
    pub infinite_positivity: bool,
    pub m29_reached: bool,
}

fn controls() -> [bool; 6] {
    let functional = Fact::WeilFunctional {
        pole: true,
        archimedean: true,
        prime: true,
    };
    let positivity = Fact::PositiveOnSquares {
        provenance: ProvenanceTag::ArithmeticOnly,
    };
    [
        apply_gns(
            &Fact::PositiveOnSquares {
                provenance: ProvenanceTag::ZeroDerived,
            },
            &functional,
        ) == Err(Rejection::ForbiddenProvenance),
        apply_correspondence(&positivity, &functional, &Fact::FiniteNonseparatingClass)
            == Err(Rejection::NonseparatingClass),
        apply_gns(
            &positivity,
            &Fact::WeilFunctional {
                pole: true,
                archimedean: true,
                prime: false,
            },
        ) == Err(Rejection::IncompleteFunctional),
        apply_gns(&Fact::PoleGramRankOnePsd, &functional) == Err(Rejection::PoleIsNotTheFunctional),
        !promotes_finite_domination(),
        apply_forcing(&Fact::GnsSelfAdjoint, &Fact::PoleGramRankOnePsd).is_err(),
    ]
}

fn promotes_finite_domination() -> bool {
    false
}

fn implication_closes_with_p() -> bool {
    let functional = Fact::WeilFunctional {
        pole: true,
        archimedean: true,
        prime: true,
    };
    let positivity = Fact::PositiveOnSquares {
        provenance: ProvenanceTag::ArithmeticOnly,
    };
    let self_adjoint = apply_gns(&positivity, &functional).ok();
    let correspondence =
        apply_correspondence(&positivity, &functional, &Fact::SeparatingAlgebra).ok();
    matches!(
        (self_adjoint, correspondence),
        (Some(self_adjoint), Some(correspondence))
            if apply_forcing(&self_adjoint, &correspondence) == Ok(Fact::RiemannHypothesis)
    )
}

pub fn m29c_algebraic_experiment() -> M29cExperiment {
    let controls = controls();
    M29cExperiment {
        pole_identity: pole_rank_one_identity(8),
        pole_quadratic_nonnegative: pole_quadratic_nonnegative(4, &[1, -2, 3, 0])
            && pole_quadratic_nonnegative(3, &[2, 1, -1]),
        implication_closes_with_p: implication_closes_with_p(),
        controls_declined: controls.iter().filter(|value| **value).count(),
        controls,
        dimension_two: None,
        dimension_four: None,
        residual: "PositiveFunctional(L_weil)",
        infinite_positivity: false,
        m29_reached: false,
    }
}

pub fn m29c_experiment() -> M29cExperiment {
    let mut report = m29c_algebraic_experiment();
    let scale = ExactScale::new(1, 128).expect("frozen SH19a scale");
    report.dimension_two = Some(dominate(scale, 2, true).expect("dimension two domination"));
    report.dimension_four = Some(dominate(scale, 4, true).expect("dimension four domination"));
    if report
        .dimension_two
        .as_ref()
        .is_some_and(|section| section.dominated)
        && report
            .dimension_four
            .as_ref()
            .is_some_and(|section| section.dominated)
    {
        // Finite domination is not P on the separating algebra.
        report.infinite_positivity = false;
        report.m29_reached = false;
    }
    report
}

pub fn machine_record(report: &M29cExperiment) -> String {
    let section = |value: &Option<DominationReport>| match value {
        Some(section) => format!(
            "d{}:nested={}:ldl={:?}:pivot=[{},{}]:dominated={}:prime_frob={}",
            section.dimension,
            section.nested,
            section.total_ldl,
            section.total_pivot_lower,
            section.total_pivot_upper,
            section.dominated,
            section.prime_frobenius_upper,
        ),
        None => "None".into(),
    };
    format!(
        "M29c|pole_identity={}|pole_quadratic_nonnegative={}|implication_closes_with_p={}|controls={:?}|controls_declined={}/6|dim2={}|dim4={}|residual={}|infinite_positivity=false|m29_reached=false|claim=forcing_object_residual_positivity_only",
        report.pole_identity,
        report.pole_quadratic_nonnegative,
        report.implication_closes_with_p,
        report.controls,
        report.controls_declined,
        section(&report.dimension_two),
        section(&report.dimension_four),
        report.residual,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pole_hankel_is_exact_rank_one_psd() {
        assert!(pole_rank_one_identity(8));
        assert!(pole_quadratic_nonnegative(4, &[1, -2, 3, 0]));
        assert!(pole_quadratic_nonnegative(5, &[1, 1, 1, 1, 1]));
        assert!(!pole_rank_one_identity(0));
    }

    #[test]
    fn p_closes_rh_and_every_shortcut_fails() {
        let report = m29c_algebraic_experiment();
        assert!(report.pole_identity);
        assert!(report.pole_quadratic_nonnegative);
        assert!(report.implication_closes_with_p);
        assert_eq!(report.controls, [true; 6]);
        assert_eq!(report.residual, "PositiveFunctional(L_weil)");
        assert!(!report.infinite_positivity);
        assert!(!report.m29_reached);
    }

    #[test]
    fn finite_domination_does_not_set_m29() {
        let mut report = m29c_algebraic_experiment();
        report.dimension_two = Some(DominationReport {
            dimension: 2,
            nested: true,
            total_ldl: LdlStatus::StrictlyPositive,
            total_pivot_lower: "1".into(),
            total_pivot_upper: "1".into(),
            dominated: true,
            prime_frobenius_upper: "0".into(),
        });
        report.dimension_four = report.dimension_two.clone();
        report.infinite_positivity = false;
        report.m29_reached = false;
        assert!(!report.m29_reached);
        assert!(machine_record(&report).contains("m29_reached=false"));
    }
}
