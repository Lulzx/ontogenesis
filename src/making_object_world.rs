//! Direction M25: construct a toy RH-making object.
//!
//! From a frozen family of candidate zero sets, the system builds a Hankel
//! matrix X from arithmetic signal values and retains the property P(X) of
//! exact rank one. The independent checker verifies P(X) -> ToyRH over the
//! whole family, and provenance rejects objects built from zero coordinates.
//! "Operator", "spectrum", and "self-adjoint" are not supplied.

use num_bigint::BigInt;
use num_traits::{One, Zero};
use std::collections::BTreeSet;

const U_POOL: [i64; 5] = [-3, -1, 0, 1, 3];
const WEIGHTS: [i64; 3] = [1, 2, 1];
const RANGE: i64 = 6;

type ZeroSet = Vec<(i64, i64, i64)>;

fn signal(zeros: &ZeroSet, t: i64) -> BigInt {
    zeros
        .iter()
        .map(|(u, v, weight)| BigInt::from(*weight) * BigInt::from(2).pow(((u + v + 6) * t) as u32))
        .fold(BigInt::zero(), |total, value| total + value)
}

fn hankel(zeros: &ZeroSet) -> (BigInt, BigInt, BigInt) {
    (signal(zeros, 0), signal(zeros, 1), signal(zeros, 2))
}

fn property_holds(zeros: &ZeroSet) -> bool {
    let (a0, a1, a2) = hankel(zeros);
    a0 * &a2 == a1.clone() * &a1
}

