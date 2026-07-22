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
    /// Signed integer (balanced-ternary tasks, signed literals).
    Int(i64),
    /// The i-th constructor *function* of an ADT (ctr tasks).
    CtorFn(u32),
    /// The pairing combinator λa.λb.λp.p(a,b) (mrg tasks' F).
    PairFn,
    /// Fixed-width tuple (Scott pair/triple λp.p(x, y[, z])) — distinct from
    /// List so the compiler can emit the right encoding.
    Tup(Vec<V>),
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

/// Church binary (LSB first, no trailing zeros), in normal form:
/// E = λo.λi.λe.e; the fold of I(O(I(E))) is λo.λi.λe.i(o(i(e))).
pub fn church_bin(t: &Term) -> Option<u64> {
    let Term::Lam(b1) = t else { return None };
    let Term::Lam(b2) = b1.as_ref() else {
        return None;
    };
    let Term::Lam(b3) = b2.as_ref() else {
        return None;
    };
    let mut val = 0u64;
    let mut pos = 0u32;
    let mut cur = b3.as_ref();
    loop {
        match cur {
            Term::Var(0) => {
                // no trailing zeros: the last constructor must be I (or none)
                if pos > 0 && val >> (pos - 1) == 0 {
                    return None;
                }
                return Some(val);
            }
            Term::App(h, rest) => {
                let bit = match h.as_ref() {
                    Term::Var(2) => 0u64,
                    Term::Var(1) => 1u64,
                    _ => return None,
                };
                if pos >= 63 {
                    return None;
                }
                val |= bit << pos;
                pos += 1;
                cur = rest.as_ref();
            }
            _ => return None,
        }
    }
}

/// Scott binary: E = λo.λi.λe.e, O(x) = λo.λi.λe.o(x), I(x) = λo.λi.λe.i(x).
pub fn scott_bin(t: &Term) -> Option<u64> {
    let mut val = 0u64;
    let mut pos = 0u32;
    let mut cur = t;
    loop {
        let Term::Lam(b1) = cur else { return None };
        let Term::Lam(b2) = b1.as_ref() else {
            return None;
        };
        let Term::Lam(b3) = b2.as_ref() else {
            return None;
        };
        match b3.as_ref() {
            Term::Var(0) => {
                if pos > 0 && val >> (pos - 1) == 0 {
                    return None;
                }
                return Some(val);
            }
            Term::App(h, rest) => {
                let bit = match h.as_ref() {
                    Term::Var(2) => 0u64,
                    Term::Var(1) => 1u64,
                    _ => return None,
                };
                if pos >= 63 {
                    return None;
                }
                val |= bit << pos;
                pos += 1;
                cur = rest.as_ref();
            }
            _ => return None,
        }
    }
}

/// Church tree in normal form: peel λn.λl., then body ::= l(x) | n(B, B).
pub fn church_tree(t: &Rc<Term>) -> Option<V> {
    let Term::Lam(b1) = t.as_ref() else { return None };
    let Term::Lam(b2) = b1.as_ref() else {
        return None;
    };
    fn body(t: &Rc<Term>) -> Option<V> {
        match t.as_ref() {
            Term::App(h, x) => match h.as_ref() {
                Term::Var(0) => decode_value(&unshift(x, 2)?),
                Term::App(n, a) if matches!(n.as_ref(), Term::Var(1)) => Some(V::Node(
                    Box::new(body(a)?),
                    Box::new(body(x)?),
                )),
                _ => None,
            },
            _ => None,
        }
    }
    body(b2)
}

/// Scott tree: Leaf(x) = λn.λl.l(x), Node(a,b) = λn.λl.n(a, b) with full
/// sub-encodings as children.
pub fn scott_tree(t: &Rc<Term>) -> Option<V> {
    let Term::Lam(b1) = t.as_ref() else { return None };
    let Term::Lam(b2) = b1.as_ref() else {
        return None;
    };
    match b2.as_ref() {
        Term::App(h, x) => match h.as_ref() {
            Term::Var(0) => decode_value(&unshift(x, 2)?),
            Term::App(n, a) if matches!(n.as_ref(), Term::Var(1)) => Some(V::Node(
                Box::new(scott_tree(&unshift(a, 2)?)?),
                Box::new(scott_tree(&unshift(x, 2)?)?),
            )),
            _ => None,
        },
        _ => None,
    }
}

