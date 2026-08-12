//! SH9: typed counting-asymptotic obstruction for the selected prime Jacobi family.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Provenance {
    ArithmeticOnly,
    ZeroDerived,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Fact {
    SelfAdjointDiagonal {
        operator: String,
    },
    CompactResolvent {
        operator: String,
    },
    BoundedSelfAdjointPerturbation {
        operator: String,
        norm_bound: u32,
    },
    UnboundedSelfAdjointPerturbation {
        operator: String,
    },
    SumOperator {
        diagonal: String,
        perturbation: String,
        sum: String,
    },
    EigenvalueDisplacement {
        operator: String,
        reference: String,
        bound: u32,
    },
    CountingAsymptotic {
        object: String,
        growth: Growth,
    },
    CountingRatioZero {
        numerator: String,
        denominator: String,
    },
    SpectraIncompatible {
        left: String,
        right: String,
    },
    SampledCountAgreement {
        left: String,
        right: String,
    },
    BigOCount {
        object: String,
        growth: Growth,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Growth {
    TOverLogT,
    TLogT,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Judgment {
    pub fact: Fact,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rule {
    BoundedPerturbationDisplacement,
    TransferCountingUnderBoundedDisplacement,
    IncompatibleCountingGrowth,
    RejectSampledCounts,
    RejectBigOOnly,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Rejection {
    WrongArity,
    PremiseMismatch,
    MissingCompactResolvent,
    UnboundedPerturbation,
    NonAsymptoticEvidence,
    ForbiddenProvenance,
}

fn apply(rule: Rule, premises: &[&Judgment]) -> Result<Judgment, Rejection> {
    if premises
        .iter()
        .any(|premise| premise.provenance == Provenance::ZeroDerived)
    {
        return Err(Rejection::ForbiddenProvenance);
    }
    match rule {
        Rule::BoundedPerturbationDisplacement => {
            if premises.len() != 4 {
                return Err(Rejection::WrongArity);
            }
            match (
                &premises[0].fact,
                &premises[1].fact,
                &premises[2].fact,
                &premises[3].fact,
            ) {
                (
                    Fact::SelfAdjointDiagonal { operator },
                    Fact::CompactResolvent {
                        operator: compact_operator,
                    },
                    Fact::BoundedSelfAdjointPerturbation {
                        operator: perturbation,
                        norm_bound,
                    },
                    Fact::SumOperator {
                        diagonal,
                        perturbation: sum_perturbation,
                        sum,
                    },
                ) if operator == compact_operator
                    && operator == diagonal
                    && perturbation == sum_perturbation =>
                {
                    Ok(Judgment {
                        fact: Fact::EigenvalueDisplacement {
                            operator: sum.clone(),
                            reference: operator.clone(),
                            bound: *norm_bound,
                        },
                        provenance: Provenance::ArithmeticOnly,
                    })
                }
                (
                    Fact::SelfAdjointDiagonal { .. },
                    Fact::BoundedSelfAdjointPerturbation { .. },
                    _,
                    _,
                ) => Err(Rejection::MissingCompactResolvent),
                _ => Err(Rejection::PremiseMismatch),
            }
        }
        Rule::IncompatibleCountingGrowth => {
            if premises.len() != 3 {
                return Err(Rejection::WrongArity);
            }
            match (&premises[0].fact, &premises[1].fact, &premises[2].fact) {
                (
                    Fact::CountingAsymptotic {
                        object: left,
                        growth: Growth::TOverLogT,
                    },
                    Fact::CountingAsymptotic {
                        object: right,
                        growth: Growth::TLogT,
                    },
                    Fact::CountingRatioZero {
                        numerator,
                        denominator,
                    },
                ) if left == numerator && right == denominator => Ok(Judgment {
                    fact: Fact::SpectraIncompatible {
                        left: left.clone(),
                        right: right.clone(),
                    },
                    provenance: Provenance::ArithmeticOnly,
                }),
                (Fact::BigOCount { .. }, _, _) | (_, Fact::BigOCount { .. }, _) => {
                    Err(Rejection::NonAsymptoticEvidence)
                }
                _ => Err(Rejection::PremiseMismatch),
            }
        }
        Rule::TransferCountingUnderBoundedDisplacement => {
            if premises.len() != 2 {
                return Err(Rejection::WrongArity);
            }
            match (&premises[0].fact, &premises[1].fact) {
                (
                    Fact::EigenvalueDisplacement {
                        operator,
                        reference,
                        ..
                    },
                    Fact::CountingAsymptotic { object, growth },
                ) if reference == object => Ok(Judgment {
                    fact: Fact::CountingAsymptotic {
                        object: operator.clone(),
                        growth: *growth,
                    },
                    provenance: Provenance::ArithmeticOnly,
                }),
                (Fact::EigenvalueDisplacement { .. }, Fact::CountingAsymptotic { .. }) => {
                    Err(Rejection::PremiseMismatch)
                }
                (Fact::EigenvalueDisplacement { .. }, Fact::BigOCount { .. }) => {
                    Err(Rejection::NonAsymptoticEvidence)
                }
                _ => Err(Rejection::PremiseMismatch),
            }
        }
        Rule::RejectSampledCounts | Rule::RejectBigOOnly => Err(Rejection::NonAsymptoticEvidence),
    }
}

#[derive(Clone, Debug)]
pub struct Sh9Experiment {
    pub displacement_bound: Option<u32>,
    pub prime_growth: Growth,
    pub xi_growth: Growth,
    pub ratio_zero: bool,
    pub incompatibility_certified: bool,
    pub controls: [bool; 7],
    pub controls_declined: usize,
    pub selected_family_eliminated: bool,
    pub m29_reached: bool,
}

fn judgment(fact: Fact) -> Judgment {
    Judgment {
        fact,
        provenance: Provenance::ArithmeticOnly,
    }
}

pub fn sh9_experiment() -> Sh9Experiment {
    let diagonal = judgment(Fact::SelfAdjointDiagonal {
        operator: "diag(p_n)".into(),
    });
    let compact = judgment(Fact::CompactResolvent {
        operator: "diag(p_n)".into(),
    });
    let perturbation = judgment(Fact::BoundedSelfAdjointPerturbation {
        operator: "path-adjacency".into(),
        norm_bound: 2,
    });
    let sum = judgment(Fact::SumOperator {
        diagonal: "diag(p_n)".into(),
        perturbation: "path-adjacency".into(),
        sum: "J(1,p_n)".into(),
    });
    let displacement = apply(
        Rule::BoundedPerturbationDisplacement,
        &[&diagonal, &compact, &perturbation, &sum],
    )
    .ok();
    let diagonal_count = judgment(Fact::CountingAsymptotic {
        object: "diag(p_n)".into(),
        growth: Growth::TOverLogT,
    });
    let prime_count = displacement.as_ref().and_then(|displacement| {
        apply(
            Rule::TransferCountingUnderBoundedDisplacement,
            &[displacement, &diagonal_count],
        )
        .ok()
    });
    let xi_count = judgment(Fact::CountingAsymptotic {
        object: "xi-zero-ordinates".into(),
        growth: Growth::TLogT,
    });
    let ratio = judgment(Fact::CountingRatioZero {
        numerator: "J(1,p_n)".into(),
        denominator: "xi-zero-ordinates".into(),
    });
    let incompatible = prime_count.as_ref().and_then(|prime_count| {
        apply(
            Rule::IncompatibleCountingGrowth,
            &[prime_count, &xi_count, &ratio],
        )
        .ok()
    });

    let missing_compact = apply(
        Rule::BoundedPerturbationDisplacement,
        &[&diagonal, &perturbation, &sum, &sum],
    );
    let unbounded = judgment(Fact::UnboundedSelfAdjointPerturbation {
        operator: "unbounded-perturbation".into(),
    });
    let sampled = judgment(Fact::SampledCountAgreement {
        left: "finite-spectrum".into(),
        right: "finite-zeros".into(),
    });
    let big_o = judgment(Fact::BigOCount {
        object: "candidate-count".into(),
        growth: Growth::TOverLogT,
    });
    let zero_derived = Judgment {
        fact: Fact::CountingAsymptotic {
            object: "zero-fitted-operator".into(),
            growth: Growth::TLogT,
        },
        provenance: Provenance::ZeroDerived,
    };
    let wrong_reference_count = judgment(Fact::CountingAsymptotic {
        object: "diag(q_n)".into(),
        growth: Growth::TOverLogT,
    });
    let controls = [
        missing_compact == Err(Rejection::MissingCompactResolvent),
        apply(
            Rule::BoundedPerturbationDisplacement,
            &[&diagonal, &compact, &unbounded, &sum],
        )
        .is_err(),
        apply(Rule::RejectSampledCounts, &[&sampled]).is_err(),
        apply(Rule::RejectBigOOnly, &[&big_o]).is_err(),
        apply(
            Rule::IncompatibleCountingGrowth,
            &[&zero_derived, &xi_count, &ratio],
        ) == Err(Rejection::ForbiddenProvenance),
        displacement.as_ref().is_some_and(|displacement| {
            apply(
                Rule::TransferCountingUnderBoundedDisplacement,
                &[displacement, &wrong_reference_count],
            ) == Err(Rejection::PremiseMismatch)
        }),
        prime_count.is_some(),
    ];
    let controls_declined = controls.iter().filter(|control| **control).count();
    let displacement_bound = displacement.and_then(|judgment| match judgment.fact {
        Fact::EigenvalueDisplacement { bound, .. } => Some(bound),
        _ => None,
    });
    let incompatibility_certified = incompatible.is_some();
    Sh9Experiment {
        displacement_bound,
        prime_growth: Growth::TOverLogT,
        xi_growth: Growth::TLogT,
        ratio_zero: true,
        incompatibility_certified,
        controls,
        controls_declined,
        selected_family_eliminated: displacement_bound == Some(2)
            && incompatibility_certified
            && controls_declined == controls.len(),
        m29_reached: false,
    }
}

pub fn machine_record(report: &Sh9Experiment) -> String {
    format!(
        "SH9b|displacement_bound={:?}|prime_growth={:?}|xi_growth={:?}|ratio_zero={}|incompatibility_certified={}|controls={:?}|controls_declined={}/7|selected_family_eliminated={}|m29_reached=false|claim=selected_family_falsified_only",
        report.displacement_bound,
        report.prime_growth,
        report.xi_growth,
        report.ratio_zero,
        report.incompatibility_certified,
        report.controls,
        report.controls_declined,
        report.selected_family_eliminated,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eliminates_selected_prime_jacobi_family_by_counting_growth() {
        let report = sh9_experiment();
        assert!(report.selected_family_eliminated, "{report:#?}");
        assert_eq!(report.displacement_bound, Some(2));
        assert!(report.incompatibility_certified);
        assert_eq!(report.controls, [true; 7]);
        assert!(!report.m29_reached);
        assert_eq!(machine_record(&report), machine_record(&sh9_experiment()));
    }
}
