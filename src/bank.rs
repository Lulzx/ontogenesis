//! The bank: bottom-up enumeration of λ-terms with behavioral deduplication.
//!
//! Terms are enumerated by size, per binder-context. A term at context `c`
//! has free variables for the task's `k` arguments plus `c` enclosing λ
//! binders. Its *key* is the vector of its normal forms under each test's
//! environment (arguments bound to the actual test inputs, context binders
//! left as free constants). Two terms with the same key are interchangeable
//! in every context for these tests, so only one representative survives —
//! this is the superposition, implemented as a hash table.
//!
//! Work is shared two ways: bank entries carry their evaluated *values* per
//! test, so building `App(f, a)` costs only the new β-redex chain, and keys
//! are hashed straight out of the evaluator without materializing normal
//! forms.

use crate::nbe::{
    eval, quote_eq, quote_hash, thunk_delayed, thunk_of_val, Abort, Env, Fuel, Head, Thunk, Val,
};
use crate::parse::Task;
use crate::term::{app, lam, var, Term};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::time::Instant;

#[derive(Clone)]
pub struct Options {
    pub max_size: u32,
    pub max_depth: u32,
    pub fuel: i64,
    pub time_budget_secs: f64,
    pub max_level_entries: usize,
    pub max_opaque_entries: usize,
    /// Library: closed seed terms injected at size 1 in every context
    /// (e.g. a Y combinator for recursion; later, mined abstractions).
    pub seeds: Vec<Rc<Term>>,
    /// Quotient-aware concepts: each closed body is injected as a `Prim`
    /// atom (size 1), and any built subterm whose head is that body (its
    /// expanded form) is canonicalized to the `Prim` — so the enumerator
    /// explores P/~L and using a concept costs 1 instead of re-deriving it.
    pub concepts: Vec<Rc<Term>>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            max_size: 14,
            max_depth: 3,
            fuel: 20_000,
            time_budget_secs: 60.0,
            max_level_entries: 200_000,
            max_opaque_entries: 20_000,
            seeds: Vec::new(),
            concepts: Vec::new(),
        }
    }
}

/// Y = λf.(λx.f(x(x)))(λx.f(x(x))) — the classic fixpoint combinator.
/// Under call-by-need evaluation, recursion built with it terminates on
/// concrete data; as a bare value it has no normal form, so it lives in
/// the opaque pool.
pub fn y_combinator() -> Rc<Term> {
    let half = lam(app(var(1), app(var(0), var(0))));
    lam(app(half.clone(), half))
}

#[derive(Default)]
pub struct Stats {
    pub built: u64,
    pub kept: u64,
    pub aborted: u64,
    pub reached_size: u32,
    pub elapsed_secs: f64,
}

pub struct Outcome {
    pub solution: Option<Rc<Term>>,
    pub stats: Stats,
}

#[derive(Clone)]
struct Entry {
    term: Rc<Term>,
    /// Evaluated value per test (whnf-level, lazily deepened by quoting).
    vals: Vec<Rc<Val>>,
}

struct Level {
    /// entries[s] = behaviorally deduped terms of size s at this context.
    entries: Vec<Vec<Entry>>,
    /// opaque[s] = terms whose normal form diverges (e.g. recursive function
    /// values built with Y). They can't be keyed behaviorally, but they are
    /// kept — syntactically deduped, capped — because applying them to
    /// concrete data later can produce normalizing candidates.
    opaque: Vec<Vec<Rc<Term>>>,
    seen: HashSet<u64>,
    seen_syn: HashSet<u64>,
    envs: Vec<Env>, // one environment per test
}

fn syn_hash(t: &Term) -> u64 {
    let mut h = DefaultHasher::new();
    t.hash(&mut h);
    h.finish()
}

/// The quotient map P → P/~L: rewrite any subterm whose head is exactly a
/// concept body (its expanded form) to the `Prim` atom for that concept. Since
/// concept bodies are closed de Bruijn terms, "exactly equals" is structural.
/// `Prim` evaluates to the body's value, so this is behavior-preserving — it
/// only collapses the syntax so a concept application costs 1 and the expanded
/// form and the primitive never become separate entries.
fn canonicalize(t: &Rc<Term>, concepts: &[Rc<Term>]) -> Rc<Term> {
    // No concepts in play → the identity quotient; skip the whole walk.
    if concepts.is_empty() {
        return t.clone();
    }
    match t.as_ref() {
        Term::App(f, a) => {
            let cf = canonicalize(f, concepts);
            let head = match concepts.iter().position(|b| Rc::ptr_eq(b, &cf) || b == &cf) {
                Some(idx) => Rc::new(Term::Prim(concepts[idx].clone())),
                None => cf.clone(),
            };
            let ca = canonicalize(a, concepts);
            if Rc::ptr_eq(&head, &cf) && Rc::ptr_eq(&ca, a) {
                t.clone() // nothing changed — share, don't allocate
            } else {
                app(head, ca)
            }
        }
        Term::Lam(b) => {
            let cb = canonicalize(b, concepts);
            if Rc::ptr_eq(&cb, b) {
                t.clone()
            } else {
                lam(cb)
            }
        }
        _ => t.clone(),
    }
}

/// How a candidate's per-test values are produced.
enum Make<'e> {
    /// Evaluate the term from scratch under the level env (atoms, opaque
    /// combinations).
    Eval,
    /// Closure over the level env (λ-abstraction; no evaluation needed).
    Closure(Rc<Term>),
    /// apply(f.vals[j], a) — the incremental fast path.
    Apply(&'e Entry, ArgSrc<'e>),
}

enum ArgSrc<'e> {
    Val(&'e Entry),
}

enum Step {
    Continue,
    Solved(Rc<Term>),
    OutOfTime,
}

struct Search<'a> {
    opts: &'a Options,
    start: Instant,
    k: u32,
    n_tests: usize,
    target: Vec<Rc<Term>>,
    target_hash: Vec<u64>,
    levels: Vec<Level>,
    stats: Stats,
    /// Canonical-keying ablation: when true, candidate and target identity use
    /// the compact canonical key ([`crate::canon::canonicalize`]) instead of the
    /// structural `quote_hash`, so ARC-sized grids stay hashable past the 2048
    /// structural cap. Representation-only — search semantics are unchanged.
    use_canon: bool,
}

impl<'a> Search<'a> {
    fn make_val(&self, c: u32, j: usize, t: &Rc<Term>, mk: &Make, fuel: &mut Fuel) -> Result<Rc<Val>, Abort> {
        let env = &self.levels[c as usize].envs[j];
        match mk {
            Make::Eval => eval(env, t, fuel),
            Make::Closure(body) => Ok(Rc::new(Val::Lam(env.clone(), body.clone()))),
            Make::Apply(f, a) => {
                let arg: Thunk = match a {
                    ArgSrc::Val(e) => thunk_of_val_rc(e.vals[j].clone()),
                };
                crate::nbe::apply(f.vals[j].clone(), arg, fuel)
            }
        }
    }