/// ADT type descriptor: a Scott list of constructor specs, each a Scott
/// list of λx.x recursive-field markers. Returns field counts per ctor.
pub fn adt_desc(t: &Rc<Term>) -> Option<Vec<usize>> {
    let specs = scott_list_raw(t)?;
    let mut out = Vec::new();
    for s in specs {
        let fields = scott_list_raw(&s)?;
        for f in &fields {
            if !matches!(f.as_ref(), Term::Lam(b) if matches!(b.as_ref(), Term::Var(0))) {
                return None;
            }
        }
        out.push(fields.len());
    }
    Some(out)
}

/// Scott list with raw term elements (no value decoding).
pub fn scott_list_raw(t: &Rc<Term>) -> Option<Vec<Rc<Term>>> {
    let mut items = Vec::new();
    let mut cur: Rc<Term> = t.clone();
    loop {
        let Term::Lam(b1) = cur.as_ref() else {
            return None;
        };
        let Term::Lam(b2) = b1.as_ref() else {
            return None;
        };
        let step: Option<(Rc<Term>, Rc<Term>)> = match b2.as_ref() {
            Term::Var(0) => None,
            Term::App(fh, tail) => {
                let Term::App(c, head) = fh.as_ref() else {
                    return None;
                };
                if !matches!(c.as_ref(), Term::Var(1)) {
                    return None;
                }
                Some((unshift(head, 2)?, unshift(tail, 2)?))
            }
            _ => return None,
        };
        match step {
            None => return Some(items),
            Some((h, tl)) => {
                items.push(h);
                cur = tl;
            }
        }
    }
}

/// Church-encoded ADT value for a given shape (field counts per ctor):
/// λh0..λhN-1. body, body ::= h_i(child_bodies...) — children are folds
/// over the same binders. Free-headed spines decode as V::App (merge's F).
pub fn church_adt(shape: &[usize], t: &Rc<Term>) -> Option<V> {
    let n = shape.len();
    let mut cur = t;
    for _ in 0..n {
        let Term::Lam(b) = cur.as_ref() else {
            return None;
        };
        cur = b;
    }
    church_adt_body(shape, cur, n as u32)
}

fn church_adt_body(shape: &[usize], t: &Rc<Term>, n: u32) -> Option<V> {
    let mut spine = Vec::new();
    let mut cur = t;
    while let Term::App(f, a) = cur.as_ref() {
        spine.push(a);
        cur = f;
    }
    spine.reverse();
    match cur.as_ref() {
        Term::Var(i) if *i < n => {
            let tag = (n - 1 - i) as usize;
            if spine.len() != shape[tag] {
                return None;
            }
            let mut fields = Vec::new();
            for a in spine {
                fields.push(church_adt_body(shape, a, n)?);
            }
            Some(V::Ctr(tag as u32, fields))
        }
        Term::Free(_) => {
            // F(V1, V2) spine: head atom applied to church-adt args.
            let mut v = decode_value(cur)?;
            for a in spine {
                let sub = church_adt(shape, &unshift(a, n)?)
                    .or_else(|| decode_value(&unshift(a, n).unwrap_or_else(|| a.clone())))?;
                v = V::App(Box::new(v), Box::new(sub));
            }
            Some(v)
        }
        _ => None,
    }
}

/// Scott-encoded ADT value: λc0..λcN-1. c_i(full child encodings).
pub fn scott_adt(shape: &[usize], t: &Rc<Term>) -> Option<V> {
    let n = shape.len();
    let mut cur = t.clone();
    for _ in 0..n {
        let Term::Lam(b) = cur.as_ref() else {
            return None;
        };
        cur = b.clone();
    }
    let mut spine = Vec::new();
    let mut head = cur.clone();
    while let Term::App(f, a) = head.clone().as_ref() {
        spine.push(a.clone());
        head = f.clone();
    }
    spine.reverse();
    match head.as_ref() {
        Term::Var(i) if (*i as usize) < n => {
            let tag = n - 1 - *i as usize;
            if spine.len() != shape[tag] {
                return None;
            }
            let mut fields = Vec::new();
            for a in &spine {
                fields.push(scott_adt(shape, &unshift(a, n as u32)?)?);
            }
            Some(V::Ctr(tag as u32, fields))
        }
        Term::Free(_) => {
            let mut v = decode_value(&head)?;
            for a in &spine {
                let sub = scott_adt(shape, &unshift(a, n as u32)?)?;
                v = V::App(Box::new(v), Box::new(sub));
            }
            Some(v)
        }
        _ => None,
    }
}

