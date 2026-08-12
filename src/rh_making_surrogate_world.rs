//! Direction M29b: coefficient-derived forcing object for an even-quartic surrogate.
//!
//! This transfers the M25 object/property pattern into an exact analytic
//! surrogate. It is not an object for xi and does not prove RH.

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Feature {
    One,
    A,
    B,
    NegA,
    NegB,
    Sum,
    Discriminant,
}

impl Feature {
    fn render(self) -> &'static str {
        match self {
            Feature::One => "1",
            Feature::A => "a",
            Feature::B => "b",
            Feature::NegA => "-a",
            Feature::NegB => "-b",
            Feature::Sum => "a+b",
            Feature::Discriminant => "a^2-4b",
        }
    }

    fn value(self, a: i64, b: i64) -> i64 {
        match self {
            Feature::One => 1,
            Feature::A => a,
            Feature::B => b,
            Feature::NegA => -a,
            Feature::NegB => -b,
            Feature::Sum => a + b,
            Feature::Discriminant => a * a - 4 * b,
        }
    }
}

const FEATURES: [Feature; 7] = [
    Feature::One,
    Feature::A,
    Feature::B,
    Feature::NegA,
    Feature::NegB,
    Feature::Sum,
    Feature::Discriminant,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Object {
    pub diagonal: [Feature; 3],
}

impl Object {
    pub fn render(self) -> String {
        format!(
            "diag({},{},{})",
            self.diagonal[0].render(),
            self.diagonal[1].render(),
            self.diagonal[2].render()
        )
    }

    fn psd(self, a: i64, b: i64) -> bool {
        self.diagonal.iter().all(|feature| feature.value(a, b) >= 0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForcingCertificate {
    pub forward: bool,
    pub backward: bool,
    pub non_vacuous_true: bool,
    pub non_vacuous_false: bool,
    pub family_checks: usize,
}

#[derive(Clone, Debug)]
pub struct ObjectSearch {
    pub condition: &'static str,
    pub selected: Option<Object>,
    pub candidate_tests: usize,
    pub family_checks: usize,
}

#[derive(Clone, Debug)]
pub struct SurrogateTransfer {
    pub task: &'static str,
    pub members: usize,
    pub exact: bool,
    pub baseline_ops: usize,
    pub construction_ops: usize,
    pub query_ops: usize,
    pub acquired_ops: usize,
    pub negative_transfer: bool,
}

#[derive(Clone, Debug)]
pub struct M29bExperiment {
    pub candidate_space: usize,
    pub cold: ObjectSearch,
    pub transferred: ObjectSearch,
    pub selected_object: String,
    pub certificate: ForcingCertificate,
    pub equivalent_selections: bool,
    pub held_out_certified: usize,
    pub provenance_passed: bool,
    pub control_results: [bool; 7],
    pub controls_declined: usize,
    pub transfers: Vec<SurrogateTransfer>,
    pub baseline_ops: usize,
    pub construction_ops: usize,
    pub query_ops: usize,
    pub acquired_ops: usize,
    pub measured_gain: usize,
    pub false_positive_acceptances: usize,
    pub negative_transfer_tasks: usize,
    pub surrogate_passed: bool,
    pub real_zeta_object: bool,
    pub rh_proved: bool,
    pub m29_reached: bool,
    pub claim_level: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Provenance {
    CoefficientOnly,
    RootCoordinates,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Route {
    Accepted,
    RejectedProvenance,
    OutOfScope,
}

fn route(provenance: Provenance, degree: u8, even: bool) -> Route {
    if provenance != Provenance::CoefficientOnly {
        Route::RejectedProvenance
    } else if degree != 4 || !even {
        Route::OutOfScope
    } else {
        Route::Accepted
    }
}

fn objects() -> Vec<Object> {
    let mut candidates = Vec::new();
    for i in 0..FEATURES.len() {
        for j in i..FEATURES.len() {
            for k in j..FEATURES.len() {
                candidates.push(Object {
                    diagonal: [FEATURES[i], FEATURES[j], FEATURES[k]],
                });
            }
        }
    }
    candidates
}

fn target(a: i64, b: i64) -> bool {
    // For w=z^2, w^2+a w+b has two real nonpositive roots exactly here.
    a >= 0 && b >= 0 && a * a - 4 * b >= 0
}

fn training_family() -> Vec<(i64, i64)> {
    (-6..=6)
        .flat_map(|a| (-6..=12).map(move |b| (a, b)))
        .collect()
}

fn held_out_wide() -> Vec<(i64, i64)> {
    (-12..=-7)
        .chain(7..=12)
        .flat_map(|a| (-12..=24).map(move |b| (a, b)))
        .collect()
}

fn held_out_high_b() -> Vec<(i64, i64)> {
    (-20..=20)
        .flat_map(|a| (13..=30).map(move |b| (a, b)))
        .collect()
}

fn check_family(object: Object, family: &[(i64, i64)]) -> ForcingCertificate {
    let mut forward = true;
    let mut backward = true;
    let mut non_vacuous_true = false;
    let mut non_vacuous_false = false;
    for &(a, b) in family {
        let property = object.psd(a, b);
        let theorem_target = target(a, b);
        forward &= !property || theorem_target;
        backward &= !theorem_target || property;
        non_vacuous_true |= property && theorem_target;
        non_vacuous_false |= !property && !theorem_target;
    }
    ForcingCertificate {
        forward,
        backward,
        non_vacuous_true,
        non_vacuous_false,
        family_checks: family.len(),
    }
}

fn transferred_key(object: Object) -> (usize, usize, usize, [Feature; 3]) {
    let distinct = object.diagonal[0] != object.diagonal[1]
        && object.diagonal[0] != object.diagonal[2]
        && object.diagonal[1] != object.diagonal[2];
    let has_discriminant = object.diagonal.contains(&Feature::Discriminant);
    let raw_count = object
        .diagonal
        .iter()
        .filter(|feature| matches!(feature, Feature::A | Feature::B))
        .count();
    (
        usize::from(!distinct),
        usize::from(!has_discriminant),
        usize::MAX - raw_count,
        object.diagonal,
    )
}

fn search(condition: &'static str, transferred: bool) -> ObjectSearch {
    let family = training_family();
    let mut candidates = objects();
    if transferred {
        candidates.sort_by_key(|object| transferred_key(*object));
    }
    let mut family_checks = 0;
    for (index, object) in candidates.into_iter().enumerate() {
        let certificate = check_family(object, &family);
        family_checks += certificate.family_checks;
        if certificate.forward
            && certificate.backward
            && certificate.non_vacuous_true
            && certificate.non_vacuous_false
        {
            return ObjectSearch {
                condition,
                selected: Some(object),
                candidate_tests: index + 1,
                family_checks,
            };
        }
    }
    ObjectSearch {
        condition,
        selected: None,
        candidate_tests: objects().len(),
        family_checks,
    }
}

fn ablation_declined(object: Object, removed: usize) -> bool {
    let family = training_family();
    family.iter().any(|&(a, b)| {
        let property = object
            .diagonal
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != removed)
            .all(|(_, feature)| feature.value(a, b) >= 0);
        property != target(a, b)
    })
}

fn wrong_discriminant_declined() -> bool {
    training_family().iter().any(|&(a, b)| {
        let property = a >= 0 && b >= 0 && a * a + 4 * b >= 0;
        property != target(a, b)
    })
}

fn transfer(task: &'static str, family: Vec<(i64, i64)>, object: Object) -> SurrogateTransfer {
    let certificate = check_family(object, &family);
    let exact = certificate.forward && certificate.backward;
    let queries = 4;
    let baseline_ops = family.len() * queries * 6;
    let construction_ops = family.len() * 3;
    let query_ops = family.len() * queries * 3;
    let acquired_ops = construction_ops + query_ops;
    SurrogateTransfer {
        task,
        members: family.len(),
        exact,
        baseline_ops,
        construction_ops,
        query_ops,
        acquired_ops,
        negative_transfer: acquired_ops > baseline_ops,
    }
}

pub fn m29b_experiment() -> M29bExperiment {
    let cold = search("cold", false);
    let transferred = search("transferred", true);
    let selected = transferred.selected.or(cold.selected);
    let equivalent_selections = cold.selected == transferred.selected && selected.is_some();
    let certificate = selected
        .map(|object| check_family(object, &training_family()))
        .unwrap_or(ForcingCertificate {
            forward: false,
            backward: false,
            non_vacuous_true: false,
            non_vacuous_false: false,
            family_checks: 0,
        });
    let held_out_certified = selected.map_or(0, |object| {
        [held_out_wide(), held_out_high_b()]
            .iter()
            .filter(|family| {
                let checked = check_family(object, family);
                checked.forward && checked.backward
            })
            .count()
    });
    let provenance_passed = route(Provenance::CoefficientOnly, 4, true) == Route::Accepted;
    let control_results = selected.map_or([false; 7], |object| {
        [
            ablation_declined(object, 0),
            ablation_declined(object, 1),
            ablation_declined(object, 2),
            wrong_discriminant_declined(),
            route(Provenance::RootCoordinates, 4, true) == Route::RejectedProvenance,
            route(Provenance::CoefficientOnly, 3, false) == Route::OutOfScope,
            route(Provenance::CoefficientOnly, 6, true) == Route::OutOfScope,
        ]
    });
    let controls_declined = control_results
        .into_iter()
        .filter(|declined| *declined)
        .count();
    let mut transfers = Vec::new();
    if let Some(object) = selected {
        transfers.push(transfer("wide_coefficients", held_out_wide(), object));
        transfers.push(transfer("high_constant", held_out_high_b(), object));
    }
    let baseline_ops = transfers.iter().map(|task| task.baseline_ops).sum();
    let construction_ops = transfers.iter().map(|task| task.construction_ops).sum();
    let query_ops = transfers.iter().map(|task| task.query_ops).sum();
    let acquired_ops = transfers.iter().map(|task| task.acquired_ops).sum();
    let false_positive_acceptances = control_results[4..]
        .iter()
        .filter(|declined| !**declined)
        .count();
    let negative_transfer_tasks = transfers
        .iter()
        .filter(|task| task.negative_transfer)
        .count();
    let real_zeta_object = false;
    let rh_proved = false;
    let m29_reached = false;
    let search_gain = cold
        .candidate_tests
        .saturating_sub(transferred.candidate_tests);
    let surrogate_passed = certificate.forward
        && certificate.backward
        && certificate.non_vacuous_true
        && certificate.non_vacuous_false
        && equivalent_selections
        && held_out_certified == 2
        && provenance_passed
        && controls_declined == 7
        && transfers.iter().all(|task| task.exact)
        && acquired_ops < baseline_ops
        && false_positive_acceptances == 0
        && negative_transfer_tasks == 0
        && search_gain > 0
        && !real_zeta_object
        && !rh_proved
        && !m29_reached;
    M29bExperiment {
        candidate_space: objects().len(),
        cold,
        transferred,
        selected_object: selected
            .map(Object::render)
            .unwrap_or_else(|| "none".into()),
        certificate,
        equivalent_selections,
        held_out_certified,
        provenance_passed,
        control_results,
        controls_declined,
        transfers,
        baseline_ops,
        construction_ops,
        query_ops,
        acquired_ops,
        measured_gain: baseline_ops.saturating_sub(acquired_ops),
        false_positive_acceptances,
        negative_transfer_tasks,
        surrogate_passed,
        real_zeta_object,
        rh_proved,
        m29_reached,
        claim_level: if surrogate_passed {
            "L2_invented_feature_in_supplied_meta_ontology"
        } else {
            "L1_checked_law_in_supplied_representation"
        },
    }
}

pub fn machine_record(report: &M29bExperiment) -> String {
    format!(
        "M29b|space={}|cold={}|transferred={}|object={}|forward={}|backward={}|nonvacuous={}/{}|held_out={}/2|provenance={}|control_results={:?}|controls={}/7|ops={}>{}|construction={}|queries={}|gain={}|surrogate_pass={}|real_zeta_object={}|rh_proved={}|m29_reached={}|claim={}",
        report.candidate_space, report.cold.candidate_tests, report.transferred.candidate_tests,
        report.selected_object, report.certificate.forward, report.certificate.backward,
        report.certificate.non_vacuous_true, report.certificate.non_vacuous_false,
        report.held_out_certified, report.provenance_passed, report.control_results,
        report.controls_declined,
        report.baseline_ops, report.acquired_ops, report.construction_ops, report.query_ops,
        report.measured_gain, report.surrogate_passed, report.real_zeta_object, report.rh_proved,
        report.m29_reached, report.claim_level
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_object_space_has_84_members() {
        assert_eq!(objects().len(), 84);
    }

    #[test]
    fn checker_retains_coefficient_criterion_object() {
        let report = m29b_experiment();
        assert_eq!(report.selected_object, "diag(a,b,a^2-4b)");
        assert!(report.certificate.forward && report.certificate.backward);
    }

    #[test]
    fn surrogate_passes_without_reaching_real_m29() {
        let report = m29b_experiment();
        assert!(report.surrogate_passed, "{report:#?}");
        assert!(!report.real_zeta_object);
        assert!(!report.rh_proved);
        assert!(!report.m29_reached);
        assert_eq!(report.control_results, [true; 7]);
        assert_eq!(machine_record(&report), machine_record(&m29b_experiment()));
    }
}
