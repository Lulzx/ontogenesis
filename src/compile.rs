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
// The referee (lam) evaluates call-by-NAME with no sharing: any state value
// consumed twice gets re-evaluated, and chained through recursion that goes
// exponential. So the stdlib is written affine-style — Scott data consumed
// by single case-analysis, Y recursion, and explicit @sdup where a computed
// value is genuinely needed twice. (Hand-rolled interaction-net dup nodes,
// which is exactly the bookkeeping HVM automates.) Re-evaluating *data*
// (test inputs, dup outputs) is cheap; only computed chains need dup.
pub const STDLIB: &str = "\
@true = λa.λb.a
@false = λa.λb.b
@not = λp.λa.λb.p(b, a)
@ifc = λp.λt.λf.p(t, f)
@Y = λf.(λx.f(x(x)))(λx.f(x(x)))
@sz = λs.λz.z
@ss = λn.λs.λz.s(n)
@c2s = λn.n(@ss, @sz)
@csucc = λn.λf.λx.f(n(f, x))
@c0 = λf.λx.x
@s2c = @Y(λr.λn.n(λp.@csucc(r(p)), @c0))
@sdup = @Y(λr.λn.λk.n(λp.r(p, λx.λy.k(@ss(x), @ss(y))), k(@sz, @sz)))
@spred = λn.n(λp.p, @sz)
@sadd = @Y(λr.λa.λb.a(λp.@ss(r(p, b)), b))
@ssub = @Y(λr.λa.λb.b(λp.r(@spred(a), p), a))
@smul = @Y(λr.λa.λb.a(λp.@sadd(b, r(p, b)), @sz))
@siszero = λn.n(λp.@false, @true)
@sleq = @Y(λr.λa.λb.a(λpa.b(λpb.r(pa, pb), @false), @true))
@slt = λa.λb.@sleq(@ss(a), b)
@seq = @Y(λr.λa.λb.a(λpa.b(λpb.r(pa, pb), @false), b(λpb.@false, @true)))
@sdiv = @Y(λr.λa.λb.@sdup(b, λb1.λbb.@sdup(bb, λb2.λb3.@sdup(a, λa1.λa2.@ifc(@sleq(b1, a1), @ss(r(@ssub(a2, b2), b3)), @sz)))))
@smod = @Y(λr.λa.λb.@sdup(b, λb1.λbb.@sdup(bb, λb2.λb3.@sdup(a, λa1.λaa.@sdup(aa, λa2.λa3.@ifc(@sleq(b1, a1), r(@ssub(a2, b2), b3), a3))))))
@sgcd = @Y(λr.λa.λb.@sdup(b, λb1.λbb.@sdup(bb, λb2.λb3.b1(λp.r(b2, @smod(a, b3)), a))))
@spow = @Y(λr.λb.λe.e(λp.@smul(b, r(b, p)), @ss(@sz)))
@ssqgo = @Y(λr.λk.λn.@sdup(k, λka.λkb.@sdup(ka, λk1.λk2.@ifc(@sleq(@smul(@ss(k1), @ss(k2)), n), r(@ss(kb), n), kb))))
@ssqrt = λn.@ssqgo(@sz, n)
@sloggo = @Y(λr.λl.λp.λn.λb.@sdup(p, λp1.λp2.@ifc(@sleq(p1, n), r(@ss(l), @smul(p2, b), n, b), l)))
@silog = λn.λb.@sloggo(@sz, b, n, b)
@snil = λc.λn.n
@scons = λh.λt.λc.λn.c(h, t)
@srange1 = @Y(λr.λn.n(λp.@sdup(p, λp1.λp2.@scons(@ss(p1), r(p2))), @snil))
@scount = @Y(λr.λl.λf.l(λh.λt.@ifc(f(h), @ss(r(t, f)), r(t, f)), @sz))
";

fn op_ref(op: Op) -> &'static str {
    match op {
        Op::Add => "@sadd",
        Op::Sub => "@ssub",
        Op::Mul => "@smul",
        Op::Div => "@sdiv",
        Op::Mod => "@smod",
        Op::Gcd => "@sgcd",
        Op::Pow => "@spow",
        Op::Isqrt => "@ssqrt",
        Op::IlogB => "@silog",
        Op::Eq => "@seq",
        Op::Lt => "@slt",
        Op::Leq => "@sleq",
        Op::IsZero => "@siszero",
        Op::Not => "@not",
        Op::If => "@ifc",
        Op::Range1 => "@srange1",
        Op::Count => "@scount",
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
        E::KNat(n) => {
            // Scott literal: @ss(...@ss(@sz))
            let mut s = "@sz".to_string();
            for _ in 0..*n {
                s = format!("@ss({s})");
            }
            s
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
    // Internal representation is Scott; adapt at the boundary.
    let arg = |i: u32| -> String { format!("a{i}") };
    let adapted: Box<dyn Fn(u32) -> String> = match family {
        Family::CNat => Box::new(move |i| format!("@c2s({})", arg(i))),
        Family::SNat => Box::new(move |i| arg(i)),
    };
    let core = emit(e, n_args, adapted.as_ref());
    let wrapped = match (family, out) {
        (Family::CNat, OutKind::Nat) => format!("@s2c({core})"),
        (Family::SNat, OutKind::Nat) => core,
        // Both families state Church booleans for predicates.
        (_, OutKind::Bool) => core,
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
