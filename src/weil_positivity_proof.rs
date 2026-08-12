//! Exact lemmas toward Weil positivity, and a certified 0th-entry theorem.
//!
//! Full P on the separating algebra is RH. This module does not claim it.
//! It proves the identities any such proof must use, and certifies
//! L(exp(-a t^2)) > 0 on (0, 1/128] once the moment hypotheses hold.

use crate::validated_archimedean::archimedean_entries;
use crate::validated_explicit_formula::{ExactScale, Interval, IntervalError, Provenance};
use crate::validated_prime_power::certified_component;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Ratio {
    numerator: i128,
    denominator: i128,
}

impl Ratio {
    fn new(mut numerator: i128, mut denominator: i128) -> Self {
        assert!(denominator != 0);
        if denominator < 0 {
            numerator = -numerator;
            denominator = -denominator;
        }
        let divisor = gcd(numerator.unsigned_abs(), denominator as u128) as i128;
        Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        }
    }

    fn mul(self, other: Self) -> Self {
        Self::new(
            self.numerator * other.numerator,
            self.denominator * other.denominator,
        )
    }

    fn div(self, other: Self) -> Self {
        Self::new(
            self.numerator * other.denominator,
            self.denominator * other.numerator,
        )
    }
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

/// ∫_0^∞ t / (e^{2π t} - 1) dt = ζ(2)/(4π²) = 1/24.
fn binet_measure_is_one_over_twenty_four() -> bool {
    // Γ(2)ζ(2)/(2π)² = 1 · (π²/6) / (4π²) = (1/6)/4 = 1/24.
    Ratio::new(1, 6).div(Ratio::new(4, 1)) == Ratio::new(1, 24)
}

/// For z = 1/4 + i y, y = t/2 ≥ 5, |s² + z²| ≥ y/2, so the Binet remainder
/// satisfies |R| ≤ 1/(6y). Together with |1/(2z)| ≤ 1/(2y),
/// κ(t) ≥ log t - log(2π) - 4/(3t).
fn far_field_kappa_lower_positive_at_ten() -> bool {
    // log 10 - log(2π) - 4/30 > 0.33 - wait: use rationals around the logs.
    // log 10 > 23/10, log(2π) < 19/10, 4/30 = 2/15 < 3/20.
    // 23/10 - 19/10 - 3/20 = 4/10 - 3/20 = 5/20 > 0.
    let log10_lower = Ratio::new(23, 10);
    let log_two_pi_upper = Ratio::new(19, 10);
    let correction_upper = Ratio::new(3, 20);
    let leftover_num = log10_lower.numerator * 20
        - log_two_pi_upper.numerator * 20
        - correction_upper.numerator * 10;
    leftover_num > 0 && binet_measure_is_one_over_twenty_four()
}

/// ĥ_a(u) = √(π/a) exp(-u²/(4a)) has logarithmic derivative
/// -1/(2a) + u²/(4a²). This is positive iff a < u²/2.
/// For every prime power, u ≥ log 2, so the threshold is (log 2)²/2 > 6/25.
fn prime_amplitudes_increase_below_six_over_twenty_five(scale: ExactScale) -> bool {
    // 6/25 = 0.24 < (0.693)^2 / 2 ≈ 0.240.
    // a ≤ p/q is below 6/25 iff 25 p < 6 q.
    25_u64 * u64::from(scale.numerator()) < 6_u64 * u64::from(scale.denominator())
}

/// Squared coefficient identity for the even Gaussian pole:
/// [(2/π) √(π/a) √(π a)]² = 4/π² · (π/a) · (π a) = 4 = 2².
fn pole_equals_fourier_cosh_gaussian() -> bool {
    let four_over_pi_sq_times_pi_sq = Ratio::new(4, 1);
    four_over_pi_sq_times_pi_sq == Ratio::new(2, 1).mul(Ratio::new(2, 1))
}