/// Balanced ternary, Scott-encoded, LSB first:
/// E = λt.λo.λi.λe.e, T(x) = ...t(x), O(x) = ...o(x), I(x) = ...i(x).
/// Canonical only: no trailing O digit.
pub fn scott_bt(t: &Rc<Term>) -> Option<i64> {
    let mut val = 0i64;
    let mut place = 1i64;
    let mut last_zero = false;
    let mut any = false;
    let mut cur = t.clone();
    loop {
        let mut b = cur.as_ref();
        for _ in 0..4 {
            let Term::Lam(inner) = b else { return None };
            b = inner.as_ref();
        }
        match b {
            Term::Var(0) => {
                if any && last_zero {
                    return None;
                }
                return Some(val);
            }
            Term::App(h, rest) => {
                let d: i64 = match h.as_ref() {
                    Term::Var(3) => -1,
                    Term::Var(2) => 0,
                    Term::Var(1) => 1,
                    _ => return None,
                };
                val += d * place;
                place = place.checked_mul(3)?;
                last_zero = d == 0;
                any = true;
                cur = unshift(rest, 4)?;
            }
            _ => return None,
        }
    }
}

/// Church balanced ternary: same digits but the tail is a fold under shared
/// binders — T(I(E)) = λt.λo.λi.λe.t(i(e)).
pub fn church_bt(t: &Term) -> Option<i64> {
    let mut b = t;
    for _ in 0..4 {
        let Term::Lam(inner) = b else { return None };
        b = inner.as_ref();
    }
    let mut val = 0i64;
    let mut place = 1i64;
    let mut last_zero = false;
    let mut any = false;
    let mut cur = b;
    loop {
        match cur {
            Term::Var(0) => {
                if any && last_zero {
                    return None;
                }
                return Some(val);
            }
            Term::App(h, rest) => {
                let d: i64 = match h.as_ref() {
                    Term::Var(3) => -1,
                    Term::Var(2) => 0,
                    Term::Var(1) => 1,
                    _ => return None,
                };
                val += d * place;
                place = place.checked_mul(3)?;
                last_zero = d == 0;
                any = true;
                cur = rest.as_ref();
            }
            _ => return None,
        }
    }
}

/// Balanced-ternary digits of n, LSB first (empty for 0).
fn bt_digits(mut n: i64) -> Vec<i64> {
    let mut ds = Vec::new();
    while n != 0 {
        let mut r = n.rem_euclid(3);
        n = n.div_euclid(3);
        if r == 2 {
            r = -1;
            n += 1;
        }
        ds.push(r);
    }
    ds
}

pub fn scott_bt_term(n: i64) -> Rc<Term> {
    let mut t = lam(lam(lam(lam(var(0)))));
    for &d in bt_digits(n).iter().rev() {
        let idx = match d {
            -1 => 3,
            0 => 2,
            _ => 1,
        };
        t = lam(lam(lam(lam(app(var(idx), t)))));
    }
    t
}

pub fn church_bt_term(n: i64) -> Rc<Term> {
    let mut body = var(0);
    for &d in bt_digits(n).iter().rev() {
        let idx = match d {
            -1 => 3,
            0 => 2,
            _ => 1,
        };
        body = app(var(idx), body);
    }
    lam(lam(lam(lam(body))))
}

