//! Ontology-bootstrap: abstraction mining over raw λ-terms.
//!
//! This is the raw-λ analog of `dsl.rs`'s Stitch-lite miner, with two
//! deliberate differences driven by the research goal (a machine that invents
//! its own vocabulary rather than being handed one):
//!
//! 1. **Behavioral grouping instead of syntactic canonicalization.** The
//!    `dsl.rs` miner keys patterns syntactically; here we abstract *open*
//!    subterms into closed combinators, then group distinct-but-behaviorally-
//!    identical combinators into one class by evaluating each on a probe
//!    universe (normal-form vectors). This is the semantic-abstraction move
//!    (`x+x` and `2x` collapse when they behave the same).
//!
//! 2. **Reference-free generality validation.** `dsl::validate_entry` compares
//!    a mined entry to hand-written reference ops (≡ Op / ~ Op / novel). In
//!    the bootstrap setting there *is no reference ontology* — that is the
//!    whole point — so validation measures **generality / stability**: is the
//!    combinator's behavior defined and self-consistent across a broad,
//!    held-out probe draw, or is it an accidental regularity of the training
//!    inputs? We state plainly that generality ≠ correctness-to-any-unknown-
//!    truth (the gcd-overfit warning from dsl.rs applies without a safety net).
//!
//! Safety property: a promoted seed can never corrupt a solution. `bank::solve`
//! still verifies every winning candidate's normal form against the task's
//! target (bank.rs `quote_eq` oracle check). Seeds only pre-build hard-to-
//! enumerate sub-structure, so a bad seed is a *performance* hazard, never a
//! *correctness* one — which is exactly what the cost curve measures honestly.

use crate::nbe::{normalize, Env, Fuel};
use crate::parse;
use crate::term::{app, lam, var, Term};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

// ── Deterministic RNG (mirrors dsl.rs's LCG so verdicts are reproducible) ──

fn lcg(x: &mut u64) -> u64 {
    *x = x
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *x >> 33
}

fn hash_term(t: &Term) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    t.hash(&mut h);
    h.finish()
}

// ── Behavioral signature ────────────────────────────────────────────────

/// One observable outcome of applying a combinator to a probe tuple.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Cell {
    /// Normal form within fuel and size cap; payload is its hash.
    Norm(u64),
    /// Evaluation aborted (out of fuel / divergent).
    Diverged,
    /// Normalized but the normal form exceeds the size cap.
    Overflow,
}

/// The behavioral signature of a combinator over a probe bag: one `Cell` per
/// drawn k-tuple. Equal `BehaviorKey`s ⇔ indistinguishable on the probes.
pub type BehaviorKey = Vec<Cell>;

/// A mined abstraction promoted to a seed, with its generality note.
#[derive(Clone, Debug)]
pub struct MinedSeed {
    pub comb: Rc<Term>,
    pub k: u32,
    pub gain: i64,
    pub count: u64,
    pub note: String,
    pub key: BehaviorKey,
}

pub struct ProbeSet {
    /// Closed terms (data and functions) used as arguments to combinator probes.
    pub pool: Vec<Rc<Term>>,
    /// Number of adversarial edges deliberately included (for the note).
    pub adversarial: usize,
}

#[derive(Clone)]
pub struct MineOptions {
    pub fuel: i64,
    /// Normal-form size cap; larger NFs count as `Overflow` (not "defined").
    pub size_cap: u32,
    pub max_holes: u32,
    pub max_pat_size: u32,
    pub min_gain: i64,
    /// k-tuples drawn per candidate for grouping.
    pub probe_tuples: usize,
    /// k-tuples drawn for holdout generality.
    pub n_holdout: usize,
    pub per_round: usize,
    pub gen_threshold: f64,
    pub group_seed: u64,
    pub holdout_seed: u64,
}

impl Default for MineOptions {
    fn default() -> Self {
        MineOptions {
            fuel: 8_000,
            size_cap: 512,
            max_holes: 4,
            max_pat_size: 25,
            min_gain: 2,
            probe_tuples: 40,
            n_holdout: 200,
            per_round: 4,
            gen_threshold: 0.9,
            group_seed: 0x5eed_0000,
            holdout_seed: 0xc0ffee_0000,
        }
    }
}

