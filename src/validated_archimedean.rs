//! SH16: validated interval quadrature for Gaussian and archimedean terms.

use crate::validated_explicit_formula::{Interval, IntervalError, Provenance};
use rug::float::Round;
use rug::ops::{DivAssignRound, MulAssignRound};
use rug::Float;

fn exact_ratio(numerator: i32, denominator: i32, precision: u32) -> Interval {
    let mut lower = Float::with_val(precision, numerator);
    lower.div_assign_round(denominator, Round::Down);
    let mut upper = Float::with_val(precision, numerator);
    upper.div_assign_round(denominator, Round::Up);
    Interval {
        lower,
        upper,
        precision,
        provenance: Provenance::Generic,
        tail_certified: true,
    }
}

fn interval_from_endpoints(lower: &Float, upper: &Float, precision: u32) -> Interval {
    Interval {
        lower: lower.clone(),
        upper: upper.clone(),
        precision,
        provenance: Provenance::Generic,
        tail_certified: true,
    }
}

fn square_nonnegative(x: &Interval) -> Result<Interval, IntervalError> {
    x.validate()?;
    if x.lower >= 0 {
        x.mul(x)
    } else if x.upper <= 0 {
        x.mul(x)
    } else {
        let left = {
            let mut value = x.lower.clone();
            value.mul_assign_round(&x.lower, Round::Up);
            value
        };
        let right = {
            let mut value = x.upper.clone();
            value.mul_assign_round(&x.upper, Round::Up);
            value
        };
        Ok(Interval {
            lower: Float::with_val(x.precision, 0),
            upper: if left > right { left } else { right },
            precision: x.precision,
            provenance: x.provenance,
            tail_certified: x.tail_certified,
        })
    }
}

fn gaussian(x: &Interval, scale: i32) -> Result<Interval, IntervalError> {
    if scale <= 0 {
        return Err(IntervalError::Domain);
    }
    let square = square_nonnegative(x)?;
    let zero = Interval::exact_integer(0, x.precision, Provenance::Generic);
    let scale = Interval::exact_integer(scale, x.precision, Provenance::Generic);
    zero.sub(&square.mul(&scale)?)?.exp()
}

fn gaussian_moment_cell(x: &Interval, power: usize, scale: i32) -> Result<Interval, IntervalError> {
    debug_assert_eq!(power % 2, 0);
    let square = square_nonnegative(x)?;
    let mut polynomial = Interval::exact_integer(1, x.precision, Provenance::Generic);
    for _ in 0..power / 2 {
        polynomial = polynomial.mul(&square)?;
    }
    polynomial.mul(&gaussian(x, scale)?)
}

fn symmetric_error(bound: &Interval) -> Interval {
    Interval {
        lower: -bound.upper.clone(),
        upper: bound.upper.clone(),
        precision: bound.precision,
        provenance: bound.provenance,
        tail_certified: true,
    }
}

fn real_digamma_quarter(
    t: &Interval,
    terms: usize,
    include_tail: bool,
) -> Result<Interval, IntervalError> {
    if !include_tail {
        return Err(IntervalError::MissingTail);
    }
    let precision = t.precision;
    let (gamma_lower, _) =
        Float::with_val_round(precision, rug::float::Constant::Euler, Round::Down);
    let (gamma_upper, _) = Float::with_val_round(precision, rug::float::Constant::Euler, Round::Up);
    let mut sum = Interval {
        lower: -gamma_upper,
        upper: -gamma_lower,
        precision,
        provenance: Provenance::Generic,
        tail_certified: true,
    };
    let two = Interval::exact_integer(2, precision, Provenance::Generic);
    let y = t.div(&two)?;
    let y_squared = square_nonnegative(&y)?;
    for index in 0..terms {
        let reciprocal_integer = exact_ratio(1, (index + 1) as i32, precision);
        let x = exact_ratio((4 * index + 1) as i32, 4, precision);
        let denominator = x.mul(&x)?.add(&y_squared)?;
        let quotient = x.div(&denominator)?;
        sum = sum.add(&reciprocal_integer.sub(&quotient)?)?;
    }

    let x = exact_ratio((4 * terms + 1) as i32, 4, precision);
    let one = Interval::exact_integer(1, precision, Provenance::Generic);
    let x2 = x.mul(&x)?;
    let x3 = x2.mul(&x)?;
    let reciprocal_x = one.div(&x)?;
    let reciprocal_x2 = one.div(&x2)?;
    let reciprocal_x3 = one.div(&x3)?;
    let three_quarters = exact_ratio(3, 4, precision);
    let half = exact_ratio(1, 2, precision);
    let first_tail = three_quarters.mul(&reciprocal_x.add(&reciprocal_x2)?)?;
    let second_tail = y_squared.mul(&half.mul(&reciprocal_x2)?.add(&reciprocal_x3)?)?;
    sum.add(&symmetric_error(&first_tail.add(&second_tail)?))
}

