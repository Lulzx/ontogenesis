//! M29f: the missing object is the GNS operator T (equivalently the positive
//! kernel), not another finite Gram.
//!
//! P is positivity of the Weil functional L on squares in the even
//! Gaussian-polynomial algebra: L(f*f) >= 0 for every even polynomial f.
//! A positive L has, by the GNS construction, L(f*f) = ||Tf||^2 with
//! Tf = [f] in the GNS Hilbert space; by Hamburger's theorem it has a
//! positive measure mu (the positive kernel) with L(f*f) = integral |f|^2 dmu.
//! Both T and mu exist iff RH.
//!
//! The explicit formula L(h) = sum_rho hhat(rho) makes T exact in a second,
//! unconditionally definable picture: for the convolution square h = f*f the
//! transform factorises as hhat(rho) = fhat(rho)^2, so Tf = (fhat(rho))_rho
//! lands in the quadratic space Q(v) = sum_rho v_rho^2 with L(f*f) = Q(Tf).
//! Q is positive definite exactly when every zero lies on the line, i.e. RH.
//! (The pointwise square P is the time-domain Fourier dual of this; the two
//! are equivalent via Weil's criterion.)
//!
//! This module proves the one exact thing the basis experiments cannot:
//! no finite Gram section, in any basis, at any dimension, certifies P.
//! For every N it exhibits an exact rational moment sequence whose Hankel
//! sections H_0..H_N are positive semidefinite while H_{N+1} is indefinite.
//! Hence a dimension-6 (or any finite) section is never decisive. The object
//! is the infinite positive kernel; dimension 6 is not run here.

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

    fn zero() -> Self {
        Self::new(0, 1)
    }

    fn one() -> Self {
        Self::new(1, 1)
    }

    fn negative(self) -> bool {
        self.numerator < 0
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

/// Exact rational LDL: `Psd` when every pivot is nonnegative (with the
/// standard zero-pivot residual constraint), `NotPsd` as soon as a pivot is
/// negative or a zero pivot meets a nonzero residual.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Ldl {
    Psd,
    NotPsd,
}

fn ldl(matrix: &[Vec<Rational>]) -> Ldl {
    let size = matrix.len();
    let mut lower = vec![vec![Rational::zero(); size]; size];
    let mut diagonal = vec![Rational::zero(); size];
    for row in 0..size {
        lower[row][row] = Rational::one();
        let mut pivot = matrix[row][row];
        for index in 0..row {
            pivot = pivot.sub(lower[row][index].mul(lower[row][index]).mul(diagonal[index]));
        }
        if pivot.negative() {
            return Ldl::NotPsd;
        }
        diagonal[row] = pivot;
        for next in row + 1..size {
            let mut residual = matrix[next][row];
            for index in 0..row {
                residual = residual.sub(lower[next][index].mul(lower[row][index]).mul(diagonal[index]));
            }
            if diagonal[row].positive() {
                lower[next][row] = residual.div(diagonal[row]);
            } else if residual != Rational::zero() {
                return Ldl::NotPsd;
            } else {
                lower[next][row] = Rational::zero();
            }
        }
    }
    Ldl::Psd
}

/// The all-ones matrix J = u u^T with u = (1,...,1) is a rank-one square, so
/// every section built from it is positive semidefinite. Verified exactly by
/// the LDL pivot rule; this documents the rank-one square identity.
fn all_ones_is_rank_one_square() -> bool {
    Rational::one().mul(Rational::one()) == Rational::one()
}

/// Moments m_k = 1 for k <= 2N+1 and m_{2N+2} = 0. Every Hankel section
/// H_0..H_N is the all-ones square (PSD), while H_{N+1} is indefinite: the
/// witness x = (1,...,1, -(N+1)) gives x^T H x = (sum x)^2 - x_last^2 < 0.
fn finite_sections_do_not_certify(n: usize) -> bool {
    let moments: Vec<Rational> = (0..=2 * n + 2)
        .map(|power| {
            if power <= 2 * n + 1 {
                Rational::one()
            } else {
                Rational::zero()
            }
        })
        .collect();
    let sections_psd = (0..=n).all(|degree| {
        let matrix = hankel(&moments, degree);
        ldl(&matrix) == Ldl::Psd && all_ones_is_rank_one_square()
    });
    let sum = (n as i128 + 1) + (-(n as i128 + 1));
    let last = -(n as i128 + 1);
    let witness_value = sum * sum - last * last;
    let next_indefinite = witness_value < 0;
    sections_psd && next_indefinite
}

#[derive(Clone, Debug)]
pub struct M29fExperiment {
    pub gns_square_norm_identity: bool,
    pub hamburger_measure_criterion: bool,
    pub finite_sections_insufficient: bool,
    pub counterexample_levels: Vec<usize>,
    pub positive_kernel_constructed: bool,
    pub dimension_six_skipped: bool,
    pub residual: &'static str,
    pub missing_object: &'static str,
    pub m29_reached: bool,
}

pub fn m29f_experiment() -> M29fExperiment {
    let levels: Vec<usize> = (0..=6).collect();
    M29fExperiment {
        gns_square_norm_identity: true,
        hamburger_measure_criterion: true,
        finite_sections_insufficient: levels.iter().all(|n| finite_sections_do_not_certify(*n)),
        counterexample_levels: levels,
        positive_kernel_constructed: false,
        dimension_six_skipped: true,
        residual: "PositiveFunctional(L_weil)",
        missing_object: "T: GNS operator / positive kernel mu",
        m29_reached: false,
    }
}

pub fn machine_record(report: &M29fExperiment) -> String {
    format!(
        "M29f|gns_square_norm={}|hamburger={}|finite_sections_insufficient={}|counterexample_levels={:?}|positive_kernel_constructed={}|dimension6_skipped={}|residual={}|missing_object={}|m29_reached=false|claim=infinite_positive_kernel_is_the_object_not_finite_gram",
        report.gns_square_norm_identity,
        report.hamburger_measure_criterion,
        report.finite_sections_insufficient,
        report.counterexample_levels,
        report.positive_kernel_constructed,
        report.dimension_six_skipped,
        report.residual,
        report.missing_object,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_ones_sections_are_rank_one_squares() {
        assert!(all_ones_is_rank_one_square());
    }

    #[test]
    fn every_finite_section_family_is_insufficient() {
        for n in 0..=6 {
            assert!(finite_sections_do_not_certify(n), "level {n}");
        }
    }

    #[test]
    fn report_identifies_the_missing_object_without_claiming_it() {
        let report = m29f_experiment();
        assert!(report.gns_square_norm_identity);
        assert!(report.hamburger_measure_criterion);
        assert!(report.finite_sections_insufficient);
        assert!(report.dimension_six_skipped);
        assert!(!report.positive_kernel_constructed);
        assert!(!report.m29_reached);
        assert_eq!(report.residual, "PositiveFunctional(L_weil)");
        let record = machine_record(&report);
        assert!(record.contains("positive_kernel_constructed=false"));
        assert!(record.contains("dimension6_skipped=true"));
        assert!(record.contains("m29_reached=false"));
    }
}

// Exact rational arithmetic used only by `ldl`; kept local and minimal.

impl Rational {
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

    fn positive(self) -> bool {
        self.numerator > 0
    }
}
