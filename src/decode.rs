//! Encoding recognizers: turn λ-normal-forms into native values and back.
//!
//! LamBench's 120 tasks are ~40 semantic functions crossed with 12 data
//! encodings. Test inputs and expected outputs are normal forms whose shape
//! is fully determined by the encoding grammar — a Church numeral is
//! syntactically λf.λx.f(...f(x)), a Scott list is a nested λc.λn.c(h, t)
//! chain. Decoding is pattern recognition; encoding is the inverse printer.
//!
//! Native values decoded here feed the semantic-level DSL search; the
//! compiler re-emits results in the task's own encoding.

use crate::term::{app, lam, var, Term};
use std::rc::Rc;

/// A native value recovered from (or destined for) a λ-encoding.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum V {
    Nat(u64),
    Bool(bool),
    List(Vec<V>),
    /// Binary tree with values at leaves: Node(l, r) | Leaf(v).
    Node(Box<V>, Box<V>),
    /// An opaque atom: a free constant (abstract test binder) that flows
    /// through the semantic function untouched.
    Atom(u32),
    /// Application of one opaque value to another (e.g. F(A) in map tasks).
    App(Box<V>, Box<V>),
    /// Church/Scott ADT constructor application: (tag, arity, fields).
    Ctr(u32, Vec<V>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Enc {
    ChurchNat,
    ChurchBin, // list of bits, LSB first, as Church list of Church bools? (per task spec)
    ScottNat,
    ScottBin,
    ChurchList,
    ScottList,
    ChurchTree,
    ScottTree,
    NTuple,
    ChurchBool,
}

// ── Recognizers ─────────────────────────────────────────────────────
//
// All recognizers take a *closed-up-to-Free* normal form. de Bruijn indices
// are used relative to the binders inside the encoding itself.

/// λf.λx.f(f(...f(x)...))  →  Nat(n)
pub fn church_nat(t: &Term) -> Option<u64> {
    // Expect Lam(Lam(body)) with body = chain of Var(1) applied ending Var(0).
    let Term::Lam(b1) = t else { return None };
    let Term::Lam(b2) = b1.as_ref() else {
        return None;
    };
    let mut n = 0u64;
    let mut cur = b2.as_ref();
    loop {
        match cur {
            Term::Var(0) => return Some(n),
            Term::App(f, a) => {
                if !matches!(f.as_ref(), Term::Var(1)) {
                    return None;
                }
                n += 1;
                cur = a.as_ref();
            }
            _ => return None,
        }
    }
}

pub fn church_nat_term(n: u64) -> Rc<Term> {
    let mut body = var(0);
    for _ in 0..n {
        body = app(var(1), body);
    }
    lam(lam(body))
}

/// Scott nat: Z = λs.λz.z, S(n) = λs.λz.s(n)
pub fn scott_nat(t: &Term) -> Option<u64> {
    let mut n = 0u64;
    let mut cur = t;
    loop {
        let Term::Lam(b1) = cur else { return None };
        let Term::Lam(b2) = b1.as_ref() else {
            return None;
        };
        match b2.as_ref() {
            Term::Var(0) => return Some(n),
            Term::App(f, a) => {
                if !matches!(f.as_ref(), Term::Var(1)) {
                    return None;
                }
                n += 1;
                cur = a.as_ref();
            }
            _ => return None,
        }
    }
}

pub fn scott_nat_term(n: u64) -> Rc<Term> {
    let mut t = lam(lam(var(0)));
    for _ in 0..n {
        t = lam(lam(app(var(1), t)));
    }
    t
}

/// Church list: [a,b,c] = λc.λn.c(a, c(b, c(c', n)))
/// Elements are decoded recursively; Free constants become Atoms.
pub fn church_list(t: &Term) -> Option<Vec<V>> {
    let Term::Lam(b1) = t else { return None };
    let Term::Lam(b2) = b1.as_ref() else {
        return None;
    };
    let mut items = Vec::new();
    let mut cur = b2.as_ref();
    loop {
        match cur {
            Term::Var(0) => return Some(items),
            Term::App(fh, tail) => {
                let Term::App(c, head) = fh.as_ref() else {
                    return None;
                };
                if !matches!(c.as_ref(), Term::Var(1)) {
                    return None;
                }
                // Head element lives under 2 binders: unshift before decode.
                items.push(decode_value(&unshift(head, 2)?)?);
                cur = tail.as_ref();
            }
            _ => return None,
        }
    }
}

/// Scott list: Nil = λc.λn.n, Cons(h,t) = λc.λn.c(h, t)
pub fn scott_list(t: &Rc<Term>) -> Option<Vec<V>> {
    let mut items = Vec::new();
    let mut cur: Rc<Term> = t.clone();
    loop {
        let Term::Lam(b1) = cur.as_ref() else {
            return None;
        };
        let Term::Lam(b2) = b1.as_ref() else {
            return None;
        };
        let step: Option<(V, Rc<Term>)> = match b2.as_ref() {
            Term::Var(0) => None,
            Term::App(fh, tail) => {
                let Term::App(c, head) = fh.as_ref() else {
                    return None;
                };
                if !matches!(c.as_ref(), Term::Var(1)) {
                    return None;
                }
                Some((decode_value(&unshift(head, 2)?)?, unshift(tail, 2)?))
            }
            _ => return None,
        };
        match step {
            None => return Some(items),
            Some((v, tl)) => {
                items.push(v);
                cur = tl;
            }
        }
    }
}

/// N-tuple: λt.t(A, B, C) — Lam over an application spine headed by the
/// binder. The empty tuple is λt.t.
pub fn ntuple(t: &Rc<Term>) -> Option<Vec<V>> {
    let Term::Lam(b) = t.as_ref() else { return None };
    let mut args_rev: Vec<&Rc<Term>> = Vec::new();
    let mut cur = b;
    while let Term::App(f, a) = cur.as_ref() {
        args_rev.push(a);
        cur = f;
    }
    if !matches!(cur.as_ref(), Term::Var(0)) {
        return None;
    }
    let mut out = Vec::new();
    for a in args_rev.iter().rev() {
        out.push(decode_value(&unshift(a, 1)?)?);
    }
    Some(out)
}

/// Generic value decoder: tries atoms, atom-applications, nats, lists.
/// Free constants decode to Atom. Church true decodes to Bool(true);
/// note Church false ≡ nat 0, so predicates are classified at task level.
pub fn decode_value(t: &Rc<Term>) -> Option<V> {
    if let Term::Free(i) = t.as_ref() {
        return Some(V::Atom(*i));
    }
    // Application spine of opaque values: F(A), M(a, x), ...
    if let Term::App(f, a) = t.as_ref() {
        return Some(V::App(
            Box::new(decode_value(f)?),
            Box::new(decode_value(a)?),
        ));
    }
    if let Some(n) = church_nat(t) {
        return Some(V::Nat(n));
    }
    if let Some(n) = scott_nat(t) {
        // Note: Z = λs.λz.z is also Church 0. Church wins above; the
        // semantic layer treats Nat uniformly, so the ambiguity at 0 is
        // harmless — encoding choice is resolved per-task, not per-value.
        return Some(V::Nat(n));
    }
    if let Some(xs) = church_list(t) {
        return Some(V::List(xs));
    }
    if let Some(xs) = scott_list(t) {
        return Some(V::List(xs));
    }
    // Church true = λa.λb.a (false ≡ nat 0, already taken above).
    if let Term::Lam(b1) = t.as_ref() {
        if let Term::Lam(b2) = b1.as_ref() {
            if matches!(b2.as_ref(), Term::Var(1)) {
                return Some(V::Bool(true));
            }
        }
    }
    None
}

/// Decrease every free de Bruijn index in `t` by `by` (indices pointing past
/// the local binders). Returns None if any index would go negative — i.e.
/// the subterm actually uses the encoding's own binders and is not liftable.
pub fn unshift(t: &Rc<Term>, by: u32) -> Option<Rc<Term>> {
    fn go(t: &Rc<Term>, by: u32, depth: u32) -> Option<Rc<Term>> {
        match t.as_ref() {
            Term::Var(i) => {
                if *i < depth {
                    Some(t.clone())
                } else if *i >= depth + by {
                    Some(var(*i - by))
                } else {
                    None
                }
            }
            Term::Free(_) => Some(t.clone()),
            Term::Lam(b) => Some(lam(go(b, by, depth + 1)?)),
            Term::App(f, a) => Some(app(go(f, by, depth)?, go(a, by, depth)?)),
        }
    }
    go(t, by, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{parse_expr, to_term};

    fn t(src: &str) -> Rc<Term> {
        to_term(&parse_expr(src).unwrap()).unwrap()
    }

    #[test]
    fn church_nats() {
        assert_eq!(church_nat(&t("λf.λx.x")), Some(0));
        assert_eq!(church_nat(&t("λf.λx.f(f(f(x)))")), Some(3));
        assert_eq!(church_nat(&t("λf.λx.f(λy.y)")), None);
        assert_eq!(church_nat(&church_nat_term(7)), Some(7));
    }

    #[test]
    fn scott_nats() {
        assert_eq!(scott_nat(&t("λs.λz.z")), Some(0));
        assert_eq!(scott_nat(&t("λs.λz.s(λs.λz.s(λs.λz.z))")), Some(2));
        assert_eq!(scott_nat(&scott_nat_term(5)), Some(5));
    }

    #[test]
    fn church_lists() {
        let l = t("λc.λn.c(λf.λx.f(x), c(λf.λx.f(f(x)), n))");
        assert_eq!(
            church_list(&l),
            Some(vec![V::Nat(1), V::Nat(2)])
        );
    }

    #[test]
    fn scott_lists() {
        let l = t("λc.λn.c(λf.λx.f(x), λc.λn.n)");
        assert_eq!(scott_list(&l), Some(vec![V::Nat(1)]));
    }
}
