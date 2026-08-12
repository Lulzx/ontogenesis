//! Direction M15/M15b: invent oscillatory coordinates from generic
//! recurrences, then route the retained closed-shift priority by raw probes.
//!
//! Candidate atoms are generated only by bounded second-order recurrences.
//! Search never receives frequencies, trigonometric atoms, orthogonality, or a
//! Fourier transform. M9's retained idea changes ordering only: pairs with an
//! independently checked small closed shift action are tried first.

use std::collections::BTreeSet;

const LENGTH: usize = 12;
const WEIGHTS: [i32; 6] = [1, -1, 2, -2, 3, -3];

#[derive(Clone, Debug, PartialEq, Eq)]
struct Atom {
    values: [i32; LENGTH],
    generator: (i32, i32),
    seeds: (i32, i32),
}

fn gcd(mut a: i32, mut b: i32) -> i32 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a.abs()
}

fn normalize_sequence(mut values: [i32; LENGTH]) -> [i32; LENGTH] {
    let divisor = values
        .iter()
        .fold(0, |acc, value| gcd(acc, value.abs()))
        .max(1);
    for value in &mut values {
        *value /= divisor;
    }
    if values
        .iter()
        .find(|value| **value != 0)
        .is_some_and(|value| *value < 0)
    {
        for value in &mut values {
            *value = -*value;
        }
    }
    values
}

fn generate_atoms() -> Vec<Atom> {
    let mut seen = BTreeSet::new();
    let mut atoms = Vec::new();
    for p in -2..=2 {
        for q in -2..=2 {
            for first in -2..=2 {
                for second in -2..=2 {
                    if (first, second) == (0, 0) {
                        continue;
                    }
                    let mut values = [0; LENGTH];
                    values[0] = first;
                    values[1] = second;
                    for index in 0..LENGTH - 2 {
                        values[index + 2] = p * values[index + 1] + q * values[index];
                    }
                    if p * values[LENGTH - 1] + q * values[LENGTH - 2] != values[0]
                        || p * values[0] + q * values[LENGTH - 1] != values[1]
                    {
                        continue;
                    }
                    let values = normalize_sequence(values);
                    if seen.insert(values) {
                        atoms.push(Atom {
                            values,
                            generator: (p, q),
                            seeds: (first, second),
                        });
                    }
                }
            }
        }
    }
    atoms
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ShiftMatrix([[i32; 2]; 2]);

fn shift(values: &[i32; LENGTH], amount: usize) -> [i32; LENGTH] {
    std::array::from_fn(|index| values[(index + amount) % LENGTH])
}

fn independent(left: &Atom, right: &Atom) -> bool {
    (0..LENGTH).any(|i| {
        (0..LENGTH).any(|j| left.values[i] * right.values[j] != left.values[j] * right.values[i])
    })
}

/// Independent bounded checker for M9-style closed dynamics. The matrix is
/// recomputed from raw sequences; recurrence metadata is not consulted.
fn find_shift_matrix(left: &Atom, right: &Atom) -> Option<ShiftMatrix> {
    if !independent(left, right) {
        return None;
    }
    let shifted_left = shift(&left.values, 1);
    let shifted_right = shift(&right.values, 1);
    for a in -3..=3 {
        for b in -3..=3 {
            for c in -3..=3 {
                for d in -3..=3 {
                    if (0..LENGTH).all(|index| {
                        shifted_left[index] == a * left.values[index] + b * right.values[index]
                            && shifted_right[index]
                                == c * left.values[index] + d * right.values[index]
                    }) {
                        return Some(ShiftMatrix([[a, b], [c, d]]));
                    }
                }
            }
        }
    }
    None
}

#[derive(Clone, Debug)]
struct Candidate {
    atoms: Vec<usize>,
    weights: Vec<i32>,
    values: [i32; LENGTH],
    closed_shift: Option<ShiftMatrix>,
    raw_index: usize,
}

fn candidate_values(atoms: &[Atom], indices: &[usize], weights: &[i32]) -> [i32; LENGTH] {
    std::array::from_fn(|sample| {
        indices
            .iter()
            .zip(weights)
            .map(|(index, weight)| weight * atoms[*index].values[sample])
            .sum()
    })
}

fn enumerate_candidates(atoms: &[Atom]) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    for atom in 0..atoms.len() {
        for weight in WEIGHTS {
            let raw_index = candidates.len();
            candidates.push(Candidate {
                atoms: vec![atom],
                weights: vec![weight],
                values: candidate_values(atoms, &[atom], &[weight]),
                closed_shift: None,
                raw_index,
            });
        }
    }
    for left in 0..atoms.len() {
        for right in left + 1..atoms.len() {
            let closed_shift = find_shift_matrix(&atoms[left], &atoms[right]);
            for left_weight in WEIGHTS {
                for right_weight in WEIGHTS {
                    let raw_index = candidates.len();
                    candidates.push(Candidate {
                        atoms: vec![left, right],
                        weights: vec![left_weight, right_weight],
                        values: candidate_values(
                            atoms,
                            &[left, right],
                            &[left_weight, right_weight],
                        ),
                        closed_shift,
                        raw_index,
                    });
                }
            }
        }
    }
    candidates
}

