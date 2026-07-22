//! Compile a semantic DSL expression to a Lamb (.lam) program.
//!
//! Strategy: a small handwritten Lamb standard library implements each DSL
//! primitive on Church encodings, with bounded iteration instead of general
//! recursion wherever possible (a Church nat is its own loop counter).
//! Scott-family tasks get decode/encode adapters at the boundary; @Y exists
//! solely for the Scott→Church adapter. The compiled program is verified
//! against every test with the internal normalizer before being emitted —
//! nothing unsound can escape, because the referee gets the final word.

use crate::decode::V;
use crate::nbe::{normalize, Env, Fuel};
use crate::parse::{self, Expr, Task};
use crate::sem::{E, Op};
use crate::term::Term;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    CNat,
    SNat,
}

impl Family {
    pub fn of_task(id: &str) -> Option<Family> {
        if id.starts_with("cnat_") {
            Some(Family::CNat)
        } else if id.starts_with("snat_") {
            Some(Family::SNat)
        } else {
            None
        }
    }
}

/// What the task's expected outputs are, structurally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutKind {
    Nat,
    Bool,
}

/// The standard library. Order matters: each definition only references
/// earlier ones (except @Y's internal self-application), so the verifier
/// can inline top-to-bottom.
pub const STDLIB: &str = "\
@c0 = λf.λx.x
@c1 = λf.λx.f(x)
@c2 = λf.λx.f(f(x))
@cnorm = λn.λf.λx.n(f, x)
@succ = λn.λf.λx.f(n(f, x))
@add = λm.λn.λf.λx.m(f, n(f, x))
@mul = λm.λn.λf.m(n(f))
@pow = λb.λe.@cnorm(e(b))
@true = λa.λb.a
@false = λa.λb.b
@not = λp.λa.λb.p(b, a)
@and = λp.λq.p(q, @false)
@ifc = λp.λt.λf.p(t, f)
@pair = λa.λb.λs.s(a, b)
@fst = λp.p(λa.λb.a)
@snd = λp.p(λa.λb.b)
@pred = λn.@fst(n(λp.@pair(@snd(p), @succ(@snd(p))), @pair(@c0, @c0)))
@sub = λm.λn.@cnorm(n(@pred, m))
@iszero = λn.n(λw.@false, @true)
@leq = λm.λn.@iszero(@sub(m, n))
@lt = λm.λn.@leq(@succ(m), n)
@eq = λm.λn.@and(@leq(m, n), @leq(n, m))
@div = λa.λb.@cnorm(@fst(a(λp.@ifc(@leq(b, @snd(p)), @pair(@succ(@fst(p)), @sub(@snd(p), b)), p), @pair(@c0, a))))
@mod = λa.λb.@cnorm(@snd(a(λp.@ifc(@leq(b, @snd(p)), @pair(@succ(@fst(p)), @sub(@snd(p), b)), p), @pair(@c0, a))))
@gcd = λa.λb.@cnorm(@fst(@add(a, b)(λp.@ifc(@iszero(@snd(p)), p, @pair(@snd(p), @mod(@fst(p), @snd(p)))), @pair(a, b))))
@isqrt = λn.@cnorm(@fst(n(λp.@ifc(@leq(@mul(@snd(p), @snd(p)), n), @pair(@snd(p), @succ(@snd(p))), p), @pair(@c0, @c1))))
@ilog = λn.λb.@cnorm(@fst(n(λp.@ifc(@leq(@snd(p), n), @pair(@succ(@fst(p)), @mul(@snd(p), b)), p), @pair(@c0, b))))
@nil = λc.λn.n
@cons = λh.λt.λc.λn.c(h, t(c, n))
@range1 = λn.@snd(n(λp.@pair(@succ(@fst(p)), @cons(@succ(@fst(p)), @snd(p))), @pair(@c0, @nil)))
@count = λl.λf.@cnorm(l(λh.λr.@ifc(f(h), @succ(r), r), @c0))
@Y = λf.(λx.f(x(x)))(λx.f(x(x)))
@sz = λs.λz.z
@ssucc = λn.λs.λz.s(n)
@c2s = λn.n(@ssucc, @sz)
@s2c = @Y(λr.λn.n(λp.@succ(r(p)), @c0))
";

fn op_ref(op: Op) -> &'static str {
    match op {
        Op::Add => "@add",
        Op::Sub => "@sub",
        Op::Mul => "@mul",
        Op::Div => "@div",
        Op::Mod => "@mod",
        Op::Gcd => "@gcd",
        Op::Pow => "@pow",
        Op::Isqrt => "@isqrt",
        Op::IlogB => "@ilog",
        Op::Eq => "@eq",
        Op::Lt => "@lt",
        Op::Leq => "@leq",
        Op::IsZero => "@iszero",
        Op::Not => "@not",
        Op::If => "@ifc",
        Op::Range1 => "@range1",
        Op::Count => "@count",
    }
}

/// Emit the expression as Lamb source. `arg_name(i)` supplies the source
/// name for task argument i (already adapted to Church encoding).
fn emit(e: &E, n_args: usize, arg_name: &dyn Fn(u32) -> String) -> String {
    match e {
        E::Var(i) => {
            if (*i as usize) < n_args {
                arg_name(*i)
            } else {
                "p".to_string() // the single lambda-body parameter
            }
        }
        E::KNat(0) => "@c0".to_string(),
        E::KNat(1) => "@c1".to_string(),
        E::KNat(2) => "@c2".to_string(),
        E::KNat(n) => {
            // λf.λx.f(...f(x))
            let mut body = "x".to_string();
            for _ in 0..*n {
                body = format!("f({body})");
            }
            format!("(λf.λx.{body})")
        }
        E::Lam1(b) => format!("λp.{}", emit(b, n_args, arg_name)),
        E::Prim(op, args) => {
            let parts: Vec<String> = args.iter().map(|a| emit(a, n_args, arg_name)).collect();
            format!("{}({})", op_ref(*op), parts.join(", "))
        }
    }
}

