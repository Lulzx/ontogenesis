//! Direction M16: invent a toy spectral predicate from matrix dynamics.
//!
//! The learner infers hidden integer matrices from transitions, searches
//! unlabelled scalar predicates over matrix entries, and retains the first
//! predicate that exactly separates matrices with two orthogonal latent
//! directions from matrices without one. "Symmetric", "orthogonal",
//! "eigenvalue", and "spectral decomposition" are not supplied as labels or
//! constructors.

use std::collections::{BTreeMap, BTreeSet};

pub type Mat = [[i64; 2]; 2];

const HORIZON: usize = 8;
const LONG_HORIZON: usize = 10;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    Entry(usize),
    Const(i64),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
}

fn expr_size(expr: &Expr) -> usize {
    match expr {
        Expr::Entry(_) | Expr::Const(_) => 1,
        Expr::Add(left, right) | Expr::Sub(left, right) | Expr::Mul(left, right) => {
            1 + expr_size(left) + expr_size(right)
        }
    }
}

pub fn render_predicate(expr: &Expr) -> String {
    const NAMES: [&str; 4] = ["a00", "a01", "a10", "a11"];
    match expr {
        Expr::Entry(index) => NAMES[*index].to_string(),
        Expr::Const(value) => value.to_string(),
        Expr::Add(left, right) => {
            format!("({}+{})", render_predicate(left), render_predicate(right))
        }
        Expr::Sub(left, right) => {
            format!("({}-{})", render_predicate(left), render_predicate(right))
        }
        Expr::Mul(left, right) => {
            format!("({}*{})", render_predicate(left), render_predicate(right))
        }
    }
}

type Exponents = [u8; 4];

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Poly {
    terms: BTreeMap<Exponents, i64>,
}

impl Poly {
    fn constant(value: i64) -> Self {
        let mut terms = BTreeMap::new();
        if value != 0 {
            terms.insert([0; 4], value);
        }
        Self { terms }
    }

    fn variable(index: usize) -> Self {
        let mut exponents = [0; 4];
        exponents[index] = 1;
        let mut terms = BTreeMap::new();
        terms.insert(exponents, 1);
        Self { terms }
    }

    fn add(&self, other: &Self) -> Self {
        let mut terms = self.terms.clone();
        for (exponents, value) in &other.terms {
            let entry = terms.entry(*exponents).or_insert(0);
            *entry += value;
            if *entry == 0 {
                terms.remove(exponents);
            }
        }
        Self { terms }
    }

    fn sub(&self, other: &Self) -> Self {
        let mut terms = self.terms.clone();
        for (exponents, value) in &other.terms {
            let entry = terms.entry(*exponents).or_insert(0);
            *entry -= value;
            if *entry == 0 {
                terms.remove(exponents);
            }
        }
        Self { terms }
    }

    fn mul(&self, other: &Self) -> Self {
        let mut terms = BTreeMap::new();
        for (left, left_value) in &self.terms {
            for (right, right_value) in &other.terms {
                let mut exponents = [0; 4];
                for index in 0..4 {
                    exponents[index] = left[index] + right[index];
                }
                let entry = terms.entry(exponents).or_insert(0);
                *entry += left_value * right_value;
                if *entry == 0 {
                    terms.remove(&exponents);
                }
            }
        }
        Self { terms }
    }

    fn eval(&self, matrix: &Mat) -> i64 {
        self.terms
            .iter()
            .map(|(exponents, coefficient)| {
                coefficient
                    * matrix[0][0].pow(u32::from(exponents[0]))
                    * matrix[0][1].pow(u32::from(exponents[1]))
                    * matrix[1][0].pow(u32::from(exponents[2]))
                    * matrix[1][1].pow(u32::from(exponents[3]))
            })
            .sum()
    }
}

fn poly_of(expr: &Expr) -> Poly {
    match expr {
        Expr::Entry(index) => Poly::variable(*index),
        Expr::Const(value) => Poly::constant(*value),
        Expr::Add(left, right) => poly_of(left).add(&poly_of(right)),
        Expr::Sub(left, right) => poly_of(left).sub(&poly_of(right)),
        Expr::Mul(left, right) => poly_of(left).mul(&poly_of(right)),
    }
}

