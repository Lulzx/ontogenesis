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
    CAdt,
    SAdt,
    Algo,
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
            "cadt" => Some(Family::CAdt),
            "sadt" => Some(Family::SAdt),
            "algo" => Some(Family::Algo),
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
    /// ADT type descriptor (kept in Scott form, passed through).
    Desc,
    /// ADT value in the family's encoding.
    AdtVal,
    /// Scott list of Church booleans (serialized bits).
    Bits,
    Atom,
    /// GN-number tree (fft): L/B tree with balanced-ternary leaves.
    Gn,
    /// Algo-family value consumed in its native encoding (identity adapter).
    Raw,
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
    /// ADT value, Church-encoded / Scott-encoded.
    AdtC,
    AdtS,
    /// Scott list of Church-encoded ADT values (cadt_chi).
    ListAdtC,
    /// Scott list of Scott-encoded ADT values (sadt_chi).
    ListAdtS,
    /// Scott list of Church booleans (serialize output).
    Bits,
    /// Element/application-tree output: passes through with no adapter.
    Raw,
    /// GN-number tree output (fft).
    Gn,
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
@vapply = λf.λl.@l2tgo(l, f)
@colk = @Y(λr.λn.λa.λk.n(λp.λx.r(p, @scons(x, a), k), k(@srev(a))))
@mkadt = λn.λi.λfs.@colk(n, @snil, λcs.@vapply(@snth(cs, i), fs))
@tgo = @Y(λr.λds.λi.λk.ds(λspec.λrest.@sdup(i, λi1.λi2.@scons(@colk(@slen(spec), @snil, λfs.k(i1, fs)), r(rest, @ss(i2), k))), @snil))
@scase = λd.λv.λk.@l2tgo(@tgo(d, @sz, k), v)
@hgo = @Y(λr.λds.λi.λn.ds(λspec.λrest.@sdup(i, λi1.λi2.@scons(@colk(@slen(spec), @snil, λfs.@mkadt(n, i1, fs)), r(rest, @ss(i2), n))), @snil))
@c2sadt = λd.λv.@l2tgo(@hgo(d, @sz, @slen(d)), v)
@s2cadt = @Y(λr.λd.λv.@scase(d, v, λi.λfs.@colk(@slen(d), @snil, λhs.@vapply(@snth(hs, i), @smapf(λc.@vapply(r(d, c), hs), fs)))))
@adtidx = λd.λv.@scase(d, v, λi.λfs.i)
@adtchi = λd.λv.@scase(d, v, λi.λfs.fs)
@adtlen = @Y(λr.λd.λv.@scase(d, v, λi.λfs.@ss(@sfoldr(λc.λacc.@sadd(r(d, c), acc), @sz, fs))))
@adtrev = @Y(λr.λd.λv.@scase(d, v, λi.λfs.@mkadt(@slen(d), i, @srev(@smapf(r(d), fs)))))
@andb = λp.λq.p(q, @false)
@nonemp = λl.l(λh.λt.@true, @false)
@andl = @Y(λr.λl.l(λh.λt.h(r(t), @false), @true))
@adteq = @Y(λr.λd.λa.λb.@scase(d, a, λi.λfs.@scase(d, b, λj.λgs.@ifc(@seq(i, j), @andl(@szipf(r(d), fs, gs)), @false))))
@swgo = @Y(λr.λw.λp.λn.@sdup(p, λp1.λp2.@ifc(@sleq(n, p1), w, r(@ss(w), @sdbl(p2), n))))
@swidth = λn.@swgo(@sz, @ss(@sz), n)
@bitsl = @Y(λr.λw.λi.w(λwp.@sdup(i, λi1.λi2.@scons(@ifc(@seven(i1), @false, @true), r(wp, @shalf(i2)))), @snil))
@sconcat = @Y(λr.λl.l(λh.λt.@sapp(h, r(t)), @snil))
@adtser = @Y(λr.λd.λv.@scase(d, v, λi.λfs.@sapp(@srev(@bitsl(@swidth(@slen(d)), i)), @sconcat(@smapf(r(d), fs)))))
@readn = @Y(λr.λw.λbits.λacc.λk.w(λwp.bits(λb.λrest.r(wp, rest, @ifc(b, @ss(@sdbl(acc)), @sdbl(acc)), k), k(acc, @snil)), k(acc, bits)))
@pfl = @Y(λr.λrec.λspec.λbits.λacc.λk.spec(λf.λfr.rec(bits, λv.λrest.r(rec, fr, rest, @scons(v, acc), k)), k(@srev(acc), bits)))
@adtdes = @Y(λr.λd.λbits.λk.@readn(@swidth(@slen(d)), bits, @sz, λi.λrest.@sdup(i, λi1.λi2.@pfl(λbs.λk2.r(d, bs, k2), @snth(d, i1), rest, @snil, λfs.λrest2.k(@mkadt(@slen(d), i2, fs), rest2)))))
@adtdesv = λd.λbits.@adtdes(d, bits, λv.λrest.v)
@adtmrg = @Y(λr.λd.λf.λa.λb.@scase(d, a, λi.λfs.@scase(d, b, λj.λgs.@sdup(i, λi1.λi2.@sdup(j, λj1.λj2.@ifc(@andb(@seq(i1, j1), @nonemp(fs)), @mkadt(@slen(d), i2, @szipf(r(d, f), fs, gs)), f(@s2cadt(d, @mkadt(@slen(d), i2, fs)), @s2cadt(d, @mkadt(@slen(d), j2, gs)))))))))
@adtmrgc = @Y(λr.λd.λf.λa.λb.@scase(d, a, λi.λfs.@scase(d, b, λj.λgs.@sdup(i, λi1.λi2.@sdup(j, λj1.λj2.@ifc(@andb(@seq(i1, j1), @nonemp(fs)), @colk(@slen(d), @snil, λhs.@vapply(@snth(hs, i2), @szipf(r(d, f), fs, gs))), f(@s2cadt(d, @mkadt(@slen(d), i2, fs)), @s2cadt(d, @mkadt(@slen(d), j2, gs)))))))))
@adtmrgs = @Y(λr.λd.λf.λa.λb.@scase(d, a, λi.λfs.@scase(d, b, λj.λgs.@sdup(i, λi1.λi2.@sdup(j, λj1.λj2.@ifc(@andb(@seq(i1, j1), @nonemp(fs)), @mkadt(@slen(d), i2, @szipf(r(d, f), fs, gs)), f(@mkadt(@slen(d), i2, fs), @mkadt(@slen(d), j2, gs))))))))
@mkcctor = λd.λi.@sdup(i, λi1.λi2.@colk(@slen(@snth(d, i1)), @snil, λfs.@colk(@slen(d), @snil, λhs.@vapply(@snth(hs, i2), @smapf(λf.@vapply(f, hs), fs)))))
@mksctor = λd.λi.@sdup(i, λi1.λi2.@colk(@slen(@snth(d, i1)), @snil, λfs.@mkadt(@slen(d), i2, fs)))
";