/// Church GN tree (ctre_fft): root spines are plain (`n(a,b)` / `l(x)`),
/// sub-level spines are self-passing (`n(a,b,n,l)` / `l(x,n,l)`), and leaf
/// content restarts the grammar (down to Church-BT scalars). Decodes to the
/// collapsed Node/Int form.
pub fn church_gn(t: &Rc<Term>) -> Option<V> {
    fn spine(t: &Rc<Term>) -> Option<(bool, Vec<Rc<Term>>)> {
        // peel λn.λl., collect application spine; head Var(1)=node / Var(0)=leaf
        let Term::Lam(b1) = t.as_ref() else { return None };
        let Term::Lam(b2) = b1.as_ref() else {
            return None;
        };
        let mut args = Vec::new();
        let mut cur = b2.clone();
        while let Term::App(f, a) = cur.clone().as_ref() {
            args.push(a.clone());
            cur = f.clone();
        }
        args.reverse();
        match cur.as_ref() {
            Term::Var(1) => Some((true, args)),
            Term::Var(0) => Some((false, args)),
            _ => None,
        }
    }
    fn content(x: &Rc<Term>) -> Option<V> {
        if let Some(n) = church_bt(x) {
            return Some(V::Int(n));
        }
        plain(x)
    }
    fn sub(t: &Rc<Term>) -> Option<V> {
        let (is_node, args) = spine(t)?;
        if is_node && args.len() == 4 {
            if !matches!(args[2].as_ref(), Term::Var(1)) || !matches!(args[3].as_ref(), Term::Var(0)) {
                return None;
            }
            Some(V::Node(
                Box::new(sub(&unshift(&args[0], 2)?)?),
                Box::new(sub(&unshift(&args[1], 2)?)?),
            ))
        } else if !is_node && args.len() == 3 {
            if !matches!(args[1].as_ref(), Term::Var(1)) || !matches!(args[2].as_ref(), Term::Var(0)) {
                return None;
            }
            content(&unshift(&args[0], 2)?)
        } else {
            None
        }
    }
    fn plain(t: &Rc<Term>) -> Option<V> {
        let (is_node, args) = spine(t)?;
        if is_node && args.len() == 2 {
            Some(V::Node(
                Box::new(sub(&unshift(&args[0], 2)?)?),
                Box::new(sub(&unshift(&args[1], 2)?)?),
            ))
        } else if !is_node && args.len() == 1 {
            content(&unshift(&args[0], 2)?)
        } else {
            None
        }
    }
    plain(t)
}


/// The pairing combinator λa.λb.λp.p(a, b).
pub fn pair_fn(t: &Term) -> bool {
    // Lam(Lam(Lam(App(App(Var0, Var2), Var1))))
    let Term::Lam(b1) = t else { return false };
    let Term::Lam(b2) = b1.as_ref() else {
        return false;
    };
    let Term::Lam(b3) = b2.as_ref() else {
        return false;
    };
    let Term::App(f, a1) = b3.as_ref() else {
        return false;
    };
    let Term::App(p, a2) = f.as_ref() else {
        return false;
    };
    matches!(p.as_ref(), Term::Var(0))
        && matches!(a2.as_ref(), Term::Var(2))
        && matches!(a1.as_ref(), Term::Var(1))
}

/// Build the i-th constructor function's normal form for a shape, in the
/// family's encoding, and compare against `t`. λf1..fk.λc0..cN-1.body where
/// body = c_i(fields), fields = f-vars (Scott) or f-var folds (Church).
pub fn ctor_fn(shape: &[usize], t: &Rc<Term>, church: bool) -> Option<u32> {
    let n = shape.len() as u32;
    for (i, &k) in shape.iter().enumerate() {
        let k = k as u32;
        // head variable c_i at de Bruijn n-1-i from innermost ctor binder
        let mut body: Rc<Term> = var(n - 1 - i as u32);
        for j in 0..k {
            // field j: outer param f_{j+1} sits above the n ctor binders
            let fv = var(n + (k - 1 - j));
            let field = if church {
                // church fields are folds: f(c0, .., cN-1)
                let mut e = fv;
                for c in (0..n).rev() {
                    e = app(e, var(c));
                }
                e
            } else {
                fv
            };
            body = app(body, field);
        }
        let mut full = body;
        for _ in 0..n {
            full = lam(full);
        }
        for _ in 0..k {
            full = lam(full);
        }
        if full == *t {
            return Some(i as u32);
        }
    }
    None
}

/// Scott ADT value that may contain 2-tuples (merge results) at any value
/// position: value ::= Ctr(tag, [value..]) | Tup([value, value]).
pub fn scott_adt_p(shape: &[usize], t: &Rc<Term>) -> Option<V> {
    if let Some(items) = ntuple_raw(t) {
        if items.len() == 2 {
            return Some(V::Tup(vec![
                scott_adt_p(shape, &items[0])?,
                scott_adt_p(shape, &items[1])?,
            ]));
        }
    }
    let (tag, fields) = scott_ctr(t, shape)?;
    let fs: Option<Vec<V>> = fields.iter().map(|f| scott_adt_p(shape, f)).collect();
    Some(V::Ctr(tag as u32, fs?))
}