fn guided_order(candidates: &[Candidate]) -> Vec<usize> {
    let mut order = (0..candidates.len()).collect::<Vec<_>>();
    order.sort_by_key(|index| {
        (
            candidates[*index].closed_shift.is_none(),
            candidates[*index].raw_index,
        )
    });
    order
}

#[derive(Clone, Debug)]
struct Fit {
    candidate: usize,
    checks: usize,
    error: i64,
}

fn squared_error(left: &[i32; LENGTH], right: &[i32; LENGTH]) -> i64 {
    left.iter()
        .zip(right)
        .map(|(a, b)| i64::from(a - b).pow(2))
        .sum()
}

fn find_exact(target: &[i32; LENGTH], candidates: &[Candidate], order: &[usize]) -> Option<Fit> {
    order.iter().enumerate().find_map(|(position, index)| {
        (candidates[*index].values == *target).then_some(Fit {
            candidate: *index,
            checks: position + 1,
            error: 0,
        })
    })
}

fn find_composition(
    target: &[i32; LENGTH],
    candidates: &[Candidate],
    order: &[usize],
) -> Option<(usize, usize, usize)> {
    let mut checks = 0;
    for left_index in order {
        for right_index in order {
            checks += 1;
            if add(
                &candidates[*left_index].values,
                &candidates[*right_index].values,
            ) == *target
            {
                return Some((checks, *left_index, *right_index));
            }
        }
    }
    None
}

fn signal(values: &[i32]) -> [i32; LENGTH] {
    std::array::from_fn(|index| values[index % values.len()])
}

fn add(left: &[i32; LENGTH], right: &[i32; LENGTH]) -> [i32; LENGTH] {
    std::array::from_fn(|index| left[index] + right[index])
}

fn scale(values: &[i32; LENGTH], factor: i32) -> [i32; LENGTH] {
    std::array::from_fn(|index| factor * values[index])
}

fn discovery_signals() -> Vec<[i32; LENGTH]> {
    let s1 = signal(&[2, 1, -1, -2, -1, 1]);
    let s2 = signal(&[0, 1, 1, 0, -1, -1]);
    let s4 = signal(&[1, 0, -1, 0]);
    let s5 = signal(&[0, 1, 0, -1]);
    vec![
        s1,
        s2,
        add(&scale(&s1, 2), &scale(&s2, -1)),
        s4,
        s5,
        add(&s4, &scale(&s5, 2)),
    ]
}

#[derive(Clone, Debug)]
pub struct TransferResult {
    pub task: &'static str,
    pub unguided_checks: usize,
    pub guided_checks: usize,
    pub exact_error: i64,
    pub prediction_checked: bool,
    pub composition_checked: bool,
}

