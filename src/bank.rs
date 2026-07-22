//! The bank: bottom-up enumeration of λ-terms with behavioral deduplication.
//!
//! Terms are enumerated by size, per binder-context. A term at context `c`
//! has free variables for the task's `k` arguments plus `c` enclosing λ
//! binders. Its *key* is the vector of its normal forms under each test's
//! environment (arguments bound to the actual test inputs, context binders
//! left as free constants). Two terms with the same key are interchangeable
//! in every context for these tests, so only one representative survives —
//! this is the superposition, implemented as a hash table.

use crate::nbe::{normalize, thunk_of_val, Env, Fuel, Head, Thunk, Val};
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
}

impl Default for Options {
    fn default() -> Self {
        Options {
            max_size: 14,
            max_depth: 3,
            fuel: 20_000,
            time_budget_secs: 60.0,
            max_level_entries: 200_000,
        }
    }
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

struct Level {
    /// terms[s] = deduped terms of size s at this context (s is 1-based).
    terms: Vec<Vec<Rc<Term>>>,
    seen: HashSet<u64>,
    envs: Vec<Env>, // one environment per test
}

struct Search<'a> {
    opts: &'a Options,
    start: Instant,
    k: u32,
    n_tests: usize,
    target: Vec<Rc<Term>>,
    levels: Vec<Level>,
    stats: Stats,
}

enum Step {
    Kept(bool), // true if the term entered the bank
    Solved(Rc<Term>),
    OutOfTime,
}

impl<'a> Search<'a> {
    /// Evaluate a candidate at context `c`: dedup it into the bank, and at
    /// context 0 check it against the target output vector.
    fn process(&mut self, c: u32, t: Rc<Term>, kept: &mut Vec<Rc<Term>>) -> Step {
        self.stats.built += 1;
        if self.stats.built % 4096 == 0
            && self.start.elapsed().as_secs_f64() > self.opts.time_budget_secs
        {
            return Step::OutOfTime;
        }
        let lvl = &self.levels[c as usize];
        let mut key: Vec<Rc<Term>> = Vec::with_capacity(self.n_tests);
        for j in 0..self.n_tests {
            let mut fuel = Fuel(self.opts.fuel);
            match normalize(&lvl.envs[j], &t, &mut fuel) {
                Ok(nf) => key.push(nf),
                Err(_) => {
                    self.stats.aborted += 1;
                    return Step::Kept(false);
                }
            }
        }
        if c == 0 && key == self.target {
            let mut sol = t;
            for _ in 0..self.k {
                sol = lam(sol);
            }
            return Step::Solved(sol);
        }
        let mut h = DefaultHasher::new();
        key.hash(&mut h);
        let kh = h.finish();
        if self.levels[c as usize].seen.insert(kh) && kept.len() < self.opts.max_level_entries {
            kept.push(t);
            self.stats.kept += 1;
            Step::Kept(true)
        } else {
            Step::Kept(false)
        }
    }
}

/// Search for `@main` for this task. Returns the full closed solution term
/// (already wrapped in the k argument lambdas) if found within budget.
pub fn solve(task: &Task, opts: &Options) -> Outcome {
    let start = Instant::now();
    let k = task.arity as u32;
    let n_tests = task.tests.len();
    let empty: Env = Rc::new(Vec::new());

    // Normalize expected outputs (the harness does the same with lam).
    let mut target: Vec<Rc<Term>> = Vec::with_capacity(n_tests);
    for t in &task.tests {
        let mut fuel = Fuel(opts.fuel);
        let stripped = normalize(&empty, &t.want, &mut fuel)
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

    // Shared, memoized argument thunks: each test input is evaluated at most
    // once across the entire search. Context binders are free-constant
    // neutrals shared across environments.
    let arg_thunks: Vec<Vec<Thunk>> = task
        .tests
        .iter()
        .map(|t| {
            t.args
                .iter()
                .map(|a| crate::nbe::thunk_delayed(empty.clone(), a.clone()))
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
                terms: vec![Vec::new()], // index 0 unused
                seen: HashSet::new(),
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
        levels,
        stats: Stats::default(),
    };

    for s in 1..=opts.max_size {
        search.stats.reached_size = s;
        for c in 0..=opts.max_depth {
            let mut kept: Vec<Rc<Term>> = Vec::new();
            let mut finish = |search: &mut Search, sol: Option<Rc<Term>>| {
                search.stats.elapsed_secs = search.start.elapsed().as_secs_f64();
                Outcome {
                    solution: sol,
                    stats: std::mem::take(&mut search.stats),
                }
            };

            // Variables.
            if s == 1 {
                for i in 0..(k + c) {
                    match search.process(c, var(i), &mut kept) {
                        Step::Solved(sol) => return finish(&mut search, Some(sol)),
                        Step::OutOfTime => return finish(&mut search, None),
                        Step::Kept(_) => {}
                    }
                }
            }
            // Lambdas: wrap bodies from context c+1, size s-1.
            if s >= 2 && c + 1 <= opts.max_depth {
                let bodies: Vec<Rc<Term>> = search.levels[(c + 1) as usize]
                    .terms
                    .get((s - 1) as usize)
                    .cloned()
                    .unwrap_or_default();
                for b in bodies {
                    match search.process(c, lam(b), &mut kept) {
                        Step::Solved(sol) => return finish(&mut search, Some(sol)),
                        Step::OutOfTime => return finish(&mut search, None),
                        Step::Kept(_) => {}
                    }
                }
            }
            // Applications: f from (c, s1), a from (c, s2), s1 + s2 = s - 1.
            if s >= 3 {
                for s1 in 1..=(s - 2) {
                    let s2 = s - 1 - s1;
                    let lvl = &search.levels[c as usize];
                    let fs: Vec<Rc<Term>> =
                        lvl.terms.get(s1 as usize).cloned().unwrap_or_default();
                    let args: Vec<Rc<Term>> =
                        lvl.terms.get(s2 as usize).cloned().unwrap_or_default();
                    for f in &fs {
                        for a in &args {
                            match search.process(c, app(f.clone(), a.clone()), &mut kept) {
                                Step::Solved(sol) => return finish(&mut search, Some(sol)),
                                Step::OutOfTime => return finish(&mut search, None),
                                Step::Kept(_) => {}
                            }
                        }
                    }
                }
            }

            let lvl = &mut search.levels[c as usize];
            while lvl.terms.len() <= s as usize {
                lvl.terms.push(Vec::new());
            }
            lvl.terms[s as usize] = kept;
        }
    }

    search.stats.elapsed_secs = start.elapsed().as_secs_f64();
    Outcome {
        solution: None,
        stats: search.stats,
    }
}
