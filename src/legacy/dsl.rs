//! DSL-term persistence, the mined-abstraction library, and the miner.
//!
//! A library entry is a k-parameter abstraction over the semantic DSL:
//! body params are Var(0..k-1); the d-th Lam1 param inside the body is
//! Var(k+d) (positional env indexing, like task terms). Entries are stored
//! Lib-free — expanded against earlier entries at insert — so evaluation
//! and expansion never recurse through the library.
//!
//! Mining is Stitch-lite compression: canonicalize every Prim-rooted
//! subterm of the solved corpus by abstracting its outer variables into
//! hole parameters, count identical patterns across tasks, and take the
//! pattern with the best compression gain. The corpus is rewritten with
//! the new Lib op and the process repeats.

use crate::decode::V;
use crate::sem::{E, Op};
use std::sync::RwLock;

pub struct LibEntry {
    pub arity: usize,
    /// Lib-free body: Var(0..arity-1) are params, Var(arity+d) internal
    /// lambda params.
    pub body: E,
    /// Held-out-validation verdict (see validate_entry), persisted as a
    /// trailing comment in the library file.
    pub note: String,
}

pub static LIBRARY: RwLock<Vec<LibEntry>> = RwLock::new(Vec::new());

pub fn lib_len() -> usize {
    LIBRARY.read().unwrap().len()
}

pub fn lib_arity(i: u16) -> usize {
    LIBRARY.read().unwrap().get(i as usize).map_or(0, |e| e.arity)
}

pub fn lib_note(i: u16) -> String {
    LIBRARY
        .read()
        .unwrap()
        .get(i as usize)
        .map_or(String::new(), |e| e.note.clone())
}

pub fn lib_ops() -> Vec<Op> {
    (0..lib_len() as u16).map(Op::Lib).collect()
}

/// Evaluate library entry `i` applied to already-evaluated argument values.
/// Bodies are Lib-free, so eval never re-enters this function.
pub fn eval_lib(i: u16, vals: &[V]) -> Option<V> {
    let lib = LIBRARY.read().unwrap();
    let entry = lib.get(i as usize)?;
    crate::sem::eval(&entry.body, vals)
}

// ── S-expression printer / parser ───────────────────────────────────

pub fn print_e(e: &E) -> String {
    match e {
        E::Var(i) => format!("${i}"),
        E::KNat(n) => format!("#{n}"),
        E::Lam1(b) => format!("(lam {})", print_e(b)),
        E::Prim(Op::Lib(i), args) => print_call(&format!("L{i}"), args),
        E::Prim(op, args) => print_call(&format!("{op:?}"), args),
    }
}

fn print_call(name: &str, args: &[E]) -> String {
    let mut s = format!("({name}");
    for a in args {
        s.push(' ');
        s.push_str(&print_e(a));
    }
    s.push(')');
    s
}

fn op_by_name(s: &str) -> Option<Op> {
    if let Some(rest) = s.strip_prefix('L') {
        if let Ok(i) = rest.parse::<u16>() {
            return Some(Op::Lib(i));
        }
    }
    Op::all()
        .iter()
        .chain(Op::seed_extras())
        .copied()
        .find(|o| format!("{o:?}") == s)
}

pub fn parse_e(s: &str) -> Option<E> {
    let toks = tokenize(s);
    let mut pos = 0;
    let e = parse_tok(&toks, &mut pos)?;
    if pos == toks.len() {
        Some(e)
    } else {
        None
    }
}

fn tokenize(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '(' | ')' => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                out.push(c.to_string());
            }
            c if c.is_whitespace() => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn parse_tok(toks: &[String], pos: &mut usize) -> Option<E> {
    let t = toks.get(*pos)?.as_str();
    *pos += 1;
    match t {
        "(" => {
            let head = toks.get(*pos)?.clone();
            *pos += 1;
            if head == "lam" {
                let b = parse_tok(toks, pos)?;
                if toks.get(*pos)? != ")" {
                    return None;
                }
                *pos += 1;
                return Some(E::Lam1(Box::new(b)));
            }
            let op = op_by_name(&head)?;
            let mut args = Vec::new();
            while toks.get(*pos)? != ")" {
                args.push(parse_tok(toks, pos)?);
            }
            *pos += 1;
            Some(E::Prim(op, args))
        }
        ")" => None,
        v if v.starts_with('$') => Some(E::Var(v[1..].parse().ok()?)),
        v if v.starts_with('#') => Some(E::KNat(v[1..].parse().ok()?)),
        _ => None,
    }
}

// ── Library persistence ─────────────────────────────────────────────