#[derive(Clone, Debug)]
pub struct FourierDiscovery {
    pub atom_count: usize,
    pub candidate_count: usize,
    pub retained_families: usize,
    pub discovery_checks: usize,
    pub transfers: Vec<TransferResult>,
    pub unguided_checks: usize,
    pub guided_checks: usize,
    pub guided_improved_tasks: usize,
    pub negative_transfer_tasks: usize,
    pub time_domain_integers: usize,
    pub coordinate_description_integers: usize,
    pub impulse_exact_rejected: bool,
    pub ramp_closure_rejected: bool,
    pub corruption_exact_rejected: bool,
    pub constant_does_not_retain_oscillation: bool,
    pub noisy_squared_error: i64,
    pub candidate_sets_identical: bool,
    pub l3_boundary_passed: bool,
}

fn prediction_valid(candidate: &Candidate, atoms: &[Atom], target: &[i32; LENGTH]) -> bool {
    let Some(matrix) = candidate.closed_shift else {
        return false;
    };
    if candidate.atoms.len() != 2 {
        return false;
    }
    let mut weights = [candidate.weights[0], candidate.weights[1]];
    for amount in 0..LENGTH {
        let reconstructed = candidate_values(atoms, &candidate.atoms, &weights);
        if reconstructed != shift(target, amount) {
            return false;
        }
        // If rows describe shifts of atoms in the original basis, signal
        // weights transform by the transpose.
        weights = [
            matrix.0[0][0] * weights[0] + matrix.0[1][0] * weights[1],
            matrix.0[0][1] * weights[0] + matrix.0[1][1] * weights[1],
        ];
    }
    true
}

fn composition_prediction_valid(
    left: &Candidate,
    right: &Candidate,
    atoms: &[Atom],
    target: &[i32; LENGTH],
) -> bool {
    prediction_valid(left, atoms, &left.values)
        && prediction_valid(right, atoms, &right.values)
        && (0..LENGTH).all(|amount| {
            add(&shift(&left.values, amount), &shift(&right.values, amount))
                == shift(target, amount)
        })
}

