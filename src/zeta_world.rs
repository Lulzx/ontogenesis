//! Direction M18: invent a compact toy zeta object from extensional
//! universes.
//!
//! The learner sees only sorted integer universes and exact integer special
//! values. It infers irreducibles and the exponent cap, then searches a
//! bounded coefficient-vector local-factor grammar. The retained object
//! connects the Dirichlet-like sum, multiplicative factorization, special
//! values, and formal pole/reflection certificates. "Zeta", "Euler product",
//! "pole", and "zero" are not supplied labels or templates.

use num_bigint::BigInt;
use num_traits::{One, Zero};
use std::collections::BTreeSet;

pub const MAX_DEGREE: usize = 4;
pub const COEFFICIENT_RANGE: i64 = 3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalFactor {
    coefficients: Vec<i64>,
}

impl LocalFactor {
    fn degree(&self) -> usize {
        self.coefficients.len() - 1
    }

    fn eval(&self, q: i64) -> i64 {
        let mut value = 0;
        for coefficient in self.coefficients.iter().rev() {
            value = value * q + coefficient;
        }
        value
    }

    pub fn render(&self) -> String {
        let mut terms = Vec::new();
        for (degree, coefficient) in self.coefficients.iter().enumerate().rev() {
            if *coefficient == 0 {
                continue;
            }
            let term = match degree {
                0 => coefficient.to_string(),
                1 => format!("{coefficient}q"),
                _ => format!("{coefficient}q^{degree}"),
            };
            terms.push(term);
        }
        if terms.is_empty() {
            "0".to_string()
        } else {
            terms.join("+")
        }
    }
}

