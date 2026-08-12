//! Direction M27: finite-data real-zeta zero-locus conjecture.
//!
//! Roots are recovered from two-dimensional seed grids against the M26
//! completed object. A fixed integer polynomial language selects a locus, but
//! the output remains conjectured: no finite computation proves RH.

use crate::real_zeta_world::completed_value;
use num_complex::Complex64;
use std::cmp::Ordering;
use std::collections::BTreeSet;

const X_SEEDS: [f64; 7] = [-0.25, 0.0, 0.25, 0.5, 0.75, 1.0, 1.25];
const COEFFICIENT_MIN: i8 = -3;
const COEFFICIENT_MAX: i8 = 3;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Locus {
    pub coefficients: [i8; 6], // x^2, xy, y^2, x, y, 1
}

impl Locus {
    pub fn render(self) -> String {
        let [a, b, c, d, e, f] = self.coefficients;
        format!("{a}x^2+{b}xy+{c}y^2+{d}x+{e}y+{f}=0")
    }

    fn value(self, point: Complex64) -> f64 {
        let [a, b, c, d, e, f] = self.coefficients.map(f64::from);
        a * point.re * point.re
            + b * point.re * point.im
            + c * point.im * point.im
            + d * point.re
            + e * point.im
            + f
    }

    fn normalized_residual(self, point: Complex64) -> f64 {
        let scale = self
            .coefficients
            .map(|value| f64::from(value).abs())
            .into_iter()
            .sum::<f64>()
            * (1.0 + point.re.abs() + point.im.abs()).powi(2);
        self.value(point).abs() / scale.max(1.0)
    }

    fn accepts(self, point: Complex64) -> bool {
        self.normalized_residual(point) <= 2e-6
    }
}

#[derive(Clone, Debug)]
pub struct LocusSearch {
    pub condition: &'static str,
    pub selected: Option<Locus>,
    pub candidate_tests: usize,
    pub polynomial_evaluations: usize,
}

#[derive(Clone, Debug)]
pub struct M27Experiment {
    pub training_roots: Vec<Complex64>,
    pub held_out_roots: Vec<Complex64>,
    pub candidate_space: usize,
    pub cold: LocusSearch,
    pub transferred: LocusSearch,
    pub selected_locus: String,
    pub equivalent_selections: bool,
    pub precision_passed: bool,
    pub held_out_passed: bool,
    pub conjugation_passed: bool,
    pub primary_falsifier: Option<Complex64>,
    pub control_results: [bool; 4],
    pub perturbed_control_roots: usize,
    pub controls_rejected: usize,
    pub search_gain: usize,
    pub proof: bool,
    pub claim_level: &'static str,
    pub m27_passed: bool,
}

fn analytic_value(s: Complex64, escalated: bool, perturbed: bool) -> Complex64 {
    let value = completed_value(s, escalated);
    if perturbed {
        value + 0.2 * completed_value(s + 0.11, escalated)
    } else {
        value
    }
}

fn derivative(s: Complex64, escalated: bool, perturbed: bool) -> Complex64 {
    let h = 1e-5;
    (analytic_value(s + h, escalated, perturbed) - analytic_value(s - h, escalated, perturbed))
        / (2.0 * h)
}

fn newton_residual(s: Complex64, escalated: bool, perturbed: bool) -> f64 {
    let d = derivative(s, escalated, perturbed);
    if d.norm() == 0.0 {
        f64::INFINITY
    } else {
        (analytic_value(s, escalated, perturbed) / d).norm()
    }
}

fn converge(
    seed: Complex64,
    min_y: f64,
    max_y: f64,
    escalated: bool,
    perturbed: bool,
) -> Option<Complex64> {
    let mut point = seed;
    for _ in 0..40 {
        let value = analytic_value(point, escalated, perturbed);
        let d = derivative(point, escalated, perturbed);
        if !value.re.is_finite() || !value.im.is_finite() || d.norm() < 1e-30 {
            return None;
        }
        let step = value / d;
        point -= step;
        if !point.re.is_finite() || !point.im.is_finite() || point.re.abs() > 4.0 {
            return None;
        }
        if step.norm() <= if escalated { 2e-10 } else { 2e-8 } {
            break;
        }
    }
    let tolerance = if escalated { 2e-10 } else { 2e-8 };
    (point.im >= min_y
        && point.im <= max_y
        && newton_residual(point, escalated, perturbed) <= tolerance)
        .then_some(point)
}