/// Church ADT value with 2-tuples allowed at value positions. Pairs are
/// opaque (not folded), so they appear as raw λp.p(A, B) subterms whose
/// elements are full encodings.
pub fn church_adt_p(shape: &[usize], t: &Rc<Term>) -> Option<V> {
    if let Some(items) = ntuple_raw(t) {
        if items.len() == 2 {
            return Some(V::Tup(vec![
                church_adt_p(shape, &items[0])?,
                church_adt_p(shape, &items[1])?,
            ]));
        }
    }
    let n = shape.len();
    let mut cur = t.clone();
    for _ in 0..n {
        let Term::Lam(b) = cur.as_ref() else {
            return None;
        };
        cur = b.clone();
    }
    church_adt_p_body(shape, &cur, n as u32)
}

fn church_adt_p_body(shape: &[usize], t: &Rc<Term>, n: u32) -> Option<V> {
    // A field that doesn't use the shared fold binders is a foreign closed
    // subterm: either a pair, or a full re-bound value (which happens when a
    // constructor's field contains pairs and so can't be a shared fold).
    if let Some(lifted) = unshift(t, n) {
        if let Some(items) = ntuple_raw(&lifted) {
            if items.len() == 2 {
                return Some(V::Tup(vec![
                    church_adt_p(shape, &items[0])?,
                    church_adt_p(shape, &items[1])?,
                ]));
            }
        }
        if let Some(v) = church_adt_p(shape, &lifted) {
            return Some(v);
        }
    }
    let mut spine = Vec::new();
    let mut cur = t.clone();
    while let Term::App(f, a) = cur.clone().as_ref() {
        spine.push(a.clone());
        cur = f.clone();
    }
    spine.reverse();
    let Term::Var(i) = cur.as_ref() else {
        return None;
    };
    if *i >= n {
        return None;
    }
    let tag = (n - 1 - i) as usize;
    if spine.len() != shape.get(tag).copied()? {
        return None;
    }
    let mut fields = Vec::new();
    for a in &spine {
        fields.push(church_adt_p_body(shape, a, n)?);
    }
    Some(V::Ctr(tag as u32, fields))
}

/// Strict Church bool: λa.λb.a / λa.λb.b only.
pub fn strict_bool(t: &Term) -> Option<bool> {
    let Term::Lam(b1) = t else { return None };
    let Term::Lam(b2) = b1.as_ref() else {
        return None;
    };
    match b2.as_ref() {
        Term::Var(1) => Some(true),
        Term::Var(0) => Some(false),
        _ => None,
    }
}

/// Scott constructor for a known shape: λc0..λcN-1. c_tag(f1, .., fk).
/// Returns the tag and the (unshifted) field terms.
pub fn scott_ctr(t: &Rc<Term>, shape: &[usize]) -> Option<(usize, Vec<Rc<Term>>)> {
    let n = shape.len();
    let mut cur = t.clone();
    for _ in 0..n {
        let Term::Lam(b) = cur.as_ref() else {
            return None;
        };
        cur = b.clone();
    }
    let mut spine = Vec::new();
    let mut head = cur;
    while let Term::App(f, a) = head.clone().as_ref() {
        spine.push(a.clone());
        head = f.clone();
    }
    spine.reverse();
    let Term::Var(i) = head.as_ref() else {
        return None;
    };
    if *i as usize >= n {
        return None;
    }
    let tag = n - 1 - *i as usize;
    if spine.len() != shape[tag] {
        return None;
    }
    let fields: Option<Vec<Rc<Term>>> = spine.iter().map(|a| unshift(a, n as u32)).collect();
    Some((tag, fields?))
}

/// Raw n-tuple elements: λt.t(A, B, C) without value decoding.
pub fn ntuple_raw(t: &Rc<Term>) -> Option<Vec<Rc<Term>>> {
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
        out.push(unshift(a, 1)?);
    }
    Some(out)
}

