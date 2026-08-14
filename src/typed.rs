//! Small typed beta-normal proposal enumeration plus explicit compositional search.
//!
//! Types prune meaningless applications; they do not encode target operations.
//! The same generator searches for any requested interface from the currently
//! acquired atoms.  In particular it contains no productions named map,
//! reverse, append, fold, or reduce. System F instantiation and the optional
//! Church-fold decomposition are represented explicitly in types/search APIs.

use crate::term::{self, Term};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Atom(u32),
    Arrow(Box<Type>, Box<Type>),
    /// Reference to a (possibly recursive) type definition in the `defs` map.
    /// Used to give Church numerals their true type `num = (num→num)→num→num`,
    /// so the enumerator can apply a numeral as a function (which the opaque
    /// `Atom` view cannot). `Rec(i)` expands to `defs[i]` during enumeration.
    Rec(u32),
    /// Type variable (de Bruijn index, bound by the nearest enclosing `Forall`).
    Var(u32),
    /// Universal quantification `∀α. body` where `α = Var(0)` in `body`.
    /// This is the System F step: a polymorphic Church numeral `num = ∀α. (α→α)→(α→α)`
    /// can be applied to a `num→boo` function (heterogeneous iteration), which the
    /// simply-typed `Rec` view cannot — that is what unblocks iszero/pred/eq/mod26.
    Forall(Box<Type>),
}

impl Type {
    pub fn arrow(a: Type, b: Type) -> Type {
        Type::Arrow(Box::new(a), Box::new(b))
    }

    pub fn forall(body: Type) -> Type {
        Type::Forall(Box::new(body))
    }
}

