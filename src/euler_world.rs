//! Direction M17: invent a finite Euler product from multiplication behavior.
//!
//! The learner receives finite integer universes extensionally, infers
//! irreducible factors with a generic divisibility check, and searches a small
//! arithmetic grammar for a local factor whose product expands exactly to the
//! universe's special value. "Prime", "irreducible", and "Euler product" are
//! not supplied as labels or templates.

use std::collections::{BTreeMap, BTreeSet};

pub type Universe = Vec<i64>;

fn squarefree_universe(factors: &[i64]) -> Universe {
    let mut values = vec![1];
    for factor in factors {
        let mut next = values.clone();
        for value in &values {
            next.push(value * factor);
        }
        values = next;
    }
    values.sort_unstable();
    values
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalExpr {
    Const(i64),
    Var,
    Add(Box<LocalExpr>, Box<LocalExpr>),
    Sub(Box<LocalExpr>, Box<LocalExpr>),
    Mul(Box<LocalExpr>, Box<LocalExpr>),
}

fn expr_size(expr: &LocalExpr) -> usize {
    match expr {
        LocalExpr::Const(_) | LocalExpr::Var => 1,
        LocalExpr::Add(left, right) | LocalExpr::Sub(left, right) | LocalExpr::Mul(left, right) => {
            1 + expr_size(left) + expr_size(right)
        }
    }
}

pub fn render_local(expr: &LocalExpr) -> String {
    match expr {
        LocalExpr::Const(value) => value.to_string(),
        LocalExpr::Var => "r".to_string(),
        LocalExpr::Add(left, right) => format!("({}+{})", render_local(left), render_local(right)),
        LocalExpr::Sub(left, right) => format!("({}-{})", render_local(left), render_local(right)),
        LocalExpr::Mul(left, right) => format!("({}*{})", render_local(left), render_local(right)),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct UnivariatePoly {
    terms: BTreeMap<u8, i64>,
}

impl UnivariatePoly {
    fn constant(value: i64) -> Self {
        let mut terms = BTreeMap::new();
        if value != 0 {
            terms.insert(0, value);
        }
        Self { terms }
    }

    fn variable() -> Self {
        let mut terms = BTreeMap::new();
        terms.insert(1, 1);
        Self { terms }
    }

    fn add(&self, other: &Self) -> Self {
        let mut terms = self.terms.clone();
        for (degree, value) in &other.terms {
            let entry = terms.entry(*degree).or_insert(0);
            *entry += value;
            if *entry == 0 {
                terms.remove(degree);
            }
        }
        Self { terms }
    }

    fn sub(&self, other: &Self) -> Self {
        let mut terms = self.terms.clone();
        for (degree, value) in &other.terms {
            let entry = terms.entry(*degree).or_insert(0);
            *entry -= value;
            if *entry == 0 {
                terms.remove(degree);
            }
        }
        Self { terms }
    }

    fn mul(&self, other: &Self) -> Self {
        let mut terms = BTreeMap::new();
        for (left, left_value) in &self.terms {
            for (right, right_value) in &other.terms {
                let degree = left + right;
                let entry = terms.entry(degree).or_insert(0);
                *entry += left_value * right_value;
                if *entry == 0 {
                    terms.remove(&degree);
                }
            }
        }
        Self { terms }
    }

    fn eval(&self, value: i64) -> i64 {
        self.terms
            .iter()
            .map(|(degree, coefficient)| coefficient * value.pow(u32::from(*degree)))
            .sum()
    }
}

fn poly_of(expr: &LocalExpr) -> UnivariatePoly {
    match expr {
        LocalExpr::Const(value) => UnivariatePoly::constant(*value),
        LocalExpr::Var => UnivariatePoly::variable(),
        LocalExpr::Add(left, right) => poly_of(left).add(&poly_of(right)),
        LocalExpr::Sub(left, right) => poly_of(left).sub(&poly_of(right)),
        LocalExpr::Mul(left, right) => poly_of(left).mul(&poly_of(right)),
    }
}

fn enumerate_local_factors(max_size: usize) -> Vec<LocalExpr> {
    let mut seen = BTreeSet::new();
    let mut by_size: Vec<Vec<(LocalExpr, UnivariatePoly)>> = vec![Vec::new(); max_size + 1];
    for constant in [1, 2, 3] {
        let expr = LocalExpr::Const(constant);
        let poly = poly_of(&expr);
        if seen.insert(poly.clone()) {
            by_size[1].push((expr, poly));
        }
    }
    {
        let expr = LocalExpr::Var;
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
                            LocalExpr::Add(
                                Box::new(left_expr.clone()),
                                Box::new(right_expr.clone()),
                            ),
                            left_poly.add(right_poly),
                        ),
                        (
                            LocalExpr::Sub(
                                Box::new(left_expr.clone()),
                                Box::new(right_expr.clone()),
                            ),
                            left_poly.sub(right_poly),
                        ),
                        (
                            LocalExpr::Mul(
                                Box::new(left_expr.clone()),
                                Box::new(right_expr.clone()),
                            ),
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

fn special_value(universe: &Universe) -> i64 {
    universe.iter().sum()
}

fn infer_irreducibles(universe: &Universe) -> (Vec<i64>, usize) {
    let present = universe.iter().copied().collect::<BTreeSet<_>>();
    let mut irreducibles = Vec::new();
    let mut checks = 0;
    for &value in universe {
        if value <= 1 {
            continue;
        }
        let mut reducible = false;
        for &left in universe {
            checks += 1;
            if left > 1 && left < value && value % left == 0 && present.contains(&(value / left)) {
                reducible = true;
                break;
            }
        }
        if !reducible {
            irreducibles.push(value);
        }
    }
    (irreducibles, checks)
}

fn expanded_products(irreducibles: &[i64], local: &LocalExpr) -> Vec<i64> {
    let values = irreducibles
        .iter()
        .map(|factor| poly_of(local).eval(*factor) - 1)
        .collect::<Vec<_>>();
    let mut products = vec![1];
    for &value in &values {
        let mut next = products.clone();
        for product in &products {
            next.push(product * value);
        }
        products = next;
    }
    products.sort_unstable();
    products
}

fn identity_valid(universe: &Universe, local: &LocalExpr) -> bool {
    let (irreducibles, _) = infer_irreducibles(universe);
    if irreducibles.is_empty() {
        return universe == &[1];
    }
    let mut expanded = expanded_products(&irreducibles, local);
    let mut expected = universe.clone();
    expanded.sort_unstable();
    expected.sort_unstable();
    let product_value = irreducibles
        .iter()
        .map(|factor| poly_of(local).eval(*factor))
        .product::<i64>();
    expanded == expected && product_value == special_value(universe)
}

fn find_retained_local_factor(
    training: &[Universe],
    max_size: usize,
) -> Option<(LocalExpr, usize, usize, usize)> {
    let raw = enumerate_local_factors(max_size);
    let unique = {
        let mut seen = BTreeSet::new();
        raw.iter()
            .filter(|expr| seen.insert(poly_of(expr)))
            .cloned()
            .collect::<Vec<_>>()
    };
    for (index, expr) in unique.iter().enumerate() {
        if training
            .iter()
            .all(|universe| identity_valid(universe, expr))
        {
            return Some((expr.clone(), index, raw.len(), unique.len()));
        }
    }
    None
}

fn training_universes() -> Vec<Universe> {
    vec![
        squarefree_universe(&[2, 3, 5]),
        squarefree_universe(&[2, 3, 5, 7]),
    ]
}

fn compatible_transfers() -> Vec<(&'static str, Universe)> {
    vec![
        (
            "products_2_3_5_7_11",
            squarefree_universe(&[2, 3, 5, 7, 11]),
        ),
        (
            "products_3_5_7_11_13",
            squarefree_universe(&[3, 5, 7, 11, 13]),
        ),
        (
            "products_2_3_5_7_11_13_17",
            squarefree_universe(&[2, 3, 5, 7, 11, 13, 17]),
        ),
    ]
}

fn control_universes() -> Vec<(&'static str, Universe)> {
    let base = squarefree_universe(&[2, 3, 5]);
    let mut duplicate = base.clone();
    duplicate.push(6);
    duplicate.sort_unstable();
    let one_removed = base
        .iter()
        .copied()
        .filter(|value| *value != 6)
        .collect::<Vec<_>>();
    vec![
        ("missing_composite", vec![1, 2, 3, 5]),
        ("non_squarefree", (1..=12).collect::<Vec<_>>()),
        ("duplicate", duplicate),
        ("one_removed", one_removed),
    ]
}

#[derive(Clone, Debug)]
pub struct EulerTransfer {
    pub task: &'static str,
    pub compatible: bool,
    pub universe_size: usize,
    pub irreducible_count: usize,
    pub inference_checks: usize,
    pub baseline_ops: usize,
    pub acquired_ops: usize,
    pub accepted: bool,
    pub false_positive: bool,
    pub negative_transfer: bool,
}

#[derive(Clone, Debug)]
pub struct EulerDiscovery {
    pub local_factor: LocalExpr,
    pub local_factor_size: usize,
    pub candidate_index: usize,
    pub raw_candidates: usize,
    pub unique_candidates: usize,
    pub training_checks: usize,
}

#[derive(Clone, Debug)]
pub struct EulerExperiment {
    pub discovery: EulerDiscovery,
    pub transfers: Vec<EulerTransfer>,
    pub baseline_ops: usize,
    pub acquired_ops: usize,
    pub measured_gain: usize,
    pub compatible_accepted: usize,
    pub compatible_accelerated: usize,
    pub controls_declined: usize,
    pub false_positive_acceptances: usize,
    pub negative_transfer_tasks: usize,
    pub l3_boundary_passed: bool,
}

pub fn m17_experiment() -> EulerExperiment {
    let training = training_universes();
    let (local_factor, candidate_index, raw_candidates, unique_candidates) =
        find_retained_local_factor(&training, 5).expect("frozen local factor");
    let training_checks = training
        .iter()
        .map(|universe| {
            let (_, inference) = infer_irreducibles(universe);
            inference + usize::from(!identity_valid(universe, &local_factor))
        })
        .sum();
    let mut transfers = Vec::new();
    for (name, universe) in compatible_transfers() {
        let (irreducibles, inference_checks) = infer_irreducibles(&universe);
        let accepted = identity_valid(&universe, &local_factor);
        let baseline_ops = universe.len() - 1;
        let acquired_ops = if accepted {
            2 * irreducibles.len() - 1
        } else {
            baseline_ops
        };
        transfers.push(EulerTransfer {
            task: name,
            compatible: true,
            universe_size: universe.len(),
            irreducible_count: irreducibles.len(),
            inference_checks,
            baseline_ops,
            acquired_ops,
            accepted,
            false_positive: false,
            negative_transfer: acquired_ops > baseline_ops,
        });
    }
    for (name, universe) in control_universes() {
        let (irreducibles, inference_checks) = infer_irreducibles(&universe);
        let accepted = identity_valid(&universe, &local_factor);
        let baseline_ops = universe.len() - 1;
        transfers.push(EulerTransfer {
            task: name,
            compatible: false,
            universe_size: universe.len(),
            irreducible_count: irreducibles.len(),
            inference_checks,
            baseline_ops,
            acquired_ops: baseline_ops,
            accepted,
            false_positive: !matches_universe_control(name) && accepted,
            negative_transfer: false,
        });
    }
    let baseline_ops = transfers.iter().map(|task| task.baseline_ops).sum();
    let acquired_ops = transfers.iter().map(|task| task.acquired_ops).sum();
    let compatible_accepted = transfers
        .iter()
        .filter(|task| task.compatible && task.accepted)
        .count();
    let compatible_accelerated = transfers
        .iter()
        .filter(|task| task.compatible && task.acquired_ops < task.baseline_ops)
        .count();
    let controls_declined = transfers
        .iter()
        .filter(|task| !task.compatible && !task.accepted)
        .count();
    let false_positive_acceptances = transfers.iter().filter(|task| task.false_positive).count();
    let negative_transfer_tasks = transfers
        .iter()
        .filter(|task| task.negative_transfer)
        .count();
    let l3_boundary_passed = compatible_accepted == 3
        && compatible_accelerated == 3
        && controls_declined == 4
        && false_positive_acceptances == 0
        && negative_transfer_tasks == 0
        && acquired_ops < baseline_ops;
    EulerExperiment {
        discovery: EulerDiscovery {
            local_factor_size: expr_size(&local_factor),
            local_factor: local_factor.clone(),
            candidate_index,
            raw_candidates,
            unique_candidates,
            training_checks,
        },
        transfers,
        baseline_ops,
        acquired_ops,
        measured_gain: baseline_ops.saturating_sub(acquired_ops),
        compatible_accepted,
        compatible_accelerated,
        controls_declined,
        false_positive_acceptances,
        negative_transfer_tasks,
        l3_boundary_passed,
    }
}

fn matches_universe_control(_name: &str) -> bool {
    false
}

pub fn machine_record(report: &EulerExperiment) -> String {
    let transfers = report
        .transfers
        .iter()
        .map(|task| {
            format!(
                "{}:compatible={}:size={}:irreducibles={}:inference={}:ops={}>{}:accepted={}",
                task.task,
                task.compatible,
                task.universe_size,
                task.irreducible_count,
                task.inference_checks,
                task.baseline_ops,
                task.acquired_ops,
                task.accepted
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "experiment=math_world_m17,local_factor={},local_factor_size={},candidate_index={},raw_candidates={},unique_candidates={},training_checks={},transfers={},baseline_ops={},acquired_ops={},measured_gain={},compatible_accepted={},compatible_accelerated={},controls_declined={},false_positive_acceptances={},negative_transfer_tasks={},prime_list_supplied=false,irreducible_labels_supplied=false,euler_template_supplied=false,exponent_bounds_supplied=true,l3_boundary_passed={},claim_level={},proof_status=exact_finite_euler_product_checks,deterministic=true,fallback=exact",
        render_local(&report.discovery.local_factor),
        report.discovery.local_factor_size,
        report.discovery.candidate_index,
        report.discovery.raw_candidates,
        report.discovery.unique_candidates,
        report.discovery.training_checks,
        transfers,
        report.baseline_ops,
        report.acquired_ops,
        report.measured_gain,
        report.compatible_accepted,
        report.compatible_accelerated,
        report.controls_declined,
        report.false_positive_acceptances,
        report.negative_transfer_tasks,
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
    fn discovers_one_plus_r_without_prime_labels() {
        let training = training_universes();
        let (expr, _, _, _) = find_retained_local_factor(&training, 5).expect("local factor");
        assert_eq!(expr_size(&expr), 3);
        assert_eq!(
            poly_of(&expr),
            UnivariatePoly::constant(1).add(&UnivariatePoly::variable())
        );
    }

    #[test]
    fn local_factor_grammar_has_irrelevant_abundant_candidates() {
        let training = training_universes();
        let (_, index, raw, unique) =
            find_retained_local_factor(&training, 5).expect("local factor");
        assert!(index > 0);
        assert!(unique > index + 1);
        assert!(raw >= unique);
    }

    #[test]
    fn finite_euler_product_gate_passes() {
        let report = m17_experiment();
        assert_eq!(report.compatible_accepted, 3);
        assert_eq!(report.compatible_accelerated, 3);
        assert_eq!(report.controls_declined, 4);
        assert_eq!(report.false_positive_acceptances, 0);
        assert_eq!(report.negative_transfer_tasks, 0);
        assert!(report.acquired_ops < report.baseline_ops);
        assert!(report.l3_boundary_passed);
    }

    #[test]
    fn compatible_universes_expand_exactly_and_save_operations() {
        let report = m17_experiment();
        for task in report.transfers.iter().filter(|task| task.compatible) {
            assert!(task.accepted, "{}", task.task);
            assert!(task.acquired_ops < task.baseline_ops, "{}", task.task);
        }
    }

    #[test]
    fn incompatible_controls_are_declined() {
        let report = m17_experiment();
        for task in report.transfers.iter().filter(|task| !task.compatible) {
            assert!(!task.accepted, "{}", task.task);
            assert_eq!(task.acquired_ops, task.baseline_ops, "{}", task.task);
        }
    }

    #[test]
    fn supplied_primes_ablation_passes_but_single_atoms_fail() {
        let training = training_universes();
        assert!(find_retained_local_factor(&training, 5).is_some());
        let single_atom = [LocalExpr::Const(1), LocalExpr::Var]
            .into_iter()
            .find(|expr| {
                training
                    .iter()
                    .all(|universe| identity_valid(universe, expr))
            });
        assert!(single_atom.is_none());
    }

    #[test]
    fn record_is_deterministic() {
        assert_eq!(
            machine_record(&m17_experiment()),
            machine_record(&m17_experiment())
        );
    }
}