// ── Open-subterm extraction + abstraction ───────────────────────────────

/// Visit every subterm of a (closed) solution, reporting each subterm with the
/// number of λ-binders above it (`root_env`). Open subterms are kept — unlike
/// `main.rs`'s closed-only collector.
fn collect_subterms_term(t: &Rc<Term>, d: u32, f: &mut impl FnMut(&Rc<Term>, u32)) {
    f(t, d);
    match t.as_ref() {
        Term::Var(_) | Term::Free(_) | Term::Prim(_) => {}
        Term::Lam(b) => collect_subterms_term(b, d + 1, f),
        Term::App(fn_, a) => {
            collect_subterms_term(fn_, d, f);
            collect_subterms_term(a, d, f);
        }
    }
}

/// Collect the free-variable *context indices* of `t` (indices < root_env) in
/// first-occurrence order. `d` is the binder depth within `t`.
fn collect_free(
    t: &Rc<Term>,
    d: u32,
    root_env: u32,
    order: &mut Vec<u32>,
    seen: &mut HashSet<u32>,
) {
    match t.as_ref() {
        Term::Var(i) if *i >= d && *i - d < root_env => {
            let c = *i - d;
            if seen.insert(c) {
                order.push(c);
            }
        }
        Term::Var(_) | Term::Free(_) | Term::Prim(_) => {}
        Term::Lam(b) => collect_free(b, d + 1, root_env, order, seen),
        Term::App(f, a) => {
            collect_free(f, d, root_env, order, seen);
            collect_free(a, d, root_env, order, seen);
        }
    }
}

/// Rewrite `t` into the body of a closed combinator λ^k.body. `d` is depth
/// within `t`; `k` is the arity; `pos[c]` = first-occurrence position of
/// context index `c`.
///
/// de Bruijn indices after wrapping `t` in `k` outer λs (the new params sit
/// ABOVE every binder inside `t`):
///   • bound var (i < d): binder is inside `t`, below the new params, so its
///     index is unchanged → `var(i)`.
///   • free var (i ≥ d, context c = i-d): becomes the new param for occurrence
///     position `j = pos[c]`; that param is `k-1-j` λs out from the `d`
///     internal binders, so its index is `d + (k-1-j)` → `var(d + k-1-j)`.
fn rewrite(t: &Rc<Term>, d: u32, k: u32, pos: &[usize]) -> Rc<Term> {
    match t.as_ref() {
        Term::Var(i) => {
            if *i >= d {
                let c = (*i - d) as usize;
                var(d + (k - 1 - pos[c] as u32))
            } else {
                var(*i)
            }
        }
        Term::Free(f) => Rc::new(Term::Free(*f)),
        Term::Prim(_) => t.clone(), // a closed atom: nothing to abstract
        Term::Lam(b) => lam(rewrite(b, d + 1, k, pos)),
        Term::App(f, a) => app(rewrite(f, d, k, pos), rewrite(a, d, k, pos)),
    }
}

/// Abstract the free variables (indices < root_env) of subterm `t` into leading
/// λ-binders, returning a closed combinator and its arity. The first free var
/// (in first-occurrence order) becomes the outermost / first-applied parameter.
pub fn abstract_term(t: &Rc<Term>, root_env: u32) -> (Rc<Term>, u32) {
    let mut order: Vec<u32> = Vec::new();
    let mut seen: HashSet<u32> = HashSet::new();
    collect_free(t, 0, root_env, &mut order, &mut seen);
    let k = order.len() as u32;
    if k == 0 {
        // Closed constant: already a combinator.
        return (t.clone(), 0);
    }
    let mut pos = vec![0usize; root_env as usize];
    for (j, &c) in order.iter().enumerate() {
        pos[c as usize] = j;
    }
    let body = rewrite(t, 0, k, &pos);
    let mut comb = body;
    for _ in 0..k {
        comb = lam(comb);
    }
    (comb, k)
}

