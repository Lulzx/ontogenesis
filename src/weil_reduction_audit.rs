//! M29g: audit the `P ==> RH` reduction, exposing its two theorem
//! obligations.
//!
//! The M29c encoding collapsed the chain into `P ==> GnsSelfAdjoint` and
//! `P + separating ==> SpectralCorrespondence`. This module re-expands the
//! chain with every premise explicit and checks that the reduction is valid
//! exactly when two theorems are supplied: essential self-adjointness (ES)
//! and spectral correspondence (SC). `P` alone forces neither.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Provenance {
    ArithmeticOnly,
    ZeroDerived,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Fact {
    WeilFunctional {
        pole: bool,
        archimedean: bool,
        prime: bool,
    },
    PositiveOnSquares {
        provenance: Provenance,
    },
    EssentialSelfAdjointness,
    SpectralCorrespondence,
    SeparatingAlgebra,
    FiniteClass,
    GnsHilbertSpace,
    SelfAdjointOperator,
    RiemannHypothesis,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Rejection {
    MissingPositivity,
    MissingEssentialSelfAdjointness,
    MissingCorrespondence,
    IncompleteFunctional,
    ForbiddenProvenance,
}

fn full_functional() -> Fact {
    Fact::WeilFunctional {
        pole: true,
        archimedean: true,
        prime: true,
    }
}

/// GNS: a positive functional on the commutative star algebra yields the
/// GNS Hilbert space and cyclic vector. This is unconditional given `P`;
/// it does not produce a self-adjoint operator.
fn build_gns(positivity: &Fact, functional: &Fact) -> Result<Fact, Rejection> {
    match (positivity, functional) {
        (
            Fact::PositiveOnSquares {
                provenance: Provenance::ZeroDerived,
            },
            _,
        ) => Err(Rejection::ForbiddenProvenance),
        (
            Fact::PositiveOnSquares {
                provenance: Provenance::ArithmeticOnly,
            },
            Fact::WeilFunctional {
                pole: true,
                archimedean: true,
                prime: true,
            },
        ) => Ok(Fact::GnsHilbertSpace),
        (_, Fact::WeilFunctional { prime: false, .. }) => Err(Rejection::IncompleteFunctional),
        _ => Err(Rejection::MissingPositivity),
    }
}

/// The coordinate is a symmetric operator on the GNS domain, but a
/// self-adjoint closure exists only with essential self-adjointness (a
/// determinacy/density theorem). `GnsHilbertSpace` alone is not enough.
fn self_adjoint(gns: &Fact, esa: &Fact) -> Result<Fact, Rejection> {
    match (gns, esa) {
        (Fact::GnsHilbertSpace, Fact::EssentialSelfAdjointness) => Ok(Fact::SelfAdjointOperator),
        (Fact::GnsHilbertSpace, _) => Err(Rejection::MissingEssentialSelfAdjointness),
        _ => Err(Rejection::MissingPositivity),
    }
}

/// Spectral correspondence: the self-adjoint operator's spectral measure is
/// the explicit formula's zero measure, so its spectrum is exactly the
/// nontrivial zeros. This is a separate theorem from `P`, and it holds only
/// on a separating class (the algebra must separate the spectrum).
fn establish_correspondence(sc: &Fact, class: &Fact) -> Result<Fact, Rejection> {
    match (sc, class) {
        (Fact::SpectralCorrespondence, Fact::SeparatingAlgebra) => Ok(Fact::SpectralCorrespondence),
        (Fact::SpectralCorrespondence, Fact::FiniteClass) => Err(Rejection::MissingCorrespondence),
        _ => Err(Rejection::MissingCorrespondence),
    }
}

fn force(self_adjoint: &Fact, correspondence: &Fact) -> Result<Fact, Rejection> {
    match (self_adjoint, correspondence) {
        (Fact::SelfAdjointOperator, Fact::SpectralCorrespondence) => Ok(Fact::RiemannHypothesis),
        (Fact::SelfAdjointOperator, _) => Err(Rejection::MissingCorrespondence),
        _ => Err(Rejection::MissingPositivity),
    }
}

fn positivity() -> Fact {
    Fact::PositiveOnSquares {
        provenance: Provenance::ArithmeticOnly,
    }
}

/// The full, explicit reduction `P ∧ ES ∧ SC ==> RH` (on the separating class).
fn p_and_es_and_sc_forces_rh() -> bool {
    let gns = build_gns(&positivity(), &full_functional()).ok();
    let self_adjoint = gns
        .as_ref()
        .and_then(|g| self_adjoint(g, &Fact::EssentialSelfAdjointness).ok());
    let correspondence =
        establish_correspondence(&Fact::SpectralCorrespondence, &Fact::SeparatingAlgebra).ok();
    matches!(
        (self_adjoint, correspondence),
        (Some(self_adjoint), Some(correspondence))
            if force(&self_adjoint, &correspondence) == Ok(Fact::RiemannHypothesis)
    )
}

/// `P` alone produces a GNS space but no self-adjoint operator.
fn p_alone_forces_self_adjoint() -> bool {
    let gns = build_gns(&positivity(), &full_functional()).ok();
    gns.as_ref()
        .is_some_and(|g| self_adjoint(g, &Fact::WeilFunctional { pole: true, archimedean: true, prime: true }).is_ok())
}

/// `P` alone produces no spectral correspondence.
fn p_alone_forces_correspondence() -> bool {
    let gns = build_gns(&positivity(), &full_functional()).ok();
    let self_adjoint = gns.as_ref().and_then(|g| self_adjoint(g, &Fact::EssentialSelfAdjointness).ok());
    self_adjoint
        .as_ref()
        .is_some_and(|s| force(s, &Fact::GnsHilbertSpace).is_ok())
}

fn controls() -> [bool; 6] {
    [
        // no P: GNS fails
        build_gns(&Fact::WeilFunctional { pole: true, archimedean: true, prime: true }, &full_functional())
            == Err(Rejection::MissingPositivity),
        // zero-derived P: forbidden
        build_gns(
            &Fact::PositiveOnSquares { provenance: Provenance::ZeroDerived },
            &full_functional(),
        ) == Err(Rejection::ForbiddenProvenance),
        // incomplete functional: rejected
        build_gns(
            &positivity(),
            &Fact::WeilFunctional { pole: true, archimedean: true, prime: false },
        ) == Err(Rejection::IncompleteFunctional),
        // P but no ES: no self-adjoint operator
        self_adjoint(&Fact::GnsHilbertSpace, &Fact::GnsHilbertSpace)
            == Err(Rejection::MissingEssentialSelfAdjointness),
        // self-adjoint but no SC: no RH
        force(&Fact::SelfAdjointOperator, &Fact::SelfAdjointOperator)
            == Err(Rejection::MissingCorrespondence),
        // finite (non-separating) class cannot supply SC
        establish_correspondence(&Fact::SpectralCorrespondence, &Fact::FiniteClass)
            == Err(Rejection::MissingCorrespondence),
    ]
}

#[derive(Clone, Debug)]
pub struct M29gExperiment {
    pub reduction_valid_with_p_es_sc: bool,
    pub p_alone_forces_self_adjoint: bool,
    pub p_alone_forces_correspondence: bool,
    pub hidden_premises: [&'static str; 2],
    pub controls: [bool; 6],
    pub controls_declined: usize,
    pub m29_reached: bool,
}

pub fn m29g_experiment() -> M29gExperiment {
    let controls = controls();
    M29gExperiment {
        reduction_valid_with_p_es_sc: p_and_es_and_sc_forces_rh(),
        p_alone_forces_self_adjoint: p_alone_forces_self_adjoint(),
        p_alone_forces_correspondence: p_alone_forces_correspondence(),
        hidden_premises: ["EssentialSelfAdjointness", "SpectralCorrespondence"],
        controls_declined: controls.iter().filter(|value| **value).count(),
        controls,
        m29_reached: false,
    }
}

pub fn machine_record(report: &M29gExperiment) -> String {
    format!(
        "M29g|reduction_valid_with_p_es_sc={}|p_alone_forces_self_adjoint={}|p_alone_forces_correspondence={}|hidden_premises={:?}|controls={:?}|controls_declined={}/6|m29_reached=false|claim=P_implies_RH_only_with_ES_and_SC",
        report.reduction_valid_with_p_es_sc,
        report.p_alone_forces_self_adjoint,
        report.p_alone_forces_correspondence,
        report.hidden_premises,
        report.controls,
        report.controls_declined,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_reduction_closes_only_with_both_theorems() {
        assert!(p_and_es_and_sc_forces_rh());
        let gns = build_gns(&positivity(), &full_functional()).ok();
        assert!(gns.is_some());
        // ES missing
        assert_eq!(
            self_adjoint(&Fact::GnsHilbertSpace, &Fact::GnsHilbertSpace),
            Err(Rejection::MissingEssentialSelfAdjointness)
        );
        // SC missing
        assert_eq!(
            force(&Fact::SelfAdjointOperator, &Fact::SelfAdjointOperator),
            Err(Rejection::MissingCorrespondence)
        );
    }

    #[test]
    fn p_alone_forces_neither_hidden_premise() {
        assert!(!p_alone_forces_self_adjoint());
        assert!(!p_alone_forces_correspondence());
    }

    #[test]
    fn every_control_declines_and_record_is_honest() {
        let report = m29g_experiment();
        assert!(report.reduction_valid_with_p_es_sc);
        assert!(!report.p_alone_forces_self_adjoint);
        assert!(!report.p_alone_forces_correspondence);
        assert_eq!(
            report.hidden_premises,
            ["EssentialSelfAdjointness", "SpectralCorrespondence"]
        );
        assert_eq!(report.controls, [true; 6]);
        assert_eq!(report.controls_declined, 6);
        assert!(!report.m29_reached);
        let record = machine_record(&report);
        assert!(record.contains("reduction_valid_with_p_es_sc=true"));
        assert!(record.contains("p_alone_forces_self_adjoint=false"));
        assert!(record.contains("m29_reached=false"));
    }
}