/// Algorithm-track library. Same affine discipline as STDLIB: computed
/// values are forced+copied through @sdup/@sldup CPS before reuse; input
/// data and dup outputs are normal forms and may be re-read freely.
/// Emitted only for Family::Algo tasks (keeps other families' binaries small).
pub const ALGO_LIB: &str = "\
@sfst = λp.p(λx.λy.x)
@ssnd = λp.p(λx.λy.y)
@spair = λx.λy.λp.p(x, y)
@smax = λa.λb.@sdup(a, λa1.λa2.@sdup(b, λb1.λb2.@ifc(@sleq(a1, b1), b2, a2)))
@smin = λa.λb.@sdup(a, λa1.λa2.@sdup(b, λb1.λb2.@ifc(@sleq(a1, b1), a2, b2)))
@srange0 = λn.@srev(@smapf(@spred, @srange1(n)))
@ssuml = λl.@sfoldr(@sadd, @sz, l)
@sminl = λl.l(λh.λt.@sfoldr(@smin, h, t), @sz)
@orl = @Y(λr.λl.l(λh.λt.@ifc(h, @true, r(t)), @false))
@none = λs.λn.n
@some = λx.λs.λn.s(x)
@orelse = λm1.λm2.m1(λx.@some(x), m2)
@sldup = @Y(λr.λl.λk.l(λh.λt.@sdup(h, λh1.λh2.r(t, λt1.λt2.k(@scons(h1, t1), @scons(h2, t2)))), k(@snil, @snil)))
@smapb = λl.λf.@smapf(f, l)
@spicks = @Y(λr.λl.l(λh.λt.@scons(@spair(h, t), @smapf(λpk.pk(λx.λrest.@spair(x, @scons(h, rest))), r(t))), @snil))
@sperms = @Y(λr.λl.l(λh.λt.@sconcat(@smapf(λpk.pk(λx.λrest.@smapf(λq.@scons(x, q), r(rest))), @spicks(@scons(h, t)))), @scons(@snil, @snil)))
@scyclecost = λm.λp.@sldup(p, λp1.λp2.@ssuml(@szipf(λi.λj.@snth(@snth(m, i), j), p1, @srotl(p2))))
@litidx = λlit.lit(λi.i, λi.i)
@litsat = λasn.λlit.lit(λi.@snth(asn, i), λi.@not(@snth(asn, i)))
@smaxl = @Y(λr.λl.l(λh.λt.@smax(h, r(t)), @sz))
@nvars = λf.@smaxl(@smapf(λlit.@ss(@litidx(lit)), @sconcat(f)))
@sat0 = λasn.λf.@andl(@smapf(λc.@orl(@smapf(@litsat(asn), c)), f))
@satgo = @Y(λr.λv.λasn.λf.v(λvp.@sdup(vp, λv1.λv2.@ifc(r(v1, @scons(@true, asn), f), @true, r(v2, @scons(@false, asn), f))), @sat0(asn, f)))
@ssatcnf = λf.@satgo(@nvars(f), @snil, f)
@slineras = λp.λq.p(λa.λb.q(λc.λd.@ifc(@andb(@siszero(@ssub(c, a)), @siszero(@ssub(d, b))), @scons(@spair(a, b), @snil), @ifc(@sleq(@ssub(d, b), @ssub(c, a)), @smapf(λk.@sdup(k, λk1.λk2.@spair(@sadd(a, k1), @sadd(b, @sdiv(@sadd(@smul(@sdbl(k2), @ssub(d, b)), @spred(@ssub(c, a))), @sdbl(@ssub(c, a)))))), @srange0(@ss(@ssub(c, a)))), @smapf(λk.@sdup(k, λk1.λk2.@spair(@sadd(a, @sdiv(@sadd(@smul(@sdbl(k2), @ssub(c, a)), @spred(@ssub(d, b))), @sdbl(@ssub(d, b)))), @sadd(b, k1))), @srange0(@ss(@ssub(d, b))))))))
@lset = @Y(λr.λl.λi.λv.l(λh.λt.i(λip.@scons(h, r(t, ip, v)), @scons(v, t)), @snil))
@gset = @Y(λr.λg.λi.λj.λv.g(λh.λt.i(λip.@scons(h, r(t, ip, j, v)), @scons(@lset(h, j, v), t)), @snil))
@gget = λg.λi.λj.@snth(@snth(g, i), j)
@bdup = λb.λk.b(k(@true, @true), k(@false, @false))
@lbdup = @Y(λr.λl.λk.l(λh.λt.@bdup(h, λh1.λh2.r(t, λt1.λt2.k(@scons(h1, t1), @scons(h2, t2)))), k(@snil, @snil)))
@gbdup = @Y(λr.λg.λk.g(λh.λt.@lbdup(h, λh1.λh2.r(t, λt1.λt2.k(@scons(h1, t1), @scons(h2, t2)))), k(@snil, @snil)))
@nbrs = λhh.λww.λi.λj.@sapp(i(λip.@scons(@spair(ip, j), @snil), @snil), @sapp(@ifc(@slt(@ss(i), hh), @scons(@spair(@ss(i), j), @snil), @snil), @sapp(j(λjp.@scons(@spair(i, jp), @snil), @snil), @ifc(@slt(@ss(j), ww), @scons(@spair(i, @ss(j)), @snil), @snil))))
@istarget = λhh.λww.λpr.pr(λi.λj.@andb(@seq(@ss(i), hh), @seq(@ss(j), ww)))
@markall = @Y(λr.λg.λfr.fr(λp.λt.p(λi.λj.r(@gset(g, i, j, @false), t)), g))
@expand = λg.λhh.λww.λfr.@sconcat(@smapf(λp.p(λi.λj.@sfilter(λq.q(λx.λy.@gget(g, x, y)), @nbrs(hh, ww, i, j))), fr))
@bfsgo = @Y(λr.λg.λfr.λsteps.fr(λq0.λq1.@sdup(@slen(g), λhh1.λhh2.@sdup(@slen(@shead(g)), λww1.λww2.@ifc(@orl(@smapf(@istarget(hh1, ww1), @scons(q0, q1))), steps, (λg2.@gbdup(g2, λga.λgb.r(gb, @expand(ga, hh2, ww2, @scons(q0, q1)), @ss(steps))))(@markall(g, @scons(q0, q1)))))), @sz))
@sgridbfs = λg.@bfsgo(g, @scons(@spair(@sz, @sz), @snil), @sz)
@better = λw.λx.λbst.bst(λpr.pr(λbw.λbx.@sdup(w, λw1.λw2.@sdup(bw, λbw1.λbw2.@ifc(@slt(w1, bw1), @some(@spair(w2, x)), @some(@spair(bw2, bx)))))), @some(@spair(w, x)))
@scanbest = λit.λes.@sfoldr(λe.λbst.e(λu.λv.λw.@ifc(@snth(it, u), @ifc(@snth(it, v), bst, @better(w, v, bst)), @ifc(@snth(it, v), @better(w, u, bst), bst))), @none, es)
@srepl = @Y(λr.λn.λx.n(λp.@scons(x, r(p, x)), @snil))
@mstgo = @Y(λr.λcnt.λit.λacc.λes.cnt(λcp.@lbdup(it, λit1.λit2.@scanbest(it1, es)(λpr.pr(λw.λx.r(cp, @lset(it2, x, @true), @sadd(acc, w), es)), acc)), acc))
@smstw = λn.λes.n(λnp.@sdup(np, λnp1.λnp2.@mstgo(np1, @lset(@srepl(@ss(np2), @false), @sz, @true), @sz, es)), @sz)
@zsub = λa.λb.@sdup(a, λa1.λa2.@sdup(b, λb1.λb2.@spair(@ssub(a1, b1), @ssub(b2, a2))))
@zmul = λx.λy.x(λp1.λn1.y(λp2.λn2.@sdup(p1, λp11.λp12.@sdup(n1, λn11.λn12.@sdup(p2, λp21.λp22.@sdup(n2, λn21.λn22.@spair(@sadd(@smul(p11, p21), @smul(n11, n21)), @sadd(@smul(p12, n22), @smul(n12, p22)))))))))
@zleq = λx.λy.x(λp1.λn1.y(λp2.λn2.@sleq(@sadd(p1, n2), @sadd(p2, n1))))
@crossle = λo.λa.λb.o(λox.λoy.a(λax.λay.b(λbx.λby.@zleq(@zmul(@zsub(ax, ox), @zsub(by, oy)), @zmul(@zsub(ay, oy), @zsub(bx, ox))))))
@plex = λp.λq.p(λa.λb.q(λc.λd.@sdup(a, λa1.λa2.@sdup(c, λc1.λc2.@ifc(@slt(a1, c1), @true, @ifc(@seq(a2, c2), @sleq(b, d), @false))))))
@peq = λp.λq.p(λa.λb.q(λc.λd.@andb(@seq(a, c), @seq(b, d))))
@sins = @Y(λr.λx.λl.l(λh.λt.@ifc(@plex(x, h), @scons(x, @scons(h, t)), @scons(h, r(x, t))), @scons(x, @snil)))
@ssortp = λl.@sfoldr(@sins, @snil, l)
@spdd = @Y(λr.λl.l(λh.λt.t(λh2.λt2.@ifc(@peq(h, h2), r(@scons(h2, t2)), @scons(h, r(@scons(h2, t2)))), @scons(h, @snil)), @snil))
@push = @Y(λr.λst.λp.st(λs1.λt1.t1(λs2.λt2.@ifc(@crossle(s2, s1, p), r(@scons(s2, t2), p), @scons(p, @scons(s1, @scons(s2, t2)))), @scons(p, @scons(s1, @snil))), @scons(p, @snil)))
@chgo = @Y(λr.λst.λl.l(λp.λt.r(@push(st, p), t), @srev(st)))
@sinit = λl.@srev(@stail(@srev(l)))
@shull = λl.@ifc(@sleq(@slen(@spdd(@ssortp(l))), @ss(@ss(@sz))), @spdd(@ssortp(l)), @sapp(@sinit(@chgo(@snil, @spdd(@ssortp(l)))), @sinit(@chgo(@snil, @srev(@spdd(@ssortp(l)))))))
@stake = @Y(λr.λn.λl.n(λp.l(λh.λt.@scons(h, r(p, t)), @snil), @snil))
@sdrop = @Y(λr.λn.λl.n(λp.l(λh.λt.r(p, t), @snil), l))
@chunksg = @Y(λr.λg.λl.l(λh.λt.@scons(@stake(@slen(@shead(g)), @scons(h, t)), r(g, @sdrop(@spred(@slen(@shead(g))), t))), @snil))
@sfind0 = @Y(λr.λi.λl.l(λh.λt.@ifc(@siszero(h), @some(i), r(@ss(i), t)), @none))
@grp = λn.λk.λi.λj.@ifc(@seq(@sdiv(i, n), @sdiv(j, n)), @true, @ifc(@seq(@smod(i, n), @smod(j, n)), @true, @andb(@seq(@sdiv(@sdiv(i, n), k), @sdiv(@sdiv(j, n), k)), @seq(@sdiv(@smod(i, n), k), @sdiv(@smod(j, n), k)))))
@validat = λg.λflat.λi.λd.@andl(@szipf(λidx.λval.@ifc(@andb(@seq(val, d), @grp(@slen(g), @ssqrt(@slen(g)), i, idx)), @false, @true), @srange0(@slen(flat)), flat))
@solveg = @Y(λr.λg.λflat.@sldup(flat, λf1.λf2.@sfind0(@sz, f1)(λi.@sfoldr(λd.λacc.@orelse(@sdup(d, λd1.λd2.@ifc(@validat(g, f2, i, d1), r(g, @lset(f2, i, d2)), @none)), acc), @none, @srange1(@slen(g))), @some(f2))))
@ssudoku = λg.@solveg(g, @sconcat(g))(λs.@sldup(s, λs1.λs2.(λa.λb.b)(s2, @chunksg(g, s1))), @snil)
@teq = @Y(λr.λx.λy.x(λn1.y(λn2.@seq(n1, n2), λc.λd.@false), λa1.λb1.y(λn.@false, λa2.λb2.@andb(r(a1, a2), r(b1, b2)))))
@tarr = λa.λb.λbase.λarr.arr(a, b)
@mnth = @Y(λr.λl.λi.l(λh.λt.i(λip.r(t, ip), @some(h)), @none))
@mbind = λm.λf.m(f, @none)
@tinf = @Y(λr.λctx.λt.t(λty.λbody.@mbind(r(@scons(ty, ctx), body), λbt.@some(@tarr(ty, bt))), λf.λx.@mbind(r(ctx, f), λft.@mbind(r(ctx, x), λxt.ft(λn.@none, λaa.λbb.@ifc(@teq(aa, xt), @some(bb), @none)))), λi.@mnth(ctx, i)))
@sstlcok = λt.@tinf(@snil, t)(λx.@true, @false)
@tlam = λb.λl.λa.λv.l(b)
@tapp = λf.λx.λl.λa.λv.a(f, x)
@tvar = λi.λl.λa.λv.v(i)
@tshift = @Y(λr.λd.λc.λt.t(λb.@tlam(r(d, @ss(c), b)), λf.λx.@tapp(r(d, c, f), r(d, c, x)), λi.@sdup(i, λi1.λi2.@ifc(@slt(i1, c), @tvar(i2), @tvar(@sadd(i2, d))))))
@tinst = @Y(λr.λj.λs.λt.t(λb.@tlam(r(@ss(j), s, b)), λf.λx.@tapp(r(j, s, f), r(j, s, x)), λi.@sdup(i, λi1.λi2.@sdup(j, λj1.λj2.@ifc(@slt(i1, j1), @tvar(i2), @ifc(@seq(i2, j2), @tshift(j, @sz, s), @tvar(@spred(i2))))))))
@tnf = @Y(λr.λt.t(λb.@tlam(r(b)), λf.λx.r(f)(λb.r(@tinst(@sz, x, b)), λg.λh.@tapp(@tapp(g, h), r(x)), λi.@tapp(@tvar(i), r(x))), λi.@tvar(i)))
@sbfrun0 = @Y(λr.λis.λL.λc.λR.λinp.λk.is(λins.λrest.ins(R(λrh.λrt.r(rest, @scons(c, L), rh, rt, inp, k), r(rest, @scons(c, L), @sz, @snil, inp, k)), L(λlh.λlt.r(rest, lt, lh, @scons(c, R), inp, k), r(rest, @snil, @sz, @scons(c, R), inp, k)), r(rest, L, @ss(c), R, inp, k), r(rest, L, @spred(c), R, inp, k), @sdup(c, λc1.λc2.@scons(c1, r(rest, L, c2, R, inp, k))), inp(λih.λit.r(rest, L, ih, R, it, k), r(rest, L, @sz, R, @snil, k)), λbody.@Y(λlp.λL1.λc1.λR1.λinp1.λkk.@sdup(c1, λca.λcb.ca(λp.r(body, L1, cb, R1, inp1, λL2.λc2.λR2.λinp2.lp(L2, c2, R2, inp2, kk)), kk(L1, @sz, R1, inp1))))(L, c, R, inp, λL2.λc2.λR2.λinp2.r(rest, L2, c2, R2, inp2, k))), k(L, c, R, inp)))
@sbfrun = λprog.λinp.@sbfrun0(prog, @snil, @sz, @snil, inp, λL.λc.λR.λi.@snil)
";

