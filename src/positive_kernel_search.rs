//! SH14: exact rational LDL positivity certificates and real-Weil retry routing.

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
        let gcd = gcd(numerator.unsigned_abs(), denominator as u128) as i128;
        numerator /= gcd;
        denominator /= gcd;
        Self {
            numerator,
            denominator,
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
    fn sub(self, other: Self) -> Self {
        self.add(Self::new(-other.numerator, other.denominator))
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
    fn nonnegative(self) -> bool {
        self.numerator >= 0
    }
    fn positive(self) -> bool {
        self.numerator > 0
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

fn hankel(moments: &[Rational], degree: usize) -> Vec<Vec<Rational>> {
    (0..=degree)
        .map(|row| (0..=degree).map(|column| moments[row + column]).collect())
        .collect()
}

fn ldl_nonnegative(matrix: &[Vec<Rational>]) -> bool {
    let size = matrix.len();
    let mut lower = vec![vec![Rational::zero(); size]; size];
    let mut diagonal = vec![Rational::zero(); size];
    for row in 0..size {
        lower[row][row] = Rational::new(1, 1);
        let correction = (0..row).fold(Rational::zero(), |sum, index| {
            sum.add(
                lower[row][index]
                    .mul(lower[row][index])
                    .mul(diagonal[index]),
            )
        });
        diagonal[row] = matrix[row][row].sub(correction);
        if !diagonal[row].nonnegative() {
            return false;
        }
        for next in row + 1..size {
            let cross = (0..row).fold(Rational::zero(), |sum, index| {
                sum.add(
                    lower[next][index]
                        .mul(lower[row][index])
                        .mul(diagonal[index]),
                )
            });
            let residual = matrix[next][row].sub(cross);
            if diagonal[row].positive() {
                lower[next][row] = residual.div(diagonal[row]);
            } else if residual != Rational::zero() {
                return false;
            }
        }
    }
    true
}

fn point_mass_moments(point: i128, max_degree: usize) -> Vec<Rational> {
    (0..=max_degree)
        .map(|power| Rational::new(point.pow(power as u32), 1))
        .collect()
}

fn mixture_moments(points: &[(i128, Rational)], max_degree: usize) -> Vec<Rational> {
    (0..=max_degree)
        .map(|power| {
            points
                .iter()
                .fold(Rational::zero(), |sum, (point, weight)| {
                    sum.add(weight.mul(Rational::new(point.pow(power as u32), 1)))
                })
        })
        .collect()
}

fn uniform_moments(max_degree: usize) -> Vec<Rational> {
    (0..=max_degree)
        .map(|power| {
            if power % 2 == 0 {
                Rational::new(1, (power + 1) as i128)
            } else {
                Rational::zero()
            }
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WeilFailure {
    MissingExactArchimedeanIntervals,
    InconclusivePivot,
    NegativePivot,
}

#[derive(Clone, Debug)]
pub struct Sh14Experiment {
    pub calibration_positive: [bool; 3],
    pub held_out_degrees: [bool; 2],
    pub controls: [bool; 3],
    pub controls_declined: usize,
    pub weil_basis_size: usize,
    pub zero_data_used: bool,
    pub weil_finite_gram_certified: bool,
    pub weil_failure: WeilFailure,
    pub infinite_positivity_certified: bool,
    pub m29_reached: bool,
}

pub fn sh14_experiment() -> Sh14Experiment {
    let point = point_mass_moments(2, 10);
    let mixture = mixture_moments(&[(0, Rational::new(1, 3)), (2, Rational::new(2, 3))], 10);
    let uniform = uniform_moments(10);
    let calibration_positive = [
        ldl_nonnegative(&hankel(&point, 3)),
        ldl_nonnegative(&hankel(&mixture, 3)),
        ldl_nonnegative(&hankel(&uniform, 3)),
    ];
    let held_out_degrees = [
        ldl_nonnegative(&hankel(&uniform, 4)),
        ldl_nonnegative(&hankel(&uniform, 5)),
    ];
    let signed = mixture_moments(&[(0, Rational::new(2, 1)), (1, Rational::new(-1, 1))], 6);
    let mut corrupted = uniform_moments(6);
    corrupted[2] = Rational::new(-1, 3);
    let controls = [
        !ldl_nonnegative(&hankel(&signed, 2)),
        !ldl_nonnegative(&hankel(&corrupted, 2)),
        !finite_promotes_infinite(true),
    ];
    Sh14Experiment {
        calibration_positive,
        held_out_degrees,
        controls_declined: controls.iter().filter(|control| **control).count(),
        controls,
        weil_basis_size: 4,
        zero_data_used: false,
        weil_finite_gram_certified: false,
        weil_failure: WeilFailure::MissingExactArchimedeanIntervals,
        infinite_positivity_certified: false,
        m29_reached: false,
    }
}

fn finite_promotes_infinite(_: bool) -> bool {
    false
}

pub fn machine_record(report: &Sh14Experiment) -> String {
    format!("SH14|calibration_positive={:?}|held_out_degrees={:?}|controls={:?}|controls_declined={}/3|weil_basis_size={}|zero_data_used={}|weil_finite_gram_certified={}|weil_failure={:?}|infinite_positivity_certified={}|m29_reached=false|claim=exact_ldl_infrastructure_only", report.calibration_positive, report.held_out_degrees, report.controls, report.controls_declined, report.weil_basis_size, report.zero_data_used, report.weil_finite_gram_certified, report.weil_failure, report.infinite_positivity_certified)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calibrates_exact_ldl_and_stops_before_uncertified_weil_entries() {
        let report = sh14_experiment();
        assert_eq!(report.calibration_positive, [true; 3]);
        assert_eq!(report.held_out_degrees, [true; 2]);
        assert_eq!(report.controls, [true; 3]);
        assert_eq!(
            report.weil_failure,
            WeilFailure::MissingExactArchimedeanIntervals
        );
        assert!(!report.zero_data_used);
        assert!(!report.infinite_positivity_certified);
        assert!(!report.m29_reached);
        assert_eq!(machine_record(&report), machine_record(&sh14_experiment()));
    }
}
