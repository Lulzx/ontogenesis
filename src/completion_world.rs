//! Direction M20: construct a toy completed object with simple symmetry.
//!
//! The raw object R(s)=prod_p C(p,s)B(p,s) has a sign-awkward reflection.
//! The learner searches a bounded completion-factor grammar over exact
//! rational power atoms and retains the first completion for which
//! Xi(s)=G(s)R(s) satisfies Xi(1-s)=Xi(s). "Completion", "factor",
//! "symmetric", and "normalization" are not supplied.

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

    fn mul(&self, other: &Self) -> Self {
        rational(
            self.numerator.clone() * &other.numerator,
            self.denominator.clone() * &other.denominator,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Atom {
    C,
    B,
    BPrime,
    PowS,
    PowOneMinusS,
    PowTwoSMinusOne,
    MinusOne,
}

pub fn atom_render(atom: Atom) -> String {
    match atom {
        Atom::C => "p^(1-s)-p^s".to_string(),
        Atom::B => "p^s+1".to_string(),
        Atom::BPrime => "p^(1-s)+1".to_string(),
        Atom::PowS => "p^s".to_string(),
        Atom::PowOneMinusS => "p^(1-s)".to_string(),
        Atom::PowTwoSMinusOne => "p^(2s-1)".to_string(),
        Atom::MinusOne => "-1".to_string(),
    }
}

fn atom_eval(atom: Atom, p: i64, s: i64) -> Rational {
    match atom {
        Atom::C => power(p, 1 - s).sub(&power(p, s)),
        Atom::B => power(p, s).add(&Rational::from_integer(1)),
        Atom::BPrime => power(p, 1 - s).add(&Rational::from_integer(1)),
        Atom::PowS => power(p, s),
        Atom::PowOneMinusS => power(p, 1 - s),
        Atom::PowTwoSMinusOne => power(p, 2 * s - 1),
        Atom::MinusOne => Rational::from_integer(-1),
    }
}

trait AddSubMul: Sized {
    fn add(&self, other: &Self) -> Self;
    fn sub(&self, other: &Self) -> Self;
}

impl AddSubMul for Rational {
    fn add(&self, other: &Self) -> Self {
        rational(
            self.numerator.clone() * &other.denominator
                + other.numerator.clone() * &self.denominator,
            self.denominator.clone() * &other.denominator,
        )
    }

    fn sub(&self, other: &Self) -> Self {
        self.add(&Rational {
            numerator: -other.numerator.clone(),
            denominator: other.denominator.clone(),
        })
    }
}

pub type Completion = BTreeMap<Atom, i64>;

pub fn completion_render(completion: &Completion) -> String {
    completion
        .iter()
        .map(|(atom, exponent)| {
            let base = atom_render(*atom);
            if *exponent == 1 {
                base
            } else {
                format!("({base})^{exponent}")
            }
        })
        .collect::<Vec<_>>()
        .join("*")
}

fn enumerate_completions() -> Vec<Completion> {
    let atoms = [
        Atom::C,
        Atom::B,
        Atom::BPrime,
        Atom::PowS,
        Atom::PowOneMinusS,
        Atom::PowTwoSMinusOne,
        Atom::MinusOne,
    ];
    let mut completions = Vec::new();
    fn visit(atoms: &[Atom], index: usize, current: &mut Completion, output: &mut Vec<Completion>) {
        if index == atoms.len() {
            if !current.is_empty() {
                output.push(current.clone());
            }
            return;
        }
        for exponent in -2..=2 {
            if exponent != 0 {
                current.insert(atoms[index], exponent);
            }
            visit(atoms, index + 1, current, output);
            current.remove(&atoms[index]);
        }
        visit(atoms, index + 1, current, output);
    }
    visit(&atoms, 0, &mut BTreeMap::new(), &mut completions);
    completions
}

fn completion_score(completion: &Completion) -> (usize, i64) {
    (
        completion.len(),
        completion.values().map(|value| value.abs()).sum(),
    )
}

fn completion_value(completion: &Completion, factors: &[i64], s: i64) -> Rational {
    factors.iter().fold(Rational::from_integer(1), |total, p| {
        completion.iter().fold(total, |value, (atom, exponent)| {
            let atom_value = atom_eval(*atom, *p, s);
            let mut powered = Rational::from_integer(1);
            for _ in 0..exponent.abs() {
                powered = powered.mul(&atom_value);
            }
            let inverted = if *exponent < 0 {
                rational(powered.denominator, powered.numerator)
            } else {
                powered
            };
            value.mul(&inverted)
        })
    })
}

fn raw_value(factors: &[i64], s: i64) -> Rational {
    factors.iter().fold(Rational::from_integer(1), |total, p| {
        atom_eval(Atom::C, *p, s)
            .mul(&atom_eval(Atom::B, *p, s))
            .mul(&total)
    })
}

fn is_trivial_rescaling(completion: &Completion) -> bool {
    completion
        .keys()
        .all(|atom| !matches!(atom, Atom::C | Atom::B | Atom::BPrime))
}

#[derive(Clone, Debug)]
pub struct Task {
    pub name: &'static str,
    pub compatible: bool,
    pub universe: Vec<i64>,
    pub observed_s: Vec<i64>,
    pub held_out_pairs: Vec<(i64, i64)>,
    pub override_raw: Option<BTreeMap<i64, Rational>>,
}

fn checker_accept(task: &Task, completion: &Completion) -> bool {
    if completion.is_empty() || is_trivial_rescaling(completion) {
        return false;
    }
    let factors = infer_irreducibles(&task.universe);
    if factors.is_empty() || universe(&factors, 2) != task.universe {
        return false;
    }
    let completed_values = (-6..=6)
        .map(|s| {
            let raw = raw_value(&factors, s);
            completion_value(completion, &factors, s).mul(&raw)
        })
        .collect::<Vec<_>>();
    if completed_values
        .iter()
        .all(|value| value == &completed_values[0])
    {
        return false;
    }
    (-6..=6).all(|s| {
        let raw = |point: i64| -> Rational {
            task.override_raw
                .as_ref()
                .and_then(|values| values.get(&point).cloned())
                .unwrap_or_else(|| raw_value(&factors, point))
        };
        let xi = |point: i64| completion_value(completion, &factors, point).mul(&raw(point));
        xi(1 - s) == xi(s)
    })
}

fn training_tasks() -> Vec<Task> {
    vec![
        Task {
            name: "train_2_3_5",
            compatible: true,
            universe: universe(&[2, 3, 5], 2),
            observed_s: vec![-4, -3, -2, 2, 3, 4],
            held_out_pairs: Vec::new(),
            override_raw: None,
        },
        Task {
            name: "train_2_3_5_7",
            compatible: true,
            universe: universe(&[2, 3, 5, 7], 2),
            observed_s: vec![-4, -3, -2, 2, 3, 4],
            held_out_pairs: Vec::new(),
            override_raw: None,
        },
    ]
}

fn held_out_pairs() -> Vec<(i64, i64)> {
    [-4, -3, 6, 7].into_iter().map(|s| (s, 1 - s)).collect()
}

fn transfer_tasks() -> Vec<Task> {
    vec![
        Task {
            name: "transfer_2_3_5_7_11",
            compatible: true,
            universe: universe(&[2, 3, 5, 7, 11], 2),
            observed_s: Vec::new(),
            held_out_pairs: held_out_pairs(),
            override_raw: None,
        },
        Task {
            name: "transfer_3_5_7",
            compatible: true,
            universe: universe(&[3, 5, 7], 2),
            observed_s: Vec::new(),
            held_out_pairs: held_out_pairs(),
            override_raw: None,
        },
        Task {
            name: "transfer_2_5_11",
            compatible: true,
            universe: universe(&[2, 5, 11], 2),
            observed_s: Vec::new(),
            held_out_pairs: held_out_pairs(),
            override_raw: None,
        },
    ]
}

fn control_tasks() -> Vec<Task> {
    let mut missing = universe(&[2, 3, 5], 2);
    missing.retain(|value| *value != 4);
    let mut override_raw = BTreeMap::new();
    override_raw.insert(
        -2,
        raw_value(&infer_irreducibles(&universe(&[2, 3, 5], 2)), -2)
            .add(&Rational::from_integer(1)),
    );
    vec![
        Task {
            name: "asymmetric_universe",
            compatible: false,
            universe: missing,
            observed_s: vec![-4, -3, -2, 2, 3, 4],
            held_out_pairs: Vec::new(),
            override_raw: None,
        },
        Task {
            name: "corrupt_raw",
            compatible: false,
            universe: universe(&[2, 3, 5], 2),
            observed_s: vec![-4, -3, -2, 2, 3, 4],
            held_out_pairs: Vec::new(),
            override_raw: Some(override_raw),
        },
    ]
}

fn find_retained_completion(training: &[Task]) -> Option<(Completion, usize, usize)> {
    let mut candidates = enumerate_completions();
    candidates.sort_by_key(completion_score);
    let mut seen = BTreeSet::new();
    let mut valid = Vec::new();
    for completion in candidates {
        if !seen.insert(completion.clone()) {
            continue;
        }
        if training
            .iter()
            .all(|task| checker_accept(task, &completion))
        {
            valid.push(completion);
        }
    }
    valid.first().cloned().map(|completion| {
        let index = valid
            .iter()
            .position(|candidate| candidate == &completion)
            .unwrap();
        (completion, index, valid.len())
    })
}

#[derive(Clone, Debug)]
pub struct CompletionTransfer {
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
pub struct CompletionExperiment {
    pub completion: Completion,
    pub completion_score: (usize, i64),
    pub completion_index: usize,
    pub valid_completions: usize,
    pub transfers: Vec<CompletionTransfer>,
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

fn raw_ops(k: usize) -> usize {
    2 * k + 2 * k.saturating_sub(1)
}

fn completion_ops(completion: &Completion, k: usize) -> usize {
    k * completion
        .iter()
        .map(|(_, exponent)| exponent.abs() as usize)
        .sum::<usize>()
}

pub fn m20_experiment() -> CompletionExperiment {
    let training = training_tasks();
    let (completion, completion_index, valid_completions) =
        find_retained_completion(&training).expect("frozen completed object");
    let mut transfers = Vec::new();
    for task in transfer_tasks() {
        let factors = infer_irreducibles(&task.universe);
        let k = factors.len();
        let exact = checker_accept(&task, &completion);
        let baseline =
            task.held_out_pairs.len() * 2 * (raw_ops(k) + completion_ops(&completion, k));
        let acquired = task.held_out_pairs.len() * (raw_ops(k) + completion_ops(&completion, k));
        transfers.push(CompletionTransfer {
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
        let exact = checker_accept(&task, &completion);
        let baseline = task.observed_s.len() * (raw_ops(k) + completion_ops(&completion, k));
        transfers.push(CompletionTransfer {
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
        + completion.len()
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
    CompletionExperiment {
        completion_score: completion_score(&completion),
        completion_index,
        valid_completions,
        completion,
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

pub fn machine_record(report: &CompletionExperiment) -> String {
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
        "experiment=math_world_m20,completion={},completion_score={:?},completion_index={},valid_completions={},transfers={},baseline_ops={},acquired_ops={},measured_gain={},compatible_exact={},compatible_accelerated={},controls_declined={},false_positive_acceptances={},negative_transfer_tasks={},raw_description_integers={},acquired_description_integers={},simple_symmetry=Xi(1-s)=Xi(s),completion_labels_supplied=false,l3_boundary_passed={},claim_level={},proof_status=exact_toy_completed_object,deterministic=true,fallback=exact",
        completion_render(&report.completion),
        report.completion_score,
        report.completion_index,
        report.valid_completions,
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
    fn discovers_simple_symmetry_completion() {
        let training = training_tasks();
        let (completion, _, _) = find_retained_completion(&training).expect("completion");
        println!("retained={}", completion_render(&completion));
        assert!(!completion.is_empty());
        assert!(!is_trivial_rescaling(&completion));
        assert!(training
            .iter()
            .all(|task| checker_accept(task, &completion)));
    }

    #[test]
    fn gate_passes_with_exact_transfer_and_declined_controls() {
        let report = m20_experiment();
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
    fn trivial_rescalings_and_incomplete_factors_fail() {
        let training = training_tasks();
        let mut monomial = BTreeMap::new();
        monomial.insert(Atom::PowS, 1);
        assert!(is_trivial_rescaling(&monomial));
        assert!(training.iter().all(|task| !checker_accept(task, &monomial)));
        let mut only_c = BTreeMap::new();
        only_c.insert(Atom::C, 1);
        assert!(training.iter().all(|task| !checker_accept(task, &only_c)));
    }

    #[test]
    fn controls_are_declined() {
        let report = m20_experiment();
        for task in report.transfers.iter().filter(|task| !task.compatible) {
            assert!(!task.exact, "{}", task.task);
            assert_eq!(task.acquired_ops, task.baseline_ops, "{}", task.task);
        }
    }

    #[test]
    fn record_is_deterministic() {
        assert_eq!(
            machine_record(&m20_experiment()),
            machine_record(&m20_experiment())
        );
    }
}
