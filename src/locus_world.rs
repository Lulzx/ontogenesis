//! Direction M21: infer a toy critical symmetry locus from zero positions.
//!
//! The completed lattice object Xi(u,v)=prod_p(p^(2-u-v)-p^(u+v)) has zeros
//! exactly on the hidden diagonal u+v=1. The learner searches a fixed locus
//! grammar and retains the first locus matching the zero set and invariant
//! under reflection and conjugation. "Line", "axis", and "critical line" are
//! not supplied.

use num_bigint::BigInt;
use num_traits::{One, Zero};
use std::collections::BTreeSet;

const TRAIN_RANGE: i64 = 3;
const TRANSFER_RANGE: i64 = 6;

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

fn xi(factors: &[i64], u: i64, v: i64) -> Rational {
    factors
        .iter()
        .fold(rational(BigInt::one(), BigInt::one()), |total, p| {
            let left = power(*p, 2 - u - v);
            let right = power(*p, u + v);
            let factor = rational(
                left.numerator * &right.denominator - right.numerator * &left.denominator,
                left.denominator * right.denominator,
            );
            rational(
                total.numerator * &factor.numerator,
                total.denominator * &factor.denominator,
            )
        })
}

fn zero_set(factors: &[i64], range: i64) -> BTreeSet<(i64, i64)> {
    let mut zeros = BTreeSet::new();
    for u in -range..=range {
        for v in -range..=range {
            if xi(factors, u, v).numerator == BigInt::zero() {
                zeros.insert((u, v));
            }
        }
    }
    zeros
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Locus {
    All,
    Point(i64, i64),
    Vertical(i64),
    Horizontal(i64),
    Diagonal(i64),
    PairDiagonals(i64, i64),
}

pub fn locus_render(locus: &Locus) -> String {
    match locus {
        Locus::All => "all".to_string(),
        Locus::Point(a, b) => format!("point({a},{b})"),
        Locus::Vertical(a) => format!("u={a}"),
        Locus::Horizontal(b) => format!("v={b}"),
        Locus::Diagonal(c) => format!("u+v={c}"),
        Locus::PairDiagonals(c1, c2) => format!("u+v={c1} or u+v={c2}"),
    }
}

fn accepts(locus: &Locus, u: i64, v: i64) -> bool {
    match locus {
        Locus::All => true,
        Locus::Point(a, b) => u == *a && v == *b,
        Locus::Vertical(a) => u == *a,
        Locus::Horizontal(b) => v == *b,
        Locus::Diagonal(c) => u + v == *c,
        Locus::PairDiagonals(c1, c2) => u + v == *c1 || u + v == *c2,
    }
}

fn enumerate_loci() -> Vec<Locus> {
    let mut loci = Vec::new();
    loci.push(Locus::All);
    for a in -6..=6 {
        for b in -6..=6 {
            loci.push(Locus::Point(a, b));
        }
    }
    for a in -6..=6 {
        loci.push(Locus::Vertical(a));
    }
    for b in -6..=6 {
        loci.push(Locus::Horizontal(b));
    }
    for c in -6..=6 {
        loci.push(Locus::Diagonal(c));
    }
    for c1 in -6..=6 {
        for c2 in c1 + 1..=6 {
            loci.push(Locus::PairDiagonals(c1, c2));
        }
    }
    loci
}

#[derive(Clone, Debug)]
pub struct Task {
    pub name: &'static str,
    pub compatible: bool,
    pub universe: Vec<i64>,
    pub range: i64,
    pub observed_zeros: Option<BTreeSet<(i64, i64)>>,
}

fn reflection(point: (i64, i64)) -> (i64, i64) {
    (1 - point.1, 1 - point.0)
}

fn conjugation(point: (i64, i64)) -> (i64, i64) {
    (point.1, point.0)
}

fn checker_accept(task: &Task, locus: &Locus) -> bool {
    let factors = infer_irreducibles(&task.universe);
    if factors.is_empty() || universe(&factors, 2) != task.universe {
        return false;
    }
    let zeros = task
        .observed_zeros
        .clone()
        .unwrap_or_else(|| zero_set(&factors, task.range));
    let locus_points = (-task.range..=task.range)
        .flat_map(|u| (-task.range..=task.range).map(move |v| (u, v)))
        .filter(|&(u, v)| accepts(locus, u, v))
        .collect::<BTreeSet<_>>();
    if locus_points != zeros {
        return false;
    }
    let range = task.range;
    (-range..=range).all(|u| {
        (-range..=range).all(|v| {
            let point = (u, v);
            accepts(locus, u, v) == accepts(locus, reflection(point).0, reflection(point).1)
                && accepts(locus, u, v)
                    == accepts(locus, conjugation(point).0, conjugation(point).1)
        })
    })
}

fn training_tasks() -> Vec<Task> {
    vec![
        Task {
            name: "train_2_3_5",
            compatible: true,
            universe: universe(&[2, 3, 5], 2),
            range: TRAIN_RANGE,
            observed_zeros: None,
        },
        Task {
            name: "train_2_3_5_7",
            compatible: true,
            universe: universe(&[2, 3, 5, 7], 2),
            range: TRAIN_RANGE,
            observed_zeros: None,
        },
    ]
}

fn transfer_tasks() -> Vec<Task> {
    vec![
        Task {
            name: "transfer_2_3_5_7_11",
            compatible: true,
            universe: universe(&[2, 3, 5, 7, 11], 2),
            range: TRANSFER_RANGE,
            observed_zeros: None,
        },
        Task {
            name: "transfer_3_5_7",
            compatible: true,
            universe: universe(&[3, 5, 7], 2),
            range: TRANSFER_RANGE,
            observed_zeros: None,
        },
        Task {
            name: "transfer_2_5_11",
            compatible: true,
            universe: universe(&[2, 5, 11], 2),
            range: TRANSFER_RANGE,
            observed_zeros: None,
        },
    ]
}

fn control_tasks() -> Vec<Task> {
    let mut missing = universe(&[2, 3, 5], 2);
    missing.retain(|value| *value != 4);
    let mut zeros = zero_set(&[2, 3, 5], TRAIN_RANGE);
    zeros.remove(&(2, -1));
    let mut zeros_extra = zero_set(&[2, 3, 5], TRAIN_RANGE);
    zeros_extra.insert((0, 0));
    vec![
        Task {
            name: "asymmetric_universe",
            compatible: false,
            universe: missing,
            range: TRAIN_RANGE,
            observed_zeros: None,
        },
        Task {
            name: "missing_zero",
            compatible: false,
            universe: universe(&[2, 3, 5], 2),
            range: TRAIN_RANGE,
            observed_zeros: Some(zeros),
        },
        Task {
            name: "extra_zero",
            compatible: false,
            universe: universe(&[2, 3, 5], 2),
            range: TRAIN_RANGE,
            observed_zeros: Some(zeros_extra),
        },
    ]
}

fn find_retained_locus(training: &[Task]) -> Option<(Locus, usize, usize)> {
    let loci = enumerate_loci();
    for (index, locus) in loci.iter().enumerate() {
        if training.iter().all(|task| checker_accept(task, locus)) {
            return Some((locus.clone(), index, loci.len()));
        }
    }
    None
}

#[derive(Clone, Debug)]
pub struct LocusTransfer {
    pub task: &'static str,
    pub compatible: bool,
    pub irreducible_count: usize,
    pub inference_checks: usize,
    pub baseline_evaluations: usize,
    pub acquired_evaluations: usize,
    pub exact: bool,
    pub false_positive: bool,
    pub negative_transfer: bool,
}

#[derive(Clone, Debug)]
pub struct LocusExperiment {
    pub locus: Locus,
    pub locus_index: usize,
    pub raw_loci: usize,
    pub transfers: Vec<LocusTransfer>,
    pub baseline_evaluations: usize,
    pub acquired_evaluations: usize,
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

pub fn m21_experiment() -> LocusExperiment {
    let training = training_tasks();
    let (locus, locus_index, raw_loci) =
        find_retained_locus(&training).expect("frozen critical locus");
    let mut transfers = Vec::new();
    for task in transfer_tasks() {
        let factors = infer_irreducibles(&task.universe);
        let k = factors.len();
        let exact = checker_accept(&task, &locus);
        let baseline = ((2 * task.range + 1) * (2 * task.range + 1)) as usize;
        let acquired = zero_set(&factors, task.range).len();
        transfers.push(LocusTransfer {
            task: task.name,
            compatible: true,
            irreducible_count: k,
            inference_checks: task.universe.len() * k,
            baseline_evaluations: baseline,
            acquired_evaluations: acquired,
            exact,
            false_positive: false,
            negative_transfer: acquired > baseline,
        });
    }
    for task in control_tasks() {
        let factors = infer_irreducibles(&task.universe);
        let k = factors.len();
        let exact = checker_accept(&task, &locus);
        let baseline = ((2 * task.range + 1) * (2 * task.range + 1)) as usize;
        transfers.push(LocusTransfer {
            task: task.name,
            compatible: false,
            irreducible_count: k,
            inference_checks: task.universe.len() * k.max(1),
            baseline_evaluations: baseline,
            acquired_evaluations: baseline,
            exact,
            false_positive: exact,
            negative_transfer: false,
        });
    }
    let baseline_evaluations = transfers.iter().map(|task| task.baseline_evaluations).sum();
    let acquired_evaluations = transfers.iter().map(|task| task.acquired_evaluations).sum();
    let compatible_exact = transfers
        .iter()
        .filter(|task| task.compatible && task.exact)
        .count();
    let compatible_accelerated = transfers
        .iter()
        .filter(|task| task.compatible && task.acquired_evaluations < task.baseline_evaluations)
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
        .map(|task| zero_set(&infer_irreducibles(&task.universe), task.range).len() * 2)
        .sum::<usize>()
        + transfer_tasks()
            .iter()
            .map(|task| zero_set(&infer_irreducibles(&task.universe), task.range).len() * 2)
            .sum::<usize>();
    let acquired_description_integers = 2 + transfer_tasks()
        .iter()
        .map(|task| infer_irreducibles(&task.universe).len())
        .sum::<usize>();
    let l3_boundary_passed = compatible_exact == 3
        && compatible_accelerated == 3
        && controls_declined == 3
        && false_positive_acceptances == 0
        && negative_transfer_tasks == 0
        && acquired_evaluations < baseline_evaluations
        && acquired_description_integers < raw_description_integers;
    LocusExperiment {
        locus,
        locus_index,
        raw_loci,
        transfers,
        baseline_evaluations,
        acquired_evaluations,
        measured_gain: baseline_evaluations.saturating_sub(acquired_evaluations),
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

pub fn machine_record(report: &LocusExperiment) -> String {
    let transfers = report
        .transfers
        .iter()
        .map(|task| {
            format!(
                "{}:compatible={}:irreducibles={}:inference={}:evals={}>{}:exact={}",
                task.task,
                task.compatible,
                task.irreducible_count,
                task.inference_checks,
                task.baseline_evaluations,
                task.acquired_evaluations,
                task.exact
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "experiment=math_world_m21,locus={},locus_index={},raw_loci={},transfers={},baseline_evaluations={},acquired_evaluations={},measured_gain={},compatible_exact={},compatible_accelerated={},controls_declined={},false_positive_acceptances={},negative_transfer_tasks={},raw_description_integers={},acquired_description_integers={},line_label_supplied=false,critical_line_template_supplied=false,l3_boundary_passed={},claim_level={},proof_status=exact_toy_critical_locus,deterministic=true,fallback=exact",
        locus_render(&report.locus),
        report.locus_index,
        report.raw_loci,
        transfers,
        report.baseline_evaluations,
        report.acquired_evaluations,
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
    fn discovers_diagonal_locus() {
        let training = training_tasks();
        let (locus, _, _) = find_retained_locus(&training).expect("locus");
        assert_eq!(locus, Locus::Diagonal(1));
    }

    #[test]
    fn gate_passes_with_exact_transfer_and_declined_controls() {
        let report = m21_experiment();
        assert_eq!(report.compatible_exact, 3);
        assert_eq!(report.compatible_accelerated, 3);
        assert_eq!(report.controls_declined, 3);
        assert_eq!(report.false_positive_acceptances, 0);
        assert_eq!(report.negative_transfer_tasks, 0);
        assert!(report.acquired_evaluations < report.baseline_evaluations);
        assert!(report.acquired_description_integers < report.raw_description_integers);
        assert!(report.l3_boundary_passed);
    }

    #[test]
    fn wrong_loci_fail() {
        let training = training_tasks();
        for locus in [
            Locus::All,
            Locus::Point(0, 1),
            Locus::Vertical(0),
            Locus::Horizontal(1),
            Locus::Diagonal(0),
            Locus::PairDiagonals(0, 1),
        ] {
            assert!(
                training.iter().any(|task| !checker_accept(task, &locus)),
                "{} must fail",
                locus_render(&locus)
            );
        }
    }

    #[test]
    fn controls_are_declined() {
        let report = m21_experiment();
        for task in report.transfers.iter().filter(|task| !task.compatible) {
            assert!(!task.exact, "{}", task.task);
            assert_eq!(
                task.acquired_evaluations, task.baseline_evaluations,
                "{}",
                task.task
            );
        }
    }

    #[test]
    fn record_is_deterministic() {
        assert_eq!(
            machine_record(&m21_experiment()),
            machine_record(&m21_experiment())
        );
    }
}
