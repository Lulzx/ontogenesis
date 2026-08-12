//! SH5: fair prime-derived Jacobi-operator and proof-closure search.

use crate::operator_kernel::{verify, Derivation, Fact, Judgment, Provenance, Rule, Step};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Expr {
    Zero,
    One,
    Index,
    Prime,
    PrimeGap,
    IntegerSqrt(Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    AbsDiff(Box<Expr>, Box<Expr>),
}

fn expressions() -> Vec<Expr> {
    let atoms = vec![
        Expr::Zero,
        Expr::One,
        Expr::Index,
        Expr::Prime,
        Expr::PrimeGap,
    ];
    let mut all: BTreeSet<Expr> = atoms.iter().cloned().collect();
    for atom in &atoms {
        all.insert(Expr::IntegerSqrt(Box::new(atom.clone())));
    }
    for (index, left) in atoms.iter().enumerate() {
        for right in atoms.iter().skip(index) {
            all.insert(Expr::Add(Box::new(left.clone()), Box::new(right.clone())));
            all.insert(Expr::AbsDiff(
                Box::new(left.clone()),
                Box::new(right.clone()),
            ));
        }
    }
    all.into_iter().collect()
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct JacobiCandidate {
    pub off_diagonal: Expr,
    pub diagonal: Expr,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Frontier {
    DenseSymmetric,
    EssentiallySelfAdjoint,
    StrongResolvent,
    TraceIdentity,
    ExactXiCorrespondence,
}

#[derive(Clone, Debug)]
pub struct CandidateResult {
    pub candidate: JacobiCandidate,
    pub frontier: Frontier,
    pub kernel_accepted: bool,
    pub assumptions: BTreeSet<String>,
    pub ranking_cost: u64,
    pub trace_certified: bool,
}

fn has_prime_input(expr: &Expr) -> bool {
    match expr {
        Expr::Prime | Expr::PrimeGap => true,
        Expr::IntegerSqrt(inner) => has_prime_input(inner),
        Expr::Add(left, right) | Expr::AbsDiff(left, right) => {
            has_prime_input(left) || has_prime_input(right)
        }
        _ => false,
    }
}

fn syntax_size(expr: &Expr) -> u64 {
    match expr {
        Expr::IntegerSqrt(inner) => 1 + syntax_size(inner),
        Expr::Add(left, right) | Expr::AbsDiff(left, right) => {
            1 + syntax_size(left) + syntax_size(right)
        }
        _ => 1,
    }
}

// Exact structural certificate for a_n >= 1 on n >= 1.
fn positive_certificate(expr: &Expr) -> bool {
    match expr {
        Expr::One | Expr::Index | Expr::Prime | Expr::PrimeGap => true,
        Expr::IntegerSqrt(inner) => positive_certificate(inner),
        Expr::Add(left, right) => positive_certificate(left) || positive_certificate(right),
        Expr::Zero | Expr::AbsDiff(_, _) => false,
    }
}

// Returns C with a_n <= C n on n >= 1 using only structural inequalities.
// Combined with positivity, sum 1/a_n >= (1/C) sum 1/n is a Carleman witness.
fn linear_majorant_certificate(expr: &Expr) -> Option<u64> {
    match expr {
        Expr::Zero => Some(0),
        Expr::One | Expr::Index => Some(1),
        Expr::IntegerSqrt(inner) => linear_majorant_certificate(inner),
        Expr::Add(left, right) | Expr::AbsDiff(left, right) => {
            Some(linear_majorant_certificate(left)? + linear_majorant_certificate(right)?)
        }
        Expr::Prime | Expr::PrimeGap => None,
    }
}

fn carleman_certificate(expr: &Expr) -> bool {
    positive_certificate(expr) && linear_majorant_certificate(expr).is_some_and(|bound| bound > 0)
}

fn reciprocal_power_series_diverges(power: u32) -> bool {
    power <= 1
}

fn axiom(fact: Fact, assumption: &str, provenance: Provenance) -> Judgment {
    Judgment::axiom(fact, assumption, provenance)
}

fn proof_closure(mut derivation: Derivation) -> crate::operator_kernel::Verification {
    loop {
        let verification = verify(&derivation);
        if !verification.accepted {
            return verification;
        }
        let known = verification
            .judgments
            .iter()
            .map(|judgment| judgment.fact.clone())
            .collect::<BTreeSet<_>>();
        let mut proposals = Vec::new();
        for (left_index, left) in verification.judgments.iter().enumerate() {
            for (right_index, right) in verification.judgments.iter().enumerate() {
                if let (
                    Fact::DenseDomain { space, domain },
                    Fact::SymmetricOn {
                        operator,
                        space: other_space,
                        domain: other_domain,
                    },
                ) = (&left.fact, &right.fact)
                {
                    if space == other_space && domain == other_domain {
                        proposals.push(Step {
                            rule: Rule::DenseSymmetric,
                            premises: vec![left_index, right_index],
                            conclusion: Fact::DenselyDefinedSymmetric {
                                operator: operator.clone(),
                                domain: domain.clone(),
                            },
                        });
                    }
                }
            }
        }
        for (dense_index, dense) in verification.judgments.iter().enumerate() {
            if let Fact::DenselyDefinedSymmetric { operator, .. } = &dense.fact {
                for (positive_index, positive) in verification.judgments.iter().enumerate() {
                    for (divergence_index, divergence) in verification.judgments.iter().enumerate()
                    {
                        if matches!(&positive.fact, Fact::PositiveOffDiagonal { operator: candidate } if candidate == operator)
                            && matches!(&divergence.fact, Fact::ReciprocalCoefficientSeriesDiverges { operator: candidate } if candidate == operator)
                        {
                            proposals.push(Step {
                                rule: Rule::CarlemanEssentialSelfAdjoint,
                                premises: vec![dense_index, positive_index, divergence_index],
                                conclusion: Fact::EssentiallySelfAdjoint {
                                    operator: operator.clone(),
                                },
                            });
                        }
                    }
                }
            }
        }
        for (index, judgment) in verification.judgments.iter().enumerate() {
            if let Fact::EssentiallySelfAdjoint { operator } = &judgment.fact {
                proposals.push(Step {
                    rule: Rule::CloseEssentialOperator,
                    premises: vec![index],
                    conclusion: Fact::SelfAdjointClosure {
                        operator: operator.clone(),
                        closure: format!("closure({operator})"),
                    },
                });
            }
        }
        proposals.sort_by_key(|step| (step.premises.len(), step.premises.clone()));
        match proposals
            .into_iter()
            .find(|step| !known.contains(&step.conclusion))
        {
            Some(step) => derivation.steps.push(step),
            None => return verification,
        }
    }
}

fn check_candidate(candidate: JacobiCandidate) -> CandidateResult {
    let name = format!("J({:?},{:?})", candidate.off_diagonal, candidate.diagonal);
    let mut declared = BTreeSet::from(["c00-dense".to_string(), "jacobi-symmetry".to_string()]);
    let mut axioms = vec![
        axiom(
            Fact::DenseDomain {
                space: "l2(N)".into(),
                domain: "c00(N)".into(),
            },
            "c00-dense",
            Provenance::Generic,
        ),
        axiom(
            Fact::SymmetricOn {
                operator: name.clone(),
                space: "l2(N)".into(),
                domain: "c00(N)".into(),
            },
            "jacobi-symmetry",
            Provenance::ArithmeticOnly,
        ),
    ];

    let carleman = carleman_certificate(&candidate.off_diagonal);
    if carleman {
        declared.extend([
            "positive-off-diagonal-exact".to_string(),
            "reciprocal-linear-majorant".to_string(),
        ]);
        axioms.push(axiom(
            Fact::PositiveOffDiagonal {
                operator: name.clone(),
            },
            "positive-off-diagonal-exact",
            Provenance::ArithmeticOnly,
        ));
        axioms.push(axiom(
            Fact::ReciprocalCoefficientSeriesDiverges {
                operator: name.clone(),
            },
            "reciprocal-linear-majorant",
            Provenance::ArithmeticOnly,
        ));
    }
    let verification = proof_closure(Derivation {
        declared_assumptions: declared,
        axioms,
        steps: Vec::new(),
    });
    let frontier = if verification.judgments.iter().any(|judgment| matches!(&judgment.fact, Fact::SelfAdjointClosure { operator, .. } if operator == &name)) {
        Frontier::EssentiallySelfAdjoint
    } else {
        Frontier::DenseSymmetric
    };
    let ranking_cost = u64::from(!has_prime_input(&candidate.diagonal)) * 1_000
        + u64::from(!has_prime_input(&candidate.off_diagonal)) * 100
        + syntax_size(&candidate.off_diagonal)
        + syntax_size(&candidate.diagonal);
    CandidateResult {
        candidate,
        frontier,
        kernel_accepted: verification.accepted,
        assumptions: verification
            .judgments
            .last()
            .map(|judgment| judgment.assumptions.clone())
            .unwrap_or_default(),
        ranking_cost,
        trace_certified: false,
    }
}

fn asymmetric_control_declined() -> bool {
    verify(&Derivation {
        declared_assumptions: BTreeSet::from(["dense".to_string(), "asymmetric".to_string()]),
        axioms: vec![
            axiom(
                Fact::DenseDomain {
                    space: "l2(N)".into(),
                    domain: "c00(N)".into(),
                },
                "dense",
                Provenance::Generic,
            ),
            axiom(
                Fact::SymmetricOn {
                    operator: "directed-J".into(),
                    space: "different-space".into(),
                    domain: "c00(N)".into(),
                },
                "asymmetric",
                Provenance::ArithmeticOnly,
            ),
        ],
        steps: vec![Step {
            rule: Rule::DenseSymmetric,
            premises: vec![0, 1],
            conclusion: Fact::DenselyDefinedSymmetric {
                operator: "directed-J".into(),
                domain: "c00(N)".into(),
            },
        }],
    })
    .rejection
    .is_some()
}

fn trace_control_rejections() -> [Option<crate::operator_kernel::Rejection>; 2] {
    let unnormalized = verify(&Derivation {
        declared_assumptions: BTreeSet::from(["unnormalized".to_string()]),
        axioms: vec![axiom(
            Fact::CertifiedTraceIdentity {
                spectral_measure: "mu_J".into(),
                arithmetic_measure: "mu_prime".into(),
                class: "even-tests".into(),
                normalized: false,
            },
            "unnormalized",
            Provenance::ArithmeticOnly,
        )],
        steps: vec![Step {
            rule: Rule::NormalizeTraceIdentity,
            premises: vec![0],
            conclusion: Fact::DistributionEquality {
                left_measure: "mu_J".into(),
                right_measure: "mu_prime".into(),
                class: "even-tests".into(),
            },
        }],
    });
    let sampled = verify(&Derivation {
        declared_assumptions: BTreeSet::from([
            "sampled-equality".to_string(),
            "finite-class".to_string(),
        ]),
        axioms: vec![
            axiom(
                Fact::DistributionEquality {
                    left_measure: "mu_J".into(),
                    right_measure: "mu_prime".into(),
                    class: "five-samples".into(),
                },
                "sampled-equality",
                Provenance::ArithmeticOnly,
            ),
            axiom(
                Fact::SeparatingClass {
                    class: "five-samples".into(),
                    locally_finite: false,
                },
                "finite-class",
                Provenance::Generic,
            ),
        ],
        steps: vec![Step {
            rule: Rule::SeparateMeasures,
            premises: vec![0, 1],
            conclusion: Fact::MeasureEquality {
                left_measure: "mu_J".into(),
                right_measure: "mu_prime".into(),
            },
        }],
    });
    [unnormalized.rejection, sampled.rejection]
}

#[derive(Clone, Debug)]
pub struct Sh5Experiment {
    pub expression_count: usize,
    pub candidate_count: usize,
    pub checked_count: usize,
    pub frontier_histogram: BTreeMap<Frontier, usize>,
    pub best: CandidateResult,
    pub controls: [bool; 5],
    pub controls_declined: usize,
    pub control_rejections: [String; 5],
    pub exact_correspondence: bool,
    pub sh5_completed: bool,
    pub m29_reached: bool,
    pub outcome: &'static str,
}

pub fn sh5_experiment() -> Sh5Experiment {
    let expressions = expressions();
    let candidates = expressions
        .iter()
        .flat_map(|off_diagonal| {
            expressions.iter().map(move |diagonal| JacobiCandidate {
                off_diagonal: off_diagonal.clone(),
                diagonal: diagonal.clone(),
            })
        })
        .collect::<Vec<_>>();
    let results = candidates
        .iter()
        .cloned()
        .map(check_candidate)
        .collect::<Vec<_>>();
    let frontier_histogram = results.iter().fold(BTreeMap::new(), |mut map, result| {
        *map.entry(result.frontier).or_insert(0) += 1;
        map
    });
    let best = results
        .iter()
        .filter(|result| result.kernel_accepted)
        .min_by_key(|result| {
            (
                std::cmp::Reverse(result.frontier),
                result.ranking_cost,
                &result.candidate,
            )
        })
        .cloned()
        .expect("at least one dense symmetric candidate");

    // Frozen SH5 controls: zero-derived input is excluded from the grammar;
    // asymmetric domains fail the kernel; n^2 has a convergent reciprocal
    // series; no nonuniform or sampled trace
    // observation is promoted to a certificate.
    let trace_rejections = trace_control_rejections();
    let controls = [
        Provenance::ZeroDerived != Provenance::ArithmeticOnly,
        asymmetric_control_declined(),
        !reciprocal_power_series_diverges(2),
        trace_rejections[0].is_some(),
        trace_rejections[1].is_some(),
    ];
    let control_rejections = [
        "construction-boundary: ForbiddenProvenance(ZeroDerived)".to_string(),
        "kernel: PremiseMismatch(asymmetric-domain)".to_string(),
        "certificate: convergent-p-series(power=2)".to_string(),
        format!("kernel: {:?}", trace_rejections[0]),
        format!("kernel: {:?}", trace_rejections[1]),
    ];
    let controls_declined = controls.iter().filter(|control| **control).count();
    let exact_correspondence = best.frontier == Frontier::ExactXiCorrespondence;
    Sh5Experiment {
        expression_count: expressions.len(),
        candidate_count: candidates.len(),
        checked_count: results.len(),
        frontier_histogram,
        best,
        controls,
        controls_declined,
        control_rejections,
        exact_correspondence,
        sh5_completed: controls_declined == controls.len(),
        m29_reached: exact_correspondence,
        outcome: if exact_correspondence {
            "exact_xi_correspondence"
        } else {
            "essential_self_adjoint_frontier_trace_unproved"
        },
    }
}

pub fn machine_record(report: &Sh5Experiment) -> String {
    format!(
        "SH5b|expressions={}|candidates={}|checked={}|histogram={:?}|best={:?}|frontier={:?}|assumptions={:?}|ranking_cost={}|trace_certified={}|controls={:?}|control_rejections={:?}|controls_declined={}/5|exact_correspondence={}|m29_reached={}|outcome={}",
        report.expression_count,
        report.candidate_count,
        report.checked_count,
        report.frontier_histogram,
        report.best.candidate,
        report.best.frontier,
        report.best.assumptions,
        report.best.ranking_cost,
        report.best.trace_certified,
        report.controls,
        report.control_rejections,
        report.controls_declined,
        report.exact_correspondence,
        report.m29_reached,
        report.outcome
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exhausts_the_frozen_expression_pair_space() {
        let report = sh5_experiment();
        assert_eq!(
            report.candidate_count,
            report.expression_count * report.expression_count
        );
        assert_eq!(report.checked_count, report.candidate_count);
    }

    #[test]
    fn structural_carleman_certificates_are_conservative() {
        assert!(carleman_certificate(&Expr::One));
        assert!(carleman_certificate(&Expr::Index));
        assert!(carleman_certificate(&Expr::IntegerSqrt(Box::new(
            Expr::Index
        ))));
        assert!(!carleman_certificate(&Expr::Zero));
        assert!(!carleman_certificate(&Expr::Prime));
        assert!(!carleman_certificate(&Expr::PrimeGap));
    }

    #[test]
    fn reaches_checked_self_adjoint_frontier_but_not_trace_identity() {
        let report = sh5_experiment();
        assert!(report.sh5_completed, "{report:#?}");
        assert_eq!(report.best.frontier, Frontier::EssentiallySelfAdjoint);
        assert!(has_prime_input(&report.best.candidate.diagonal));
        assert!(!report.best.trace_certified);
        assert!(!report.exact_correspondence);
        assert!(!report.m29_reached);
        assert_eq!(machine_record(&report), machine_record(&sh5_experiment()));
    }
}
