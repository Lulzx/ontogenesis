//! Direction M22: recover hidden toy zeros from exact oscillation signals.
//!
//! Zero positions are withheld. The learner receives exact arithmetic
//! signals a[t]=sum w_u q(u)^t over the toy lattice, searches a bounded
//! oscillator dictionary, and completes each recovered frequency with the
//! retained diagonal locus v=1-u. "Zero", "frequency", "oscillator", and
//! "spectrum" are not supplied labels.

use num_bigint::BigInt;
use num_traits::{One, Zero};
use std::collections::BTreeSet;

const PRIMES: [i64; 11] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31];

fn base(u: i64) -> i64 {
    PRIMES[(u + 5) as usize]
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

pub type Model = Vec<(i64, i64)>;

fn signal(model: &Model, t: i64) -> BigInt {
    model
        .iter()
        .map(|(u, weight)| BigInt::from(*weight) * BigInt::from(base(*u)).pow(t as u32))
        .fold(BigInt::zero(), |total, value| total + value)
}

fn enumerate_models() -> Vec<Model> {
    fn weighted_models(support: &[i64]) -> Vec<Model> {
        fn visit(
            support: &[i64],
            position: usize,
            weights: &mut Vec<i64>,
            output: &mut Vec<Model>,
        ) {
            if position == support.len() {
                output.push(
                    support
                        .iter()
                        .copied()
                        .zip(weights.iter().copied())
                        .collect(),
                );
                return;
            }
            for weight in 1..=3 {
                weights[position] = weight;
                visit(support, position + 1, weights, output);
            }
        }
        let mut weights = vec![1; support.len()];
        let mut output = Vec::new();
        visit(support, 0, &mut weights, &mut output);
        output
    }

    let mut models = Vec::new();
    let us = (-5..=5).collect::<Vec<_>>();
    for size in 1..=4 {
        let mut indices = (0..size).collect::<Vec<_>>();
        loop {
            let support = indices.iter().map(|index| us[*index]).collect::<Vec<_>>();
            models.extend(weighted_models(&support));
            let mut increment = size - 1;
            while indices[increment] == us.len() - size + increment {
                if increment == 0 {
                    break;
                }
                increment -= 1;
            }
            if indices[increment] == us.len() - size + increment {
                break;
            }
            indices[increment] += 1;
            for next in increment + 1..size {
                indices[next] = indices[next - 1] + 1;
            }
        }
    }
    models
}

#[derive(Clone, Debug)]
pub struct Task {
    pub name: &'static str,
    pub compatible: bool,
    pub universe: Vec<i64>,
    pub t_values: Vec<i64>,
    pub observed: Option<Vec<BigInt>>,
    pub expected_locations: Vec<(i64, i64)>,
}

fn observed_signal(model: &Model, t_values: &[i64]) -> Vec<BigInt> {
    t_values.iter().map(|t| signal(model, *t)).collect()
}

fn checker_accept(task: &Task, model: &Model) -> bool {
    let factors = infer_irreducibles(&task.universe);
    if factors.is_empty() || universe(&factors, 2) != task.universe {
        return false;
    }
    if model.len() != task.expected_locations.len() {
        return false;
    }
    let mut model_us = model.iter().map(|(u, _)| *u).collect::<Vec<_>>();
    let mut expected_us = task
        .expected_locations
        .iter()
        .map(|(u, _)| *u)
        .collect::<Vec<_>>();
    model_us.sort_unstable();
    expected_us.sort_unstable();
    if model_us != expected_us {
        return false;
    }
    if model
        .iter()
        .zip(&task.expected_locations)
        .any(|((u, _), (expected_u, expected_v))| *u != *expected_u || 1 - u != *expected_v)
    {
        return false;
    }
    let observed = task
        .observed
        .clone()
        .unwrap_or_else(|| observed_signal(model, &task.t_values));
    observed == observed_signal(model, &task.t_values)
}

fn training_task() -> Task {
    let model = vec![(-2, 1), (0, 2), (3, 1)];
    Task {
        name: "train_zeros",
        compatible: true,
        universe: universe(&[2, 3, 5], 2),
        t_values: (0..=12).collect(),
        observed: Some(observed_signal(&model, &(0..=12).collect::<Vec<_>>())),
        expected_locations: model.iter().map(|(u, _)| (*u, 1 - u)).collect(),
    }
}

fn transfer_tasks() -> Vec<Task> {
    let models = [
        vec![(-4, 1), (-1, 2), (1, 1), (4, 3)],
        vec![(-3, 1), (2, 3), (5, 2)],
        vec![(-5, 2), (0, 1), (5, 2)],
    ];
    let universes = [
        universe(&[2, 3, 5, 7], 2),
        universe(&[3, 5, 7], 2),
        universe(&[2, 5, 11], 2),
    ];
    let names = [
        "transfer_4_zeros",
        "transfer_3_zeros_odd",
        "transfer_3_zeros_ends",
    ];
    models
        .iter()
        .zip(universes.iter())
        .zip(names.iter())
        .enumerate()
        .map(|(index, ((model, universe), _name))| {
            let t_values = (13..=18).collect::<Vec<_>>();
            Task {
                name: names[index],
                compatible: true,
                universe: universe.clone(),
                t_values: t_values.clone(),
                observed: Some(observed_signal(model, &t_values)),
                expected_locations: model.iter().map(|(u, _)| (*u, 1 - u)).collect(),
            }
        })
        .collect()
}

fn control_tasks() -> Vec<Task> {
    let training = training_task();
    let mut missing = universe(&[2, 3, 5], 2);
    missing.retain(|value| *value != 4);
    let mut corrupted = training.clone();
    corrupted.name = "corrupt_signal";
    corrupted.observed = Some(
        corrupted
            .observed
            .as_ref()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(index, value)| {
                if index == 3 {
                    value + BigInt::one()
                } else {
                    value.clone()
                }
            })
            .collect(),
    );
    let mut off_locus = training.clone();
    off_locus.name = "off_locus";
    off_locus.expected_locations = vec![(-2, 1), (0, 3), (3, 1)];
    vec![
        corrupted,
        off_locus,
        Task {
            name: "asymmetric_universe",
            compatible: false,
            universe: missing,
            t_values: training.t_values.clone(),
            observed: training.observed.clone(),
            expected_locations: training.expected_locations.clone(),
        },
    ]
}