pub fn save_library(path: &std::path::Path) -> std::io::Result<()> {
    let lib = LIBRARY.read().unwrap();
    let mut out = String::new();
    for e in lib.iter() {
        if e.note.is_empty() {
            out.push_str(&format!("{} {}\n", e.arity, print_e(&e.body)));
        } else {
            out.push_str(&format!("{} {}  // {}\n", e.arity, print_e(&e.body), e.note));
        }
    }
    std::fs::write(path, out)
}

pub fn load_library(path: &std::path::Path) -> Result<usize, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut lib = LIBRARY.write().unwrap();
    lib.clear();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let (line, note) = match line.split_once("  //") {
            Some((l, n)) => (l.trim(), n.trim().to_string()),
            None => (line, String::new()),
        };
        let (k, body) = line.split_once(' ').ok_or_else(|| format!("bad lib line: {line}"))?;
        let arity: usize = k.parse().map_err(|_| format!("bad arity: {line}"))?;
        let body = parse_e(body).ok_or_else(|| format!("bad body: {line}"))?;
        lib.push(LibEntry { arity, body, note });
    }
    Ok(lib.len())
}

// ── Expansion (Lib elimination before compile) ──────────────────────

/// Replace every Lib op by its body with args substituted. `n_args` is the
/// task arity of `e`'s context. Positional env indexing makes this a pure
/// renumbering: a body-internal lambda param k+d becomes n_args+depth+d at
/// a call site under `depth` lambdas; spliced args keep their indices.
pub fn expand(e: &E, n_args: usize) -> E {
    fn go(e: &E, n_args: usize, depth: u32) -> E {
        match e {
            E::Var(_) | E::KNat(_) => e.clone(),
            E::Lam1(b) => E::Lam1(Box::new(go(b, n_args, depth + 1))),
            E::Prim(Op::Lib(i), args) => {
                let args: Vec<E> = args.iter().map(|a| go(a, n_args, depth)).collect();
                let (arity, body) = {
                    let lib = LIBRARY.read().unwrap();
                    let entry = &lib[*i as usize];
                    (entry.arity, entry.body.clone())
                };
                subst(&body, arity, &args, n_args, depth)
            }
            E::Prim(op, args) => E::Prim(
                *op,
                args.iter().map(|a| go(a, n_args, depth)).collect(),
            ),
        }
    }
    fn subst(body: &E, k: usize, args: &[E], n_args: usize, depth: u32) -> E {
        match body {
            E::Var(j) if (*j as usize) < k => args[*j as usize].clone(),
            E::Var(j) => E::Var(n_args as u32 + depth + (j - k as u32)),
            E::KNat(_) => body.clone(),
            E::Lam1(b) => E::Lam1(Box::new(subst(b, k, args, n_args, depth))),
            E::Prim(op, bargs) => E::Prim(
                *op,
                bargs.iter().map(|a| subst(a, k, args, n_args, depth)).collect(),
            ),
        }
    }
    go(e, n_args, 0)
}

// ── Held-out validation ─────────────────────────────────────────────
//
// A finite test set can't distinguish the intended function from the
// smallest hypothesis consistent with it (the mined "gcd" surrogate
// s ← s − (b mod s) passes every benchmark pair yet disagrees with gcd
// on 31% of small inputs). So every mined entry is differentially tested
// on deterministic-random inputs against every hand-written op of the
// same arity:
//   ≡ Op   — agrees everywhere both are defined: a validated rediscovery
//   ~ Op   — best match agrees only partially: an overfit surrogate
//   novel  — matches nothing: a new abstraction (neither refuted nor named)

const VAL_SAMPLES: u64 = 200;
const VAL_MIN_OVERLAP: usize = 30;

/// Deterministic LCG so validation verdicts are reproducible.
fn lcg(x: &mut u64) -> u64 {
    *x = x
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *x >> 33
}

/// Random probe value: mostly small nats (where surrogates diverge),
/// sometimes short nat lists.
fn rand_val(x: &mut u64) -> V {
    match lcg(x) % 10 {
        0..=6 => V::Nat(lcg(x) % 48),
        7 => V::Nat(lcg(x) % 8),
        _ => {
            let len = (lcg(x) % 7) as usize;
            V::List((0..len).map(|_| V::Nat(lcg(x) % 20)).collect())
        }
    }
}

