//! Generic recurrence induction from finite program unrollings.
//!
//! This module does not contain `fold`, `map`, `reverse`, or a catalog of
//! recursion-shaped target programs.  It compares a depth-indexed family of
//! symbolic computations and asks whether every transition has the same form
//!
//! ```text
//! q[n + 1] = C(head[n + 1], shift(q[n]))
//! ```
//!
//! where `C` is an arbitrary two-hole syntax context.  If so, the residual at
//! depth one determines the base case and the result is an executable law
//! `R([]) = base; R(h::t) = C(h, R(t))`.

use crate::{
    nbe,
    term::{self, Term},
};
use std::rc::Rc;

/// Input atoms in an observed unrolling are `Free(0)..Free(depth - 1)`.
/// Higher free IDs may describe a shared surrounding observation context.
const HEAD_HOLE: u32 = u32::MAX;
const RECUR_HOLE: u32 = u32::MAX - 1;
const INPUT_LIST_HOLE: u32 = u32::MAX - 2;
const OBSERVATION_FREE_BASE: u32 = 1 << 29;

/// One normalized, symbolically opened finite program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unrolling {
    pub depth: usize,
    pub body: Rc<Term>,
}

/// Execute a discovered `depth`-ary program on fresh symbolic inputs, normalize
/// it, and open the result's leading lambdas onto stable observation constants.
/// This removes accidental beta-redex spelling differences between independently
/// discovered programs before recurrence induction.
pub fn observe_program(
    program: &Rc<Term>,
    depth: usize,
    fuel: i64,
) -> Option<(Unrolling, Vec<u32>)> {
    let applied = (0..depth).fold(program.clone(), |p, i| {
        term::app(p, Rc::new(Term::Free(i as u32)))
    });
    let empty: nbe::Env = Rc::new(Vec::new());
    let mut budget = nbe::Fuel(fuel);
    let normalized = nbe::normalize(&empty, &applied, &mut budget).ok()?;

    let mut output_arity = 0usize;
    let mut cursor = normalized.as_ref();
    while let Term::Lam(body) = cursor {
        output_arity += 1;
        cursor = body.as_ref();
    }
    let output_ids: Vec<u32> = (0..output_arity)
        .map(|i| OBSERVATION_FREE_BASE + i as u32)
        .collect();
    let opened = output_ids
        .iter()
        .fold(normalized, |p, id| term::app(p, Rc::new(Term::Free(*id))));
    let mut budget = nbe::Fuel(fuel);
    let body = nbe::normalize(&empty, &opened, &mut budget).ok()?;
    Some((Unrolling { depth, body }, output_ids))
}

/// Normalize a closed discovered program after replacing the instance atoms it
/// used with symbolic inputs.  This is useful when raw search solved a closed
/// synthesis instance: the data atoms are task inputs, not ontology concepts,
/// and symbolization reveals the reusable computation they participate in.
pub fn observe_instantiated_program(
    program: &Rc<Term>,
    head_atoms: &[Rc<Term>],
    external_atoms: &[Rc<Term>],
    fuel: i64,
) -> Option<(Unrolling, Vec<u32>)> {
    let mut replacements: Vec<(&Rc<Term>, u32)> = head_atoms
        .iter()
        .enumerate()
        .map(|(i, body)| (body, i as u32))
        .collect();
    let external_ids: Vec<u32> = (0..external_atoms.len())
        .map(|i| OBSERVATION_FREE_BASE + i as u32)
        .collect();
    replacements.extend(external_atoms.iter().zip(external_ids.iter().copied()));
    let symbolic = symbolize_primitives(program, &replacements);
    let empty: nbe::Env = Rc::new(Vec::new());
    let mut budget = nbe::Fuel(fuel);
    let body = nbe::normalize(&empty, &symbolic, &mut budget).ok()?;
    Some((
        Unrolling {
            depth: head_atoms.len(),
            body,
        },
        external_ids,
    ))
}

fn symbolize_primitives(t: &Rc<Term>, replacements: &[(&Rc<Term>, u32)]) -> Rc<Term> {
    match t.as_ref() {
        Term::Prim(body) => replacements
            .iter()
            .find_map(|(candidate, id)| (*candidate == body).then(|| Rc::new(Term::Free(*id))))
            .unwrap_or_else(|| t.clone()),
        Term::Var(_) | Term::Free(_) => t.clone(),
        Term::Lam(b) => term::lam(symbolize_primitives(b, replacements)),
        Term::App(f, a) => term::app(
            symbolize_primitives(f, replacements),
            symbolize_primitives(a, replacements),
        ),
    }
}

