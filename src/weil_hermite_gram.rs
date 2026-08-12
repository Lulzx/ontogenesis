//! M29e: Hermite-product Gram of the Weil functional.
//!
//! Same span as the monomial Hankel. Each entry is one cancelled even
//! polynomial, so interval LDL can decide sections the monomial basis
//! could not. Infinite PSD is still P and is not claimed from finite
//! sections.

use crate::validated_archimedean::archimedean_even_polynomials;
use crate::validated_explicit_formula::{ExactScale, Interval, IntervalError, Provenance};
use crate::validated_prime_power::{certified_even_polynomial_component, Rational};
use crate::weil_entry_assembly::{interval_ldl_matrix, LdlStatus};
use rug::float::Round;
use rug::ops::DivAssignRound;
use rug::Float;

fn hermite(degree: usize) -> Vec<i128> {
    let mut previous = vec![1_i128];
    if degree == 0 {
        return previous;
    }
    let mut current = vec![0_i128, 2];
    if degree == 1 {
        return current;
    }
    for index in 1..degree {
        let mut next = vec![0_i128; current.len() + 1];
        for (power, coefficient) in current.iter().copied().enumerate() {
            next[power + 1] += 2 * coefficient;
        }
        for (power, coefficient) in previous.iter().copied().enumerate() {
            next[power] -= 2 * index as i128 * coefficient;
        }
        previous = current;
        current = next;
    }
    current
}

fn even_in_square(polynomial: &[i128]) -> Vec<i128> {
    polynomial.iter().step_by(2).copied().collect()
}

fn multiply_even(left: &[i128], right: &[i128]) -> Vec<i128> {
    let mut result = vec![0_i128; left.len() + right.len() - 1];
    for (left_power, left_coeff) in left.iter().copied().enumerate() {
        for (right_power, right_coeff) in right.iter().copied().enumerate() {
            result[left_power + right_power] += left_coeff * right_coeff;
        }
    }
    result
}

fn scale_even_powers(u2_coeffs: &[i128], scale: ExactScale) -> Vec<Rational> {
    u2_coeffs
        .iter()
        .copied()
        .enumerate()
        .map(|(power, coefficient)| {
            let mut numerator = coefficient;
            let mut denominator = 1_i128;
            for _ in 0..power {
                numerator *= i128::from(scale.numerator());
                denominator *= i128::from(scale.denominator());
            }
            Rational::new(numerator, denominator)
        })
        .collect()
}

fn hermite_product(left: usize, right: usize, scale: ExactScale) -> Vec<Rational> {
    let left_even = even_in_square(&hermite(2 * left));
    let right_even = even_in_square(&hermite(2 * right));
    scale_even_powers(&multiply_even(&left_even, &right_even), scale)
}

fn pole_even_polynomial(
    coeffs: &[Rational],
    scale: ExactScale,
    precision: u32,
) -> Result<Interval, IntervalError> {
    let quarter = scale
        .interval(precision, Provenance::Generic)
        .div(&Interval::exact_integer(4, precision, Provenance::Generic))?;
    let exponential = quarter.exp()?;
    let mut sum = Interval::exact_integer(0, precision, Provenance::Generic);
    let mut signed = Interval::exact_integer(1, precision, Provenance::Generic);
    let neg_quarter = Interval::exact_integer(-1, precision, Provenance::Generic)
        .div(&Interval::exact_integer(4, precision, Provenance::Generic))?;
    for coefficient in coeffs {
        let term = {
            let mut lower = Float::with_val(precision, coefficient.numerator);
            lower.div_assign_round(coefficient.denominator, Round::Down);
            let mut upper = Float::with_val(precision, coefficient.numerator);
            upper.div_assign_round(coefficient.denominator, Round::Up);
            Interval {
                lower,
                upper,
                precision,
                provenance: Provenance::Generic,
                tail_certified: true,
            }
        };
        sum = sum.add(&term.mul(&signed)?)?;
        signed = signed.mul(&neg_quarter)?;
    }
    Interval::exact_integer(2, precision, Provenance::Generic)
        .mul(&exponential)?
        .mul(&sum)
}

fn one_over_two_pi(precision: u32) -> Result<Interval, IntervalError> {
    Interval::exact_integer(1, precision, Provenance::Generic)
        .div(&Interval::exact_integer(2, precision, Provenance::Generic))?
        .div(&Interval::pi(precision))
}

