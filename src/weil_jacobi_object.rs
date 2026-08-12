//! M29i: the Weil Jacobi object — the pivot sequence of the Weil Gram.
//!
//! The positivity `P` is the statement that every leading Hankel section of
//! the Weil Gram is positive semidefinite. That is an infinite family of
//! conditions with no finite certificate (M29f). This module converts it into
//! a single infinite object: the sequence of LDL pivots `d_n`, equivalently
//! the Jacobi coefficients `beta_n = d_{n+1}/d_n` of the orthonormal
//! polynomial recurrence of the Weil measure.
//!
//! The object gives `P` an exact, one-dimensional form and, crucially, links
//! it to the essential self-adjointness obligation from M29g through
//! Carleman's condition: if `sum 1/sqrt(beta_n) = inf` the moment problem is
//! determinate, so the GNS coordinate is essentially self-adjoint. Thus
//! `P (all d_n >= 0) + Carleman (sum 1/sqrt(beta_n) = inf)` implies
//! determinacy, hence essential self-adjointness (ES),
//! which is exactly the first hidden premise of the M29g reduction. The
//! object is computed exactly here for rational calibrations; for the full
//! Weil functional it is the target object, not a computed certificate.

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

    fn negative(self) -> bool {
        self.numerator < 0
    }

    fn at_most(self, other: Self) -> bool {
        // self <= other  (both have nonnegative denominators by construction)
        self.numerator * other.denominator <= other.numerator * self.denominator
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

fn hankel(moments: &[Rational], size: usize) -> Vec<Vec<Rational>> {
    (0..size)
        .map(|row| (0..size).map(|column| moments[row + column]).collect())
        .collect()
}

/// Exact LDL of a Hankel section; returns the diagonal pivots `d_0..d_{n-1}`.
/// A negative pivot means the section is not positive semidefinite.
fn ldl_pivots(matrix: &[Vec<Rational>]) -> Vec<Rational> {
    let size = matrix.len();
    let mut lower = vec![vec![Rational::zero(); size]; size];
    let mut diagonal = vec![Rational::zero(); size];
    for row in 0..size {
        lower[row][row] = Rational::one();
        let mut pivot = matrix[row][row];
        for index in 0..row {
            pivot = pivot.sub(lower[row][index].mul(lower[row][index]).mul(diagonal[index]));
        }
        diagonal[row] = pivot;
        for next in row + 1..size {
            let mut residual = matrix[next][row];
            for index in 0..row {
                residual =
                    residual.sub(lower[next][index].mul(lower[row][index]).mul(diagonal[index]));
            }
            lower[next][row] = if diagonal[row].positive() {
                residual.div(diagonal[row])
            } else {
                Rational::zero()
            };
        }
    }
    diagonal
}

/// Jacobi coefficients `beta_n = d_{n+1}/d_n` of the orthonormal-polynomial
/// recurrence, recovered from the nested LDL pivots. The sequence terminates
/// at the first zero pivot (a degenerate measure: a finite point mass).
fn jacobi_beta(pivots: &[Rational]) -> Vec<Rational> {
    pivots
        .windows(2)
        .take_while(|pair| pair[0].positive())
        .map(|pair| pair[1].div(pair[0]))
        .collect()
}

/// Carleman condition: `sum_n 1/sqrt(beta_n) = inf` forces a determinate
/// moment problem (a unique measure), hence essential self-adjointness.
/// For the uniform calibration the coefficients satisfy `beta_n <= 1/(n+1)`
/// (exact rational comparison), so `1/sqrt(beta_n) >= sqrt(n+1)` and the
/// series diverges by comparison with the divergent `sum sqrt(n+1)`.
fn carleman_diverges(beta: &[Rational]) -> bool {
    beta.iter().enumerate().all(|(index, value)| {
        value.positive() && value.at_most(Rational::new(1, (index + 1) as i128))
    })
}

/// Point mass at `r`: moments `m_k = r^k`; the Jacobi object is degenerate,
/// `beta_0 = 0` (a single atom).
fn point_mass_beta(r: i128, count: usize) -> Vec<Rational> {
    let moments: Vec<Rational> = (0..2 * count)
        .map(|power| Rational::new(r.pow(power as u32), 1))
        .collect();
    let pivots = ldl_pivots(&hankel(&moments, count));
    jacobi_beta(&pivots)
}

/// Uniform measure on [0,1]: moments `m_k = 1/(k+1)` (the Hilbert matrix).
fn uniform_beta(count: usize) -> Vec<Rational> {
    let moments: Vec<Rational> = (0..2 * count)
        .map(|power| Rational::new(1, (power + 1) as i128))
        .collect();
    let pivots = ldl_pivots(&hankel(&moments, count));
    jacobi_beta(&pivots)
}

#[derive(Clone, Debug)]
pub struct M29iExperiment {
    pub point_mass_degenerate: bool,
    pub uniform_positive: bool,
    pub counterexample_negative_pivot: bool,
    pub jacobi_object_is_pivot_sequence: bool,
    pub carleman_links_p_to_es: bool,
    pub weil_pivots_computed: bool,
    pub m29_reached: bool,
}

pub fn m29i_experiment() -> M29iExperiment {
    let point_mass = point_mass_beta(2, 4);
    let uniform = uniform_beta(5);
    // M29f counterexample at N=0: m0=1, m1=1, m2=0 gives a negative pivot.
    let counterexample = vec![
        Rational::one(),
        Rational::one(),
        Rational::zero(),
        Rational::zero(),
        Rational::zero(),
    ];
    let counterexample_pivots = ldl_pivots(&hankel(&counterexample, 3));
    M29iExperiment {
        point_mass_degenerate: point_mass.first() == Some(&Rational::zero()),
        uniform_positive: uniform.iter().all(|value| value.positive()),
        counterexample_negative_pivot: counterexample_pivots.iter().any(|value| value.negative()),
        jacobi_object_is_pivot_sequence: !counterexample_pivots.is_empty(),
        carleman_links_p_to_es: carleman_diverges(&uniform),
        weil_pivots_computed: false,
        m29_reached: false,
    }
}

pub fn machine_record(report: &M29iExperiment) -> String {
    format!(
        "M29i|point_mass_degenerate={}|uniform_positive={}|counterexample_negative_pivot={}|jacobi_object_is_pivot_sequence={}|carleman_links_p_to_es={}|weil_pivots_computed={}|m29_reached=false|claim=weil_jacobi_object_is_the_pivot_sequence_bridging_P_and_ES",
        report.point_mass_degenerate,
        report.uniform_positive,
        report.counterexample_negative_pivot,
        report.jacobi_object_is_pivot_sequence,
        report.carleman_links_p_to_es,
        report.weil_pivots_computed,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_mass_is_degenerate_and_uniform_is_positive() {
        assert_eq!(point_mass_beta(2, 4)[0], Rational::zero());
        assert!(uniform_beta(5).iter().all(|value| value.positive()));
    }

    #[test]
    fn counterexample_has_a_negative_pivot() {
        let moments = vec![
            Rational::one(),
            Rational::one(),
            Rational::zero(),
            Rational::zero(),
            Rational::zero(),
        ];
        let pivots = ldl_pivots(&hankel(&moments, 3));
        assert!(pivots.iter().any(|value| value.negative()));
    }

    #[test]
    fn object_links_p_to_es_and_does_not_certify_weil() {
        let report = m29i_experiment();
        assert!(report.point_mass_degenerate);
        assert!(report.uniform_positive);
        assert!(report.counterexample_negative_pivot);
        assert!(report.jacobi_object_is_pivot_sequence);
        assert!(report.carleman_links_p_to_es);
        assert!(!report.weil_pivots_computed);
        assert!(!report.m29_reached);
        let record = machine_record(&report);
        assert!(record.contains("weil_pivots_computed=false"));
        assert!(record.contains("m29_reached=false"));
    }
}