/// Differentially test library entry `idx` against the full op set and
/// return a human-readable verdict.
pub fn validate_entry(idx: u16) -> String {
    let (arity, body) = {
        let lib = LIBRARY.read().unwrap();
        let Some(e) = lib.get(idx as usize) else {
            return "missing entry".into();
        };
        (e.arity, e.body.clone())
    };
    // Sample once; reuse the same inputs for every candidate op.
    let mut seed = 0x5eed_0000 + idx as u64;
    let samples: Vec<Vec<V>> = (0..VAL_SAMPLES)
        .map(|_| (0..arity).map(|_| rand_val(&mut seed)).collect())
        .collect();
    let mine: Vec<Option<V>> = samples
        .iter()
        .map(|vals| crate::sem::eval(&body, vals))
        .collect();
    if mine.iter().all(|o| o.is_none()) {
        return "novel (undefined on random probes)".into();
    }

    // (op, agree, comparable, permuted) — mined holes are ordered by first
    // occurrence, so a rediscovery can carry the op's args in any order.
    let mut best: Option<(Op, usize, usize, bool)> = None;
    for op in Op::all().iter().chain(Op::seed_extras()) {
        let (va, la) = op.sig();
        if la != 0 || va != arity || matches!(op, Op::Lib(_)) {
            continue;
        }
        for perm in permutations(arity) {
            let call = E::Prim(*op, perm.iter().map(|i| E::Var(*i)).collect());
            let mut agree = 0usize;
            let mut comparable = 0usize;
            for (vals, m) in samples.iter().zip(&mine) {
                let theirs = crate::sem::eval(&call, vals);
                if let (Some(a), Some(b)) = (m, &theirs) {
                    comparable += 1;
                    if a == b {
                        agree += 1;
                    }
                }
            }
            let permuted = !perm.iter().enumerate().all(|(i, p)| i as u32 == *p);
            if comparable >= VAL_MIN_OVERLAP
                && best.map_or(true, |(_, ba, bc, _)| agree * bc > ba * comparable)
            {
                best = Some((*op, agree, comparable, permuted));
            }
        }
    }
    match best {
        Some((op, agree, comparable, permuted)) if agree == comparable => {
            let tag = if permuted { ", args reordered" } else { "" };
            format!("VALIDATED ≡ {op:?} ({comparable} random inputs{tag})")
        }
        Some((op, agree, comparable, _)) if agree * 100 >= comparable * 60 => {
            format!(
                "OVERFIT ~ {op:?} ({}% of {comparable} random inputs)",
                agree * 100 / comparable
            )
        }
        _ => "novel (matches no single op)".into(),
    }
}

fn permutations(n: usize) -> Vec<Vec<u32>> {
    if n == 0 {
        return vec![Vec::new()];
    }
    let mut out = Vec::new();
    for rest in permutations(n - 1) {
        for pos in 0..=rest.len() {
            let mut p = rest.clone();
            p.insert(pos, (n - 1) as u32);
            out.push(p);
        }
    }
    out
}

/// Validate every entry, store verdicts in their notes, and return them.
pub fn validate_all() -> Vec<(usize, String)> {
    let n = lib_len();
    let mut out = Vec::new();
    for i in 0..n {
        let v = validate_entry(i as u16);
        LIBRARY.write().unwrap()[i].note = v.clone();
        out.push((i, v));
    }
    out
}

// ── The miner ───────────────────────────────────────────────────────

pub struct CorpusEntry {
    pub id: String,
    pub n_args: u32,
    pub e: E,
}

/// A pattern: holes are Var(0..k-1), internal lambda params Var(k+d).
#[derive(Clone)]
struct Pat {
    k: u32,
    body: E,
}

/// Canonicalize a Prim-rooted subterm at env length `root_env`: outer vars
/// (index < root_env) become holes in first-occurrence order; vars bound by
/// Lam1s inside the subterm are renumbered to k+d. KNat stays concrete.
fn canonicalize(t: &E, root_env: u32) -> Pat {
    fn scan(e: &E, root_env: u32, outer: &mut Vec<u32>) {
        match e {
            E::Var(i) if *i < root_env => {
                if !outer.contains(i) {
                    outer.push(*i);
                }
            }
            E::Prim(_, args) => args.iter().for_each(|a| scan(a, root_env, outer)),
            E::Lam1(b) => scan(b, root_env, outer),
            _ => {}
        }
    }
    let mut outer = Vec::new();
    scan(t, root_env, &mut outer);
    let k = outer.len() as u32;
    fn rw(e: &E, root_env: u32, outer: &[u32], k: u32) -> E {
        match e {
            E::Var(i) if *i < root_env => {
                E::Var(outer.iter().position(|o| o == i).unwrap() as u32)
            }
            E::Var(i) => E::Var(k + (i - root_env)),
            E::KNat(_) => e.clone(),
            E::Lam1(b) => E::Lam1(Box::new(rw(b, root_env, outer, k))),
            E::Prim(op, args) => E::Prim(
                *op,
                args.iter().map(|a| rw(a, root_env, outer, k)).collect(),
            ),
        }
    }
    Pat { k, body: rw(t, root_env, &outer, k) }
}