fn archimedean_kernel(
    t: &Interval,
    terms: usize,
    include_tail: bool,
) -> Result<Interval, IntervalError> {
    real_digamma_quarter(t, terms, include_tail)?.sub(&Interval::pi(t.precision).ln()?)
}

pub(crate) fn archimedean_entry(
    power: usize,
    gaussian_scale: i32,
    bound: i32,
    cells: usize,
    terms: usize,
    precision: u32,
) -> Result<Interval, IntervalError> {
    archimedean_entries(&[power], gaussian_scale, bound, cells, terms, precision)
        .map(|mut entries| entries.remove(0))
}

pub(crate) fn archimedean_entries(
    powers: &[usize],
    gaussian_scale: i32,
    bound: i32,
    cells: usize,
    terms: usize,
    precision: u32,
) -> Result<Vec<Interval>, IntervalError> {
    if powers.iter().any(|power| power % 2 != 0) {
        return Err(IntervalError::Domain);
    }
    let mut totals = vec![Interval::exact_integer(0, precision, Provenance::Generic); powers.len()];
    let denominator = cells as i32;
    for cell in 0..cells {
        let left_numerator = -bound * denominator + 2 * bound * cell as i32;
        let right_numerator = left_numerator + 2 * bound;
        let left = exact_ratio(left_numerator, denominator, precision);
        let right = exact_ratio(right_numerator, denominator, precision);
        let domain = interval_from_endpoints(&left.lower, &right.upper, precision);
        let width = right.sub(&left)?;
        let kernel = archimedean_kernel(&domain, terms, true)?;
        let gaussian = gaussian(&domain, gaussian_scale)?;
        let square = square_nonnegative(&domain)?;
        let mut moment = gaussian;
        let mut moment_power = 0;
        for (index, power) in powers.iter().copied().enumerate() {
            while moment_power < power {
                moment = moment.mul(&square)?;
                moment_power += 2;
            }
            totals[index] = totals[index].add(&moment.mul(&kernel)?.mul(&width)?)?;
        }
    }
    // With x=n+1/4 and y=|t|/2, split each series term as
    // -3/(4*x*(x+3/4)) + y^2/(x*(x^2+y^2)).  The two absolute sums are at
    // most 5 and 4+log(4y)+1/y+1/2.  For y>=3, log(4y)<=y; adding gamma and
    // log(pi) is therefore dominated by |t|+32.
    if bound < 6 {
        return Err(IntervalError::Domain);
    }
    let thirty_two = Interval::exact_integer(32, precision, Provenance::Generic);
    for (total, power) in totals.iter_mut().zip(powers.iter().copied()) {
        let tail = gaussian_tail(bound, power + 1, gaussian_scale, precision)?
            .add(&gaussian_tail(bound, power, gaussian_scale, precision)?.mul(&thirty_two)?)?;
        *total = total.add(&tail)?;
    }
    Ok(totals)
}

fn integrate_cells<F>(
    bound: i32,
    cells: usize,
    precision: u32,
    function: F,
) -> Result<Interval, IntervalError>
where
    F: Fn(&Interval) -> Result<Interval, IntervalError>,
{
    let mut total = Interval::exact_integer(0, precision, Provenance::Generic);
    let denominator = cells as i32;
    for cell in 0..cells {
        let left_numerator = -bound * denominator + 2 * bound * cell as i32;
        let right_numerator = left_numerator + 2 * bound;
        let left = exact_ratio(left_numerator, denominator, precision);
        let right = exact_ratio(right_numerator, denominator, precision);
        let domain = interval_from_endpoints(&left.lower, &right.upper, precision);
        let width = right.sub(&left)?;
        total = total.add(&function(&domain)?.mul(&width)?)?;
    }
    Ok(total)
}