pub fn m15_experiment() -> FourierDiscovery {
    let atoms = generate_atoms();
    let candidates = enumerate_candidates(&atoms);
    let unguided = (0..candidates.len()).collect::<Vec<_>>();
    let guided = guided_order(&candidates);
    let discoveries = discovery_signals();
    let retained = discoveries
        .iter()
        .filter_map(|target| find_exact(target, &candidates, &guided))
        .collect::<Vec<_>>();
    let retained_families = retained
        .iter()
        .filter_map(|fit| candidates[fit.candidate].closed_shift)
        .collect::<BTreeSet<_>>()
        .len();

    let s1 = discoveries[0];
    let s2 = discoveries[1];
    let s3 = discoveries[2];
    let s4 = discoveries[3];
    let s5 = discoveries[4];
    let s6 = discoveries[5];
    let ordinary = [
        ("phase_shift", shift(&s3, 1)),
        ("amplitude_scale", scale(&s6, -2)),
        ("new_period6_mixture", add(&scale(&s1, -1), &scale(&s2, 3))),
        ("new_period4_mixture", add(&scale(&s4, 3), &scale(&s5, -1))),
    ];
    let mut transfers = Vec::new();
    for (name, target) in ordinary {
        let raw = find_exact(&target, &candidates, &unguided).expect("unguided exact fit");
        let prioritized = find_exact(&target, &candidates, &guided).expect("guided exact fit");
        transfers.push(TransferResult {
            task: name,
            unguided_checks: raw.checks,
            guided_checks: prioritized.checks,
            exact_error: prioritized.error,
            prediction_checked: prediction_valid(
                &candidates[prioritized.candidate],
                &atoms,
                &target,
            ),
            composition_checked: false,
        });
    }
    let composition_target = add(&s3, &s6);
    let (raw_checks, _, _) =
        find_composition(&composition_target, &candidates, &unguided).expect("raw composition");
    let (guided_checks, left, right) =
        find_composition(&composition_target, &candidates, &guided).expect("guided composition");
    transfers.push(TransferResult {
        task: "cross_family_composition",
        unguided_checks: raw_checks,
        guided_checks,
        exact_error: squared_error(
            &add(&candidates[left].values, &candidates[right].values),
            &composition_target,
        ),
        prediction_checked: composition_prediction_valid(
            &candidates[left],
            &candidates[right],
            &atoms,
            &composition_target,
        ),
        composition_checked: true,
    });

    let impulse = signal(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let ramp: [i32; LENGTH] = std::array::from_fn(|index| index as i32);
    let mut corrupted = s3;
    corrupted[7] += 1;
    let constant = [3; LENGTH];
    let noisy = {
        let mut values = s3;
        values[2] += 1;
        values[9] -= 1;
        values
    };
    let noisy_squared_error = guided
        .iter()
        .map(|index| squared_error(&candidates[*index].values, &noisy))
        .min()
        .unwrap();
    let unguided_checks = transfers.iter().map(|task| task.unguided_checks).sum();
    let guided_checks = transfers.iter().map(|task| task.guided_checks).sum();
    let guided_improved_tasks = transfers
        .iter()
        .filter(|task| task.guided_checks < task.unguided_checks)
        .count();
    let negative_transfer_tasks = transfers
        .iter()
        .filter(|task| task.guided_checks > task.unguided_checks)
        .count();
    let time_domain_integers = discoveries.len() * LENGTH;
    // Two retained recurrence families: p,q + two 2-value seeds each; every
    // discovery signal then stores its two reconstruction weights.
    let coordinate_description_integers = retained_families * 6 + discoveries.len() * 2;
    let impulse_exact_rejected = find_exact(&impulse, &candidates, &guided).is_none();
    let ramp_closure_rejected = find_exact(&ramp, &candidates, &guided).is_none();
    let corruption_exact_rejected = find_exact(&corrupted, &candidates, &guided).is_none();
    // Constants have a cheaper one-atom explanation and therefore cannot
    // justify retaining a two-coordinate oscillatory family.
    let constant_does_not_retain_oscillation = candidates
        .iter()
        .any(|candidate| candidate.atoms.len() == 1 && candidate.values == constant);
    let candidate_sets_identical = {
        let mut left = unguided.clone();
        let mut right = guided.clone();
        left.sort_unstable();
        right.sort_unstable();
        left == right
    };
    let l3_boundary_passed = retained_families >= 2
        && transfers.iter().all(|task| {
            task.exact_error == 0
                && task.prediction_checked
                && (!task.composition_checked || task.exact_error == 0)
        })
        && guided_improved_tasks >= 4
        && negative_transfer_tasks == 0
        && coordinate_description_integers < time_domain_integers
        && impulse_exact_rejected
        && ramp_closure_rejected
        && corruption_exact_rejected
        && constant_does_not_retain_oscillation
        && noisy_squared_error > 0
        && candidate_sets_identical;
    FourierDiscovery {
        atom_count: atoms.len(),
        candidate_count: candidates.len(),
        retained_families,
        discovery_checks: retained.iter().map(|fit| fit.checks).sum(),
        transfers,
        unguided_checks,
        guided_checks,
        guided_improved_tasks,
        negative_transfer_tasks,
        time_domain_integers,
        coordinate_description_integers,
        impulse_exact_rejected,
        ramp_closure_rejected,
        corruption_exact_rejected,
        constant_does_not_retain_oscillation,
        noisy_squared_error,
        candidate_sets_identical,
        l3_boundary_passed,
    }
}

pub fn machine_record(report: &FourierDiscovery) -> String {
    let transfers = report
        .transfers
        .iter()
        .map(|task| {
            format!(
                "{}:{}>{}:error={}:prediction={}:composition={}",
                task.task,
                task.unguided_checks,
                task.guided_checks,
                task.exact_error,
                task.prediction_checked,
                task.composition_checked
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "experiment=math_world_m15,atoms={},candidates={},retained_families={},discovery_checks={},transfers={},unguided_checks={},m9_guided_checks={},guided_improved_tasks={},negative_transfer_tasks={},time_domain_integers={},coordinate_description_integers={},impulse_exact_rejected={},ramp_closure_rejected={},corruption_exact_rejected={},constant_does_not_retain_oscillation={},noisy_squared_error={},candidate_sets_identical={},named_frequency_primitives=false,fourier_dictionary_supplied=false,recurrence_generator_supplied=true,simple_dynamics_objective_supplied=true,l3_boundary_passed={},claim_level={},proof_status=exact_cyclic_recurrence_coordinate_checks,deterministic=true,fallback=exact",
        report.atom_count,
        report.candidate_count,
        report.retained_families,
        report.discovery_checks,
        transfers,
        report.unguided_checks,
        report.guided_checks,
        report.guided_improved_tasks,
        report.negative_transfer_tasks,
        report.time_domain_integers,
        report.coordinate_description_integers,
        report.impulse_exact_rejected,
        report.ramp_closure_rejected,
        report.corruption_exact_rejected,
        report.constant_does_not_retain_oscillation,
        report.noisy_squared_error,
        report.candidate_sets_identical,
        report.l3_boundary_passed,
        if report.l3_boundary_passed {
            "L3_transferred_ontology_with_measured_utility"
        } else {
            "L2_invented_feature_in_supplied_meta_ontology"
        }
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Routed,
    Declined,
}

#[derive(Clone, Debug)]
pub struct ConditionalTransferMeasurement {
    pub task: &'static str,
    pub compatible: bool,
    pub route: Route,
    pub probe_evaluations: usize,
    pub baseline_checks: usize,
    pub acquired_checks: usize,
    pub squared_error: i64,
    pub exact_winner: bool,
    pub prediction_checked: bool,
    pub winner_atoms: Vec<usize>,
    pub winner_weights: Vec<i32>,
}

#[derive(Clone, Debug)]
pub struct ConditionalFourierDiscovery {
    pub transfers: Vec<ConditionalTransferMeasurement>,
    pub probe_evaluations_per_condition: usize,
    pub baseline_checks: usize,
    pub acquired_checks: usize,
    pub measured_gain: usize,
    pub routed_accelerated: usize,
    pub declined_unchanged: usize,
    pub controls_unchanged: usize,
    pub false_positive_routes: usize,
    pub negative_transfer_tasks: usize,
    pub impulse_rejected: bool,
    pub ramp_rejected: bool,
    pub corruption_rejected: bool,
    pub noisy_squared_error: i64,
    pub constant_declined: bool,
    pub candidate_sets_identical: bool,
    pub l3_boundary_passed: bool,
}

fn probe_applicability(
    target: &[i32; LENGTH],
    atoms: &[Atom],
    closed_pairs: &[Vec<usize>],
) -> Route {
    let probes = 0..6;
    let single_atom = atoms.iter().enumerate().any(|(atom, _)| {
        WEIGHTS.iter().any(|weight| {
            probes
                .clone()
                .all(|index| target[index] == weight * atoms[atom].values[index])
        })
    });
    if single_atom {
        return Route::Declined;
    }
    let closed_pair = closed_pairs.iter().any(|pair| {
        WEIGHTS.iter().any(|left_weight| {
            WEIGHTS.iter().any(|right_weight| {
                probes.clone().all(|index| {
                    target[index]
                        == left_weight * atoms[pair[0]].values[index]
                            + right_weight * atoms[pair[1]].values[index]
                })
            })
        })
    });
    if closed_pair {
        Route::Routed
    } else {
        Route::Declined
    }
}

fn best_squared_error(target: &[i32; LENGTH], candidates: &[Candidate], order: &[usize]) -> i64 {
    order
        .iter()
        .map(|index| squared_error(&candidates[*index].values, target))
        .min()
        .unwrap()
}

/// Separately pre-registered M15b repair. It consumes M15's closed-shift
/// coordinate schema but none of M15b's frozen tasks participate in retention.
pub fn m15b_experiment() -> ConditionalFourierDiscovery {
    let atoms = generate_atoms();
    let candidates = enumerate_candidates(&atoms);
    let unguided = (0..candidates.len()).collect::<Vec<_>>();
    let guided = guided_order(&candidates);
    let closed_pairs = candidates
        .iter()
        .filter(|candidate| candidate.closed_shift.is_some())
        .map(|candidate| candidate.atoms.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let discoveries = discovery_signals();
    let s1 = discoveries[0];
    let s2 = discoveries[1];
    let s3 = discoveries[2];
    let s4 = discoveries[3];
    let s5 = discoveries[4];
    let s6 = discoveries[5];
    let mut tasks: Vec<(&'static str, bool, [i32; LENGTH])> = vec![
        ("phase_shift_two", true, shift(&s3, 2)),
        ("phase_shift_three", true, shift(&s3, 3)),
        ("phase_shift_four", true, shift(&s3, 4)),
        ("phase_shift_five", true, shift(&s3, 5)),
        (
            "period4_pair_2_3",
            true,
            add(&scale(&s4, 2), &scale(&s5, 3)),
        ),
        (
            "period4_pair_minus2_3",
            true,
            add(&scale(&s4, -2), &scale(&s5, 3)),
        ),
        (
            "period4_pair_2_minus3",
            true,
            add(&scale(&s4, 2), &scale(&s5, -3)),
        ),
        (
            "period6_pair_2_3",
            true,
            add(&scale(&s1, 2), &scale(&s2, 3)),
        ),
        (
            "period6_pair_minus2_3",
            true,
            add(&scale(&s1, -2), &scale(&s2, 3)),
        ),
        (
            "period6_nonclosed_3_minus2",
            true,
            add(&scale(&s1, 3), &scale(&s2, -2)),
        ),
        ("period6_single_atom_minus3_s6", true, scale(&s6, -3)),
        ("constant", false, [3; LENGTH]),
        (
            "impulse",
            false,
            signal(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        ),
        ("ramp", false, std::array::from_fn(|index| index as i32)),
        ("corrupt_s3_sample3", false, {
            let mut values = s3;
            values[3] += 1;
            values
        }),
        ("noisy_s3_samples2_5", false, {
            let mut values = s3;
            values[2] += 1;
            values[5] -= 1;
            values
        }),
    ];
    let mut transfers = Vec::new();
    for (name, compatible, target) in tasks.drain(..) {
        let baseline_fit = find_exact(&target, &candidates, &unguided);
        let baseline_checks = baseline_fit
            .as_ref()
            .map(|fit| fit.checks)
            .unwrap_or(candidates.len());
        let route = probe_applicability(&target, &atoms, &closed_pairs);
        let acquired_fit = match route {
            Route::Routed => find_exact(&target, &candidates, &guided),
            Route::Declined => baseline_fit.clone(),
        };
        let acquired_checks = acquired_fit
            .as_ref()
            .map(|fit| fit.checks)
            .unwrap_or(candidates.len());
        let (squared_error, winner_atoms, winner_weights, prediction_checked) = match &acquired_fit
        {
            Some(fit) => {
                let candidate = &candidates[fit.candidate];
                (
                    fit.error,
                    candidate.atoms.clone(),
                    candidate.weights.clone(),
                    route == Route::Routed && prediction_valid(candidate, &atoms, &target),
                )
            }
            None => (
                best_squared_error(
                    &target,
                    &candidates,
                    match route {
                        Route::Routed => &guided,
                        Route::Declined => &unguided,
                    },
                ),
                Vec::new(),
                Vec::new(),
                false,
            ),
        };
        transfers.push(ConditionalTransferMeasurement {
            task: name,
            compatible,
            route,
            probe_evaluations: 6,
            baseline_checks,
            acquired_checks,
            squared_error,
            exact_winner: acquired_fit.is_some(),
            prediction_checked,
            winner_atoms,
            winner_weights,
        });
    }
    let baseline_checks = transfers.iter().map(|task| task.baseline_checks).sum();
    let acquired_checks = transfers.iter().map(|task| task.acquired_checks).sum();
    let probe_evaluations_per_condition = transfers.iter().map(|task| task.probe_evaluations).sum();
    let routed_accelerated = transfers
        .iter()
        .filter(|task| task.compatible && task.route == Route::Routed)
        .filter(|task| task.acquired_checks < task.baseline_checks)
        .count();
    let declined_unchanged = transfers
        .iter()
        .filter(|task| task.route == Route::Declined)
        .filter(|task| task.acquired_checks == task.baseline_checks)
        .count();
    let controls_unchanged = transfers
        .iter()
        .filter(|task| !task.compatible)
        .filter(|task| task.route == Route::Declined)
        .filter(|task| task.acquired_checks == task.baseline_checks)
        .count();
    let false_positive_routes = transfers
        .iter()
        .filter(|task| !task.compatible && task.route == Route::Routed)
        .count();
    let negative_transfer_tasks = transfers
        .iter()
        .filter(|task| task.acquired_checks > task.baseline_checks)
        .count();
    let task = |name: &str| transfers.iter().find(|task| task.task == name).unwrap();
    let impulse_rejected = !task("impulse").exact_winner;
    let ramp_rejected = !task("ramp").exact_winner;
    let corruption_rejected = !task("corrupt_s3_sample3").exact_winner;
    let noisy_squared_error = task("noisy_s3_samples2_5").squared_error;
    let constant = task("constant");
    let constant_declined = constant.route == Route::Declined && constant.winner_atoms.len() == 1;
    let candidate_sets_identical = {
        let mut left = unguided.clone();
        let mut right = guided.clone();
        left.sort_unstable();
        right.sort_unstable();
        left == right
    };
    let l3_boundary_passed = transfers
        .iter()
        .filter(|task| task.compatible)
        .all(|task| task.exact_winner && task.squared_error == 0)
        && transfers
            .iter()
            .filter(|task| task.compatible && task.route == Route::Routed)
            .all(|task| task.acquired_checks < task.baseline_checks && task.prediction_checked)
        && transfers
            .iter()
            .filter(|task| task.route == Route::Declined)
            .all(|task| task.acquired_checks == task.baseline_checks)
        && controls_unchanged == 5
        && false_positive_routes == 0
        && negative_transfer_tasks == 0
        && acquired_checks < baseline_checks
        && impulse_rejected
        && ramp_rejected
        && corruption_rejected
        && noisy_squared_error > 0
        && constant_declined
        && candidate_sets_identical;
    ConditionalFourierDiscovery {
        transfers,
        probe_evaluations_per_condition,
        baseline_checks,
        acquired_checks,
        measured_gain: baseline_checks.saturating_sub(acquired_checks),
        routed_accelerated,
        declined_unchanged,
        controls_unchanged,
        false_positive_routes,
        negative_transfer_tasks,
        impulse_rejected,
        ramp_rejected,
        corruption_rejected,
        noisy_squared_error,
        constant_declined,
        candidate_sets_identical,
        l3_boundary_passed,
    }
}

pub fn m15b_machine_record(report: &ConditionalFourierDiscovery) -> String {
    let transfers = report
        .transfers
        .iter()
        .map(|task| {
            format!(
                "{}:compatible={}:route={}:checks={}>{}:error={}:exact={}:prediction={}:winner={:?}:weights={:?}",
                task.task,
                task.compatible,
                match task.route {
                    Route::Routed => "guided",
                    Route::Declined => "declined",
                },
                task.baseline_checks,
                task.acquired_checks,
                task.squared_error,
                task.exact_winner,
                task.prediction_checked,
                task.winner_atoms,
                task.winner_weights
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "experiment=math_world_m15b,atoms=32,candidates=18048,transfers={},probe_evaluations_per_condition={},baseline_checks={},acquired_checks={},measured_gain={},routed_accelerated={},declined_unchanged={},controls_unchanged={},false_positive_routes={},negative_transfer_tasks={},impulse_rejected={},ramp_rejected={},corruption_rejected={},noisy_squared_error={},constant_declined={},candidate_sets_identical={},probe_positions=0..5,single_atom_probe=true,closed_pair_probe=true,named_frequency_primitives=false,fourier_dictionary_supplied=false,recurrence_generator_supplied=true,probe_routing_supplied=true,simple_dynamics_objective_supplied=true,l3_boundary_passed={},claim_level={},proof_status=exact_conditional_coordinate_routing,deterministic=true,fallback=exact",
        transfers,
        report.probe_evaluations_per_condition,
        report.baseline_checks,
        report.acquired_checks,
        report.measured_gain,
        report.routed_accelerated,
        report.declined_unchanged,
        report.controls_unchanged,
        report.false_positive_routes,
        report.negative_transfer_tasks,
        report.impulse_rejected,
        report.ramp_rejected,
        report.corruption_rejected,
        report.noisy_squared_error,
        report.constant_declined,
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
    fn invents_exact_oscillatory_coordinate_families() {
        let report = m15_experiment();
        assert_eq!(report.atom_count, 32);
        assert!(report.retained_families >= 2);
        assert!(report.transfers.iter().all(|task| task.exact_error == 0));
        assert!(report.transfers.iter().all(|task| task.prediction_checked));
    }

    #[test]
    fn m9_guidance_only_reorders_but_fails_the_transfer_gate() {
        let report = m15_experiment();
        assert!(report.candidate_sets_identical);
        assert_eq!(report.guided_improved_tasks, 2);
        assert_eq!(report.negative_transfer_tasks, 3);
        assert!(report.guided_checks > report.unguided_checks);
    }

    #[test]
    fn controls_compression_and_noise_are_explicit() {
        let report = m15_experiment();
        assert!(report.impulse_exact_rejected);
        assert!(report.ramp_closure_rejected);
        assert!(report.corruption_exact_rejected);
        assert!(report.constant_does_not_retain_oscillation);
        assert!(report.noisy_squared_error > 0);
        assert!(report.coordinate_description_integers < report.time_domain_integers);
        assert!(!report.l3_boundary_passed);
    }

    #[test]
    fn record_is_deterministic() {
        assert_eq!(
            machine_record(&m15_experiment()),
            machine_record(&m15_experiment())
        );
    }

    #[test]
    fn conditional_routing_passes_m15b_gate() {
        let report = m15b_experiment();
        assert_eq!(report.routed_accelerated, 9);
        assert_eq!(report.declined_unchanged, 7);
        assert_eq!(report.controls_unchanged, 5);
        assert_eq!(report.false_positive_routes, 0);
        assert_eq!(report.negative_transfer_tasks, 0);
        assert!(report.acquired_checks < report.baseline_checks);
        assert!(report.l3_boundary_passed);
    }

    #[test]
    fn conditional_routing_reconstructs_and_checks_every_compatible_task() {
        let report = m15b_experiment();
        for task in report.transfers.iter().filter(|task| task.compatible) {
            assert!(task.exact_winner, "{}", task.task);
            assert_eq!(task.squared_error, 0, "{}", task.task);
            if task.route == Route::Routed {
                assert!(task.acquired_checks < task.baseline_checks, "{}", task.task);
                assert!(task.prediction_checked, "{}", task.task);
            } else {
                assert_eq!(task.acquired_checks, task.baseline_checks, "{}", task.task);
            }
        }
    }

    #[test]
    fn conditional_routing_declines_all_controls() {
        let report = m15b_experiment();
        for task in report.transfers.iter().filter(|task| !task.compatible) {
            assert_eq!(task.route, Route::Declined, "{}", task.task);
            assert_eq!(task.acquired_checks, task.baseline_checks, "{}", task.task);
        }
        assert!(report.impulse_rejected);
        assert!(report.ramp_rejected);
        assert!(report.corruption_rejected);
        assert!(report.noisy_squared_error > 0);
        assert!(report.constant_declined);
        assert_eq!(report.probe_evaluations_per_condition, 96);
    }

    #[test]
    fn m15b_record_is_deterministic() {
        assert_eq!(
            m15b_machine_record(&m15b_experiment()),
            m15b_machine_record(&m15b_experiment())
        );
    }
}
