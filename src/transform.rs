//! Generic program-transformation meta-search: context abstraction + composition.
//!
//! These are operations over program *representation* (syntax), NOT object-level
//! concepts available to the task solver. They are the meta-search machinery the
//! machine uses to invent new primitives by restructuring its own discovered code —
//! the distinction the project keeps forever: **meta-search machinery ≠ learned
//! ontology**.
//!
//! The core move is **context abstraction**: given a program `p = λc₁…λcₙ. body`
//! and a subterm `s` occurring ≥2 times in `body`, factor the repeated subterm into
//! a hole, producing BOTH a new concept `C = λx. body[s:=x]` AND a rewritten program
//! `p' = λc₁…λcₙ. C s` that uses it, with the invariant `p ≡ p'` (checked
//! semantically by the caller). This is NOT "stick a lambda around a repeated
//! subtree" — the binder is placed on the *body* (not the whole term, which would
//! yield `λx.λc. …`, binder in the wrong place) and de Bruijn indices are shifted
//! properly.

use crate::term::{self, Term};
use std::collections::HashSet;
use std::rc::Rc;

/// A context abstraction: a new concept `C`, the program `p'` rewritten to use it,
/// and the subterm `s` that was factored out. Invariant: `p ≡ p'` (β-equivalent).
#[derive(Debug, Clone)]
pub struct Abstraction {
    /// C = λx. body[s:=x] — the reusable context.
    pub concept: Rc<Term>,
    /// p' = λc₁…λcₙ. C s — the original program rewritten to use C.
    pub rewritten_program: Rc<Term>,
    /// s — the extracted subterm.
    pub extracted_subterm: Rc<Term>,
}

/// Standard de Bruijn shift: add `d` to every free variable index ≥ `cutoff`.
pub fn shift(t: &Rc<Term>, d: i32, cutoff: u32) -> Rc<Term> {
    match t.as_ref() {
        Term::Var(i) => {
            if *i >= cutoff {
                let shifted = if d >= 0 {
                    i.checked_add(d as u32)
                } else {
                    i.checked_sub(d.unsigned_abs())
                }
                .expect("invalid de Bruijn shift");
                term::var(shifted)
            } else {
                t.clone()
            }
        }
        // `Free` identifies a bank-level context entry, not a lambda binder.
        Term::Free(_) => t.clone(),
        Term::Lam(b) => term::lam(shift(b, d, cutoff + 1)),
        Term::App(f, a) => term::app(shift(f, d, cutoff), shift(a, d, cutoff)),
        // A primitive is an opaque syntax atom. Its embedded implementation is
        // closed and is not part of the program representation being factored.
        Term::Prim(_) => t.clone(),
    }
}

/// Replace every occurrence of `s` in `t` with `r` (structural equality).
pub fn replace_subterm(t: &Rc<Term>, s: &Rc<Term>, r: &Rc<Term>) -> Rc<Term> {
    if t == s {
        return r.clone();
    }
    match t.as_ref() {
        Term::Var(_) | Term::Free(_) => t.clone(),
        Term::Lam(b) => term::lam(replace_subterm(b, s, r)),
        Term::App(f, a) => term::app(replace_subterm(f, s, r), replace_subterm(a, s, r)),
        Term::Prim(_) => t.clone(),
    }
}

/// Replace occurrences of `s` while accounting for binders crossed during the
/// traversal. A term seen beneath `depth` additional lambdas is compared with
/// `shift(s, depth, 0)`, and its replacement is shifted by the same amount.
fn replace_subterm_scoped(t: &Rc<Term>, s: &Rc<Term>, r: &Rc<Term>, depth: u32) -> Rc<Term> {
    if *t == shift(s, depth as i32, 0) {
        return shift(r, depth as i32, 0);
    }
    match t.as_ref() {
        Term::Var(_) | Term::Free(_) => t.clone(),
        Term::Lam(b) => term::lam(replace_subterm_scoped(b, s, r, depth + 1)),
        Term::App(f, a) => term::app(
            replace_subterm_scoped(f, s, r, depth),
            replace_subterm_scoped(a, s, r, depth),
        ),
        Term::Prim(_) => t.clone(),
    }
}

/// Count occurrences of `s` in `t` (structural equality).
pub fn count_occurrences(t: &Rc<Term>, s: &Rc<Term>) -> u32 {
    let mut n = 0;
    count_go(t, s, &mut n);
    n
}

fn count_go(t: &Rc<Term>, s: &Rc<Term>, n: &mut u32) {
    if t == s {
        *n += 1;
    }
    match t.as_ref() {
        Term::Var(_) | Term::Free(_) => {}
        Term::Lam(b) => count_go(b, s, n),
        Term::App(f, a) => {
            count_go(f, s, n);
            count_go(a, s, n);
        }
        Term::Prim(_) => {}
    }
}

/// Count occurrences of `s` modulo the de Bruijn shifts introduced by binders
/// crossed while walking the surrounding context.
fn count_occurrences_scoped(t: &Rc<Term>, s: &Rc<Term>, depth: u32) -> u32 {
    let mut n = u32::from(*t == shift(s, depth as i32, 0));
    match t.as_ref() {
        Term::Var(_) | Term::Free(_) => {}
        Term::Lam(b) => n += count_occurrences_scoped(b, s, depth + 1),
        Term::App(f, a) => {
            n += count_occurrences_scoped(f, s, depth);
            n += count_occurrences_scoped(a, s, depth);
        }
        Term::Prim(_) => {}
    }
    n
}

