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
    CList,
    SList,
    NTup,
    CBin,
    SBin,
    CTre,
    STre,
}

impl Family {
    pub fn of_task(id: &str) -> Option<Family> {
        match id.split('_').next()? {
            "cnat" => Some(Family::CNat),
            "snat" => Some(Family::SNat),
            "clst" => Some(Family::CList),
            "slst" => Some(Family::SList),
            "ntup" => Some(Family::NTup),
            "cbin" => Some(Family::CBin),
            "sbin" => Some(Family::SBin),
            "ctre" => Some(Family::CTre),
            "stre" => Some(Family::STre),
            _ => None,
        }
    }
}

/// Per-argument-position structural kind, decided across all tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgKind {
    Nat,
    List,
    Tuple,
    Tree,
    Atom,
}

/// What the task's expected outputs are, structurally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutKind {
    Nat,
    Bool,
    List,
    /// Scott-list output in a family whose default list is Church.
    ListS,
    Tuple,
    Tree,
    /// Element/application-tree output: passes through with no adapter.
    Raw,
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
@cnil = λc.λn.n
@ccons = λh.λt.λc.λn.c(h, t(c, n))
@c2sl = λl.l(@scons, @snil)
@s2cl = @Y(λr.λl.l(λh.λt.@ccons(h, r(t)), @cnil))
@shead = λl.l(λh.λt.h, @sz)
@stail = λl.l(λh.λt.t, @snil)
@slast = @Y(λr.λl.l(λh.λt.t(λh2.λt2.r(@scons(h2, t2)), h), @sz))
@snth = @Y(λr.λl.λi.i(λp.r(@stail(l), p), @shead(l)))
@srevgo = @Y(λr.λl.λa.l(λh.λt.r(t, @scons(h, a)), a))
@srev = λl.@srevgo(l, @snil)
@sapp = @Y(λr.λa.λb.a(λh.λt.@scons(h, r(t, b)), b))
@srotl = λl.l(λh.λt.@sapp(t, @scons(h, @snil)), @snil)
@srotr = λl.@srev(@srotl(@srev(l)))
@slen = @Y(λr.λl.l(λh.λt.@ss(r(t)), @sz))
@smapf = @Y(λr.λf.λl.l(λh.λt.@scons(f(h), r(f, t)), @snil))
@szipf = @Y(λr.λf.λa.λb.a(λha.λta.b(λhb.λtb.@scons(f(ha, hb), r(f, ta, tb)), @snil), @snil))
@sfoldr = @Y(λr.λf.λz.λl.l(λh.λt.f(h, r(f, z, t)), z))
@sfilter = @Y(λr.λf.λl.l(λh.λt.@ifc(f(h), @scons(h, r(f, t)), r(f, t)), @snil))
@id = λx.x
@ssortb = λl.@sapp(@sfilter(@not, l), @sfilter(@id, l))
@tupcol = @Y(λr.λn.λa.n(λp.λx.r(p, @scons(x, a)), @srev(a)))
@tup2list = λn.λt.t(@tupcol(n, @snil))
@l2tgo = @Y(λr.λl.λk.l(λh.λt.r(t, k(h)), k))
@list2tup = λl.λk.@l2tgo(l, k)
@sdbl = @Y(λr.λn.n(λp.@ss(@ss(r(p))), @sz))
@shalf = @Y(λr.λn.n(λp.p(λq.@ss(r(q)), @sz), @sz))
@seven = @Y(λr.λn.n(λp.p(λq.r(q), @false), @true))
@cb2s = λb.b(λx.@sdbl(x), λx.@ss(@sdbl(x)), @sz)
@sb2s = @Y(λr.λb.b(λx.@sdbl(r(x)), λx.@ss(@sdbl(r(x))), @sz))
@cbE = λo.λi.λe.e
@cbO = λx.λo.λi.λe.o(x(o, i, e))
@cbI = λx.λo.λi.λe.i(x(o, i, e))
@sbE = λo.λi.λe.e
@sbO = λx.λo.λi.λe.o(x)
@sbI = λx.λo.λi.λe.i(x)
@s2cb = @Y(λr.λn.n(λp.@sdup(@ss(p), λn1.λn2.@ifc(@seven(n1), @cbO(r(@shalf(n2))), @cbI(r(@shalf(n2))))), @cbE))
@s2sb = @Y(λr.λn.n(λp.@sdup(@ss(p), λn1.λn2.@ifc(@seven(n1), @sbO(r(@shalf(n2))), @sbI(r(@shalf(n2))))), @sbE))
@stleaf = λx.λn.λl.l(x)
@stnode = λa.λb.λn.λl.n(a, b)
@ct2st = λt.t(@stnode, @stleaf)
@st2ct = @Y(λr.λt.t(λa.λb.λn.λl.n(r(a)(n, l), r(b)(n, l)), λx.λn.λl.l(x)))
@stflat = @Y(λr.λt.t(λa.λb.@sapp(r(a), r(b)), λx.@scons(x, @snil)))
@stmirror = @Y(λr.λt.t(λa.λb.@stnode(r(b), r(a)), λx.@stleaf(x)))
@sbfsq = @Y(λr.λq.q(λh.λt.h(λa.λb.r(@sapp(t, @scons(a, @scons(b, @snil)))), λx.@scons(x, r(t))), @snil))
@stbfs = λt.@sbfsq(@scons(t, @snil))
@stmerge = @Y(λr.λf.λa.λb.a(λa1.λa2.b(λb1.λb2.@stnode(r(f, a1, b1), r(f, a2, b2)), @stleaf(@sz)), λx.b(λw1.λw2.@stleaf(@sz), λy.@stleaf(f(x, y)))))
@stidxgo = @Y(λr.λt.λi.λf.λk.t(λa.λb.r(a, i, f, λa2.λi2.r(b, i2, f, λb2.λi3.k(@stnode(a2, b2), i3))), λx.@sdup(i, λi1.λi2.k(@stleaf(f(i1, x)), @ss(i2)))))
@stidx = λf.λt.@stidxgo(t, @sz, f, λt2.λi.t2)
@stscango = @Y(λr.λt.λz.λf.λk.t(λa.λb.r(a, z, f, λa2.λz2.r(b, z2, f, λb2.λz3.k(@stnode(a2, b2), z3))), λx.k(@stleaf(z), f(z, x))))
@stscan = λf.λz.λt.@stscango(t, z, f, λt2.λz2.t2)
@sevns = @Y(λr.λl.l(λh.λt.@scons(h, t(λh2.λt2.r(t2), @snil)), @snil))
@sodds = λl.l(λh.λt.@sevns(t), @snil)
@sbrp = @Y(λr.λl.l(λh.λt.t(λh2.λt2.@sapp(r(@sevns(@scons(h, @scons(h2, t2)))), r(@sodds(@scons(h, @scons(h2, t2))))), @scons(h, @snil)), @snil))
@spair2 = @Y(λr.λl.l(λh.λt.t(λh2.λt2.@scons(@stnode(h, h2), r(t2)), @snil), @snil))
@sbuildgo = @Y(λr.λl.l(λh.λt.t(λh2.λt2.r(@spair2(@scons(h, @scons(h2, t2)))), h), @sz))
@stbuild = λl.@sbuildgo(@smapf(@stleaf, l))
@stbrev = λt.@stbuild(@sbrp(@stflat(t)))
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
        Op::Head => "@shead",
        Op::Last => "@slast",
        Op::Nth => "@snth",
        Op::Rev => "@srev",
        Op::RotL => "@srotl",
        Op::RotR => "@srotr",
        Op::Len => "@slen",
        Op::AppendL => "@sapp",
        Op::SortB => "@ssortb",
        Op::MapAp => "@smapf",
        Op::ZipAp => "@szipf",
        Op::FoldrAp => "@sfoldr",
        Op::TFlat => "@stflat",
        Op::TBfs => "@stbfs",
        Op::TMirror => "@stmirror",
        Op::TBuild => "@stbuild",
        Op::TMergeAp => "@stmerge",
        Op::TIdxAp => "@stidx",
        Op::TScanAp => "@stscan",
        Op::TBitRev => "@stbrev",
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
/// Internal representation is Scott; adapters sit at the boundary.
pub fn program(family: Family, out: OutKind, e: &E, kinds: &[ArgKind], size_idx: usize) -> String {
    let n_args = kinds.len();
    let kinds_owned: Vec<ArgKind> = kinds.to_vec();
    let adapted = move |i: u32| -> String {
        let a = format!("a{i}");
        match (family, kinds_owned[i as usize]) {
            (Family::SNat | Family::SList, ArgKind::Nat) => a,
            (Family::CBin, ArgKind::Nat) => format!("@cb2s({a})"),
            (Family::SBin, ArgKind::Nat) => format!("@sb2s({a})"),
            (_, ArgKind::Nat) => format!("@c2s({a})"),
            (Family::SList, ArgKind::List) => a,
            (_, ArgKind::List) => format!("@c2sl({a})"),
            // N-tuples arrive with a Church-nat size argument; its position
            // varies per task, so the caller tries each Nat position until
            // one verifies.
            (_, ArgKind::Tuple) => format!("@tup2list(@c2s(a{size_idx}), {a})"),
            (Family::CTre, ArgKind::Tree) => format!("@ct2st({a})"),
            (_, ArgKind::Tree) => a,
            (_, ArgKind::Atom) => a,
        }
    };
    let core = emit(e, n_args, &adapted);
    let wrapped = match (family, out) {
        (Family::SNat | Family::SList, OutKind::Nat) => core,
        (Family::CBin, OutKind::Nat) => format!("@s2cb({core})"),
        (Family::SBin, OutKind::Nat) => format!("@s2sb({core})"),
        (_, OutKind::Nat) => format!("@s2c({core})"),
        (Family::SList, OutKind::List) => core,
        (_, OutKind::List) => format!("@s2cl({core})"),
        (_, OutKind::ListS) => core,
        (_, OutKind::Tuple) => format!("@list2tup({core})"),
        (Family::CTre, OutKind::Tree) => format!("@st2ct({core})"),
        (_, OutKind::Tree) => core,
        (_, OutKind::Bool) | (_, OutKind::Raw) => core,
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

/// Decode all tests of a supported-family task into native I/O, classify
/// each argument position and the output kind. None if anything fails.
pub fn decode_task(
    family: Family,
    task: &Task,
) -> Option<(Vec<Vec<V>>, Vec<V>, Vec<ArgKind>, OutKind)> {
    let empty: Env = Rc::new(Vec::new());
    let norm = |t: &Rc<Term>| -> Option<Rc<Term>> {
        let mut fuel = Fuel(1_000_000);
        normalize(&empty, t, &mut fuel).ok()
    };
    let scott_side = matches!(family, Family::SNat | Family::SList);
    let dec_nat = |t: &Rc<Term>| -> Option<V> {
        let n = match family {
            Family::CBin => crate::decode::church_bin(t)?,
            Family::SBin => crate::decode::scott_bin(t)?,
            _ if scott_side => crate::decode::scott_nat(t)?,
            _ => crate::decode::church_nat(t)?,
        };
        Some(V::Nat(n))
    };
    let dec_list = |t: &Rc<Term>| -> Option<V> {
        let xs = match family {
            Family::SList => crate::decode::scott_list(t)?,
            _ => crate::decode::church_list(t)?,
        };
        Some(V::List(xs))
    };
    let dec_tuple = |t: &Rc<Term>| -> Option<V> { Some(V::List(crate::decode::ntuple(t)?)) };
    let dec_tree = |t: &Rc<Term>| -> Option<V> {
        match family {
            Family::CTre => crate::decode::church_tree(t),
            _ => crate::decode::scott_tree(t),
        }
    };
    let dec_slist = |t: &Rc<Term>| -> Option<V> { Some(V::List(crate::decode::scott_list(t)?)) };
    let dec_clist = |t: &Rc<Term>| -> Option<V> { Some(V::List(crate::decode::church_list(t)?)) };
    let dec_atomish = |t: &Rc<Term>| -> Option<V> { crate::decode::decode_value(t) };
    let dec_bool = |t: &Rc<Term>| -> Option<V> {
        match t.as_ref() {
            Term::Lam(b1) => match b1.as_ref() {
                Term::Lam(b2) => match b2.as_ref() {
                    Term::Var(1) => Some(V::Bool(true)),
                    Term::Var(0) => Some(V::Bool(false)),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    };

    // Normalize everything once.
    let mut arg_nfs: Vec<Vec<Rc<Term>>> = Vec::new(); // [test][arg]
    let mut want_nfs: Vec<Rc<Term>> = Vec::new();
    for t in &task.tests {
        let mut row = Vec::new();
        for a in &t.args {
            row.push(norm(a)?);
        }
        arg_nfs.push(row);
        // Strip the test's outer binders (λA.λB. wrappers) and substitute
        // them with the same Free constants the parser used in the args.
        want_nfs.push(crate::parse::strip_outer(&norm(&t.want)?, t.outer)?);
    }
    let n_args = task.arity;

    // Per-position kind: first interpretation that decodes in every test.
    type Dec<'d> = &'d dyn Fn(&Rc<Term>) -> Option<V>;
    let try_all = |dec: Dec, col: &dyn Fn(usize) -> Rc<Term>, n: usize| -> Option<Vec<V>> {
        (0..n).map(|j| dec(&col(j))).collect()
    };
    let mut kinds: Vec<ArgKind> = Vec::new();
    let mut cols: Vec<Vec<V>> = Vec::new(); // [arg][test]
    for p in 0..n_args {
        let col = |j: usize| arg_nfs[j][p].clone();
        let candidates: Vec<(ArgKind, Dec)> = match family {
            Family::CNat | Family::SNat | Family::CBin | Family::SBin => {
                vec![(ArgKind::Nat, &dec_nat)]
            }
            Family::CList | Family::SList => vec![
                (ArgKind::Nat, &dec_nat),
                (ArgKind::List, &dec_list),
                (ArgKind::Atom, &dec_atomish),
            ],
            Family::NTup => vec![
                (ArgKind::Nat, &dec_nat),
                (ArgKind::Tuple, &dec_tuple),
                (ArgKind::Atom, &dec_atomish),
            ],
            Family::CTre | Family::STre => vec![
                (ArgKind::Tree, &dec_tree),
                (ArgKind::Atom, &dec_atomish),
            ],
        };
        let mut hit = None;
        for (k, dec) in candidates {
            if let Some(vals) = try_all(dec, &col, task.tests.len()) {
                hit = Some((k, vals));
                break;
            }
        }
        let (k, vals) = hit?;
        kinds.push(k);
        cols.push(vals);
    }
    let inputs: Vec<Vec<V>> = (0..task.tests.len())
        .map(|j| (0..n_args).map(|p| cols[p][j].clone()).collect())
        .collect();

    // Output kind: ordered attempts.
    let wcol = |j: usize| want_nfs[j].clone();
    let out_candidates: Vec<(OutKind, Dec)> = match family {
        Family::CNat | Family::SNat | Family::CBin | Family::SBin => {
            vec![(OutKind::Nat, &dec_nat), (OutKind::Bool, &dec_bool)]
        }
        Family::NTup => vec![
            (OutKind::Tuple, &dec_tuple),
            (OutKind::Nat, &dec_nat),
            (OutKind::Bool, &dec_bool),
            (OutKind::Raw, &dec_atomish),
        ],
        Family::CTre => vec![
            (OutKind::Tree, &dec_tree),
            (OutKind::List, &dec_clist),
            (OutKind::ListS, &dec_slist),
            (OutKind::Raw, &dec_atomish),
        ],
        Family::STre => vec![
            (OutKind::Tree, &dec_tree),
            (OutKind::ListS, &dec_slist),
            (OutKind::List, &dec_clist),
            (OutKind::Raw, &dec_atomish),
        ],
        _ => vec![
            (OutKind::List, &dec_list),
            (OutKind::Nat, &dec_nat),
            (OutKind::Bool, &dec_bool),
            (OutKind::Raw, &dec_atomish),
        ],
    };
    for (k, dec) in out_candidates {
        if let Some(outs) = try_all(dec, &wcol, task.tests.len()) {
            return Some((inputs, outs, kinds, k));
        }
    }
    None
}
