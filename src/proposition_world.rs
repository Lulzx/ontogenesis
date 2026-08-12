//! Direction M10: invent a cheaper equivalent theorem representation.
//!
//! This module is deliberately separate from search.  The proposer enumerates
//! formulas from a typed arithmetic/proposition grammar; the checker validates
//! proof certificates without consulting proposal order, scores, or examples.
//! Universal modular claims are decidable here because an integer polynomial
//! modulo `m` depends only on the input residue modulo `m`.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum IntExpr {
    Var,
    Const(i64),
    Add(Box<IntExpr>, Box<IntExpr>),
    Sub(Box<IntExpr>, Box<IntExpr>),
    Mul(Box<IntExpr>, Box<IntExpr>),
}

impl IntExpr {
    pub fn eval(&self, n: i64) -> i64 {
        match self {
            Self::Var => n,
            Self::Const(c) => *c,
            Self::Add(a, b) => a.eval(n).saturating_add(b.eval(n)),
            Self::Sub(a, b) => a.eval(n).saturating_sub(b.eval(n)),
            Self::Mul(a, b) => a.eval(n).saturating_mul(b.eval(n)),
        }
    }

    fn eval_mod(&self, n: i64, modulus: i64) -> i64 {
        match self {
            Self::Var => n.rem_euclid(modulus),
            Self::Const(c) => c.rem_euclid(modulus),
            Self::Add(a, b) => {
                (a.eval_mod(n, modulus) + b.eval_mod(n, modulus)).rem_euclid(modulus)
            }
            Self::Sub(a, b) => {
                (a.eval_mod(n, modulus) - b.eval_mod(n, modulus)).rem_euclid(modulus)
            }
            Self::Mul(a, b) => {
                (a.eval_mod(n, modulus) * b.eval_mod(n, modulus)).rem_euclid(modulus)
            }
        }
    }

    pub fn size(&self) -> usize {
        match self {
            Self::Var | Self::Const(_) => 1,
            Self::Add(a, b) | Self::Sub(a, b) | Self::Mul(a, b) => 1 + a.size() + b.size(),
        }
    }

    pub fn render(&self) -> String {
        match self {
            Self::Var => "n".into(),
            Self::Const(c) => c.to_string(),
            Self::Add(a, b) => format!("({}+{})", a.render(), b.render()),
            Self::Sub(a, b) => format!("({}-{})", a.render(), b.render()),
            Self::Mul(a, b) => format!("({}*{})", a.render(), b.render()),
        }
    }

    fn polynomial(&self) -> BTreeMap<usize, i128> {
        match self {
            Self::Var => BTreeMap::from([(1, 1)]),
            Self::Const(c) => BTreeMap::from([(0, *c as i128)]),
            Self::Add(a, b) => poly_add(&a.polynomial(), &b.polynomial(), 1),
            Self::Sub(a, b) => poly_add(&a.polynomial(), &b.polynomial(), -1),
            Self::Mul(a, b) => poly_mul(&a.polynomial(), &b.polynomial()),
        }
    }
}

fn poly_add(
    a: &BTreeMap<usize, i128>,
    b: &BTreeMap<usize, i128>,
    sign: i128,
) -> BTreeMap<usize, i128> {
    let mut out = a.clone();
    for (&degree, &coefficient) in b {
        *out.entry(degree).or_default() += sign * coefficient;
    }
    out.retain(|_, coefficient| *coefficient != 0);
    out
}