    fn process(
        &mut self,
        c: u32,
        t: Rc<Term>,
        mk: Make,
        kept: &mut Vec<Entry>,
        opaque: &mut Vec<Rc<Term>>,
    ) -> Step {
        self.stats.built += 1;
        // Quotient: collapse concept expansions to their Prim atom before the
        // candidate is evaluated or kept, so P/~L is what actually gets searched.
        let t = canonicalize(&t, &self.opts.concepts);
        if self.stats.built % 256 == 0
            && self.start.elapsed().as_secs_f64() > self.opts.time_budget_secs
        {
            return Step::OutOfTime;
        }

        let mut vals: Vec<Rc<Val>> = Vec::with_capacity(self.n_tests);
        let mut hashes: Vec<u64> = Vec::with_capacity(self.n_tests);
        let mut diverged = false;
        for j in 0..self.n_tests {
            let mut fuel = Fuel(self.opts.fuel);
            let r = self.make_val(c, j, &t, &mk, &mut fuel).and_then(|v| {
                if self.use_canon {
                    let mut h = DefaultHasher::new();
                    crate::canon::canonicalize(&v, &mut fuel, &mut h).map(|cv| (v, cv.key()))
                } else {
                    let mut h = DefaultHasher::new();
                    quote_hash(&v, 0, &mut fuel, &mut h)?;
                    Ok((v, h.finish()))
                }
            });
            match r {
                Ok((v, h)) => {
                    vals.push(v);
                    hashes.push(h);
                }
                Err(Abort) => {
                    diverged = true;
                    break;
                }
            }
        }

        if diverged {
            self.stats.aborted += 1;
            // The opaque pool only earns its keep when a fixpoint seed is in
            // play: that's when "no normal form now, normalizes when applied
            // to data" is a real phenomenon. Without seeds, divergent
            // candidates are just fuel-blowups (e.g. numeral exponentiation
            // towers) and keeping them cascades aborts through every level.
            if !self.opts.seeds.is_empty() {
                let sh = syn_hash(&t);
                if self.levels[c as usize].seen_syn.insert(sh)
                    && opaque.len() < self.opts.max_opaque_entries
                {
                    opaque.push(t);
                }
            }
            return Step::Continue;
        }

        if c == 0 && hashes == self.target_hash {
            // Hash match: verify structurally before declaring victory. In
            // canonical mode the key IS the exact identity (compact numeral/grid
            // key), so no structural re-check is needed — and quoting an
            // ARC-sized grid to verify would itself blow the fuel budget.
            let verified = if self.use_canon {
                true
            } else {
                (0..self.n_tests).all(|j| {
                    let mut fuel = Fuel(self.opts.fuel);
                    quote_eq(&vals[j], &self.target[j], 0, &mut fuel).unwrap_or(false)
                })
            };
            if verified {
                let mut sol = t;
                for _ in 0..self.k {
                    sol = lam(sol);
                }
                return Step::Solved(sol);
            }
        }

        let mut h = DefaultHasher::new();
        hashes.hash(&mut h);
        let kh = h.finish();
        if self.levels[c as usize].seen.insert(kh) && kept.len() < self.opts.max_level_entries {
            kept.push(Entry { term: t, vals });
            self.stats.kept += 1;
        }
        Step::Continue
    }
}

fn thunk_of_val_rc(v: Rc<Val>) -> Thunk {
    Rc::new(std::cell::RefCell::new(crate::nbe::Th::Done(v)))
}

/// Search for `@main` for this task. Returns the full closed solution term
/// (already wrapped in the k argument lambdas) if found within budget.
pub fn solve(task: &Task, opts: &Options) -> Outcome {
    solve_internal(task, opts, false)
}

/// [`solve`] through the canonical-keying ablation path: candidate and target
/// identity use the compact canonical key instead of the structural hash, so
/// ARC-sized grids stay hashable past the 2048 structural cap. Representation-
/// only — the search language is unchanged. This is what lets the A1 slice's
/// "naive seeds" control (B) reason about 8×8 grids on the same footing as the
/// canonical concept path (C), so the ontogenesis-vs-seeds comparison is not
/// confounded by the structural wall.
///
/// Lib-only API: consumed by the `arc1` demo crate, not by main.rs's private
/// copy of this module (which is why it is `#[allow(dead_code)]` here).
#[allow(dead_code)]
pub fn solve_abl(task: &Task, opts: &Options, use_canon: bool) -> Outcome {
    solve_internal(task, opts, use_canon)
}

