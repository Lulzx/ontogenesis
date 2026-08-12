//! SH13: construct spectral operators from certified positive functionals.

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Provenance {
    Generic,
    ArithmeticOnly,
    ZeroDerived,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Fact {
    UnitalCommutativeStarAlgebra {
        algebra: String,
    },
    NormalizedLinearFunctional {
        algebra: String,
        functional: String,
    },
    PositiveFunctional {
        functional: String,
    },
    NullSpaceTwoSidedIdeal {
        functional: String,
    },
    GnsHilbertSpace {
        functional: String,
        space: String,
        cyclic_vector: String,
    },
    RealCoordinate {
        algebra: String,
        coordinate: String,
    },
    CoordinateEssentiallySelfAdjoint {
        functional: String,
        coordinate: String,
    },
    CoordinateSymmetricOnly {
        functional: String,
        coordinate: String,
    },
    SpectralRepresentation {
        functional: String,
        operator: String,
        measure: String,
        class: String,
    },
    SeparatingInfiniteClass {
        class: String,
    },
    FiniteNonseparatingClass {
        class: String,
    },
    ExactMeasureRepresentation {
        functional: String,
        measure: String,
    },
    WeilExplicitFunctional {
        functional: String,
        class: String,
        archimedean: bool,
        pole: bool,
        prime_power: bool,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Judgment {
    fact: Fact,
    provenance: Vec<Provenance>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Rule {
    BuildGns,
    CoordinateSpectralRepresentation,
    SeparateMeasure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Rejection {
    WrongArity,
    PremiseMismatch,
    PositivityMissing,
    NullIdealMissing,
    EssentialSelfAdjointnessMissing,
    NonseparatingClass,
    ForbiddenProvenance,
}

fn combined(fact: Fact, premises: &[&Judgment]) -> Judgment {
    let mut provenance = premises
        .iter()
        .flat_map(|premise| premise.provenance.iter().copied())
        .collect::<Vec<_>>();
    provenance.sort();
    provenance.dedup();
    Judgment { fact, provenance }
}

fn apply(rule: Rule, premises: &[&Judgment]) -> Result<Judgment, Rejection> {
    if premises
        .iter()
        .any(|premise| premise.provenance.contains(&Provenance::ZeroDerived))
    {
        return Err(Rejection::ForbiddenProvenance);
    }
    match rule {
        Rule::BuildGns => {
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
                    Fact::UnitalCommutativeStarAlgebra { algebra },
                    Fact::NormalizedLinearFunctional {
                        algebra: functional_algebra,
                        functional,
                    },
                    Fact::PositiveFunctional {
                        functional: positive,
                    },
                    Fact::NullSpaceTwoSidedIdeal { functional: ideal },
                ) if algebra == functional_algebra
                    && functional == positive
                    && functional == ideal =>
                {
                    Ok(combined(
                        Fact::GnsHilbertSpace {
                            functional: functional.clone(),
                            space: format!("GNS({functional})"),
                            cyclic_vector: "Omega".into(),
                        },
                        premises,
                    ))
                }
                (_, _, Fact::CoordinateSymmetricOnly { .. }, _) => {
                    Err(Rejection::PositivityMissing)
                }
                (_, _, Fact::PositiveFunctional { .. }, Fact::CoordinateSymmetricOnly { .. }) => {
                    Err(Rejection::NullIdealMissing)
                }
                _ => Err(Rejection::PremiseMismatch),
            }
        }
        Rule::CoordinateSpectralRepresentation => {
            if premises.len() != 3 {
                return Err(Rejection::WrongArity);
            }
            match (&premises[0].fact, &premises[1].fact, &premises[2].fact) {
                (
                    Fact::GnsHilbertSpace { functional, .. },
                    Fact::RealCoordinate { coordinate, .. },
                    Fact::CoordinateEssentiallySelfAdjoint {
                        functional: essential_functional,
                        coordinate: essential_coordinate,
                    },
                ) if functional == essential_functional && coordinate == essential_coordinate => {
                    Ok(combined(
                        Fact::SpectralRepresentation {
                            functional: functional.clone(),
                            operator: format!("closure(M_{coordinate})"),
                            measure: format!("mu_{functional}"),
                            class: "A".into(),
                        },
                        premises,
                    ))
                }
                (
                    Fact::GnsHilbertSpace { .. },
                    Fact::RealCoordinate { .. },
                    Fact::CoordinateSymmetricOnly { .. },
                ) => Err(Rejection::EssentialSelfAdjointnessMissing),
                _ => Err(Rejection::PremiseMismatch),
            }
        }
        Rule::SeparateMeasure => {
            if premises.len() != 2 {
                return Err(Rejection::WrongArity);
            }
            match (&premises[0].fact, &premises[1].fact) {
                (
                    Fact::SpectralRepresentation {
                        functional,
                        measure,
                        class,
                        ..
                    },
                    Fact::SeparatingInfiniteClass { class: separating },
                ) if class == separating => Ok(combined(
                    Fact::ExactMeasureRepresentation {
                        functional: functional.clone(),
                        measure: measure.clone(),
                    },
                    premises,
                )),
                (Fact::SpectralRepresentation { .. }, Fact::FiniteNonseparatingClass { .. }) => {
                    Err(Rejection::NonseparatingClass)
                }
                _ => Err(Rejection::PremiseMismatch),
            }
        }
    }
}

fn judgment(fact: Fact, provenance: Provenance) -> Judgment {
    Judgment {
        fact,
        provenance: vec![provenance],
    }
}

fn calibrated_domain(name: &str) -> bool {
    let algebra = judgment(
        Fact::UnitalCommutativeStarAlgebra {
            algebra: format!("A_{name}"),
        },
        Provenance::Generic,
    );
    let linear = judgment(
        Fact::NormalizedLinearFunctional {
            algebra: format!("A_{name}"),
            functional: format!("L_{name}"),
        },
        Provenance::Generic,
    );
    let positive = judgment(
        Fact::PositiveFunctional {
            functional: format!("L_{name}"),
        },
        Provenance::Generic,
    );
    let ideal = judgment(
        Fact::NullSpaceTwoSidedIdeal {
            functional: format!("L_{name}"),
        },
        Provenance::Generic,
    );
    let gns = apply(Rule::BuildGns, &[&algebra, &linear, &positive, &ideal]).ok();
    let coordinate = judgment(
        Fact::RealCoordinate {
            algebra: format!("A_{name}"),
            coordinate: "x".into(),
        },
        Provenance::Generic,
    );
    let essential = judgment(
        Fact::CoordinateEssentiallySelfAdjoint {
            functional: format!("L_{name}"),
            coordinate: "x".into(),
        },
        Provenance::Generic,
    );
    let representation = gns.as_ref().and_then(|gns| {
        apply(
            Rule::CoordinateSpectralRepresentation,
            &[gns, &coordinate, &essential],
        )
        .ok()
    });
    let separating = judgment(
        Fact::SeparatingInfiniteClass { class: "A".into() },
        Provenance::Generic,
    );
    representation.as_ref().is_some_and(|representation| {
        apply(Rule::SeparateMeasure, &[representation, &separating]).is_ok()
    })
}

#[derive(Clone, Debug)]
pub struct Sh13Experiment {
    pub calibration_domains: [bool; 3],
    pub controls: [bool; 5],
    pub controls_declined: usize,
    pub weil_functional_well_formed: bool,
    pub weil_positivity_certified: bool,
    pub weil_gns_constructed: bool,
    pub first_missing_premise: &'static str,
    pub m29_reached: bool,
}

pub fn sh13_experiment() -> Sh13Experiment {
    let calibration_domains = [
        calibrated_domain("weighted_integers"),
        calibrated_domain("haar_circle"),
        calibrated_domain("graph_moments"),
    ];
    let algebra = judgment(
        Fact::UnitalCommutativeStarAlgebra {
            algebra: "A_weil".into(),
        },
        Provenance::Generic,
    );
    let linear = judgment(
        Fact::NormalizedLinearFunctional {
            algebra: "A_weil".into(),
            functional: "L_weil".into(),
        },
        Provenance::ArithmeticOnly,
    );
    let ideal = judgment(
        Fact::NullSpaceTwoSidedIdeal {
            functional: "L_weil".into(),
        },
        Provenance::ArithmeticOnly,
    );
    let merely_symmetric = judgment(
        Fact::CoordinateSymmetricOnly {
            functional: "L_weil".into(),
            coordinate: "x".into(),
        },
        Provenance::ArithmeticOnly,
    );
    let finite = judgment(
        Fact::FiniteNonseparatingClass {
            class: "finite-tests".into(),
        },
        Provenance::Generic,
    );
    let fake_representation = judgment(
        Fact::SpectralRepresentation {
            functional: "L_weil".into(),
            operator: "T".into(),
            measure: "mu".into(),
            class: "finite-tests".into(),
        },
        Provenance::ArithmeticOnly,
    );
    let zero_positive = judgment(
        Fact::PositiveFunctional {
            functional: "L_weil".into(),
        },
        Provenance::ZeroDerived,
    );
    let controls = [
        apply(
            Rule::BuildGns,
            &[&algebra, &linear, &merely_symmetric, &ideal],
        )
        .is_err(),
        apply(
            Rule::BuildGns,
            &[
                &algebra,
                &linear,
                &judgment(
                    Fact::PositiveFunctional {
                        functional: "L_weil".into(),
                    },
                    Provenance::ArithmeticOnly,
                ),
                &merely_symmetric,
            ],
        )
        .is_err(),
        apply(
            Rule::CoordinateSpectralRepresentation,
            &[
                &judgment(
                    Fact::GnsHilbertSpace {
                        functional: "L_weil".into(),
                        space: "H".into(),
                        cyclic_vector: "Omega".into(),
                    },
                    Provenance::ArithmeticOnly,
                ),
                &judgment(
                    Fact::RealCoordinate {
                        algebra: "A_weil".into(),
                        coordinate: "x".into(),
                    },
                    Provenance::Generic,
                ),
                &merely_symmetric,
            ],
        )
        .is_err(),
        apply(Rule::SeparateMeasure, &[&fake_representation, &finite])
            == Err(Rejection::NonseparatingClass),
        apply(Rule::BuildGns, &[&algebra, &linear, &zero_positive, &ideal])
            == Err(Rejection::ForbiddenProvenance),
    ];
    let weil_functional = Fact::WeilExplicitFunctional {
        functional: "L_weil".into(),
        class: "A_weil".into(),
        archimedean: true,
        pole: true,
        prime_power: true,
    };
    Sh13Experiment {
        calibration_domains,
        controls_declined: controls.iter().filter(|control| **control).count(),
        controls,
        weil_functional_well_formed: matches!(
            weil_functional,
            Fact::WeilExplicitFunctional {
                archimedean: true,
                pole: true,
                prime_power: true,
                ..
            }
        ),
        weil_positivity_certified: false,
        weil_gns_constructed: false,
        first_missing_premise: "PositiveFunctional(L_weil)",
        m29_reached: false,
    }
}

pub fn machine_record(report: &Sh13Experiment) -> String {
    format!("SH13|calibration_domains={:?}|controls={:?}|controls_declined={}/5|weil_functional_well_formed={}|weil_positivity_certified={}|weil_gns_constructed={}|first_missing_premise={}|m29_reached=false|claim=generic_positive_functional_constructor_only", report.calibration_domains, report.controls, report.controls_declined, report.weil_functional_well_formed, report.weil_positivity_certified, report.weil_gns_constructed, report.first_missing_premise)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transfers_constructor_and_stops_at_real_weil_positivity() {
        let report = sh13_experiment();
        assert_eq!(report.calibration_domains, [true; 3]);
        assert_eq!(report.controls, [true; 5]);
        assert!(report.weil_functional_well_formed);
        assert!(!report.weil_positivity_certified);
        assert!(!report.weil_gns_constructed);
        assert!(!report.m29_reached);
        assert_eq!(machine_record(&report), machine_record(&sh13_experiment()));
    }
}