/// True if every Var index in `e` is < `limit`. Rejects fillers that would
/// capture pattern-internal binders when hoisted to an argument position
/// (conservative: also rejects fillers with their own used lambda params).
fn vars_below(e: &E, limit: u32) -> bool {
    match e {
        E::Var(i) => *i < limit,
        E::KNat(_) => true,
        E::Lam1(b) => vars_below(b, limit),
        E::Prim(_, args) => args.iter().all(|a| vars_below(a, limit)),
    }
}

/// Match `pat` (holes < k) against `node` sitting at env length `l_root`.
/// Holes bind consistently; fillers must not reference anything bound at or
/// above `l_root` (they are hoisted to Lib-argument position).
fn match_pat(pat: &E, k: u32, node: &E, l_root: u32, binds: &mut Vec<Option<E>>) -> bool {
    match (pat, node) {
        (E::Var(j), n) if *j < k => {
            if let Some(prev) = &binds[*j as usize] {
                prev == n
            } else if vars_below(n, l_root) {
                binds[*j as usize] = Some(n.clone());
                true
            } else {
                false
            }
        }
        (E::Var(j), E::Var(i)) => *i == l_root + (*j - k),
        (E::KNat(a), E::KNat(b)) => a == b,
        (E::Lam1(a), E::Lam1(b)) => match_pat(a, k, b, l_root, binds),
        (E::Prim(o1, a1), E::Prim(o2, a2)) => {
            o1 == o2
                && a1.len() == a2.len()
                && a1
                    .iter()
                    .zip(a2)
                    .all(|(p, n)| match_pat(p, k, n, l_root, binds))
        }
        _ => false,
    }
}

/// Rewrite every match of `pat` in `e` (post-order) into Prim(Lib(idx), args).
fn rewrite(e: &E, env: u32, pat: &Pat, idx: u16) -> E {
    let node = match e {
        E::Var(_) | E::KNat(_) => e.clone(),
        E::Lam1(b) => E::Lam1(Box::new(rewrite(b, env + 1, pat, idx))),
        E::Prim(op, args) => E::Prim(
            *op,
            args.iter().map(|a| rewrite(a, env, pat, idx)).collect(),
        ),
    };
    if matches!(node, E::Prim(_, _)) {
        let mut binds = vec![None; pat.k as usize];
        if match_pat(&pat.body, pat.k, &node, env, &mut binds) {
            if let Some(args) = binds.into_iter().collect::<Option<Vec<E>>>() {
                return E::Prim(Op::Lib(idx), args);
            }
        }
    }
    node
}

const MAX_HOLES: u32 = 4;
const MAX_PAT_SIZE: u32 = 25;
const MIN_GAIN: i64 = 2;

/// One mining pass: repeatedly extract the best-gain pattern from the corpus
/// (up to `max_new` entries), appending each to LIBRARY and rewriting the
/// corpus in place. Returns (lib index, printed body) per new entry.
///
/// Gain model: an occurrence of pattern size S with k holes shrinks by
/// S - k - 1 nodes when replaced; storing the entry costs S. So
/// gain = count·(S - k - 1) - S, required ≥ MIN_GAIN with count ≥ 2.
pub fn mine_round(corpus: &mut [CorpusEntry], max_new: usize) -> Vec<(usize, String)> {
    let mut added = Vec::new();
    for _ in 0..max_new {
        use std::collections::HashMap;
        let mut counts: HashMap<String, (Pat, i64)> = HashMap::new();
        for entry in corpus.iter() {
            collect_subterms(&entry.e, entry.n_args, &mut |t, env| {
                if !matches!(t, E::Prim(_, _)) {
                    return;
                }
                let size = t.size();
                if !(3..=MAX_PAT_SIZE).contains(&size) {
                    return;
                }
                let pat = canonicalize(t, env);
                if pat.k > MAX_HOLES || size < pat.k + 2 {
                    return;
                }
                let key = format!("{}:{}", pat.k, print_e(&pat.body));
                counts.entry(key).or_insert_with(|| (pat, 0)).1 += 1;
            });
        }
        let existing: std::collections::HashSet<String> = {
            let lib = LIBRARY.read().unwrap();
            lib.iter().map(|e| print_e(&e.body)).collect()
        };
        let best = counts
            .into_values()
            .filter(|(p, n)| *n >= 2 && !existing.contains(&print_e(&p.body)))
            .map(|(p, n)| {
                let s = p.body.size() as i64;
                let gain = n * (s - p.k as i64 - 1) - s;
                (p, gain)
            })
            .max_by_key(|(_, g)| *g);
        let Some((pat, gain)) = best else { break };
        if gain < MIN_GAIN {
            break;
        }
        // Store Lib-free (patterns mined from a compressed corpus may
        // reference earlier entries).
        let body = expand(&pat.body, pat.k as usize);
        let idx = {
            let mut lib = LIBRARY.write().unwrap();
            lib.push(LibEntry { arity: pat.k as usize, body, note: String::new() });
            (lib.len() - 1) as u16
        };
        let verdict = validate_entry(idx);
        if let Some(e) = LIBRARY.write().unwrap().get_mut(idx as usize) {
            e.note = verdict;
        }
        for entry in corpus.iter_mut() {
            entry.e = rewrite(&entry.e, entry.n_args, &pat, idx);
        }
        added.push((idx as usize, print_e(&pat.body)));
    }
    added
}