fn solve_internal(task: &Task, opts: &Options, use_canon: bool) -> Outcome {
    let start = Instant::now();
    let k = task.arity as u32;
    let n_tests = task.tests.len();
    let empty: Env = Rc::new(Vec::new());

    // Normalize expected outputs and strip their outer test binders (the
    // harness normalizes the same way with lam).
    let mut target: Vec<Rc<Term>> = Vec::with_capacity(n_tests);
    for t in &task.tests {
        let mut fuel = Fuel(opts.fuel);
        let stripped = crate::nbe::normalize(&empty, &t.want, &mut fuel)
            .ok()
            .and_then(|nf| crate::parse::strip_outer(&nf, t.outer));
        match stripped {
            Some(nf) => target.push(nf),
            None => {
                return Outcome {
                    solution: None,
                    stats: Stats::default(),
                }
            }
        }
    }
    // Target hashes must be computed with the same streaming scheme used for
    // candidates: evaluate the (already normal) target term and hash it.
    let mut target_hash: Vec<u64> = Vec::with_capacity(n_tests);
    for nf in &target {
        let mut fuel = Fuel(i64::MAX / 2);
        let v = match eval(&empty, nf, &mut fuel) {
            Ok(v) => v,
            Err(_) => {
                return Outcome {
                    solution: None,
                    stats: Stats::default(),
                }
            }
        };
        let key = if use_canon {
            let mut h = DefaultHasher::new();
            match crate::canon::canonicalize(&v, &mut fuel, &mut h) {
                Ok(cv) => Some(cv.key()),
                Err(_) => None,
            }
        } else {
            let mut h = DefaultHasher::new();
            quote_hash(&v, 0, &mut fuel, &mut h).ok().map(|_| h.finish())
        };
        match key {
            Some(k) => target_hash.push(k),
            None => {
                return Outcome {
                    solution: None,
                    stats: Stats::default(),
                }
            }
        }
    }

    // Shared, memoized argument thunks: each test input is evaluated at most
    // once across the entire search. Context binders are free-constant
    // neutrals shared across environments.
    let arg_thunks: Vec<Vec<Thunk>> = task
        .tests
        .iter()
        .map(|t| {
            t.args
                .iter()
                .map(|a| thunk_delayed(empty.clone(), a.clone()))
                .collect()
        })
        .collect();

    let levels: Vec<Level> = (0..=opts.max_depth)
        .map(|c| {
            let envs = (0..n_tests)
                .map(|j| {
                    let mut e: Vec<Thunk> = arg_thunks[j].clone();
                    for i in 0..c {
                        e.push(thunk_of_val(Val::Neu(Head::Ctx(i), Vec::new())));
                    }
                    Rc::new(e)
                })
                .collect();
            Level {
                entries: vec![Vec::new()], // index 0 unused
                opaque: vec![Vec::new()],
                seen: HashSet::new(),
                seen_syn: HashSet::new(),
                envs,
            }
        })
        .collect();

    let mut search = Search {
        opts,
        start,
        k,
        n_tests,
        target,
        target_hash,
        levels,
        stats: Stats::default(),
        use_canon,
    };

    macro_rules! step {
        ($search:expr, $r:expr) => {
            match $r {
                Step::Solved(sol) => {
                    $search.stats.elapsed_secs = $search.start.elapsed().as_secs_f64();
                    return Outcome {
                        solution: Some(sol),
                        stats: std::mem::take(&mut $search.stats),
                    };
                }
                Step::OutOfTime => {
                    $search.stats.elapsed_secs = $search.start.elapsed().as_secs_f64();
                    return Outcome {
                        solution: None,
                        stats: std::mem::take(&mut $search.stats),
                    };
                }
                Step::Continue => {}
            }
        };
    }

    for s in 1..=opts.max_size {
        search.stats.reached_size = s;
        for c in 0..=opts.max_depth {
            let mut kept: Vec<Entry> = Vec::new();
            let mut opq: Vec<Rc<Term>> = Vec::new();

            // Variables, library seeds, and quotient concepts.
            if s == 1 {
                let mut atoms: Vec<Rc<Term>> = (0..(k + c)).map(var).collect();
                atoms.extend(opts.seeds.iter().cloned());
                // Each concept is injected as its Prim atom (size 1), so it is
                // a genuine primitive the search can think *through*.
                atoms.extend(opts.concepts.iter().map(|b| Rc::new(Term::Prim(b.clone()))));
                for t in atoms {
                    let r = search.process(c, t, Make::Eval, &mut kept, &mut opq);
                    step!(search, r);
                }
            }
            // Lambdas: wrap bodies from context c+1, size s-1. The value is
            // just a closure over this level's env — no evaluation at all.
            if s >= 2 && c + 1 <= opts.max_depth {
                let up = &search.levels[(c + 1) as usize];
                let mut bodies: Vec<Rc<Term>> = up
                    .entries
                    .get((s - 1) as usize)
                    .map(|v| v.iter().map(|e| e.term.clone()).collect())
                    .unwrap_or_default();
                if let Some(o) = up.opaque.get((s - 1) as usize) {
                    bodies.extend(o.iter().cloned());
                }
                for b in bodies {
                    let t = lam(b.clone());
                    let r = search.process(c, t, Make::Closure(b), &mut kept, &mut opq);
                    step!(search, r);
                }
            }
            // Applications: f from (c, s1), a from (c, s2), s1 + s2 = s - 1.
            if s >= 3 {
                for s1 in 1..=(s - 2) {
                    let s2 = s - 1 - s1;
                    let lvl = &search.levels[c as usize];
                    let fs: Vec<Entry> = lvl.entries.get(s1 as usize).cloned().unwrap_or_default();
                    let fo: Vec<Rc<Term>> =
                        lvl.opaque.get(s1 as usize).cloned().unwrap_or_default();
                    let asn: Vec<Entry> = lvl.entries.get(s2 as usize).cloned().unwrap_or_default();
                    let aso: Vec<Rc<Term>> =
                        lvl.opaque.get(s2 as usize).cloned().unwrap_or_default();

                    // Opaque terms act only as application *heads* (the
                    // Y(F) recursion shape); allowing them as arguments
                    // cascades divergent junk through every level.
                    for f in &fs {
                        for a in &asn {
                            let t = app(f.term.clone(), a.term.clone());
                            let r = search.process(
                                c,
                                t,
                                Make::Apply(f, ArgSrc::Val(a)),
                                &mut kept,
                                &mut opq,
                            );
                            step!(search, r);
                        }
                    }
                    for f in &fo {
                        for a in &asn {
                            let t = app(f.clone(), a.term.clone());
                            let r = search.process(c, t, Make::Eval, &mut kept, &mut opq);
                            step!(search, r);
                        }
                    }
                    let _ = &aso;
                }
            }

            let lvl = &mut search.levels[c as usize];
            while lvl.entries.len() <= s as usize {
                lvl.entries.push(Vec::new());
                lvl.opaque.push(Vec::new());
            }
            lvl.entries[s as usize] = kept;
            lvl.opaque[s as usize] = opq;
        }
    }

    search.stats.elapsed_secs = start.elapsed().as_secs_f64();
    Outcome {
        solution: None,
        stats: search.stats,
    }
}

/// A mined/invented concept made available to the quotient-aware search: a
/// closed body (e.g. `mul`'s combinator) plus a display name.
///
/// `arity` is the concept's *composition* arity — how many inputs it consumes
/// to produce a value in its result domain — NOT its λ-arity. `mul =
/// λa.λb.λc.b(a(c))` has three leading λs but is applied to two numerals to
/// yield a product, so its composition arity is 2. This is a property of how
/// the concept is *used*, recorded when it was mined from solved applications.
#[derive(Clone)]
pub struct Concept {
    pub body: Rc<Term>,
    pub name: String,
    pub arity: u32,
}

struct PoolEntry {
    term: Rc<Term>,
    vals: Vec<Rc<Val>>,
}

fn val_hash(v: &Val, fuel: &mut Fuel) -> Option<u64> {
    let mut h = DefaultHasher::new();
    quote_hash(v, 0, fuel, &mut h).ok()?;
    Some(h.finish())
}

/// The single identity-key abstraction for a value, used uniformly for the task
/// arguments, the (optionally seeded) concept-value pool entries, the generated
/// candidates, and the target. Keeping every site on this one function guarantees
/// concept seeds enter `seen` through exactly the same structural/canonical mode
/// as ordinary generated entries — otherwise the initial and generated pool states
/// could diverge in dedup semantics.
///
/// Canonical mode bounds the canonicalizer by `opts.fuel` (the same budget the
/// engine uses); the structural fallback is bounded by `struct_fuel`, which callers
/// set per site to preserve historical budgets (targets got a near-unbounded
/// budget, generated candidates got 2048, seeds got `opts.fuel`).
fn value_key(v: &Val, use_canon: bool, struct_fuel: i64, opts: &Options) -> Option<u64> {
    if use_canon {
        let mut fuel = Fuel(opts.fuel);
        let mut h = DefaultHasher::new();
        crate::canon::canonicalize(v, &mut fuel, &mut h)
            .ok()
            .map(|cv| cv.key())
    } else {
        val_hash(v, &mut Fuel(struct_fuel))
    }
}