/// GN-number library for the fft tasks (balanced ternary + the w-tower).
/// Emitted for tree-family tasks whose decode produced a Gn kind.
pub const GN_LIB: &str = "\
@spair = λx.λy.λp.p(x, y)
@btE = λt.λo.λi.λe.e
@btT = λx.λt.λo.λi.λe.t(x)
@btO = λx.λt.λo.λi.λe.o(x)
@btI = λx.λt.λo.λi.λe.i(x)
@srange0 = λn.@srev(@smapf(@spred, @srange1(n)))
@bt2z = @Y(λr.λb.b(λx.r(x)(λp.λn.@spair(@smul(@ss(@ss(@ss(@sz))), p), @ss(@smul(@ss(@ss(@ss(@sz))), n)))), λx.r(x)(λp.λn.@spair(@smul(@ss(@ss(@ss(@sz))), p), @smul(@ss(@ss(@ss(@sz))), n))), λx.r(x)(λp.λn.@spair(@ss(@smul(@ss(@ss(@ss(@sz))), p)), @smul(@ss(@ss(@ss(@sz))), n))), @spair(@sz, @sz)))
@btneg = @Y(λr.λb.b(λx.@btI(r(x)), λx.@btO(r(x)), λx.@btT(r(x)), @btE))
@pos2bt = @Y(λr.λm.m(λmp.@sdup(@ss(mp), λm1.λm2.@sdup(@smod(m1, @ss(@ss(@ss(@sz)))), λd1.λd2.@ifc(@siszero(d1), @btO(r(@sdiv(m2, @ss(@ss(@ss(@sz)))))), @ifc(@seq(d2, @ss(@sz)), @btI(r(@sdiv(m2, @ss(@ss(@ss(@sz)))))), @btT(r(@sdiv(@ss(m2), @ss(@ss(@ss(@sz)))))))))), @btE))
@z2bt = λz.z(λp.λn.@sdup(p, λp1.λp2.@sdup(n, λn1.λn2.@ifc(@sleq(n1, p1), @pos2bt(@ssub(p2, n2)), @btneg(@pos2bt(@ssub(n2, p2)))))))
@btadd = λa.λb.@z2bt(@bt2z(a)(λp1.λn1.@bt2z(b)(λp2.λn2.@spair(@sadd(p1, p2), @sadd(n1, n2)))))
@gneg = @Y(λr.λg.g(λa.λb.@stnode(r(a), r(b)), λx.@stleaf(@btneg(x))))
@gmulw = @Y(λr.λg.g(λa.λb.@stnode(r(b), a), λx.@stleaf(@btneg(x))))
@gadd = @Y(λr.λx.λy.x(λa.λb.y(λc.λd.@stnode(r(a, c), r(b, d)), @stleaf(@btE)), λu.y(λc.λd.@stleaf(@btE), λv.@stleaf(@btadd(u, v)))))
@gnpow = @Y(λr.λe.λg.e(λep.r(ep, @gmulw(g)), g))
@gdepth = @Y(λr.λg.g(λa.λb.@ss(r(a)), λx.@sz))
@gflatk = @Y(λr.λk.λg.k(λkp.@sdup(kp, λk1.λk2.g(λa.λb.@sapp(r(k1, a), r(k2, b)), @snil)), g(λa.λb.@snil, λx.@scons(x, @snil))))
@gsum = λl.l(λh.λt.@sfoldr(@gadd, h, t), @stleaf(@btE))
@gndft0 = λt.@sdup(@gdepth(t), λk1.λk2.@sdup(@spow(@ss(@ss(@sz)), k1), λn1.λn2.@smapf(λm.@gsum(@szipf(λj.λc.@gnpow(@smod(@smul(m, j), n1), c), @srange0(n2), @sbrp(@gflatk(k2, t)))), @srange0(@spow(@ss(@ss(@sz)), @gdepth(t))))))
@gndft = λt.@stbuild(@sbrp(@gndft0(t)))
@gndftn = λt.@stbuild(@gndft0(t))
@cbtT = λx.λt.λo.λi.λe.t(x(t, o, i, e))
@cbtO = λx.λt.λo.λi.λe.o(x(t, o, i, e))
@cbtI = λx.λt.λo.λi.λe.i(x(t, o, i, e))
@cbtE = λt.λo.λi.λe.e
@cbt2s = λx.x(@btT, @btO, @btI, @btE)
@sbt2c = @Y(λr.λb.b(λx.@cbtT(r(x)), λx.@cbtO(r(x)), λx.@cbtI(r(x)), @cbtE))
@gnh = λa.λb.λn.λl.@stnode(a(n, l), b(n, l))
@glh0 = λx.λn.λl.@stleaf(@cbt2s(x))
@gcv0 = λt.t(@gnh, @glh0, @gnh, @glh0)
@glh1 = λx.λn.λl.@stleaf(@gcv0(x))
@gnc2s = λt.t(@gnh, @glh1, @gnh, @glh1)
@gsub0 = @Y(λr.λg.g(λa.λb.λn.λl.n(r(a), r(b), n, l), λx.λn.λl.l(@sbt2c(x), n, l)))
@groot0 = λg.g(λa.λb.λn.λl.n(@gsub0(a), @gsub0(b)), λx.λn.λl.l(@sbt2c(x)))
@gsub1 = @Y(λr.λg.g(λa.λb.λn.λl.n(r(a), r(b), n, l), λx.λn.λl.l(@groot0(x), n, l)))
@gns2c = λg.g(λa.λb.λn.λl.n(@gsub1(a), @gsub1(b)), λx.λn.λl.l(@groot0(x)))
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
        Op::AdtLen => "@adtlen",
        Op::AdtIdx => "@adtidx",
        Op::AdtChi => "@adtchi",
        Op::AdtRev => "@adtrev",
        Op::AdtEq => "@adteq",
        Op::AdtSer => "@adtser",
        Op::AdtDes => "@adtdesv",
        Op::AdtMrg => "@adtmrg",
        Op::Fst => "@sfst",
        Op::Snd => "@ssnd",
        Op::Range0 => "@srange0",
        Op::SumL => "@ssuml",
        Op::MinL => "@sminl",
        Op::Perms => "@sperms",
        Op::MapB => "@smapb",
        Op::CycleCost => "@scyclecost",
        Op::SatCnf => "@ssatcnf",
        Op::LineRas => "@slineras",
        Op::GridBfs => "@sgridbfs",
        Op::MstW => "@smstw",
        Op::Hull => "@shull",
        Op::Sudoku => "@ssudoku",
        Op::StlcOk => "@sstlcok",
        Op::LamNf => "@tnf",
        Op::BfRun => "@sbfrun",
        Op::GnDft => "@gndft",
        Op::GnDftN => "@gndftn",
    }
}