fn recover_roots(min_y: f64, max_y: f64, escalated: bool, perturbed: bool) -> Vec<Complex64> {
    let mut roots: Vec<Complex64> = Vec::new();
    let mut y = min_y;
    while y <= max_y + 1e-9 {
        for x in X_SEEDS {
            if let Some(root) = converge(Complex64::new(x, y), min_y, max_y, escalated, perturbed) {
                if !roots.iter().any(|old| (*old - root).norm() <= 1e-5) {
                    roots.push(root);
                }
            }
        }
        y += 0.5;
    }
    roots.sort_by(|a, b| a.im.total_cmp(&b.im).then(a.re.total_cmp(&b.re)));
    roots
}

fn gcd(mut a: i8, mut b: i8) -> i8 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

fn normalized(mut coefficients: [i8; 6]) -> Option<Locus> {
    if coefficients[..5].iter().all(|value| *value == 0) {
        return None;
    }
    let divisor = coefficients.iter().copied().fold(0, gcd).max(1);
    for value in &mut coefficients {
        *value /= divisor;
    }
    if coefficients.iter().find(|value| **value != 0).copied()? < 0 {
        for value in &mut coefficients {
            *value = -*value;
        }
    }
    Some(Locus { coefficients })
}

fn loci() -> Vec<Locus> {
    let mut unique = BTreeSet::new();
    for a in COEFFICIENT_MIN..=COEFFICIENT_MAX {
        for b in COEFFICIENT_MIN..=COEFFICIENT_MAX {
            for c in COEFFICIENT_MIN..=COEFFICIENT_MAX {
                for d in COEFFICIENT_MIN..=COEFFICIENT_MAX {
                    for e in COEFFICIENT_MIN..=COEFFICIENT_MAX {
                        for f in COEFFICIENT_MIN..=COEFFICIENT_MAX {
                            if let Some(locus) = normalized([a, b, c, d, e, f]) {
                                unique.insert(locus);
                            }
                        }
                    }
                }
            }
        }
    }
    unique.into_iter().collect()
}

fn degree(locus: Locus) -> usize {
    if locus.coefficients[..3].iter().any(|value| *value != 0) {
        2
    } else {
        1
    }
}

fn complexity(locus: Locus) -> (usize, usize, usize, [i8; 6]) {
    (
        degree(locus),
        locus
            .coefficients
            .iter()
            .filter(|value| **value != 0)
            .count(),
        locus
            .coefficients
            .iter()
            .map(|value| value.unsigned_abs() as usize)
            .sum(),
        locus.coefficients,
    )
}

fn reflection_compatible(locus: Locus) -> bool {
    // Exact coefficient comparison of P(x,y) and +/-P(1-x,y).
    let [a, b, c, d, e, f] = locus.coefficients;
    let reflected = [a, -b, c, -2 * a - d, b + e, a + d + f];
    reflected == locus.coefficients || reflected == locus.coefficients.map(|v| -v)
}

fn transferred_cmp(left: &Locus, right: &Locus) -> Ordering {
    let key = |locus: Locus| {
        (
            usize::from(!reflection_compatible(locus)),
            complexity(locus),
        )
    };
    key(*left).cmp(&key(*right))
}

fn primary_falsifier(locus: Locus) -> Option<Complex64> {
    for x in [-1.0, -0.5, 0.0, 0.5, 1.0, 1.5, 2.0] {
        for y in [0.0, 1.0, 2.0] {
            let point = Complex64::new(x, y);
            if !locus.accepts(point) {
                return Some(point);
            }
        }
    }
    None
}

fn search(condition: &'static str, roots: &[Complex64], transferred: bool) -> LocusSearch {
    let mut candidates = loci();
    if transferred {
        candidates.sort_by(transferred_cmp);
    } else {
        candidates.sort_by_key(|locus| complexity(*locus));
    }
    let mut polynomial_evaluations = 0;
    for (index, locus) in candidates.into_iter().enumerate() {
        let fits = roots.iter().all(|root| {
            polynomial_evaluations += 1;
            locus.accepts(*root)
        });
        if fits && primary_falsifier(locus).is_some() {
            return LocusSearch {
                condition,
                selected: Some(locus),
                candidate_tests: index + 1,
                polynomial_evaluations,
            };
        }
    }
    LocusSearch {
        condition,
        selected: None,
        candidate_tests: loci().len(),
        polynomial_evaluations,
    }
}

fn scalar_equivalent(left: Locus, right: Locus) -> bool {
    left == right
}