/// An induced first-order structural recurrence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurrenceLaw {
    /// The zero-depth residual.
    pub base: Rc<Term>,
    /// The shared syntax context, represented with opaque head/recursive holes.
    step_context: Rc<Term>,
}

impl RecurrenceLaw {
    /// Materialize `C(head, recursive_result)`.
    pub fn step(&self, head: Rc<Term>, recursive_result: Rc<Term>) -> Rc<Term> {
        instantiate_holes(&self.step_context, &head, &recursive_result)
    }

    /// Reconstruct the finite unrolling at `depth` over symbolic inputs.
    pub fn unroll(&self, depth: usize) -> Rc<Term> {
        (0..depth).rev().fold(self.base.clone(), |tail, i| {
            self.step(Rc::new(Term::Free(i as u32)), tail)
        })
    }

    pub fn uses_head(&self) -> bool {
        contains_free(&self.step_context, HEAD_HOLE)
    }

    pub fn uses_recursive_result(&self) -> bool {
        contains_free(&self.step_context, RECUR_HOLE)
    }

    /// Compile the inferred equation for Church-encoded input lists.
    ///
    /// This is a generic equation compiler, not a proposal schema: the base and
    /// step are wholly induced from the observed programs.  Any non-input
    /// `Free` atoms in the law become parameters of the resulting executable,
    /// ordered by ID.  For example an observed constructor algebra containing
    /// `Free(1_000_000)` and `Free(1_000_001)` compiles to
    /// `λxs.λa.λb. xs (λh.λr.C(h,r)) base`.
    pub fn compile_church(&self) -> Rc<Term> {
        let head = Rc::new(Term::Free(HEAD_HOLE));
        let recur = Rc::new(Term::Free(RECUR_HOLE));
        let step_body = instantiate_holes(&self.step_context, &head, &recur);
        let step = close_over(step_body, &[HEAD_HOLE, RECUR_HOLE]);
        let core = term::app(
            term::app(Rc::new(Term::Free(INPUT_LIST_HOLE)), step),
            self.base.clone(),
        );

        let mut parameters = Vec::new();
        collect_external_frees(&core, &mut parameters);
        parameters.sort_unstable();
        parameters.dedup();
        parameters.retain(|id| *id != INPUT_LIST_HOLE && *id != HEAD_HOLE && *id != RECUR_HOLE);

        let mut binders = vec![INPUT_LIST_HOLE];
        binders.extend(parameters);
        close_over(core, &binders)
    }

    /// Reify the structural recursion *scheme* exposed by this law.  Unlike the
    /// specialized executable above, the inferred base and context positions
    /// become parameters, yielding a new object-level reasoning primitive.
    /// This is only available after a genuine two-hole law has been induced.
    pub fn compile_church_scheme(&self) -> Rc<Term> {
        assert!(self.uses_head() && self.uses_recursive_result());
        // λstep.λbase.λxs. xs step base
        term::lam(term::lam(term::lam(term::app(
            term::app(term::var(0), term::var(2)),
            term::var(1),
        ))))
    }

    pub fn step_context(&self) -> &Rc<Term> {
        &self.step_context
    }
}

/// Infer one recurrence from consecutive depths 1..=k.
///
/// The check is deliberately strict.  A single invariant context must explain
/// every adjacent pair, and it must consume both the new head and the previous
/// result.  This excludes constant laws, head/tail projections, depth lookup
/// tables, and contexts that only coincide observationally on a tiny sample.
pub fn infer(unrollings: &[Unrolling]) -> Option<RecurrenceLaw> {
    if unrollings.len() < 2 {
        return None;
    }
    for (i, u) in unrollings.iter().enumerate() {
        if u.depth != i + 1 {
            return None;
        }
    }

    let mut invariant: Option<Rc<Term>> = None;
    for pair in unrollings.windows(2) {
        let prev = &pair[0];
        let next = &pair[1];
        let shifted_prev = shift_input_ids(&prev.body, prev.depth, 1);
        if count_occurrences(&next.body, &shifted_prev) != 1 {
            return None;
        }
        let with_recur = replace_exact(&next.body, &shifted_prev, &Rc::new(Term::Free(RECUR_HOLE)));
        let context = replace_free(&with_recur, 0, HEAD_HOLE);
        if !contains_free(&context, HEAD_HOLE) || !contains_free(&context, RECUR_HOLE) {
            return None;
        }
        match &invariant {
            Some(expected) if **expected != *context => return None,
            None => invariant = Some(context),
            _ => {}
        }
    }

    let step_context = invariant?;
    let base = match_base(&step_context, &unrollings[0].body)?;
    let law = RecurrenceLaw { base, step_context };
    if unrollings.iter().all(|u| law.unroll(u.depth) == u.body) {
        Some(law)
    } else {
        None
    }
}