fn collect_subterms(e: &E, env: u32, f: &mut impl FnMut(&E, u32)) {
    f(e, env);
    match e {
        E::Prim(_, args) => args.iter().for_each(|a| collect_subterms(a, env, f)),
        E::Lam1(b) => collect_subterms(b, env + 1, f),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem::eval;

    /// Both tests mutate the global LIBRARY; serialize them.
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn totient() -> E {
        // Count(Range1($0), lam Eq(Gcd(p, $0), 1)) with n_args = 1 (p = $1)
        E::Prim(
            Op::Count,
            vec![
                E::Prim(Op::Range1, vec![E::Var(0)]),
                E::Lam1(Box::new(E::Prim(
                    Op::Eq,
                    vec![
                        E::Prim(Op::Gcd, vec![E::Var(1), E::Var(0)]),
                        E::KNat(1),
                    ],
                ))),
            ],
        )
    }

    #[test]
    fn roundtrip() {
        let e = totient();
        let s = print_e(&e);
        assert_eq!(parse_e(&s).unwrap(), e);
        let nil = E::Prim(Op::Nil, vec![]);
        assert_eq!(parse_e(&print_e(&nil)).unwrap(), nil);
    }

    #[test]
    fn validation_separates_rediscovery_from_overfit() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // Body context: params $0,$1; internal lambda param $2.
        let pow = E::Prim(
            Op::Iter,
            vec![
                E::Var(0),
                E::KNat(1),
                E::Lam1(Box::new(E::Prim(Op::Mul, vec![E::Var(1), E::Var(2)]))),
            ],
        );
        // The benchmark-gcd surrogate: Iter(a, a, λs. s − (b mod s)).
        let gcd_like = E::Prim(
            Op::Iter,
            vec![
                E::Var(0),
                E::Var(0),
                E::Lam1(Box::new(E::Prim(
                    Op::Sub,
                    vec![
                        E::Var(2),
                        E::Prim(Op::Mod, vec![E::Var(1), E::Var(2)]),
                    ],
                ))),
            ],
        );
        let (vp, vg);
        {
            let mut lib = LIBRARY.write().unwrap();
            lib.clear();
            lib.push(LibEntry { arity: 2, body: pow, note: String::new() });
            lib.push(LibEntry { arity: 2, body: gcd_like, note: String::new() });
        }
        vp = validate_entry(0);
        vg = validate_entry(1);
        LIBRARY.write().unwrap().clear();
        assert!(vp.contains("VALIDATED") && vp.contains("Pow"), "{vp}");
        assert!(vg.contains("OVERFIT") && vg.contains("Gcd"), "{vg}");
    }

    #[test]
    fn mine_extracts_and_expansion_agrees() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // Two tasks share the totient shape; mining should abstract it and
        // the compressed term must evaluate identically after expansion.
        let mut corpus = vec![
            CorpusEntry { id: "a".into(), n_args: 1, e: totient() },
            CorpusEntry { id: "b".into(), n_args: 1, e: totient() },
        ];
        {
            LIBRARY.write().unwrap().clear();
        }
        let added = mine_round(&mut corpus, 4);
        assert!(!added.is_empty(), "no pattern extracted");
        let compressed = corpus[0].e.clone();
        assert!(print_e(&compressed).contains("(L"), "{}", print_e(&compressed));
        let expanded = expand(&compressed, 1);
        assert_eq!(expanded, totient());
        // Behavioral check through the Lib op itself.
        let got = eval(&compressed, &[V::Nat(10)]).unwrap();
        assert_eq!(got, V::Nat(4)); // φ(10) = 4
        LIBRARY.write().unwrap().clear();
    }
}