/// Condition C — a search that *thinks through* its concepts.
///
/// The raw bank (`solve`) re-derives a concept's expansion from scratch, and
/// naive seeding sprays it as a universal atom; neither reduces search cost.
/// This variant composes the given concepts over its inputs instead: the pool
/// starts with the k task arguments, and each round applies every concept to
/// every arity-tuple of pool values. Because the pool holds only inputs and
/// concept-results, applications stay on the concept's domain — no junk from
/// applying it to arbitrary intermediate functions.
///
/// The emitted solution is a tree of `Prim` applications, so `size_L(C(x,y))=1`
/// is literal, and the search's state count is the number of concept
/// compositions tried — tiny compared to the raw λ enumeration. `built` counts
/// those compositions, the honest cost of reasoning *through* the concepts.
pub fn concept_solve(task: &Task, concepts: &[Concept], opts: &Options) -> Outcome {
    let start = Instant::now();
    let k = task.arity as u32;
    let n_tests = task.tests.len();
    let empty: Env = Rc::new(Vec::new());

    // Target normal forms and hashes — same protocol as `solve`.
    let mut target: Vec<Rc<Term>> = Vec::with_capacity(n_tests);
    for t in &task.tests {
        let mut fuel = Fuel(opts.fuel);
        let stripped = crate::nbe::normalize(&empty, &t.want, &mut fuel)
            .ok()
            .and_then(|nf| crate::parse::strip_outer(&nf, t.outer));
        match stripped {
            Some(nf) => target.push(nf),
            None => {
                return Outcome {
                    solution: None,
                    stats: Stats::default(),
                }
            }
        }
    }
    let mut target_hash: Vec<u64> = Vec::with_capacity(n_tests);
    for nf in &target {
        let mut fuel = Fuel(i64::MAX / 2);
        let v = match eval(&empty, nf, &mut fuel) {
            Ok(v) => v,
            Err(_) => return Outcome { solution: None, stats: Stats::default() },
        };
        let mut h = DefaultHasher::new();
        if quote_hash(&v, 0, &mut fuel, &mut h).is_err() {
            return Outcome { solution: None, stats: Stats::default() };
        }
        target_hash.push(h.finish());
    }

    // Pool seeded with the task arguments: `var(i)` has value = arg_i per test.
    let mut pool: Vec<PoolEntry> = Vec::new();
    for i in 0..k as usize {
        let mut vals = Vec::with_capacity(n_tests);
        for j in 0..n_tests {
            let mut fuel = Fuel(opts.fuel);
            let v = match eval(&empty, &task.tests[j].args[i], &mut fuel) {
                Ok(v) => v,
                Err(_) => return Outcome { solution: None, stats: Stats::default() },
            };
            vals.push(v);
        }
        pool.push(PoolEntry { term: var(i as u32), vals });
    }

    let mut seen: HashSet<Vec<u64>> = HashSet::new();
    for e in &pool {
        let hashes: Vec<u64> = (0..n_tests)
            .map(|j| val_hash(&e.vals[j], &mut Fuel(opts.fuel)).unwrap_or(0))
            .collect();
        if hashes == target_hash {
            let mut sol = e.term.clone();
            for _ in 0..k {
                sol = lam(sol);
            }
            return Outcome {
                solution: Some(sol),
                stats: Stats { built: 1, ..Default::default() },
            };
        }
        seen.insert(hashes);
    }

    let mut built: u64 = 0;
    // Bounded pool: giant concept-results (e.g. `mul` of large numerals) would
    // otherwise balloon the composition space. The hash fuel below prunes any
    // normal form over ~2k nodes, keeping the pool on small, meaningful values.
    let pool_cap = 64usize;
    loop {
        let before = pool.len();
        let mut additions: Vec<PoolEntry> = Vec::new();
        for concept in concepts {
            let a = concept.arity as usize;
            if a == 0 {
                continue;
            }
            let mut tuple: Vec<usize> = vec![0; a];
            loop {
                built += 1;
                if built % 512 == 0 && start.elapsed().as_secs_f64() > opts.time_budget_secs {
                    return Outcome {
                        solution: None,
                        stats: Stats { built, ..Default::default() },
                    };
                }
                // Compute the concept applied to this tuple, per test.
                let mut vals: Option<Vec<Rc<Val>>> = Some(Vec::with_capacity(n_tests));
                for j in 0..n_tests {
                    let mut fuel = Fuel(opts.fuel);
                    let mut v = match eval(&empty, &concept.body, &mut fuel) {
                        Ok(v) => v,
                        Err(_) => {
                            vals = None;
                            break;
                        }
                    };
                    let mut ok = true;
                    for &ti in &tuple {
                        let arg = thunk_of_val_rc(pool[ti].vals[j].clone());
                        match crate::nbe::apply(v.clone(), arg, &mut fuel) {
                            Ok(nv) => v = nv,
                            Err(_) => {
                                ok = false;
                                break;
                            }
                        }
                    }
                    if !ok {
                        vals = None;
                        break;
                    }
                    if let Some(vs) = vals.as_mut() {
                        vs.push(v);
                    }
                }
                if let Some(vs) = vals {
                    let mut term = Rc::new(Term::Prim(concept.body.clone()));
                    for &ti in &tuple {
                        term = app(term, pool[ti].term.clone());
                    }
                    // Bound the normal form: if any test's value is too large to
                    // hash within the cap, skip the tuple entirely (don't keep).
                    let mut hashes = Vec::with_capacity(n_tests);
                    let mut ok_hash = true;
                    for j in 0..n_tests {
                        match val_hash(&vs[j], &mut Fuel(2048)) {
                            Some(h) => hashes.push(h),
                            None => {
                                ok_hash = false;
                                break;
                            }
                        }
                    }
                    if !ok_hash {
                        // advance tuple (handled below)
                    } else if hashes == target_hash {
                        let mut sol = term.clone();
                        for _ in 0..k {
                            sol = lam(sol);
                        }
                        let mut st = Stats::default();
                        st.built = built;
                        st.elapsed_secs = start.elapsed().as_secs_f64();
                        return Outcome { solution: Some(sol), stats: st };
                    } else if seen.insert(hashes.clone()) && additions.len() < pool_cap {
                        additions.push(PoolEntry { term, vals: vs });
                    }
                }
                // Advance the tuple counter (odometer over the pool).
                let mut done = true;
                for d in tuple.iter_mut() {
                    if *d + 1 < pool.len() {
                        *d += 1;
                        done = false;
                        break;
                    }
                    *d = 0;
                }
                if done {
                    break;
                }
            }
        }
        if additions.is_empty() {
            break;
        }
        pool.extend(additions);
        if pool.len() >= pool_cap || pool.len() == before {
            break;
        }
    }

    Outcome {
        solution: None,
        stats: Stats { built, ..Default::default() },
    }
}

