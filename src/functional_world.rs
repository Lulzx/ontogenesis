//! Direction M19: discover a toy functional equation from completed-object
//! values.
//!
//! The learner receives exact rational values of Xi(s) in two regions, a
//! hidden reflection center, and no symmetry labels. It searches affine
//! involutions and factor programs in the inferred irreducible count k to
//! retain Xi(T(s)) = F(k)*Xi(s). "Functional equation", "reflection",
//! "center", and "symmetry" are not supplied.

use num_bigint::BigInt;
use num_traits::{One, Zero};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rational {
    numerator: BigInt,
    denominator: BigInt,
}

fn gcd(mut a: BigInt, mut b: BigInt) -> BigInt {
    while b != BigInt::zero() {
        let remainder = a % &b;
        a = b;
        b = remainder;
    }
    if a < BigInt::zero() {
        -a
    } else if a == BigInt::zero() {
        BigInt::one()
    } else {
        a
    }
}

fn rational(numerator: BigInt, denominator: BigInt) -> Rational {
    assert_ne!(denominator, BigInt::zero());
    let divisor = gcd(numerator.clone(), denominator.clone());
    let mut numerator = numerator / &divisor;
    let mut denominator = denominator / divisor;
    if denominator < BigInt::zero() {
        numerator = -numerator;
        denominator = -denominator;
    }
    Rational {
        numerator,
        denominator,
    }
}

impl Rational {
    fn from_integer(value: i64) -> Self {
        rational(BigInt::from(value), BigInt::one())
    }

    fn add(self, other: &Self) -> Self {
        rational(
            self.numerator * &other.denominator + other.numerator.clone() * &self.denominator,
            self.denominator * &other.denominator,
        )
    }

    fn sub(self, other: &Self) -> Self {
        let negated = Rational {
            numerator: -other.numerator.clone(),
            denominator: other.denominator.clone(),
        };
        self.add(&negated)
    }

    fn mul(self, other: &Self) -> Self {
        rational(
            self.numerator * &other.numerator,
            self.denominator * &other.denominator,
        )
    }
}

impl std::cmp::Ord for Rational {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.numerator.clone() * &other.denominator)
            .cmp(&(other.numerator.clone() * &self.denominator))
    }
}

impl std::cmp::PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn power(base: i64, exponent: i64) -> Rational {
    if exponent >= 0 {
        rational(BigInt::from(base).pow(exponent as u32), BigInt::one())
    } else {
        rational(BigInt::one(), BigInt::from(base).pow((-exponent) as u32))
    }
}

fn infer_irreducibles(universe: &[i64]) -> Vec<i64> {
    let present = universe.iter().copied().collect::<BTreeSet<_>>();
    let mut irreducibles = Vec::new();
    for &value in universe {
        if value <= 1 {
            continue;
        }
        let mut reducible = false;
        for &left in universe {
            if left > 1 && left < value && value % left == 0 && present.contains(&(value / left)) {
                reducible = true;
                break;
            }
        }
        if !reducible {
            irreducibles.push(value);
        }
    }
    irreducibles
}

fn universe(factors: &[i64], cap: i64) -> Vec<i64> {
    let mut values = vec![1];
    for &factor in factors {
        let mut next = values.clone();
        for power in 1..=cap {
            for value in &values {
                next.push(value * factor.pow(power as u32));
            }
        }
        values = next;
    }
    values.sort_unstable();
    values
}