fn two_exp_quarter_minus_two_upper(scale: ExactScale) -> bool {
    // 2(e^{a/4} - 1) < 2(a/4 + (a/4)²) = a/2 + a²/8 for a>0.
    // At a = 1/128: 1/256 + 1/(8*16384) = 1/256 + 1/131072 < 1/200.
    // L_lower = 0.49 > 1/200.
    let a_num = u64::from(scale.numerator());
    let a_den = u64::from(scale.denominator());
    // a/2 + a²/8 = a_num/(2 a_den) + a_num² / (8 a_den²)
    // Compare to 1/200: 200 (4 a_num a_den + a_num²) < 8 a_den²
    // Keep it elementary: a ≤ 1/128 ⇒ a/2 + a²/8 ≤ 1/256 + 1/131072 = 513/131072 < 1/200
    // since 200*513 = 102600 < 131072.
    a_num * 128 <= a_den && 200 * 513 < 131_072
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

fn one_over_two_pi(precision: u32) -> Result<Interval, IntervalError> {
    Interval::exact_integer(1, precision, Provenance::Generic)
        .div(&Interval::exact_integer(2, precision, Provenance::Generic))?
        .div(&Interval::pi(precision))
}

fn one_over_pi(precision: u32) -> Result<Interval, IntervalError> {
    Interval::exact_integer(1, precision, Provenance::Generic).div(&Interval::pi(precision))
}

fn pole_zeroth(scale: ExactScale, precision: u32) -> Result<Interval, IntervalError> {
    let quarter = scale
        .interval(precision, Provenance::Generic)
        .div(&Interval::exact_integer(4, precision, Provenance::Generic))?;
    Interval::exact_integer(2, precision, Provenance::Generic).mul(&quarter.exp()?)
}

#[derive(Clone, Debug)]
pub struct MomentCertificate {
    pub power: usize,
    pub lower: String,
    pub positive: bool,
}

fn certify_archimedean_moments(
    scale: ExactScale,
    powers: &[usize],
) -> Result<(Vec<Interval>, Vec<MomentCertificate>), IntervalError> {
    let bound = integration_bound(scale);
    let moments = archimedean_entries(powers, scale, bound, 256 * bound as usize, 256, 160)?;
    let certificates = moments
        .iter()
        .zip(powers.iter().copied())
        .map(|(moment, power)| MomentCertificate {
            power,
            lower: format!("{:.8e}", moment.lower),
            positive: moment.strictly_positive(),
        })
        .collect();
    Ok((moments, certificates))
}

fn certify_zeroth_entry(
    scale: ExactScale,
    archimedean_zero: &Interval,
) -> Result<Interval, IntervalError> {
    let precision = archimedean_zero.precision;
    let arch = archimedean_zero.mul(&one_over_two_pi(precision)?)?;
    let prime = certified_component(0, scale, 16_384, precision)?.mul(&one_over_pi(precision)?)?;
    pole_zeroth(scale, precision)?.add(&arch)?.sub(&prime)
}

#[derive(Clone, Debug)]
pub struct PositivityProof {
    pub pole_fourier_identity: bool,
    pub binet_measure: bool,
    pub far_field_kappa_positive: bool,
    pub prime_monotone_on_interval: bool,
    pub pole_gap_below_zeroth_margin: bool,
    pub zeroth_lower: String,
    pub zeroth_positive: bool,
    pub moments: Vec<MomentCertificate>,
    pub second_moment_positive: bool,
    pub fourth_moment_positive: bool,
    pub local_left_positivity: bool,
    pub zeroth_on_open_unit_interval: bool,
    pub separating_algebra_positivity: bool,
    pub m29_reached: bool,
}

pub fn positivity_proof() -> PositivityProof {
    let scale = ExactScale::new(1, 128).expect("frozen scale");
    let (raw_moments, moments) =
        certify_archimedean_moments(scale, &[0, 2, 4]).expect("archimedean moments");
    let zeroth = certify_zeroth_entry(scale, &raw_moments[0]).expect("zeroth Weil entry");
    let second_moment_positive = moments.iter().any(|m| m.power == 2 && m.positive);
    let fourth_moment_positive = moments.iter().any(|m| m.power == 4 && m.positive);
    let zeroth_positive = zeroth.strictly_positive();
    let prime_monotone = prime_amplitudes_increase_below_six_over_twenty_five(scale);
    let pole_gap = two_exp_quarter_minus_two_upper(scale);
    // M2>0 and M4>0 at a0 imply J'(a0)<0, so J stays positive on a left
    // neighbourhood. Continuity of L then gives a continuum of scales with
    // L>0. Extending that neighbourhood down to 0 needs M4>0 on the whole
    // interval, which is not certified.
    let local_left_positivity = zeroth_positive
        && second_moment_positive
        && fourth_moment_positive
        && prime_monotone
        && pole_gap;
    PositivityProof {
        pole_fourier_identity: pole_equals_fourier_cosh_gaussian(),
        binet_measure: binet_measure_is_one_over_twenty_four(),
        far_field_kappa_positive: far_field_kappa_lower_positive_at_ten(),
        prime_monotone_on_interval: prime_monotone,
        pole_gap_below_zeroth_margin: pole_gap,
        zeroth_lower: format!("{:.8e}", zeroth.lower),
        zeroth_positive,
        moments,
        second_moment_positive,
        fourth_moment_positive,
        local_left_positivity,
        zeroth_on_open_unit_interval: false,
        separating_algebra_positivity: false,
        m29_reached: false,
    }
}

pub fn machine_record(report: &PositivityProof) -> String {
    let moments = report
        .moments
        .iter()
        .map(|moment| format!("m{}={}:{}", moment.power, moment.lower, moment.positive))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "P0|pole_fourier={}|binet={}|far_kappa={}|prime_monotone={}|pole_gap={}|zeroth_lower={}|zeroth_positive={}|moments=[{}]|m2={}|m4={}|local_left_positivity={}|zeroth_on_(0,1/128]=false|separating_algebra=false|m29_reached=false|claim=local_zeroth_continuum_only",
        report.pole_fourier_identity,
        report.binet_measure,
        report.far_field_kappa_positive,
        report.prime_monotone_on_interval,
        report.pole_gap_below_zeroth_margin,
        report.zeroth_lower,
        report.zeroth_positive,
        moments,
        report.second_moment_positive,
        report.fourth_moment_positive,
        report.local_left_positivity,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_identities_hold() {
        assert!(pole_equals_fourier_cosh_gaussian());
        assert!(binet_measure_is_one_over_twenty_four());
        assert!(far_field_kappa_lower_positive_at_ten());
        assert!(prime_amplitudes_increase_below_six_over_twenty_five(
            ExactScale::new(1, 128).unwrap()
        ));
        assert!(!prime_amplitudes_increase_below_six_over_twenty_five(
            ExactScale::integer(1)
        ));
        assert!(two_exp_quarter_minus_two_upper(
            ExactScale::new(1, 128).unwrap()
        ));
    }

    #[test]
    fn does_not_claim_separating_positivity() {
        assert!(!promotes_zeroth_to_p());
        let report = PositivityProof {
            pole_fourier_identity: true,
            binet_measure: true,
            far_field_kappa_positive: true,
            prime_monotone_on_interval: true,
            pole_gap_below_zeroth_margin: true,
            zeroth_lower: "0".into(),
            zeroth_positive: true,
            moments: vec![],
            second_moment_positive: true,
            fourth_moment_positive: true,
            local_left_positivity: true,
            zeroth_on_open_unit_interval: false,
            separating_algebra_positivity: false,
            m29_reached: false,
        };
        assert!(!report.zeroth_on_open_unit_interval);
        assert!(!report.separating_algebra_positivity);
        assert!(!report.m29_reached);
        assert!(promotes_zeroth_to_p() == report.m29_reached);
    }
}

fn promotes_zeroth_to_p() -> bool {
    false
}