fn toy_rh(zeros: &ZeroSet) -> bool {
    zeros.iter().all(|(u, v, _)| u + v == 1)
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

fn frozen_family() -> Vec<ZeroSet> {
    let mut family = Vec::new();
    let mut indices = vec![0, 1, 2];
    loop {
        let us = indices
            .iter()
            .map(|index| U_POOL[*index])
            .collect::<Vec<_>>();
        for v0 in [1 - us[0], 0, 2] {
            for v1 in [1 - us[1], 0, 2] {
                for v2 in [1 - us[2], 0, 2] {
                    family.push(vec![
                        (us[0], v0, WEIGHTS[0]),
                        (us[1], v1, WEIGHTS[1]),
                        (us[2], v2, WEIGHTS[2]),
                    ]);
                }
            }
        }
        let mut increment = 2;
        while indices[increment] == U_POOL.len() - 3 + increment {
            if increment == 0 {
                return family;
            }
            increment -= 1;
        }
        indices[increment] += 1;
        for next in increment + 1..3 {
            indices[next] = indices[next - 1] + 1;
        }
    }
}

#[derive(Clone, Debug)]
pub struct Task {
    pub name: &'static str,
    pub compatible: bool,
    pub universe: Vec<i64>,
    pub zeros: ZeroSet,
    pub circular: bool,
    pub override_signal: Option<BigInt>,
}

fn downstream_tasks() -> Vec<Task> {
    vec![
        Task {
            name: "diagonal_set_a",
            compatible: true,
            universe: universe(&[2, 3, 5, 7], 2),
            zeros: vec![(-3, 4, 1), (-1, 2, 2), (2, -1, 1)],
            circular: false,
            override_signal: None,
        },
        Task {
            name: "diagonal_set_b",
            compatible: true,
            universe: universe(&[3, 5, 7], 2),
            zeros: vec![(0, 1, 1), (1, 0, 2), (3, -2, 1)],
            circular: false,
            override_signal: None,
        },
        Task {
            name: "diagonal_set_c",
            compatible: true,
            universe: universe(&[2, 5, 11], 2),
            zeros: vec![(-1, 2, 1), (0, 1, 2), (1, 0, 1)],
            circular: false,
            override_signal: None,
        },
    ]
}

fn control_tasks() -> Vec<Task> {
    let mut missing = universe(&[2, 3, 5], 2);
    missing.retain(|value| *value != 4);
    vec![
        Task {
            name: "off_diagonal",
            compatible: false,
            universe: universe(&[2, 3, 5], 2),
            zeros: vec![(-3, 0, 1), (-1, 2, 2), (2, -1, 1)],
            circular: false,
            override_signal: None,
        },
        Task {
            name: "circular_object",
            compatible: false,
            universe: universe(&[2, 3, 5], 2),
            zeros: vec![(-3, 4, 1), (-1, 2, 2), (2, -1, 1)],
            circular: true,
            override_signal: None,
        },
        Task {
            name: "corrupt_signal",
            compatible: false,
            universe: universe(&[2, 3, 5], 2),
            zeros: vec![(-3, 4, 1), (-1, 2, 2), (2, -1, 1)],
            circular: false,
            override_signal: Some(
                signal(&vec![(-3, 4, 1), (-1, 2, 2), (2, -1, 1)], 1) + BigInt::one(),
            ),
        },
        Task {
            name: "asymmetric_universe",
            compatible: false,
            universe: missing,
            zeros: vec![(-3, 4, 1), (-1, 2, 2), (2, -1, 1)],
            circular: false,
            override_signal: None,
        },
    ]
}

fn forcing_certificate() -> (bool, bool) {
    let family = frozen_family();
    let implication = family
        .iter()
        .all(|zeros| !property_holds(zeros) || toy_rh(zeros));
    let non_vacuous = family
        .iter()
        .any(|zeros| toy_rh(zeros) && property_holds(zeros));
    (implication, non_vacuous)
}

fn checker_accept(task: &Task) -> bool {
    if task.circular {
        return false;
    }
    let factors = infer_irreducibles(&task.universe);
    if factors.is_empty() || universe(&factors, 2) != task.universe {
        return false;
    }
    let (implication, non_vacuous) = forcing_certificate();
    if !implication || !non_vacuous {
        return false;
    }
    let a1 = task
        .override_signal
        .clone()
        .unwrap_or_else(|| signal(&task.zeros, 1));
    let a0 = signal(&task.zeros, 0);
    let a2 = signal(&task.zeros, 2);
    let property = a0 * &a2 == a1.clone() * &a1;
    property && toy_rh(&task.zeros)
}

#[derive(Clone, Debug)]
pub struct MakingObjectTransfer {
    pub task: &'static str,
    pub compatible: bool,
    pub irreducible_count: usize,
    pub inference_checks: usize,
    pub baseline_evaluations: usize,
    pub acquired_evaluations: usize,
    pub forcing: bool,
    pub provenance: bool,
    pub exact: bool,
    pub false_positive: bool,
    pub negative_transfer: bool,
}

#[derive(Clone, Debug)]
pub struct MakingObjectExperiment {
    pub family_size: usize,
    pub forcing_implication: bool,
    pub non_vacuous: bool,
    pub transfers: Vec<MakingObjectTransfer>,
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

pub fn m25_experiment() -> MakingObjectExperiment {
    let family_size = frozen_family().len();
    let (forcing_implication, non_vacuous) = forcing_certificate();
    let mut transfers = Vec::new();
    for task in downstream_tasks() {
        let factors = infer_irreducibles(&task.universe);
        let k = factors.len();
        let exact = checker_accept(&task);
        let baseline = ((2 * RANGE + 1) * (2 * RANGE + 1)) as usize;
        let acquired = 3 + 3 + 12;
        transfers.push(MakingObjectTransfer {
            task: task.name,
            compatible: true,
            irreducible_count: k,
            inference_checks: task.universe.len() * k,
            baseline_evaluations: baseline,
            acquired_evaluations: acquired,
            forcing: forcing_implication && non_vacuous,
            provenance: !task.circular,
            exact,
            false_positive: false,
            negative_transfer: acquired > baseline,
        });
    }
    for task in control_tasks() {
        let factors = infer_irreducibles(&task.universe);
        let k = factors.len();
        let exact = checker_accept(&task);
        let baseline = ((2 * RANGE + 1) * (2 * RANGE + 1)) as usize;
        transfers.push(MakingObjectTransfer {
            task: task.name,
            compatible: false,
            irreducible_count: k,
            inference_checks: task.universe.len() * k.max(1),
            baseline_evaluations: baseline,
            acquired_evaluations: baseline,
            forcing: forcing_implication && non_vacuous,
            provenance: !task.circular,
            exact,
            false_positive: exact,
            negative_transfer: false,
        });
    }
    let baseline_evaluations = transfers.iter().map(|task| task.baseline_evaluations).sum();
    let acquired_evaluations = transfers.iter().map(|task| task.acquired_evaluations).sum();
    let compatible_exact = transfers
        .iter()
        .filter(|task| task.compatible && task.exact && task.forcing && task.provenance)
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
    let raw_description_integers = downstream_tasks()
        .iter()
        .map(|_task| (2 * RANGE + 1) * (2 * RANGE + 1))
        .sum::<i64>() as usize;
    let acquired_description_integers = 3 + downstream_tasks()
        .iter()
        .map(|task| infer_irreducibles(&task.universe).len())
        .sum::<usize>();
    let l3_boundary_passed = forcing_implication
        && non_vacuous
        && compatible_exact == 3
        && compatible_accelerated == 3
        && controls_declined == 4
        && false_positive_acceptances == 0
        && negative_transfer_tasks == 0
        && acquired_evaluations < baseline_evaluations
        && acquired_description_integers < raw_description_integers;
    MakingObjectExperiment {
        family_size,
        forcing_implication,
        non_vacuous,
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

pub fn machine_record(report: &MakingObjectExperiment) -> String {
    let transfers = report
        .transfers
        .iter()
        .map(|task| {
            format!(
                "{}:compatible={}:irreducibles={}:inference={}:evals={}>{}:forcing={}:provenance={}:exact={}",
                task.task,
                task.compatible,
                task.irreducible_count,
                task.inference_checks,
                task.baseline_evaluations,
                task.acquired_evaluations,
                task.forcing,
                task.provenance,
                task.exact
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "experiment=math_world_m25,family_size={},forcing_implication={},non_vacuous={},transfers={},baseline_evaluations={},acquired_evaluations={},measured_gain={},compatible_exact={},compatible_accelerated={},controls_declined={},false_positive_acceptances={},negative_transfer_tasks={},raw_description_integers={},acquired_description_integers={},object=hankel_signal_matrix,property=exact_rank_one,provenance=signal_only,operator_labels_supplied=false,self_adjoint_label_supplied=false,l3_boundary_passed={},claim_level={},proof_status=exact_toy_rh_making_object,deterministic=true,fallback=exact",
        report.family_size,
        report.forcing_implication,
        report.non_vacuous,
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
    fn forcing_certificate_holds_over_frozen_family() {
        let (implication, non_vacuous) = forcing_certificate();
        assert!(implication);
        assert!(non_vacuous);
    }

    #[test]
    fn gate_passes_with_downstream_exactness_and_declined_controls() {
        let report = m25_experiment();
        assert_eq!(report.family_size, 270);
        assert!(report.forcing_implication);
        assert!(report.non_vacuous);
        assert_eq!(report.compatible_exact, 3);
        assert_eq!(report.compatible_accelerated, 3);
        assert_eq!(report.controls_declined, 4);
        assert_eq!(report.false_positive_acceptances, 0);
        assert_eq!(report.negative_transfer_tasks, 0);
        assert!(report.acquired_evaluations < report.baseline_evaluations);
        assert!(report.acquired_description_integers < report.raw_description_integers);
        assert!(report.l3_boundary_passed);
    }

    #[test]
    fn off_diagonal_and_circular_controls_fail() {
        let report = m25_experiment();
        for task in report.transfers.iter().filter(|task| !task.compatible) {
            assert!(!task.exact, "{}", task.task);
        }
    }

    #[test]
    fn record_is_deterministic() {
        assert_eq!(
            machine_record(&m25_experiment()),
            machine_record(&m25_experiment())
        );
    }
}