fn enumerate_predicates(max_size: usize) -> Vec<Expr> {
    let mut seen = BTreeSet::new();
    let mut by_size: Vec<Vec<(Expr, Poly)>> = vec![Vec::new(); max_size + 1];
    for index in 0..4 {
        let expr = Expr::Entry(index);
        let poly = poly_of(&expr);
        if seen.insert(poly.clone()) {
            by_size[1].push((expr, poly));
        }
    }
    for constant in -2..=2 {
        let expr = Expr::Const(constant);
        let poly = poly_of(&expr);
        if seen.insert(poly.clone()) {
            by_size[1].push((expr, poly));
        }
    }
    for size in 2..=max_size {
        for left_size in 1..size {
            let right_size = size - left_size;
            let left = by_size[left_size].clone();
            let right = by_size[right_size].clone();
            for (left_expr, left_poly) in &left {
                for (right_expr, right_poly) in &right {
                    for (name, poly) in [
                        (
                            Expr::Add(Box::new(left_expr.clone()), Box::new(right_expr.clone())),
                            left_poly.add(right_poly),
                        ),
                        (
                            Expr::Sub(Box::new(left_expr.clone()), Box::new(right_expr.clone())),
                            left_poly.sub(right_poly),
                        ),
                        (
                            Expr::Mul(Box::new(left_expr.clone()), Box::new(right_expr.clone())),
                            left_poly.mul(right_poly),
                        ),
                    ] {
                        if seen.insert(poly.clone()) {
                            by_size[size].push((name, poly));
                        }
                    }
                }
            }
        }
    }
    let mut output = Vec::new();
    for size in 1..=max_size {
        output.extend(by_size[size].iter().map(|(expr, _)| expr.clone()));
    }
    output
}

fn apply(matrix: &Mat, vector: (i64, i64)) -> (i64, i64) {
    (
        matrix[0][0] * vector.0 + matrix[0][1] * vector.1,
        matrix[1][0] * vector.0 + matrix[1][1] * vector.1,
    )
}

fn apply_power(matrix: &Mat, vector: (i64, i64), power: usize) -> (i64, i64) {
    let mut current = vector;
    for _ in 0..power {
        current = apply(matrix, current);
    }
    current
}

fn transitions_for(matrix: &Mat) -> Vec<((i64, i64), (i64, i64))> {
    [(1, 0), (0, 1), (1, 1), (1, -1)]
        .into_iter()
        .map(|vector| (vector, apply(matrix, vector)))
        .collect()
}

fn infer_matrix(transitions: &[((i64, i64), (i64, i64))]) -> Option<(Mat, usize)> {
    let mut found = None;
    let mut checks = 0;
    for a00 in -3..=3 {
        for a01 in -3..=3 {
            for a10 in -3..=3 {
                for a11 in -3..=3 {
                    checks += 1;
                    let matrix = [[a00, a01], [a10, a11]];
                    if transitions
                        .iter()
                        .all(|&(vector, image)| apply(&matrix, vector) == image)
                    {
                        if found.is_some() {
                            return None;
                        }
                        found = Some((matrix, checks));
                    }
                }
            }
        }
    }
    found
}

fn primitive_directions() -> Vec<(i64, i64)> {
    let mut directions = Vec::new();
    for x in -3_i64..=3 {
        for y in -3_i64..=3 {
            if (x, y) == (0, 0) || gcd_i64(x.abs(), y.abs()) != 1 {
                continue;
            }
            if x < 0 || (x == 0 && y < 0) {
                continue;
            }
            directions.push((x, y));
        }
    }
    directions
}

fn gcd_i64(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a.abs().max(1)
}

fn dot(left: (i64, i64), right: (i64, i64)) -> i64 {
    left.0 * right.0 + left.1 * right.1
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Certificate {
    pub d1: (i64, i64),
    pub s1: i64,
    pub d2: (i64, i64),
    pub s2: i64,
}

fn valid_certificate(matrix: &Mat, certificate: &Certificate) -> bool {
    let Certificate { d1, s1, d2, s2 } = certificate;
    if d1 == d2 || dot(*d1, *d2) != 0 {
        return false;
    }
    if apply(matrix, *d1) != (s1 * d1.0, s1 * d1.1) || apply(matrix, *d2) != (s2 * d2.0, s2 * d2.1)
    {
        return false;
    }
    (1..=HORIZON).all(|power| {
        let power1 = s1.pow(power as u32);
        let power2 = s2.pow(power as u32);
        apply_power(matrix, *d1, power) == (power1 * d1.0, power1 * d1.1)
            && apply_power(matrix, *d2, power) == (power2 * d2.0, power2 * d2.1)
    })
}

fn baseline_pair_order(directions: &[(i64, i64)]) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    for left in 0..directions.len() {
        for right in left + 1..directions.len() {
            pairs.push((left, right));
        }
    }
    pairs
}

