//! Canonical *observation* of a closed normal form — representation only.
//!
//! The user-facing constraint that drives this module: do NOT change how
//! evaluation executes. `apply`, β-reduction, and the searchable λ-language
//! are untouched. The underlying value stays `Rc<Val>`. `canonicalize` merely
//! *observes* a normal form and collapses the special shape `λf.λx.f^n(x)`
//! (a Church numeral) to a compact `ChurchNumeral(n)`, so that hashing,
//! equality, pool storage, and target matching stop depending on the numeral's
//! syntactic expansion (`f(f(f(...f(x))))` — 2n+2 nodes).
//!
//! Everything that is not exactly a Church numeral falls back to structural
//! hashing, exactly as the engine already does. No arithmetic is performed:
//! `mul(ChurchNumeral(a), ChurchNumeral(b)) => ChurchNumeral(a*b)` is
//! deliberately NOT implemented here. The product value is computed by ordinary
//! Church β-reduction (it is a real `Val`), and only then recognized.

use crate::nbe::*;
use crate::term::Term;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

/// A canonical semantic key for a closed normal form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalValue {
    /// The exact shape `λf.λx.f^n(x)` — a Church numeral, compact.
    ChurchNumeral(u64),
    /// Any other normal form, keyed by its structural hash.
    StructuralHash(u64),
}

impl CanonicalValue {
    /// A single u64 usable in a dedup/target key vector.
    pub fn key(&self) -> u64 {
        match self {
            // Tag numerals into a distinct band so they can never collide with a
            // structural hash; keep `n` recoverable for diagnostics.
            CanonicalValue::ChurchNumeral(n) => (1u64 << 62) | *n,
            CanonicalValue::StructuralHash(h) => *h & ((1u64 << 62) - 1),
        }
    }
}

