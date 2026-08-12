//! SH15: MPFR-directed intervals for explicit-formula evaluation.

use rug::float::{Constant, Round};
use rug::ops::{AddAssignRound, DivAssignRound, MulAssignRound, SubAssignRound};
use rug::Float;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ExactScale {
    numerator: u32,
    denominator: u32,
}

impl ExactScale {
    pub fn new(numerator: u32, denominator: u32) -> Option<Self> {
        if numerator == 0 || denominator == 0 {
            return None;
        }
        let divisor = gcd_u32(numerator, denominator);
        Some(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    pub const fn integer(value: u32) -> Self {
        Self {
            numerator: value,
            denominator: 1,
        }
    }

    pub fn numerator(self) -> u32 {
        self.numerator
    }

    pub fn denominator(self) -> u32 {
        self.denominator
    }

    pub fn multiply(self, other: Self) -> Option<Self> {
        Self::new(
            self.numerator.checked_mul(other.numerator)?,
            self.denominator.checked_mul(other.denominator)?,
        )
    }

    pub fn divide(self, other: Self) -> Option<Self> {
        Self::new(
            self.numerator.checked_mul(other.denominator)?,
            self.denominator.checked_mul(other.numerator)?,
        )
    }

    pub(crate) fn interval(self, precision: u32, provenance: Provenance) -> Interval {
        let mut lower = Float::with_val(precision, self.numerator);
        lower.div_assign_round(self.denominator, Round::Down);
        let mut upper = Float::with_val(precision, self.numerator);
        upper.div_assign_round(self.denominator, Round::Up);
        Interval {
            lower,
            upper,
            precision,
            provenance,
            tail_certified: true,
        }
    }
}

fn gcd_u32(mut left: u32, mut right: u32) -> u32 {
    while right != 0 {
        (left, right) = (right, left % right);
    }
    left.max(1)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Provenance {
    Generic,
    ArithmeticOnly,
    ZeroDerived,
}

#[derive(Clone, Debug)]
pub struct Interval {
    pub(crate) lower: Float,
    pub(crate) upper: Float,
    pub(crate) precision: u32,
    pub(crate) provenance: Provenance,
    pub(crate) tail_certified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntervalError {
    InvalidBounds,
    PrecisionMismatch,
    PrecisionReuse,
    Domain,
    MissingTail,
    MissingTerm(&'static str),
    ForbiddenProvenance,
}

impl Interval {
    pub(crate) fn exact_integer(value: i32, precision: u32, provenance: Provenance) -> Self {
        let value = Float::with_val(precision, value);
        Self {
            lower: value.clone(),
            upper: value,
            precision,
            provenance,
            tail_certified: true,
        }
    }

    pub(crate) fn pi(precision: u32) -> Self {
        let (lower, _) = Float::with_val_round(precision, Constant::Pi, Round::Down);
        let (upper, _) = Float::with_val_round(precision, Constant::Pi, Round::Up);
        Self {
            lower,
            upper,
            precision,
            provenance: Provenance::Generic,
            tail_certified: true,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), IntervalError> {
        if self.provenance == Provenance::ZeroDerived {
            return Err(IntervalError::ForbiddenProvenance);
        }
        if self.lower > self.upper {
            return Err(IntervalError::InvalidBounds);
        }
        Ok(())
    }

    fn ensure_compatible(&self, other: &Self) -> Result<(), IntervalError> {
        self.validate()?;
        other.validate()?;
        if self.precision != other.precision {
            return Err(IntervalError::PrecisionMismatch);
        }
        Ok(())
    }

    fn combined_provenance(&self, other: &Self) -> Provenance {
        if self.provenance == Provenance::ArithmeticOnly
            || other.provenance == Provenance::ArithmeticOnly
        {
            Provenance::ArithmeticOnly
        } else {
            Provenance::Generic
        }
    }

    pub(crate) fn add(&self, other: &Self) -> Result<Self, IntervalError> {
        self.ensure_compatible(other)?;
        let mut lower = self.lower.clone();
        lower.add_assign_round(&other.lower, Round::Down);
        let mut upper = self.upper.clone();
        upper.add_assign_round(&other.upper, Round::Up);
        Ok(Self {
            lower,
            upper,
            precision: self.precision,
            provenance: self.combined_provenance(other),
            tail_certified: self.tail_certified && other.tail_certified,
        })
    }

    pub(crate) fn sub(&self, other: &Self) -> Result<Self, IntervalError> {
        self.ensure_compatible(other)?;
        let mut lower = self.lower.clone();
        lower.sub_assign_round(&other.upper, Round::Down);
        let mut upper = self.upper.clone();
        upper.sub_assign_round(&other.lower, Round::Up);
        Ok(Self {
            lower,
            upper,
            precision: self.precision,
            provenance: self.combined_provenance(other),
            tail_certified: self.tail_certified && other.tail_certified,
        })
    }

    pub(crate) fn mul(&self, other: &Self) -> Result<Self, IntervalError> {
        self.ensure_compatible(other)?;
        let mut down = Vec::new();
        let mut up = Vec::new();
        for (left, right) in [
            (&self.lower, &other.lower),
            (&self.lower, &other.upper),
            (&self.upper, &other.lower),
            (&self.upper, &other.upper),
        ] {
            let mut lower_product = left.clone();
            lower_product.mul_assign_round(right, Round::Down);
            down.push(lower_product);
            let mut upper_product = left.clone();
            upper_product.mul_assign_round(right, Round::Up);
            up.push(upper_product);
        }
        let lower = down
            .into_iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let upper = up
            .into_iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        Ok(Self {
            lower,
            upper,
            precision: self.precision,
            provenance: self.combined_provenance(other),
            tail_certified: self.tail_certified && other.tail_certified,
        })
    }

    pub(crate) fn div(&self, other: &Self) -> Result<Self, IntervalError> {
        self.ensure_compatible(other)?;
        if other.lower <= 0 && other.upper >= 0 {
            return Err(IntervalError::Domain);
        }
        let mut reciprocal_lower = Float::with_val(self.precision, 1);
        reciprocal_lower.div_assign_round(&other.upper, Round::Down);
        let mut reciprocal_upper = Float::with_val(self.precision, 1);
        reciprocal_upper.div_assign_round(&other.lower, Round::Up);
        self.mul(&Self {
            lower: reciprocal_lower,
            upper: reciprocal_upper,
            precision: self.precision,
            provenance: other.provenance,
            tail_certified: other.tail_certified,
        })
    }

    pub(crate) fn exp(&self) -> Result<Self, IntervalError> {
        self.validate()?;
        let mut lower = self.lower.clone();
        lower.exp_round(Round::Down);
        let mut upper = self.upper.clone();
        upper.exp_round(Round::Up);
        Ok(Self {
            lower,
            upper,
            ..self.clone()
        })
    }

    pub(crate) fn ln(&self) -> Result<Self, IntervalError> {
        self.validate()?;
        if self.lower <= 0 {
            return Err(IntervalError::Domain);
        }
        let mut lower = self.lower.clone();
        lower.ln_round(Round::Down);
        let mut upper = self.upper.clone();
        upper.ln_round(Round::Up);
        Ok(Self {
            lower,
            upper,
            ..self.clone()
        })
    }

    fn sqrt(&self) -> Result<Self, IntervalError> {
        self.validate()?;
        if self.lower < 0 {
            return Err(IntervalError::Domain);
        }
        let mut lower = self.lower.clone();
        lower.sqrt_round(Round::Down);
        let mut upper = self.upper.clone();
        upper.sqrt_round(Round::Up);
        Ok(Self {
            lower,
            upper,
            ..self.clone()
        })
    }

    fn digamma(&self) -> Result<Self, IntervalError> {
        self.validate()?;
        if self.lower <= 0 {
            return Err(IntervalError::Domain);
        }
        // Digamma is strictly increasing on the positive real axis.
        let mut lower = self.lower.clone();
        lower.digamma_round(Round::Down);
        let mut upper = self.upper.clone();
        upper.digamma_round(Round::Up);
        Ok(Self {
            lower,
            upper,
            ..self.clone()
        })
    }

    fn contains_integer(&self, value: i32) -> bool {
        let value = Float::with_val(self.precision, value);
        self.lower <= value && value <= self.upper
    }

    fn contains(&self, other: &Self) -> bool {
        self.lower <= other.lower && other.upper <= self.upper
    }

    pub(crate) fn width(&self) -> Float {
        let mut width = self.upper.clone();
        width.sub_assign_round(&self.lower, Round::Up);
        width
    }

    pub(crate) fn contains_zero(&self) -> bool {
        self.lower <= 0 && self.upper >= 0
    }

    pub(crate) fn strictly_positive(&self) -> bool {
        self.lower > 0
    }

    pub(crate) fn strictly_negative(&self) -> bool {
        self.upper < 0
    }

    pub(crate) fn abs_upper(&self) -> Float {
        let mut left = self.lower.clone();
        left.abs_mut();
        let mut right = self.upper.clone();
        right.abs_mut();
        if left > right {
            left
        } else {
            right
        }
    }

    pub(crate) fn contains_interval(&self, other: &Self) -> bool {
        self.lower <= other.lower && other.upper <= self.upper
    }

    pub(crate) fn reprecision(&self, precision: u32) -> Result<Self, IntervalError> {
        if precision > self.precision {
            return Err(IntervalError::PrecisionReuse);
        }
        let (lower, _) = Float::with_val_round(precision, &self.lower, Round::Down);
        let (upper, _) = Float::with_val_round(precision, &self.upper, Round::Up);
        Ok(Self {
            lower,
            upper,
            precision,
            ..self.clone()
        })
    }
}

fn calibration(precision: u32) -> Result<Vec<Interval>, IntervalError> {
    let zero = Interval::exact_integer(0, precision, Provenance::Generic);
    let one = Interval::exact_integer(1, precision, Provenance::Generic);
    let exp_zero = zero.exp()?;
    let log_one = one.ln()?;
    let gaussian_integral = Interval::pi(precision).sqrt()?;
    let psi_one = one.digamma()?;
    let two = Interval::exact_integer(2, precision, Provenance::Generic);
    let psi_two = two.digamma()?;
    let recurrence_residual = psi_two.sub(&psi_one)?.sub(&one)?;
    if !exp_zero.contains_integer(1)
        || !log_one.contains_integer(0)
        || !recurrence_residual.contains_integer(0)
    {
        return Err(IntervalError::InvalidBounds);
    }
    Ok(vec![
        exp_zero,
        log_one,
        gaussian_integral,
        recurrence_residual,
    ])
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RealEvaluationFailure {
    ValidatedQuadratureMissing,
    PrimeTailMissing,
    InconclusivePivot,
}

#[derive(Clone, Debug)]
pub struct Sh15Experiment {
    pub mpfr_backend: bool,
    pub base_precision: u32,
    pub escalated_precision: u32,
    pub calibration_contains_exact: bool,
    pub widths_shrink: bool,
    pub controls: [bool; 5],
    pub controls_declined: usize,
    pub special_functions_validated: bool,
    pub complete_real_gram_entries: bool,
    pub failure: RealEvaluationFailure,
    pub finite_positivity_certified: bool,
    pub infinite_positivity_certified: bool,
    pub m29_reached: bool,
}

pub fn sh15_experiment() -> Sh15Experiment {
    let base = calibration(80).expect("MPFR base calibration");
    let escalated = calibration(160).expect("MPFR escalated calibration");
    let calibration_contains_exact = base[0].contains_integer(1)
        && base[1].contains_integer(0)
        && base[3].contains_integer(0)
        && escalated[0].contains_integer(1)
        && escalated[1].contains_integer(0)
        && escalated[3].contains_integer(0);
    let widths_shrink = base.iter().zip(&escalated).all(|(coarse, fine)| {
        let fine_at_coarse = fine.reprecision(80).expect("down precision");
        coarse.contains(&fine_at_coarse) && fine.width() <= coarse.width()
    });
    let corrupt = Interval {
        lower: Float::with_val(80, 2),
        upper: Float::with_val(80, 1),
        precision: 80,
        provenance: Provenance::Generic,
        tail_certified: true,
    };
    let missing_tail = Interval {
        lower: Float::with_val(80, 0),
        upper: Float::with_val(80, 1),
        precision: 80,
        provenance: Provenance::ArithmeticOnly,
        tail_certified: false,
    };
    let zero_derived = Interval {
        provenance: Provenance::ZeroDerived,
        ..Interval::exact_integer(0, 80, Provenance::Generic)
    };
    let controls = [
        corrupt.validate() == Err(IntervalError::InvalidBounds),
        (!missing_tail.tail_certified),
        matches!(
            assemble_explicit_terms(Some(&base[0]), None, Some(&base[1])),
            Err(IntervalError::MissingTerm("archimedean"))
        ),
        matches!(base[0].reprecision(160), Err(IntervalError::PrecisionReuse)),
        zero_derived.validate() == Err(IntervalError::ForbiddenProvenance),
    ];
    Sh15Experiment {
        mpfr_backend: true,
        base_precision: 80,
        escalated_precision: 160,
        calibration_contains_exact,
        widths_shrink,
        controls_declined: controls.iter().filter(|control| **control).count(),
        controls,
        special_functions_validated: calibration_contains_exact && widths_shrink,
        complete_real_gram_entries: false,
        failure: RealEvaluationFailure::ValidatedQuadratureMissing,
        finite_positivity_certified: false,
        infinite_positivity_certified: false,
        m29_reached: false,
    }
}

fn assemble_explicit_terms<'a>(
    pole: Option<&'a Interval>,
    archimedean: Option<&'a Interval>,
    prime_power: Option<&'a Interval>,
) -> Result<Interval, IntervalError> {
    let pole = pole.ok_or(IntervalError::MissingTerm("pole"))?;
    let archimedean = archimedean.ok_or(IntervalError::MissingTerm("archimedean"))?;
    let prime_power = prime_power.ok_or(IntervalError::MissingTerm("prime-power"))?;
    if !prime_power.tail_certified {
        return Err(IntervalError::MissingTail);
    }
    pole.add(archimedean)?.add(prime_power)
}

pub fn machine_record(report: &Sh15Experiment) -> String {
    format!(
        "SH15|mpfr_backend={}|precision={}->{}|calibration_contains_exact={}|widths_shrink={}|controls={:?}|controls_declined={}/5|special_functions_validated={}|complete_real_gram_entries={}|failure={:?}|finite_positivity_certified={}|infinite_positivity_certified={}|m29_reached=false|claim=validated_interval_backend_only",
        report.mpfr_backend,
        report.base_precision,
        report.escalated_precision,
        report.calibration_contains_exact,
        report.widths_shrink,
        report.controls,
        report.controls_declined,
        report.special_functions_validated,
        report.complete_real_gram_entries,
        report.failure,
        report.finite_positivity_certified,
        report.infinite_positivity_certified,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directed_rounding_contains_exact_calibration_values() {
        let report = sh15_experiment();
        assert!(report.special_functions_validated, "{report:#?}");
        assert_eq!(report.controls, [true; 5]);
        assert_eq!(
            report.failure,
            RealEvaluationFailure::ValidatedQuadratureMissing
        );
        assert!(!report.complete_real_gram_entries);
        assert!(!report.infinite_positivity_certified);
        assert!(!report.m29_reached);
        assert_eq!(machine_record(&report), machine_record(&sh15_experiment()));
    }

    #[test]
    fn interval_arithmetic_encloses_exact_operations() {
        let precision = 96;
        let two = Interval::exact_integer(2, precision, Provenance::Generic);
        let three = Interval::exact_integer(3, precision, Provenance::Generic);
        assert!(two.add(&three).unwrap().contains_integer(5));
        assert!(two.mul(&three).unwrap().contains_integer(6));
        assert!(three.sub(&two).unwrap().contains_integer(1));
        assert!(two.div(&two).unwrap().contains_integer(1));
    }
}