/// Meters for the value-representation ablation: where does the "materializing
/// semantic values" wall sit? Read from the thread-local nbe/canon meters after
/// a `concept_solve_abl` call on the same thread.
#[derive(Default, Clone, Debug)]
pub struct Meters {
    /// β-reduction steps during candidate evaluation (normalization work).
    pub norm_steps: u64,
    /// Candidate evaluations aborted because eval/apply ran out of fuel.
    pub eval_aborts: u64,
    /// Candidates dropped because a value couldn't be identified within budget
    /// (structural: the 2048-fuel hash cap; canonical: a canonicalize abort).
    pub hash_aborts: u64,
    /// Node-walks performed by numeral canonicalization.
    pub canon_nodes: u64,
    /// Canonicalization passes that aborted (value too big to observe).
    pub canon_aborts: u64,
    /// Node-reads performed by structural (fallback) hashing.
    pub quote_nodes: u64,
    /// Largest single numeral the canonicalizer walked in one pass (~ the
    /// materialized transient's size) — the peak transient the pool would
    /// otherwise have had to keep.
    pub max_transient: u64,
    /// Final pool size (how many distinct values the search kept).
    pub pool_entries: usize,
}

impl Meters {
    /// Fill the nbe/canon meter fields from the thread-locals. Called before
    /// BOTH the solution and non-solution returns so a solved size still reports
    /// real C_value numbers (previously only the non-solution exit filled them).
    /// `eval_aborts`/`hash_aborts` are counted in the search loop and left alone.
    fn fill(&mut self, pool_len: usize) {
        self.pool_entries = pool_len;
        self.norm_steps = crate::nbe::beta_steps();
        self.quote_nodes = crate::nbe::quote_nodes();
        self.canon_nodes = crate::canon::canon_nodes();
        self.canon_aborts = crate::canon::canon_aborts();
        self.max_transient = crate::canon::max_transient();
    }
}

/// Ablation variant of `concept_solve` (condition C). Identical search,
/// ontology, pool cap, and evaluation fuel — the ONLY knob is how a value's
/// identity is computed for dedup + target matching:
///
/// - `Structural`: the existing engine — `val_hash` with the 2048-fuel cap
///   (exactly as `concept_solve`). Baseline.
/// - `Canonical`: canonical-key observation (`canon::canonicalize`) with the
///   full evaluation fuel budget. A Church numeral `λf.λx.f^n(x)` collapses to
///   `ChurchNumeral(n)`, O(1) to store/hash/compare, even though the value was
///   produced by ordinary Church β-reduction. No arithmetic; β-reduction and the
///   searchable language are untouched.
///
/// The experiment: run folds 4–10 under both, see where the wall moves.
///
/// Measured result (see `supsearch ablation`): the two columns are IDENTICAL on
/// this family — every fold solves/fails the same way at the same pool size. The
/// canonical path keeps large numerals (max_trans up to 6561 nodes observed) but
/// never changes reachability, because this family's target values are small and
/// no large value is on the critical path. The fold≥9 wall is the composition
/// search space (pool_cap=64), not the value representation: raising the cap to
/// 512 unlocks fold9 in both modes. This is a falsifying negative for the
/// representation-only hypothesis.
pub fn concept_solve_abl(
    task: &Task,
    concepts: &[Concept],
    opts: &Options,
    use_canon: bool,
) -> (Outcome, Meters) {
    concept_solve_internal(task, concepts, opts, use_canon, false)
}

/// Higher-order quotient search (C8): identical to [`concept_solve_abl`] except
/// the pool is additionally seeded with each installed concept's body as a
/// function-valued entry, so higher-order concepts (`map`, `compose`) can take
/// other concepts as arguments — e.g. `map` applied to `(reverse, grid)` yields
/// the mirror. No new concepts are added; the ontology is frozen. Only the
/// search language changes: it can now hold function-typed intermediates.
///
/// The decisive diagnostic test: rerun the 400-task arcdiag with this engine and
/// the 4 EXPRESSIBLE tasks (mirror/v-tile/rotation) should move into SOLVED
/// without any ontology growth — SOLVED 4→8, EXPRESSIBLE 4→0.
///
/// This is a lib-only API entry point consumed by the arc1 crate. The main
/// binary compiles its own private copy of this module (which does not call it),
/// so keep it in the public API without a dead-code warning from that copy.
#[allow(dead_code)]
pub fn concept_solve_ho_abl(
    task: &Task,
    concepts: &[Concept],
    opts: &Options,
    use_canon: bool,
) -> (Outcome, Meters) {
    concept_solve_internal(task, concepts, opts, use_canon, true)
}

