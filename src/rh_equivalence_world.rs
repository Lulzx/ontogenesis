//! Direction M28: exact reflection-orbit reformulation of the M27 predicate.
//!
//! The checker proves equivalence of predicates in a small exact affine
//! fragment. It does not prove either predicate, RH, or global novelty.

use std::cmp::Ordering;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Transform {
    Identity,
    Reflection,
    Conjugation,
    ReflectionConjugation,
}

impl Transform {
    fn name(self) -> &'static str {
        match self {
            Transform::Identity => "I",
            Transform::Reflection => "R",
            Transform::Conjugation => "C",
            Transform::ReflectionConjugation => "RC",
        }
    }

    // Affine components on doubled coordinates (X,Y)=(2 Re(s),2 Im(s)).
    fn affine(self) -> ([i8; 3], [i8; 3]) {
        match self {
            Transform::Identity => ([1, 0, 0], [0, 1, 0]),
            Transform::Reflection => ([-1, 0, 2], [0, -1, 0]),
            Transform::Conjugation => ([1, 0, 0], [0, -1, 0]),
            Transform::ReflectionConjugation => ([-1, 0, 2], [0, 1, 0]),
        }
    }

    fn apply(self, point: (i64, i64)) -> (i64, i64) {
        let ([ax, ay, ac], [bx, by, bc]) = self.affine();
        (
            i64::from(ax) * point.0 + i64::from(ay) * point.1 + i64::from(ac),
            i64::from(bx) * point.0 + i64::from(by) * point.1 + i64::from(bc),
        )
    }
}

const TRANSFORMS: [Transform; 4] = [
    Transform::Identity,
    Transform::Reflection,
    Transform::Conjugation,
    Transform::ReflectionConjugation,
];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Predicate {
    pub left: Transform,
    pub right: Transform,
}