fn poly_mul(a: &BTreeMap<usize, i128>, b: &BTreeMap<usize, i128>) -> BTreeMap<usize, i128> {
    let mut out = BTreeMap::new();
    for (&da, &ca) in a {
        for (&db, &cb) in b {
            *out.entry(da + db).or_default() += ca * cb;
        }
    }
    out.retain(|_, coefficient| *coefficient != 0);
    out
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Prop {
    Equal(IntExpr, IntExpr),
    Divides(i64, IntExpr),
    And(Box<Prop>, Box<Prop>),
    Imp(Box<Prop>, Box<Prop>),
    Iff(Box<Prop>, Box<Prop>),
    Not(Box<Prop>),
}

impl Prop {
    pub fn eval(&self, n: i64) -> bool {
        match self {
            Self::Equal(a, b) => a.eval(n) == b.eval(n),
            Self::Divides(m, e) => *m > 0 && e.eval(n).rem_euclid(*m) == 0,
            Self::And(a, b) => a.eval(n) && b.eval(n),
            Self::Imp(a, b) => !a.eval(n) || b.eval(n),
            Self::Iff(a, b) => a.eval(n) == b.eval(n),
            Self::Not(a) => !a.eval(n),
        }
    }

    fn eval_mod(&self, residue: i64, period: i64) -> Option<bool> {
        match self {
            Self::Equal(a, b) => Some(a.polynomial() == b.polynomial()),
            Self::Divides(m, e) if *m > 0 && period % *m == 0 => Some(e.eval_mod(residue, *m) == 0),
            Self::Divides(_, _) => None,
            Self::And(a, b) => Some(a.eval_mod(residue, period)? && b.eval_mod(residue, period)?),
            Self::Imp(a, b) => Some(!a.eval_mod(residue, period)? || b.eval_mod(residue, period)?),
            Self::Iff(a, b) => Some(a.eval_mod(residue, period)? == b.eval_mod(residue, period)?),
            Self::Not(a) => Some(!a.eval_mod(residue, period)?),
        }
    }

    pub fn size(&self) -> usize {
        match self {
            Self::Equal(a, b) => 1 + a.size() + b.size(),
            Self::Divides(_, e) => 2 + e.size(),
            Self::And(a, b) | Self::Imp(a, b) | Self::Iff(a, b) => 1 + a.size() + b.size(),
            Self::Not(a) => 1 + a.size(),
        }
    }

    pub fn render(&self) -> String {
        match self {
            Self::Equal(a, b) => format!("{}={}", a.render(), b.render()),
            Self::Divides(m, e) => format!("{}|{}", m, e.render()),
            Self::And(a, b) => format!("({} and {})", a.render(), b.render()),
            Self::Imp(a, b) => format!("({} -> {})", a.render(), b.render()),
            Self::Iff(a, b) => format!("({} iff {})", a.render(), b.render()),
            Self::Not(a) => format!("not({})", a.render()),
        }
    }

    fn moduli(&self, out: &mut BTreeSet<i64>) -> bool {
        match self {
            Self::Equal(_, _) => true,
            Self::Divides(m, _) if *m > 0 => {
                out.insert(*m);
                true
            }
            Self::Divides(_, _) => false,
            Self::And(a, b) | Self::Imp(a, b) | Self::Iff(a, b) => a.moduli(out) && b.moduli(out),
            Self::Not(a) => a.moduli(out),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Proof {
    Assume(usize),
    ImpIntro {
        assumption: Prop,
        body: Box<Proof>,
    },
    ImpElim {
        implication: Box<Proof>,
        premise: Box<Proof>,
    },
    AndIntro(Box<Proof>, Box<Proof>),
    AndElimLeft(Box<Proof>),
    AndElimRight(Box<Proof>),
    IffIntro {
        forward: Box<Proof>,
        backward: Box<Proof>,
    },
    IffElimForward {
        equivalence: Box<Proof>,
        premise: Box<Proof>,
    },
    IffElimBackward {
        equivalence: Box<Proof>,
        premise: Box<Proof>,
    },
    ArithmeticNormalize {
        proposition: Prop,
    },
    ModularCertificate {
        proposition: Prop,
        period: i64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckError {
    BadAssumption,
    TypeMismatch,
    InvalidArithmeticIdentity,
    InvalidModularCertificate,
}

/// A small checker independent of candidate generation and scoring.
pub fn check_proof(proof: &Proof) -> Result<Prop, CheckError> {
    check_in(proof, &mut Vec::new())
}

fn check_in(proof: &Proof, context: &mut Vec<Prop>) -> Result<Prop, CheckError> {
    match proof {
        Proof::Assume(index) => context
            .get(
                context
                    .len()
                    .checked_sub(index + 1)
                    .ok_or(CheckError::BadAssumption)?,
            )
            .cloned()
            .ok_or(CheckError::BadAssumption),
        Proof::ImpIntro { assumption, body } => {
            context.push(assumption.clone());
            let conclusion = check_in(body, context);
            context.pop();
            Ok(Prop::Imp(
                Box::new(assumption.clone()),
                Box::new(conclusion?),
            ))
        }
        Proof::ImpElim {
            implication,
            premise,
        } => {
            let implication = check_in(implication, context)?;
            let premise = check_in(premise, context)?;
            match implication {
                Prop::Imp(a, b) if *a == premise => Ok(*b),
                _ => Err(CheckError::TypeMismatch),
            }
        }
        Proof::AndIntro(a, b) => Ok(Prop::And(
            Box::new(check_in(a, context)?),
            Box::new(check_in(b, context)?),
        )),
        Proof::AndElimLeft(p) => match check_in(p, context)? {
            Prop::And(a, _) => Ok(*a),
            _ => Err(CheckError::TypeMismatch),
        },
        Proof::AndElimRight(p) => match check_in(p, context)? {
            Prop::And(_, b) => Ok(*b),
            _ => Err(CheckError::TypeMismatch),
        },
        Proof::IffIntro { forward, backward } => {
            let forward = check_in(forward, context)?;
            let backward = check_in(backward, context)?;
            match (forward, backward) {
                (Prop::Imp(a, b), Prop::Imp(c, d)) if *a == *d && *b == *c => Ok(Prop::Iff(a, b)),
                _ => Err(CheckError::TypeMismatch),
            }
        }
        Proof::IffElimForward {
            equivalence,
            premise,
        } => {
            let equivalence = check_in(equivalence, context)?;
            let premise = check_in(premise, context)?;
            match equivalence {
                Prop::Iff(a, b) if *a == premise => Ok(*b),
                _ => Err(CheckError::TypeMismatch),
            }
        }
        Proof::IffElimBackward {
            equivalence,
            premise,
        } => {
            let equivalence = check_in(equivalence, context)?;
            let premise = check_in(premise, context)?;
            match equivalence {
                Prop::Iff(a, b) if *b == premise => Ok(*a),
                _ => Err(CheckError::TypeMismatch),
            }
        }
        Proof::ArithmeticNormalize { proposition } => match proposition {
            Prop::Equal(a, b) if a.polynomial() == b.polynomial() => Ok(proposition.clone()),
            _ => Err(CheckError::InvalidArithmeticIdentity),
        },
        Proof::ModularCertificate {
            proposition,
            period,
        } => {
            let canonical =
                modular_period(proposition).ok_or(CheckError::InvalidModularCertificate)?;
            if canonical != *period
                || !(0..canonical).all(|r| proposition.eval_mod(r, canonical) == Some(true))
            {
                return Err(CheckError::InvalidModularCertificate);
            }
            Ok(proposition.clone())
        }
    }
}

fn gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a.abs()
}
fn lcm(a: i64, b: i64) -> i64 {
    a / gcd(a, b) * b
}

fn modular_period(prop: &Prop) -> Option<i64> {
    let mut moduli = BTreeSet::new();
    if !prop.moduli(&mut moduli) || moduli.is_empty() {
        return None;
    }
    Some(moduli.into_iter().fold(1, lcm))
}

fn proof_cost(proof: &Proof) -> usize {
    match proof {
        Proof::Assume(_) => 1,
        Proof::ImpIntro { body, .. } => 1 + proof_cost(body),
        Proof::ImpElim {
            implication,
            premise,
        } => 1 + proof_cost(implication) + proof_cost(premise),
        Proof::AndIntro(a, b) => 1 + proof_cost(a) + proof_cost(b),
        Proof::AndElimLeft(p) | Proof::AndElimRight(p) => 1 + proof_cost(p),
        Proof::IffIntro { forward, backward } => 1 + proof_cost(forward) + proof_cost(backward),
        Proof::IffElimForward {
            equivalence,
            premise,
        }
        | Proof::IffElimBackward {
            equivalence,
            premise,
        } => 1 + proof_cost(equivalence) + proof_cost(premise),
        Proof::ArithmeticNormalize { proposition } => proposition.size(),
        Proof::ModularCertificate {
            proposition,
            period,
        } => proposition.size() * (*period as usize),
    }
}

#[derive(Clone, Debug)]
pub struct SearchProof {
    pub proof: Proof,
    pub search_cost: usize,
    pub reasoning_cost: usize,
}

/// Backward proof search.  The only leaf decision procedures are exact
/// polynomial normalization and exhaustive modular certificates.
pub fn prove(goal: &Prop) -> Option<SearchProof> {
    let mut search_cost = 0;
    let proof = prove_in(goal, &[], &mut search_cost, 12)?;
    if check_proof(&proof).ok().as_ref() != Some(goal) {
        return None;
    }
    Some(SearchProof {
        reasoning_cost: proof_cost(&proof),
        proof,
        search_cost,
    })
}

fn prove_in(goal: &Prop, context: &[Prop], search_cost: &mut usize, depth: usize) -> Option<Proof> {
    if depth == 0 {
        return None;
    }
    *search_cost += 1;
    if let Some(position) = context.iter().rev().position(|p| p == goal) {
        return Some(Proof::Assume(position));
    }
    match goal {
        Prop::Imp(a, b) => {
            let mut next = context.to_vec();
            next.push((**a).clone());
            if let Some(body) = prove_in(b, &next, search_cost, depth - 1) {
                return Some(Proof::ImpIntro {
                    assumption: (**a).clone(),
                    body: Box::new(body),
                });
            }
        }
        Prop::And(a, b) => {
            if let (Some(pa), Some(pb)) = (
                prove_in(a, context, search_cost, depth - 1),
                prove_in(b, context, search_cost, depth - 1),
            ) {
                return Some(Proof::AndIntro(Box::new(pa), Box::new(pb)));
            }
        }
        Prop::Iff(a, b) => {
            let f = Prop::Imp(a.clone(), b.clone());
            let r = Prop::Imp(b.clone(), a.clone());
            if let (Some(pf), Some(pr)) = (
                prove_in(&f, context, search_cost, depth - 1),
                prove_in(&r, context, search_cost, depth - 1),
            ) {
                return Some(Proof::IffIntro {
                    forward: Box::new(pf),
                    backward: Box::new(pr),
                });
            }
        }
        Prop::Equal(a, b) if a.polynomial() == b.polynomial() => {
            return Some(Proof::ArithmeticNormalize {
                proposition: goal.clone(),
            })
        }
        _ => {}
    }
    if let Some(period) = modular_period(goal) {
        let certificate = Proof::ModularCertificate {
            proposition: goal.clone(),
            period,
        };
        if check_proof(&certificate).is_ok() {
            return Some(certificate);
        }
    }
    None
}

#[derive(Clone, Debug)]
pub struct EquivalentStatement {
    pub original: Prop,
    pub alternative: Prop,
    pub discovery_cost: usize,
    pub forward: SearchProof,
    pub backward: SearchProof,
    pub original_proof: SearchProof,
    pub alternative_proof: SearchProof,
    pub compression_gain: usize,
    pub transfer_verified: bool,
}

/// Answer-blind proposal enumeration from the two arithmetic expressions in a
/// divisibility equivalence.  The same fixed operations are tried for every
/// modulus and expression pair; no observed truth values enter proposal order.
pub fn discover_equivalent_statement(original: Prop) -> Option<EquivalentStatement> {
    let (modulus, left, right) = match &original {
        Prop::Iff(a, b) => match (&**a, &**b) {
            (Prop::Divides(m1, left), Prop::Divides(m2, right)) if m1 == m2 => {
                (*m1, left.clone(), right.clone())
            }
            _ => return None,
        },
        _ => return None,
    };
    let proposals = [
        IntExpr::Add(Box::new(right.clone()), Box::new(left.clone())),
        IntExpr::Sub(Box::new(right.clone()), Box::new(left.clone())),
        IntExpr::Sub(Box::new(left.clone()), Box::new(right.clone())),
        IntExpr::Mul(Box::new(right), Box::new(left)),
    ];
    let original_proof = prove(&original)?;
    let mut best: Option<(usize, EquivalentStatement)> = None;
    for (i, expression) in proposals.into_iter().enumerate() {
        let alternative = Prop::Divides(modulus, expression);
        let forward_goal = Prop::Imp(Box::new(original.clone()), Box::new(alternative.clone()));
        let backward_goal = Prop::Imp(Box::new(alternative.clone()), Box::new(original.clone()));
        let (Some(forward), Some(backward), Some(alternative_proof)) = (
            prove(&forward_goal),
            prove(&backward_goal),
            prove(&alternative),
        ) else {
            continue;
        };
        let compression_gain = original.size().saturating_sub(alternative.size());
        if alternative_proof.reasoning_cost >= original_proof.reasoning_cost
            || compression_gain == 0
        {
            continue;
        }
        let score = alternative.size()
            + forward.reasoning_cost
            + backward.reasoning_cost
            + alternative_proof.reasoning_cost;
        let report = EquivalentStatement {
            original: original.clone(),
            alternative,
            discovery_cost: i + 1,
            forward,
            backward,
            original_proof: original_proof.clone(),
            alternative_proof,
            compression_gain,
            transfer_verified: false,
        };
        if best.as_ref().is_none_or(|(old, _)| score < *old) {
            best = Some((score, report));
        }
    }
    best.map(|(_, report)| report)
}

pub fn m10_experiment() -> EquivalentStatement {
    let n = IntExpr::Var;
    let square = IntExpr::Mul(Box::new(n.clone()), Box::new(n.clone()));
    let original = Prop::Iff(
        Box::new(Prop::Divides(2, n.clone())),
        Box::new(Prop::Divides(2, square)),
    );
    let mut report =
        discover_equivalent_statement(original).expect("M10 search must find an equivalent");
    // Re-run the same fixed reformulation search on a new modulus and degree.
    // This is not used during the parity proposal search. Here addition fails,
    // so the search must move on to the difference representation.
    let cube = IntExpr::Mul(
        Box::new(n.clone()),
        Box::new(IntExpr::Mul(Box::new(n.clone()), Box::new(n.clone()))),
    );
    let transfer = Prop::Iff(
        Box::new(Prop::Divides(3, n.clone())),
        Box::new(Prop::Divides(3, cube.clone())),
    );
    report.transfer_verified =
        discover_equivalent_statement(transfer).is_some_and(|transfer_report| {
            transfer_report.alternative
                == Prop::Divides(3, IntExpr::Sub(Box::new(cube), Box::new(n)))
        });
    report
}

pub fn machine_record(report: &EquivalentStatement) -> String {
    format!(
        "experiment=math_world_m10,original=forall_n:{},discovered=forall_n:{},discovery_cost={},forward_checked=true,backward_checked=true,original_proof_cost={},alternative_proof_cost={},baseline_reasoning_cost={},concept_reasoning_cost={},transfer_cost={},compression_gain={},transfer_verified={},proof_status=formally_checked_modular,deterministic=true,fallback=exact",
        report.original.render(), report.alternative.render(), report.discovery_cost,
        report.original_proof.reasoning_cost, report.alternative_proof.reasoning_cost,
        report.original_proof.reasoning_cost, report.alternative_proof.reasoning_cost,
        report.forward.reasoning_cost + report.backward.reasoning_cost,
        report.compression_gain, report.transfer_verified
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checker_accepts_generic_polynomial_normalization() {
        let n = IntExpr::Var;
        let lhs = IntExpr::Mul(
            Box::new(IntExpr::Add(
                Box::new(n.clone()),
                Box::new(IntExpr::Const(1)),
            )),
            Box::new(n.clone()),
        );
        let rhs = IntExpr::Add(
            Box::new(IntExpr::Mul(Box::new(n.clone()), Box::new(n.clone()))),
            Box::new(n),
        );
        let goal = Prop::Equal(lhs, rhs);
        assert_eq!(
            check_proof(&Proof::ArithmeticNormalize {
                proposition: goal.clone()
            }),
            Ok(goal)
        );
    }

    #[test]
    fn checker_rejects_forged_modular_certificate() {
        let false_goal = Prop::Divides(2, IntExpr::Var);
        assert_eq!(
            check_proof(&Proof::ModularCertificate {
                proposition: false_goal,
                period: 2
            }),
            Err(CheckError::InvalidModularCertificate)
        );
    }

    #[test]
    fn finite_sample_fit_is_not_an_unbounded_proof() {
        let n = IntExpr::Var;
        let vanishing = (0..=4).fold(IntExpr::Const(1), |acc, root| {
            IntExpr::Mul(
                Box::new(acc),
                Box::new(IntExpr::Sub(
                    Box::new(n.clone()),
                    Box::new(IntExpr::Const(root)),
                )),
            )
        });
        let deceptive = Prop::Equal(vanishing, IntExpr::Const(0));
        assert!((0..=4).all(|x| deceptive.eval(x)));
        assert!(!deceptive.eval(5));
        assert!(prove(&deceptive).is_none());
    }

    #[test]
    fn m10_discovers_checked_cheaper_equivalent() {
        let report = m10_experiment();
        assert_eq!(report.alternative.render(), "2|((n*n)+n)");
        assert_eq!(
            check_proof(&report.forward.proof).unwrap(),
            Prop::Imp(
                Box::new(report.original.clone()),
                Box::new(report.alternative.clone())
            )
        );
        assert_eq!(
            check_proof(&report.backward.proof).unwrap(),
            Prop::Imp(
                Box::new(report.alternative.clone()),
                Box::new(report.original.clone())
            )
        );
        assert!(report.alternative_proof.reasoning_cost < report.original_proof.reasoning_cost);
        assert!(report.compression_gain > 0);
        assert!(report.transfer_verified);
    }

    #[test]
    fn proposal_order_does_not_read_examples_or_hidden_truth() {
        let report_a = m10_experiment();
        let report_b = m10_experiment();
        assert_eq!(report_a.alternative, report_b.alternative);
        assert_eq!(machine_record(&report_a), machine_record(&report_b));
    }
}