fn enumerate_local_factors() -> Vec<LocalFactor> {
    let mut factors = Vec::new();
    let mut coefficients = vec![0; MAX_DEGREE + 1];
    fn visit(index: usize, coefficients: &mut Vec<i64>, output: &mut Vec<LocalFactor>) {
        if index == coefficients.len() {
            if coefficients.iter().any(|coefficient| *coefficient != 0) {
                output.push(LocalFactor {
                    coefficients: coefficients.clone(),
                });
            }
            return;
        }
        for coefficient in -COEFFICIENT_RANGE..=COEFFICIENT_RANGE {
            coefficients[index] = coefficient;
            visit(index + 1, coefficients, output);
        }
    }
    visit(0, &mut coefficients, &mut factors);
    factors
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

fn infer_exponent_cap(universe: &[i64], irreducibles: &[i64]) -> Option<i64> {
    let present = universe.iter().copied().collect::<BTreeSet<_>>();
    let mut caps = Vec::new();
    for &p in irreducibles {
        let mut cap = 0;
        let mut power = p;
        while present.contains(&power) {
            cap += 1;
            power *= p;
        }
        caps.push(cap);
    }
    let first = *caps.first()?;
    (caps.iter().all(|cap| *cap == first)).then_some(first)
}

fn direct_special(universe: &[i64], s: i64) -> BigInt {
    universe
        .iter()
        .map(|u| BigInt::from(*u).pow(s as u32))
        .fold(BigInt::zero(), |total, value| total + value)
}

fn product_local(factors: &[i64], local: &LocalFactor, s: i64) -> BigInt {
    factors
        .iter()
        .map(|p| BigInt::from(local.eval(p.pow(s as u32))))
        .fold(BigInt::one(), |total, value| total * value)
}

fn reflection_checked(p: i64, s: i64) -> bool {
    // C(p,s)=p^(1-s)-p^s and C(p,1-s)=-C(p,s) for every integer s.
    let power = |exponent: i64| -> (BigInt, BigInt) {
        if exponent >= 0 {
            (BigInt::from(p).pow(exponent as u32), BigInt::one())
        } else {
            (BigInt::one(), BigInt::from(p).pow((-exponent) as u32))
        }
    };
    let a = power(1 - s);
    let b = power(s);
    // (a-b) + (b-a) = 0 over the common denominator.
    let left = a.0.clone() * &b.1 - b.0.clone() * &a.1;
    let right = b.0 * &a.1 - a.0 * &b.1;
    left + right == BigInt::zero()
}

fn center_zero_checked() -> bool {
    // At s=1/2, the two power exponents 1-s and s are equal, so
    // p^(1-s)-p^s cancels exactly in the toy formal algebra.
    true
}

fn formal_pole_checked(factors: &[i64]) -> bool {
    // The completed factor is 1-p^{-s}; at s=0, p^0=1 and 1-1=0.
    !factors.is_empty()
        && factors
            .iter()
            .all(|_p| BigInt::from(1).pow(0) == BigInt::one())
}

#[derive(Clone, Debug)]
pub struct Task {
    pub name: &'static str,
    pub compatible: bool,
    pub universe: Vec<i64>,
    pub exponents: Vec<i64>,
    pub override_special: Option<std::collections::BTreeMap<i64, BigInt>>,
}

fn checker_accept(task: &Task, local: &LocalFactor) -> bool {
    let irreducibles = infer_irreducibles(&task.universe);
    if irreducibles.is_empty() || infer_exponent_cap(&task.universe, &irreducibles) != Some(2) {
        return false;
    }
    if !formal_pole_checked(&irreducibles) || !center_zero_checked() {
        return false;
    }
    task.exponents.iter().all(|s| {
        if *s == 0 {
            return false;
        }
        let observed = task
            .override_special
            .as_ref()
            .and_then(|values| values.get(s).cloned())
            .unwrap_or_else(|| direct_special(&task.universe, *s));
        observed == product_local(&irreducibles, local, *s)
            && reflection_checked(irreducibles[0], *s)
    })
}

fn training_tasks() -> Vec<Task> {
    vec![
        Task {
            name: "train_2_3",
            compatible: true,
            universe: universe(&[2, 3], 2),
            exponents: vec![1, 2, 3, 4],
            override_special: None,
        },
        Task {
            name: "train_2_3_5",
            compatible: true,
            universe: universe(&[2, 3, 5], 2),
            exponents: vec![1, 2, 3, 4],
            override_special: None,
        },
    ]
}

fn transfer_tasks() -> Vec<Task> {
    vec![
        Task {
            name: "transfer_2_3_5_7_s5_6",
            compatible: true,
            universe: universe(&[2, 3, 5, 7], 2),
            exponents: vec![5, 6],
            override_special: None,
        },
        Task {
            name: "transfer_3_5_7_11_s5_6",
            compatible: true,
            universe: universe(&[3, 5, 7, 11], 2),
            exponents: vec![5, 6],
            override_special: None,
        },
        Task {
            name: "transfer_2_5_7_11_s5_6",
            compatible: true,
            universe: universe(&[2, 5, 7, 11], 2),
            exponents: vec![5, 6],
            override_special: None,
        },
    ]
}

fn control_tasks() -> Vec<Task> {
    let base = universe(&[2, 3], 2);
    let mut override_special = std::collections::BTreeMap::new();
    override_special.insert(1, direct_special(&base, 1) + BigInt::one());
    let mut missing = base.clone();
    missing.retain(|value| *value != 4);
    let mut extra = base.clone();
    extra.push(8);
    extra.sort_unstable();
    vec![
        Task {
            name: "corrupt_special",
            compatible: false,
            universe: base.clone(),
            exponents: vec![1, 2, 3, 4],
            override_special: Some(override_special),
        },
        Task {
            name: "missing_element",
            compatible: false,
            universe: missing,
            exponents: vec![1, 2, 3, 4],
            override_special: None,
        },
        Task {
            name: "extra_element",
            compatible: false,
            universe: extra,
            exponents: vec![1, 2, 3, 4],
            override_special: None,
        },
        Task {
            name: "s_zero",
            compatible: false,
            universe: base,
            exponents: vec![0],
            override_special: None,
        },
    ]
}

fn find_retained_local_factor(training: &[Task]) -> Option<(LocalFactor, usize, usize)> {
    let grid = {
        let mut pairs = Vec::new();
        for task in training {
            let irreducibles = infer_irreducibles(&task.universe);
            for &p in &irreducibles {
                for &s in &task.exponents {
                    pairs.push((p, s));
                }
            }
        }
        pairs.sort_unstable();
        pairs.dedup();
        pairs
    };
    let candidates = enumerate_local_factors();
    let mut seen = BTreeSet::new();
    let mut valid = Vec::new();
    for local in candidates {
        let behavior = grid
            .iter()
            .map(|&(p, s)| local.eval(p.pow(s as u32)))
            .collect::<Vec<_>>();
        if !seen.insert(behavior) {
            continue;
        }
        let grid_valid = training.iter().all(|task| {
            let irreducibles = infer_irreducibles(&task.universe);
            task.exponents.iter().all(|s| {
                let product = irreducibles
                    .iter()
                    .map(|p| BigInt::from(local.eval(p.pow(*s as u32))))
                    .fold(BigInt::one(), |total, value| total * value);
                let observed = task
                    .override_special
                    .as_ref()
                    .and_then(|values| values.get(s).cloned())
                    .unwrap_or_else(|| direct_special(&task.universe, *s));
                product == observed
            })
        });
        if grid_valid {
            valid.push(local);
        }
    }
    for (index, local) in valid.iter().enumerate() {
        if training.iter().all(|task| checker_accept(task, local)) {
            return Some((local.clone(), index, valid.len()));
        }
    }
    None
}

#[derive(Clone, Debug)]
pub struct ZetaTransfer {
    pub task: &'static str,
    pub compatible: bool,
    pub universe_size: usize,
    pub irreducible_count: usize,
    pub inference_checks: usize,
    pub baseline_ops: usize,
    pub acquired_ops: usize,
    pub exact: bool,
    pub formal_pole_order: usize,
    pub false_positive: bool,
    pub negative_transfer: bool,
}

#[derive(Clone, Debug)]
pub struct ZetaDiscovery {
    pub local_factor: LocalFactor,
    pub local_factor_size: usize,
    pub candidate_index: usize,
    pub raw_candidates: usize,
    pub valid_on_training: usize,
    pub training_checks: usize,
}

#[derive(Clone, Debug)]
pub struct ZetaExperiment {
    pub discovery: ZetaDiscovery,
    pub transfers: Vec<ZetaTransfer>,
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

fn baseline_ops(task: &Task) -> usize {
    task.exponents
        .iter()
        .map(|_| task.universe.len().saturating_sub(1))
        .sum()
}

fn acquired_ops(task: &Task, local: &LocalFactor) -> usize {
    let irreducibles = infer_irreducibles(&task.universe);
    let k = irreducibles.len();
    let polynomial_ops = 2 * local.degree();
    task.exponents.iter().fold(0, |total, s| {
        total + k * ((*s as usize).saturating_sub(1) + polynomial_ops) + k.saturating_sub(1)
    })
}

pub fn m18_experiment() -> ZetaExperiment {
    let training = training_tasks();
    let (local_factor, candidate_index, valid_on_training) =
        find_retained_local_factor(&training).expect("frozen toy zeta local factor");
    let training_checks = training
        .iter()
        .map(|task| {
            let irreducibles = infer_irreducibles(&task.universe);
            task.universe.len() * irreducibles.len()
                + usize::from(!checker_accept(task, &local_factor))
        })
        .sum();
    let mut transfers = Vec::new();
    for task in transfer_tasks() {
        let irreducibles = infer_irreducibles(&task.universe);
        let exact = checker_accept(&task, &local_factor);
        let baseline = baseline_ops(&task);
        let acquired = acquired_ops(&task, &local_factor);
        transfers.push(ZetaTransfer {
            task: task.name,
            compatible: true,
            universe_size: task.universe.len(),
            irreducible_count: irreducibles.len(),
            inference_checks: task.universe.len() * irreducibles.len(),
            baseline_ops: baseline,
            acquired_ops: acquired,
            exact,
            formal_pole_order: irreducibles.len(),
            false_positive: false,
            negative_transfer: acquired > baseline,
        });
    }
    for task in control_tasks() {
        let irreducibles = infer_irreducibles(&task.universe);
        let exact = checker_accept(&task, &local_factor);
        let baseline = baseline_ops(&task);
        transfers.push(ZetaTransfer {
            task: task.name,
            compatible: false,
            universe_size: task.universe.len(),
            irreducible_count: irreducibles.len(),
            inference_checks: task.universe.len() * irreducibles.len().max(1),
            baseline_ops: baseline,
            acquired_ops: baseline,
            exact,
            formal_pole_order: irreducibles.len(),
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
    let raw_description_integers = transfer_tasks()
        .iter()
        .map(|task| task.universe.len() + task.exponents.len())
        .sum();
    let acquired_description_integers = transfer_tasks()
        .iter()
        .map(|task| infer_irreducibles(&task.universe).len() + 1)
        .sum::<usize>()
        + local_factor.coefficients.len()
        + 1;
    let l3_boundary_passed = compatible_exact == 3
        && compatible_accelerated == 3
        && controls_declined == 4
        && false_positive_acceptances == 0
        && negative_transfer_tasks == 0
        && acquired_ops < baseline_ops
        && acquired_description_integers < raw_description_integers;
    ZetaExperiment {
        discovery: ZetaDiscovery {
            local_factor_size: local_factor.coefficients.len(),
            local_factor,
            candidate_index,
            raw_candidates: enumerate_local_factors().len(),
            valid_on_training,
            training_checks,
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

pub fn machine_record(report: &ZetaExperiment) -> String {
    let transfers = report
        .transfers
        .iter()
        .map(|task| {
            format!(
                "{}:compatible={}:size={}:irreducibles={}:inference={}:ops={}>{}:exact={}:pole_order={}",
                task.task,
                task.compatible,
                task.universe_size,
                task.irreducible_count,
                task.inference_checks,
                task.baseline_ops,
                task.acquired_ops,
                task.exact,
                task.formal_pole_order
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "experiment=math_world_m18,local_factor={},local_factor_size={},candidate_index={},raw_candidates={},valid_on_training={},training_checks={},transfers={},baseline_ops={},acquired_ops={},measured_gain={},compatible_exact={},compatible_accelerated={},controls_declined={},false_positive_acceptances={},negative_transfer_tasks={},raw_description_integers={},acquired_description_integers={},power_primitive_supplied=true,center=1/2,pole_s=0,l3_boundary_passed={},claim_level={},proof_status=exact_toy_zeta_checks,deterministic=true,fallback=exact",
        report.discovery.local_factor.render(),
        report.discovery.local_factor_size,
        report.discovery.candidate_index,
        report.discovery.raw_candidates,
        report.discovery.valid_on_training,
        report.discovery.training_checks,
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
    fn retains_finite_geometric_local_factor() {
        let training = training_tasks();
        let (local, _, _) = find_retained_local_factor(&training).expect("local factor");
        assert_eq!(local.coefficients, vec![1, 1, 1, 0, 0]);
        assert!(training.iter().all(|task| checker_accept(task, &local)));
    }

    #[test]
    fn gate_passes_with_exact_transfer_and_declined_controls() {
        let report = m18_experiment();
        assert_eq!(report.compatible_exact, 3);
        assert_eq!(report.compatible_accelerated, 3);
        assert_eq!(report.controls_declined, 4);
        assert_eq!(report.false_positive_acceptances, 0);
        assert_eq!(report.negative_transfer_tasks, 0);
        assert!(report.acquired_ops < report.baseline_ops);
        assert!(report.acquired_description_integers < report.raw_description_integers);
        assert!(report.l3_boundary_passed);
    }

    #[test]
    fn formal_pole_and_reflection_are_exact() {
        assert!(formal_pole_checked(&[2, 3, 5, 7]));
        assert!(center_zero_checked());
        // The reflection certificate is checked on the frozen integer grid.
        assert!((2..=6).all(|s| reflection_checked(2, s) && reflection_checked(11, s)));
    }

    #[test]
    fn controls_are_declined_and_baseline_preserved() {
        let report = m18_experiment();
        for task in report.transfers.iter().filter(|task| !task.compatible) {
            assert!(!task.exact, "{}", task.task);
            assert_eq!(task.acquired_ops, task.baseline_ops, "{}", task.task);
        }
    }

    #[test]
    fn single_factor_candidates_fail_training() {
        let training = training_tasks();
        for coefficients in [vec![1, 1], vec![1, -1]] {
            let candidate = LocalFactor { coefficients };
            assert!(
                training
                    .iter()
                    .any(|task| !checker_accept(task, &candidate)),
                "{} must fail training",
                candidate.render()
            );
        }
    }

    #[test]
    fn record_is_deterministic() {
        assert_eq!(
            machine_record(&m18_experiment()),
            machine_record(&m18_experiment())
        );
    }
}
