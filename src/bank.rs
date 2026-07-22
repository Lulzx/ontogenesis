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
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::time::Instant;

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
    Thunk(Rc<Term>),
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
                    ArgSrc::Thunk(at) => thunk_delayed(env.clone(), at.clone()),
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
        if self.stats.built % 4096 == 0
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
                let mut h = DefaultHasher::new();
                quote_hash(&v, 0, &mut fuel, &mut h)?;
                Ok((v, h.finish()))
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
            // Hash match: verify structurally before declaring victory.
            let verified = (0..self.n_tests).all(|j| {
                let mut fuel = Fuel(self.opts.fuel);
                quote_eq(&vals[j], &self.target[j], 0, &mut fuel).unwrap_or(false)
            });
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
        let mut h = DefaultHasher::new();
        if quote_hash(&v, 0, &mut fuel, &mut h).is_err() {
            return Outcome {
                solution: None,
                stats: Stats::default(),
            };
        }
        target_hash.push(h.finish());
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

            // Variables and library seeds.
            if s == 1 {
                let mut atoms: Vec<Rc<Term>> = (0..(k + c)).map(var).collect();
                atoms.extend(opts.seeds.iter().cloned());
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
                        for a in &aso {
                            let t = app(f.term.clone(), a.clone());
                            let r = search.process(
                                c,
                                t,
                                Make::Apply(f, ArgSrc::Thunk(a.clone())),
                                &mut kept,
                                &mut opq,
                            );
                            step!(search, r);
                        }
                    }
                    for f in &fo {
                        for a in asn.iter().map(|e| &e.term).chain(aso.iter()) {
                            let t = app(f.clone(), a.clone());
                            let r = search.process(c, t, Make::Eval, &mut kept, &mut opq);
                            step!(search, r);
                        }
                    }
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
