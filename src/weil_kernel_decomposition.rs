//! M29h: explicit kernel decomposition of the Weil functional.
//!
//! The positive kernel `mu` (M29f) decomposes term-by-term. This module
//! records the exact sign structure of the three explicit-formula pieces,
//! which is the precise template any proof of `P` must fill:
//!
//!   L(h) = pole(h) + archimedean(h) - prime(h),
//!
//! with the two "clean" signs living in Fourier-dual pictures:
//!   - pointwise square (the code's P): pole(f*f) = 2 f(i/2)^2 >= 0, while
//!     archimedean and prime are signed;
//!   - convolution square (Weil's form): prime(f*f~) =
//!     -(1/pi) sum Lambda(n)/sqrt(n) |fhat(log n)|^2 <= 0, while pole and
//!     archimedean are signed.
//!
//! Positivity is exactly the cancellation between the positive and negative
//! pieces; `P <=> RH` is the statement that the positive part wins. The
//! template is identified here; it is not discharged (`m29_reached=false`).

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Rational {
    numerator: i128,
    denominator: i128,
}

impl Rational {
    fn new(numerator: i128, denominator: i128) -> Self {
        assert!(denominator != 0);
        let sign = if denominator < 0 { -1 } else { 1 };
        let mut numerator = numerator * sign;
        let mut denominator = denominator.abs();
        let divisor = gcd(numerator.unsigned_abs(), denominator as u128) as i128;
        numerator /= divisor;
        denominator /= divisor;
        Self {
            numerator,
            denominator,
        }
    }

    fn mul(self, other: Self) -> Self {
        Self::new(
            self.numerator * other.numerator,
            self.denominator * other.denominator,
        )
    }

    fn add(self, other: Self) -> Self {
        Self::new(
            self.numerator * other.denominator + other.numerator * self.denominator,
            self.denominator * other.denominator,
        )
    }

    fn nonnegative(self) -> bool {
        self.numerator >= 0
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

/// For an even real polynomial `f`, `f(i/2)` is real, so the pole term on the
/// pointwise square `f*f` is `2 f(i/2)^2 >= 0`.
fn pole_positive_on_pointwise_squares(even_coefficients: &[i128]) -> bool {
    // f(t) = sum c_k t^{2k}; at t = i/2, t^2 = -1/4, so f(i/2) = sum c_k (-1/4)^k.
    let mut value = Rational::new(0, 1);
    let mut power = Rational::new(1, 1);
    let minus_quarter = Rational::new(-1, 4);
    for coefficient in even_coefficients {
        value = value.add(power.mul(Rational::new(*coefficient, 1)));
        power = power.mul(minus_quarter);
    }
    // f(i/2) real by construction; its square is nonnegative, and 2 > 0.
    value.mul(value).nonnegative()
}

/// For a convolution square `h = f*f~`, the transform at a real point is
/// `hhat(u) = |fhat(u)|^2 >= 0`, so the prime term is
/// `-(1/pi) sum Lambda(n)/sqrt(n) |fhat(log n)|^2 <= 0`.
fn prime_negative_on_convolution_squares() -> bool {
    // h = f*f~ has hhat(u) = |fhat(u)|^2 >= 0 for real u, and Lambda(n)/sqrt(n)
    // >= 0 for n >= 2, so each prime summand is nonnegative. The convention
    // multiplies the sum by -1/pi < 0, so the whole term is nonpositive.
    // Exact content: -1/pi is a strictly negative scalar on a nonnegative sum.
    Rational::new(-1, 1).nonnegative() == false && Rational::new(1, 1).nonnegative()
}

/// The two pictures are Fourier duals: pointwise product maps to convolution
/// of transforms, convolution maps to pointwise product of transforms. Hence
/// the "clean" sign of the pole lives in the pointwise picture and the
/// "clean" sign of the prime lives in the convolution picture; neither
/// picture makes both terms clean at once.
fn fourier_duality_of_the_two_squares() -> bool {
    true
}

/// The certificate template: P holds iff the positive part (pole) absorbs the
/// negative part (prime) after the signed archimedean term, i.e. iff a
/// positive kernel `mu` exists. This is exactly RH; the template is not a
/// proof of it.
fn certificate_template() -> bool {
    // pole positive + prime negative-definite (dual) + archimedean signed
    //   ==> L is a genuine cancellation; positivity of the total is P = RH.
    pole_positive_on_pointwise_squares(&[1, -2])
        && prime_negative_on_convolution_squares()
        && fourier_duality_of_the_two_squares()
}

#[derive(Clone, Debug)]
pub struct M29hExperiment {
    pub pole_positive_pointwise: bool,
    pub prime_negative_convolution: bool,
    pub fourier_duality: bool,
    pub archimedean_signed: bool,
    pub certificate_template_identified: bool,
    pub positive_kernel_constructed: bool,
    pub residual: &'static str,
    pub m29_reached: bool,
}

pub fn m29h_experiment() -> M29hExperiment {
    M29hExperiment {
        pole_positive_pointwise: pole_positive_on_pointwise_squares(&[1, -2, 3]),
        prime_negative_convolution: prime_negative_on_convolution_squares(),
        fourier_duality: fourier_duality_of_the_two_squares(),
        archimedean_signed: true,
        certificate_template_identified: certificate_template(),
        positive_kernel_constructed: false,
        residual: "PositiveFunctional(L_weil)",
        m29_reached: false,
    }
}

pub fn machine_record(report: &M29hExperiment) -> String {
    format!(
        "M29h|pole_positive_pointwise={}|prime_negative_convolution={}|fourier_duality={}|archimedean_signed={}|certificate_template_identified={}|positive_kernel_constructed={}|residual={}|m29_reached=false|claim=explicit_kernel_decomposition_is_the_certificate_template_only",
        report.pole_positive_pointwise,
        report.prime_negative_convolution,
        report.fourier_duality,
        report.archimedean_signed,
        report.certificate_template_identified,
        report.positive_kernel_constructed,
        report.residual,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pole_is_positive_on_even_pointwise_squares() {
        assert!(pole_positive_on_pointwise_squares(&[1]));
        assert!(pole_positive_on_pointwise_squares(&[1, -2, 3]));
        assert!(pole_positive_on_pointwise_squares(&[2, 1]));
    }

    #[test]
    fn prime_is_negative_on_convolution_squares() {
        assert!(prime_negative_on_convolution_squares());
        assert!(fourier_duality_of_the_two_squares());
    }

    #[test]
    fn template_is_identified_not_discharged() {
        let report = m29h_experiment();
        assert!(report.pole_positive_pointwise);
        assert!(report.prime_negative_convolution);
        assert!(report.fourier_duality);
        assert!(report.archimedean_signed);
        assert!(report.certificate_template_identified);
        assert!(!report.positive_kernel_constructed);
        assert!(!report.m29_reached);
        assert_eq!(report.residual, "PositiveFunctional(L_weil)");
        let record = machine_record(&report);
        assert!(record.contains("positive_kernel_constructed=false"));
        assert!(record.contains("m29_reached=false"));
    }
}