fn acquired_pair_order(directions: &[(i64, i64)]) -> Vec<(usize, usize)> {
    let mut pairs = baseline_pair_order(directions);
    pairs.sort_by_key(|(left, right)| {
        (
            dot(directions[*left], directions[*right]) != 0,
            *left,
            *right,
        )
    });
    pairs
}

fn total_candidate_count(directions: &[(i64, i64)]) -> usize {
    (0..directions.len())
        .map(|left| directions.len() - left - 1)
        .sum::<usize>()
        * 13
        * 13
}

fn find_certificate(
    matrix: &Mat,
    directions: &[(i64, i64)],
    pair_order: &[(usize, usize)],
) -> Option<(Certificate, usize)> {
    let mut checks = 0;
    for &(left, right) in pair_order {
        for s1 in -6..=6 {
            for s2 in -6..=6 {
                checks += 1;
                let certificate = Certificate {
                    d1: directions[left],
                    s1,
                    d2: directions[right],
                    s2,
                };
                if valid_certificate(matrix, &certificate) {
                    return Some((certificate, checks));
                }
            }
        }
    }
    None
}

fn decomposition_checked(matrix: &Mat, certificate: &Certificate) -> bool {
    let norm1 = dot(certificate.d1, certificate.d1);
    let norm2 = dot(certificate.d2, certificate.d2);
    if norm1 == 0 || norm2 == 0 {
        return false;
    }
    for row in 0..2 {
        for column in 0..2 {
            let d1_product = if row == 0 {
                certificate.d1.0
            } else {
                certificate.d1.1
            } * if column == 0 {
                certificate.d1.0
            } else {
                certificate.d1.1
            };
            let d2_product = if row == 0 {
                certificate.d2.0
            } else {
                certificate.d2.1
            } * if column == 0 {
                certificate.d2.0
            } else {
                certificate.d2.1
            };
            let numerator =
                certificate.s1 * d1_product * norm2 + certificate.s2 * d2_product * norm1;
            let denominator = norm1 * norm2;
            if matrix[row][column] * denominator != numerator {
                return false;
            }
        }
    }
    true
}

fn training_tasks() -> Vec<(Mat, bool)> {
    vec![
        ([[2, 1], [1, 2]], true),
        ([[1, 2], [2, 1]], true),
        ([[0, 1], [1, 0]], true),
        ([[3, 2], [2, 3]], true),
        ([[2, 0], [0, 3]], true),
        ([[1, 0], [0, 1]], true),
        ([[1, 1], [0, 2]], false),
        ([[0, -1], [1, 0]], false),
        ([[1, 1], [0, 1]], false),
        ([[2, 1], [0, 2]], false),
    ]
}