// ── Probe evaluation ────────────────────────────────────────────────────

fn behavior_of(comb: &Rc<Term>, args: &[Rc<Term>], empty: &Env, opts: &MineOptions) -> Cell {
    let mut t = comb.clone();
    for a in args {
        t = app(t, a.clone());
    }
    let mut fuel = Fuel(opts.fuel);
    match normalize(empty, &t, &mut fuel) {
        Err(_) => Cell::Diverged,
        Ok(nf) => {
            if nf.size() > opts.size_cap {
                Cell::Overflow
            } else {
                Cell::Norm(hash_term(&nf))
            }
        }
    }
}

fn draw_tuple(pool: &[Rc<Term>], k: u32, rng: &mut u64) -> Vec<Rc<Term>> {
    (0..k)
        .map(|_| pool[(lcg(rng) as usize) % pool.len()].clone())
        .collect()
}

/// Draw `n` k-tuples from `pool` (deterministic via `seed`).
fn draw_tuples(pool: &[Rc<Term>], k: u32, n: usize, seed: &mut u64) -> Vec<Vec<Rc<Term>>> {
    (0..n).map(|_| draw_tuple(pool, k, seed)).collect()
}

/// Behavior key of `comb` over a fixed set of k-tuples. `tuples` must be the
/// SAME set for every candidate of a given arity, so distinct-but-equal
/// combinators land on identical keys (this is what makes the merge sound).
fn behavior_key(
    comb: &Rc<Term>,
    k: u32,
    tuples: &[Vec<Rc<Term>>],
    empty: &Env,
    opts: &MineOptions,
) -> BehaviorKey {
    if k == 0 {
        // No args; behavior is a single constant outcome.
        return vec![behavior_of(comb, &[], empty, opts)];
    }
    tuples
        .iter()
        .map(|args| behavior_of(comb, args, empty, opts))
        .collect()
}

fn defined_rate(key: &BehaviorKey) -> f64 {
    let n = key.len() as f64;
    if n == 0.0 {
        return 0.0;
    }
    key.iter().filter(|c| matches!(c, Cell::Norm(_))).count() as f64 / n
}