/// Shared body of the canonical-keying quotient search (condition C). When
/// `seed_concepts`, each concept's body (arity ≥ 1) is also seeded into the pool
/// as a function-valued entry (its closure), alongside the task-argument values.
fn concept_solve_internal(
    task: &Task,
    concepts: &[Concept],
    opts: &Options,
    use_canon: bool,
    seed_concepts: bool,
) -> (Outcome, Meters) {
    let start = Instant::now();
    let k = task.arity as u32;
    let n_tests = task.tests.len();
    let empty: Env = Rc::new(Vec::new());
    let mut m = Meters::default();

    // Target normal forms.
    let mut target: Vec<Rc<Term>> = Vec::with_capacity(n_tests);
    for t in &task.tests {
        let mut fuel = Fuel(opts.fuel);
        let stripped = crate::nbe::normalize(&empty, &t.want, &mut fuel)
            .ok()
            .and_then(|nf| crate::parse::strip_outer(&nf, t.outer));
        match stripped {
            Some(nf) => target.push(nf),
            None => {
                return (Outcome { solution: None, stats: Stats::default() }, m);
            }
        }
    }
    // Target identity: canonical keys if `use_canon`, else structural hashes.
    // The near-unbounded structural budget (i64::MAX/2) preserves the historical
    // target path.
    let mut target_keys: Vec<u64> = Vec::with_capacity(n_tests);
    for nf in &target {
        let mut fuel = Fuel(opts.fuel);
        let v = match eval(&empty, nf, &mut fuel) {
            Ok(v) => v,
            Err(_) => return (Outcome { solution: None, stats: Stats::default() }, m),
        };
        match value_key(&v, use_canon, i64::MAX / 2, opts) {
            Some(k) => target_keys.push(k),
            None => return (Outcome { solution: None, stats: Stats::default() }, m),
        }
    }

    // Pool seeded with the task arguments.
    let mut pool: Vec<PoolEntry> = Vec::new();
    for i in 0..k as usize {
        let mut vals = Vec::with_capacity(n_tests);
        for j in 0..n_tests {
            let mut fuel = Fuel(opts.fuel);
            let v = match eval(&empty, &task.tests[j].args[i], &mut fuel) {
                Ok(v) => v,
                Err(_) => return (Outcome { solution: None, stats: Stats::default() }, m),
            };
            vals.push(v);
        }
        pool.push(PoolEntry {
            term: var(i as u32),
            vals,
        });
    }

    // C8 higher-order search: seed each concept's body as a function-valued pool
    // entry (its closure), so higher-order concepts (map, compose) can receive
    // other concepts as arguments — e.g. map applied to (reverse, grid) → mirror.
    // Function-valued entries hash fine (quote_hash/canonicalize handle Val::Lam)
    // and never equal a grid target, so they enable composition without false
    // solves. `c.body` is the closed λ-term; its eval'd value is the closure.
    //
    // Load-bearing: the entry's TERM is `Prim(c.body)` — the quotient atom — not
    // `c.body` itself. When the search composes (map, reverse, grid) it must emit
    // `Prim(map);Prim(reverse);grid`, so the returned program references the
    // concept as an atom and only ever expands it under evaluation, rather than
    // embedding reverse's λ-body inline. The runtime value is still the evaluated
    // closure; only the emitted term differs.
    if seed_concepts {
        for c in concepts {
            if c.arity == 0 {
                continue;
            }
            // A closed concept body evaluates to the same closure in the empty
            // env across tests, so eval once and replicate the Rc across tests.
            let mut fuel = Fuel(opts.fuel);
            match eval(&empty, &c.body, &mut fuel) {
                Ok(v) => {
                    let vals = std::iter::repeat(v).take(n_tests).collect();
                    pool.push(PoolEntry {
                        term: Rc::new(Term::Prim(c.body.clone())),
                        vals,
                    });
                }
                Err(_) => continue,
            }
        }
    }

    let mut seen: HashSet<Vec<u64>> = HashSet::new();
    for e in &pool {
        let hashes: Vec<u64> = (0..n_tests)
            .map(|j| value_key(&e.vals[j], use_canon, opts.fuel, opts).unwrap_or(0))
            .collect();
        if hashes == target_keys {
            let mut sol = e.term.clone();
            for _ in 0..k {
                sol = lam(sol);
            }
            return (
                Outcome {
                    solution: Some(sol),
                    stats: Stats { built: 1, ..Default::default() },
                },
                m,
            );
        }
        seen.insert(hashes);
    }

    let mut built: u64 = 0;
    let pool_cap = 64usize;
    loop {
        let before = pool.len();
        let mut additions: Vec<PoolEntry> = Vec::new();
        for concept in concepts {
            let a = concept.arity as usize;
            if a == 0 {
                continue;
            }
            let mut tuple: Vec<usize> = vec![0; a];
            loop {
                built += 1;
                if built % 512 == 0 && start.elapsed().as_secs_f64() > opts.time_budget_secs {
                    return (
                        Outcome { solution: None, stats: Stats { built, ..Default::default() } },
                        m,
                    );
                }
                let mut vals: Option<Vec<Rc<Val>>> = Some(Vec::with_capacity(n_tests));
                for j in 0..n_tests {
                    let mut fuel = Fuel(opts.fuel);
                    let mut v = match eval(&empty, &concept.body, &mut fuel) {
                        Ok(v) => v,
                        Err(_) => {
                            m.eval_aborts += 1;
                            vals = None;
                            break;
                        }
                    };
                    let mut ok = true;
                    for &ti in &tuple {
                        let arg = thunk_of_val_rc(pool[ti].vals[j].clone());
                        match crate::nbe::apply(v.clone(), arg, &mut fuel) {
                            Ok(nv) => v = nv,
                            Err(_) => {
                                ok = false;
                                break;
                            }
                        }
                    }
                    if !ok {
                        m.eval_aborts += 1;
                        vals = None;
                        break;
                    }
                    if let Some(vs) = vals.as_mut() {
                        vs.push(v);
                    }
                }
                if let Some(vs) = vals {
                    let mut term = Rc::new(Term::Prim(concept.body.clone()));
                    for &ti in &tuple {
                        term = app(term, pool[ti].term.clone());
                    }
                    // Identity of the tuple's values: same unified key abstraction
                    // as the seeds and target (canonical keys OR structural hash),
                    // with the historical 2048 structural budget for candidates.
                    let mut keys = Vec::with_capacity(n_tests);
                    let mut ok_key = true;
                    for j in 0..n_tests {
                        let key = value_key(&vs[j], use_canon, 2048, opts);
                        match key {
                            Some(k) => keys.push(k),
                            None => {
                                if use_canon {
                                    m.canon_aborts += 1;
                                }
                                ok_key = false;
                                break;
                            }
                        }
                    }
                    if !ok_key {
                        m.hash_aborts += 1;
                        // drop the tuple (too big to identify within budget)
                    } else if keys == target_keys {
                        let mut sol = term.clone();
                        for _ in 0..k {
                            sol = lam(sol);
                        }
                        m.fill(pool.len());
                        return (
                            Outcome {
                                solution: Some(sol),
                                stats: Stats { built, ..Default::default() },
                            },
                            m,
                        );
                    } else if seen.insert(keys.clone())
                        && additions.len() < pool_cap
                        && (!seed_concepts || pool.len() + additions.len() < pool_cap)
                    {
                        additions.push(PoolEntry { term, vals: vs });
                    }
                }
                let mut done = true;
                for d in tuple.iter_mut() {
                    if *d + 1 < pool.len() {
                        *d += 1;
                        done = false;
                        break;
                    }
                    *d = 0;
                }
                if done {
                    break;
                }
            }
        }
        if additions.is_empty() {
            break;
        }
        pool.extend(additions);
        // Baseline keeps its historical single-round bound: stop as soon as the
        // pool hits the cap (its pool starts at just the task args, so this fires
        // after round 1). The higher-order path (seed_concepts) instead keeps
        // composing until no new distinct value appears — round 2 is what applies
        // `reverse` to the round-1 mirror grid to reach rotation. Both paths stay
        // memory-bounded at `pool_cap`: the additions guard above refuses to grow
        // the pool beyond it, so the seeded loop stops adding once the pool fills
        // and only continues enumeration, which the time budget bounds.
        if !seed_concepts && (pool.len() >= pool_cap || pool.len() == before) {
            break;
        }
    }

    m.fill(pool.len());
    (Outcome { solution: None, stats: Stats { built, ..Default::default() } }, m)
}

// ─────────────────────────────────────────────────────────────────────────────
// C5A diagnostic instrumentation. A mirror of `concept_solve` (structural
// keys, 2048-fuel hash cap) that records PROVENANCE for every admitted pool
// entry, every BUILT candidate, and the winning composition, so the fold≥9
// composition wall can be measured rather than assumed. Search semantics are
// identical to `concept_solve` when `prune == false` and `pool_cap == 64`
// (verified by matching `built` counts). Observational only.
// ─────────────────────────────────────────────────────────────────────────────