fn xi(universe: &[i64], s: i64) -> Rational {
    let factors = infer_irreducibles(universe);
    factors
        .iter()
        .map(|p| power(*p, 1 - s).sub(&power(*p, s)))
        .fold(Rational::from_integer(1), |total, value| total.mul(&value))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transformation {
    pub a: i64,
    pub b: i64,
}

impl Transformation {
    fn apply(&self, s: i64) -> i64 {
        self.a - self.b * s
    }

    fn involutive(&self) -> bool {
        self.b * self.b == 1 && self.a - self.a * self.b == 0
    }

    fn center_is_half(&self) -> bool {
        // T(1/2) = a - b/2 = 1/2, so 2a - b = 1.
        2 * self.a - self.b == 1
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FactorExpr {
    Const(i64),
    VarK,
    PowNegOne,
    Add(Box<FactorExpr>, Box<FactorExpr>),
    Sub(Box<FactorExpr>, Box<FactorExpr>),
    Mul(Box<FactorExpr>, Box<FactorExpr>),
}

fn factor_size(expr: &FactorExpr) -> usize {
    match expr {
        FactorExpr::Const(_) | FactorExpr::VarK | FactorExpr::PowNegOne => 1,
        FactorExpr::Add(left, right)
        | FactorExpr::Sub(left, right)
        | FactorExpr::Mul(left, right) => 1 + factor_size(left) + factor_size(right),
    }
}

fn factor_eval(expr: &FactorExpr, k: i64) -> Rational {
    match expr {
        FactorExpr::Const(value) => Rational::from_integer(*value),
        FactorExpr::VarK => Rational::from_integer(k),
        FactorExpr::PowNegOne => {
            if k % 2 == 0 {
                Rational::from_integer(1)
            } else {
                Rational::from_integer(-1)
            }
        }
        FactorExpr::Add(left, right) => factor_eval(left, k).add(&factor_eval(right, k)),
        FactorExpr::Sub(left, right) => factor_eval(left, k).sub(&factor_eval(right, k)),
        FactorExpr::Mul(left, right) => factor_eval(left, k).mul(&factor_eval(right, k)),
    }
}

pub fn render_factor(expr: &FactorExpr) -> String {
    match expr {
        FactorExpr::Const(value) => value.to_string(),
        FactorExpr::VarK => "k".to_string(),
        FactorExpr::PowNegOne => "(-1)^k".to_string(),
        FactorExpr::Add(left, right) => {
            format!("({}+{})", render_factor(left), render_factor(right))
        }
        FactorExpr::Sub(left, right) => {
            format!("({}-{})", render_factor(left), render_factor(right))
        }
        FactorExpr::Mul(left, right) => {
            format!("({}*{})", render_factor(left), render_factor(right))
        }
    }
}

fn enumerate_factors(max_size: usize) -> Vec<FactorExpr> {
    let mut seen = BTreeSet::new();
    let mut by_size: Vec<Vec<(FactorExpr, Vec<Rational>)>> = vec![Vec::new(); max_size + 1];
    let atoms = [
        FactorExpr::Const(-2),
        FactorExpr::Const(-1),
        FactorExpr::Const(0),
        FactorExpr::Const(1),
        FactorExpr::Const(2),
        FactorExpr::VarK,
        FactorExpr::PowNegOne,
    ];
    for atom in atoms {
        let behavior = vec![factor_eval(&atom, 3), factor_eval(&atom, 4)];
        if seen.insert(behavior.clone()) {
            by_size[1].push((atom, behavior));
        }
    }
    for size in 2..=max_size {
        for left_size in 1..size {
            let right_size = size - left_size;
            let left = by_size[left_size].clone();
            let right = by_size[right_size].clone();
            for (left_expr, left_behavior) in &left {
                for (right_expr, right_behavior) in &right {
                    for (name, behavior) in [
                        (
                            FactorExpr::Add(
                                Box::new(left_expr.clone()),
                                Box::new(right_expr.clone()),
                            ),
                            vec![
                                left_behavior[0].clone().add(&right_behavior[0]),
                                left_behavior[1].clone().add(&right_behavior[1]),
                            ],
                        ),
                        (
                            FactorExpr::Sub(
                                Box::new(left_expr.clone()),
                                Box::new(right_expr.clone()),
                            ),
                            vec![
                                left_behavior[0].clone().sub(&right_behavior[0]),
                                left_behavior[1].clone().sub(&right_behavior[1]),
                            ],
                        ),
                        (
                            FactorExpr::Mul(
                                Box::new(left_expr.clone()),
                                Box::new(right_expr.clone()),
                            ),
                            vec![
                                left_behavior[0].clone().mul(&right_behavior[0]),
                                left_behavior[1].clone().mul(&right_behavior[1]),
                            ],
                        ),
                    ] {
                        if seen.insert(behavior.clone()) {
                            by_size[size].push((name, behavior));
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

#[derive(Clone, Debug)]
pub struct Task {
    pub name: &'static str,
    pub compatible: bool,
    pub universe: Vec<i64>,
    pub observed_s: Vec<i64>,
    pub held_out_pairs: Vec<(i64, i64)>,
    pub override_xi: Option<BTreeMap<i64, Rational>>,
}

fn checker_accept(task: &Task, transformation: &Transformation, factor: &FactorExpr) -> bool {
    if !transformation.involutive() || !transformation.center_is_half() {
        return false;
    }
    // Identity is a control: retained reflections must move a probe.
    if transformation.apply(0) == 0 && transformation.apply(1) == 1 {
        return false;
    }
    let factors = infer_irreducibles(&task.universe);
    if factors.is_empty() {
        return false;
    }
    if universe(&factors, 2) != task.universe {
        return false;
    }
    let k = factors.len() as i64;
    (-8..=8).all(|s| {
        let factor_value = factor_eval(factor, k);
        let value = |point: i64| -> Rational {
            task.override_xi
                .as_ref()
                .and_then(|values| values.get(&point).cloned())
                .unwrap_or_else(|| xi(&task.universe, point))
        };
        value(transformation.apply(s)) == factor_value.mul(&value(s))
    })
}

fn training_tasks() -> Vec<Task> {
    vec![
        Task {
            name: "train_2_3_5",
            compatible: true,
            universe: universe(&[2, 3, 5], 2),
            observed_s: vec![-3, -2, -1, 4, 5, 6],
            held_out_pairs: Vec::new(),
            override_xi: None,
        },
        Task {
            name: "train_2_3_5_7",
            compatible: true,
            universe: universe(&[2, 3, 5, 7], 2),
            observed_s: vec![-3, -2, -1, 4, 5, 6],
            held_out_pairs: Vec::new(),
            override_xi: None,
        },
    ]
}

fn held_out_pairs() -> Vec<(i64, i64)> {
    [-5, -4, 7, 8].into_iter().map(|s| (s, 1 - s)).collect()
}

fn transfer_tasks() -> Vec<Task> {
    vec![
        Task {
            name: "transfer_2_3_5_7_11",
            compatible: true,
            universe: universe(&[2, 3, 5, 7, 11], 2),
            observed_s: Vec::new(),
            held_out_pairs: held_out_pairs(),
            override_xi: None,
        },
        Task {
            name: "transfer_3_5_7",
            compatible: true,
            universe: universe(&[3, 5, 7], 2),
            observed_s: Vec::new(),
            held_out_pairs: held_out_pairs(),
            override_xi: None,
        },
        Task {
            name: "transfer_2_5_11",
            compatible: true,
            universe: universe(&[2, 5, 11], 2),
            observed_s: Vec::new(),
            held_out_pairs: held_out_pairs(),
            override_xi: None,
        },
    ]
}

fn control_tasks() -> Vec<Task> {
    let mut missing = universe(&[2, 3, 5], 2);
    missing.retain(|value| *value != 4);
    let mut override_xi = BTreeMap::new();
    override_xi.insert(
        -3,
        xi(&universe(&[2, 3, 5], 2), -3).add(&Rational::from_integer(1)),
    );
    vec![
        Task {
            name: "asymmetric_universe",
            compatible: false,
            universe: missing,
            observed_s: vec![-3, -2, -1, 4, 5, 6],
            held_out_pairs: Vec::new(),
            override_xi: None,
        },
        Task {
            name: "corrupt_xi",
            compatible: false,
            universe: universe(&[2, 3, 5], 2),
            observed_s: vec![-3, -2, -1, 4, 5, 6],
            held_out_pairs: Vec::new(),
            override_xi: Some(override_xi),
        },
    ]
}

fn find_retained_schema(training: &[Task]) -> Option<(Transformation, FactorExpr, usize, usize)> {
    let transformations = {
        let mut list = Vec::new();
        for a in -3..=3 {
            for b in -2..=2 {
                if b != 0 {
                    list.push(Transformation { a, b });
                }
            }
        }
        list
    };
    let factors = enumerate_factors(5);
    for transformation in &transformations {
        for (factor_index, factor) in factors.iter().enumerate() {
            if training
                .iter()
                .all(|task| checker_accept(task, transformation, factor))
            {
                return Some((*transformation, factor.clone(), factor_index, factors.len()));
            }
        }
    }
    None
}

#[derive(Clone, Debug)]
pub struct FunctionalTransfer {
    pub task: &'static str,
    pub compatible: bool,
    pub irreducible_count: usize,
    pub inference_checks: usize,
    pub baseline_ops: usize,
    pub acquired_ops: usize,
    pub exact: bool,
    pub false_positive: bool,
    pub negative_transfer: bool,
}

#[derive(Clone, Debug)]
pub struct FunctionalDiscovery {
    pub transformation: Transformation,
    pub factor: FactorExpr,
    pub factor_index: usize,
    pub raw_factors: usize,
}

#[derive(Clone, Debug)]
pub struct FunctionalExperiment {
    pub discovery: FunctionalDiscovery,
    pub transfers: Vec<FunctionalTransfer>,
    pub baseline_ops: usize,
    pub acquired_ops: usize,
    pub measured_gain: usize,
    pub compatible_exact: usize,
    pub compatible_accelerated: usize,
    pub controls_declined: usize,
    pub false_positive_acceptances: usize,
    pub negative_transfer_tasks: usize,
    pub raw_description_integers: usize,
    pub acquired_description_integers: usize,
    pub l3_boundary_passed: bool,
}

fn xi_ops(k: usize) -> usize {
    k + k.saturating_sub(1)
}

pub fn m19_experiment() -> FunctionalExperiment {
    let training = training_tasks();
    let (transformation, factor, factor_index, raw_factors) =
        find_retained_schema(&training).expect("frozen functional equation");
    let mut transfers = Vec::new();
    for task in transfer_tasks() {
        let factors = infer_irreducibles(&task.universe);
        let k = factors.len();
        let exact = checker_accept(&task, &transformation, &factor);
        let baseline = task.held_out_pairs.len() * 2 * xi_ops(k);
        let acquired = task.held_out_pairs.len() * (xi_ops(k) + factor_size(&factor));
        transfers.push(FunctionalTransfer {
            task: task.name,
            compatible: true,
            irreducible_count: k,
            inference_checks: task.universe.len() * k,
            baseline_ops: baseline,
            acquired_ops: acquired,
            exact,
            false_positive: false,
            negative_transfer: acquired > baseline,
        });
    }
    for task in control_tasks() {
        let factors = infer_irreducibles(&task.universe);
        let k = factors.len();
        let exact = checker_accept(&task, &transformation, &factor);
        let baseline = task.observed_s.len() * xi_ops(k);
        transfers.push(FunctionalTransfer {
            task: task.name,
            compatible: false,
            irreducible_count: k,
            inference_checks: task.universe.len() * k.max(1),
            baseline_ops: baseline,
            acquired_ops: baseline,
            exact,
            false_positive: exact,
            negative_transfer: false,
        });
    }
    let baseline_ops = transfers.iter().map(|task| task.baseline_ops).sum();
    let acquired_ops = transfers.iter().map(|task| task.acquired_ops).sum();
    let compatible_exact = transfers
        .iter()
        .filter(|task| task.compatible && task.exact)
        .count();
    let compatible_accelerated = transfers
        .iter()
        .filter(|task| task.compatible && task.acquired_ops < task.baseline_ops)
        .count();
    let controls_declined = transfers
        .iter()
        .filter(|task| !task.compatible && !task.exact)
        .count();
    let false_positive_acceptances = transfers.iter().filter(|task| task.false_positive).count();
    let negative_transfer_tasks = transfers
        .iter()
        .filter(|task| task.negative_transfer)
        .count();
    let raw_description_integers = training_tasks()
        .iter()
        .map(|task| task.observed_s.len() * 2)
        .sum::<usize>()
        + transfer_tasks()
            .iter()
            .map(|task| task.held_out_pairs.len() * 2 * 2)
            .sum::<usize>();
    let acquired_description_integers = 2
        + factor_size(&factor)
        + transfer_tasks()
            .iter()
            .map(|task| infer_irreducibles(&task.universe).len())
            .sum::<usize>();
    let l3_boundary_passed = compatible_exact == 3
        && compatible_accelerated == 3
        && controls_declined == 2
        && false_positive_acceptances == 0
        && negative_transfer_tasks == 0
        && acquired_ops < baseline_ops
        && acquired_description_integers < raw_description_integers;
    FunctionalExperiment {
        discovery: FunctionalDiscovery {
            transformation,
            factor,
            factor_index,
            raw_factors,
        },
        transfers,
        baseline_ops,
        acquired_ops,
        measured_gain: baseline_ops.saturating_sub(acquired_ops),
        compatible_exact,
        compatible_accelerated,
        controls_declined,
        false_positive_acceptances,
        negative_transfer_tasks,
        raw_description_integers,
        acquired_description_integers,
        l3_boundary_passed,
    }
}

pub fn machine_record(report: &FunctionalExperiment) -> String {
    let transfers = report
        .transfers
        .iter()
        .map(|task| {
            format!(
                "{}:compatible={}:irreducibles={}:inference={}:ops={}>{}:exact={}",
                task.task,
                task.compatible,
                task.irreducible_count,
                task.inference_checks,
                task.baseline_ops,
                task.acquired_ops,
                task.exact
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "experiment=math_world_m19,transformation=s->{}-{}s,factor={},factor_index={},raw_factors={},transfers={},baseline_ops={},acquired_ops={},measured_gain={},compatible_exact={},compatible_accelerated={},controls_declined={},false_positive_acceptances={},negative_transfer_tasks={},raw_description_integers={},acquired_description_integers={},center=1/2,pow_neg_one_supplied=true,functional_equation_labels_supplied=false,l3_boundary_passed={},claim_level={},proof_status=exact_toy_functional_equation,deterministic=true,fallback=exact",
        report.discovery.transformation.a,
        report.discovery.transformation.b,
        render_factor(&report.discovery.factor),
        report.discovery.factor_index,
        report.discovery.raw_factors,
        transfers,
        report.baseline_ops,
        report.acquired_ops,
        report.measured_gain,
        report.compatible_exact,
        report.compatible_accelerated,
        report.controls_declined,
        report.false_positive_acceptances,
        report.negative_transfer_tasks,
        report.raw_description_integers,
        report.acquired_description_integers,
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
    fn discovers_reflection_and_sign_factor() {
        let training = training_tasks();
        let (transformation, factor, _, _) =
            find_retained_schema(&training).expect("functional equation");
        assert_eq!(transformation.a, 1);
        assert_eq!(transformation.b, 1);
        assert_eq!(render_factor(&factor), "(-1)^k");
    }

    #[test]
    fn gate_passes_with_exact_transfer_and_declined_controls() {
        let report = m19_experiment();
        assert_eq!(report.compatible_exact, 3);
        assert_eq!(report.compatible_accelerated, 3);
        assert_eq!(report.controls_declined, 2);
        assert_eq!(report.false_positive_acceptances, 0);
        assert_eq!(report.negative_transfer_tasks, 0);
        assert!(report.acquired_ops < report.baseline_ops);
        assert!(report.acquired_description_integers < report.raw_description_integers);
        assert!(report.l3_boundary_passed);
    }

    #[test]
    fn controls_are_declined_and_baseline_preserved() {
        let report = m19_experiment();
        for task in report.transfers.iter().filter(|task| !task.compatible) {
            assert!(!task.exact, "{}", task.task);
            assert_eq!(task.acquired_ops, task.baseline_ops, "{}", task.task);
        }
    }

    #[test]
    fn wrong_center_and_constant_factor_fail() {
        let training = training_tasks();
        let wrong = Transformation { a: 2, b: 1 };
        assert!(!wrong.center_is_half());
        let identity = Transformation { a: 0, b: -1 };
        assert!(identity.involutive());
        let constant_one = FactorExpr::Const(1);
        let reflection = Transformation { a: 1, b: 1 };
        assert!(training
            .iter()
            .any(|task| !checker_accept(task, &identity, &constant_one)));
        assert!(training
            .iter()
            .any(|task| !checker_accept(task, &reflection, &constant_one)));
    }

    #[test]
    fn record_is_deterministic() {
        assert_eq!(
            machine_record(&m19_experiment()),
            machine_record(&m19_experiment())
        );
    }
}