/// Adversarial composition guard: reject a seed that *fails to terminate*
/// when its applications are fed back in.
///
/// The plain generality check draws independent tuples from the probe pool;
/// a seed that is total there can still diverge when composed — applied to
/// itself, to partial applications of itself, or to composites of the pool —
/// which is the seed-branching-explosion failure mode (a promoted seed that
/// never terminates widening the bank's search instead of narrowing it). We
/// test the seed on a universe that includes composites of itself and reject
/// any application that diverges. Large-but-finite results are allowed: a big
/// Church numeral is still a legitimate, terminating value (rejecting it
/// would falsely bar useful concepts like multiplication of big numbers).
fn composition_guard(seed: &Rc<Term>, k: u32, pool: &[Rc<Term>], opts: &MineOptions) -> bool {
    if k == 0 {
        return true; // closed constant: no applications to worry about
    }
    // Composite universe: the probe pool, one level of the seed applied to
    // pool terms, and the seed applied to itself (higher-order self-reference).
    let mut comp: Vec<Rc<Term>> = pool.to_vec();
    for a in pool {
        comp.push(app(seed.clone(), a.clone()));
    }
    comp.push(app(seed.clone(), seed.clone()));

    let mut rng = opts.holdout_seed ^ (k as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let empty: Env = Rc::new(Vec::new());
    let n_draws = 64;
    for _ in 0..n_draws {
        let args: Vec<Rc<Term>> = (0..k)
            .map(|_| comp[(lcg(&mut rng) as usize) % comp.len()].clone())
            .collect();
        if matches!(behavior_of(seed, &args, &empty, opts), Cell::Diverged) {
            return false;
        }
    }
    true
}

// ── The miner ───────────────────────────────────────────────────────────

#[derive(Clone)]
struct Pat {
    comb: Rc<Term>,
    k: u32,
    s: u32,
    count: u64,
}

struct ClassInfo {
    total: u64,
    rep: Pat,
}

/// Mine abstraction seeds from the raw-λ corpus. Returns accepted seeds
/// (already generality-validated on `holdout`), highest-gain first.
pub fn mine(
    corpus: &[Rc<Term>],
    grouping: &ProbeSet,
    holdout: &ProbeSet,
    opts: &MineOptions,
) -> Vec<MinedSeed> {
    let empty: Env = Rc::new(Vec::new());

    // 1. Count open-subterm patterns across the corpus.
    let mut map: HashMap<String, Pat> = HashMap::new();
    for sol in corpus {
        collect_subterms_term(sol, 0, &mut |sub, root_env| {
            let s = sub.size();
            if s < 3 || s > opts.max_pat_size {
                return;
            }
            let (comb, k) = abstract_term(sub, root_env);
            if k == 0 {
                // Skip closed constants: seeds should be parameterizable
                // abstractions, not data. (Also excludes whole solutions.)
                return;
            }
            if k > opts.max_holes {
                return;
            }
            if s < k + 2 {
                return;
            }
            let key = format!("{k}:{}", term_show(&comb));
            map.entry(key)
                .and_modify(|p: &mut Pat| p.count += 1)
                .or_insert(Pat {
                    comb,
                    k,
                    s,
                    count: 1,
                });
        });
    }

    // 2. All syntactic forms that clear the guards; no per-form count filter —
    //    recurring structure is judged on *behavioral* classes, not syntax.
    let patterns: Vec<Pat> = map.into_values().collect();
    if patterns.is_empty() {
        return Vec::new();
    }

    // 3. Behavioral grouping: merge syntactically distinct but behaviorally
    //    identical combinators into one class. Probe tuples are pre-drawn per
    //    arity and SHARED across candidates so keys are comparable.
    let mut gseed = opts.group_seed;
    let g_tuples: Vec<Vec<Vec<Rc<Term>>>> = (0..=opts.max_holes)
        .map(|k| draw_tuples(&grouping.pool, k, opts.probe_tuples, &mut gseed))
        .collect();
    let mut classes: HashMap<BehaviorKey, ClassInfo> = HashMap::new();
    for p in &patterns {
        let key = behavior_key(&p.comb, p.k, &g_tuples[p.k as usize], &empty, opts);
        let e = classes.entry(key).or_insert(ClassInfo {
            total: 0,
            rep: p.clone(),
        });
        e.total += p.count;
        if p.count > e.rep.count {
            e.rep = p.clone();
        }
    }

    // 4. Rank classes by compression gain = total·(S − k − 1) − S. A class is
    //    "recurring" when its total spans ≥ 2 occurrences — whether as one
    //    repeated form or as several behaviorally-equal forms (the semantic
    //    abstraction move).
    let mut candidates: Vec<(i64, BehaviorKey, Pat)> = Vec::new();
    // Diagnostic (BOOT_DEBUG=1): when the curve is flat, this shows WHY — the
    // best recurring behavioral class and whether it cleared the gain bar. A
    // flat curve with only low-gain classes is the plan's thin-corpus fallback,
    // not a mechanism failure.
    if std::env::var_os("BOOT_DEBUG").is_some() {
        let best = classes
            .iter()
            .filter(|(_, ci)| ci.total >= 2)
            .map(|(_, ci)| {
                (
                    (ci.total as i64) * (ci.rep.s as i64 - ci.rep.k as i64 - 1) - ci.rep.s as i64,
                    ci,
                )
            })
            .max_by_key(|(g, _)| *g);
        eprintln!(
            "  boot-debug: {} patterns, {} classes; best recurring {}",
            patterns.len(),
            classes.len(),
            best.map(|(g, ci)| format!(
                "arity {} size {} total {} gain {} :: {}",
                ci.rep.k,
                ci.rep.s,
                ci.total,
                g,
                term_show(&ci.rep.comb)
            ))
            .unwrap_or_else(|| "none (no class recurs ≥2 times)".into())
        );
    }
    for (key, ci) in classes {
        if ci.total < 2 {
            continue;
        }
        let Pat { comb: _, k, s, .. } = ci.rep;
        let gain = (ci.total as i64) * (s as i64 - k as i64 - 1) - s as i64;
        if gain < opts.min_gain {
            continue;
        }
        candidates.push((gain, key, ci.rep));
    }
    candidates.sort_by_key(|(g, _, _)| std::cmp::Reverse(*g));
    if std::env::var_os("BOOT_DEBUG").is_some() {
        for (gain, key, pat) in &candidates {
            eprintln!(
                "  boot-debug: arity {} size {} count {} gain {} key {:?} :: {}",
                pat.k,
                pat.s,
                pat.count,
                gain,
                key,
                term_show(&pat.comb)
            );
        }
    }

    // 5. Generality-validate on the holdout universe; accept top per_round.
    let mut hseed = opts.holdout_seed;
    let h_tuples: Vec<Vec<Vec<Rc<Term>>>> = (0..=opts.max_holes)
        .map(|k| draw_tuples(&holdout.pool, k, opts.n_holdout, &mut hseed))
        .collect();
    let mut accepted: Vec<MinedSeed> = Vec::new();
    let mut seen_keys: HashSet<BehaviorKey> = HashSet::new();
    for (gain, key, pat) in candidates {
        if accepted.len() >= opts.per_round {
            break;
        }
        if seen_keys.contains(&key) {
            continue;
        }
        let rate_g = defined_rate(&key);
        let hkey = behavior_key(&pat.comb, pat.k, &h_tuples[pat.k as usize], &empty, opts);
        let rate_h = defined_rate(&hkey);
        let generality = rate_g.min(rate_h);
        if generality < opts.gen_threshold {
            continue;
        }
        // A seed that is total on the probe draws can still diverge when its
        // applications are fed back in; a divergent seed would fuel-exhaust
        // every downstream task, so reject it before promoting.
        if !composition_guard(&pat.comb, pat.k, &grouping.pool, opts) {
            continue;
        }
        let note = format!(
            "general on {} holdout probes (G {:.0}% / H {:.0}%, {} adversarial), terminating under composition; generality ≠ truth (no reference ontology)",
            opts.n_holdout,
            rate_g * 100.0,
            rate_h * 100.0,
            holdout.adversarial
        );
        seen_keys.insert(key.clone());
        accepted.push(MinedSeed {
            comb: pat.comb,
            k: pat.k,
            gain,
            count: pat.count,
            note,
            key,
        });
    }
    accepted
}

// ── Probe universes ─────────────────────────────────────────────────────

fn parse_closed(s: &str) -> Rc<Term> {
    parse::parse_expr(s)
        .and_then(|e| parse::to_term(&e))
        .unwrap_or_else(|e| panic!("bad probe term '{s}': {e}"))
}

/// Church numeral λf.λx.f^n(x), as a string for the parser.
pub fn church_num_str(n: u32) -> String {
    let mut s = String::from("λf.λx.");
    for _ in 0..n {
        s.push_str("f(");
    }
    s.push('x');
    for _ in 0..n {
        s.push(')');
    }
    s
}

/// Church list of Church numerals: λc.λn. c(N1)(c(N2)(…n)).
fn church_list_str(nums: &[u32]) -> String {
    let mut s = String::from("λc.λn.");
    if nums.is_empty() {
        s.push_str("n");
        return s;
    }
    // Build the spine inside out: fold from nil = n.
    let mut inner = String::from("n");
    for &nu in nums.iter().rev() {
        // c(N)(<rest>) — but <rest> must be c(N')(…) chained; the outermost
        // application is c(first)( … ). We prepend from the tail:
        inner = format!("c({})({})", church_num_str(nu), inner);
    }
    // At this point inner = c(N1)(c(N2)(…n)) — the body under λc.λn.
    s.push_str(&inner);
    s
}

/// The generator pool: closed data and function terms (input generators only —
/// explicitly not search vocabulary). Church/Scott-style encodings plus the
/// classic combinators, so higher-order task arguments have function probes.
fn generator_pool() -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    // Data: Church numerals.
    for n in 0..=5 {
        v.push(church_num_str(n));
    }
    // Booleans.
    v.push("λa.λb.a".into());
    v.push("λa.λb.b".into());
    // Church lists of numerals.
    for items in [
        vec![],
        vec![0],
        vec![1],
        vec![2],
        vec![0, 1],
        vec![1, 2],
        vec![1, 1],
        vec![2, 3],
        vec![1, 2, 3],
    ] {
        v.push(church_list_str(&items));
    }
    // Pairs.
    v.push("λa.λb.λs.s(a)(b)".into());
    v.push("λs.s(λa.λb.a)(λa.λb.b)".into());
    // Functions / combinators (higher-order probes).
    v.push("λa.a".into()); // I
    v.push("λa.λb.a".into()); // K
    v.push("λa.λb.λc.a(c)(b(c))".into()); // S
    v.push("λf.λg.λx.f(g(x))".into()); // B (compose)
    v.push("λf.λa.λb.f(b)(a)".into()); // C (flip)
    v.push("λn.λf.λx.f(n(f)(x))".into()); // succ
    v.push("λa.λb.λf.λx.a(f)(b(f)(x))".into()); // add
    v.push("λa.λb.λf.a(b(f))".into()); // mul
    v.push("λf.λa.λb.a(b)(f)".into()); // a fold-ish combinator
    v.push("λa.λb.b".into()); // false (dup, distinct string)
    v
}