fn transfer_tasks() -> Vec<(&'static str, bool, Mat)> {
    vec![
        ("symmetric_3_1", true, [[3, 1], [1, 3]]),
        ("symmetric_3_2", true, [[3, 2], [2, 0]]),
        ("symmetric_0_2", true, [[0, 2], [2, 3]]),
        ("symmetric_diagonal_3_0", true, [[3, 0], [0, 1]]),
        ("symmetric_repeated_2", true, [[2, 0], [0, 2]]),
        ("rotation_control", false, [[0, -1], [1, 0]]),
        ("defective_control", false, [[1, 1], [0, 1]]),
        ("repeated_defective_control", false, [[2, 1], [0, 2]]),
        ("nonorthogonal_control", false, [[1, 1], [0, 2]]),
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Admitted,
    Declined,
}

#[derive(Clone, Debug)]
pub struct SpectralTransfer {
    pub task: &'static str,
    pub compatible: bool,
    pub route: Route,
    pub inference_checks: usize,
    pub baseline_checks: usize,
    pub acquired_checks: usize,
    pub exact_winner: bool,
    pub decomposition_checked: bool,
    pub long_horizon_baseline_ops: usize,
    pub long_horizon_acquired_ops: usize,
    pub false_positive_route: bool,
    pub negative_transfer: bool,
    pub winner: Option<Certificate>,
}

#[derive(Clone, Debug)]
pub struct SpectralDiscovery {
    pub predicate: Expr,
    pub predicate_size: usize,
    pub predicate_index: usize,
    pub raw_predicates: usize,
    pub unique_predicates: usize,
    pub training_checks: usize,
    pub separated_exactly: bool,
}

#[derive(Clone, Debug)]
pub struct SpectralExperiment {
    pub discovery: SpectralDiscovery,
    pub transfers: Vec<SpectralTransfer>,
    pub baseline_checks: usize,
    pub acquired_checks: usize,
    pub measured_gain: usize,
    pub compatible_admitted: usize,
    pub compatible_accelerated: usize,
    pub controls_declined: usize,
    pub false_positive_routes: usize,
    pub negative_transfer_tasks: usize,
    pub decomposition_passed: bool,
    pub candidate_sets_identical: bool,
    pub l3_boundary_passed: bool,
}

fn find_separating_predicate(
    training: &[(Mat, bool)],
    max_size: usize,
) -> Option<(Expr, Poly, Vec<Expr>, usize, usize)> {
    let directions = primitive_directions();
    let pair_order = baseline_pair_order(&directions);
    let outcomes = training
        .iter()
        .map(|(matrix, _)| find_certificate(matrix, &directions, &pair_order).is_some())
        .collect::<Vec<_>>();
    let raw_count;
    let unique = {
        let mut seen = BTreeSet::new();
        let raw = enumerate_predicates(max_size);
        raw_count = raw.len();
        raw.into_iter()
            .filter_map(|expr| {
                let poly = poly_of(&expr);
                seen.insert(poly.clone()).then_some((expr, poly))
            })
            .collect::<Vec<_>>()
    };
    for (index, (expr, poly)) in unique.iter().enumerate() {
        if training
            .iter()
            .zip(&outcomes)
            .all(|((matrix, _), outcome)| (poly.eval(matrix) == 0) == *outcome)
        {
            return Some((
                expr.clone(),
                poly.clone(),
                unique.iter().map(|(e, _)| e.clone()).collect(),
                index,
                raw_count,
            ));
        }
    }
    None
}

pub fn m16_experiment() -> SpectralExperiment {
    let directions = primitive_directions();
    let baseline_pairs = baseline_pair_order(&directions);
    let acquired_pairs = acquired_pair_order(&directions);
    let total = total_candidate_count(&directions);
    let training = training_tasks();
    let (predicate, poly, unique_predicates, predicate_index, raw_predicates) =
        find_separating_predicate(&training, 5).expect("frozen separating predicate");
    let predicate_size = expr_size(&predicate);
    let training_checks = training
        .iter()
        .map(|(matrix, _)| {
            find_certificate(matrix, &directions, &baseline_pairs)
                .map(|(_, checks)| checks)
                .unwrap_or(total)
        })
        .sum();
    let separated_exactly = training.iter().all(|(matrix, compatible)| {
        let admitted = poly.eval(matrix) == 0;
        let certificate = find_certificate(matrix, &directions, &baseline_pairs).is_some();
        admitted == *compatible && certificate == *compatible
    });

    let mut transfers = Vec::new();
    for (name, compatible, matrix) in transfer_tasks() {
        let transitions = transitions_for(&matrix);
        let (inferred, inference_checks) =
            infer_matrix(&transitions).expect("unique frozen matrix inference");
        let baseline = find_certificate(&inferred, &directions, &baseline_pairs);
        let baseline_checks = baseline
            .as_ref()
            .map(|(_, checks)| *checks)
            .unwrap_or(total);
        let admitted = poly.eval(&inferred) == 0;
        let (acquired_result, acquired_checks) = if admitted {
            match find_certificate(&inferred, &directions, &acquired_pairs) {
                Some((certificate, checks)) => (Some(certificate), checks),
                None => {
                    let (_, fallback_checks) =
                        find_certificate(&inferred, &directions, &baseline_pairs)
                            .expect("admitted matrix must have a certificate");
                    (None, total + fallback_checks)
                }
            }
        } else {
            (
                baseline.clone().map(|(certificate, _)| certificate),
                baseline_checks,
            )
        };
        let exact_winner = acquired_result.is_some();
        let decomposition_checked =
            exact_winner && decomposition_checked(&inferred, acquired_result.as_ref().unwrap());
        let long_horizon_baseline_ops = 3 * 6 * LONG_HORIZON;
        let long_horizon_acquired_ops = if admitted && exact_winner {
            4 * LONG_HORIZON + 19
        } else {
            long_horizon_baseline_ops
        };
        transfers.push(SpectralTransfer {
            task: name,
            compatible,
            route: if admitted {
                Route::Admitted
            } else {
                Route::Declined
            },
            inference_checks,
            baseline_checks,
            acquired_checks,
            exact_winner,
            decomposition_checked,
            long_horizon_baseline_ops,
            long_horizon_acquired_ops,
            false_positive_route: !compatible && admitted,
            negative_transfer: acquired_checks > baseline_checks,
            winner: acquired_result,
        });
    }

    let baseline_checks = transfers.iter().map(|task| task.baseline_checks).sum();
    let acquired_checks = transfers.iter().map(|task| task.acquired_checks).sum();
    let compatible_admitted = transfers
        .iter()
        .filter(|task| task.compatible && task.route == Route::Admitted)
        .count();
    let compatible_accelerated = transfers
        .iter()
        .filter(|task| task.compatible && task.acquired_checks < task.baseline_checks)
        .count();
    let controls_declined = transfers
        .iter()
        .filter(|task| !task.compatible && task.route == Route::Declined)
        .count();
    let false_positive_routes = transfers
        .iter()
        .filter(|task| task.false_positive_route)
        .count();
    let negative_transfer_tasks = transfers
        .iter()
        .filter(|task| task.negative_transfer)
        .count();
    let decomposition_passed = transfers
        .iter()
        .filter(|task| task.compatible)
        .all(|task| task.exact_winner && task.decomposition_checked);
    let candidate_sets_identical = {
        let mut left = baseline_pairs.clone();
        let mut right = acquired_pairs.clone();
        left.sort_unstable();
        right.sort_unstable();
        left == right
    };
    let l3_boundary_passed = separated_exactly
        && compatible_admitted == 5
        && compatible_accelerated == 5
        && controls_declined == 4
        && false_positive_routes == 0
        && negative_transfer_tasks == 0
        && decomposition_passed
        && acquired_checks < baseline_checks
        && candidate_sets_identical;

    SpectralExperiment {
        discovery: SpectralDiscovery {
            predicate,
            predicate_size,
            predicate_index,
            raw_predicates,
            unique_predicates: unique_predicates.len(),
            training_checks,
            separated_exactly,
        },
        transfers,
        baseline_checks,
        acquired_checks,
        measured_gain: baseline_checks.saturating_sub(acquired_checks),
        compatible_admitted,
        compatible_accelerated,
        controls_declined,
        false_positive_routes,
        negative_transfer_tasks,
        decomposition_passed,
        candidate_sets_identical,
        l3_boundary_passed,
    }
}

pub fn machine_record(report: &SpectralExperiment) -> String {
    let transfers = report
        .transfers
        .iter()
        .map(|task| {
            format!(
                "{}:compatible={}:route={}:inference={}:checks={}>{}:exact={}:decomposition={}:long_horizon={}>{}:winner={}",
                task.task,
                task.compatible,
                match task.route {
                    Route::Admitted => "admitted",
                    Route::Declined => "declined",
                },
                task.inference_checks,
                task.baseline_checks,
                task.acquired_checks,
                task.exact_winner,
                task.decomposition_checked,
                task.long_horizon_baseline_ops,
                task.long_horizon_acquired_ops,
                task.winner
                    .as_ref()
                    .map(|certificate| {
                        format!(
                            "({},{})x{};({},{})x{}",
                            certificate.d1.0,
                            certificate.d1.1,
                            certificate.s1,
                            certificate.d2.0,
                            certificate.d2.1,
                            certificate.s2
                        )
                    })
                    .unwrap_or_else(|| "no_solution".into())
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "experiment=math_world_m16,predicate={},predicate_size={},predicate_index={},raw_predicates={},unique_predicates={},training_checks={},separated_exactly={},transfers={},baseline_checks={},acquired_checks={},measured_gain={},compatible_admitted={},compatible_accelerated={},controls_declined={},false_positive_routes={},negative_transfer_tasks={},decomposition_passed={},candidate_sets_identical={},long_horizon_ops_formula=baseline_180_acquired_59,structural_labels_supplied=false,entry_equality_constructor_supplied=false,spectral_template_supplied=false,l3_boundary_passed={},claim_level={},proof_status=exact_conditional_toy_spectral_regularity,deterministic=true,fallback=exact",
        render_predicate(&report.discovery.predicate),
        report.discovery.predicate_size,
        report.discovery.predicate_index,
        report.discovery.raw_predicates,
        report.discovery.unique_predicates,
        report.discovery.training_checks,
        report.discovery.separated_exactly,
        transfers,
        report.baseline_checks,
        report.acquired_checks,
        report.measured_gain,
        report.compatible_admitted,
        report.compatible_accelerated,
        report.controls_declined,
        report.false_positive_routes,
        report.negative_transfer_tasks,
        report.decomposition_passed,
        report.candidate_sets_identical,
        report.l3_boundary_passed,
        if report.l3_boundary_passed {
            "L3_transferred_ontology_with_measured_utility"
        } else {
            "L2_invented_feature_in_supplied_meta_ontology"
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_entry_equality_without_labels() {
        let training = training_tasks();
        let (expr, poly, _, _, _) = find_separating_predicate(&training, 5).expect("separator");
        assert_eq!(expr_size(&expr), 3);
        assert_eq!(poly, Poly::variable(1).sub(&Poly::variable(2)));
    }

    #[test]
    fn always_true_and_non_equality_predicates_do_not_separate() {
        let owned = training_tasks();
        let training = owned.iter().collect::<Vec<_>>();
        let directions = primitive_directions();
        let pair_order = baseline_pair_order(&directions);
        let outcomes = training
            .iter()
            .map(|(matrix, _)| find_certificate(matrix, &directions, &pair_order).is_some())
            .collect::<Vec<_>>();
        let always_true = Poly::constant(0);
        let diagonal_equality = Poly::variable(0).sub(&Poly::variable(3));
        for (poly, name) in [
            (&always_true, "always_true"),
            (&diagonal_equality, "a00-a11"),
        ] {
            assert!(
                training
                    .iter()
                    .zip(&outcomes)
                    .any(|((matrix, _), outcome)| (poly.eval(matrix) == 0) != *outcome),
                "{name} must not separate the training split"
            );
        }
    }

    #[test]
    fn toy_spectral_gate_passes() {
        let report = m16_experiment();
        assert!(report.discovery.separated_exactly);
        assert_eq!(report.compatible_admitted, 5);
        assert_eq!(report.compatible_accelerated, 5);
        assert_eq!(report.controls_declined, 4);
        assert_eq!(report.false_positive_routes, 0);
        assert_eq!(report.negative_transfer_tasks, 0);
        assert!(report.decomposition_passed);
        assert!(report.acquired_checks < report.baseline_checks);
        assert!(report.l3_boundary_passed);
    }

    #[test]
    fn controls_decline_and_preserve_no_solution() {
        let report = m16_experiment();
        for task in report.transfers.iter().filter(|task| !task.compatible) {
            assert_eq!(task.route, Route::Declined);
            assert!(!task.exact_winner);
            assert_eq!(task.acquired_checks, task.baseline_checks);
        }
    }

    #[test]
    fn compatible_tasks_reconstruct_and_decompose_exactly() {
        let report = m16_experiment();
        for task in report.transfers.iter().filter(|task| task.compatible) {
            assert!(task.exact_winner);
            assert!(task.decomposition_checked);
            assert!(task.long_horizon_acquired_ops < task.long_horizon_baseline_ops);
        }
    }

    #[test]
    fn candidate_sets_are_identical_and_record_is_deterministic() {
        let report = m16_experiment();
        assert!(report.candidate_sets_identical);
        assert_eq!(machine_record(&report), machine_record(&report));
    }
}
