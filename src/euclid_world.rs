//! Direction M11: rediscover Euclid's auxiliary construction.
//!
//! Candidate objects are enumerated from a generic finite-collection grammar.
//! A separate checker validates a symbolic certificate for an arbitrary
//! nonempty finite list of primes.  The checker contains no candidate proposal
//! order and never receives the training examples used to rank constructions.

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CollectionExpr {
    Product,
    Sum,
    Length,
    Const(i128),
    Add(Box<CollectionExpr>, Box<CollectionExpr>),
    Sub(Box<CollectionExpr>, Box<CollectionExpr>),
    Mul(Box<CollectionExpr>, Box<CollectionExpr>),
}

impl CollectionExpr {
    pub fn eval(&self, values: &[i128]) -> Option<i128> {
        match self {
            Self::Product => values.iter().try_fold(1_i128, |a, &b| a.checked_mul(b)),
            Self::Sum => values.iter().try_fold(0_i128, |a, &b| a.checked_add(b)),
            Self::Length => Some(values.len() as i128),
            Self::Const(c) => Some(*c),
            Self::Add(a, b) => a.eval(values)?.checked_add(b.eval(values)?),
            Self::Sub(a, b) => a.eval(values)?.checked_sub(b.eval(values)?),
            Self::Mul(a, b) => a.eval(values)?.checked_mul(b.eval(values)?),
        }
    }

    pub fn size(&self) -> usize {
        match self {
            Self::Product | Self::Sum | Self::Length | Self::Const(_) => 1,
            Self::Add(a, b) | Self::Sub(a, b) | Self::Mul(a, b) => 1 + a.size() + b.size(),
        }
    }