/// Whether to apply semantic-dominance pruning (C5A A6). In `Prune` mode a
/// newly built candidate that is behaviorally equivalent to an already-admitted
/// representative is discarded unless it is strictly cheaper, in which case it
/// replaces the representative. Search order, seen-set gating, and the pool
/// length cap are otherwise unchanged.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiagMode {
    Baseline,
    Prune,
}

/// One admitted pool entry with its provenance.
#[derive(Clone)]
pub struct DiagEntry {
    /// Logical admission index (== final pool position; parent ids use these).
    pub id: usize,
    /// Search level that produced it: 0 = the raw task-argument leaves.
    pub generation: u32,
    /// The composed term (Prim applications over pool terms; a leaf is `var`).
    pub term: Rc<Term>,
    /// Term cost (node count; Prim and Var both count 1).
    pub cost: u32,
    /// Behavior identity per test (the full signature vector).
    pub keys: Vec<u64>,
    /// Pool indices of the operands that built this entry (empty for leaves).
    pub parent_ids: Vec<usize>,
    /// The concept/operator that built it, or "<arg>" for a leaf.
    pub constructor: String,
    /// Whether this entry replaced an earlier representative of the same key
    /// (prune mode) rather than being freshly admitted.
    pub replaced: bool,
}

/// Every built candidate (one per concept-application tuple tried). Includes
/// candidates that were deduped by the seen-set, dropped when the pool cap was
/// saturated, and the winner itself — so redundancy can be classified.
///
/// `parents`/`constructor` are A1 provenance, recorded observationally for every
/// candidate; only `key`/`cost`/`admitted` are consumed by the printed analysis,
/// but the full record is kept so a future pass can walk arbitrary candidate
/// ancestry without re-running the search.
#[allow(dead_code)]
pub struct DiagCandidate {
    pub key: Vec<u64>,
    pub cost: u32,
    /// True if it claimed a pool slot (fresh key admitted, or a replacement).
    pub admitted: bool,
    pub parents: Vec<usize>,
    pub constructor: String,
}

/// The winning composition (key == target). Not admitted to the pool (the
/// search returns immediately on a match), so it carries its own provenance.
#[allow(dead_code)]
pub struct DiagWinner {
    pub key: Vec<u64>,
    pub cost: u32,
    pub parents: Vec<usize>,
    pub constructor: String,
    /// Number of pool entries admitted before the winner was found.
    pub pool_len_at_solve: usize,
}

pub struct Diag {
    pub solution: Option<Rc<Term>>,
    pub built: u64,
    /// All admitted entries, in admission order (this is the pool).
    pub pool: Vec<DiagEntry>,
    /// All built candidates (for redundancy classification).
    pub candidates: Vec<DiagCandidate>,
    pub winner: Option<DiagWinner>,
    pub time_budget_hit: bool,
}