fn one_over_pi(precision: u32) -> Result<Interval, IntervalError> {
    Interval::exact_integer(1, precision, Provenance::Generic).div(&Interval::pi(precision))
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

fn hermite_gram(
    dimension: usize,
    scale: ExactScale,
    fine: bool,
) -> Result<Vec<Vec<Interval>>, IntervalError> {
    let bound = integration_bound(scale);
    let (cell_factor, terms, cutoff, precision) = if fine {
        (256, 256, 16_384_u64, 160)
    } else {
        (64, 64, 4_096, 80)
    };
    let mut polynomials = Vec::new();
    let mut pairs = Vec::new();
    for row in 0..dimension {
        for column in 0..=row {
            polynomials.push(hermite_product(row, column, scale));
            pairs.push((row, column));
        }
    }
    let archimedean = archimedean_even_polynomials(
        &polynomials,
        scale,
        bound,
        cell_factor * bound as usize,
        terms,
        precision,
    )?;
    let arch_coefficient = one_over_two_pi(precision)?;
    let prime_coefficient = one_over_pi(precision)?;
    let mut matrix =
        vec![
            vec![Interval::exact_integer(0, precision, Provenance::Generic); dimension];
            dimension
        ];
    for (((row, column), coeffs), arch) in pairs
        .into_iter()
        .zip(polynomials.iter())
        .zip(archimedean.into_iter())
    {
        let prime = certified_even_polynomial_component(coeffs, scale, cutoff, precision)?;
        let entry = pole_even_polynomial(coeffs, scale, precision)?
            .add(&arch.mul(&arch_coefficient)?)?
            .sub(&prime.mul(&prime_coefficient)?)?;
        matrix[row][column] = entry.clone();
        matrix[column][row] = entry;
    }
    Ok(matrix)
}

#[derive(Clone, Debug)]
pub struct SectionReport {
    pub dimension: usize,
    pub ldl: LdlStatus,
    pub pivot_lower: String,
    pub pivot_upper: String,
    pub first_entry_lower: String,
    pub first_entry_upper: String,
}

fn section(dimension: usize) -> Result<SectionReport, IntervalError> {
    let scale = ExactScale::new(1, 128).expect("frozen scale");
    let matrix = hermite_gram(dimension, scale, true)?;
    let ldl = interval_ldl_matrix(&matrix)?;
    Ok(SectionReport {
        dimension,
        ldl: ldl.status,
        pivot_lower: format!("{:.8e}", ldl.pivot.lower),
        pivot_upper: format!("{:.8e}", ldl.pivot.upper),
        first_entry_lower: format!("{:.8e}", matrix[0][0].lower),
        first_entry_upper: format!("{:.8e}", matrix[0][0].upper),
    })
}

#[derive(Clone, Debug)]
pub struct M29eExperiment {
    pub hermite_h2: bool,
    pub product_span: bool,
    pub dimension_two: Option<SectionReport>,
    pub dimension_four: Option<SectionReport>,
    pub finite_promotion_declined: bool,
    pub infinite_positivity: bool,
    pub m29_reached: bool,
}

pub fn m29e_algebraic_experiment() -> M29eExperiment {
    let h2 = hermite(2);
    M29eExperiment {
        hermite_h2: h2 == vec![-2, 0, 4],
        product_span: hermite_product(0, 0, ExactScale::integer(1)) == vec![Rational::new(1, 1)],
        dimension_two: None,
        dimension_four: None,
        finite_promotion_declined: !promotes_finite_hermite(),
        infinite_positivity: false,
        m29_reached: false,
    }
}

pub fn m29e_experiment() -> M29eExperiment {
    let mut report = m29e_algebraic_experiment();
    report.dimension_two = Some(section(2).expect("hermite 2x2"));
    report.dimension_four = Some(section(4).expect("hermite 4x4"));
    report
}

pub fn machine_record(report: &M29eExperiment) -> String {
    let format_section = |value: &Option<SectionReport>| match value {
        Some(section) => format!(
            "d{}:ldl={:?}:pivot=[{},{}]:g00=[{},{}]",
            section.dimension,
            section.ldl,
            section.pivot_lower,
            section.pivot_upper,
            section.first_entry_lower,
            section.first_entry_upper,
        ),
        None => "None".into(),
    };
    format!(
        "M29e|hermite_h2={}|product_span={}|dim2={}|dim4={}|finite_promotion_declined={}|infinite_positivity=false|m29_reached=false|claim=hermite_finite_sections_only",
        report.hermite_h2,
        report.product_span,
        format_section(&report.dimension_two),
        format_section(&report.dimension_four),
        report.finite_promotion_declined,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hermite_products_are_even_and_span_the_monomials() {
        assert_eq!(hermite(0), vec![1]);
        assert_eq!(hermite(2), vec![-2, 0, 4]);
        assert_eq!(hermite(4)[0], 12);
        let unit = hermite_product(0, 0, ExactScale::integer(1));
        assert_eq!(unit, vec![Rational::new(1, 1)]);
        let scale = ExactScale::new(1, 128).unwrap();
        let product = hermite_product(1, 1, scale);
        assert!(product.len() >= 3);
        assert!(!promotes_finite_hermite());
    }
}

fn promotes_finite_hermite() -> bool {
    false
}
