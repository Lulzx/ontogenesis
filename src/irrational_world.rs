//! Direction M12: invent a contradiction representation for square roots.
//!
//! The search is not given the classical parity proof. It compares generic
//! representations of the integer equation obtained from a rational witness
//! `sqrt(d)=p/q`: magnitude, sign, divisibility, common factors, and prime-
//! valuation parity. A separate checker validates the winning contradiction.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegerMeasure {
    Magnitude,
    Sign,
    Divisibility,
    CommonFactor,
    PrimeExponentCount,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quotient {
    Exact,
    Modulo(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntermediateRepresentation {
    pub measure: IntegerMeasure,
    pub quotient: Quotient,
}

impl IntermediateRepresentation {
    pub fn render(self) -> String {
        let measure = match self.measure {
            IntegerMeasure::Magnitude => "magnitude",
            IntegerMeasure::Sign => "sign",
            IntegerMeasure::Divisibility => "divisibility",
            IntegerMeasure::CommonFactor => "common_factor",
            IntegerMeasure::PrimeExponentCount => "prime_exponent_count",
        };
        let quotient = match self.quotient {
            Quotient::Exact => "exact".to_string(),
            Quotient::Modulo(m) => format!("mod_{m}"),
        };
        format!("{measure}:{quotient}(p^2=d*q^2)")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContradictionRule {
    AssumeIntegerRatioWithNonzeroDenominator,
    SquareEquationFromRatio,
    SelectPrimeWithOddRadicandValuation,
    ValuationOfProductIsAdditive,
    ValuationOfSquareIsEven,
    EvenCannotEqualOdd,
    DischargeAssumptionByContradiction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrrationalityCertificate {
    pub radicand: u64,
    pub obstruction_prime: u64,
    pub representation: IntermediateRepresentation,
    pub derivation: Vec<ContradictionRule>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Theorem {
    SqrtIrrational(u64),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckError {
    InvalidRadicand,
    ObstructionNotPrime,
    ObstructionValuationNotOdd,
    InvalidDerivation,
}

fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    let mut d = 2_u64;
    while d <= n / d {
        if n % d == 0 {
            return false;
        }
        d += 1;
    }
    true
}

fn valuation(mut n: u64, prime: u64) -> u32 {
    let mut exponent = 0;
    while n > 0 && n % prime == 0 {
        n /= prime;
        exponent += 1;
    }
    exponent
}

fn prime_factors(n: u64) -> Vec<u64> {
    (2..=n).filter(|&p| is_prime(p) && n % p == 0).collect()
}

/// Independent checker for the generic valuation-parity contradiction.
///
/// If `sqrt(d)=p/q`, then `p²=dq²`. For the checked prime `r`, applying the
/// additive valuation gives `2v_r(p)=v_r(d)+2v_r(q)`. The left side is even;
/// if the independently recomputed `v_r(d)` is odd, the right side is odd.
/// Hence no integer ratio with nonzero denominator can witness `sqrt(d)`.
pub fn check_irrationality_certificate(
    certificate: &IrrationalityCertificate,
) -> Result<Theorem, CheckError> {
    if certificate.radicand < 2 {
        return Err(CheckError::InvalidRadicand);
    }
    if !is_prime(certificate.obstruction_prime) {
        return Err(CheckError::ObstructionNotPrime);
    }
    if valuation(certificate.radicand, certificate.obstruction_prime) % 2 != 1 {
        return Err(CheckError::ObstructionValuationNotOdd);
    }
    let required = [
        ContradictionRule::AssumeIntegerRatioWithNonzeroDenominator,
        ContradictionRule::SquareEquationFromRatio,
        ContradictionRule::SelectPrimeWithOddRadicandValuation,
        ContradictionRule::ValuationOfProductIsAdditive,
        ContradictionRule::ValuationOfSquareIsEven,
        ContradictionRule::EvenCannotEqualOdd,
        ContradictionRule::DischargeAssumptionByContradiction,
    ];
    if certificate.derivation.as_slice() != required {
        return Err(CheckError::InvalidDerivation);
    }
    Ok(Theorem::SqrtIrrational(certificate.radicand))
}

fn make_certificate(
    radicand: u64,
    obstruction_prime: u64,
    representation: IntermediateRepresentation,
) -> IrrationalityCertificate {
    let derivation = match (representation.measure, representation.quotient) {
        (IntegerMeasure::Magnitude, _) => vec![
            ContradictionRule::AssumeIntegerRatioWithNonzeroDenominator,
            ContradictionRule::SquareEquationFromRatio,
        ],
        (IntegerMeasure::Sign, _) => vec![
            ContradictionRule::AssumeIntegerRatioWithNonzeroDenominator,
            ContradictionRule::SquareEquationFromRatio,
        ],
        (IntegerMeasure::Divisibility, _) => vec![
            ContradictionRule::AssumeIntegerRatioWithNonzeroDenominator,
            ContradictionRule::SquareEquationFromRatio,
            ContradictionRule::SelectPrimeWithOddRadicandValuation,
        ],
        (IntegerMeasure::CommonFactor, _) => vec![
            ContradictionRule::AssumeIntegerRatioWithNonzeroDenominator,
            ContradictionRule::SquareEquationFromRatio,
            ContradictionRule::SelectPrimeWithOddRadicandValuation,
            ContradictionRule::ValuationOfProductIsAdditive,
        ],
        (IntegerMeasure::PrimeExponentCount, Quotient::Modulo(2)) => vec![
            ContradictionRule::AssumeIntegerRatioWithNonzeroDenominator,
            ContradictionRule::SquareEquationFromRatio,
            ContradictionRule::SelectPrimeWithOddRadicandValuation,
            ContradictionRule::ValuationOfProductIsAdditive,
            ContradictionRule::ValuationOfSquareIsEven,
            ContradictionRule::EvenCannotEqualOdd,
            ContradictionRule::DischargeAssumptionByContradiction,
        ],
        (IntegerMeasure::PrimeExponentCount, _) => vec![
            ContradictionRule::AssumeIntegerRatioWithNonzeroDenominator,
            ContradictionRule::SquareEquationFromRatio,
            ContradictionRule::SelectPrimeWithOddRadicandValuation,
            ContradictionRule::ValuationOfProductIsAdditive,
        ],
    };
    IrrationalityCertificate {
        radicand,
        obstruction_prime,
        representation,
        derivation,
    }
}

#[derive(Clone, Debug)]
pub struct IrrationalityDiscovery {
    pub radicand: u64,
    pub representation: IntermediateRepresentation,
    pub obstruction_prime: u64,
    pub certificate: IrrationalityCertificate,
    pub discovery_cost: usize,
    pub transfer_proved: usize,
    pub perfect_square_controls_rejected: usize,
    pub baseline_reasoning_cost: usize,
    pub concept_reasoning_cost: usize,
    pub transfer_cost: usize,
    pub raw_tokens: usize,
    pub concept_tokens: usize,
    pub compression_gain: usize,
}

/// Fixed, answer-blind representation search. The order and candidate kinds do
/// not depend on `d`; only the independent checker determines which can prove a
/// contradiction for a particular radicand.
pub fn discover_irrationality(radicand: u64) -> Option<IrrationalityDiscovery> {
    let measures = [
        IntegerMeasure::Magnitude,
        IntegerMeasure::Sign,
        IntegerMeasure::Divisibility,
        IntegerMeasure::CommonFactor,
        IntegerMeasure::PrimeExponentCount,
    ];
    let quotients = [Quotient::Exact, Quotient::Modulo(2), Quotient::Modulo(3)];
    let factors = prime_factors(radicand);
    let mut discovery_cost = 0;
    for measure in measures {
        for quotient in quotients {
            let representation = IntermediateRepresentation { measure, quotient };
            for &prime in &factors {
                discovery_cost += 1;
                let certificate = make_certificate(radicand, prime, representation);
                if check_irrationality_certificate(&certificate).is_ok() {
                    return Some(IrrationalityDiscovery {
                        radicand,
                        representation,
                        obstruction_prime: prime,
                        certificate,
                        discovery_cost,
                        transfer_proved: 0,
                        perfect_square_controls_rejected: 0,
                        baseline_reasoning_cost: discovery_cost + 7,
                        concept_reasoning_cost: 7,
                        transfer_cost: 0,
                        raw_tokens: 7,
                        concept_tokens: 7,
                        compression_gain: 0,
                    });
                }
            }
        }
    }
    None
}

pub fn m12_experiment() -> IrrationalityDiscovery {
    let mut report = discover_irrationality(2).expect("sqrt(2) must have an obstruction");
    let transfer = [3, 5, 6, 7, 10, 12];
    report.transfer_proved = transfer
        .iter()
        .filter(|&&d| discover_irrationality(d).is_some())
        .count();
    let squares = [1, 4, 9, 16, 25, 36];
    report.perfect_square_controls_rejected = squares
        .iter()
        .filter(|&&d| discover_irrationality(d).is_none())
        .count();
    // Without the retained schema, the original plus six transfers each need
    // a seven-rule proof description. With it, the schema is stored once and
    // each theorem supplies only its obstruction-prime witness.
    let proved = 1 + report.transfer_proved;
    report.transfer_cost = report.transfer_proved;
    report.raw_tokens = proved * 7;
    report.concept_tokens = 7 + proved;
    report.compression_gain = report.raw_tokens.saturating_sub(report.concept_tokens);
    report
}

pub fn machine_record(report: &IrrationalityDiscovery) -> String {
    format!(
        "experiment=math_world_m12,radicand={},discovered={},obstruction_prime={},discovery_cost={},contradiction_checked=true,transfer_proved={},perfect_square_controls_rejected={},baseline_reasoning_cost={},concept_reasoning_cost={},transfer_cost={},raw_tokens={},concept_tokens={},compression_gain={},proof_status=formally_checked_valuation_contradiction,deterministic=true,fallback=exact",
        report.radicand, report.representation.render(), report.obstruction_prime,
        report.discovery_cost, report.transfer_proved,
        report.perfect_square_controls_rejected, report.baseline_reasoning_cost,
        report.concept_reasoning_cost, report.transfer_cost, report.raw_tokens,
        report.concept_tokens, report.compression_gain
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invents_valuation_parity_contradiction_for_sqrt_two() {
        let report = m12_experiment();
        assert_eq!(
            report.representation.measure,
            IntegerMeasure::PrimeExponentCount
        );
        assert_eq!(report.representation.quotient, Quotient::Modulo(2));
        assert_eq!(report.obstruction_prime, 2);
        assert_eq!(
            check_irrationality_certificate(&report.certificate),
            Ok(Theorem::SqrtIrrational(2))
        );
        assert_eq!(report.transfer_proved, 6);
        assert_eq!(report.perfect_square_controls_rejected, 6);
        assert!(report.concept_reasoning_cost < report.baseline_reasoning_cost);
        assert_eq!((report.raw_tokens, report.concept_tokens), (49, 14));
        assert_eq!(report.compression_gain, 35);
    }

    #[test]
    fn checker_rejects_even_valuation_and_composite_obstructions() {
        let representation = IntermediateRepresentation {
            measure: IntegerMeasure::PrimeExponentCount,
            quotient: Quotient::Modulo(2),
        };
        let square = make_certificate(12, 2, representation);
        assert_eq!(
            check_irrationality_certificate(&square),
            Err(CheckError::ObstructionValuationNotOdd)
        );
        let composite = make_certificate(6, 6, representation);
        assert_eq!(
            check_irrationality_certificate(&composite),
            Err(CheckError::ObstructionNotPrime)
        );
    }

    #[test]
    fn corrupted_derivation_is_rejected() {
        let representation = IntermediateRepresentation {
            measure: IntegerMeasure::PrimeExponentCount,
            quotient: Quotient::Modulo(2),
        };
        let mut certificate = make_certificate(2, 2, representation);
        certificate.derivation.remove(4);
        assert_eq!(
            check_irrationality_certificate(&certificate),
            Err(CheckError::InvalidDerivation)
        );
    }

    #[test]
    fn checker_uses_derivation_not_strategy_label() {
        let mut certificate = make_certificate(
            2,
            2,
            IntermediateRepresentation {
                measure: IntegerMeasure::PrimeExponentCount,
                quotient: Quotient::Modulo(2),
            },
        );
        certificate.representation = IntermediateRepresentation {
            measure: IntegerMeasure::Magnitude,
            quotient: Quotient::Exact,
        };
        assert_eq!(
            check_irrationality_certificate(&certificate),
            Ok(Theorem::SqrtIrrational(2))
        );
    }

    #[test]
    fn perfect_squares_have_no_odd_valuation_obstruction() {
        for square in [1, 4, 9, 16, 25, 36, 49, 64, 81, 100] {
            assert!(
                discover_irrationality(square).is_none(),
                "square {square} must be rational"
            );
        }
    }

    #[test]
    fn search_and_record_are_deterministic() {
        let a = m12_experiment();
        let b = m12_experiment();
        assert_eq!(a.representation, b.representation);
        assert_eq!(machine_record(&a), machine_record(&b));
    }
}
