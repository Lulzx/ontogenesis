//! Semantic-level synthesis: find the function connecting decoded native
//! I/O values, in a small typed DSL, by bottom-up enumeration with
//! behavioral dedup. This is where the missing bits come from: the search
//! happens in "gcd-space", not in λ-term space.

use crate::decode::V;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Op {
    Add,
    Sub, // saturating (monus) — matches λ-side truncated subtraction
    Mul,
    Div,
    Mod,
    Gcd,
    Pow,
    Isqrt,
    IlogB, // IlogB(n, b) = floor(log_b n), n ≥ 1, b ≥ 2
    Eq,
    Lt,
    Leq,
    IsZero,
    Not,
    If,
    Range1, // Range1(n) = {1..n} (order-insensitive uses only)
    Count,  // Count(list, λx.pred)
    // ── list-structure ops (elements are opaque) ──
    Head,
    Last,
    Nth, // Nth(list, i) — 0-based
    Rev,
    RotL,
    RotR,
    Len,
    AppendL,
    SortB,   // stable partition: falsy (Nat 0 / false) before true
    MapAp,   // MapAp(f, l) = [f(e) for e in l], f opaque
    ZipAp,   // ZipAp(f, a, b) = [f(x, y) pairwise]
    FoldrAp, // FoldrAp(f, z, l) = f(e1, f(e2, ... f(en, z)))
    // ── tree ops (perfect or arbitrary binary trees, leaf-valued) ──
    TFlat,   // depth-first leaves
    TBfs,    // breadth-first leaves (shallowest first)
    TMirror, // swap children recursively
    TBuild,  // perfect tree from list (len = 2^k)
    TMergeAp, // TMergeAp(f, t1, t2): same-shape zip, leaves f(x, y)
    TIdxAp,   // TIdxAp(f, t): leaves f(i, x), i = left-to-right index
    TScanAp,  // TScanAp(f, z, t): exclusive prefix fold over leaves
    TBitRev,  // bit-reversal permutation of leaves (perfect tree)
}