pub fn m27_experiment() -> M27Experiment {
    let training_roots = recover_roots(10.0, 30.0, false, false);
    let held_out_roots = recover_roots(30.5, 55.0, false, false);
    let escalated_training = recover_roots(10.0, 30.0, true, false);
    let escalated_held_out = recover_roots(30.5, 55.0, true, false);
    let precision_passed = training_roots.iter().chain(&held_out_roots).all(|root| {
        escalated_training
            .iter()
            .chain(&escalated_held_out)
            .any(|other| (*root - *other).norm() <= 2e-6)
    });
    let candidate_space = loci().len();
    let cold = search("cold", &training_roots, false);
    let transferred = search("transferred", &training_roots, true);
    let selected = transferred.selected.or(cold.selected);
    let equivalent_selections = matches!((cold.selected, transferred.selected), (Some(a), Some(b)) if scalar_equivalent(a, b));
    let held_out_passed =
        selected.is_some_and(|locus| held_out_roots.iter().all(|root| locus.accepts(*root)));
    let conjugation_passed =
        selected.is_some_and(|locus| held_out_roots.iter().all(|root| locus.accepts(root.conj())));
    let primary_falsifier = selected.and_then(primary_falsifier);
    let (control_results, perturbed_control_roots) = selected.map_or(([false; 4], 0), |locus| {
        let shifted_one = training_roots
            .first()
            .is_some_and(|root| !locus.accepts(*root + 0.08));
        let shifted_all = training_roots
            .iter()
            .all(|root| !locus.accepts(*root + 0.08));
        let algebraic_excluded =
            !locus.accepts(Complex64::new(0.0, 0.0)) && !locus.accepts(Complex64::new(1.0, 0.0));
        let corrupted_roots = recover_roots(10.0, 30.0, false, true);
        let corrupted_excluded =
            corrupted_roots.len() >= 3 && corrupted_roots.iter().all(|root| !locus.accepts(*root));
        (
            [
                shifted_one,
                shifted_all,
                algebraic_excluded,
                corrupted_excluded,
            ],
            corrupted_roots.len(),
        )
    });
    let controls_rejected = control_results.into_iter().filter(|passed| *passed).count();
    let search_gain = cold
        .candidate_tests
        .saturating_sub(transferred.candidate_tests);
    let proof = false;
    let m27_passed = training_roots.len() >= 3
        && held_out_roots.len() >= 3
        && precision_passed
        && equivalent_selections
        && held_out_passed
        && conjugation_passed
        && primary_falsifier.is_some()
        && controls_rejected == 4
        && search_gain > 0
        && !proof;
    M27Experiment {
        training_roots,
        held_out_roots,
        candidate_space,
        cold,
        transferred,
        selected_locus: selected.map(Locus::render).unwrap_or_else(|| "none".into()),
        equivalent_selections,
        precision_passed,
        held_out_passed,
        conjugation_passed,
        primary_falsifier,
        control_results,
        perturbed_control_roots,
        controls_rejected,
        search_gain,
        proof,
        claim_level: if m27_passed {
            "L2_invented_feature_in_supplied_meta_ontology"
        } else {
            "L0_finite_fit"
        },
        m27_passed,
    }
}

pub fn machine_record(report: &M27Experiment) -> String {
    format!(
        "M27|training_roots={}|held_out_roots={}|space={}|cold={}|transferred={}|gain={}|locus={}|precision={}|held_out={}|control_results={:?}|perturbed_roots={}|controls={}/4|proof={}|claim={}|pass={}",
        report.training_roots.len(), report.held_out_roots.len(), report.candidate_space,
        report.cold.candidate_tests, report.transferred.candidate_tests, report.search_gain,
        report.selected_locus, report.precision_passed, report.held_out_passed,
        report.control_results, report.perturbed_control_roots, report.controls_rejected,
        report.proof, report.claim_level, report.m27_passed
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locus_space_is_normalized_and_nonvacuous() {
        let space = loci();
        assert!(!space.is_empty());
        assert_eq!(
            space.iter().copied().collect::<BTreeSet<_>>().len(),
            space.len()
        );
        assert!(space
            .iter()
            .all(|locus| primary_falsifier(*locus).is_some()));
    }

    #[test]
    fn recovers_roots_from_two_dimensional_seeds() {
        let roots = recover_roots(10.0, 30.0, false, false);
        assert!(roots.len() >= 3, "{roots:?}");
        assert!(roots
            .iter()
            .all(|root| newton_residual(*root, false, false) <= 2e-8));
    }

    #[test]
    fn m27_passes_as_conjecture_not_proof() {
        let report = m27_experiment();
        assert!(report.m27_passed, "{report:#?}");
        assert!(!report.proof);
        assert_eq!(report.control_results, [true; 4]);
        assert!(report.perturbed_control_roots >= 3);
        assert_eq!(machine_record(&report), machine_record(&m27_experiment()));
    }
}
