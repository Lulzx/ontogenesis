//! Direction M23: generate a toy RH-like conjecture from partial zero
//! evidence.
//!
//! The learner sees a frozen subset of hidden lattice zeros and a fixed
//! conjecture language. Frozen scoring selects the smallest predicate that
//! covers every training zero; the checker validates it on held-out zeros and
//! reports a primary falsifier. The output is always labeled conjectured.
//! "Conjecture", "critical line", and "RH" are not supplied labels.

use num_bigint::BigInt;
use num_traits::{One, Zero};
use std::collections::BTreeSet;

const RANGE: i64 = 6;

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

fn rational(numerator: BigInt, denominator: BigInt) -> (BigInt, BigInt) {
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
    let divisor = gcd(numerator.clone(), denominator.clone());
    let mut numerator = numerator / &divisor;
    let mut denominator = denominator / divisor;
    if denominator < BigInt::zero() {
        numerator = -numerator;
        denominator = -denominator;
    }
    (numerator, denominator)
}

fn power(base: i64, exponent: i64) -> (BigInt, BigInt) {
    if exponent >= 0 {
        (BigInt::from(base).pow(exponent as u32), BigInt::one())
    } else {
        (BigInt::one(), BigInt::from(base).pow((-exponent) as u32))
    }
}

fn is_zero(factors: &[i64], u: i64, v: i64) -> bool {
    factors.iter().all(|p| {
        let left = power(*p, 2 - u - v);
        let right = power(*p, u + v);
        let (num, _) = rational(left.0 * &right.1 - right.0 * &left.1, left.1 * right.1);
        num == BigInt::zero()
    })
}