impl Op {
    /// (value-arity, lambda-arity). Lambda args are 1-parameter bodies.
    pub fn sig(self) -> (usize, usize) {
        match self {
            Op::Isqrt | Op::IsZero | Op::Not | Op::Range1 => (1, 0),
            Op::Head | Op::Last | Op::Rev | Op::RotL | Op::RotR | Op::Len | Op::SortB => (1, 0),
            Op::TFlat | Op::TBfs | Op::TMirror | Op::TBuild | Op::TBitRev => (1, 0),
            Op::If | Op::ZipAp | Op::FoldrAp | Op::TMergeAp | Op::TScanAp => (3, 0),
            Op::Count => (1, 1),
            _ => (2, 0),
        }
    }
    pub fn all() -> &'static [Op] {
        &[
            Op::Add,
            Op::Sub,
            Op::Mul,
            Op::Div,
            Op::Mod,
            Op::Gcd,
            Op::Pow,
            Op::Isqrt,
            Op::IlogB,
            Op::Eq,
            Op::Lt,
            Op::Leq,
            Op::IsZero,
            Op::Not,
            Op::If,
            Op::Range1,
            Op::Count,
            Op::Head,
            Op::Last,
            Op::Nth,
            Op::Rev,
            Op::RotL,
            Op::RotR,
            Op::Len,
            Op::AppendL,
            Op::SortB,
            Op::MapAp,
            Op::ZipAp,
            Op::FoldrAp,
            Op::TFlat,
            Op::TBfs,
            Op::TMirror,
            Op::TBuild,
            Op::TMergeAp,
            Op::TIdxAp,
            Op::TScanAp,
            Op::TBitRev,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum E {
    /// Task argument i, or (in a lambda body) the parameter at index k.
    Var(u32),
    KNat(u64),
    Prim(Op, Vec<E>),
    /// One-parameter lambda body; the parameter is Var(n_args) in the body.
    Lam1(Box<E>),
}

impl E {
    pub fn size(&self) -> u32 {
        match self {
            E::Var(_) | E::KNat(_) => 1,
            E::Prim(_, args) => 1 + args.iter().map(E::size).sum::<u32>(),
            E::Lam1(b) => b.size(), // the λ wrapper is free: it comes with the op
        }
    }
}

fn nat(v: &V) -> Option<u64> {
    match v {
        V::Nat(n) => Some(*n),
        _ => None,
    }
}
fn boolean(v: &V) -> Option<bool> {
    match v {
        V::Bool(b) => Some(*b),
        _ => None,
    }
}
fn list1(v: &V) -> Option<Vec<V>> {
    match v {
        V::List(xs) => Some(xs.clone()),
        _ => None,
    }
}

fn is_treeish(v: &V) -> bool {
    matches!(v, V::Node(_, _) | V::Atom(_) | V::App(_, _) | V::Nat(_) | V::Bool(_))
}

/// Depth-first leaves of a tree value (a non-Node value is a single leaf).
fn tleaves(v: &V) -> Option<Vec<V>> {
    if matches!(v, V::List(_) | V::Ctr(_, _)) {
        return None;
    }
    match v {
        V::Node(a, b) => {
            let mut l = tleaves(a)?;
            l.extend(tleaves(b)?);
            Some(l)
        }
        _ => Some(vec![v.clone()]),
    }
}

fn tbfs(v: &V) -> Option<Vec<V>> {
    if !is_treeish(v) {
        return None;
    }
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(v.clone());
    let mut out = Vec::new();
    while let Some(t) = queue.pop_front() {
        match t {
            V::Node(a, b) => {
                queue.push_back(*a);
                queue.push_back(*b);
            }
            leaf => out.push(leaf),
        }
    }
    Some(out)
}

fn tmirror(v: &V) -> Option<V> {
    if !is_treeish(v) {
        return None;
    }
    Some(match v {
        V::Node(a, b) => V::Node(Box::new(tmirror(b)?), Box::new(tmirror(a)?)),
        other => other.clone(),
    })
}

/// Perfect tree from a list of leaves (len must be a power of two ≥ 1).
fn tbuild(xs: &[V]) -> Option<V> {
    match xs.len() {
        0 => None,
        1 => Some(xs[0].clone()),
        n if n % 2 == 0 => {
            let (a, b) = xs.split_at(n / 2);
            Some(V::Node(Box::new(tbuild(a)?), Box::new(tbuild(b)?)))
        }
        _ => None,
    }
}

fn tmerge(f: &V, a: &V, b: &V) -> Option<V> {
    if !is_treeish(a) || !is_treeish(b) {
        return None;
    }
    match (a, b) {
        (V::Node(a1, a2), V::Node(b1, b2)) => Some(V::Node(
            Box::new(tmerge(f, a1, b1)?),
            Box::new(tmerge(f, a2, b2)?),
        )),
        (V::Node(_, _), _) | (_, V::Node(_, _)) => None,
        (x, y) => Some(V::App(
            Box::new(V::App(Box::new(f.clone()), Box::new(x.clone()))),
            Box::new(y.clone()),
        )),
    }
}

fn tidx(f: &V, v: &V, i: &mut u64) -> Option<V> {
    if !is_treeish(v) {
        return None;
    }
    match v {
        V::Node(a, b) => {
            let l = tidx(f, a, i)?;
            let r = tidx(f, b, i)?;
            Some(V::Node(Box::new(l), Box::new(r)))
        }
        leaf => {
            let out = V::App(
                Box::new(V::App(Box::new(f.clone()), Box::new(V::Nat(*i)))),
                Box::new(leaf.clone()),
            );
            *i += 1;
            Some(out)
        }
    }
}

fn tscan(f: &V, v: &V, acc: &mut V) -> Option<V> {
    if !is_treeish(v) {
        return None;
    }
    match v {
        V::Node(a, b) => {
            let l = tscan(f, a, acc)?;
            let r = tscan(f, b, acc)?;
            Some(V::Node(Box::new(l), Box::new(r)))
        }
        leaf => {
            let out = acc.clone();
            *acc = V::App(
                Box::new(V::App(Box::new(f.clone()), Box::new(acc.clone()))),
                Box::new(leaf.clone()),
            );
            Some(out)
        }
    }
}

const NAT_CAP: u64 = 1 << 40;
const LIST_CAP: usize = 4096;

/// Evaluate an expression under an environment (task args, then any lambda
/// params appended). Returns None on type mismatch / overflow / undefined.
pub fn eval(e: &E, env: &[V]) -> Option<V> {
    match e {
        E::Var(i) => env.get(*i as usize).cloned(),
        E::KNat(n) => Some(V::Nat(*n)),
        E::Lam1(_) => None, // lambdas only appear in op positions
        E::Prim(op, args) => {
            use Op::*;
            match op {
                If => {
                    let c = boolean(&eval(&args[0], env)?)?;
                    eval(&args[if c { 1 } else { 2 }], env)
                }
                Count => {
                    let V::List(xs) = eval(&args[0], env)? else {
                        return None;
                    };
                    let E::Lam1(body) = &args[1] else { return None };
                    let mut n = 0u64;
                    let mut env2 = env.to_vec();
                    env2.push(V::Nat(0));
                    for x in xs {
                        *env2.last_mut().unwrap() = x;
                        if boolean(&eval(body, &env2)?)? {
                            n += 1;
                        }
                    }
                    Some(V::Nat(n))
                }
                Range1 => {
                    let n = nat(&eval(&args[0], env)?)?;
                    if n as usize > LIST_CAP {
                        return None;
                    }
                    Some(V::List((1..=n).map(V::Nat).collect()))
                }
                Head => list1(&eval(&args[0], env)?)?.first().cloned(),
                Last => list1(&eval(&args[0], env)?)?.last().cloned(),
                Nth => {
                    let xs = list1(&eval(&args[0], env)?)?;
                    let i = nat(&eval(&args[1], env)?)? as usize;
                    xs.get(i).cloned()
                }
                Rev => {
                    let mut xs = list1(&eval(&args[0], env)?)?;
                    xs.reverse();
                    Some(V::List(xs))
                }
                RotL => {
                    let mut xs = list1(&eval(&args[0], env)?)?;
                    if !xs.is_empty() {
                        xs.rotate_left(1);
                    }
                    Some(V::List(xs))
                }
                RotR => {
                    let mut xs = list1(&eval(&args[0], env)?)?;
                    if !xs.is_empty() {
                        xs.rotate_right(1);
                    }
                    Some(V::List(xs))
                }
                Len => Some(V::Nat(list1(&eval(&args[0], env)?)?.len() as u64)),
                AppendL => {
                    let mut a = list1(&eval(&args[0], env)?)?;
                    a.extend(list1(&eval(&args[1], env)?)?);
                    Some(V::List(a))
                }
                SortB => {
                    let xs = list1(&eval(&args[0], env)?)?;
                    let (t, f): (Vec<V>, Vec<V>) = xs
                        .into_iter()
                        .partition(|v| matches!(v, V::Bool(true)));
                    let mut out = f;
                    out.extend(t);
                    Some(V::List(out))
                }
                MapAp => {
                    let f = eval(&args[0], env)?;
                    let xs = list1(&eval(&args[1], env)?)?;
                    Some(V::List(
                        xs.into_iter()
                            .map(|x| V::App(Box::new(f.clone()), Box::new(x)))
                            .collect(),
                    ))
                }
                ZipAp => {
                    let f = eval(&args[0], env)?;
                    let a = list1(&eval(&args[1], env)?)?;
                    let b = list1(&eval(&args[2], env)?)?;
                    Some(V::List(
                        a.into_iter()
                            .zip(b)
                            .map(|(x, y)| {
                                V::App(
                                    Box::new(V::App(Box::new(f.clone()), Box::new(x))),
                                    Box::new(y),
                                )
                            })
                            .collect(),
                    ))
                }
                FoldrAp => {
                    let f = eval(&args[0], env)?;
                    let z = eval(&args[1], env)?;
                    let xs = list1(&eval(&args[2], env)?)?;
                    let mut acc = z;
                    for x in xs.into_iter().rev() {
                        acc = V::App(
                            Box::new(V::App(Box::new(f.clone()), Box::new(x))),
                            Box::new(acc),
                        );
                    }
                    Some(acc)
                }
                TFlat => Some(V::List(tleaves(&eval(&args[0], env)?)?)),
                TBfs => Some(V::List(tbfs(&eval(&args[0], env)?)?)),
                TMirror => tmirror(&eval(&args[0], env)?),
                TBuild => tbuild(&list1(&eval(&args[0], env)?)?),
                TMergeAp => {
                    let f = eval(&args[0], env)?;
                    tmerge(&f, &eval(&args[1], env)?, &eval(&args[2], env)?)
                }
                TIdxAp => {
                    let f = eval(&args[0], env)?;
                    let mut i = 0u64;
                    tidx(&f, &eval(&args[1], env)?, &mut i)
                }
                TScanAp => {
                    let f = eval(&args[0], env)?;
                    let mut acc = eval(&args[1], env)?;
                    tscan(&f, &eval(&args[2], env)?, &mut acc)
                }
                TBitRev => {
                    let t = eval(&args[0], env)?;
                    let leaves = tleaves(&t)?;
                    let n = leaves.len();
                    if n == 0 || (n & (n - 1)) != 0 {
                        return None;
                    }
                    let bits = n.trailing_zeros();
                    let mut out = leaves.clone();
                    for (i, v) in leaves.into_iter().enumerate() {
                        let mut j = 0usize;
                        for b in 0..bits {
                            if i >> b & 1 == 1 {
                                j |= 1 << (bits - 1 - b);
                            }
                        }
                        out[j] = v;
                    }
                    tbuild(&out)
                }
                IsZero => Some(V::Bool(nat(&eval(&args[0], env)?)? == 0)),
                Not => Some(V::Bool(!boolean(&eval(&args[0], env)?)?)),
                Isqrt => {
                    let n = nat(&eval(&args[0], env)?)?;
                    Some(V::Nat(n.isqrt()))
                }
                _ => {
                    let a = nat(&eval(&args[0], env)?)?;
                    let b = nat(&eval(&args[1], env)?)?;
                    let r = match op {
                        Add => a.checked_add(b)?,
                        Sub => a.saturating_sub(b),
                        Mul => a.checked_mul(b)?,
                        Div => a.checked_div(b)?,
                        Mod => a.checked_rem(b)?,
                        Gcd => gcd(a, b),
                        Pow => {
                            if b > 64 && a > 1 {
                                return None;
                            }
                            a.checked_pow(b.try_into().ok()?)?
                        }
                        IlogB => {
                            if a == 0 || b < 2 {
                                return None;
                            }
                            let mut l = 0u64;
                            let mut x = a;
                            while x >= b {
                                x /= b;
                                l += 1;
                            }
                            l
                        }
                        Eq => return Some(V::Bool(a == b)),
                        Lt => return Some(V::Bool(a < b)),
                        Leq => return Some(V::Bool(a <= b)),
                        _ => unreachable!(),
                    };
                    if r > NAT_CAP {
                        return None;
                    }
                    Some(V::Nat(r))
                }
            }
        }
    }
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

pub struct SemOptions {
    pub max_size: u32,
    pub max_body_size: u32,
    pub max_entries: usize,
}

impl Default for SemOptions {
    fn default() -> Self {
        SemOptions {
            max_size: 10,
            max_body_size: 6,
            max_entries: 200_000,
        }
    }
}

/// Find an expression over the task args matching all (inputs, output)
/// examples. `inputs[j]` are test j's decoded arguments.
pub fn solve(inputs: &[Vec<V>], outputs: &[V], opts: &SemOptions) -> Option<E> {
    let n_args = inputs.first()?.len();
    let n_tests = inputs.len();

    // Restrict the op set to what the task's value shapes can possibly use:
    // arithmetic needs nats, structure ops need lists, application ops need
    // opaque atoms. This is type pruning at its crudest and buys the most.
    fn contains_kind(v: &V, f: &dyn Fn(&V) -> bool) -> bool {
        if f(v) {
            return true;
        }
        match v {
            V::List(xs) => xs.iter().any(|x| contains_kind(x, f)),
            V::App(a, b) => contains_kind(a, f) || contains_kind(b, f),
            V::Node(a, b) => contains_kind(a, f) || contains_kind(b, f),
            V::Ctr(_, xs) => xs.iter().any(|x| contains_kind(x, f)),
            _ => false,
        }
    }
    let all_vals = || inputs.iter().flatten().chain(outputs.iter());
    let has_nat = all_vals().any(|v| matches!(v, V::Nat(_)));
    let has_list = all_vals().any(|v| matches!(v, V::List(_)));
    let has_atom =
        all_vals().any(|v| contains_kind(v, &|x| matches!(x, V::Atom(_) | V::App(_, _))));
    let has_tree = all_vals().any(|v| contains_kind(v, &|x| matches!(x, V::Node(_, _))));
    let ops: Vec<Op> = Op::all()
        .iter()
        .copied()
        .filter(|op| {
            use Op::*;
            match op {
                TFlat | TBfs | TMirror | TBuild | TMergeAp | TIdxAp | TScanAp | TBitRev => {
                    has_tree
                }
                MapAp | ZipAp | FoldrAp => has_atom && (has_list || has_tree),
                Head | Last | Nth | Rev | RotL | RotR | Len | AppendL | SortB => {
                    has_list || has_tree
                }
                Range1 | Count => has_nat,
                If | Eq | Not => true,
                _ => has_nat,
            }
        })
        .collect();

    // ── Body bank: 1-param lambda bodies keyed on probe values ─────────
    // Probes: for each test, plausible values the parameter can take
    // (elements of Range1 of nat args, capped). Approximate but sound:
    // the final composed candidate is always verified on the real tests.
    let mut probes: Vec<(usize, V)> = Vec::new();
    for (j, inp) in inputs.iter().enumerate() {
        let mut vals: Vec<u64> = vec![0, 1, 2];
        for v in inp {
            if let V::Nat(n) = v {
                for d in 1..=(*n).min(12) {
                    vals.push(d);
                }
                vals.push(*n);
            }
            if let V::List(xs) = v {
                for x in xs.iter().take(8) {
                    if let V::Nat(n) = x {
                        vals.push(*n);
                    }
                }
            }
        }
        vals.sort_unstable();
        vals.dedup();
        for n in vals {
            probes.push((j, V::Nat(n)));
        }
    }

    let key_hash = |outs: &[Option<V>]| -> u64 {
        let mut h = DefaultHasher::new();
        outs.hash(&mut h);
        h.finish()
    };

    // Bodies: env = task args of the probe's test + the param value.
    let body_eval = |e: &E| -> Vec<Option<V>> {
        probes
            .iter()
            .map(|(j, p)| {
                let mut env = inputs[*j].clone();
                env.push(p.clone());
                eval(e, &env)
            })
            .collect()
    };

    let mut bodies: Vec<Vec<E>> = vec![Vec::new()]; // by size
    let mut bodies_seen: HashSet<u64> = HashSet::new();
    let param = n_args as u32;

    for s in 1..=opts.max_body_size {
        let mut level: Vec<E> = Vec::new();
        let mut push = |e: E, level: &mut Vec<E>, seen: &mut HashSet<u64>| {
            let outs = body_eval(&e);
            if outs.iter().all(|o| o.is_none()) {
                return;
            }
            if seen.insert(key_hash(&outs)) && level.len() < opts.max_entries {
                level.push(e);
            }
        };
        if s == 1 {
            for i in 0..=n_args as u32 {
                push(E::Var(i), &mut level, &mut bodies_seen);
            }
            push(E::Var(param), &mut level, &mut bodies_seen);
            for k in [0u64, 1, 2] {
                push(E::KNat(k), &mut level, &mut bodies_seen);
            }
        } else {
            for op in &ops {
                let (va, la) = op.sig();
                if la > 0 {
                    continue; // no nested lambdas in bodies
                }
                enumerate_args(&bodies, s - 1, va, &mut |args| {
                    push(E::Prim(*op, args.to_vec()), &mut level, &mut bodies_seen);
                });
            }
        }
        bodies.push(level);
    }

    // ── Main bank: expressions over the task args ───────────────────────
    let main_eval = |e: &E| -> Vec<Option<V>> {
        inputs.iter().map(|inp| eval(e, inp)).collect()
    };
    let target: Vec<Option<V>> = outputs.iter().map(|v| Some(v.clone())).collect();

    let mut main: Vec<Vec<E>> = vec![Vec::new()];
    let mut main_seen: HashSet<u64> = HashSet::new();

    for s in 1..=opts.max_size {
        let mut level: Vec<E> = Vec::new();
        let mut found: Option<E> = None;
        let mut push = |e: E, level: &mut Vec<E>, seen: &mut HashSet<u64>| -> Option<E> {
            let outs = main_eval(&e);
            if outs == target {
                return Some(e);
            }
            if outs.iter().all(|o| o.is_none()) {
                return None;
            }
            if seen.insert(key_hash(&outs)) && level.len() < opts.max_entries {
                level.push(e);
            }
            None
        };
        if s == 1 {
            for i in 0..n_args as u32 {
                if let Some(e) = push(E::Var(i), &mut level, &mut main_seen) {
                    return Some(e);
                }
            }
            for k in [0u64, 1, 2] {
                if let Some(e) = push(E::KNat(k), &mut level, &mut main_seen) {
                    return Some(e);
                }
            }
        } else {
            'ops: for op in &ops {
                let (va, la) = op.sig();
                if la == 0 {
                    enumerate_args(&main, s - 1, va, &mut |args| {
                        if found.is_none() {
                            if let Some(e) =
                                push(E::Prim(*op, args.to_vec()), &mut level, &mut main_seen)
                            {
                                found = Some(e);
                            }
                        }
                    });
                } else {
                    // ops with one value arg + one lambda arg (Count)
                    for s1 in 1..(s - 1) {
                        let s2 = s - 1 - s1;
                        if main.len() <= s1 as usize || bodies.len() <= s2 as usize {
                            continue;
                        }
                        for l in main[s1 as usize].clone() {
                            for b in &bodies[s2 as usize] {
                                let e =
                                    E::Prim(*op, vec![l.clone(), E::Lam1(Box::new(b.clone()))]);
                                if let Some(e) = push(e, &mut level, &mut main_seen) {
                                    found = Some(e);
                                    break 'ops;
                                }
                            }
                        }
                    }
                }
                if found.is_some() {
                    break;
                }
            }
        }
        if let Some(e) = found {
            return Some(e);
        }
        main.push(level);
    }
    None
}

/// Enumerate all ways to pick `arity` args with total size `budget` from a
/// size-indexed bank; calls `f` with each argument vector.
fn enumerate_args(bank: &[Vec<E>], budget: u32, arity: usize, f: &mut impl FnMut(&[E])) {
    fn go(
        bank: &[Vec<E>],
        budget: u32,
        rem: usize,
        acc: &mut Vec<E>,
        f: &mut impl FnMut(&[E]),
    ) {
        if rem == 0 {
            if budget == 0 {
                f(acc);
            }
            return;
        }
        let min_rest = (rem - 1) as u32;
        for s in 1..=budget.saturating_sub(min_rest) {
            if (s as usize) < bank.len() {
                for e in &bank[s as usize] {
                    acc.push(e.clone());
                    go(bank, budget - s, rem - 1, acc, f);
                    acc.pop();
                }
            }
        }
    }
    go(bank, budget, arity, &mut Vec::new(), f);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_div() {
        let inputs = vec![
            vec![V::Nat(10), V::Nat(3)],
            vec![V::Nat(9), V::Nat(2)],
            vec![V::Nat(7), V::Nat(7)],
            vec![V::Nat(0), V::Nat(5)],
            vec![V::Nat(14), V::Nat(4)],
        ];
        let outputs = vec![V::Nat(3), V::Nat(4), V::Nat(1), V::Nat(0), V::Nat(3)];
        let e = solve(&inputs, &outputs, &SemOptions::default()).unwrap();
        assert_eq!(e, E::Prim(Op::Div, vec![E::Var(0), E::Var(1)]));
    }

    #[test]
    fn finds_totient() {
        // φ: 1→1, 6→2, 9→6, 10→4, 12→4, 7→6
        let inputs: Vec<Vec<V>> = [1u64, 6, 9, 10, 12, 7]
            .iter()
            .map(|&n| vec![V::Nat(n)])
            .collect();
        let outputs = vec![
            V::Nat(1),
            V::Nat(2),
            V::Nat(6),
            V::Nat(4),
            V::Nat(4),
            V::Nat(6),
        ];
        let e = solve(&inputs, &outputs, &SemOptions::default()).expect("totient");
        // Expect something like Count(Range1(n), λd. Eq(Gcd(d, n), 1))
        let out = eval(&e, &[V::Nat(20)]).unwrap();
        assert_eq!(out, V::Nat(8)); // φ(20) = 8 — generalizes beyond examples
    }

    #[test]
    fn finds_primality() {
        let inputs: Vec<Vec<V>> = [0u64, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]
            .iter()
            .map(|&n| vec![V::Nat(n)])
            .collect();
        let outputs: Vec<V> = [
            false, false, true, true, false, true, false, true, false, false, false, true, false,
            true,
        ]
        .iter()
        .map(|&b| V::Bool(b))
        .collect();
        let e = solve(&inputs, &outputs, &SemOptions::default()).expect("primality");
        let p = |n: u64| eval(&e, &[V::Nat(n)]).unwrap();
        assert_eq!(p(17), V::Bool(true));
        assert_eq!(p(21), V::Bool(false)); // generalizes
    }
}