pub fn concept_solve_diag(
    task: &Task,
    concepts: &[Concept],
    opts: &Options,
    pool_cap: usize,
    mode: DiagMode,
) -> Diag {
    let start = Instant::now();
    let k = task.arity as u32;
    let n_tests = task.tests.len();
    let empty: Env = Rc::new(Vec::new());
    let prune = mode == DiagMode::Prune;

    // Target normal forms and hashes — identical to `concept_solve`.
    let mut target: Vec<Rc<Term>> = Vec::with_capacity(n_tests);
    for t in &task.tests {
        let mut fuel = Fuel(opts.fuel);
        let stripped = crate::nbe::normalize(&empty, &t.want, &mut fuel)
            .ok()
            .and_then(|nf| crate::parse::strip_outer(&nf, t.outer));
        match stripped {
            Some(nf) => target.push(nf),
            None => {
                return Diag {
                    solution: None,
                    built: 0,
                    pool: Vec::new(),
                    candidates: Vec::new(),
                    winner: None,
                    time_budget_hit: false,
                }
            }
        }
    }
    let mut target_hash: Vec<u64> = Vec::with_capacity(n_tests);
    for nf in &target {
        let mut fuel = Fuel(i64::MAX / 2);
        let v = match eval(&empty, nf, &mut fuel) {
            Ok(v) => v,
            Err(_) => {
                return Diag {
                    solution: None,
                    built: 0,
                    pool: Vec::new(),
                    candidates: Vec::new(),
                    winner: None,
                    time_budget_hit: false,
                }
            }
        };
        let mut h = DefaultHasher::new();
        if quote_hash(&v, 0, &mut fuel, &mut h).is_err() {
            return Diag {
                solution: None,
                built: 0,
                pool: Vec::new(),
                candidates: Vec::new(),
                winner: None,
                time_budget_hit: false,
            };
        }
        target_hash.push(h.finish());
    }

    // Pool seeded with the task arguments (leaves, generation 0).
    let mut pool: Vec<DiagEntry> = Vec::new();
    let mut pool_vals: Vec<Vec<Rc<Val>>> = Vec::new();
    for i in 0..k as usize {
        let mut vals = Vec::with_capacity(n_tests);
        for j in 0..n_tests {
            let mut fuel = Fuel(opts.fuel);
            let v = match eval(&empty, &task.tests[j].args[i], &mut fuel) {
                Ok(v) => v,
                Err(_) => {
                    return Diag {
                        solution: None,
                        built: 0,
                        pool: Vec::new(),
                        candidates: Vec::new(),
                        winner: None,
                        time_budget_hit: false,
                    }
                }
            };
            vals.push(v);
        }
        pool_vals.push(vals);
        pool.push(DiagEntry {
            id: i,
            generation: 0,
            term: var(i as u32),
            cost: 1,
            keys: Vec::new(),
            parent_ids: Vec::new(),
            constructor: "<arg>".into(),
            replaced: false,
        });
    }

    // seen: every distinct key ever observed (blocks re-admission, unbounded).
    let mut seen: HashSet<Vec<u64>> = HashSet::new();
    // rep (prune only): key -> pool id of the cheapest representative so far.
    let mut rep: HashMap<Vec<u64>, usize> = HashMap::new();
    for i in 0..pool.len() {
        let hashes: Vec<u64> = (0..n_tests)
            .map(|j| val_hash(&pool_vals[i][j], &mut Fuel(opts.fuel)).unwrap_or(0))
            .collect();
        // NOTE: leaf keys are not target (leaves are single args, target is the
        // whole product), but record them for seen gating just like concept_solve.
        if hashes == target_hash {
            let mut sol = pool[i].term.clone();
            for _ in 0..k {
                sol = lam(sol);
            }
            return Diag {
                solution: Some(sol),
                built: 1,
                pool: pool.clone(),
                candidates: Vec::new(),
                winner: None,
                time_budget_hit: false,
            };
        }
        seen.insert(hashes.clone());
        if prune {
            rep.insert(hashes, i);
        }
    }

    let mut candidates: Vec<DiagCandidate> = Vec::new();
    let mut built: u64 = 0;
    let mut generation: u32 = 0;
    loop {
        generation += 1;
        let before = pool.len();
        let mut additions: Vec<(DiagEntry, Vec<Rc<Val>>)> = Vec::new();
        let mut time_budget_hit = false;
        for concept in concepts {
            let a = concept.arity as usize;
            if a == 0 {
                continue;
            }
            let mut tuple: Vec<usize> = vec![0; a];
            loop {
                built += 1;
                if built % 512 == 0 && start.elapsed().as_secs_f64() > opts.time_budget_secs {
                    time_budget_hit = true;
                    break;
                }
                // Compute the concept applied to this tuple, per test.
                let mut vals: Option<Vec<Rc<Val>>> = Some(Vec::with_capacity(n_tests));
                for j in 0..n_tests {
                    let mut fuel = Fuel(opts.fuel);
                    let mut v = match eval(&empty, &concept.body, &mut fuel) {
                        Ok(v) => v,
                        Err(_) => {
                            vals = None;
                            break;
                        }
                    };
                    let mut ok = true;
                    for &ti in &tuple {
                        let arg = thunk_of_val_rc(pool_vals[ti][j].clone());
                        match crate::nbe::apply(v.clone(), arg, &mut fuel) {
                            Ok(nv) => v = nv,
                            Err(_) => {
                                ok = false;
                                break;
                            }
                        }
                    }
                    if !ok {
                        vals = None;
                        break;
                    }
                    if let Some(vs) = vals.as_mut() {
                        vs.push(v);
                    }
                }
                if time_budget_hit {
                    break;
                }
                if let Some(vs) = vals {
                    let mut term = Rc::new(Term::Prim(concept.body.clone()));
                    for &ti in &tuple {
                        term = app(term, pool[ti].term.clone());
                    }
                    let cost = term.size();
                    // Behavior identity: structural hash, 2048-fuel cap.
                    let mut hashes = Vec::with_capacity(n_tests);
                    let mut ok_hash = true;
                    for j in 0..n_tests {
                        match val_hash(&vs[j], &mut Fuel(2048)) {
                            Some(h) => hashes.push(h),
                            None => {
                                ok_hash = false;
                                break;
                            }
                        }
                    }
                    if ok_hash {
                        // parents reference pool ids; if any parent was itself
                        // freshly added this round, keep the logical id mapping.
                        let parent_ids = tuple
                            .iter()
                            .map(|&ti| pool[ti].id)
                            .collect::<Vec<_>>();
                        if hashes == target_hash {
                            let mut sol = term.clone();
                            for _ in 0..k {
                                sol = lam(sol);
                            }
                            let pool_len_at_solve = pool.len() + additions.len();
                            return Diag {
                                solution: Some(sol),
                                built,
                                pool,
                                candidates,
                                winner: Some(DiagWinner {
                                    key: hashes,
                                    cost,
                                    parents: parent_ids,
                                    constructor: concept.name.clone(),
                                    pool_len_at_solve,
                                }),
                                time_budget_hit: false,
                            };
                        }
                        // Classify/record the candidate regardless of admission.
                        if prune {
                            if let Some(&idx) = rep.get(&hashes) {
                                // Already have a representative. Replace only if cheaper.
                                if cost < pool[idx].cost {
                                    let old = &mut pool[idx];
                                    old.term = term.clone();
                                    old.cost = cost;
                                    old.keys = hashes.clone();
                                    old.parent_ids = parent_ids.clone();
                                    old.constructor = concept.name.clone();
                                    old.replaced = true;
                                    old.generation = generation;
                                    pool_vals[idx] = vs;
                                    candidates.push(DiagCandidate {
                                        key: hashes,
                                        cost,
                                        admitted: true,
                                        parents: parent_ids,
                                        constructor: concept.name.clone(),
                                    });
                                } else {
                                    candidates.push(DiagCandidate {
                                        key: hashes,
                                        cost,
                                        admitted: false,
                                        parents: parent_ids,
                                        constructor: concept.name.clone(),
                                    });
                                }
                            } else if seen.insert(hashes.clone()) && additions.len() < pool_cap {
                                let id = pool.len() + additions.len();
                                additions.push((
                                    DiagEntry {
                                        id,
                                        generation,
                                        term: term.clone(),
                                        cost,
                                        keys: hashes.clone(),
                                        parent_ids: parent_ids.clone(),
                                        constructor: concept.name.clone(),
                                        replaced: false,
                                    },
                                    vs,
                                ));
                                // NOTE: rep is populated only after the round's
                                // additions are appended to the pool (below), so a
                                // `rep.get` never yields a not-yet-applied index.
                                candidates.push(DiagCandidate {
                                    key: hashes,
                                    cost,
                                    admitted: true,
                                    parents: parent_ids,
                                    constructor: concept.name.clone(),
                                });
                            } else {
                                // Fresh key but pool saturated (or additions full):
                                // record it, mark seen so it is never re-attempted.
                                seen.insert(hashes.clone());
                                candidates.push(DiagCandidate {
                                    key: hashes,
                                    cost,
                                    admitted: false,
                                    parents: parent_ids,
                                    constructor: concept.name.clone(),
                                });
                            }
                        } else {
                            if seen.insert(hashes.clone()) && additions.len() < pool_cap {
                                let id = pool.len() + additions.len();
                                additions.push((
                                    DiagEntry {
                                        id,
                                        generation,
                                        term: term.clone(),
                                        cost,
                                        keys: hashes.clone(),
                                        parent_ids: parent_ids.clone(),
                                        constructor: concept.name.clone(),
                                        replaced: false,
                                    },
                                    vs,
                                ));
                                candidates.push(DiagCandidate {
                                    key: hashes,
                                    cost,
                                    admitted: true,
                                    parents: parent_ids,
                                    constructor: concept.name.clone(),
                                });
                            } else {
                                candidates.push(DiagCandidate {
                                    key: hashes,
                                    cost,
                                    admitted: false,
                                    parents: parent_ids,
                                    constructor: concept.name.clone(),
                                });
                            }
                        }
                    }
                }
                // Advance the tuple counter (odometer over the pool).
                let mut done = true;
                for d in tuple.iter_mut() {
                    if *d + 1 < pool.len() {
                        *d += 1;
                        done = false;
                        break;
                    }
                    *d = 0;
                }
                if done {
                    break;
                }
            }
            if time_budget_hit {
                break;
            }
        }
        if time_budget_hit {
            return Diag {
                solution: None,
                built,
                pool,
                candidates,
                winner: None,
                time_budget_hit: true,
            };
        }
        if additions.is_empty() {
            break;
        }
        for (e, v) in additions {
            pool_vals.push(v);
            let id = pool.len();
            pool.push(e);
            if prune {
                rep.insert(pool[id].keys.clone(), id);
            }
        }
        if pool.len() >= pool_cap || pool.len() == before {
            break;
        }
    }

    Diag {
        solution: None,
        built,
        pool,
        candidates,
        winner: None,
        time_budget_hit: false,
    }
}