thread_local! {
    static METER_ON: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static CANON_NODES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static CANON_ABORTS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static MAX_TRANSIENT: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

pub fn meter_on(on: bool) {
    METER_ON.with(|m| m.set(on));
}
pub fn meter_reset() {
    CANON_NODES.with(|m| m.set(0));
    CANON_ABORTS.with(|m| m.set(0));
    MAX_TRANSIENT.with(|m| m.set(0));
}
pub fn canon_nodes() -> u64 {
    CANON_NODES.with(|m| m.get())
}
pub fn canon_aborts() -> u64 {
    CANON_ABORTS.with(|m| m.get())
}
pub fn max_transient() -> u64 {
    MAX_TRANSIENT.with(|m| m.get())
}

fn bump_nodes(k: u64) {
    if METER_ON.with(|m| m.get()) {
        CANON_NODES.with(|m| m.set(m.get().saturating_add(k)));
        MAX_TRANSIENT.with(|m| {
            let cur = m.get();
            if k > cur {
                m.set(k);
            }
        });
    }
}

/// Observe `v` (a closed normal form, depth 0) and return its canonical key.
///
/// Recognition first quotes `v` to its β-normal term (`quote` is the "semantic
/// evaluation" that establishes what the value *is* — a composition like
/// `mul(3)(4)` is a closure `λc.b(a(c))` with 3,4 captured in its environment,
/// not yet syntactically the numeral `λf.λx.f^12(x)`), then checks the quoted
/// term for the exact Church-numeral shape. If it is a numeral we return the
/// compact `ChurchNumeral(n)` key; otherwise we fall back to a structural hash
/// of the same quoted term. In both cases the *pool stores only the compact
/// key* — the transient normal form is observed once and discarded.
///
/// `fuel` bounds the quote (it charges per normal-form node, so a numeral whose
/// expansion exceeds the budget — e.g. 3^10 ≈ 118k nodes — aborts and the caller
/// drops it, exactly as the engine already drops values it cannot hash within
/// budget). `h` is a scratch hasher for the structural fallback.
pub fn canonicalize(
    v: &Val,
    fuel: &mut Fuel,
    h: &mut DefaultHasher,
) -> Result<CanonicalValue, Abort> {
    // Quote to the normal-form term. This is the one-time canonicalization pass
    // with the full evaluation budget; the value itself (`Rc<Val>`) is untouched.
    let nf = quote(v, 0, fuel)?;
    if let Some(n) = numeral_of_term(&nf) {
        bump_nodes(n);
        Ok(CanonicalValue::ChurchNumeral(n))
    } else {
        hash_term(&nf, h);
        Ok(CanonicalValue::StructuralHash(h.finish()))
    }
}

/// Recognize the exact Church-numeral shape `λf.λx.f^n(x)` in a quoted,
/// β-normal `Term`. In de Bruijn the outer binder is `f = Var(1)`, the inner is
/// `x = Var(0)`; the body must be a right-nested chain of applications of `f`
/// to the seed `x` — `f(f(...(f(x))...))` — and `n` counts the `f`s. Returns
/// `None` when the term is not exactly this shape (caller falls back to
/// structural hashing).
fn numeral_of_term(t: &Term) -> Option<u64> {
    let Term::Lam(fb) = t else {
        return None;
    };
    let Term::Lam(xb) = fb.as_ref() else {
        return None;
    };
    let mut count = 0u64;
    let mut cur = xb.as_ref();
    loop {
        match cur {
            // Seed `x` alone → numeral 0.
            Term::Var(0) => return Some(count),
            // An application of `f` to the remainder.
            Term::App(h, rest) => {
                if !matches!(h.as_ref(), Term::Var(1)) {
                    return None;
                }
                count += 1;
                cur = rest.as_ref();
            }
            _ => return None,
        }
    }
}

/// Hash a β-normal `Term` structurally, for the non-numeral fallback. Tag the
/// constructor into a distinct band from the numeral tag so a numeral can never
/// collide with a structural hash.
fn hash_term(t: &Term, h: &mut DefaultHasher) {
    match t {
        Term::Var(i) => {
            h.write_u8(0);
            h.write_u32(*i);
        }
        Term::Free(i) => {
            h.write_u8(1);
            h.write_u32(*i);
        }
        Term::Lam(b) => {
            h.write_u8(2);
            hash_term(b, h);
        }
        Term::Prim(b) => {
            h.write_u8(3);
            hash_term(b, h);
        }
        Term::App(f, a) => {
            h.write_u8(4);
            hash_term(f, h);
            hash_term(a, h);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    use std::rc::Rc;

    fn num(n: u64) -> Rc<crate::term::Term> {
        let e = parse::parse_expr(&crate::bootstrap::church_num_str(n as u32)).unwrap();
        parse::to_term(&e).unwrap()
    }

    fn canon_key(n: u64) -> CanonicalValue {
        let empty: Env = Rc::new(Vec::new());
        let t = num(n);
        let mut fuel = Fuel(1_000_000);
        let v = eval(&empty, &t, &mut fuel).unwrap();
        let mut h = DefaultHasher::new();
        canonicalize(&v, &mut fuel, &mut h).unwrap()
    }

    #[test]
    fn recognize_plain_numerals() {
        for n in [0u64, 1, 2, 3, 6, 8] {
            assert_eq!(canon_key(n), CanonicalValue::ChurchNumeral(n), "numeral {n}");
        }
    }

    #[test]
    fn product_normalizes_and_recognizes() {
        // mul applied to two numerals must β-reduce to the product numeral, then
        // be recognized as ChurchNumeral(a*b) — NO arithmetic shortcut.
        let mul = parse::parse_expr("λa.λb.λc.b(a(c))").and_then(|e| parse::to_term(&e)).unwrap();
        let empty: Env = Rc::new(Vec::new());
        let a = num(3);
        let b = num(4);
        let mut fuel = Fuel(1_000_000);
        // mul a b  =  App(App(mul,a),b) — evaluates to a *closure* λc.b(a(c))
        // with 3,4 captured, NOT a syntactically numeral-shaped value. The
        // canonicalizer must still quote it to the numeral and recognize it.
        let applied = crate::term::app(crate::term::app(mul, a), b);
        let v = eval(&empty, &applied, &mut fuel).unwrap();
        let mut h = DefaultHasher::new();
        let k = canonicalize(&v, &mut fuel, &mut h).unwrap();
        assert_eq!(k, CanonicalValue::ChurchNumeral(12), "3*4 not recognized");
        // And its key must equal the key of the closed numeral 12 directly.
        assert_eq!(k, canon_key(12));
    }
}