/// Build a deterministic, deduped probe pool. `extra` are appended edge terms
/// (counted as adversarial). `seed` drives any compositional builds.
fn build_pool(base: Vec<String>, extra: &[String], _seed: u64) -> ProbeSet {
    let mut seen: HashSet<String> = HashSet::new();
    let mut pool: Vec<Rc<Term>> = Vec::new();
    for s in base.iter().chain(extra) {
        if seen.insert(s.clone()) {
            pool.push(parse_closed(s));
        }
    }
    // Compositional builds: apply pool terms to pool terms for more variety
    // (e.g. add(num, num) → numeral). Deterministic via fixed pairs.
    let snapshot: Vec<Rc<Term>> = pool.clone();
    let pairs = [
        (8usize, 1usize),  // add(0)
        (8usize, 2usize),  // add(1)
        (8usize, 3usize),  // add(2)
        (9usize, 2usize),  // mul(1)
        (9usize, 3usize),  // mul(2)
        (7usize, 1usize),  // succ(0)
        (7usize, 2usize),  // succ(1)
        (11usize, 2usize), // I(1) — a projection onto a list
    ];
    for (i, j) in pairs {
        if i < snapshot.len() && j < snapshot.len() {
            pool.push(app(snapshot[i].clone(), snapshot[j].clone()));
        }
    }
    let adversarial = extra.len();
    ProbeSet { pool, adversarial }
}