/// Ops whose stdlib implementation takes the type descriptor as a leading
/// extra argument that the DSL-level op elides.
fn needs_desc(op: Op) -> bool {
    matches!(
        op,
        Op::AdtLen | Op::AdtIdx | Op::AdtChi | Op::AdtRev | Op::AdtEq | Op::AdtMrg
    )
}

/// Emit the expression as Lamb source. `arg_name(i)` supplies the source
/// name for task argument i (already adapted to Church encoding).
fn emit(e: &E, n_args: usize, arg_name: &dyn Fn(u32) -> String, desc: Option<usize>) -> String {
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
        E::Lam1(b) => format!("λp.{}", emit(b, n_args, arg_name, desc)),
        E::Prim(op, args) => {
            let mut parts: Vec<String> = Vec::new();
            if needs_desc(*op) {
                let d = desc.expect("adt op outside adt family");
                parts.push(format!("a{d}"));
            }
            parts.extend(args.iter().map(|a| emit(a, n_args, arg_name, desc)));
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
            (Family::Algo, _) => a,
            (Family::CTre, ArgKind::Gn) => format!("@gnc2s({a})"),
            (_, ArgKind::Gn) | (_, ArgKind::Raw) => a,
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
            (_, ArgKind::Desc) | (_, ArgKind::Bits) => a,
            (Family::CAdt, ArgKind::AdtVal) => format!("@c2sadt(a{size_idx}, {a})"),
            (_, ArgKind::AdtVal) => a,
            (_, ArgKind::Atom) => a,
        }
    };
    let is_adt = matches!(family, Family::CAdt | Family::SAdt);
    let core = emit(e, n_args, &adapted, is_adt.then_some(size_idx));
    let wrapped = match (family, out) {
        (Family::Algo, _) => core,
        (Family::CTre, OutKind::Gn) => format!("@gns2c({core})"),
        (_, OutKind::Gn) => core,
        (Family::SNat | Family::SList | Family::CAdt | Family::SAdt, OutKind::Nat) => core,
        (Family::CBin, OutKind::Nat) => format!("@s2cb({core})"),
        (Family::SBin, OutKind::Nat) => format!("@s2sb({core})"),
        (_, OutKind::Nat) => format!("@s2c({core})"),
        (Family::SList, OutKind::List) => core,
        (_, OutKind::List) => format!("@s2cl({core})"),
        (_, OutKind::ListS) => core,
        (_, OutKind::Tuple) => format!("@list2tup({core})"),
        (Family::CTre, OutKind::Tree) => format!("@st2ct({core})"),
        (_, OutKind::Tree) => core,
        (_, OutKind::AdtC) => format!("@s2cadt(a{size_idx}, {core})"),
        (_, OutKind::AdtS) => core,
        (_, OutKind::ListAdtC) => format!("@smapf(@s2cadt(a{size_idx}), {core})"),
        (_, OutKind::ListAdtS) => core,
        (_, OutKind::Bits) => core,
        (_, OutKind::Bool) | (_, OutKind::Raw) => core,
    };
    let mut lams = String::new();
    for i in 0..n_args {
        lams.push_str(&format!("λa{i}."));
    }
    let mut lib = String::from(STDLIB);
    if matches!(family, Family::Algo) {
        lib.push_str(ALGO_LIB);
    }
    if kinds.contains(&ArgKind::Gn) || matches!(out, OutKind::Gn) {
        lib.push_str(GN_LIB);
    }
    format!("{lib}@main = {lams}{wrapped}\n")
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
    if matches!(family, Family::CAdt | Family::SAdt) {
        return None; // adt families use decode_task_adt (multi-candidate)
    }
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
    // GN-number trees (fft): Scott side decodes via the family tree decoder
    // with all-Int leaves; the Church side has its own self-passing grammar.
    let dec_gn = |t: &Rc<Term>| -> Option<V> {
        fn leaves_int(v: &V) -> bool {
            match v {
                V::Node(a, b) => leaves_int(a) && leaves_int(b),
                V::Int(_) => true,
                _ => false,
            }
        }
        let v = match family {
            Family::CTre => crate::decode::church_gn(t)?,
            _ => dec_tree(t)?,
        };
        if matches!(v, V::Node(_, _)) && leaves_int(&v) {
            Some(v)
        } else {
            None
        }
    };
    // Algo-family recipe decoders (everything Scott-encoded).
    use crate::decode::{
        d_bf_instr, d_bool as db, d_lam_term, d_lit, d_slist_of, d_snat, d_stlc_term, d_tup_of,
    };
    let dec_list_nat = |t: &Rc<Term>| d_slist_of(t, &d_snat);
    let dec_grid_bool = |t: &Rc<Term>| d_slist_of(t, &|r| d_slist_of(r, &db));
    let dec_grid_nat = |t: &Rc<Term>| d_slist_of(t, &|r| d_slist_of(r, &d_snat));
    let dec_pair_nat = |t: &Rc<Term>| d_tup_of(t, 2, &d_snat);
    let dec_list_pair = |t: &Rc<Term>| d_slist_of(t, &|p| d_tup_of(p, 2, &d_snat));
    let dec_edges = |t: &Rc<Term>| d_slist_of(t, &|p| d_tup_of(p, 3, &d_snat));
    let dec_cnf = |t: &Rc<Term>| d_slist_of(t, &|c| d_slist_of(c, &d_lit));
    let dec_bfprog = |t: &Rc<Term>| d_slist_of(t, &d_bf_instr);
    let dec_lam = |t: &Rc<Term>| d_lam_term(t);
    let dec_stlc = |t: &Rc<Term>| d_stlc_term(t);
    let dec_snat_strict = |t: &Rc<Term>| d_snat(t);
    let dec_bool_strict = |t: &Rc<Term>| db(t);

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
                (ArgKind::Gn, &dec_gn),
                (ArgKind::Tree, &dec_tree),
                (ArgKind::Atom, &dec_atomish),
            ],
            Family::Algo => vec![
                (ArgKind::Raw, &dec_snat_strict),
                (ArgKind::Raw, &dec_bool_strict),
                (ArgKind::Raw, &dec_grid_bool),
                (ArgKind::Raw, &dec_cnf),
                (ArgKind::Raw, &dec_grid_nat),
                (ArgKind::Raw, &dec_list_nat),
                (ArgKind::Raw, &dec_pair_nat),
                (ArgKind::Raw, &dec_list_pair),
                (ArgKind::Raw, &dec_edges),
                (ArgKind::Raw, &dec_bfprog),
                (ArgKind::Raw, &dec_lam),
                (ArgKind::Raw, &dec_stlc),
                (ArgKind::Raw, &dec_atomish),
            ],
            Family::CAdt | Family::SAdt => unreachable!("adt uses decode_task_adt"),
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
            (OutKind::Gn, &dec_gn),
            (OutKind::Tree, &dec_tree),
            (OutKind::List, &dec_clist),
            (OutKind::ListS, &dec_slist),
            (OutKind::Raw, &dec_atomish),
        ],
        Family::STre => vec![
            (OutKind::Gn, &dec_gn),
            (OutKind::Tree, &dec_tree),
            (OutKind::ListS, &dec_slist),
            (OutKind::List, &dec_clist),
            (OutKind::Raw, &dec_atomish),
        ],
        Family::Algo => vec![
            (OutKind::Raw, &dec_snat_strict),
            (OutKind::Raw, &dec_bool_strict),
            (OutKind::Raw, &dec_grid_nat),
            (OutKind::Raw, &dec_list_nat),
            (OutKind::Raw, &dec_list_pair),
            (OutKind::Raw, &dec_lam),
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

/// ADT-family decoding. The descriptor argument is found first; values are
/// then decoded against each test's own descriptor. Output-kind candidates
/// are returned in order — Church/Scott zero-field values are syntactically
/// identical, so the caller tries each candidate until one verifies.
pub fn decode_task_adt(
    family: Family,
    task: &Task,
) -> Vec<(Vec<Vec<V>>, Vec<V>, Vec<ArgKind>, OutKind, usize)> {
    let empty: Env = Rc::new(Vec::new());
    let norm = |t: &Rc<Term>| -> Option<Rc<Term>> {
        let mut fuel = Fuel(1_000_000);
        normalize(&empty, t, &mut fuel).ok()
    };
    let mut arg_nfs: Vec<Vec<Rc<Term>>> = Vec::new();
    let mut want_nfs: Vec<Rc<Term>> = Vec::new();
    for t in &task.tests {
        let mut row = Vec::new();
        for a in &t.args {
            let Some(nf) = norm(a) else { return Vec::new() };
            row.push(nf);
        }
        arg_nfs.push(row);
        let Some(w) = norm(&t.want).and_then(|nf| crate::parse::strip_outer(&nf, t.outer)) else {
            return Vec::new();
        };
        want_nfs.push(w);
    }
    let n_args = task.arity;
    let n_tests = task.tests.len();

    // Find the descriptor position.
    let mut desc_pos = None;
    let mut shapes: Vec<Vec<usize>> = Vec::new();
    for p in 0..n_args {
        let sh: Option<Vec<Vec<usize>>> = (0..n_tests)
            .map(|j| crate::decode::adt_desc(&arg_nfs[j][p]))
            .collect();
        if let Some(sh) = sh {
            desc_pos = Some(p);
            shapes = sh;
            break;
        }
    }
    let Some(desc_pos) = desc_pos else { return Vec::new() };
    let desc_val = |shape: &[usize]| -> V {
        V::List(
            shape
                .iter()
                .map(|k| V::List(vec![V::Nat(0); *k]))
                .collect(),
        )
    };
    let fam_adt = |shape: &[usize], t: &Rc<Term>| -> Option<V> {
        match family {
            Family::CAdt => crate::decode::church_adt(shape, t),
            _ => crate::decode::scott_adt(shape, t),
        }
    };
    let dec_bool_v = |t: &Rc<Term>| -> Option<V> {
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
    let dec_bits = |t: &Rc<Term>| -> Option<V> {
        let items = crate::decode::scott_list_raw(t)?;
        let mut out = Vec::new();
        for i in items {
            out.push(dec_bool_v(&i)?);
        }
        Some(V::List(out))
    };

    // Argument kinds and values.
    let mut kinds: Vec<ArgKind> = Vec::new();
    let mut cols: Vec<Vec<V>> = Vec::new();
    for p in 0..n_args {
        if p == desc_pos {
            kinds.push(ArgKind::Desc);
            cols.push(shapes.iter().map(|s| desc_val(s)).collect());
            continue;
        }
        let val: Option<Vec<V>> = (0..n_tests)
            .map(|j| fam_adt(&shapes[j], &arg_nfs[j][p]))
            .collect();
        if let Some(vals) = val {
            kinds.push(ArgKind::AdtVal);
            cols.push(vals);
            continue;
        }
        let nat: Option<Vec<V>> = (0..n_tests)
            .map(|j| Some(V::Nat(crate::decode::scott_nat(&arg_nfs[j][p])?)))
            .collect();
        if let Some(vals) = nat {
            kinds.push(ArgKind::Nat);
            cols.push(vals);
            continue;
        }
        let bits: Option<Vec<V>> = (0..n_tests).map(|j| dec_bits(&arg_nfs[j][p])).collect();
        if let Some(vals) = bits {
            kinds.push(ArgKind::Bits);
            cols.push(vals);
            continue;
        }
        let atom: Option<Vec<V>> = (0..n_tests)
            .map(|j| crate::decode::decode_value(&arg_nfs[j][p]))
            .collect();
        if let Some(vals) = atom {
            kinds.push(ArgKind::Atom);
            cols.push(vals);
            continue;
        }
        return Vec::new();
    }
    let inputs: Vec<Vec<V>> = (0..n_tests)
        .map(|j| (0..n_args).map(|p| cols[p][j].clone()).collect())
        .collect();

    // Output-kind candidates, in preference order for the family.
    let church_out = |j: usize| crate::decode::church_adt(&shapes[j], &want_nfs[j]);
    let scott_out = |j: usize| crate::decode::scott_adt(&shapes[j], &want_nfs[j]);
    let list_of = |j: usize, church: bool| -> Option<V> {
        let items = crate::decode::scott_list_raw(&want_nfs[j])?;
        let mut out = Vec::new();
        for i in items {
            out.push(if church {
                crate::decode::church_adt(&shapes[j], &i)?
            } else {
                crate::decode::scott_adt(&shapes[j], &i)?
            });
        }
        Some(V::List(out))
    };

    let mut cands: Vec<(OutKind, Vec<V>)> = Vec::new();
    let mut push = |k: OutKind, vals: Option<Vec<V>>| {
        if let Some(v) = vals {
            cands.push((k, v));
        }
    };
    let all = |f: &dyn Fn(usize) -> Option<V>| -> Option<Vec<V>> {
        (0..n_tests).map(f).collect()
    };
    match family {
        Family::CAdt => {
            push(OutKind::AdtC, all(&church_out));
            push(OutKind::AdtS, all(&scott_out));
        }
        _ => {
            push(OutKind::AdtS, all(&scott_out));
            push(OutKind::AdtC, all(&church_out));
        }
    }
    push(OutKind::Nat, all(&|j| {
        Some(V::Nat(crate::decode::scott_nat(&want_nfs[j])?))
    }));
    push(OutKind::Bool, all(&|j| dec_bool_v(&want_nfs[j])));
    push(OutKind::Bits, all(&|j| dec_bits(&want_nfs[j])));
    push(OutKind::ListAdtC, all(&|j| list_of(j, true)));
    push(OutKind::ListAdtS, all(&|j| list_of(j, false)));

    cands
        .into_iter()
        .map(|(k, outs)| (inputs.clone(), outs, kinds.clone(), k, desc_pos))
        .collect()
}
