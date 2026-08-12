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
use std::rc::Rc;

/// A canonical semantic key for a closed normal form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalValue {
    /// The exact shape `λf.λx.f^n(x)` — a Church numeral, compact.
    ChurchNumeral(u64),
    /// A rectangular nested Church list-of-lists-of-Church-numerals (a grid),
    /// recognized compactly at the Val level — O(wh), no numeral expansion.
    Grid {
        width: u64,
        height: u64,
        cells: Vec<u64>,
    },
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
            // Grids take the top bit (bit63), disjoint from numerals (bit62) and
            // structural hashes (neither). The key is a hash of the compact
            // (width,height,cells) identity — representation only, no semantics.
            CanonicalValue::Grid {
                width,
                height,
                cells,
            } => {
                let mut h = DefaultHasher::new();
                h.write_u64(*width);
                h.write_u64(*height);
                for c in cells {
                    h.write_u64(*c);
                }
                (1u64 << 63) | (h.finish() & ((1u64 << 63) - 1))
            }
            CanonicalValue::StructuralHash(h) => *h & ((1u64 << 62) - 1),
        }
    }

    /// The compact representation cost of a recognized value — the node-walk the
    /// canonicalizer spent to observe it, used for the `canon_nodes`/`max_transient`
    /// meters. For a numeral this is `n` (matching the historical quote-path
    /// accounting); for a grid it is one node per cell plus one per row plus the
    /// grid itself — the O(wh) compact walk, not the expanded normal form.
    fn walk_cost(&self) -> u64 {
        match self {
            CanonicalValue::ChurchNumeral(n) => *n,
            CanonicalValue::Grid { height, cells, .. } => cells.len() as u64 + *height + 1,
            CanonicalValue::StructuralHash(_) => 0,
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

/// Recognize a Church numeral `λf.λx.f^n(x)` in a `Val` by binding `f`/`x` to
/// fresh neutrals, evaluating the body, and walking the left-nested `f`-chain
/// down to `x`. This mirrors how `quote` handles `Val::Lam` but inspects the
/// spine instead of building the expanded term, so a numeral costs O(n) fuel
/// (the chain walk), not O(2n+2) nodes of materialized normal form. Returns
/// `None` when `v` is not exactly this shape.
fn numeral_value(v: &Val, fuel: &mut Fuel) -> Result<Option<u64>, Abort> {
    let Val::Lam(env, body) = v else {
        return Ok(None);
    };
    // Bind f = Bound(0), evaluate the body → should be λx.rest.
    let mut env2 = (**env).clone();
    env2.push(thunk_of_val(Val::Neu(Head::Bound(0), Vec::new())));
    let xlam = eval(&Rc::new(env2), body, fuel)?;
    let Val::Lam(env3, body2) = xlam.as_ref() else {
        return Ok(None);
    };
    // Bind x = Bound(1), evaluate → the numeral spine.
    let mut env4 = (**env3).clone();
    env4.push(thunk_of_val(Val::Neu(Head::Bound(1), Vec::new())));
    let xv = eval(&Rc::new(env4), body2, fuel)?;
    // xv is a left-nested chain Neu(f,[Neu(f,[...Neu(f,[x])...])]); count the f's.
    let mut n = 0u64;
    let mut cur = xv;
    loop {
        // Base: the value is exactly the bound x (Bound(1), empty spine).
        if matches!(cur.as_ref(), Val::Neu(Head::Bound(1), sp) if sp.is_empty()) {
            return Ok(Some(n));
        }
        let Val::Neu(head, sp) = cur.as_ref() else {
            return Ok(None);
        };
        if *head != Head::Bound(0) {
            return Ok(None);
        }
        let [inner] = sp.as_slice() else {
            return Ok(None);
        };
        cur = force(inner, fuel)?;
        n += 1;
    }
}

/// Recognize a Church list `λf.λz.f(e1)(f(e2)(...f(ek)(z)))` in a `Val` and
/// return the element values. The normal form is *left-nested*: `f(e1)(f(e2)(z))`
/// evaluates to `Neu(f, [e1, Neu(f, [e2, z])])`, so each spine is `[elem, rest]`
/// where `rest` is either the base `z` (empty spine) or another `Neu(f, [next, rest2])`.
/// `None` when `v` is not exactly this shape (including the empty list).
fn church_list_elems(v: &Val, fuel: &mut Fuel) -> Result<Option<Vec<Rc<Val>>>, Abort> {
    let Val::Lam(env, body) = v else {
        return Ok(None);
    };
    // Bind f = Bound(0), evaluate → should be λz.rest.
    let mut env2 = (**env).clone();
    env2.push(thunk_of_val(Val::Neu(Head::Bound(0), Vec::new())));
    let zlam = eval(&Rc::new(env2), body, fuel)?;
    let Val::Lam(env3, body2) = zlam.as_ref() else {
        return Ok(None);
    };
    // Bind z = Bound(1), evaluate → the list spine.
    let mut env4 = (**env3).clone();
    env4.push(thunk_of_val(Val::Neu(Head::Bound(1), Vec::new())));
    let zv = eval(&Rc::new(env4), body2, fuel)?;
    // Walk the left-nested spine: [elem, rest], rest = z or Neu(f,[next,rest2]).
    let mut out = Vec::new();
    let mut cur = zv;
    loop {
        let Val::Neu(head, sp) = cur.as_ref() else {
            return Ok(None);
        };
        if *head != Head::Bound(0) {
            return Ok(None);
        }
        let [elem, rest] = sp.as_slice() else {
            return Ok(None);
        };
        out.push(force(elem, fuel)?);
        let rest_v = force(rest, fuel)?;
        // Base: rest is the bound z (empty spine).
        if matches!(rest_v.as_ref(), Val::Neu(Head::Bound(1), s) if s.is_empty()) {
            return Ok(Some(out));
        }
        cur = rest_v;
    }
}

/// Recognize a closed `Val` as a compact canonical value: a Church numeral, or a
/// rectangular nested Church list-of-lists-of-Church-numerals (a grid). Returns
/// `None` when the shape isn't recognized (the caller falls back to full quote).
fn recognize_compact(v: &Val, fuel: &mut Fuel) -> Result<Option<CanonicalValue>, Abort> {
    // A single numeral.
    if let Some(n) = numeral_value(v, fuel)? {
        return Ok(Some(CanonicalValue::ChurchNumeral(n)));
    }
    // A grid: a list of rows, each a list of numerals, all rows equal length.
    let Some(rows) = church_list_elems(v, fuel)? else {
        return Ok(None);
    };
    let mut width: Option<u64> = None;
    let mut cells: Vec<u64> = Vec::new();
    for row in &rows {
        let Some(row_elems) = church_list_elems(row, fuel)? else {
            return Ok(None);
        };
        let w = row_elems.len() as u64;
        if let Some(prev) = width {
            if prev != w {
                return Ok(None); // ragged — not a rectangular grid
            }
        } else {
            width = Some(w);
        }
        for cell in &row_elems {
            match numeral_value(cell, fuel)? {
                Some(n) => cells.push(n),
                None => return Ok(None),
            }
        }
    }
    let width = width.unwrap_or(0);
    let height = rows.len() as u64;
    Ok(Some(CanonicalValue::Grid {
        width,
        height,
        cells,
    }))
}

/// Observe `v` (a closed normal form, depth 0) and return its canonical key.
///
/// Recognition first tries a compact Val-level pass ([`recognize_compact`]) that
/// walks the value directly — binding fresh neutrals and inspecting spines, the
/// way `quote` does — to collapse a Church numeral to `ChurchNumeral(n)` or a
/// rectangular nested list-of-lists-of-numerals to `Grid{width,height,cells}` in
/// O(size), *without* materializing the expanded normal form. This is what keeps
/// ARC-sized grids hashable within budget: a 30×30 grid is ~900 compact cells,
/// not ~10k expanded `f(f(...f(x)))` nodes.
///
/// Only when the shape is not recognized does it fall back to the historical
/// path: quote `v` to its β-normal term, check for the exact Church-numeral
/// shape, else structural-hash the quoted term. In all cases the *pool stores
/// only the compact key* — the transient normal form is observed once and
/// discarded.
///
/// `fuel` bounds the walk (it charges per node, so a value whose expansion
/// exceeds the budget aborts and the caller drops it, exactly as the engine
/// already drops values it cannot hash within budget). `h` is a scratch hasher
/// for the structural fallback.
pub fn canonicalize(
    v: &Val,
    fuel: &mut Fuel,
    h: &mut DefaultHasher,
) -> Result<CanonicalValue, Abort> {
    // Compact Val-level recognition first — O(size), no full-term expansion.
    if let Some(cv) = recognize_compact(v, fuel)? {
        bump_nodes(cv.walk_cost());
        return Ok(cv);
    }
    // Fallback: full quote + numeral-of-term + structural hash (as before).
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
            assert_eq!(
                canon_key(n),
                CanonicalValue::ChurchNumeral(n),
                "numeral {n}"
            );
        }
    }

    #[test]
    fn product_normalizes_and_recognizes() {
        // mul applied to two numerals must β-reduce to the product numeral, then
        // be recognized as ChurchNumeral(a*b) — NO arithmetic shortcut.
        let mul = parse::parse_expr("λa.λb.λc.b(a(c))")
            .and_then(|e| parse::to_term(&e))
            .unwrap();
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

    /// Build a Church list term `[items]` = cons i1 (cons i2 (... nil)).
    fn church_list_term(items: &[Rc<crate::term::Term>]) -> Rc<crate::term::Term> {
        let cons = parse::parse_expr("λc.λs.λf.λz.f(c)(s(f)(z))")
            .and_then(|e| parse::to_term(&e))
            .unwrap();
        let nil = parse::parse_expr("λf.λz.z")
            .and_then(|e| parse::to_term(&e))
            .unwrap();
        items.iter().rev().fold(nil, |acc, it| {
            crate::term::app(crate::term::app(cons.clone(), it.clone()), acc)
        })
    }

    /// Build a grid term: a Church list of rows, each a Church list of numerals.
    fn grid_term(rows: &[&[u64]]) -> Rc<crate::term::Term> {
        let row_terms: Vec<Rc<crate::term::Term>> = rows
            .iter()
            .map(|row| {
                let cells: Vec<Rc<crate::term::Term>> = row.iter().map(|&c| num(c)).collect();
                church_list_term(&cells)
            })
            .collect();
        church_list_term(&row_terms)
    }

    /// A rectangular nested list-of-lists-of-numerals must canonicalize to a
    /// compact `Grid` with the right width/height/cells, and its key must sit in
    /// the grid band (bit63), disjoint from the numeral and structural bands.
    #[test]
    fn grid_recognized_compact() {
        let empty: Env = Rc::new(Vec::new());
        let g = grid_term(&[&[1, 2, 3], &[3, 1, 2]]); // 2 rows × 3 cols
        let mut fuel = Fuel(1_000_000);
        let v = eval(&empty, &g, &mut fuel).unwrap();
        let mut h = DefaultHasher::new();
        let cv = canonicalize(&v, &mut fuel, &mut h).unwrap();
        match cv {
            CanonicalValue::Grid {
                width,
                height,
                ref cells,
            } => {
                assert_eq!(width, 3);
                assert_eq!(height, 2);
                assert_eq!(*cells, vec![1, 2, 3, 3, 1, 2]);
            }
            other => panic!("expected Grid, got {other:?}"),
        }
        let k = cv.key();
        assert!(k & (1 << 63) != 0, "grid key must be in the grid band");
        assert_ne!(k, CanonicalValue::ChurchNumeral(1).key());
        assert_ne!(k, CanonicalValue::StructuralHash(0).key());
    }

    /// A flat list of numerals is NOT a grid (not list-of-lists) → falls back to
    /// the structural hash, exactly as before.
    #[test]
    fn non_grid_falls_back_to_structural() {
        let empty: Env = Rc::new(Vec::new());
        let flat = church_list_term(&[num(1), num(2)]);
        let mut fuel = Fuel(1_000_000);
        let v = eval(&empty, &flat, &mut fuel).unwrap();
        let mut h = DefaultHasher::new();
        let cv = canonicalize(&v, &mut fuel, &mut h).unwrap();
        assert!(
            matches!(cv, CanonicalValue::StructuralHash(_)),
            "flat list must fall back to structural, got {cv:?}"
        );
    }
}