/// Build the full .lam source for a solved task.
pub fn program(family: Family, out: OutKind, e: &E, n_args: usize) -> String {
    let arg = |i: u32| -> String { format!("a{i}") };
    let adapted: Box<dyn Fn(u32) -> String> = match family {
        Family::CNat => Box::new(move |i| arg(i)),
        Family::SNat => Box::new(move |i| format!("@s2c({})", arg(i))),
    };
    let core = emit(e, n_args, adapted.as_ref());
    let wrapped = match (family, out) {
        (Family::CNat, OutKind::Nat) => format!("@cnorm({core})"),
        (Family::CNat, OutKind::Bool) => core,
        (Family::SNat, OutKind::Nat) => format!("@c2s({core})"),
        // Scott tasks state "Church booleans" for predicates too.
        (Family::SNat, OutKind::Bool) => core,
    };
    let mut lams = String::new();
    for i in 0..n_args {
        lams.push_str(&format!("λa{i}."));
    }
    format!("{STDLIB}@main = {lams}{wrapped}\n")
}

// ── Internal verification ───────────────────────────────────────────

/// Resolve a book (ordered @defs) into a single closed Term for @main by
/// inlining references top-to-bottom.
pub fn inline_main(src: &str) -> Result<Rc<Term>, String> {
    let mut defs: HashMap<String, Rc<Term>> = HashMap::new();
    for line in src.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let (name, rhs) = line
            .split_once('=')
            .ok_or_else(|| format!("bad def line: {line}"))?;
        let name = name.trim().strip_prefix('@').ok_or("def must start with @")?;
        let expr = parse::parse_expr(rhs.trim())?;
        let term = resolve(&expr, &defs)?;
        defs.insert(name.to_string(), term);
    }
    defs.get("main").cloned().ok_or_else(|| "no @main".to_string())
}

fn resolve(e: &Expr, defs: &HashMap<String, Rc<Term>>) -> Result<Rc<Term>, String> {
    match e {
        Expr::Var(i) => Ok(crate::term::var(*i)),
        Expr::Lam(b) => Ok(crate::term::lam(resolve(b, defs)?)),
        Expr::App(f, a) => Ok(crate::term::app(resolve(f, defs)?, resolve(a, defs)?)),
        Expr::Ref(n) => defs
            .get(n)
            .cloned()
            .ok_or_else(|| format!("forward/unknown reference @{n}")),
    }
}

/// Check the compiled program against every test with the internal
/// normalizer (same semantics as the referee). Returns true only if all
/// tests match exactly.
pub fn verify(main: &Rc<Term>, task: &Task, fuel: i64) -> bool {
    let empty: Env = Rc::new(Vec::new());
    for t in &task.tests {
        // Build @main(a1, ..., ak) as a term and normalize.
        let mut appl = main.clone();
        for a in &t.args {
            appl = crate::term::app(appl, a.clone());
        }
        let mut f1 = Fuel(fuel);
        let Ok(got) = normalize(&empty, &appl, &mut f1) else {
            return false;
        };
        let mut f2 = Fuel(fuel);
        let want = normalize(&empty, &t.want, &mut f2)
            .ok()
            .and_then(|nf| parse::strip_outer(&nf, t.outer));
        match want {
            Some(w) => {
                if got != w {
                    return false;
                }
            }
            None => return false,
        }
    }
    true
}

// ── Task-level decoding for the supported families ──────────────────

/// Decode all tests of a cnat_/snat_ task into native I/O, and classify the
/// output kind. Returns None if anything fails to decode.
pub fn decode_task(family: Family, task: &Task) -> Option<(Vec<Vec<V>>, Vec<V>, OutKind)> {
    let empty: Env = Rc::new(Vec::new());
    let norm = |t: &Rc<Term>| -> Option<Rc<Term>> {
        let mut fuel = Fuel(1_000_000);
        normalize(&empty, t, &mut fuel).ok()
    };
    let dec_nat = |t: &Rc<Term>| -> Option<u64> {
        match family {
            Family::CNat => crate::decode::church_nat(t),
            Family::SNat => crate::decode::scott_nat(t),
        }
    };
    let dec_bool = |t: &Rc<Term>| -> Option<bool> {
        // Church booleans: λa.λb.a / λa.λb.b (used by both families).
        match t.as_ref() {
            Term::Lam(b1) => match b1.as_ref() {
                Term::Lam(b2) => match b2.as_ref() {
                    Term::Var(1) => Some(true),
                    Term::Var(0) => Some(false),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    };

    let mut inputs = Vec::new();
    let mut want_nfs = Vec::new();
    for t in &task.tests {
        if t.outer != 0 {
            return None; // abstract binders don't occur in nat tasks
        }
        let mut row = Vec::new();
        for a in &t.args {
            let nf = norm(a)?;
            row.push(V::Nat(dec_nat(&nf)?));
        }
        inputs.push(row);
        want_nfs.push(norm(&t.want)?);
    }
    // Church false is also Church 0, so classify the output kind globally:
    // all outputs decode as nats, else all as booleans.
    if let Some(nats) = want_nfs.iter().map(|t| dec_nat(t)).collect::<Option<Vec<_>>>() {
        return Some((inputs, nats.into_iter().map(V::Nat).collect(), OutKind::Nat));
    }
    if let Some(bs) = want_nfs.iter().map(|t| dec_bool(t)).collect::<Option<Vec<_>>>() {
        return Some((inputs, bs.into_iter().map(V::Bool).collect(), OutKind::Bool));
    }
    None
}
