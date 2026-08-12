//! SH1: executable causal diagnosis of the M29-to-real-RH stall.

use crate::real_rh_ontogenesis::{
    ablation_run, m30a_experiment, AblationResult, Inference, Statement,
};
use crate::rh_making_surrogate_world::m29b_experiment;

#[derive(Clone, Debug)]
pub struct DebtAblation {
    pub debt: &'static str,
    pub mechanism: &'static str,
    pub result: AblationResult,
    pub load_bearing: bool,
}

#[derive(Clone, Debug)]
pub struct StallDiagnosis {
    pub last_success_replayed: bool,
    pub first_failure_replayed: bool,
    pub ablations: Vec<DebtAblation>,
    pub load_bearing: Vec<&'static str>,
    pub non_load_bearing: Vec<&'static str>,
    pub first_self_hosting_target: &'static str,
    pub diagnosis_passed: bool,
    pub m29_reached: bool,
    pub claim: &'static str,
}

fn single(debt: &'static str, mechanism: &'static str, statement: Statement) -> DebtAblation {
    let result = ablation_run(mechanism, &[statement], None);
    let load_bearing = !result.frontier_survives;
    DebtAblation {
        debt,
        mechanism,
        result,
        load_bearing,
    }
}

pub fn sh1_diagnosis() -> StallDiagnosis {
    let last_success_replayed = m29b_experiment().surrogate_passed;
    let m30 = m30a_experiment();
    let first_failure_replayed = m30.run_completed && !m30.m30_reached;
    let mut ablations = vec![
        single("D17", "finite_zero_evidence", Statement::FiniteZeroEvidence),
        single("D20", "even_quartic_forcing", Statement::EvenQuarticForcing),
        single(
            "D15",
            "euler_product_semantics",
            Statement::EulerProductHalfPlane,
        ),
        single(
            "D16",
            "xi_functional_equation",
            Statement::XiFunctionalEquation,
        ),
        single("D16", "conjugation_closure", Statement::ConjugationClosure),
        single(
            "D19",
            "reflection_equivalence",
            Statement::ReflectionEquivalence,
        ),
        single(
            "D21-open",
            "spectral_correspondence",
            Statement::SpectralCorrespondence,
        ),
        single("D21-open", "self_adjointness", Statement::SelfAdjointness),
    ];
    let joint = ablation_run(
        "joint_spectral_bridge",
        &[
            Statement::SpectralCorrespondence,
            Statement::SelfAdjointness,
        ],
        None,
    );
    ablations.push(DebtAblation {
        debt: "D21-open",
        mechanism: "joint_spectral_bridge",
        load_bearing: !joint.frontier_survives,
        result: joint,
    });
    let inference = ablation_run(
        "spectral_forcing_rule",
        &[],
        Some(Inference::SpectralForcing),
    );
    ablations.push(DebtAblation {
        debt: "D21",
        mechanism: "spectral_forcing_rule",
        load_bearing: !inference.frontier_survives,
        result: inference,
    });
    let load_bearing = ablations
        .iter()
        .filter(|item| item.load_bearing)
        .map(|item| item.mechanism)
        .collect::<Vec<_>>();
    let non_load_bearing = ablations
        .iter()
        .filter(|item| !item.load_bearing)
        .map(|item| item.mechanism)
        .collect::<Vec<_>>();
    let diagnosis_passed = last_success_replayed
        && first_failure_replayed
        && ablations.len() == 10
        && load_bearing.contains(&"spectral_correspondence")
        && load_bearing.contains(&"self_adjointness")
        && non_load_bearing.contains(&"finite_zero_evidence")
        && non_load_bearing.contains(&"even_quartic_forcing")
        && ablations
            .iter()
            .all(|item| !item.result.proof_found && !item.result.reduction_found);
    StallDiagnosis {
        last_success_replayed,
        first_failure_replayed,
        ablations,
        load_bearing,
        non_load_bearing,
        first_self_hosting_target:
            "generic spectral-correspondence generator with self-adjointness obligations",
        diagnosis_passed,
        m29_reached: false,
        claim: "bounded_causal_stall_diagnosis",
    }
}

pub fn machine_record(report: &StallDiagnosis) -> String {
    let ablations = report
        .ablations
        .iter()
        .map(|item| {
            format!(
                "{}:{}/{:?}/{}",
                item.mechanism,
                item.result.frontier_survives,
                item.result.frontier_rank,
                item.result.programs_enumerated
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "SH1|last_success={}|first_failure={}|ablations={}|load_bearing={:?}|non_load_bearing={:?}|target={}|diagnosis_pass={}|m29_reached={}|claim={}",
        report.last_success_replayed,
        report.first_failure_replayed,
        ablations,
        report.load_bearing,
        report.non_load_bearing,
        report.first_self_hosting_target,
        report.diagnosis_passed,
        report.m29_reached,
        report.claim
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replays_the_frozen_success_failure_pair() {
        let report = sh1_diagnosis();
        assert!(report.last_success_replayed);
        assert!(report.first_failure_replayed);
    }

    #[test]
    fn isolates_the_spectral_bridge_as_load_bearing() {
        let report = sh1_diagnosis();
        assert!(report.diagnosis_passed, "{report:#?}");
        assert!(report.load_bearing.contains(&"spectral_correspondence"));
        assert!(report.load_bearing.contains(&"self_adjointness"));
        assert!(report.non_load_bearing.contains(&"reflection_equivalence"));
        assert!(!report.m29_reached);
    }

    #[test]
    fn record_is_deterministic() {
        assert_eq!(
            machine_record(&sh1_diagnosis()),
            machine_record(&sh1_diagnosis())
        );
    }
}