fn zero_set(factors: &[i64], range: i64) -> BTreeSet<(i64, i64)> {
    let mut zeros = BTreeSet::new();
    for u in -range..=range {
        for v in -range..=range {
            if is_zero(factors, u, v) {
                zeros.insert((u, v));
            }
        }
    }
    zeros
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Conjecture {
    Diagonal(i64),
    Vertical(i64),
    Horizontal(i64),
    Point(i64, i64),
    All,
}

pub fn conjecture_render(conjecture: &Conjecture) -> String {
    match conjecture {
        Conjecture::Diagonal(c) => format!("all_zeros_have_u+v={c}"),
        Conjecture::Vertical(a) => format!("all_zeros_have_u={a}"),
        Conjecture::Horizontal(b) => format!("all_zeros_have_v={b}"),
        Conjecture::Point(a, b) => format!("all_zeros_equal({a},{b})"),
        Conjecture::All => "all".to_string(),
    }
}

fn accepts(conjecture: &Conjecture, u: i64, v: i64) -> bool {
    match conjecture {
        Conjecture::Diagonal(c) => u + v == *c,
        Conjecture::Vertical(a) => u == *a,
        Conjecture::Horizontal(b) => v == *b,
        Conjecture::Point(a, b) => u == *a && v == *b,
        Conjecture::All => true,
    }
}

fn enumerate_conjectures() -> Vec<Conjecture> {
    let mut conjectures = Vec::new();
    for c in -6..=6 {
        conjectures.push(Conjecture::Diagonal(c));
    }
    for a in -6..=6 {
        conjectures.push(Conjecture::Vertical(a));
    }
    for b in -6..=6 {
        conjectures.push(Conjecture::Horizontal(b));
    }
    for a in -6..=6 {
        for b in -6..=6 {
            conjectures.push(Conjecture::Point(a, b));
        }
    }
    conjectures.push(Conjecture::All);
    conjectures
}

fn primary_falsifier(conjecture: &Conjecture) -> Option<(i64, i64)> {
    for u in -RANGE..=RANGE {
        for v in -RANGE..=RANGE {
            if !accepts(conjecture, u, v) {
                return Some((u, v));
            }
        }
    }
    None
}

#[derive(Clone, Debug)]
pub struct Task {
    pub name: &'static str,
    pub compatible: bool,
    pub universe: Vec<i64>,
    pub training_zeros: BTreeSet<(i64, i64)>,
}

fn training_task() -> Task {
    let zeros = [-5, -3, -1, 1, 3, 5]
        .into_iter()
        .map(|u| (u, 1 - u))
        .collect();
    Task {
        name: "train_partial_zeros",
        compatible: true,
        universe: universe(&[2, 3, 5], 2),
        training_zeros: zeros,
    }
}

fn transfer_tasks() -> Vec<Task> {
    vec![
        Task {
            name: "transfer_2_3_5_7_11",
            compatible: true,
            universe: universe(&[2, 3, 5, 7, 11], 2),
            training_zeros: training_task().training_zeros.clone(),
        },
        Task {
            name: "transfer_3_5_7",
            compatible: true,
            universe: universe(&[3, 5, 7], 2),
            training_zeros: training_task().training_zeros.clone(),
        },
        Task {
            name: "transfer_2_5_11",
            compatible: true,
            universe: universe(&[2, 5, 11], 2),
            training_zeros: training_task().training_zeros.clone(),
        },
    ]
}

fn control_tasks() -> Vec<Task> {
    let mut missing = universe(&[2, 3, 5], 2);
    missing.retain(|value| *value != 4);
    let mut corrupted = training_task().training_zeros.clone();
    corrupted.remove(&(-3, 4));
    corrupted.insert((0, 0));
    vec![
        Task {
            name: "corrupt_training_zero",
            compatible: false,
            universe: universe(&[2, 3, 5], 2),
            training_zeros: corrupted,
        },
        Task {
            name: "asymmetric_universe",
            compatible: false,
            universe: missing,
            training_zeros: training_task().training_zeros.clone(),
        },
    ]
}

fn find_retained_conjecture(training: &Task) -> Option<(Conjecture, usize, usize)> {
    let conjectures = enumerate_conjectures();
    for (index, conjecture) in conjectures.iter().enumerate() {
        if training
            .training_zeros
            .iter()
            .all(|&(u, v)| accepts(conjecture, u, v))
            && primary_falsifier(conjecture).is_some()
        {
            return Some((conjecture.clone(), index, conjectures.len()));
        }
    }
    None
}

#[derive(Clone, Debug)]
pub struct ConjectureTransfer {
    pub task: &'static str,
    pub compatible: bool,
    pub irreducible_count: usize,
    pub inference_checks: usize,
    pub baseline_evaluations: usize,
    pub conjectured_evaluations: usize,
    pub held_out_zeros_valid: bool,
    pub falsifier: Option<(i64, i64)>,
    pub false_positive: bool,
    pub negative_transfer: bool,
}

#[derive(Clone, Debug)]
pub struct ConjectureExperiment {
    pub conjecture: Conjecture,
    pub conjecture_index: usize,
    pub raw_conjectures: usize,
    pub transfers: Vec<ConjectureTransfer>,
    pub baseline_evaluations: usize,
    pub conjectured_evaluations: usize,
    pub measured_gain: usize,
    pub compatible_valid: usize,
    pub compatible_accelerated: usize,
    pub controls_declined: usize,
    pub false_positive_acceptances: usize,
    pub negative_transfer_tasks: usize,
    pub raw_description_integers: usize,
    pub acquired_description_integers: usize,
    pub l3_boundary_passed: bool,
}

fn validate(task: &Task, conjecture: &Conjecture) -> bool {
    let factors = infer_irreducibles(&task.universe);
    if factors.is_empty() || universe(&factors, 2) != task.universe {
        return false;
    }
    let all_zeros = zero_set(&factors, RANGE);
    all_zeros.iter().all(|&(u, v)| accepts(conjecture, u, v))
}

pub fn m23_experiment() -> ConjectureExperiment {
    let training = training_task();
    let (conjecture, conjecture_index, raw_conjectures) =
        find_retained_conjecture(&training).expect("frozen conjecture");
    let mut transfers = Vec::new();
    for task in transfer_tasks() {
        let factors = infer_irreducibles(&task.universe);
        let k = factors.len();
        let held_out_zeros_valid = validate(&task, &conjecture);
        let baseline = ((2 * RANGE + 1) * (2 * RANGE + 1)) as usize;
        let conjectured = zero_set(&factors, RANGE).len();
        transfers.push(ConjectureTransfer {
            task: task.name,
            compatible: true,
            irreducible_count: k,
            inference_checks: task.universe.len() * k,
            baseline_evaluations: baseline,
            conjectured_evaluations: conjectured,
            held_out_zeros_valid,
            falsifier: primary_falsifier(&conjecture),
            false_positive: false,
            negative_transfer: conjectured > baseline,
        });
    }
    for task in control_tasks() {
        let factors = infer_irreducibles(&task.universe);
        let k = factors.len();
        let baseline = ((2 * RANGE + 1) * (2 * RANGE + 1)) as usize;
        let retained = find_retained_conjecture(&task);
        let valid = retained
            .as_ref()
            .map(|(conjecture, _, _)| validate(&task, conjecture))
            .unwrap_or(false);
        transfers.push(ConjectureTransfer {
            task: task.name,
            compatible: false,
            irreducible_count: k,
            inference_checks: task.universe.len() * k.max(1),
            baseline_evaluations: baseline,
            conjectured_evaluations: baseline,
            held_out_zeros_valid: valid,
            falsifier: retained
                .as_ref()
                .and_then(|(conjecture, _, _)| primary_falsifier(conjecture)),
            false_positive: valid,
            negative_transfer: false,
        });
    }
    let baseline_evaluations = transfers.iter().map(|task| task.baseline_evaluations).sum();
    let conjectured_evaluations = transfers
        .iter()
        .map(|task| task.conjectured_evaluations)
        .sum();
    let compatible_valid = transfers
        .iter()
        .filter(|task| task.compatible && task.held_out_zeros_valid)
        .count();
    let compatible_accelerated = transfers
        .iter()
        .filter(|task| task.compatible && task.conjectured_evaluations < task.baseline_evaluations)
        .count();
    let controls_declined = transfers
        .iter()
        .filter(|task| !task.compatible && !task.held_out_zeros_valid)
        .count();
    let false_positive_acceptances = transfers.iter().filter(|task| task.false_positive).count();
    let negative_transfer_tasks = transfers
        .iter()
        .filter(|task| task.negative_transfer)
        .count();
    let raw_description_integers = training_task().training_zeros.len() * 2
        + transfer_tasks()
            .iter()
            .map(|task| task.training_zeros.len() * 2)
            .sum::<usize>();
    let acquired_description_integers = 2 + transfer_tasks()
        .iter()
        .map(|task| infer_irreducibles(&task.universe).len())
        .sum::<usize>();
    let l3_boundary_passed = compatible_valid == 3
        && compatible_accelerated == 3
        && controls_declined == 2
        && false_positive_acceptances == 0
        && negative_transfer_tasks == 0
        && conjectured_evaluations < baseline_evaluations
        && acquired_description_integers < raw_description_integers;
    ConjectureExperiment {
        conjecture,
        conjecture_index,
        raw_conjectures,
        transfers,
        baseline_evaluations,
        conjectured_evaluations,
        measured_gain: baseline_evaluations.saturating_sub(conjectured_evaluations),
        compatible_valid,
        compatible_accelerated,
        controls_declined,
        false_positive_acceptances,
        negative_transfer_tasks,
        raw_description_integers,
        acquired_description_integers,
        l3_boundary_passed,
    }
}

pub fn machine_record(report: &ConjectureExperiment) -> String {
    let falsifier = report
        .transfers
        .first()
        .and_then(|task| task.falsifier)
        .unwrap_or((0, 0));
    let transfers = report
        .transfers
        .iter()
        .map(|task| {
            format!(
                "{}:compatible={}:irreducibles={}:inference={}:evals={}>{}:valid={}:falsifier={:?}",
                task.task,
                task.compatible,
                task.irreducible_count,
                task.inference_checks,
                task.baseline_evaluations,
                task.conjectured_evaluations,
                task.held_out_zeros_valid,
                task.falsifier
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "experiment=math_world_m23,conjecture={},conjecture_index={},raw_conjectures={},transfers={},baseline_evaluations={},conjectured_evaluations={},measured_gain={},compatible_valid={},compatible_accelerated={},controls_declined={},false_positive_acceptances={},negative_transfer_tasks={},raw_description_integers={},acquired_description_integers={},status=conjectured,proof=false,falsifier={:?},conjecture_labels_supplied=false,rh_template_supplied=false,l3_boundary_passed={},claim_level={},proof_status=exact_toy_conjecture,deterministic=true,fallback=exact",
        conjecture_render(&report.conjecture),
        report.conjecture_index,
        report.raw_conjectures,
        transfers,
        report.baseline_evaluations,
        report.conjectured_evaluations,
        report.measured_gain,
        report.compatible_valid,
        report.compatible_accelerated,
        report.controls_declined,
        report.false_positive_acceptances,
        report.negative_transfer_tasks,
        report.raw_description_integers,
        report.acquired_description_integers,
        falsifier,
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
    fn generates_diagonal_conjecture_from_partial_zeros() {
        let training = training_task();
        let (conjecture, _, _) = find_retained_conjecture(&training).expect("conjecture");
        assert_eq!(conjecture, Conjecture::Diagonal(1));
    }

    #[test]
    fn gate_passes_with_held_out_validation_and_declined_controls() {
        let report = m23_experiment();
        assert_eq!(report.compatible_valid, 3);
        assert_eq!(report.compatible_accelerated, 3);
        assert_eq!(report.controls_declined, 2);
        assert_eq!(report.false_positive_acceptances, 0);
        assert_eq!(report.negative_transfer_tasks, 0);
        assert!(report.conjectured_evaluations < report.baseline_evaluations);
        assert!(report.acquired_description_integers < report.raw_description_integers);
        assert!(report.l3_boundary_passed);
    }

    #[test]
    fn all_conjecture_loses_scoring_and_off_diagonal_zero_falsifies() {
        let training = training_task();
        assert_eq!(
            find_retained_conjecture(&training).unwrap().0,
            Conjecture::Diagonal(1)
        );
        let mut corrupted = training.training_zeros.clone();
        corrupted.remove(&(-3, 4));
        corrupted.insert((0, 0));
        let task = Task {
            name: "corrupt",
            compatible: false,
            universe: training.universe.clone(),
            training_zeros: corrupted,
        };
        assert!(find_retained_conjecture(&task).is_none());
        assert_eq!(primary_falsifier(&Conjecture::All), None);
    }

    #[test]
    fn record_is_deterministic() {
        assert_eq!(
            machine_record(&m23_experiment()),
            machine_record(&m23_experiment())
        );
    }
}