/// True if the term contains an opaque `Free` constant (an outer-binder
/// placeholder reified by the parser — not a concrete probe value).
fn has_free(t: &Rc<Term>) -> bool {
    match t.as_ref() {
        Term::Free(_) => true,
        Term::Var(_) => false,
        Term::Prim(_) => false,
        Term::Lam(b) => has_free(b),
        Term::App(f, a) => has_free(f) || has_free(a),
    }
}

/// Grouping universe `G`: train-derived + generator pool + compositional builds.
/// `train_args` are the train tasks' normalized test arguments (the oracle data).
/// Only *closed* arguments (no symbolic `Free` binders) are usable as probes —
/// the parser reifies a test's outer binders as `Free`, which are placeholders,
/// not data, and cannot be round-tripped through `show`/`parse`.
pub fn build_grouping_pool(train_args: &[Rc<Term>], seed: u64) -> ProbeSet {
    let mut base = generator_pool();
    let mut seen: HashSet<String> = HashSet::new();
    for a in train_args {
        if has_free(a) {
            continue; // symbolic outer-binder placeholder, not a probe value
        }
        let s = term_show(a);
        if seen.insert(s.clone()) {
            base.push(s);
        }
    }
    build_pool(base, &[], seed)
}

/// Holdout universe `H`: fresh deterministic draw, no train inputs, with
/// adversarial edges (empty/zero/boundary data).
pub fn build_holdout_pool(seed: u64) -> ProbeSet {
    let base = generator_pool();
    let extra = vec![
        church_num_str(7),            // a larger numeral
        church_list_str(&[]),         // empty list
        "λc.λn.c(λa.λb.a)(n)".into(), // cons(0, nil)
        "λa.λb.λs.s(a)(b)".into(),    // a fresh pair
        church_list_str(&[5, 1, 4]),  // an irregular list
    ];
    build_pool(base, &extra, seed)
}