fn shift_input_ids(t: &Rc<Term>, depth: usize, amount: u32) -> Rc<Term> {
    match t.as_ref() {
        Term::Free(i) if (*i as usize) < depth => Rc::new(Term::Free(i + amount)),
        Term::Free(_) | Term::Var(_) | Term::Prim(_) => t.clone(),
        Term::Lam(b) => term::lam(shift_input_ids(b, depth, amount)),
        Term::App(f, a) => term::app(
            shift_input_ids(f, depth, amount),
            shift_input_ids(a, depth, amount),
        ),
    }
}

fn replace_free(t: &Rc<Term>, from: u32, to: u32) -> Rc<Term> {
    match t.as_ref() {
        Term::Free(i) if *i == from => Rc::new(Term::Free(to)),
        Term::Free(_) | Term::Var(_) | Term::Prim(_) => t.clone(),
        Term::Lam(b) => term::lam(replace_free(b, from, to)),
        Term::App(f, a) => term::app(replace_free(f, from, to), replace_free(a, from, to)),
    }
}

fn replace_exact(t: &Rc<Term>, needle: &Rc<Term>, replacement: &Rc<Term>) -> Rc<Term> {
    if t == needle {
        return replacement.clone();
    }
    match t.as_ref() {
        Term::Var(_) | Term::Free(_) | Term::Prim(_) => t.clone(),
        Term::Lam(b) => term::lam(replace_exact(b, needle, replacement)),
        Term::App(f, a) => term::app(
            replace_exact(f, needle, replacement),
            replace_exact(a, needle, replacement),
        ),
    }
}

fn count_occurrences(t: &Rc<Term>, needle: &Rc<Term>) -> usize {
    let here = usize::from(t == needle);
    here + match t.as_ref() {
        Term::Var(_) | Term::Free(_) | Term::Prim(_) => 0,
        Term::Lam(b) => count_occurrences(b, needle),
        Term::App(f, a) => count_occurrences(f, needle) + count_occurrences(a, needle),
    }
}

fn contains_free(t: &Rc<Term>, id: u32) -> bool {
    match t.as_ref() {
        Term::Free(i) => *i == id,
        Term::Var(_) | Term::Prim(_) => false,
        Term::Lam(b) => contains_free(b, id),
        Term::App(f, a) => contains_free(f, id) || contains_free(a, id),
    }
}

fn instantiate_holes(t: &Rc<Term>, head: &Rc<Term>, recur: &Rc<Term>) -> Rc<Term> {
    match t.as_ref() {
        Term::Free(HEAD_HOLE) => head.clone(),
        Term::Free(RECUR_HOLE) => recur.clone(),
        Term::Var(_) | Term::Free(_) | Term::Prim(_) => t.clone(),
        Term::Lam(b) => term::lam(instantiate_holes(b, head, recur)),
        Term::App(f, a) => term::app(
            instantiate_holes(f, head, recur),
            instantiate_holes(a, head, recur),
        ),
    }
}

fn match_base(pattern: &Rc<Term>, target: &Rc<Term>) -> Option<Rc<Term>> {
    fn go(pattern: &Rc<Term>, target: &Rc<Term>, found: &mut Option<Rc<Term>>) -> bool {
        match pattern.as_ref() {
            Term::Free(HEAD_HOLE) => target.as_ref() == &Term::Free(0),
            Term::Free(RECUR_HOLE) => match found {
                Some(x) => x == target,
                None => {
                    *found = Some(target.clone());
                    true
                }
            },
            Term::Var(a) => matches!(target.as_ref(), Term::Var(b) if a == b),
            Term::Free(a) => matches!(target.as_ref(), Term::Free(b) if a == b),
            Term::Prim(a) => matches!(target.as_ref(), Term::Prim(b) if a == b),
            Term::Lam(a) => matches!(target.as_ref(), Term::Lam(b) if go(a, b, found)),
            Term::App(af, aa) => matches!(target.as_ref(), Term::App(bf, ba)
                if go(af, bf, found) && go(aa, ba, found)),
        }
    }

    let mut found = None;
    if go(pattern, target, &mut found) {
        found
    } else {
        None
    }
}

