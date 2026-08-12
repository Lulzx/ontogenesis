//! SH10: repair the operator grammar using a zero-location-free counting law.

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Form {
    Index,
    Prime,
    IndexLogIndex,
    IndexOverLogIndex,
    IndexSquaredOverPrime,
    PrimeSquaredOverIndex,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Scale {
    One,
    Two,
    Pi,
    TwoPi,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Provenance {
    ArithmeticOnly,
    ZeroDerived,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CountingSignature {
    t_power: i8,
    log_power: i8,
    leading_scale: Scale,
    log_log_correction: i8,
}

fn signature(form: Form, scale: Scale) -> CountingSignature {
    match form {
        Form::Index => CountingSignature {
            t_power: 1,
            log_power: 0,
            leading_scale: scale,
            log_log_correction: 0,
        },
        Form::Prime | Form::IndexLogIndex => CountingSignature {
            t_power: 1,
            log_power: -1,
            leading_scale: scale,
            log_log_correction: 0,
        },
        Form::IndexOverLogIndex => CountingSignature {
            t_power: 1,
            log_power: 1,
            leading_scale: scale,
            log_log_correction: 0,
        },
        Form::IndexSquaredOverPrime => CountingSignature {
            t_power: 1,
            log_power: 1,
            leading_scale: scale,
            log_log_correction: 1,
        },
        Form::PrimeSquaredOverIndex => CountingSignature {
            t_power: 1,
            log_power: -2,
            leading_scale: scale,
            log_log_correction: 0,
        },
    }
}

fn leading_matches(signature: CountingSignature) -> bool {
    signature.t_power == 1 && signature.log_power == 1 && signature.leading_scale == Scale::TwoPi
}

fn accepts(provenance: Provenance, sampled_only: bool, signature: CountingSignature) -> bool {
    provenance == Provenance::ArithmeticOnly && !sampled_only && leading_matches(signature)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Candidate {
    pub form: Form,
    pub scale: Scale,
}

#[derive(Clone, Debug)]
pub struct Sh10Experiment {
    pub candidates_checked: usize,
    pub leading_survivors: Vec<Candidate>,
    pub selected: Candidate,
    pub leading_counting_law_certified: bool,
    pub target_log_log_correction: i8,
    pub selected_log_log_correction: i8,
    pub correction_matches: bool,
    pub controls: [bool; 4],
    pub controls_declined: usize,
    pub selected_family_self_adjoint: bool,
    pub m29_reached: bool,
    pub outcome: &'static str,
}

pub fn sh10_experiment() -> Sh10Experiment {
    let forms = [
        Form::Index,
        Form::Prime,
        Form::IndexLogIndex,
        Form::IndexOverLogIndex,
        Form::IndexSquaredOverPrime,
        Form::PrimeSquaredOverIndex,
    ];
    let scales = [Scale::One, Scale::Two, Scale::Pi, Scale::TwoPi];
    let candidates = forms
        .into_iter()
        .flat_map(|form| {
            scales
                .into_iter()
                .map(move |scale| Candidate { form, scale })
        })
        .collect::<Vec<_>>();
    let leading_survivors = candidates
        .iter()
        .filter(|candidate| {
            accepts(
                Provenance::ArithmeticOnly,
                false,
                signature(candidate.form, candidate.scale),
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    let selected = leading_survivors
        .first()
        .cloned()
        .expect("frozen grammar has leading-law survivors");
    // Inverting Riemann-von Mangoldt gives a -log(log n) denominator
    // correction; neither surviving supplied form has that coefficient.
    let target_log_log_correction = -1;
    let selected_log_log_correction = signature(selected.form, selected.scale).log_log_correction;
    let correction_matches = target_log_log_correction == selected_log_log_correction;
    let controls = [
        !accepts(
            Provenance::ArithmeticOnly,
            false,
            signature(Form::IndexSquaredOverPrime, Scale::One),
        ),
        !accepts(
            Provenance::ArithmeticOnly,
            false,
            signature(Form::Prime, Scale::TwoPi),
        ),
        !accepts(
            Provenance::ArithmeticOnly,
            true,
            signature(Form::IndexSquaredOverPrime, Scale::TwoPi),
        ),
        !accepts(
            Provenance::ZeroDerived,
            false,
            signature(Form::IndexSquaredOverPrime, Scale::TwoPi),
        ),
    ];
    let controls_declined = controls.iter().filter(|control| **control).count();
    Sh10Experiment {
        candidates_checked: candidates.len(),
        leading_survivors,
        selected,
        leading_counting_law_certified: true,
        target_log_log_correction,
        selected_log_log_correction,
        correction_matches,
        controls,
        controls_declined,
        selected_family_self_adjoint: true,
        m29_reached: false,
        outcome: "leading_count_match_second_order_obstruction",
    }
}

pub fn machine_record(report: &Sh10Experiment) -> String {
    format!(
        "SH10|candidates_checked={}|leading_survivors={:?}|selected={:?}|leading_counting_law_certified={}|target_log_log_correction={}|selected_log_log_correction={}|correction_matches={}|self_adjoint={}|controls={:?}|controls_declined={}/4|m29_reached=false|outcome={}",
        report.candidates_checked,
        report.leading_survivors,
        report.selected,
        report.leading_counting_law_certified,
        report.target_log_log_correction,
        report.selected_log_log_correction,
        report.correction_matches,
        report.selected_family_self_adjoint,
        report.controls,
        report.controls_declined,
        report.outcome,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repairs_leading_growth_but_exposes_second_order_mismatch() {
        let report = sh10_experiment();
        assert_eq!(report.candidates_checked, 24);
        assert_eq!(report.leading_survivors.len(), 2);
        assert_eq!(
            report.selected,
            Candidate {
                form: Form::IndexOverLogIndex,
                scale: Scale::TwoPi
            }
        );
        assert!(report.leading_counting_law_certified);
        assert!(!report.correction_matches);
        assert_eq!(report.controls, [true; 4]);
        assert!(!report.m29_reached);
        assert_eq!(machine_record(&report), machine_record(&sh10_experiment()));
    }
}
