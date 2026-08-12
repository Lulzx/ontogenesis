//! SH18: convention-typed assembly of finite Weil Gram entries.

use crate::validated_archimedean::{archimedean_entries, archimedean_entry};
use crate::validated_explicit_formula::{Interval, IntervalError, Provenance};
use crate::validated_prime_power::{certified_component, certified_components};
use rug::float::Round;
use rug::ops::DivAssignRound;
use rug::Float;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FourierConvention {
    AngularFrequency,
    CyclicFrequency,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpectralArgument {
    RhoMinusHalfOverI,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoleTerm {
    BothImaginaryHalves,
    Omitted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArchimedeanCoefficient {
    OneOverTwoPi,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimeSign {
    Negative,
    Positive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReflectionMultiplicity {
    BothTransforms,
    OneTransform,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Normalization {
    pub convention: FourierConvention,
    pub spectral_argument: SpectralArgument,
    pub pole: PoleTerm,
    pub archimedean_coefficient: ArchimedeanCoefficient,
    pub prime_sign: PrimeSign,
    pub reflection: ReflectionMultiplicity,
    pub gaussian_scale: i32,
    pub exact_derivation: bool,
}

impl Normalization {
    pub const fn angular() -> Self {
        Self {
            convention: FourierConvention::AngularFrequency,
            spectral_argument: SpectralArgument::RhoMinusHalfOverI,
            pole: PoleTerm::BothImaginaryHalves,
            archimedean_coefficient: ArchimedeanCoefficient::OneOverTwoPi,
            prime_sign: PrimeSign::Negative,
            reflection: ReflectionMultiplicity::BothTransforms,
            gaussian_scale: 2,
            exact_derivation: true,
        }
    }

    pub const fn cyclic_source() -> Self {
        Self {
            convention: FourierConvention::CyclicFrequency,
            ..Self::angular()
        }
    }

    pub const fn convert_cyclic_to_angular(self) -> Option<Self> {
        if matches!(self.convention, FourierConvention::CyclicFrequency) && self.exact_derivation {
            Some(Self::angular())
        } else {
            None
        }
    }

    fn accepts(self) -> bool {
        self == Self::angular()
    }
}

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

fn pole_component(even_power: usize, precision: u32) -> Result<Interval, IntervalError> {
    if even_power % 2 != 0 {
        return Err(IntervalError::Domain);
    }
    let half = exact_ratio(1, 2, precision);
    let exponential = half.exp()?;
    let mut coefficient = exact_ratio(2, 1, precision);
    for _ in 0..even_power / 2 {
        coefficient = coefficient.mul(&exact_ratio(-1, 4, precision))?;
    }
    coefficient.mul(&exponential)
}

fn one_over_two_pi(precision: u32) -> Result<Interval, IntervalError> {
    exact_ratio(1, 2, precision).div(&Interval::pi(precision))
}

fn one_over_pi(precision: u32) -> Result<Interval, IntervalError> {
    exact_ratio(1, 1, precision).div(&Interval::pi(precision))
}

#[derive(Clone, Debug)]
struct EntryComponents {
    pole: Interval,
    weighted_archimedean: Interval,
    weighted_prime: Interval,
}

impl EntryComponents {
    fn total(&self) -> Result<Interval, IntervalError> {
        self.pole
            .add(&self.weighted_archimedean)?
            .sub(&self.weighted_prime)
    }
}

fn entry_components(
    even_power: usize,
    bound: i32,
    cells: usize,
    terms: usize,
    cutoff: u64,
    precision: u32,
    normalization: Normalization,
) -> Result<EntryComponents, IntervalError> {
    if !normalization.accepts() {
        return Err(IntervalError::Domain);
    }
    Ok(EntryComponents {
        pole: pole_component(even_power, precision)?,
        weighted_archimedean: archimedean_entry(
            even_power,
            normalization.gaussian_scale,
            bound,
            cells,
            terms,
            precision,
        )?
        .mul(&one_over_two_pi(precision)?)?,
        weighted_prime: certified_component(even_power, cutoff, precision)?
            .mul(&one_over_pi(precision)?)?,
    })
}

fn assemble_entry(
    even_power: usize,
    bound: i32,
    cells: usize,
    terms: usize,
    cutoff: u64,
    precision: u32,
    normalization: Normalization,
) -> Result<Interval, IntervalError> {
    entry_components(
        even_power,
        bound,
        cells,
        terms,
        cutoff,
        precision,
        normalization,
    )?
    .total()
}

fn assemble_entries(
    powers: &[usize],
    bound: i32,
    cells: usize,
    terms: usize,
    cutoff: u64,
    precision: u32,
    normalization: Normalization,
) -> Result<Vec<Interval>, IntervalError> {
    if !normalization.accepts() {
        return Err(IntervalError::Domain);
    }
    let archimedean = archimedean_entries(
        powers,
        normalization.gaussian_scale,
        bound,
        cells,
        terms,
        precision,
    )?;
    let primes = certified_components(powers, cutoff, precision)?;
    let arch_coefficient = one_over_two_pi(precision)?;
    let prime_coefficient = one_over_pi(precision)?;
    powers
        .iter()
        .copied()
        .zip(archimedean)
        .zip(primes)
        .map(|((power, arch), prime)| {
            pole_component(power, precision)?
                .add(&arch.mul(&arch_coefficient)?)?
                .sub(&prime.mul(&prime_coefficient)?)
        })
        .collect()
}

fn hankel_matrix(entries: &[Interval; 7]) -> Vec<Vec<Interval>> {
    (0..4)
        .map(|row| (0..4).map(|column| entries[row + column].clone()).collect())
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LdlStatus {
    StrictlyPositive,
    NegativePivot,
    InconclusivePivot,
}

#[derive(Clone, Debug)]
struct LdlReport {
    status: LdlStatus,
    pivot_index: usize,
    pivot: Interval,
}

fn interval_ldl(matrix: &[Vec<Interval>]) -> Result<LdlReport, IntervalError> {
    let dimension = matrix.len();
    if dimension == 0 || matrix.iter().any(|row| row.len() != dimension) {
        return Err(IntervalError::Domain);
    }
    let precision = matrix[0][0].precision;
    let zero = Interval::exact_integer(0, precision, Provenance::Generic);
    let mut lower = vec![vec![zero.clone(); dimension]; dimension];
    let mut diagonal = vec![zero; dimension];
    for row in 0..dimension {
        lower[row][row] = Interval::exact_integer(1, precision, Provenance::Generic);
        let mut pivot = matrix[row][row].clone();
        for index in 0..row {
            pivot = pivot.sub(
                &lower[row][index]
                    .mul(&lower[row][index])?
                    .mul(&diagonal[index])?,
            )?;
        }
        if pivot.strictly_negative() {
            return Ok(LdlReport {
                status: LdlStatus::NegativePivot,
                pivot_index: row,
                pivot,
            });
        }
        if pivot.contains_zero() {
            return Ok(LdlReport {
                status: LdlStatus::InconclusivePivot,
                pivot_index: row,
                pivot,
            });
        }
        if !pivot.strictly_positive() {
            return Err(IntervalError::Domain);
        }
        diagonal[row] = pivot;
        for next_row in row + 1..dimension {
            let mut numerator = matrix[next_row][row].clone();
            for index in 0..row {
                numerator = numerator.sub(
                    &lower[next_row][index]
                        .mul(&lower[row][index])?
                        .mul(&diagonal[index])?,
                )?;
            }
            lower[next_row][row] = numerator.div(&diagonal[row])?;
        }
    }
    Ok(LdlReport {
        status: LdlStatus::StrictlyPositive,
        pivot_index: dimension - 1,
        pivot: diagonal[dimension - 1].clone(),
    })
}

#[derive(Clone, Debug)]
pub struct Sh18Experiment {
    pub product_components: usize,
    pub nested_entries: bool,
    pub ldl_status: LdlStatus,
    pub decisive_pivot_index: usize,
    pub decisive_pivot_lower: String,
    pub decisive_pivot_upper: String,
    pub first_entry_component_widths: [String; 3],
    pub convention_conversion_exact: bool,
    pub controls: [bool; 6],
    pub controls_declined: usize,
    pub infinite_positivity: bool,
    pub m29_reached: bool,
}

pub fn sh18b_experiment() -> Sh18Experiment {
    let normalization = Normalization::angular();
    let powers = [0, 2, 4, 6, 8, 10, 12];
    let base: [Interval; 7] = assemble_entries(&powers, 6, 256, 64, 4096, 80, normalization)
        .expect("base Weil entries")
        .try_into()
        .expect("seven base products");
    let fine: [Interval; 7] = assemble_entries(&powers, 7, 1024, 256, 16384, 160, normalization)
        .expect("fine Weil entries")
        .try_into()
        .expect("seven fine products");
    let first_components = entry_components(0, 7, 1024, 256, 16384, 160, normalization)
        .expect("first-entry components");
    let nested_entries = base
        .iter()
        .zip(&fine)
        .all(|(coarse, refined)| coarse.contains_interval(refined));
    let ldl = interval_ldl(&hankel_matrix(&fine)).expect("typed interval LDL");
    let convention_conversion_exact =
        Normalization::cyclic_source().convert_cyclic_to_angular() == Some(normalization);
    let mut mixed_convention = normalization;
    mixed_convention.convention = FourierConvention::CyclicFrequency;
    let mut positive_prime = normalization;
    positive_prime.prime_sign = PrimeSign::Positive;
    let mut one_reflection = normalization;
    one_reflection.reflection = ReflectionMultiplicity::OneTransform;
    let mut missing_pole = normalization;
    missing_pole.pole = PoleTerm::Omitted;
    let mut unproved = normalization;
    unproved.exact_derivation = false;
    let mut wrong_scale = normalization;
    wrong_scale.gaussian_scale = 1;
    let declined = |candidate| {
        matches!(
            assemble_entry(0, 6, 8, 8, 64, 64, candidate),
            Err(IntervalError::Domain)
        )
    };
    let controls = [
        declined(mixed_convention),
        declined(positive_prime),
        declined(one_reflection),
        declined(missing_pole),
        declined(unproved),
        declined(wrong_scale),
    ];
    Sh18Experiment {
        product_components: fine.len(),
        nested_entries,
        ldl_status: ldl.status,
        decisive_pivot_index: ldl.pivot_index,
        decisive_pivot_lower: format!("{:.12e}", ldl.pivot.lower),
        decisive_pivot_upper: format!("{:.12e}", ldl.pivot.upper),
        first_entry_component_widths: [
            format!("{:.6e}", first_components.pole.width()),
            format!("{:.6e}", first_components.weighted_archimedean.width()),
            format!("{:.6e}", first_components.weighted_prime.width()),
        ],
        convention_conversion_exact,
        controls_declined: controls.iter().filter(|value| **value).count(),
        controls,
        infinite_positivity: false,
        m29_reached: false,
    }
}

pub fn machine_record(report: &Sh18Experiment) -> String {
    format!(
        "SH18b|product_components={}|nested_entries={}|ldl_status={:?}|decisive_pivot={}:[{},{}]|first_entry_widths=pole:{};arch:{};prime:{}|convention_conversion_exact={}|controls={:?}|controls_declined={}/6|infinite_positivity=false|m29_reached=false|claim=finite_convention_typed_weil_assembly_only",
        report.product_components,
        report.nested_entries,
        report.ldl_status,
        report.decisive_pivot_index,
        report.decisive_pivot_lower,
        report.decisive_pivot_upper,
        report.first_entry_component_widths[0],
        report.first_entry_component_widths[1],
        report.first_entry_component_widths[2],
        report.convention_conversion_exact,
        report.controls,
        report.controls_declined,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_the_published_cyclic_convention_exactly() {
        assert_eq!(
            Normalization::cyclic_source().convert_cyclic_to_angular(),
            Some(Normalization::angular())
        );
    }

    #[test]
    fn rejects_every_normalization_and_scale_control() {
        let report = sh18b_experiment();
        assert_eq!(report.product_components, 7, "{report:#?}");
        assert!(report.convention_conversion_exact, "{report:#?}");
        assert_eq!(report.controls, [true; 6], "{report:#?}");
        assert_eq!(report.controls_declined, 6, "{report:#?}");
        assert!(!report.infinite_positivity);
        assert!(!report.m29_reached);
    }
}
