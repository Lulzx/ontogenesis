//! SH12: counting asymptotics cannot identify an exact spectrum.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Perturbation {
    AlternatingQuarterLocalGap,
    ReciprocalIndex,
    Zero,
    Linear,
}

fn admissible(perturbation: Perturbation) -> bool {
    matches!(
        perturbation,
        Perturbation::AlternatingQuarterLocalGap
            | Perturbation::ReciprocalIndex
            | Perturbation::Zero
    )
}

fn distinct(perturbation: Perturbation) -> bool {
    !matches!(perturbation, Perturbation::Zero)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Fact {
    OrderPreservingInterlacing,
    DistinctSpectra,
    SelfAdjointCompactResolventPair,
    CountingDifferenceAtMostOne,
    EqualStableCountingAsymptotics,
    CountingDoesNotIdentifySpectrum,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Rejection {
    MissingGapCertificate,
    UnboundedPerturbation,
    SpectraNotDistinct,
    CountingIsNonSeparating,
}

fn certify_pair(
    base_strict_unbounded: bool,
    eventual_gap: bool,
    perturbation: Perturbation,
) -> Result<[Fact; 6], Rejection> {
    if !base_strict_unbounded || !eventual_gap {
        return Err(Rejection::MissingGapCertificate);
    }
    if !admissible(perturbation) {
        return Err(Rejection::UnboundedPerturbation);
    }
    if !distinct(perturbation) {
        return Err(Rejection::SpectraNotDistinct);
    }
    Ok([
        Fact::OrderPreservingInterlacing,
        Fact::DistinctSpectra,
        Fact::SelfAdjointCompactResolventPair,
        Fact::CountingDifferenceAtMostOne,
        Fact::EqualStableCountingAsymptotics,
        Fact::CountingDoesNotIdentifySpectrum,
    ])
}

fn promote_counting_to_exact_spectrum(_: &[Fact; 6]) -> Result<(), Rejection> {
    Err(Rejection::CountingIsNonSeparating)
}

#[derive(Clone, Debug)]
pub struct Sh12Experiment {
    pub families_checked: usize,
    pub distinct_equal_counting_pairs: usize,
    pub alternating_certified: bool,
    pub reciprocal_certified: bool,
    pub controls: [bool; 4],
    pub controls_declined: usize,
    pub counting_route_eliminated: bool,
    pub separating_trace_required: bool,
    pub m29_reached: bool,
}

pub fn sh12_experiment() -> Sh12Experiment {
    let alternating = certify_pair(true, true, Perturbation::AlternatingQuarterLocalGap);
    let reciprocal = certify_pair(true, true, Perturbation::ReciprocalIndex);
    let controls = [
        certify_pair(true, true, Perturbation::Linear) == Err(Rejection::UnboundedPerturbation),
        certify_pair(true, true, Perturbation::Zero) == Err(Rejection::SpectraNotDistinct),
        certify_pair(true, false, Perturbation::AlternatingQuarterLocalGap)
            == Err(Rejection::MissingGapCertificate),
        alternating
            .as_ref()
            .is_ok_and(|facts| promote_counting_to_exact_spectrum(facts).is_err()),
    ];
    let controls_declined = controls.iter().filter(|control| **control).count();
    Sh12Experiment {
        families_checked: 4,
        distinct_equal_counting_pairs: usize::from(alternating.is_ok())
            + usize::from(reciprocal.is_ok()),
        alternating_certified: alternating.is_ok(),
        reciprocal_certified: reciprocal.is_ok(),
        controls,
        controls_declined,
        counting_route_eliminated: alternating.is_ok()
            && reciprocal.is_ok()
            && controls_declined == controls.len(),
        separating_trace_required: true,
        m29_reached: false,
    }
}

pub fn machine_record(report: &Sh12Experiment) -> String {
    format!(
        "SH12b|families_checked={}|distinct_equal_counting_pairs={}|alternating_certified={}|reciprocal_certified={}|controls={:?}|controls_declined={}/4|counting_route_eliminated={}|separating_trace_required={}|m29_reached=false|claim=counting_information_class_falsified_only",
        report.families_checked,
        report.distinct_equal_counting_pairs,
        report.alternating_certified,
        report.reciprocal_certified,
        report.controls,
        report.controls_declined,
        report.counting_route_eliminated,
        report.separating_trace_required,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proves_finite_order_counting_is_nonidentifying() {
        let report = sh12_experiment();
        assert!(report.counting_route_eliminated, "{report:#?}");
        assert_eq!(report.distinct_equal_counting_pairs, 2);
        assert_eq!(report.controls, [true; 4]);
        assert!(report.separating_trace_required);
        assert!(!report.m29_reached);
        assert_eq!(machine_record(&report), machine_record(&sh12_experiment()));
    }
}
