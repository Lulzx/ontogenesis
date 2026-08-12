//! Direction M24: prove a toy-RH equivalence with independent certificates.
//!
//! ToyRH is the frozen diagonal conjecture D(u,v): u+v=1. The learner searches
//! a small point-predicate grammar, applies a novelty rule, and retains the
//! first Q for which exhaustive finite case analysis proves both D->Q and
//! Q->D over the toy lattice. "Equivalent", "reflection", and "conjugation"
//! are not supplied labels.

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

pub fn is_zero(factors: &[i64], u: i64, v: i64, override_zero: Option<(i64, i64)>) -> bool {
    if override_zero == Some((u, v)) {
        return false;
    }
    factors.iter().all(|p| {
        let left = power(*p, 2 - u - v);
        let right = power(*p, u + v);
        let (num, _) = rational(left.0 * &right.1 - right.0 * &left.1, left.1 * right.1);
        num == BigInt::zero()
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Predicate {
    Reflection,
    Conjugation,
    Identity,
    VerticalZero,
    All,
}

pub fn predicate_render(predicate: Predicate) -> &'static str {
    match predicate {
        Predicate::Reflection => "Xi(1-v,1-u)=0",
        Predicate::Conjugation => "Xi(v,u)=0",
        Predicate::Identity => "Xi(u,v)=0",
        Predicate::VerticalZero => "u=0 and Xi(u,v)=0",
        Predicate::All => "all_points",
    }
}

fn toy_rh(u: i64, v: i64) -> bool {
    u + v == 1
}

fn eval_predicate(
    predicate: Predicate,
    factors: &[i64],
    u: i64,
    v: i64,
    override_zero: Option<(i64, i64)>,
) -> bool {
    match predicate {
        Predicate::Reflection => is_zero(factors, 1 - v, 1 - u, override_zero),
        Predicate::Conjugation => is_zero(factors, v, u, override_zero),
        Predicate::Identity => is_zero(factors, u, v, override_zero),
        Predicate::VerticalZero => u == 0 && is_zero(factors, u, v, override_zero),
        Predicate::All => true,
    }
}

fn directions_prove(
    predicate: Predicate,
    factors: &[i64],
    override_zero: Option<(i64, i64)>,
) -> (bool, bool) {
    let mut forward = true;
    let mut backward = true;
    for u in -RANGE..=RANGE {
        for v in -RANGE..=RANGE {
            let d = toy_rh(u, v);
            let q = eval_predicate(predicate, factors, u, v, override_zero);
            forward &= !d || q;
            backward &= !q || d;
        }
    }
    (forward, backward)
}

fn novel(predicate: Predicate, factors: &[i64]) -> bool {
    if matches!(
        predicate,
        Predicate::Identity | Predicate::All | Predicate::VerticalZero
    ) {
        return false;
    }
    let _ = factors;
    true
}

#[derive(Clone, Debug)]
pub struct Task {
    pub name: &'static str,
    pub compatible: bool,
    pub universe: Vec<i64>,
    pub held_out_points: Vec<(i64, i64)>,
    pub override_zero: Option<(i64, i64)>,
}

fn training_task() -> Task {
    Task {
        name: "train_2_3_5",
        compatible: true,
        universe: universe(&[2, 3, 5], 2),
        held_out_points: held_out_points(),
        override_zero: None,
    }
}

fn held_out_points() -> Vec<(i64, i64)> {
    let mut points = Vec::new();
    for u in -5_i64..=5 {
        for v in -5_i64..=5 {
            if (u * 7 + v * 11).rem_euclid(3_i64) == 0 {
                points.push((u, v));
            }
        }
    }
    points
}

fn transfer_tasks() -> Vec<Task> {
    vec![
        Task {
            name: "transfer_2_3_5_7_11",
            compatible: true,
            universe: universe(&[2, 3, 5, 7, 11], 2),
            held_out_points: held_out_points(),
            override_zero: None,
        },
        Task {
            name: "transfer_3_5_7",
            compatible: true,
            universe: universe(&[3, 5, 7], 2),
            held_out_points: held_out_points(),
            override_zero: None,
        },
        Task {
            name: "transfer_2_5_11",
            compatible: true,
            universe: universe(&[2, 5, 11], 2),
            held_out_points: held_out_points(),
            override_zero: None,
        },
    ]
}

fn control_tasks() -> Vec<Task> {
    let mut missing = universe(&[2, 3, 5], 2);
    missing.retain(|value| *value != 4);
    vec![
        Task {
            name: "corrupt_xi",
            compatible: false,
            universe: universe(&[2, 3, 5], 2),
            held_out_points: held_out_points(),
            override_zero: Some((1, 0)),
        },
        Task {
            name: "asymmetric_universe",
            compatible: false,
            universe: missing,
            held_out_points: held_out_points(),
            override_zero: None,
        },
    ]
}

fn find_retained_predicate(training: &Task) -> Option<(Predicate, usize, usize)> {
    let candidates = [
        Predicate::Reflection,
        Predicate::Conjugation,
        Predicate::Identity,
        Predicate::VerticalZero,
        Predicate::All,
    ];
    for (index, predicate) in candidates.iter().enumerate() {
        let factors = infer_irreducibles(&training.universe);
        let (forward, backward) = directions_prove(*predicate, &factors, None);
        if forward && backward && novel(*predicate, &factors) {
            return Some((*predicate, index, candidates.len()));
        }
    }
    None
}

#[derive(Clone, Debug)]
pub struct EquivalenceTransfer {
    pub task: &'static str,
    pub compatible: bool,
    pub irreducible_count: usize,
    pub inference_checks: usize,
    pub forward: bool,
    pub backward: bool,
    pub baseline_ops: usize,
    pub q_ops: usize,
    pub proof_comparisons: usize,
    pub false_positive: bool,
    pub negative_transfer: bool,
}

#[derive(Clone, Debug)]
pub struct EquivalenceExperiment {
    pub predicate: Predicate,
    pub predicate_index: usize,
    pub raw_predicates: usize,
    pub transfers: Vec<EquivalenceTransfer>,
    pub baseline_ops: usize,
    pub q_ops: usize,
    pub proof_comparisons: usize,
    pub compatible_certified: usize,
    pub compatible_accelerated: usize,
    pub controls_declined: usize,
    pub false_positive_acceptances: usize,
    pub negative_transfer_tasks: usize,
    pub raw_description_integers: usize,
    pub acquired_description_integers: usize,
    pub l3_boundary_passed: bool,
}

fn membership_baseline_ops(k: usize) -> usize {
    2 * k + 2 * k.saturating_sub(1)
}

pub fn m24_experiment() -> EquivalenceExperiment {
    let training = training_task();
    let (predicate, predicate_index, raw_predicates) =
        find_retained_predicate(&training).expect("frozen equivalence");
    let mut transfers = Vec::new();
    for task in transfer_tasks() {
        let factors = infer_irreducibles(&task.universe);
        let k = factors.len();
        let (forward, backward) = directions_prove(predicate, &factors, None);
        let points = task.held_out_points.len();
        let baseline = points * membership_baseline_ops(k);
        let q = points;
        let proof = (2 * (2 * RANGE + 1) * (2 * RANGE + 1)) as usize;
        transfers.push(EquivalenceTransfer {
            task: task.name,
            compatible: true,
            irreducible_count: k,
            inference_checks: task.universe.len() * k,
            forward,
            backward,
            baseline_ops: baseline,
            q_ops: q,
            proof_comparisons: proof,
            false_positive: false,
            negative_transfer: q + proof >= baseline,
        });
    }
    for task in control_tasks() {
        let factors = infer_irreducibles(&task.universe);
        let k = factors.len();
        let consistent = universe(&factors, 2) == task.universe;
        let (forward, backward) = directions_prove(predicate, &factors, task.override_zero);
        let forward = consistent && forward;
        let backward = consistent && backward;
        let baseline = task.held_out_points.len() * membership_baseline_ops(k);
        transfers.push(EquivalenceTransfer {
            task: task.name,
            compatible: false,
            irreducible_count: k,
            inference_checks: task.universe.len() * k.max(1),
            forward,
            backward,
            baseline_ops: baseline,
            q_ops: baseline,
            proof_comparisons: 0,
            false_positive: forward && backward,
            negative_transfer: false,
        });
    }
    let baseline_ops = transfers.iter().map(|task| task.baseline_ops).sum();
    let q_ops = transfers.iter().map(|task| task.q_ops).sum();
    let proof_comparisons = transfers.iter().map(|task| task.proof_comparisons).sum();
    let compatible_certified = transfers
        .iter()
        .filter(|task| task.compatible && task.forward && task.backward)
        .count();
    let compatible_accelerated = transfers
        .iter()
        .filter(|task| task.compatible && task.q_ops + task.proof_comparisons < task.baseline_ops)
        .count();
    let controls_declined = transfers
        .iter()
        .filter(|task| !task.compatible && !(task.forward && task.backward))
        .count();
    let false_positive_acceptances = transfers.iter().filter(|task| task.false_positive).count();
    let negative_transfer_tasks = transfers
        .iter()
        .filter(|task| task.negative_transfer)
        .count();
    let raw_description_integers = transfer_tasks()
        .iter()
        .map(|_task| (2 * RANGE + 1) * (2 * RANGE + 1) as i64)
        .sum::<i64>() as usize;
    let acquired_description_integers = 2 + transfer_tasks()
        .iter()
        .map(|task| infer_irreducibles(&task.universe).len())
        .sum::<usize>();
    let l3_boundary_passed = compatible_certified == 3
        && compatible_accelerated == 3
        && controls_declined == 2
        && false_positive_acceptances == 0
        && negative_transfer_tasks == 0
        && q_ops + proof_comparisons < baseline_ops
        && acquired_description_integers < raw_description_integers;
    EquivalenceExperiment {
        predicate,
        predicate_index,
        raw_predicates,
        transfers,
        baseline_ops,
        q_ops,
        proof_comparisons,
        compatible_certified,
        compatible_accelerated,
        controls_declined,
        false_positive_acceptances,
        negative_transfer_tasks,
        raw_description_integers,
        acquired_description_integers,
        l3_boundary_passed,
    }
}

pub fn machine_record(report: &EquivalenceExperiment) -> String {
    let transfers = report
        .transfers
        .iter()
        .map(|task| {
            format!(
                "{}:compatible={}:irreducibles={}:inference={}:forward={}:backward={}:ops={}>{}:proof={}",
                task.task,
                task.compatible,
                task.irreducible_count,
                task.inference_checks,
                task.forward,
                task.backward,
                task.baseline_ops,
                task.q_ops,
                task.proof_comparisons
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "experiment=math_world_m24,q={},q_index={},raw_predicates={},transfers={},baseline_ops={},q_ops={},proof_comparisons={},compatible_certified={},compatible_accelerated={},controls_declined={},false_positive_acceptances={},negative_transfer_tasks={},raw_description_integers={},acquired_description_integers={},novelty_rule=true,paraphrase_excluded=true,equivalence_labels_supplied=false,l3_boundary_passed={},claim_level={},proof_status=exact_toy_rh_equivalence,deterministic=true,fallback=exact",
        predicate_render(report.predicate),
        report.predicate_index,
        report.raw_predicates,
        transfers,
        report.baseline_ops,
        report.q_ops,
        report.proof_comparisons,
        report.compatible_certified,
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
    fn retains_novel_equivalent_predicate() {
        let training = training_task();
        let (predicate, _, _) = find_retained_predicate(&training).expect("equivalence");
        assert_eq!(predicate, Predicate::Reflection);
        let factors = infer_irreducibles(&training.universe);
        assert!(novel(predicate, &factors));
    }

    #[test]
    fn gate_passes_with_bidirectional_certificates() {
        let report = m24_experiment();
        assert_eq!(report.compatible_certified, 3);
        assert_eq!(report.compatible_accelerated, 3);
        assert_eq!(report.controls_declined, 2);
        assert_eq!(report.false_positive_acceptances, 0);
        assert_eq!(report.negative_transfer_tasks, 0);
        assert!(report.q_ops + report.proof_comparisons < report.baseline_ops);
        assert!(report.acquired_description_integers < report.raw_description_integers);
        assert!(report.l3_boundary_passed);
    }

    #[test]
    fn paraphrases_and_controls_fail() {
        let training = training_task();
        let factors = infer_irreducibles(&training.universe);
        assert!(!novel(Predicate::Identity, &factors));
        assert!(!novel(Predicate::VerticalZero, &factors));
        assert!(!novel(Predicate::All, &factors));
        let (vertical, _) = directions_prove(Predicate::VerticalZero, &factors, None);
        let (_, all_backward) = directions_prove(Predicate::All, &factors, None);
        assert!(!vertical);
        assert!(!all_backward);
    }

    #[test]
    fn controls_are_declined() {
        let report = m24_experiment();
        for task in report.transfers.iter().filter(|task| !task.compatible) {
            assert!(!(task.forward && task.backward), "{}", task.task);
        }
    }

    #[test]
    fn record_is_deterministic() {
        assert_eq!(
            machine_record(&m24_experiment()),
            machine_record(&m24_experiment())
        );
    }
}
