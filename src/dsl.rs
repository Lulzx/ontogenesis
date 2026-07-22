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
}

pub static LIBRARY: RwLock<Vec<LibEntry>> = RwLock::new(Vec::new());

pub fn lib_len() -> usize {
    LIBRARY.read().unwrap().len()
}

pub fn lib_arity(i: u16) -> usize {
    LIBRARY.read().unwrap().get(i as usize).map_or(0, |e| e.arity)
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
        out.push_str(&format!("{} {}\n", e.arity, print_e(&e.body)));
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
        let (k, body) = line.split_once(' ').ok_or_else(|| format!("bad lib line: {line}"))?;
        let arity: usize = k.parse().map_err(|_| format!("bad arity: {line}"))?;
        let body = parse_e(body).ok_or_else(|| format!("bad body: {line}"))?;
        lib.push(LibEntry { arity, body });
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
            lib.push(LibEntry { arity: pat.k as usize, body });
            (lib.len() - 1) as u16
        };
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
    fn mine_extracts_and_expansion_agrees() {
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
