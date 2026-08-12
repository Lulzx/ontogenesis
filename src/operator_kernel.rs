//! SH4: typed proof kernel for infinite-operator and trace-formula search.
//!
//! Search code may submit derivations to this module, but cannot add rules.
//! The kernel checks premise identity, analytic side conditions, provenance,
//! and declared assumptions independently.

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Provenance {
    Generic,
    ArithmeticOnly,
    ZeroDerived,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Fact {
    DenseDomain {
        space: String,
        domain: String,
    },
    SymmetricOn {
        operator: String,
        space: String,
        domain: String,
    },
    DenselyDefinedSymmetric {
        operator: String,
        domain: String,
    },
    PositiveOffDiagonal {
        operator: String,
    },
    ReciprocalCoefficientSeriesDiverges {
        operator: String,
    },
    DeficiencyIndices {
        operator: String,
        positive: u32,
        negative: u32,
    },
    EssentiallySelfAdjoint {
        operator: String,
    },
    SelfAdjointClosure {
        operator: String,
        closure: String,
    },
    TruncationsSelfAdjoint {
        family: String,
    },
    CommonCoreResolventLimit {
        family: String,
        limit_operator: String,
        core: String,
    },
    StrongOperatorLimit {
        family: String,
        limit_operator: String,
    },
    StrongResolventLimit {
        family: String,
        limit_operator: String,
    },
    ContinuousVanishingAtInfinityClass {
        class: String,
    },
    BoundedContinuousClass {
        class: String,
    },
    SpectralIntegralConvergence {
        family: String,
        limit_operator: String,
        class: String,
    },
    PointSpectrumConvergence {
        family: String,
        limit_operator: String,
    },
    CertifiedTraceIdentity {
        spectral_measure: String,
        arithmetic_measure: String,
        class: String,
        normalized: bool,
    },
    DistributionEquality {
        left_measure: String,
        right_measure: String,
        class: String,
    },
    SeparatingClass {
        class: String,
        locally_finite: bool,
    },
    MeasureEquality {
        left_measure: String,
        right_measure: String,
    },
    OperatorSpectralMeasure {
        operator: String,
        measure: String,
    },
    XiZeroMeasure {
        measure: String,
    },
    ExactXiSpectrumCorrespondence {
        operator: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Judgment {
    pub fact: Fact,
    pub assumptions: BTreeSet<String>,
    pub provenance: BTreeSet<Provenance>,
}

impl Judgment {
    pub fn axiom(fact: Fact, assumption: &str, provenance: Provenance) -> Self {
        Self {
            fact,
            assumptions: BTreeSet::from([assumption.to_string()]),
            provenance: BTreeSet::from([provenance]),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rule {
    DenseSymmetric,
    CarlemanEssentialSelfAdjoint,
    ZeroDeficiency,
    CloseEssentialOperator,
    CommonCoreStrongResolvent,
    TransferBoundedContinuousIntegrals,
    NormalizeTraceIdentity,
    SeparateMeasures,
    CertifyXiCorrespondence,
    InvalidFiniteSymmetryToLimit,
    InvalidSymmetryToEssential,
    InvalidStrongOperatorToResolvent,
    InvalidResolventToPointSpectrum,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Step {
    pub rule: Rule,
    pub premises: Vec<usize>,
    pub conclusion: Fact,
}

#[derive(Clone, Debug)]
pub struct Derivation {
    pub declared_assumptions: BTreeSet<String>,
    pub axioms: Vec<Judgment>,
    pub steps: Vec<Step>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Rejection {
    UndeclaredAssumption(String),
    UnknownPremise(usize),
    WrongArity { expected: usize, actual: usize },
    PremiseMismatch,
    ConclusionMismatch,
    InvalidSideCondition(&'static str),
    UnsupportedInference(Rule),
    ForbiddenProvenance(Provenance),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Verification {
    pub accepted: bool,
    pub judgments: Vec<Judgment>,
    pub rejection: Option<Rejection>,
}

fn combined(premises: &[&Judgment], fact: Fact) -> Judgment {
    Judgment {
        fact,
        assumptions: premises
            .iter()
            .flat_map(|premise| premise.assumptions.iter().cloned())
            .collect(),
        provenance: premises
            .iter()
            .flat_map(|premise| premise.provenance.iter().copied())
            .collect(),
    }
}

fn require_arity(premises: &[&Judgment], expected: usize) -> Result<(), Rejection> {
    if premises.len() == expected {
        Ok(())
    } else {
        Err(Rejection::WrongArity {
            expected,
            actual: premises.len(),
        })
    }
}

fn apply(rule: Rule, premises: &[&Judgment]) -> Result<Judgment, Rejection> {
    match rule {
        Rule::DenseSymmetric => {
            require_arity(premises, 2)?;
            match (&premises[0].fact, &premises[1].fact) {
                (
                    Fact::DenseDomain { space, domain },
                    Fact::SymmetricOn {
                        operator,
                        space: symmetric_space,
                        domain: symmetric_domain,
                    },
                ) if space == symmetric_space && domain == symmetric_domain => Ok(combined(
                    premises,
                    Fact::DenselyDefinedSymmetric {
                        operator: operator.clone(),
                        domain: domain.clone(),
                    },
                )),
                _ => Err(Rejection::PremiseMismatch),
            }
        }
        Rule::CarlemanEssentialSelfAdjoint => {
            require_arity(premises, 3)?;
            match (&premises[0].fact, &premises[1].fact, &premises[2].fact) {
                (
                    Fact::DenselyDefinedSymmetric { operator, .. },
                    Fact::PositiveOffDiagonal {
                        operator: positive_operator,
                    },
                    Fact::ReciprocalCoefficientSeriesDiverges {
                        operator: divergent_operator,
                    },
                ) if operator == positive_operator && operator == divergent_operator => {
                    Ok(combined(
                        premises,
                        Fact::EssentiallySelfAdjoint {
                            operator: operator.clone(),
                        },
                    ))
                }
                _ => Err(Rejection::PremiseMismatch),
            }
        }
        Rule::ZeroDeficiency => {
            require_arity(premises, 2)?;
            match (&premises[0].fact, &premises[1].fact) {
                (
                    Fact::DenselyDefinedSymmetric { operator, .. },
                    Fact::DeficiencyIndices {
                        operator: deficiency_operator,
                        positive,
                        negative,
                    },
                ) if operator == deficiency_operator && *positive == 0 && *negative == 0 => {
                    Ok(combined(
                        premises,
                        Fact::EssentiallySelfAdjoint {
                            operator: operator.clone(),
                        },
                    ))
                }
                (Fact::DenselyDefinedSymmetric { .. }, Fact::DeficiencyIndices { .. }) => Err(
                    Rejection::InvalidSideCondition("deficiency indices must both be zero"),
                ),
                _ => Err(Rejection::PremiseMismatch),
            }
        }
        Rule::CloseEssentialOperator => {
            require_arity(premises, 1)?;
            match &premises[0].fact {
                Fact::EssentiallySelfAdjoint { operator } => Ok(combined(
                    premises,
                    Fact::SelfAdjointClosure {
                        operator: operator.clone(),
                        closure: format!("closure({operator})"),
                    },
                )),
                _ => Err(Rejection::PremiseMismatch),
            }
        }
        Rule::CommonCoreStrongResolvent => {
            require_arity(premises, 2)?;
            match (&premises[0].fact, &premises[1].fact) {
                (
                    Fact::TruncationsSelfAdjoint { family },
                    Fact::CommonCoreResolventLimit {
                        family: limit_family,
                        limit_operator,
                        ..
                    },
                ) if family == limit_family => Ok(combined(
                    premises,
                    Fact::StrongResolventLimit {
                        family: family.clone(),
                        limit_operator: limit_operator.clone(),
                    },
                )),
                _ => Err(Rejection::PremiseMismatch),
            }
        }
        Rule::TransferBoundedContinuousIntegrals => {
            require_arity(premises, 2)?;
            match (&premises[0].fact, &premises[1].fact) {
                (
                    Fact::StrongResolventLimit {
                        family,
                        limit_operator,
                    },
                    Fact::ContinuousVanishingAtInfinityClass { class },
                ) => Ok(combined(
                    premises,
                    Fact::SpectralIntegralConvergence {
                        family: family.clone(),
                        limit_operator: limit_operator.clone(),
                        class: class.clone(),
                    },
                )),
                _ => Err(Rejection::PremiseMismatch),
            }
        }
        Rule::NormalizeTraceIdentity => {
            require_arity(premises, 1)?;
            match &premises[0].fact {
                Fact::CertifiedTraceIdentity {
                    spectral_measure,
                    arithmetic_measure,
                    class,
                    normalized: true,
                } => Ok(combined(
                    premises,
                    Fact::DistributionEquality {
                        left_measure: spectral_measure.clone(),
                        right_measure: arithmetic_measure.clone(),
                        class: class.clone(),
                    },
                )),
                Fact::CertifiedTraceIdentity {
                    normalized: false, ..
                } => Err(Rejection::InvalidSideCondition(
                    "trace identity must be independently normalized",
                )),
                _ => Err(Rejection::PremiseMismatch),
            }
        }
        Rule::SeparateMeasures => {
            require_arity(premises, 2)?;
            match (&premises[0].fact, &premises[1].fact) {
                (
                    Fact::DistributionEquality {
                        left_measure,
                        right_measure,
                        class,
                    },
                    Fact::SeparatingClass {
                        class: separating_class,
                        locally_finite: true,
                    },
                ) if class == separating_class => Ok(combined(
                    premises,
                    Fact::MeasureEquality {
                        left_measure: left_measure.clone(),
                        right_measure: right_measure.clone(),
                    },
                )),
                (Fact::DistributionEquality { .. }, Fact::SeparatingClass { .. }) => {
                    Err(Rejection::InvalidSideCondition(
                        "test class must match and separate locally finite measures",
                    ))
                }
                _ => Err(Rejection::PremiseMismatch),
            }
        }
        Rule::CertifyXiCorrespondence => {
            require_arity(premises, 4)?;
            let (operator, spectral_measure) = match &premises[0].fact {
                Fact::OperatorSpectralMeasure { operator, measure } => (operator, measure),
                _ => return Err(Rejection::PremiseMismatch),
            };
            let closure_operator = match &premises[1].fact {
                Fact::SelfAdjointClosure { closure, .. } => closure,
                _ => return Err(Rejection::PremiseMismatch),
            };
            let (left, right) = match &premises[2].fact {
                Fact::MeasureEquality {
                    left_measure,
                    right_measure,
                } => (left_measure, right_measure),
                _ => return Err(Rejection::PremiseMismatch),
            };
            let xi_measure = match &premises[3].fact {
                Fact::XiZeroMeasure { measure } => measure,
                _ => return Err(Rejection::PremiseMismatch),
            };
            if premises
                .iter()
                .any(|premise| premise.provenance.contains(&Provenance::ZeroDerived))
            {
                return Err(Rejection::ForbiddenProvenance(Provenance::ZeroDerived));
            }
            if operator != closure_operator || left != spectral_measure || right != xi_measure {
                return Err(Rejection::PremiseMismatch);
            }
            Ok(combined(
                premises,
                Fact::ExactXiSpectrumCorrespondence {
                    operator: operator.clone(),
                },
            ))
        }
        Rule::InvalidFiniteSymmetryToLimit
        | Rule::InvalidSymmetryToEssential
        | Rule::InvalidStrongOperatorToResolvent
        | Rule::InvalidResolventToPointSpectrum => Err(Rejection::UnsupportedInference(rule)),
    }
}

pub fn verify(derivation: &Derivation) -> Verification {
    for axiom in &derivation.axioms {
        if let Some(assumption) = axiom
            .assumptions
            .iter()
            .find(|assumption| !derivation.declared_assumptions.contains(*assumption))
        {
            return Verification {
                accepted: false,
                judgments: Vec::new(),
                rejection: Some(Rejection::UndeclaredAssumption(assumption.clone())),
            };
        }
    }
    let mut judgments = derivation.axioms.clone();
    for step in &derivation.steps {
        let premises = match step
            .premises
            .iter()
            .map(|index| {
                judgments
                    .get(*index)
                    .ok_or(Rejection::UnknownPremise(*index))
            })
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(premises) => premises,
            Err(rejection) => {
                return Verification {
                    accepted: false,
                    judgments,
                    rejection: Some(rejection),
                };
            }
        };
        match apply(step.rule, &premises) {
            Ok(judgment) if judgment.fact == step.conclusion => judgments.push(judgment),
            Ok(_) => {
                return Verification {
                    accepted: false,
                    judgments,
                    rejection: Some(Rejection::ConclusionMismatch),
                };
            }
            Err(rejection) => {
                return Verification {
                    accepted: false,
                    judgments,
                    rejection: Some(rejection),
                };
            }
        }
    }
    Verification {
        accepted: true,
        judgments,
        rejection: None,
    }
}

#[derive(Clone, Debug)]
pub struct Sh4Experiment {
    pub self_adjoint_chain: bool,
    pub resolvent_chain: bool,
    pub trace_measure_chain: bool,
    pub xi_adapter_chain: bool,
    pub carleman_chain: bool,
    pub controls: [bool; 11],
    pub controls_declined: usize,
    pub deterministic_assumptions: bool,
    pub sh4_passed: bool,
    pub m29_reached: bool,
}

fn judgment(fact: Fact, name: &str, provenance: Provenance) -> Judgment {
    Judgment::axiom(fact, name, provenance)
}

pub fn sh4_experiment() -> Sh4Experiment {
    let dense = judgment(
        Fact::DenseDomain {
            space: "H".into(),
            domain: "D".into(),
        },
        "dense",
        Provenance::Generic,
    );
    let symmetric = judgment(
        Fact::SymmetricOn {
            operator: "T".into(),
            space: "H".into(),
            domain: "D".into(),
        },
        "symmetric",
        Provenance::ArithmeticOnly,
    );
    let dense_symmetric = apply(Rule::DenseSymmetric, &[&dense, &symmetric]).ok();
    let deficiency = judgment(
        Fact::DeficiencyIndices {
            operator: "T".into(),
            positive: 0,
            negative: 0,
        },
        "zero-deficiency",
        Provenance::Generic,
    );
    let essential = dense_symmetric
        .as_ref()
        .and_then(|fact| apply(Rule::ZeroDeficiency, &[fact, &deficiency]).ok());
    let closure = essential
        .as_ref()
        .and_then(|fact| apply(Rule::CloseEssentialOperator, &[fact]).ok());
    let self_adjoint_chain = closure.is_some();

    let positive = judgment(
        Fact::PositiveOffDiagonal {
            operator: "T".into(),
        },
        "positive-off-diagonal",
        Provenance::ArithmeticOnly,
    );
    let reciprocal_divergence = judgment(
        Fact::ReciprocalCoefficientSeriesDiverges {
            operator: "T".into(),
        },
        "reciprocal-series-diverges",
        Provenance::ArithmeticOnly,
    );
    let carleman_chain = dense_symmetric.as_ref().is_some_and(|dense_symmetric| {
        apply(
            Rule::CarlemanEssentialSelfAdjoint,
            &[dense_symmetric, &positive, &reciprocal_divergence],
        )
        .is_ok()
    });

    let truncations = judgment(
        Fact::TruncationsSelfAdjoint {
            family: "T_n".into(),
        },
        "finite-self-adjoint",
        Provenance::ArithmeticOnly,
    );
    let core_limit = judgment(
        Fact::CommonCoreResolventLimit {
            family: "T_n".into(),
            limit_operator: "closure(T)".into(),
            core: "D".into(),
        },
        "common-core-resolvent",
        Provenance::Generic,
    );
    let resolvent = apply(
        Rule::CommonCoreStrongResolvent,
        &[&truncations, &core_limit],
    )
    .ok();
    let bounded_class = judgment(
        Fact::ContinuousVanishingAtInfinityClass {
            class: "C_0".into(),
        },
        "continuous-vanishing-at-infinity",
        Provenance::Generic,
    );
    let resolvent_chain = resolvent.as_ref().is_some_and(|limit| {
        apply(
            Rule::TransferBoundedContinuousIntegrals,
            &[limit, &bounded_class],
        )
        .is_ok()
    });

    let trace = judgment(
        Fact::CertifiedTraceIdentity {
            spectral_measure: "mu_T".into(),
            arithmetic_measure: "mu_xi".into(),
            class: "Schwartz_even".into(),
            normalized: true,
        },
        "normalized-trace",
        Provenance::ArithmeticOnly,
    );
    let distribution = apply(Rule::NormalizeTraceIdentity, &[&trace]).ok();
    let separating = judgment(
        Fact::SeparatingClass {
            class: "Schwartz_even".into(),
            locally_finite: true,
        },
        "separating-class",
        Provenance::Generic,
    );
    let measure_equality = distribution
        .as_ref()
        .and_then(|fact| apply(Rule::SeparateMeasures, &[fact, &separating]).ok());
    let trace_measure_chain = measure_equality.is_some();

    let operator_measure = judgment(
        Fact::OperatorSpectralMeasure {
            operator: "closure(T)".into(),
            measure: "mu_T".into(),
        },
        "spectral-theorem",
        Provenance::Generic,
    );
    let xi_measure = judgment(
        Fact::XiZeroMeasure {
            measure: "mu_xi".into(),
        },
        "xi-measure-definition",
        Provenance::ArithmeticOnly,
    );
    let xi_adapter_chain = match (&closure, &measure_equality) {
        (Some(closure), Some(equality)) => apply(
            Rule::CertifyXiCorrespondence,
            &[&operator_measure, closure, equality, &xi_measure],
        )
        .is_ok(),
        _ => false,
    };

    let unnormalized_trace = judgment(
        Fact::CertifiedTraceIdentity {
            spectral_measure: "mu_T".into(),
            arithmetic_measure: "mu_xi".into(),
            class: "Schwartz_even".into(),
            normalized: false,
        },
        "unnormalized",
        Provenance::ArithmeticOnly,
    );
    let nonseparating = judgment(
        Fact::SeparatingClass {
            class: "finite_tests".into(),
            locally_finite: false,
        },
        "nonseparating",
        Provenance::Generic,
    );
    let finite_distribution = judgment(
        Fact::DistributionEquality {
            left_measure: "mu_T".into(),
            right_measure: "mu_xi".into(),
            class: "finite_tests".into(),
        },
        "finite-distribution",
        Provenance::Generic,
    );
    let zero_operator_measure = judgment(
        Fact::OperatorSpectralMeasure {
            operator: "closure(T)".into(),
            measure: "mu_T".into(),
        },
        "zero-derived",
        Provenance::ZeroDerived,
    );
    let controls = [
        apply(Rule::InvalidFiniteSymmetryToLimit, &[&symmetric]).is_err(),
        apply(Rule::InvalidSymmetryToEssential, &[&symmetric]).is_err(),
        apply(
            Rule::InvalidStrongOperatorToResolvent,
            &[&judgment(
                Fact::StrongOperatorLimit {
                    family: "T_n".into(),
                    limit_operator: "T".into(),
                },
                "strong-operator",
                Provenance::Generic,
            )],
        )
        .is_err(),
        resolvent
            .as_ref()
            .is_some_and(|limit| apply(Rule::InvalidResolventToPointSpectrum, &[limit]).is_err()),
        resolvent.as_ref().is_some_and(|limit| {
            apply(
                Rule::TransferBoundedContinuousIntegrals,
                &[
                    limit,
                    &judgment(
                        Fact::BoundedContinuousClass {
                            class: "C_b".into(),
                        },
                        "bounded-continuous",
                        Provenance::Generic,
                    ),
                ],
            )
            .is_err()
        }),
        apply(Rule::NormalizeTraceIdentity, &[&unnormalized_trace]).is_err(),
        apply(
            Rule::SeparateMeasures,
            &[&finite_distribution, &nonseparating],
        )
        .is_err(),
        match (&closure, &measure_equality) {
            (Some(closure), Some(equality)) => apply(
                Rule::CertifyXiCorrespondence,
                &[&zero_operator_measure, closure, equality, &xi_measure],
            )
            .is_err(),
            _ => false,
        },
        verify(&Derivation {
            declared_assumptions: BTreeSet::new(),
            axioms: vec![xi_measure.clone()],
            steps: Vec::new(),
        })
        .rejection
        .is_some(),
        dense_symmetric.as_ref().is_some_and(|dense_symmetric| {
            apply(
                Rule::CarlemanEssentialSelfAdjoint,
                &[dense_symmetric, &positive],
            )
            .is_err()
        }),
        dense_symmetric.as_ref().is_some_and(|dense_symmetric| {
            let wrong_operator = judgment(
                Fact::ReciprocalCoefficientSeriesDiverges {
                    operator: "S".into(),
                },
                "wrong-reciprocal-series",
                Provenance::ArithmeticOnly,
            );
            apply(
                Rule::CarlemanEssentialSelfAdjoint,
                &[dense_symmetric, &positive, &wrong_operator],
            )
            .is_err()
        }),
    ];
    let controls_declined = controls.iter().filter(|declined| **declined).count();
    let deterministic_assumptions = closure.as_ref().is_some_and(|closed| {
        closed.assumptions
            == BTreeSet::from([
                "dense".to_string(),
                "symmetric".to_string(),
                "zero-deficiency".to_string(),
            ])
            && closed.provenance
                == BTreeSet::from([Provenance::Generic, Provenance::ArithmeticOnly])
    });
    let sh4_passed = self_adjoint_chain
        && resolvent_chain
        && trace_measure_chain
        && xi_adapter_chain
        && carleman_chain
        && controls_declined == controls.len()
        && deterministic_assumptions;
    Sh4Experiment {
        self_adjoint_chain,
        resolvent_chain,
        trace_measure_chain,
        xi_adapter_chain,
        carleman_chain,
        controls,
        controls_declined,
        deterministic_assumptions,
        sh4_passed,
        m29_reached: false,
    }
}

pub fn machine_record(report: &Sh4Experiment) -> String {
    format!(
        "SH4c|self_adjoint_chain={}|carleman_chain={}|resolvent_chain={}|trace_measure_chain={}|xi_adapter_chain={}|controls={:?}|controls_declined={}/11|assumptions_deterministic={}|kernel_pass={}|m29_reached={}|claim=trusted_infrastructure_only",
        report.self_adjoint_chain,
        report.carleman_chain,
        report.resolvent_chain,
        report.trace_measure_chain,
        report.xi_adapter_chain,
        report.controls,
        report.controls_declined,
        report.deterministic_assumptions,
        report.sh4_passed,
        report.m29_reached
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declared(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| (*name).to_string()).collect()
    }

    #[test]
    fn certifies_dense_symmetric_zero_deficiency_chain() {
        let derivation = Derivation {
            declared_assumptions: declared(&["dense", "symmetric", "deficiency"]),
            axioms: vec![
                Judgment::axiom(
                    Fact::DenseDomain {
                        space: "H".into(),
                        domain: "D".into(),
                    },
                    "dense",
                    Provenance::Generic,
                ),
                Judgment::axiom(
                    Fact::SymmetricOn {
                        operator: "T".into(),
                        space: "H".into(),
                        domain: "D".into(),
                    },
                    "symmetric",
                    Provenance::Generic,
                ),
                Judgment::axiom(
                    Fact::DeficiencyIndices {
                        operator: "T".into(),
                        positive: 0,
                        negative: 0,
                    },
                    "deficiency",
                    Provenance::Generic,
                ),
            ],
            steps: vec![
                Step {
                    rule: Rule::DenseSymmetric,
                    premises: vec![0, 1],
                    conclusion: Fact::DenselyDefinedSymmetric {
                        operator: "T".into(),
                        domain: "D".into(),
                    },
                },
                Step {
                    rule: Rule::ZeroDeficiency,
                    premises: vec![3, 2],
                    conclusion: Fact::EssentiallySelfAdjoint {
                        operator: "T".into(),
                    },
                },
                Step {
                    rule: Rule::CloseEssentialOperator,
                    premises: vec![4],
                    conclusion: Fact::SelfAdjointClosure {
                        operator: "T".into(),
                        closure: "closure(T)".into(),
                    },
                },
            ],
        };
        assert!(verify(&derivation).accepted);
    }

    #[test]
    fn rejects_unsound_finite_to_infinite_and_convergence_upgrades() {
        for rule in [
            Rule::InvalidFiniteSymmetryToLimit,
            Rule::InvalidSymmetryToEssential,
            Rule::InvalidStrongOperatorToResolvent,
            Rule::InvalidResolventToPointSpectrum,
        ] {
            let result = apply(rule, &[]);
            assert_eq!(result, Err(Rejection::UnsupportedInference(rule)));
        }
    }

    #[test]
    fn rejects_nonzero_deficiency_and_unnormalized_trace_identity() {
        let dense = Judgment::axiom(
            Fact::DenselyDefinedSymmetric {
                operator: "T".into(),
                domain: "D".into(),
            },
            "dense-symmetric",
            Provenance::Generic,
        );
        let deficiency = Judgment::axiom(
            Fact::DeficiencyIndices {
                operator: "T".into(),
                positive: 1,
                negative: 1,
            },
            "deficiency",
            Provenance::Generic,
        );
        assert!(matches!(
            apply(Rule::ZeroDeficiency, &[&dense, &deficiency]),
            Err(Rejection::InvalidSideCondition(_))
        ));
        let trace = Judgment::axiom(
            Fact::CertifiedTraceIdentity {
                spectral_measure: "mu".into(),
                arithmetic_measure: "nu".into(),
                class: "C".into(),
                normalized: false,
            },
            "trace",
            Provenance::ArithmeticOnly,
        );
        assert!(matches!(
            apply(Rule::NormalizeTraceIdentity, &[&trace]),
            Err(Rejection::InvalidSideCondition(_))
        ));
    }

    #[test]
    fn rejects_nonseparating_class_undeclared_assumption_and_zero_provenance() {
        let equality = Judgment::axiom(
            Fact::DistributionEquality {
                left_measure: "mu".into(),
                right_measure: "xi".into(),
                class: "finite-tests".into(),
            },
            "distribution",
            Provenance::ArithmeticOnly,
        );
        let class = Judgment::axiom(
            Fact::SeparatingClass {
                class: "finite-tests".into(),
                locally_finite: false,
            },
            "class",
            Provenance::Generic,
        );
        assert!(matches!(
            apply(Rule::SeparateMeasures, &[&equality, &class]),
            Err(Rejection::InvalidSideCondition(_))
        ));

        let undeclared = verify(&Derivation {
            declared_assumptions: BTreeSet::new(),
            axioms: vec![Judgment::axiom(
                Fact::XiZeroMeasure {
                    measure: "xi".into(),
                },
                "hidden",
                Provenance::Generic,
            )],
            steps: Vec::new(),
        });
        assert_eq!(
            undeclared.rejection,
            Some(Rejection::UndeclaredAssumption("hidden".into()))
        );

        let premises = [
            Judgment::axiom(
                Fact::OperatorSpectralMeasure {
                    operator: "A".into(),
                    measure: "mu".into(),
                },
                "spectral",
                Provenance::ZeroDerived,
            ),
            Judgment::axiom(
                Fact::SelfAdjointClosure {
                    operator: "raw".into(),
                    closure: "A".into(),
                },
                "self-adjoint",
                Provenance::Generic,
            ),
            Judgment::axiom(
                Fact::MeasureEquality {
                    left_measure: "mu".into(),
                    right_measure: "xi".into(),
                },
                "measure",
                Provenance::Generic,
            ),
            Judgment::axiom(
                Fact::XiZeroMeasure {
                    measure: "xi".into(),
                },
                "xi",
                Provenance::Generic,
            ),
        ];
        assert_eq!(
            apply(
                Rule::CertifyXiCorrespondence,
                &premises.iter().collect::<Vec<_>>()
            ),
            Err(Rejection::ForbiddenProvenance(Provenance::ZeroDerived))
        );
    }

    #[test]
    fn sh4_gate_passes_without_claiming_m29() {
        let report = sh4_experiment();
        assert!(report.sh4_passed, "{report:#?}");
        assert_eq!(report.controls, [true; 11]);
        assert!(!report.m29_reached);
        assert_eq!(machine_record(&report), machine_record(&sh4_experiment()));
    }
}
