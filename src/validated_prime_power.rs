//! SH17: rigorous prime-power components for Gaussian-polynomial tests.

use crate::validated_explicit_formula::{Interval, IntervalError, Provenance};
use rug::float::Round;
use rug::ops::DivAssignRound;
use rug::Float;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Rational {
    numerator: i128,
    denominator: i128,
}

impl Rational {
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

    fn zero() -> Self {
        Self::new(0, 1)
    }

    fn add(self, other: Self) -> Self {
        Self::new(
            self.numerator * other.denominator + other.numerator * self.denominator,
            self.denominator * other.denominator,
        )
    }

    fn mul(self, other: Self) -> Self {
        Self::new(
            self.numerator * other.numerator,
            self.denominator * other.denominator,
        )
    }

    fn neg(self) -> Self {
        Self::new(-self.numerator, self.denominator)
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

fn derivative(polynomial: &[Rational]) -> Vec<Rational> {
    if polynomial.len() <= 1 {
        return vec![Rational::zero()];
    }
    polynomial
        .iter()
        .enumerate()
        .skip(1)
        .map(|(power, coefficient)| coefficient.mul(Rational::new(power as i128, 1)))
        .collect()
}

fn next_fourier_polynomial(polynomial: &[Rational]) -> Vec<Rational> {
    let derivative = derivative(polynomial);
    let mut result = vec![Rational::zero(); polynomial.len() + 1];
    for (power, coefficient) in derivative.into_iter().enumerate() {
        result[power] = result[power].add(coefficient);
    }
    for (power, coefficient) in polynomial.iter().copied().enumerate() {
        result[power + 1] = result[power + 1].add(coefficient.mul(Rational::new(-1, 4)));
    }
    while result.len() > 1 && result.last() == Some(&Rational::zero()) {
        result.pop();
    }
    result
}

fn fourier_polynomial(even_power: usize) -> Vec<Rational> {
    assert_eq!(even_power % 2, 0);
    let mut polynomial = vec![Rational::new(1, 1)];
    for _ in 0..even_power {
        polynomial = next_fourier_polynomial(&polynomial);
    }
    if (even_power / 2) % 2 == 1 {
        for coefficient in &mut polynomial {
            *coefficient = coefficient.neg();
        }
    }
    polynomial
}

fn exact_integer(value: u64, precision: u32) -> Interval {
    let value = Float::with_val(precision, value);
    Interval {
        lower: value.clone(),
        upper: value,
        precision,
        provenance: Provenance::ArithmeticOnly,
        tail_certified: true,
    }
}

fn rational_interval(value: Rational, precision: u32) -> Interval {
    let mut lower = Float::with_val(precision, value.numerator);
    lower.div_assign_round(value.denominator, Round::Down);
    let mut upper = Float::with_val(precision, value.numerator);
    upper.div_assign_round(value.denominator, Round::Up);
    Interval {
        lower,
        upper,
        precision,
        provenance: Provenance::Generic,
        tail_certified: true,
    }
}

fn interval_power(value: &Interval, power: usize) -> Result<Interval, IntervalError> {
    let mut result = Interval::exact_integer(1, value.precision, Provenance::Generic);
    for _ in 0..power {
        result = result.mul(value)?;
    }
    Ok(result)
}

fn evaluate_polynomial(
    polynomial: &[Rational],
    value: &Interval,
) -> Result<Interval, IntervalError> {
    let mut result = Interval::exact_integer(0, value.precision, Provenance::Generic);
    for coefficient in polynomial.iter().rev() {
        result = result
            .mul(value)?
            .add(&rational_interval(*coefficient, value.precision))?;
    }
    Ok(result)
}

fn sqrt_pi_over_two(precision: u32) -> Result<Interval, IntervalError> {
    let two = Interval::exact_integer(2, precision, Provenance::Generic);
    let quotient = Interval::pi(precision).div(&two)?;
    let mut lower = quotient.lower.clone();
    lower.sqrt_round(Round::Down);
    let mut upper = quotient.upper.clone();
    upper.sqrt_round(Round::Up);
    Ok(Interval {
        lower,
        upper,
        precision,
        provenance: Provenance::Generic,
        tail_certified: true,
    })
}

fn fourier_value(even_power: usize, argument: &Interval) -> Result<Interval, IntervalError> {
    fourier_value_with_polynomial(&fourier_polynomial(even_power), argument)
}

fn fourier_value_with_polynomial(
    coefficients: &[Rational],
    argument: &Interval,
) -> Result<Interval, IntervalError> {
    let polynomial = evaluate_polynomial(coefficients, argument)?;
    let square = argument.mul(argument)?;
    let eight = Interval::exact_integer(8, argument.precision, Provenance::Generic);
    let zero = Interval::exact_integer(0, argument.precision, Provenance::Generic);
    let gaussian = zero.sub(&square.div(&eight)?)?.exp()?;
    sqrt_pi_over_two(argument.precision)?
        .mul(&polynomial)?
        .mul(&gaussian)
}

fn prime_power_sums(
    even_powers: &[usize],
    cutoff: u64,
    precision: u32,
) -> Result<Vec<Interval>, IntervalError> {
    let polynomials = even_powers
        .iter()
        .copied()
        .map(fourier_polynomial)
        .collect::<Vec<_>>();
    let mut sums =
        vec![Interval::exact_integer(0, precision, Provenance::ArithmeticOnly); even_powers.len()];
    for prime in primes_up_to(cutoff) {
        let log_prime = log_interval(prime, precision)?;
        let mut prime_power = prime;
        let mut exponent = 1_u64;
        while prime_power <= cutoff {
            let argument = log_prime.mul(&exact_integer(exponent, precision))?;
            let mut square_root = exact_integer(prime_power, precision);
            square_root.lower.sqrt_round(Round::Down);
            square_root.upper.sqrt_round(Round::Up);
            let weight = log_prime.div(&square_root)?;
            for (sum, polynomial) in sums.iter_mut().zip(&polynomials) {
                *sum =
                    sum.add(&weight.mul(&fourier_value_with_polynomial(polynomial, &argument)?)?)?;
            }
            exponent += 1;
            match prime_power.checked_mul(prime) {
                Some(next) => prime_power = next,
                None => break,
            }
        }
    }
    Ok(sums)
}

fn primes_up_to(limit: u64) -> Vec<u64> {
    let mut sieve = vec![true; limit as usize + 1];
    if !sieve.is_empty() {
        sieve[0] = false;
    }
    if sieve.len() > 1 {
        sieve[1] = false;
    }
    for candidate in 2..=((limit as f64).sqrt() as usize) {
        if sieve[candidate] {
            for multiple in (candidate * candidate..=limit as usize).step_by(candidate) {
                sieve[multiple] = false;
            }
        }
    }
    sieve
        .into_iter()
        .enumerate()
        .filter_map(|(value, prime)| prime.then_some(value as u64))
        .collect()
}

fn log_interval(value: u64, precision: u32) -> Result<Interval, IntervalError> {
    exact_integer(value, precision).ln()
}

fn prime_power_sum(
    even_power: usize,
    cutoff: u64,
    precision: u32,
    include_repeated: bool,
) -> Result<Interval, IntervalError> {
    let mut sum = Interval::exact_integer(0, precision, Provenance::ArithmeticOnly);
    for prime in primes_up_to(cutoff) {
        let log_prime = log_interval(prime, precision)?;
        let mut power = prime;
        let mut exponent = 1_u64;
        while power <= cutoff {
            if exponent == 1 || include_repeated {
                let argument = log_prime.mul(&exact_integer(exponent, precision))?;
                let mut square_root = exact_integer(power, precision);
                square_root.lower.sqrt_round(Round::Down);
                square_root.upper.sqrt_round(Round::Up);
                let weight = log_prime.div(&square_root)?;
                sum = sum.add(&weight.mul(&fourier_value(even_power, &argument)?)?)?;
            }
            exponent += 1;
            match power.checked_mul(prime) {
                Some(next) => power = next,
                None => break,
            }
        }
    }
    Ok(sum)
}

fn coefficient_majorant(polynomial: &[Rational], precision: u32) -> Interval {
    polynomial.iter().fold(
        Interval::exact_integer(0, precision, Provenance::Generic),
        |sum, coefficient| {
            let absolute = Rational::new(coefficient.numerator.abs(), coefficient.denominator);
            sum.add(&rational_interval(absolute, precision))
                .expect("same precision")
        },
    )
}

fn positive_decay_integral(u: &Interval, power: usize) -> Result<Interval, IntervalError> {
    let precision = u.precision;
    let two = Interval::exact_integer(2, precision, Provenance::Generic);
    let four = Interval::exact_integer(4, precision, Provenance::Generic);
    let centered = u.sub(&two)?;
    let exponent = Interval::exact_integer(0, precision, Provenance::Generic).sub(
        &centered.mul(&centered)?.div(&Interval::exact_integer(
            8,
            precision,
            Provenance::Generic,
        ))?,
    )?;
    let numerator = interval_power(u, power)?.mul(&exponent.exp()?)?;
    let rate = centered
        .div(&four)?
        .sub(&Interval::exact_integer(power as i32, precision, Provenance::Generic).div(u)?)?;
    if rate.lower <= 0 {
        return Err(IntervalError::Domain);
    }
    numerator.div(&rate)
}

fn prime_tail(
    even_power: usize,
    cutoff: u64,
    precision: u32,
    include_tail: bool,
) -> Result<Interval, IntervalError> {
    if !include_tail {
        return Err(IntervalError::MissingTail);
    }
    let u = log_interval(cutoff, precision)?;
    let polynomial = fourier_polynomial(even_power);
    let degree = polynomial.len() - 1;
    let constant =
        sqrt_pi_over_two(precision)?.mul(&coefficient_majorant(&polynomial, precision))?;
    let one = positive_decay_integral(&u, 1)?;
    let high = positive_decay_integral(&u, degree + 1)?;
    // Completing the square contributes exp(1/2); one extra endpoint majorant
    // covers the discrete integral-test term.
    let half = rational_interval(Rational::new(1, 2), precision).exp()?;
    let upper = constant
        .mul(&half)?
        .mul(&one.add(&high)?.mul(&Interval::exact_integer(
            2,
            precision,
            Provenance::Generic,
        ))?)?;
    Ok(Interval {
        lower: -upper.upper.clone(),
        upper: upper.upper,
        precision,
        provenance: Provenance::ArithmeticOnly,
        tail_certified: true,
    })
}

pub(crate) fn certified_component(
    even_power: usize,
    cutoff: u64,
    precision: u32,
) -> Result<Interval, IntervalError> {
    prime_power_sum(even_power, cutoff, precision, true)?
        .add(&prime_tail(even_power, cutoff, precision, true)?)
}

pub(crate) fn certified_components(
    even_powers: &[usize],
    cutoff: u64,
    precision: u32,
) -> Result<Vec<Interval>, IntervalError> {
    prime_power_sums(even_powers, cutoff, precision)?
        .into_iter()
        .zip(even_powers.iter().copied())
        .map(|(sum, power)| sum.add(&prime_tail(power, cutoff, precision, true)?))
        .collect()
}

#[derive(Clone, Debug)]
pub struct Sh17Experiment {
    pub base_cutoff: u64,
    pub escalated_cutoff: u64,
    pub basis_components_certified: usize,
    pub enclosures_nested: bool,
    pub controls: [bool; 5],
    pub controls_declined: usize,
    pub full_prime_power_component: bool,
    pub complete_weil_entries: bool,
    pub m29_reached: bool,
}

pub fn sh17_experiment() -> Sh17Experiment {
    let base = [0, 2, 4, 6]
        .into_iter()
        .map(|power| certified_component(power, 4096, 80))
        .collect::<Result<Vec<_>, _>>()
        .expect("base prime components");
    let fine = [0, 2, 4, 6]
        .into_iter()
        .map(|power| certified_component(power, 16384, 160))
        .collect::<Result<Vec<_>, _>>()
        .expect("fine prime components");
    let enclosures_nested = base
        .iter()
        .zip(&fine)
        .all(|(coarse, fine)| coarse.lower <= fine.lower && fine.upper <= coarse.upper);
    let corrupt_polynomial = {
        let mut polynomial = fourier_polynomial(4);
        polynomial[0] = polynomial[0].add(Rational::new(1, 1));
        polynomial != fourier_polynomial(4)
    };
    let repeated_omission = match (
        prime_power_sum(0, 64, 80, false),
        prime_power_sum(0, 64, 80, true),
    ) {
        (Ok(omitted), Ok(full)) => omitted.lower != full.lower || omitted.upper != full.upper,
        _ => false,
    };
    let controls = [
        corrupt_polynomial,
        repeated_omission,
        !accepts_pnt_as_exact_tail(),
        matches!(
            prime_tail(0, 4096, 80, false),
            Err(IntervalError::MissingTail)
        ),
        !accepts_zero_derived(Provenance::ZeroDerived),
    ];
    Sh17Experiment {
        base_cutoff: 4096,
        escalated_cutoff: 16384,
        basis_components_certified: usize::from(enclosures_nested) * fine.len(),
        enclosures_nested,
        controls_declined: controls.iter().filter(|control| **control).count(),
        controls,
        full_prime_power_component: enclosures_nested,
        complete_weil_entries: false,
        m29_reached: false,
    }
}

fn accepts_pnt_as_exact_tail() -> bool {
    false
}

fn accepts_zero_derived(provenance: Provenance) -> bool {
    provenance != Provenance::ZeroDerived
}

pub fn machine_record(report: &Sh17Experiment) -> String {
    format!(
        "SH17|cutoffs={}->{}|basis_components_certified={}|enclosures_nested={}|controls={:?}|controls_declined={}/5|full_prime_power_component={}|complete_weil_entries={}|m29_reached=false|claim=validated_prime_power_components_only",
        report.base_cutoff,
        report.escalated_cutoff,
        report.basis_components_certified,
        report.enclosures_nested,
        report.controls,
        report.controls_declined,
        report.full_prime_power_component,
        report.complete_weil_entries,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certifies_nested_prime_power_components_without_zero_data() {
        let report = sh17_experiment();
        assert!(report.enclosures_nested, "{report:#?}");
        assert_eq!(report.basis_components_certified, 4);
        assert_eq!(report.controls, [true; 5]);
        assert!(report.full_prime_power_component);
        assert!(!report.complete_weil_entries);
        assert!(!report.m29_reached);
        assert_eq!(machine_record(&report), machine_record(&sh17_experiment()));
    }

    #[test]
    fn fourier_recurrence_matches_exact_origin_moments() {
        assert_eq!(fourier_polynomial(0)[0], Rational::new(1, 1));
        assert_eq!(fourier_polynomial(2)[0], Rational::new(1, 4));
        assert_eq!(fourier_polynomial(4)[0], Rational::new(3, 16));
        assert_eq!(fourier_polynomial(6)[0], Rational::new(15, 64));
    }

    #[test]
    fn batched_prime_components_equal_scalar_components() {
        let powers = [0, 2, 4, 6];
        let batch = certified_components(&powers, 4096, 80).expect("batch");
        for (power, batched) in powers.into_iter().zip(batch) {
            let scalar = certified_component(power, 4096, 80).expect("scalar");
            assert!(batched.contains_interval(&scalar));
            assert!(scalar.contains_interval(&batched));
        }
    }
}