// ── Algo-family recipe decoders ─────────────────────────────────────

pub fn d_snat(t: &Rc<Term>) -> Option<V> {
    Some(V::Nat(scott_nat(t)?))
}

pub fn d_bool(t: &Rc<Term>) -> Option<V> {
    Some(V::Bool(strict_bool(t)?))
}

pub fn d_slist_of(t: &Rc<Term>, elem: &dyn Fn(&Rc<Term>) -> Option<V>) -> Option<V> {
    let items = scott_list_raw(t)?;
    Some(V::List(
        items.iter().map(elem).collect::<Option<Vec<V>>>()?,
    ))
}

pub fn d_tup_of(
    t: &Rc<Term>,
    arity: usize,
    elem: &dyn Fn(&Rc<Term>) -> Option<V>,
) -> Option<V> {
    let items = ntuple_raw(t)?;
    if items.len() != arity {
        return None;
    }
    Some(V::Tup(
        items.iter().map(elem).collect::<Option<Vec<V>>>()?,
    ))
}

/// SAT literal: Pos(n) = λp.λn.p(n) → +(n+1); Neg(n) → −(n+1). DIMACS-style.
pub fn d_lit(t: &Rc<Term>) -> Option<V> {
    let (tag, fs) = scott_ctr(t, &[1, 1])?;
    let n = scott_nat(&fs[0])? as i64;
    Some(V::Int(if tag == 0 { n + 1 } else { -(n + 1) }))
}

/// Brainfuck instruction: 7-variant Scott; Loop's field is a program list.
pub fn d_bf_instr(t: &Rc<Term>) -> Option<V> {
    let (tag, fs) = scott_ctr(t, &[0, 0, 0, 0, 0, 0, 1])?;
    if tag == 6 {
        Some(V::Ctr(6, vec![d_slist_of(&fs[0], &d_bf_instr)?]))
    } else {
        Some(V::Ctr(tag as u32, Vec::new()))
    }
}

/// de Bruijn λ term: Lam(body) | App(f, a) | Var(nat).
pub fn d_lam_term(t: &Rc<Term>) -> Option<V> {
    let (tag, fs) = scott_ctr(t, &[1, 2, 1])?;
    match tag {
        0 => Some(V::Ctr(0, vec![d_lam_term(&fs[0])?])),
        1 => Some(V::Ctr(1, vec![d_lam_term(&fs[0])?, d_lam_term(&fs[1])?])),
        _ => Some(V::Ctr(2, vec![V::Nat(scott_nat(&fs[0])?)])),
    }
}

/// STLC type: Base(nat) | Arr(a, b).
pub fn d_stlc_ty(t: &Rc<Term>) -> Option<V> {
    let (tag, fs) = scott_ctr(t, &[1, 2])?;
    match tag {
        0 => Some(V::Ctr(0, vec![V::Nat(scott_nat(&fs[0])?)])),
        _ => Some(V::Ctr(1, vec![d_stlc_ty(&fs[0])?, d_stlc_ty(&fs[1])?])),
    }
}

/// STLC term: Lam(ty, body) | App(f, x) | Var(nat).
pub fn d_stlc_term(t: &Rc<Term>) -> Option<V> {
    let (tag, fs) = scott_ctr(t, &[2, 2, 1])?;
    match tag {
        0 => Some(V::Ctr(0, vec![d_stlc_ty(&fs[0])?, d_stlc_term(&fs[1])?])),
        1 => Some(V::Ctr(1, vec![d_stlc_term(&fs[0])?, d_stlc_term(&fs[1])?])),
        _ => Some(V::Ctr(2, vec![V::Nat(scott_nat(&fs[0])?)])),
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
    // Balanced-ternary integers (fft leaves). Ordered late: 4-binder shapes
    // don't collide with the nat/list/bool recognizers above.
    if let Some(n) = scott_bt(t) {
        return Some(V::Int(n));
    }
    if let Some(n) = church_bt(t) {
        return Some(V::Int(n));
    }
    // Nested trees (GN numbers are L/B trees whose leaves are BT ints).
    // Ordered last: Cons-shaped nodes prefer the list recognizers above.
    if let Some(v) = scott_tree(t) {
        return Some(v);
    }
    if let Some(v) = church_tree(t) {
        return Some(v);
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