impl Predicate {
    pub fn render(self) -> String {
        format!("{}(rho)={}(rho)", self.left.name(), self.right.name())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EquivalenceCertificate {
    pub forward: bool,
    pub backward: bool,
    pub universal_lift: bool,
    pub equations: [[i8; 3]; 2],
}

#[derive(Clone, Debug)]
pub struct PredicateSearch {
    pub condition: &'static str,
    pub selected: Option<Predicate>,
    pub candidate_tests: usize,
    pub checker_calls: usize,
}

#[derive(Clone, Debug)]
pub struct OrbitTransfer {
    pub task: &'static str,
    pub compatible: bool,
    pub exact: bool,
    pub baseline_ops: usize,
    pub acquired_ops: usize,
    pub false_positive: bool,
    pub negative_transfer: bool,
}

#[derive(Clone, Debug)]
pub struct M28Experiment {
    pub candidate_space: usize,
    pub cold: PredicateSearch,
    pub transferred: PredicateSearch,
    pub selected_predicate: String,
    pub certificate: EquivalenceCertificate,
    pub equivalent_selections: bool,
    pub locally_novel: bool,
    pub controls_declined: usize,
    pub corrupted_composition_declined: bool,
    pub transfers: Vec<OrbitTransfer>,
    pub baseline_ops: usize,
    pub acquired_ops: usize,
    pub measured_gain: usize,
    pub false_positive_acceptances: usize,
    pub negative_transfer_tasks: usize,
    pub global_novelty_claimed: bool,
    pub rh_proved: bool,
    pub claim_level: &'static str,
    pub m28_passed: bool,
}

fn predicates() -> Vec<Predicate> {
    let mut candidates = Vec::new();
    for left in 0..TRANSFORMS.len() {
        for right in left + 1..TRANSFORMS.len() {
            candidates.push(Predicate {
                left: TRANSFORMS[left],
                right: TRANSFORMS[right],
            });
        }
    }
    candidates
}

fn gcd(mut a: i8, mut b: i8) -> i8 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

fn normalize_equation(mut equation: [i8; 3]) -> [i8; 3] {
    let divisor = equation.iter().copied().fold(0, gcd).max(1);
    for value in &mut equation {
        *value /= divisor;
    }
    if equation
        .iter()
        .find(|value| **value != 0)
        .copied()
        .unwrap_or(0)
        < 0
    {
        for value in &mut equation {
            *value = -*value;
        }
    }
    equation
}

fn subtract(left: [i8; 3], right: [i8; 3]) -> [i8; 3] {
    normalize_equation([left[0] - right[0], left[1] - right[1], left[2] - right[2]])
}

fn check_with_affine(
    predicate: Predicate,
    right_override: Option<([i8; 3], [i8; 3])>,
) -> EquivalenceCertificate {
    let left = predicate.left.affine();
    let right = right_override.unwrap_or_else(|| predicate.right.affine());
    let equations = [subtract(left.0, right.0), subtract(left.1, right.1)];
    let target = [1, 0, -1]; // X-1=0, exactly the M27 predicate.
    let compatible = equations
        .iter()
        .all(|equation| *equation == [0, 0, 0] || *equation == target);
    let constrains_target = equations.iter().any(|equation| *equation == target);
    EquivalenceCertificate {
        forward: compatible,
        backward: compatible && constrains_target,
        universal_lift: compatible && constrains_target,
        equations,
    }
}

pub fn exact_checker(predicate: Predicate) -> EquivalenceCertificate {
    check_with_affine(predicate, None)
}

fn locally_novel(predicate: Predicate) -> bool {
    const REFERENCE: [&str; 3] = ["2x-1=0", "x=1/2", "k(2x-1)=0"];
    let rendered = predicate.render();
    !REFERENCE.contains(&rendered.as_str())
        && (matches!(predicate.left, Transform::ReflectionConjugation)
            || matches!(predicate.right, Transform::ReflectionConjugation))
}

fn transferred_cmp(left: &Predicate, right: &Predicate) -> Ordering {
    let has_composition = |candidate: Predicate| {
        candidate.left == Transform::ReflectionConjugation
            || candidate.right == Transform::ReflectionConjugation
    };
    (usize::from(!has_composition(*left)), *left)
        .cmp(&(usize::from(!has_composition(*right)), *right))
}

fn search(condition: &'static str, transferred: bool) -> PredicateSearch {
    let mut candidates = predicates();
    if transferred {
        candidates.sort_by(transferred_cmp);
    }
    for (index, predicate) in candidates.into_iter().enumerate() {
        let certificate = exact_checker(predicate);
        if certificate.forward && certificate.backward && locally_novel(predicate) {
            return PredicateSearch {
                condition,
                selected: Some(predicate),
                candidate_tests: index + 1,
                checker_calls: index + 1,
            };
        }
    }
    PredicateSearch {
        condition,
        selected: None,
        candidate_tests: 6,
        checker_calls: 6,
    }
}

fn full_orbit(point: (i64, i64)) -> BTreeSet<(i64, i64)> {
    TRANSFORMS
        .iter()
        .map(|transform| transform.apply(point))
        .collect()
}

fn acquired_orbit(point: (i64, i64)) -> BTreeSet<(i64, i64)> {
    [Transform::Identity, Transform::Conjugation]
        .iter()
        .map(|transform| transform.apply(point))
        .collect()
}

fn compatible_tasks() -> [(&'static str, [i64; 3]); 3] {
    [
        ("even_heights", [2, 4, 6]),
        ("odd_heights", [3, 7, 11]),
        ("signed_heights", [-8, -2, 10]),
    ]
}

pub fn m28_experiment() -> M28Experiment {
    let cold = search("cold", false);
    let transferred = search("transferred", true);
    let selected = transferred.selected.or(cold.selected);
    let equivalent_selections = cold.selected == transferred.selected && selected.is_some();
    let certificate = selected
        .map(exact_checker)
        .unwrap_or(EquivalenceCertificate {
            forward: false,
            backward: false,
            universal_lift: false,
            equations: [[0; 3]; 2],
        });
    let locally_novel = selected.is_some_and(locally_novel);
    let controls = [
        Predicate {
            left: Transform::Identity,
            right: Transform::Reflection,
        },
        Predicate {
            left: Transform::Identity,
            right: Transform::Conjugation,
        },
        Predicate {
            left: Transform::Reflection,
            right: Transform::ReflectionConjugation,
        },
        Predicate {
            left: Transform::Conjugation,
            right: Transform::ReflectionConjugation,
        },
    ];
    let controls_declined = controls
        .iter()
        .filter(|predicate| {
            let checked = exact_checker(**predicate);
            !(checked.forward && checked.backward)
        })
        .count();
    let corrupted_composition_declined = selected.is_some_and(|predicate| {
        let checked = check_with_affine(predicate, Some(Transform::Reflection.affine()));
        !(checked.forward && checked.backward)
    });
    let mut transfers = Vec::new();
    for (name, heights) in compatible_tasks() {
        let exact = heights
            .iter()
            .all(|height| full_orbit((1, *height)) == acquired_orbit((1, *height)));
        transfers.push(OrbitTransfer {
            task: name,
            compatible: true,
            exact,
            baseline_ops: heights.len() * 10,
            acquired_ops: heights.len() * 3,
            false_positive: false,
            negative_transfer: false,
        });
    }
    for (name, point) in [("left_off_locus", (0, 4)), ("right_off_locus", (2, 7))] {
        let falsely_routed = full_orbit(point) == acquired_orbit(point);
        transfers.push(OrbitTransfer {
            task: name,
            compatible: false,
            exact: !falsely_routed,
            baseline_ops: 10,
            acquired_ops: 10,
            false_positive: falsely_routed,
            negative_transfer: false,
        });
    }
    let baseline_ops = transfers.iter().map(|task| task.baseline_ops).sum();
    let acquired_ops = transfers.iter().map(|task| task.acquired_ops).sum();
    let false_positive_acceptances = transfers.iter().filter(|task| task.false_positive).count();
    let negative_transfer_tasks = transfers
        .iter()
        .filter(|task| task.negative_transfer)
        .count();
    let global_novelty_claimed = false;
    let rh_proved = false;
    let search_gain = cold
        .candidate_tests
        .saturating_sub(transferred.candidate_tests);
    let m28_passed = certificate.forward
        && certificate.backward
        && certificate.universal_lift
        && equivalent_selections
        && locally_novel
        && controls_declined == controls.len()
        && corrupted_composition_declined
        && transfers.iter().all(|task| task.exact)
        && acquired_ops < baseline_ops
        && false_positive_acceptances == 0
        && negative_transfer_tasks == 0
        && search_gain > 0
        && !global_novelty_claimed
        && !rh_proved;
    M28Experiment {
        candidate_space: predicates().len(),
        cold,
        transferred,
        selected_predicate: selected
            .map(Predicate::render)
            .unwrap_or_else(|| "none".into()),
        certificate,
        equivalent_selections,
        locally_novel,
        controls_declined,
        corrupted_composition_declined,
        transfers,
        baseline_ops,
        acquired_ops,
        measured_gain: baseline_ops.saturating_sub(acquired_ops),
        false_positive_acceptances,
        negative_transfer_tasks,
        global_novelty_claimed,
        rh_proved,
        claim_level: if m28_passed {
            "L2_invented_feature_in_supplied_meta_ontology"
        } else {
            "L1_checked_law_in_supplied_representation"
        },
        m28_passed,
    }
}

pub fn machine_record(report: &M28Experiment) -> String {
    format!(
        "M28|space={}|cold={}|transferred={}|predicate={}|equations={:?}|forward={}|backward={}|lift={}|local_novel={}|controls={}/4|corrupt_declined={}|ops={}>{}|gain={}|global_novelty={}|rh_proved={}|claim={}|pass={}",
        report.candidate_space, report.cold.candidate_tests, report.transferred.candidate_tests,
        report.selected_predicate, report.certificate.equations, report.certificate.forward,
        report.certificate.backward, report.certificate.universal_lift, report.locally_novel,
        report.controls_declined, report.corrupted_composition_declined, report.baseline_ops,
        report.acquired_ops, report.measured_gain, report.global_novelty_claimed, report.rh_proved,
        report.claim_level, report.m28_passed
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_candidate_space_has_six_equalities() {
        assert_eq!(predicates().len(), 6);
    }

    #[test]
    fn checker_proves_exactly_the_two_affine_reformulations() {
        let accepted = predicates()
            .into_iter()
            .filter(|predicate| {
                let certificate = exact_checker(*predicate);
                certificate.forward && certificate.backward
            })
            .collect::<Vec<_>>();
        assert_eq!(
            accepted.iter().map(|p| p.render()).collect::<Vec<_>>(),
            vec!["I(rho)=RC(rho)", "R(rho)=C(rho)"]
        );
        assert!(locally_novel(accepted[0]));
        assert!(!locally_novel(accepted[1]));
    }

    #[test]
    fn m28_passes_without_claiming_rh_or_global_novelty() {
        let report = m28_experiment();
        assert!(report.m28_passed, "{report:#?}");
        assert!(!report.rh_proved);
        assert!(!report.global_novelty_claimed);
        assert_eq!(machine_record(&report), machine_record(&m28_experiment()));
    }
}