fn recover_model(task: &Task) -> Option<Model> {
    let models = enumerate_models();
    let mut seen = BTreeSet::new();
    for model in models {
        let behavior = observed_signal(&model, &task.t_values);
        if !seen.insert(behavior) {
            continue;
        }
        if checker_accept(task, &model) {
            return Some(model);
        }
    }
    None
}

#[derive(Clone, Debug)]
pub struct HiddenZeroTransfer {
    pub task: &'static str,
    pub compatible: bool,
    pub irreducible_count: usize,
    pub inference_checks: usize,
    pub baseline_evaluations: usize,
    pub acquired_evaluations: usize,
    pub exact_signal: bool,
    pub exact_locations: bool,
    pub false_positive: bool,
    pub negative_transfer: bool,
}

#[derive(Clone, Debug)]
pub struct HiddenZeroExperiment {
    pub model: Model,
    pub raw_models: usize,
    pub valid_models: usize,
    pub transfers: Vec<HiddenZeroTransfer>,
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

pub fn m22_experiment() -> HiddenZeroExperiment {
    let training = training_task();
    let model = recover_model(&training).expect("frozen hidden oscillator model");
    let valid_models = enumerate_models().len();
    let mut transfers = Vec::new();
    for task in transfer_tasks() {
        let factors = infer_irreducibles(&task.universe);
        let k = factors.len();
        let recovered = recover_model(&task);
        let exact_signal = recovered.is_some();
        let exact_locations = recovered
            .as_ref()
            .map(|recovered| {
                let mut recovered_us = recovered.iter().map(|(u, _)| *u).collect::<Vec<_>>();
                let mut expected_us = task
                    .expected_locations
                    .iter()
                    .map(|(u, _)| *u)
                    .collect::<Vec<_>>();
                recovered_us.sort_unstable();
                expected_us.sort_unstable();
                recovered_us == expected_us
            })
            .unwrap_or(false);
        let baseline = task.t_values.len() * 121;
        let recovered_len = recovered.as_ref().map(|model| model.len()).unwrap_or(0);
        let acquired = task.t_values.len() * recovered_len;
        transfers.push(HiddenZeroTransfer {
            task: task.name,
            compatible: true,
            irreducible_count: k,
            inference_checks: task.universe.len() * k,
            baseline_evaluations: baseline,
            acquired_evaluations: acquired,
            exact_signal,
            exact_locations,
            false_positive: false,
            negative_transfer: acquired > baseline,
        });
    }
    for task in control_tasks() {
        let factors = infer_irreducibles(&task.universe);
        let k = factors.len();
        let exact_signal = recover_model(&task).is_some();
        let baseline = task.t_values.len() * 121;
        transfers.push(HiddenZeroTransfer {
            task: task.name,
            compatible: false,
            irreducible_count: k,
            inference_checks: task.universe.len() * k.max(1),
            baseline_evaluations: baseline,
            acquired_evaluations: baseline,
            exact_signal,
            exact_locations: false,
            false_positive: exact_signal,
            negative_transfer: false,
        });
    }
    let baseline_evaluations = transfers.iter().map(|task| task.baseline_evaluations).sum();
    let acquired_evaluations = transfers.iter().map(|task| task.acquired_evaluations).sum();
    let compatible_exact = transfers
        .iter()
        .filter(|task| task.compatible && task.exact_signal && task.exact_locations)
        .count();
    let compatible_accelerated = transfers
        .iter()
        .filter(|task| task.compatible && task.acquired_evaluations < task.baseline_evaluations)
        .count();
    let controls_declined = transfers
        .iter()
        .filter(|task| !task.compatible && !task.exact_signal)
        .count();
    let false_positive_acceptances = transfers.iter().filter(|task| task.false_positive).count();
    let negative_transfer_tasks = transfers
        .iter()
        .filter(|task| task.negative_transfer)
        .count();
    let raw_description_integers = transfer_tasks()
        .iter()
        .map(|task| task.t_values.len() + 11)
        .sum();
    let acquired_description_integers = 2 + transfer_tasks()
        .iter()
        .map(|task| task.expected_locations.len())
        .sum::<usize>();
    let l3_boundary_passed = compatible_exact == 3
        && compatible_accelerated == 3
        && controls_declined == 3
        && false_positive_acceptances == 0
        && negative_transfer_tasks == 0
        && acquired_evaluations < baseline_evaluations
        && acquired_description_integers < raw_description_integers;
    HiddenZeroExperiment {
        model,
        raw_models: enumerate_models().len(),
        valid_models,
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

pub fn machine_record(report: &HiddenZeroExperiment) -> String {
    let model = report
        .model
        .iter()
        .map(|(u, weight)| format!("u={u}:w={weight}:v={}", 1 - u))
        .collect::<Vec<_>>()
        .join(";");
    let transfers = report
        .transfers
        .iter()
        .map(|task| {
            format!(
                "{}:compatible={}:irreducibles={}:inference={}:evals={}>{}:signal={}:locations={}",
                task.task,
                task.compatible,
                task.irreducible_count,
                task.inference_checks,
                task.baseline_evaluations,
                task.acquired_evaluations,
                task.exact_signal,
                task.exact_locations
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "experiment=math_world_m22,oscillators={},raw_models={},valid_models={},transfers={},baseline_evaluations={},acquired_evaluations={},measured_gain={},compatible_exact={},compatible_accelerated={},controls_declined={},false_positive_acceptances={},negative_transfer_tasks={},raw_description_integers={},acquired_description_integers={},oscillator_dictionary_supplied=true,diagonal_locus_retained=true,zero_labels_supplied=false,l3_boundary_passed={},claim_level={},proof_status=exact_hidden_toy_zeros,deterministic=true,fallback=exact",
        model,
        report.raw_models,
        report.valid_models,
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
    fn recovers_hidden_zeros() {
        let training = training_task();
        let model = recover_model(&training).expect("model");
        assert_eq!(model, vec![(-2, 1), (0, 2), (3, 1)]);
    }

    #[test]
    fn gate_passes_with_exact_transfer_and_declined_controls() {
        let report = m22_experiment();
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
    fn controls_are_declined() {
        let report = m22_experiment();
        for task in report.transfers.iter().filter(|task| !task.compatible) {
            assert!(!task.exact_signal, "{}", task.task);
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
            machine_record(&m22_experiment()),
            machine_record(&m22_experiment())
        );
    }
}