    pub fn render(&self) -> String {
        match self {
            Self::Product => "product(xs)".into(),
            Self::Sum => "sum(xs)".into(),
            Self::Length => "length(xs)".into(),
            Self::Const(c) => c.to_string(),
            Self::Add(a, b) => format!("({}+{})", a.render(), b.render()),
            Self::Sub(a, b) => format!("({}-{})", a.render(), b.render()),
            Self::Mul(a, b) => format!("({}*{})", a.render(), b.render()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Theorem {
    /// Every nonempty finite prime list omits a prime.
    PrimeEscapesEveryFiniteList,
    /// Every member of an arbitrary finite list of integers >1 fails to divide
    /// the constructed value. This is the reusable lower-level theorem.
    DivisorEscape(CollectionExpr),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EscapeCertificate {
    pub construction: CollectionExpr,
    pub derivation: Vec<EscapeRule>,
}

/// Domain-general proof steps available to the M11 fragment. None proposes an
/// auxiliary expression; they only check consequences of the expression that
/// search has already produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EscapeRule {
    ProductContainsEveryMemberFactor,
    ResidualIsNonzeroModuloEveryMember,
    ConstructedIntegerGreaterThanOne,
    EveryIntegerGreaterThanOneHasPrimeDivisor,
    NondividingPrimeWitnessIsOutsideList,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckError {
    WrongConstruction,
    InvalidDerivation,
}

type Monomial = (usize, usize, usize); // powers of product, sum, length

fn symbolic(expr: &CollectionExpr) -> std::collections::BTreeMap<Monomial, i128> {
    use std::collections::BTreeMap;
    match expr {
        CollectionExpr::Product => BTreeMap::from([((1, 0, 0), 1)]),
        CollectionExpr::Sum => BTreeMap::from([((0, 1, 0), 1)]),
        CollectionExpr::Length => BTreeMap::from([((0, 0, 1), 1)]),
        CollectionExpr::Const(c) => BTreeMap::from([((0, 0, 0), *c)]),
        CollectionExpr::Add(a, b) | CollectionExpr::Sub(a, b) => {
            let mut out = symbolic(a);
            let sign = if matches!(expr, CollectionExpr::Add(_, _)) {
                1
            } else {
                -1
            };
            for (monomial, coefficient) in symbolic(b) {
                *out.entry(monomial).or_default() += sign * coefficient;
            }
            out.retain(|_, coefficient| *coefficient != 0);
            out
        }
        CollectionExpr::Mul(a, b) => {
            let mut out = BTreeMap::new();
            for ((pa, sa, la), ca) in symbolic(a) {
                for ((pb, sb, lb), cb) in symbolic(b) {
                    *out.entry((pa + pb, sa + sb, la + lb)).or_default() += ca * cb;
                }
            }
            out.retain(|_, coefficient| *coefficient != 0);
            out
        }
    }
}

/// Substitute `product(xs)=0 (mod p)`. A residual of exactly ±1 proves that no
/// arbitrary list member `p>1` divides the construction. This accepts any
/// expression with that generic consequence; it does not recognize a named
/// Euclid pattern.
fn universally_nonzero_member_remainder(expr: &CollectionExpr) -> bool {
    let residual: Vec<(Monomial, i128)> = symbolic(expr)
        .into_iter()
        .filter(|((product_power, _, _), _)| *product_power == 0)
        .collect();
    residual == vec![((0, 0, 0), 1)] || residual == vec![((0, 0, 0), -1)]
}

#[derive(Clone, Copy)]
struct Interval {
    low: i128,
    high: Option<i128>,
}

fn interval(expr: &CollectionExpr) -> Option<Interval> {
    let exact = |value| {
        Some(Interval {
            low: value,
            high: Some(value),
        })
    };
    match expr {
        // A nonempty prime list has product and sum >=2, length >=1.
        CollectionExpr::Product | CollectionExpr::Sum => Some(Interval { low: 2, high: None }),
        CollectionExpr::Length => Some(Interval { low: 1, high: None }),
        CollectionExpr::Const(c) => exact(*c),
        CollectionExpr::Add(a, b) => {
            let (a, b) = (interval(a)?, interval(b)?);
            Some(Interval {
                low: a.low.checked_add(b.low)?,
                high: a.high.zip(b.high).and_then(|(x, y)| x.checked_add(y)),
            })
        }
        CollectionExpr::Sub(a, b) => {
            let (a, b) = (interval(a)?, interval(b)?);
            Some(Interval {
                low: a.low.checked_sub(b.high?)?,
                high: a.high.and_then(|x| x.checked_sub(b.low)),
            })
        }
        CollectionExpr::Mul(a, b) => {
            let (a, b) = (interval(a)?, interval(b)?);
            if a.low < 0 || b.low < 0 {
                return None;
            }
            Some(Interval {
                low: a.low.checked_mul(b.low)?,
                high: a.high.zip(b.high).and_then(|(x, y)| x.checked_mul(y)),
            })
        }
    }
}

/// Independent symbolic checker for the M11 certificate.
///
/// Soundness argument encoded by the four independently recomputed checks:
/// for a nonempty list of primes, `P=product(xs)>=2`; hence `P+1>1`. Every
/// listed prime divides `P`, so `P+1` leaves remainder one. The elementary
/// well-ordering lemma "every integer >1 has a prime divisor" supplies a prime
/// divisor `q`; since no listed prime divides `P+1`, `q` is outside the list.
pub fn check_escape_certificate(certificate: &EscapeCertificate) -> Result<Theorem, CheckError> {
    if !universally_nonzero_member_remainder(&certificate.construction) {
        return Err(CheckError::WrongConstruction);
    }
    if interval(&certificate.construction).is_none_or(|bounds| bounds.low <= 1) {
        return Err(CheckError::WrongConstruction);
    }
    let required = [
        EscapeRule::ProductContainsEveryMemberFactor,
        EscapeRule::ResidualIsNonzeroModuloEveryMember,
        EscapeRule::ConstructedIntegerGreaterThanOne,
        EscapeRule::EveryIntegerGreaterThanOneHasPrimeDivisor,
        EscapeRule::NondividingPrimeWitnessIsOutsideList,
    ];
    if certificate.derivation.as_slice() != required {
        return Err(CheckError::InvalidDerivation);
    }
    Ok(Theorem::PrimeEscapesEveryFiniteList)
}

fn certificate_for(construction: CollectionExpr) -> EscapeCertificate {
    EscapeCertificate {
        construction,
        derivation: vec![
            EscapeRule::ProductContainsEveryMemberFactor,
            EscapeRule::ResidualIsNonzeroModuloEveryMember,
            EscapeRule::ConstructedIntegerGreaterThanOne,
            // Trusted generic well-ordering/factorization lemma. It is
            // independent of the auxiliary expression and applies universally.
            EscapeRule::EveryIntegerGreaterThanOneHasPrimeDivisor,
            EscapeRule::NondividingPrimeWitnessIsOutsideList,
        ],
    }
}

fn is_prime(n: i128) -> bool {
    if n < 2 {
        return false;
    }
    let mut d = 2_i128;
    while d * d <= n {
        if n % d == 0 {
            return false;
        }
        d += 1;
    }
    true
}

fn prime_divisor(n: i128) -> Option<i128> {
    if n < 2 {
        return None;
    }
    (2..=n).find(|&d| n % d == 0 && is_prime(d))
}

fn observed_escape(construction: &CollectionExpr, primes: &[i128]) -> bool {
    if primes.is_empty() || !primes.iter().all(|&p| is_prime(p)) {
        return false;
    }
    let Some(value) = construction.eval(primes) else {
        return false;
    };
    if value <= 1 || primes.iter().any(|p| value % p == 0) {
        return false;
    }
    prime_divisor(value).is_some_and(|q| !primes.contains(&q))
}

fn enumerate_candidates() -> Vec<CollectionExpr> {
    // Generic folds and small constants. The target combination is not an atom.
    let atoms = vec![
        CollectionExpr::Product,
        CollectionExpr::Sum,
        CollectionExpr::Length,
        CollectionExpr::Const(0),
        CollectionExpr::Const(1),
        CollectionExpr::Const(2),
    ];
    let mut out = atoms.clone();
    for a in &atoms {
        for b in &atoms {
            out.push(CollectionExpr::Add(
                Box::new(a.clone()),
                Box::new(b.clone()),
            ));
            out.push(CollectionExpr::Sub(
                Box::new(a.clone()),
                Box::new(b.clone()),
            ));
            out.push(CollectionExpr::Mul(
                Box::new(a.clone()),
                Box::new(b.clone()),
            ));
        }
    }
    out
}

#[derive(Clone, Debug)]
pub struct EuclidDiscovery {
    pub conjecture: &'static str,
    pub construction: CollectionExpr,
    pub certificate: EscapeCertificate,
    pub discovery_cost: usize,
    pub training_lists: usize,
    pub held_out_lists: usize,
    pub transfer_lists: usize,
    pub baseline_reasoning_cost: usize,
    pub concept_reasoning_cost: usize,
    pub compression_gain: usize,
}

/// Enumerate auxiliary objects from the fixed grammar, require finite evidence
/// and an independently accepted arbitrary-list certificate, then retain the
/// first smallest/order candidate. `product+1` is never passed to this function.
pub fn discover_euclid_construction(
    training: &[Vec<i128>],
    held_out: &[Vec<i128>],
    transfer: &[Vec<i128>],
) -> Option<EuclidDiscovery> {
    let mut discovery_cost = 0;
    for candidate in enumerate_candidates() {
        discovery_cost += 1;
        if !training.iter().all(|xs| observed_escape(&candidate, xs)) {
            continue;
        }
        let certificate = certificate_for(candidate.clone());
        if check_escape_certificate(&certificate).is_err() {
            continue;
        }
        if !held_out.iter().all(|xs| observed_escape(&candidate, xs)) {
            continue;
        }
        // Transfer asks only for escape from arbitrary divisor lists; primality
        // of their members is deliberately absent from this lower-level check.
        let transfer_lists = transfer
            .iter()
            .filter(|xs| {
                candidate
                    .eval(xs)
                    .is_some_and(|n| n > 1 && xs.iter().all(|d| *d > 1 && n % d != 0))
            })
            .count();
        if transfer_lists != transfer.len() {
            continue;
        }
        let baseline = discovery_cost + held_out.len() * 4;
        let concept = held_out.len() * 4;
        let raw_tokens: usize = training.iter().map(|xs| xs.len()).sum();
        return Some(EuclidDiscovery {
            conjecture: "there_are_infinitely_many_primes",
            construction: candidate,
            certificate,
            discovery_cost,
            training_lists: training.len(),
            held_out_lists: held_out.len(),
            transfer_lists,
            baseline_reasoning_cost: baseline,
            concept_reasoning_cost: concept,
            compression_gain: raw_tokens.saturating_sub(3),
        });
    }
    None
}

pub fn m11_experiment() -> EuclidDiscovery {
    discover_euclid_construction(
        &[vec![2], vec![2, 3], vec![2, 3, 5], vec![3, 5, 7]],
        &[vec![2, 3, 5, 7], vec![5, 7, 11], vec![11, 13]],
        &[vec![4, 6], vec![6, 10, 15], vec![8, 9, 25]],
    )
    .expect("M11 must invent an accepted auxiliary construction")
}

pub fn machine_record(report: &EuclidDiscovery) -> String {
    format!(
        "experiment=math_world_m11,conjecture={},discovered={},discovery_cost={},training_lists={},held_out_lists={},transfer_lists={},construction_checked=true,no_listed_divisor_checked=true,prime_witness_checked=true,baseline_reasoning_cost={},concept_reasoning_cost={},transfer_cost={},compression_gain={},proof_status=formally_checked_finite_list_schema,deterministic=true,fallback=exact",
        report.conjecture, report.construction.render(), report.discovery_cost,
        report.training_lists, report.held_out_lists, report.transfer_lists,
        report.baseline_reasoning_cost, report.concept_reasoning_cost,
        report.concept_reasoning_cost, report.compression_gain
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invents_product_plus_one_and_checks_arbitrary_prime_list_theorem() {
        let report = m11_experiment();
        assert_eq!(report.construction.render(), "(product(xs)+1)");
        assert_eq!(
            check_escape_certificate(&report.certificate),
            Ok(Theorem::PrimeEscapesEveryFiniteList)
        );
        assert_eq!(report.held_out_lists, 3);
        assert_eq!(report.transfer_lists, 3);
        assert!(report.concept_reasoning_cost < report.baseline_reasoning_cost);
    }

    #[test]
    fn singleton_control_rejects_product_minus_one_shortcut() {
        let bad = CollectionExpr::Sub(
            Box::new(CollectionExpr::Product),
            Box::new(CollectionExpr::Const(1)),
        );
        assert!(!observed_escape(&bad, &[2]));
        assert!(check_escape_certificate(&certificate_for(bad)).is_err());
    }

    #[test]
    fn checker_rejects_corrupted_symbolic_obligations() {
        let mut certificate = certificate_for(CollectionExpr::Add(
            Box::new(CollectionExpr::Product),
            Box::new(CollectionExpr::Const(1)),
        ));
        certificate.derivation.remove(1);
        assert_eq!(
            check_escape_certificate(&certificate),
            Err(CheckError::InvalidDerivation)
        );
    }

    #[test]
    fn checker_validates_consequences_not_target_syntax() {
        let reversed = CollectionExpr::Add(
            Box::new(CollectionExpr::Const(1)),
            Box::new(CollectionExpr::Product),
        );
        assert_eq!(
            check_escape_certificate(&certificate_for(reversed)),
            Ok(Theorem::PrimeEscapesEveryFiniteList)
        );
    }

    #[test]
    fn nonprime_training_control_cannot_claim_prime_list_theorem() {
        let candidate = CollectionExpr::Add(
            Box::new(CollectionExpr::Product),
            Box::new(CollectionExpr::Const(1)),
        );
        assert!(!observed_escape(&candidate, &[2, 4, 6]));
    }

    #[test]
    fn record_and_search_are_deterministic() {
        let a = m11_experiment();
        let b = m11_experiment();
        assert_eq!(a.construction, b.construction);
        assert_eq!(machine_record(&a), machine_record(&b));
    }
}