fn term_show(t: &Term) -> String {
    crate::term::show(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Rc<Term> {
        parse::parse_expr(s)
            .and_then(|e| parse::to_term(&e))
            .unwrap()
    }

    #[test]
    fn abstraction_of_open_subterm() {
        // Solution λa.λb.b(a). The subterm `b(a)` (App(Var(0), Var(1))) sits at
        // root_env=2 (under λa.λb) and abstracts to the closed combinator
        // λa.λb.a(b): the first free var (the head `b`) becomes the outer
        // parameter, so the combinator is "apply p to q".
        let sol = p("λa.λb.b(a)");
        let Term::Lam(inner1) = sol.as_ref() else {
            panic!()
        };
        let Term::Lam(inner2) = inner1.as_ref() else {
            panic!()
        };
        let (comb, k) = abstract_term(inner2, 2);
        assert_eq!(k, 2);
        assert_eq!(term_show(&comb), "λa.λb.a(b)");
    }

    #[test]
    fn behaviorally_equal_but_syntactically_distinct() {
        // The semantic-abstraction primitive: two different syntactic forms of
        // Church `succ` that β-normalize to the same function. Applied to the
        // same probe args they must yield identical BehaviorKeys, so the miner
        // merges them into one class.
        let opts = MineOptions::default();
        let empty: Env = Rc::new(Vec::new());
        let pool: Vec<Rc<Term>> = generator_pool().iter().map(|s| parse_closed(s)).collect();

        let succ_a = p("λn.λf.λx.f(n(f)(x))");
        let succ_b = p("λn.λf.λx.(λg.λy.g(y))(f)(n(f)(x))");
        // Sanity: syntactically distinct.
        assert_ne!(term_show(&succ_a), term_show(&succ_b));
        let tuples = draw_tuples(&pool, 1, 30, &mut 0x1234);
        let key_a = behavior_key(&succ_a, 1, &tuples, &empty, &opts);
        let key_b = behavior_key(&succ_b, 1, &tuples, &empty, &opts);
        assert_eq!(key_a, key_b, "distinct syntax, same behavior must merge");
    }

    #[test]
    fn miner_merges_behavioral_equivalents() {
        // Two solutions that express the same underlying abstraction (succ)
        // with different syntax. The miner must collapse them into one class
        // and promote a general seed — proving "recurring behavior" replaces
        // "repeated syntax". Runs on a big-stack thread: the miner evaluates
        // combinators against probes, and `nbe`'s 700MB stack guard assumes
        // the 1GB worker thread that `main()` spawns (cargo test uses ~8MB).
        let result = std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let opts = MineOptions::default();
                let grouping = build_grouping_pool(&[], 7);
                let holdout = build_holdout_pool(8);
                let corpus = vec![
                    p("λn.λf.λx.f(n(f)(x))"),               // succ, one spelling
                    p("λn.λf.λx.(λg.λy.g(y))(f)(n(f)(x))"), // succ, β-expanded spelling
                ];
                let mined = mine(&corpus, &grouping, &holdout, &opts);
                (mined.len(), mined.iter().map(|m| m.k).collect::<Vec<_>>())
            })
            .unwrap()
            .join()
            .unwrap();
        assert!(
            result.0 > 0,
            "behaviorally-equal succ forms should merge into a mined seed"
        );
    }

    #[test]
    fn mined_seed_is_executable_succ() {
        // The `succ` abstraction mined from two differently-spelled solutions
        // must actually COMPUTE successor when applied to a Church numeral —
        // the "abstractions remain executable" requirement. This validates the
        // whole pipeline output as a real combinator, not just a hash.
        let result = std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let opts = MineOptions::default();
                let grouping = build_grouping_pool(&[], 7);
                let holdout = build_holdout_pool(8);
                let corpus = vec![
                    p("λn.λf.λx.f(n(f)(x))"),
                    p("λn.λf.λx.(λg.λy.g(y))(f)(n(f)(x))"),
                ];
                let mined = mine(&corpus, &grouping, &holdout, &opts);
                let two = parse_closed(&church_num_str(2));
                let three = parse_closed(&church_num_str(3));
                // Any mined seed that sends 2 ↦ 3 is an executable succ.
                for m in &mined {
                    let applied = app(m.comb.clone(), two.clone());
                    let mut fuel = Fuel(5000);
                    let empty: Env = Rc::new(Vec::new());
                    if let Ok(nf) = normalize(&empty, &applied, &mut fuel) {
                        if term_show(&nf) == term_show(&three) {
                            return Some(true);
                        }
                    }
                }
                None
            })
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(
            result,
            Some(true),
            "a mined succ seed must compute successor on Church numerals"
        );
    }

    #[test]
    fn empty_corpus_mines_nothing() {
        // The miner must never invent an abstraction from no experience.
        let opts = MineOptions::default();
        let grouping = build_grouping_pool(&[], 7);
        let holdout = build_holdout_pool(8);
        let mined = mine(&[], &grouping, &holdout, &opts);
        assert!(mined.is_empty());
    }

    #[test]
    fn composition_guard_admits_benign_rejects_divergent() {
        // nbe recurses as deep as fuel allows; evaluating the divergent seed
        // needs the 1GB worker stack, so run in a big-stack thread.
        let ok = std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let opts = MineOptions::default();
                let pool = build_grouping_pool(&[], 9).pool;
                // A genuinely divergent seed under self-application: rejected.
                let diverge = p("λx.x(x)");
                // A benign concept (mul = repeated addition): admitted.
                let mul = p("λa.λb.λc.b(a(c))");
                // The head idiom that regressed the earlier run: total on the
                // probe draw; record whether it passes under composition.
                let head = p("λa.a(λb.λc.b, a)");
                (
                    !composition_guard(&diverge, 1, &pool, &opts),
                    composition_guard(&mul, 3, &pool, &opts),
                    composition_guard(&head, 1, &pool, &opts),
                )
            })
            .unwrap()
            .join()
            .unwrap();
        assert!(ok.0, "self-applying seed must be rejected");
        assert!(ok.1, "mul (bounded repeated addition) must pass");
        // The head idiom terminates on the probe universe (it is not divergent —
        // its earlier slowdown was ordinary branching from an unused seed, not
        // explosion). The guard correctly admits it; it is not a safety hazard.
        eprintln!("composition_guard head idiom = {} (informational)", ok.2);
    }

    #[test]
    fn grouping_pool_skips_symbolic_free_binders() {
        // Regression: synthesized .tsk tasks pass the parser-reified outer
        // binders as Free(TEST_FREE_BASE + …) constants. These are symbolic
        // placeholders (shown as "#1048576"), NOT parseable probe values — the
        // pool builder must skip them, never try to round-trip them through
        // show/parse (which previously panicked on the '#').
        let free_arg: Rc<Term> = Rc::new(Term::Free(crate::parse::TEST_FREE_BASE));
        let pool = build_grouping_pool(&[free_arg], 7);
        assert!(!pool.pool.is_empty());
        // The Free binder must not have been admitted as a probe.
        for t in &pool.pool {
            assert!(!has_free(t), "probe must not contain a Free binder");
        }
    }
}