/// Enumerate all subterms of `t` (deduplicated by structural equality).
pub fn subterms(t: &Rc<Term>) -> Vec<Rc<Term>> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    subterms_go(t, &mut out, &mut seen);
    out
}

fn subterms_go(t: &Rc<Term>, out: &mut Vec<Rc<Term>>, seen: &mut HashSet<Rc<Term>>) {
    if seen.insert(t.clone()) {
        out.push(t.clone());
    }
    match t.as_ref() {
        Term::Var(_) | Term::Free(_) => {}
        Term::Lam(b) => subterms_go(b, out, seen),
        Term::App(f, a) => {
            subterms_go(f, out, seen);
            subterms_go(a, out, seen);
        }
        Term::Prim(_) => {}
    }
}

/// Is `t` closed (no free variables)?
pub fn is_closed(t: &Rc<Term>) -> bool {
    closed_go(t, 0)
}

fn closed_go(t: &Rc<Term>, depth: u32) -> bool {
    match t.as_ref() {
        Term::Var(i) => *i < depth,
        Term::Free(_) => false,
        Term::Lam(b) => closed_go(b, depth + 1),
        Term::App(f, a) => closed_go(f, depth) && closed_go(a, depth),
        Term::Prim(_) => true,
    }
}

/// Context abstraction: factor a repeated subterm `s` (≥2 occurrences in the body)
/// out of `p = λc₁…λcₙ. body` into a hole. Returns `C = λx. body[s:=x]` and
/// `p' = λc₁…λcₙ. C s`. `None` if `s` doesn't occur ≥2 times in the body.
///
/// The construction operates on the body (NOT the whole term — the naive "add an
/// outermost binder" yields `λx.λc. …`, binder in the wrong place) and shifts de
/// Bruijn indices: `body' = shift(body, 1, 0)`, `s' = shift(s, 1, 0)`,
/// `body'' = body'[s' := Var(0)]`, `C = λx. body''`.
pub fn abstract_subterm(p: &Rc<Term>, s: &Rc<Term>) -> Option<Abstraction> {
    // Strip the outer λ-binders of p, collecting the body.
    let mut n_binders = 0u32;
    let mut body = p.clone();
    while let Term::Lam(b) = body.as_ref() {
        n_binders += 1;
        body = b.clone();
    }
    if count_occurrences_scoped(&body, s, 0) < 2 {
        return None;
    }
    // C = λx. body[s:=x], with proper shifting.
    let body_shifted = shift(&body, 1, 0);
    let s_shifted = shift(s, 1, 0);
    let body_abstracted = replace_subterm_scoped(&body_shifted, &s_shifted, &term::var(0), 0);
    let concept = term::lam(body_abstracted);
    // p' = λc₁…λcₙ. C s
    let rewritten = term::app(concept.clone(), s.clone());
    let rewritten_program = (0..n_binders).fold(rewritten, |acc, _| term::lam(acc));
    Some(Abstraction {
        concept,
        rewritten_program,
        extracted_subterm: s.clone(),
    })
}

/// Enumerate all context abstractions of `p` over its repeated body subterms.
/// Each candidate's `concept` is closed (the semantics/interface check) — the
/// caller verifies factorization (`p' ≡ p`) and counterfactual worth separately.
pub fn enumerate_abstractions(p: &Rc<Term>) -> Vec<Abstraction> {
    // Strip binders to get the body.
    let mut body = p.clone();
    while let Term::Lam(b) = body.as_ref() {
        body = b.clone();
    }
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for s in subterms(&body) {
        if let Some(a) = abstract_subterm(p, &s) {
            if is_closed(&a.concept) && seen.insert(a.concept.clone()) {
                out.push(a);
            }
        }
    }
    out
}

/// Composition as a meta-transformation on syntax: `f, g ↦ λx. f (g x)`.
/// This is NOT an object-level concept — it is part of the meta-search machinery.
pub fn compose(f: &Rc<Term>, g: &Rc<Term>) -> Rc<Term> {
    term::lam(term::app(f.clone(), term::app(g.clone(), term::var(0))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abstracts_same_outer_variable_beneath_a_binder() {
        // λa. pair a (λb. b a): the two occurrences of `a` are Var(0) and
        // Var(1), respectively, but denote the same expression.
        let pair = term::lam(term::lam(term::var(1)));
        let pair_prim = Rc::new(Term::Prim(pair));
        let body = term::app(
            term::app(pair_prim.clone(), term::var(0)),
            term::lam(term::app(term::var(0), term::var(1))),
        );
        let program = term::lam(body);

        let abstraction = abstract_subterm(&program, &term::var(0))
            .expect("binder-shifted occurrences should form one context");

        assert!(is_closed(&abstraction.concept));
        let expected = term::lam(term::app(
            term::app(pair_prim, term::var(0)),
            term::lam(term::app(term::var(0), term::var(1))),
        ));
        assert_eq!(abstraction.concept, expected);
    }

    #[test]
    fn shift_supports_negative_offsets_and_keeps_primitives_opaque() {
        assert_eq!(shift(&term::var(2), -1, 0), term::var(1));

        let implementation = term::lam(term::var(0));
        let primitive = Rc::new(Term::Prim(implementation));
        assert_eq!(shift(&primitive, 3, 0), primitive);
        assert_eq!(subterms(&primitive), vec![primitive]);
    }
}