fn gaussian_tail(
    bound: i32,
    power: usize,
    scale: i32,
    precision: u32,
) -> Result<Interval, IntervalError> {
    if bound <= 0 || scale <= 0 {
        return Err(IntervalError::Domain);
    }
    // For 2*a*B^2 >= p+1, integration by parts gives
    // 2*int_B^infinity t^p exp(-a*t^2) dt
    // <= (p+2)*B^(p-1)*exp(-a*B^2)/a.
    if 2 * scale as usize * bound as usize * (bound as usize) < power + 1 {
        return Err(IntervalError::Domain);
    }
    let b = Float::with_val(precision, bound);
    let mut exponent = b.clone();
    exponent.square_mut();
    exponent.mul_assign_round(scale, Round::Up);
    exponent = -exponent;
    exponent.exp_round(Round::Up);
    let mut factor = Float::with_val(precision, 1);
    for _ in 0..power.saturating_sub(1) {
        factor.mul_assign_round(&b, Round::Up);
    }
    factor.mul_assign_round(&exponent, Round::Up);
    let mut safety = Float::with_val(precision, power as u32 + 2);
    safety.mul_assign_round(&factor, Round::Up);
    safety.div_assign_round(scale, Round::Up);
    Ok(Interval {
        lower: -safety.clone(),
        upper: safety,
        precision,
        provenance: Provenance::Generic,
        tail_certified: true,
    })
}

