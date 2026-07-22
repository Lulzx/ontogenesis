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
}

impl Op {
    /// (value-arity, lambda-arity). Lambda args are 1-parameter bodies.
    pub fn sig(self) -> (usize, usize) {
        match self {
            Op::Isqrt | Op::IsZero | Op::Not | Op::Range1 => (1, 0),
            Op::If => (3, 0),
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
            for op in Op::all() {
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
            'ops: for op in Op::all() {
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