#[derive(Debug, Clone)]
pub struct Atom {
    pub body: Rc<Term>,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct Found {
    pub term: Rc<Term>,
    pub size: u32,
    pub generated: u64,
}

#[derive(Debug, Clone)]
pub struct Enumeration {
    pub terms: Vec<Rc<Term>>,
    /// Terms materialized across all memoized type/context/size cells.
    pub generated: u64,
    pub max_size: u32,
    pub per_cell_cap: usize,
    /// At least one otherwise-new inhabitant may have been excluded by a cap.
    pub truncated: bool,
}

#[derive(Debug, Clone)]
pub struct ChurchFoldFound {
    pub term: Rc<Term>,
    pub size: u32,
    /// Terms generated while enumerating the three component spaces.
    pub generated: u64,
    /// Fully assembled candidates checked against the end-to-end gate.
    pub assembled: u64,
    /// Whether any component cell hit its cap.
    pub truncated: bool,
}

/// Goal-directed search for a Church-iterator program. The search receives
/// only the accumulator decomposition (projection, transition builder, seed
/// builder), enumerates all three closed component spaces independently, and
/// checks their assemblies against the original end-to-end gate. It receives
/// no behavioral oracle for an individual component.
pub fn find_church_fold_with_defs(
    project_type: &Type,
    step_builder_type: &Type,
    seed_builder_type: &Type,
    component_atoms: [&[Atom]; 3],
    defs: &HashMap<u32, Type>,
    component_max_sizes: [u32; 3],
    per_cell_cap: usize,
    mut accepts: impl FnMut(&Rc<Term>) -> bool,
) -> Option<ChurchFoldFound> {
    let projects = enumerate_closed_with_defs(
        project_type,
        component_atoms[0],
        defs,
        component_max_sizes[0],
        per_cell_cap,
    );
    let steps = enumerate_closed_with_defs(
        step_builder_type,
        component_atoms[1],
        defs,
        component_max_sizes[1],
        per_cell_cap,
    );
    let seeds = enumerate_closed_with_defs(
        seed_builder_type,
        component_atoms[2],
        defs,
        component_max_sizes[2],
        per_cell_cap,
    );
    let generated = projects.generated + steps.generated + seeds.generated;
    let truncated = projects.truncated || steps.truncated || seeds.truncated;

    let mut candidates = Vec::new();
    for (project_index, project) in projects.terms.iter().enumerate() {
        for (step_index, step) in steps.terms.iter().enumerate() {
            for (seed_index, seed) in seeds.terms.iter().enumerate() {
                if let Some(term) = church_fold_candidate(project.clone(), step.clone(), seed.clone()) {
                    candidates.push((term.size(), project_index, step_index, seed_index, term));
                }
            }
        }
    }
    candidates.sort_by_key(|(size, project, step, seed, _)| (*size, *project, *step, *seed));

    for (index, (size, _, _, _, term)) in candidates.into_iter().enumerate() {
        if accepts(&term) {
            return Some(ChurchFoldFound {
                term,
                size,
                generated,
                assembled: index as u64 + 1,
                truncated,
            });
        }
    }
    None
}

/// Assemble a three-argument Church fold from independently synthesized,
/// closed components:
///
/// `\u{3bb}n.\u{3bb}f.\u{3bb}x. project (n (step_builder f) (seed_builder x))`.
///
/// The two builder applications are beta-contracted while `Prim` nodes remain
/// opaque.  This is important for compositional search accounting: acquired
/// atoms keep cost one, but the helper lambdas introduced solely to make the
/// subgoals closed disappear from the assembled beta-normal candidate.
pub fn church_fold_candidate(
    project: Rc<Term>,
    step_builder: Rc<Term>,
    seed_builder: Rc<Term>,
) -> Option<Rc<Term>> {
    let step = beta_apply_preserving_prims(&step_builder, term::var(1))?;
    let seed = beta_apply_preserving_prims(&seed_builder, term::var(0))?;
    let folded = term::app(term::app(term::var(2), step), seed);
    let projected = beta_apply_preserving_prims(&project, folded.clone())
        .unwrap_or_else(|| term::app(project, folded));
    Some(term::lam(term::lam(term::lam(projected))))
}

/// Contract one beta redex without expanding acquired primitives.
fn beta_apply_preserving_prims(function: &Rc<Term>, argument: Rc<Term>) -> Option<Rc<Term>> {
    let Term::Lam(body) = function.as_ref() else {
        return None;
    };
    Some(substitute_outer_binder(body, &argument, 0))
}

fn substitute_outer_binder(term: &Rc<Term>, argument: &Rc<Term>, depth: u32) -> Rc<Term> {
    match term.as_ref() {
        Term::Var(i) if *i == depth => shift_free_indices(argument, depth, 0),
        Term::Var(i) if *i > depth => term::var(i - 1),
        Term::Var(i) => term::var(*i),
        Term::Free(i) => Rc::new(Term::Free(*i)),
        Term::Prim(body) => Rc::new(Term::Prim(body.clone())),
        Term::Lam(body) => term::lam(substitute_outer_binder(body, argument, depth + 1)),
        Term::App(function, arg) => term::app(
            substitute_outer_binder(function, argument, depth),
            substitute_outer_binder(arg, argument, depth),
        ),
    }
}

fn shift_free_indices(term: &Rc<Term>, amount: u32, cutoff: u32) -> Rc<Term> {
    match term.as_ref() {
        Term::Var(i) if *i >= cutoff => term::var(i + amount),
        Term::Var(i) => term::var(*i),
        Term::Free(i) => Rc::new(Term::Free(*i)),
        Term::Prim(body) => Rc::new(Term::Prim(body.clone())),
        Term::Lam(body) => term::lam(shift_free_indices(body, amount, cutoff + 1)),
        Term::App(function, arg) => term::app(
            shift_free_indices(function, amount, cutoff),
            shift_free_indices(arg, amount, cutoff),
        ),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Key {
    target: Type,
    context: Vec<Type>,
    size: u32,
}

struct Enumerator<'a> {
    atoms: &'a [Atom],
    defs: &'a HashMap<u32, Type>,
    memo: HashMap<Key, Vec<Rc<Term>>>,
    generated: u64,
    per_cell_cap: usize,
    truncated: bool,
    rec_calls: u64,
}

/// Search closed beta-normal terms by increasing syntax size.
pub fn find_closed(
    target: &Type,
    atoms: &[Atom],
    max_size: u32,
    per_cell_cap: usize,
    accepts: impl FnMut(&Rc<Term>) -> bool,
) -> Option<Found> {
    find_closed_with_defs(target, atoms, &HashMap::new(), max_size, per_cell_cap, accepts)
}

/// `find_closed` with a map of recursive type definitions (see [`Type::Rec`]).
pub fn find_closed_with_defs(
    target: &Type,
    atoms: &[Atom],
    defs: &HashMap<u32, Type>,
    max_size: u32,
    per_cell_cap: usize,
    mut accepts: impl FnMut(&Rc<Term>) -> bool,
) -> Option<Found> {
    let mut e = Enumerator {
        atoms,
        defs,
        memo: HashMap::new(),
        generated: 0,
        per_cell_cap,
        truncated: false,
        rec_calls: 0,
    };
    for size in 1..=max_size {
        for candidate in e.terms(target, &[], size) {
            if accepts(&candidate) {
                return Some(Found {
                    term: candidate,
                    size,
                    generated: e.generated,
                });
            }
        }
    }
    None
}

/// Enumerate every distinct closed beta-normal term within the declared size
/// and per-cell boundaries. This is used when a verifier must quantify over
/// all bounded inhabitants (for example, observational mediator uniqueness)
/// rather than stop at the first accepted program.
pub fn enumerate_closed(
    target: &Type,
    atoms: &[Atom],
    max_size: u32,
    per_cell_cap: usize,
) -> Enumeration {
    enumerate_closed_with_defs(target, atoms, &HashMap::new(), max_size, per_cell_cap)
}

/// `enumerate_closed` with a map of recursive type definitions (see [`Type::Rec`]).
pub fn enumerate_closed_with_defs(
    target: &Type,
    atoms: &[Atom],
    defs: &HashMap<u32, Type>,
    max_size: u32,
    per_cell_cap: usize,
) -> Enumeration {
    let mut e = Enumerator {
        atoms,
        defs,
        memo: HashMap::new(),
        generated: 0,
        per_cell_cap,
        truncated: false,
        rec_calls: 0,
    };
    let mut terms = Vec::new();
    let mut seen = HashSet::new();
    for size in 1..=max_size {
        for candidate in e.terms(target, &[], size) {
            if seen.insert(candidate.clone()) {
                terms.push(candidate);
            }
        }
    }
    Enumeration {
        terms,
        generated: e.generated,
        max_size,
        per_cell_cap,
        truncated: e.truncated,
    }
}

impl Enumerator<'_> {
    fn terms(&mut self, target: &Type, context: &[Type], size: u32) -> Vec<Rc<Term>> {
        let key = Key {
            target: target.clone(),
            context: context.to_vec(),
            size,
        };
        if let Some(found) = self.memo.get(&key) {
            return found.clone();
        }
        // DEBUG: detect runaway recursion on recursive types.
        if let Type::Rec(i) = target {
            self.rec_calls += 1;
            if self.rec_calls > 5_000_000 {
                panic!("typed: runaway Rec({i}) expansion at size {size}, context len {}", context.len());
            }
        }

        let mut out = Vec::new();
        let mut seen = HashSet::new();
        // Expand a recursive type reference to its definition before generating.
        let expanded: Option<Type> = match target {
            Type::Rec(i) => self.defs.get(i).cloned(),
            _ => None,
        };
        let effective = expanded.as_ref().unwrap_or(target);
        if let Type::Arrow(arg, result) = effective {
            if size >= 2 {
                let mut inner = context.to_vec();
                inner.push((**arg).clone());
                for body in self.terms(result, &inner, size - 1) {
                    if !push_unique(&mut out, &mut seen, term::lam(body), self.per_cell_cap) {
                        self.truncated = true;
                    }
                }
            }
        }

        // Neutral forms may themselves have function type (e.g. an acquired
        // higher-order primitive passed as an argument) and need not be
        // eta-expanded.
        let mut heads: Vec<(Rc<Term>, Type)> = self
            .atoms
            .iter()
            .map(|a| (Rc::new(Term::Prim(a.body.clone())), a.ty.clone()))
            .collect();
        heads.extend(
            context
                .iter()
                .rev()
                .enumerate()
                .map(|(i, ty)| (term::var(i as u32), ty.clone())),
        );

        for (head, head_ty) in heads {
            for args in arguments_to(head_ty, target, self.defs) {
                let overhead = 1 + args.len() as u32;
                if size < overhead + args.len() as u32 {
                    continue;
                }
                let argument_budget = size - overhead;
                for sizes in positive_compositions(argument_budget, args.len()) {
                    let choices: Vec<Vec<Rc<Term>>> = args
                        .iter()
                        .zip(sizes)
                        .map(|(ty, n)| self.terms(ty, context, n))
                        .collect();
                    if choices.iter().any(Vec::is_empty) {
                        continue;
                    }
                    let mut products = vec![Vec::<Rc<Term>>::new()];
                    for choice in choices {
                        let mut next = Vec::new();
                        for prefix in &products {
                            for item in &choice {
                                let mut p = prefix.clone();
                                p.push(item.clone());
                                next.push(p);
                                if next.len() >= self.per_cell_cap {
                                    self.truncated = true;
                                    break;
                                }
                            }
                            if next.len() >= self.per_cell_cap {
                                break;
                            }
                        }
                        products = next;
                    }
                    for product in products {
                        let candidate = product.into_iter().fold(head.clone(), term::app);
                        if !push_unique(&mut out, &mut seen, candidate, self.per_cell_cap) {
                            self.truncated = true;
                        }
                        if out.len() >= self.per_cell_cap {
                            break;
                        }
                    }
                }
            }
        }
        self.generated += out.len() as u64;
        self.memo.insert(key, out.clone());
        out
    }
}

/// Reserved index for the meta-variable introduced when instantiating a `Forall`.
const META: u32 = u32::MAX;

/// Replace `Var(var)` with `replacement` throughout `ty`.
fn subst(ty: &Type, var: u32, replacement: &Type) -> Type {
    match ty {
        Type::Var(i) if *i == var => replacement.clone(),
        Type::Arrow(a, b) => Type::arrow(subst(a, var, replacement), subst(b, var, replacement)),
        Type::Forall(b) => Type::forall(subst(b, var, replacement)),
        other => other.clone(),
    }
}

/// Find `σ` such that `ty[Var(META) := σ] == target`. Returns `Some(σ)` or `None`.
/// `ty` may contain the meta-variable `Var(META)` (from a `Forall` instantiation);
/// concrete types must match `target` exactly. This is the small unification step
/// that lets a polymorphic head (e.g. a Church numeral) be applied at the target type.
fn solve(ty: &Type, target: &Type) -> Option<Type> {
    match ty {
        Type::Var(i) if *i == META => Some(target.clone()),
        Type::Arrow(a, b) => {
            if let Type::Arrow(ta, tb) = target {
                let sa = solve(a, ta)?;
                let sb = solve(b, tb)?;
                if sa == sb {
                    Some(sa)
                } else {
                    None
                }
            } else {
                None
            }
        }
        other => {
            if other == target {
                Some(other.clone())
            } else {
                None
            }
        }
    }
}

/// All ways to apply a head of type `ty` to reach `target`, as argument-type lists.
/// A recursive type (e.g. `Rec(1) = Rec(1)→Rec(1)→Rec(1)`) can be used as-is (0 args)
/// OR applied to 2 args to reach itself again, so there can be several arg-lists.
/// A polymorphic type (`Forall`) is instantiated at the target via `solve`, which is
/// what lets a Church numeral `num = ∀α. (α→α)→(α→α)` be applied to a `num→boo`
/// function (iszero) or a `num→num` function (add). The arg-list length is capped
/// (the size budget bounds feasible applications anyway).
fn arguments_to(mut ty: Type, target: &Type, defs: &HashMap<u32, Type>) -> Vec<Vec<Type>> {
    let mut results = Vec::new();
    let mut seen = HashSet::new();
    let mut args = Vec::new();
    let mut rec_expansions = 0u32;
    loop {
        // Can the current type (possibly with a meta-variable from a Forall) be
        // unified with the target? If so, record the concrete arg-list.
        if let Some(sigma) = solve(&ty, target) {
            let concrete: Vec<Type> = args.iter().map(|a| subst(a, META, &sigma)).collect();
            if seen.insert(concrete.clone()) {
                results.push(concrete);
            }
            // Reached the target via a non-recursive, non-polymorphic type: no
            // further application is possible.
            if !matches!(ty, Type::Rec(_) | Type::Forall(_)) {
                break;
            }
        }
        if args.len() >= 4 {
            break;
        }
        match ty {
            Type::Arrow(a, b) => {
                args.push(*a);
                ty = *b;
            }
            Type::Atom(_) => break,
            Type::Rec(i) => {
                // Expand a recursive reference. Cap the number of expansions: a regular
                // recursive type (e.g. Church numerals) needs only a bounded number of
                // unfoldings to reach any target, and an unreachable target would
                // otherwise loop forever (Rec → Arrow(Arrow(Rec,Rec), Arrow(Rec,Rec)) → Rec).
                rec_expansions += 1;
                if rec_expansions > 100 {
                    break;
                }
                match defs.get(&i) {
                    Some(d) if *d != Type::Rec(i) => ty = d.clone(),
                    _ => break,
                }
            }
            Type::Forall(body) => {
                // Instantiate the bound variable at the meta-variable; the walk
                // then solves for it against the target.
                ty = subst(&body, 0, &Type::Var(META));
            }
            Type::Var(_) => break,
        }
    }
    results
}

fn positive_compositions(total: u32, parts: usize) -> Vec<Vec<u32>> {
    fn go(total: u32, parts: usize, prefix: &mut Vec<u32>, out: &mut Vec<Vec<u32>>) {
        if parts == 0 {
            if total == 0 {
                out.push(prefix.clone());
            }
            return;
        }
        for first in 1..=total.saturating_sub(parts as u32 - 1) {
            prefix.push(first);
            go(total - first, parts - 1, prefix, out);
            prefix.pop();
        }
    }
    let mut out = Vec::new();
    go(total, parts, &mut Vec::new(), &mut out);
    out
}

fn push_unique(
    out: &mut Vec<Rc<Term>>,
    seen: &mut HashSet<Rc<Term>>,
    term: Rc<Term>,
    cap: usize,
) -> bool {
    if seen.contains(&term) {
        true
    } else if out.len() < cap {
        seen.insert(term.clone());
        out.push(term);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumerates_identity_in_beta_normal_form() {
        let a = Type::Atom(0);
        let found = find_closed(&Type::arrow(a.clone(), a), &[], 2, 100, |_| true).unwrap();
        assert_eq!(found.term, term::lam(term::var(0)));
        assert_eq!(found.size, 2);
    }

    #[test]
    fn bounded_enumeration_returns_all_sizes_deterministically() {
        let a = Type::Atom(0);
        let first = enumerate_closed(&Type::arrow(a.clone(), a.clone()), &[], 4, 100);
        let second = enumerate_closed(&Type::arrow(a.clone(), a.clone()), &[], 4, 100);
        assert_eq!(first.terms, second.terms);
        assert_eq!(first.generated, second.generated);
        assert!(!first.truncated);
        assert!(first.terms.contains(&term::lam(term::var(0))));
        assert!(first.terms.iter().all(|term| term.size() <= 4));

        let capped = enumerate_closed(
            &Type::arrow(a.clone(), Type::arrow(a.clone(), a.clone())),
            &[],
            3,
            1,
        );
        let capped_replay = enumerate_closed(
            &Type::arrow(a.clone(), Type::arrow(a.clone(), a)),
            &[],
            3,
            1,
        );
        assert!(capped.truncated);
        assert_eq!(capped.terms, capped_replay.terms);
        assert_eq!(capped.generated, capped_replay.generated);
    }

    #[test]
    fn church_fold_composition_contracts_builders_but_preserves_atoms() {
        let project_body = term::lam(term::var(0));
        let project = Rc::new(Term::Prim(project_body.clone()));
        let step_atom = Rc::new(Term::Prim(term::lam(term::var(0))));
        let seed_atom = Rc::new(Term::Prim(term::lam(term::var(0))));
        let step_builder = term::lam(term::lam(term::app(step_atom.clone(), term::var(0))));
        let seed_builder = term::lam(term::app(seed_atom.clone(), term::var(0)));

        let candidate = church_fold_candidate(project.clone(), step_builder, seed_builder).unwrap();
        assert_eq!(candidate.size(), 15);
        assert_eq!(
            candidate,
            term::lam(term::lam(term::lam(term::app(
                project,
                term::app(
                    term::app(
                        term::var(2),
                        term::lam(term::app(step_atom, term::var(0))),
                    ),
                    term::app(seed_atom, term::var(0)),
                ),
            ))))
        );
    }

    #[test]
    fn goal_directed_church_fold_enumerates_components_before_assembly() {
        let f = Type::Atom(0);
        let x = Type::Atom(1);
        let state = Type::Atom(2);
        let output = Type::Atom(3);
        let project_body = term::lam(term::var(0));
        let step_body = term::lam(term::lam(term::var(0)));
        let seed_body = term::lam(term::var(0));
        let project_atoms = [Atom {
            body: project_body,
            ty: Type::arrow(state.clone(), output.clone()),
        }];
        let step_atoms = [Atom {
            body: step_body,
            ty: Type::arrow(f.clone(), Type::arrow(state.clone(), state.clone())),
        }];
        let seed_atoms = [Atom {
            body: seed_body,
            ty: Type::arrow(x.clone(), state.clone()),
        }];

        let found = find_church_fold_with_defs(
            &Type::arrow(state.clone(), output),
            &Type::arrow(f, Type::arrow(state.clone(), state.clone())),
            &Type::arrow(x, state),
            [&project_atoms, &step_atoms, &seed_atoms],
            &HashMap::new(),
            [4, 7, 4],
            100,
            |_| true,
        )
        .unwrap();

        assert_eq!(found.assembled, 1);
        assert!(!found.truncated);
        assert!(found.generated > 0);
    }
}