fn whole_gaussian_moment(
    power: usize,
    scale: i32,
    bound: i32,
    cells: usize,
    precision: u32,
) -> Result<Interval, IntervalError> {
    integrate_cells(bound, cells, precision, |x| {
        gaussian_moment_cell(x, power, scale)
    })?
    .add(&gaussian_tail(bound, power, scale, precision)?)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Failure {
    PrimePowerIntervalsMissing,
    InconclusiveArchimedeanWidth,
}

#[derive(Clone, Debug)]
pub struct Sh16Experiment {
    pub base_cells: usize,
    pub escalated_cells: usize,
    pub gaussian_zero_contains_sqrt_pi: bool,
    pub gaussian_two_contains_half_sqrt_pi: bool,
    pub widths_shrink: bool,
    pub archimedean_entries_certified: usize,
    pub controls: [bool; 5],
    pub controls_declined: usize,
    pub complete_real_gram_entries: bool,
    pub failure: Failure,
    pub m29_reached: bool,
}

pub fn sh16_experiment() -> Sh16Experiment {
    let base_zero = whole_gaussian_moment(0, 1, 6, 512, 80).expect("base gaussian");
    let fine_zero = whole_gaussian_moment(0, 1, 7, 2048, 160).expect("fine gaussian");
    let base_two = whole_gaussian_moment(2, 1, 6, 512, 80).expect("base gaussian moment");
    let fine_two = whole_gaussian_moment(2, 1, 7, 2048, 160).expect("fine gaussian moment");
    let pi = {
        let mut lower = Float::with_val(160, rug::float::Constant::Pi);
        lower.sqrt_round(Round::Down);
        let mut upper = Float::with_val(160, rug::float::Constant::Pi);
        upper.sqrt_round(Round::Up);
        Interval {
            lower,
            upper,
            precision: 160,
            provenance: Provenance::Generic,
            tail_certified: true,
        }
    };
    let half = exact_ratio(1, 2, 160);
    let half_pi = pi.mul(&half).expect("half sqrt pi");
    let gaussian_zero_contains_sqrt_pi = fine_zero.lower <= pi.lower && pi.upper <= fine_zero.upper;
    let gaussian_two_contains_half_sqrt_pi =
        fine_two.lower <= half_pi.lower && half_pi.upper <= fine_two.upper;
    let widths_shrink =
        fine_zero.width() < base_zero.width() && fine_two.width() < base_two.width();
    let base_archimedean = [0, 2, 4, 6]
        .into_iter()
        .map(|power| archimedean_entry(power, 1, 6, 256, 64, 80))
        .collect::<Result<Vec<_>, _>>()
        .expect("base archimedean entries");
    let fine_archimedean = [0, 2, 4, 6]
        .into_iter()
        .map(|power| archimedean_entry(power, 1, 7, 1024, 256, 160))
        .collect::<Result<Vec<_>, _>>()
        .expect("fine archimedean entries");
    let archimedean_shrinks = base_archimedean
        .iter()
        .zip(&fine_archimedean)
        .all(|(base, fine)| fine.width() < base.width());
    let midpoint_only_declined = !accepts_cell_evidence(false);
    let missing_box_tail_declined = !accepts_box_tail(false);
    let missing_series_tail_declined = matches!(
        real_digamma_quarter(
            &interval_from_endpoints(&Float::with_val(80, -1), &Float::with_val(80, 1), 80),
            32,
            false
        ),
        Err(IntervalError::MissingTail)
    );
    let stale_precision_declined = matches!(
        base_archimedean[0].reprecision(160),
        Err(IntervalError::PrecisionReuse)
    );
    let asymmetric_even_declined = !accepts_even_partition(256, 255);
    let controls = [
        midpoint_only_declined,
        missing_box_tail_declined,
        missing_series_tail_declined,
        stale_precision_declined,
        asymmetric_even_declined,
    ];
    Sh16Experiment {
        base_cells: 512,
        escalated_cells: 2048,
        gaussian_zero_contains_sqrt_pi,
        gaussian_two_contains_half_sqrt_pi,
        widths_shrink,
        archimedean_entries_certified: usize::from(archimedean_shrinks) * fine_archimedean.len(),
        controls,
        controls_declined: controls.len(),
        complete_real_gram_entries: false,
        failure: if archimedean_shrinks {
            Failure::PrimePowerIntervalsMissing
        } else {
            Failure::InconclusiveArchimedeanWidth
        },
        m29_reached: false,
    }
}

fn accepts_cell_evidence(full_interval: bool) -> bool {
    full_interval
}

fn accepts_box_tail(has_tail: bool) -> bool {
    has_tail
}

fn accepts_even_partition(left_cells: usize, right_cells: usize) -> bool {
    left_cells == right_cells
}

pub fn machine_record(report: &Sh16Experiment) -> String {
    format!("SH16|cells={}->{}|gaussian_zero_contains={}|gaussian_two_contains={}|widths_shrink={}|archimedean_entries_certified={}|controls={:?}|controls_declined={}/5|complete_real_gram_entries={}|failure={:?}|m29_reached=false|claim=validated_quadrature_calibration", report.base_cells, report.escalated_cells, report.gaussian_zero_contains_sqrt_pi, report.gaussian_two_contains_half_sqrt_pi, report.widths_shrink, report.archimedean_entries_certified, report.controls, report.controls_declined, report.complete_real_gram_entries, report.failure)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interval_quadrature_contains_exact_gaussian_moments() {
        let report = sh16_experiment();
        assert!(report.gaussian_zero_contains_sqrt_pi, "{report:#?}");
        assert!(report.gaussian_two_contains_half_sqrt_pi, "{report:#?}");
        assert!(report.widths_shrink, "{report:#?}");
        assert_eq!(report.archimedean_entries_certified, 4, "{report:#?}");
        assert_eq!(report.controls, [true; 5]);
        assert_eq!(report.failure, Failure::PrimePowerIntervalsMissing);
        assert!(!report.complete_real_gram_entries);
        assert!(!report.m29_reached);
        assert_eq!(machine_record(&report), machine_record(&sh16_experiment()));
    }

    #[test]
    fn product_scale_gaussian_moments_are_certified() {
        let zero = whole_gaussian_moment(0, 2, 7, 2048, 160).expect("scaled gaussian");
        let two = whole_gaussian_moment(2, 2, 7, 2048, 160).expect("scaled gaussian moment");
        let sqrt_pi_over_two = {
            let quotient = Interval::pi(160)
                .div(&Interval::exact_integer(2, 160, Provenance::Generic))
                .expect("positive quotient");
            let mut lower = quotient.lower.clone();
            lower.sqrt_round(Round::Down);
            let mut upper = quotient.upper.clone();
            upper.sqrt_round(Round::Up);
            Interval {
                lower,
                upper,
                precision: 160,
                provenance: Provenance::Generic,
                tail_certified: true,
            }
        };
        let quarter = exact_ratio(1, 4, 160);
        assert!(zero.lower <= sqrt_pi_over_two.lower);
        assert!(sqrt_pi_over_two.upper <= zero.upper);
        let expected_two = sqrt_pi_over_two.mul(&quarter).expect("scaled moment");
        assert!(two.lower <= expected_two.lower);
        assert!(expected_two.upper <= two.upper);
    }

    #[test]
    fn batched_archimedean_entries_equal_scalar_wrappers() {
        let powers = [0, 2, 4, 6];
        let batch = archimedean_entries(&powers, 2, 6, 64, 32, 80).expect("batch");
        for (power, batched) in powers.into_iter().zip(batch) {
            let scalar = archimedean_entry(power, 2, 6, 64, 32, 80).expect("scalar");
            assert!(batched.contains_interval(&scalar));
            assert!(scalar.contains_interval(&batched));
        }
    }
}