fn collect_external_frees(t: &Rc<Term>, out: &mut Vec<u32>) {
    match t.as_ref() {
        Term::Free(i) => out.push(*i),
        Term::Var(_) | Term::Prim(_) => {}
        Term::Lam(b) => collect_external_frees(b, out),
        Term::App(f, a) => {
            collect_external_frees(f, out);
            collect_external_frees(a, out);
        }
    }
}

/// Close a term over free IDs in the requested outer-to-inner binder order.
fn close_over(mut t: Rc<Term>, ids: &[u32]) -> Rc<Term> {
    for id in ids.iter().rev() {
        t = abstract_free(&t, *id, 0);
        t = term::lam(t);
    }
    t
}

fn abstract_free(t: &Rc<Term>, id: u32, depth: u32) -> Rc<Term> {
    match t.as_ref() {
        Term::Free(i) if *i == id => term::var(depth),
        Term::Free(_) | Term::Prim(_) => t.clone(),
        Term::Var(i) => term::var(if *i >= depth { i + 1 } else { *i }),
        Term::Lam(b) => term::lam(abstract_free(b, id, depth + 1)),
        Term::App(f, a) => term::app(abstract_free(f, id, depth), abstract_free(a, id, depth)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn free(i: u32) -> Rc<Term> {
        Rc::new(Term::Free(i))
    }

    fn f(h: Rc<Term>, t: Rc<Term>) -> Rc<Term> {
        term::app(term::app(free(100), h), t)
    }

    fn good_family() -> Vec<Unrolling> {
        let z = free(101);
        vec![
            Unrolling {
                depth: 1,
                body: f(free(0), z.clone()),
            },
            Unrolling {
                depth: 2,
                body: f(free(0), f(free(1), z.clone())),
            },
            Unrolling {
                depth: 3,
                body: f(free(0), f(free(1), f(free(2), z))),
            },
        ]
    }

    #[test]
    fn induces_and_exactly_reconstructs_a_recurrence() {
        let family = good_family();
        let law = infer(&family).expect("a single invariant context exists");
        assert_eq!(law.base, free(101));
        assert!(law.uses_head());
        assert!(law.uses_recursive_result());
        assert_eq!(
            law.unroll(7),
            (0..7).rev().fold(free(101), |t, i| f(free(i), t))
        );
    }

    #[test]
    fn rejects_constant_headless_tailless_and_depth_specific_cheats() {
        let z = free(101);
        let constants = vec![
            Unrolling {
                depth: 1,
                body: z.clone(),
            },
            Unrolling {
                depth: 2,
                body: z.clone(),
            },
        ];
        assert!(infer(&constants).is_none());

        let headless = vec![
            Unrolling {
                depth: 1,
                body: f(free(9), z.clone()),
            },
            Unrolling {
                depth: 2,
                body: f(free(9), f(free(9), z.clone())),
            },
        ];
        assert!(infer(&headless).is_none());

        let tailless = vec![
            Unrolling {
                depth: 1,
                body: free(0),
            },
            Unrolling {
                depth: 2,
                body: free(0),
            },
        ];
        assert!(infer(&tailless).is_none());

        let mut depth_specific = good_family();
        depth_specific[2].body = f(free(0), f(free(1), f(free(2), f(free(9), z))));
        assert!(infer(&depth_specific).is_none());

        // A beta-redex can be observationally equal on every tiny example but
        // is not evidence for one repeated syntactic transition law.
        let mut accidental_equivalence = good_family();
        let q2_shifted = shift_input_ids(&accidental_equivalence[1].body, 2, 1);
        let disguised = term::app(term::lam(term::var(0)), q2_shifted);
        accidental_equivalence[2].body = f(free(0), disguised);
        assert!(infer(&accidental_equivalence).is_none());
    }

    #[test]
    fn compiled_equation_is_closed_and_has_no_named_recursive_primitive() {
        let law = infer(&good_family()).unwrap();
        let executable = law.compile_church();
        assert!(crate::transform::is_closed(&executable));
        assert!(!matches!(executable.as_ref(), Term::Prim(_)));
    }
}
