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
    // ── ADT ops (values are V::Ctr constructor trees) ──
    AdtLen,   // total constructor count
    AdtIdx,   // outermost constructor tag
    AdtChi,   // recursive children of the root, as a list
    AdtRev,   // reverse every constructor's fields recursively
    AdtEq,    // structural equality
    AdtSer,   // AdtSer(desc, v): bits — ceil(log2 N) tag bits MSB-first, then children
    AdtDes,   // AdtDes(desc, bits): parse back
    AdtMrg,   // AdtMrg(f, a, b): recurse same-ctor-with-children, else f(a, b)
    // ── algo-track ops: tuples, combinatorics, evaluated lambdas ──
    Fst,      // first of a Tup
    Snd,      // second of a Tup
    Range0,   // Range0(n) = [0..n-1]
    SumL,     // sum of a nat list
    MinL,     // minimum of a nat list (None on empty)
    Perms,    // all permutations of a list
    MapB,     // MapB(list, λx.body) — evaluated map
    CycleCost, // CycleCost(matrix, path) = Σ m[p_i][p_{(i+1) mod n}]
    // ── algorithm-library primitives (each = one stdlib routine) ──
    SatCnf,   // DIMACS-style CNF (list of list of nonzero Int) → Bool
    LineRas,  // Bresenham raster from Tup2 to Tup2 (x1≥x0, y1≥y0)
    GridBfs,  // shortest path length through a bool grid, corners, 4-dir
    MstW,     // MstW(n, edges as Tup3 list) = total MST weight
    Hull,     // convex hull of Tup2 points, CCW from lex-min, strict vertices
    Sudoku,   // complete a k²×k² latin grid with box constraint (0 = empty)
    StlcOk,   // STLC type check in empty context → Bool
    LamNf,    // β-normal form of a de Bruijn λ term (Ctr encoding)
    BfRun,    // brainfuck: BfRun(prog, input) → output list
    GnDft,    // DFT over the GN number tower (collapsed L/B tree, Int leaves)
    GnDftN,   // same, output storage in natural order (ctre convention)
}

impl Op {
    /// (value-arity, lambda-arity). Lambda args are 1-parameter bodies.
    pub fn sig(self) -> (usize, usize) {
        match self {
            Op::Isqrt | Op::IsZero | Op::Not | Op::Range1 => (1, 0),
            Op::Head | Op::Last | Op::Rev | Op::RotL | Op::RotR | Op::Len | Op::SortB => (1, 0),
            Op::TFlat | Op::TBfs | Op::TMirror | Op::TBuild | Op::TBitRev => (1, 0),
            Op::AdtLen | Op::AdtIdx | Op::AdtChi | Op::AdtRev => (1, 0),
            Op::If | Op::ZipAp | Op::FoldrAp | Op::TMergeAp | Op::TScanAp | Op::AdtMrg => (3, 0),
            Op::Count | Op::MapB => (1, 1),
            Op::Fst | Op::Snd | Op::Range0 | Op::SumL | Op::MinL | Op::Perms => (1, 0),
            Op::SatCnf | Op::GridBfs | Op::Hull | Op::Sudoku | Op::StlcOk | Op::LamNf
            | Op::GnDft | Op::GnDftN => (1, 0),
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
            Op::AdtLen,
            Op::AdtIdx,
            Op::AdtChi,
            Op::AdtRev,
            Op::AdtEq,
            Op::AdtSer,
            Op::AdtDes,
            Op::AdtMrg,
            Op::Fst,
            Op::Snd,
            Op::Range0,
            Op::SumL,
            Op::MinL,
            Op::Perms,
            Op::MapB,
            Op::CycleCost,
            Op::SatCnf,
            Op::LineRas,
            Op::GridBfs,
            Op::MstW,
            Op::Hull,
            Op::Sudoku,
            Op::StlcOk,
            Op::LamNf,
            Op::BfRun,
            Op::GnDft,
            Op::GnDftN,
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

fn adt_len(v: &V) -> Option<u64> {
    match v {
        V::Ctr(_, fields) => {
            let mut n = 1u64;
            for f in fields {
                n += adt_len(f)?;
            }
            Some(n)
        }
        _ => None,
    }
}

fn adt_rev(v: &V) -> Option<V> {
    match v {
        V::Ctr(tag, fields) => {
            let mut fs: Vec<V> = fields.iter().map(adt_rev).collect::<Option<_>>()?;
            fs.reverse();
            Some(V::Ctr(*tag, fs))
        }
        _ => None,
    }
}

/// Descriptor value: List of List (of anything) → field counts per ctor.
fn desc_shape(v: &V) -> Option<Vec<usize>> {
    let specs = list1(v)?;
    specs.iter().map(|s| Some(list1(s)?.len())).collect()
}

fn ceil_log2(n: usize) -> u32 {
    let mut w = 0u32;
    while (1usize << w) < n {
        w += 1;
    }
    w
}

fn adt_ser(shape: &[usize], v: &V, out: &mut Vec<V>) -> Option<()> {
    let V::Ctr(tag, fields) = v else { return None };
    let w = ceil_log2(shape.len().max(1));
    for b in (0..w).rev() {
        out.push(V::Bool((*tag as usize >> b) & 1 == 1));
    }
    for f in fields {
        adt_ser(shape, f, out)?;
    }
    Some(())
}

fn adt_des(shape: &[usize], bits: &[V], pos: &mut usize) -> Option<V> {
    let w = ceil_log2(shape.len().max(1));
    // A 1-ctor shape with fields consumes no tag bits: the recursion would
    // never make progress (infinite descent → stack overflow).
    if w == 0 && shape.first().is_some_and(|k| *k > 0) {
        return None;
    }
    let mut tag = 0usize;
    for _ in 0..w {
        let b = bits.get(*pos)?;
        *pos += 1;
        let bit = match b {
            V::Bool(x) => *x,
            V::Nat(0) => false,
            _ => return None,
        };
        tag = tag << 1 | usize::from(bit);
    }
    let k = *shape.get(tag)?;
    let mut fields = Vec::new();
    for _ in 0..k {
        fields.push(adt_des(shape, bits, pos)?);
    }
    Some(V::Ctr(tag as u32, fields))
}

fn adt_mrg(f: &V, a: &V, b: &V) -> Option<V> {
    match (a, b) {
        (V::Ctr(ta, fa), V::Ctr(tb, _)) if ta == tb && !fa.is_empty() => {
            let V::Ctr(_, fb) = b else { unreachable!() };
            if fa.len() != fb.len() {
                return None;
            }
            let fields: Vec<V> = fa
                .iter()
                .zip(fb)
                .map(|(x, y)| adt_mrg(f, x, y))
                .collect::<Option<_>>()?;
            Some(V::Ctr(*ta, fields))
        }
        (V::Ctr(_, _), V::Ctr(_, _)) => Some(V::App(
            Box::new(V::App(Box::new(f.clone()), Box::new(a.clone()))),
            Box::new(b.clone()),
        )),
        _ => None,
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

// ── algo-track helpers ──────────────────────────────────────────────

fn tup(v: &V) -> Option<&[V]> {
    match v {
        V::Tup(xs) => Some(xs),
        _ => None,
    }
}

fn nat_list(v: &V) -> Option<Vec<u64>> {
    list1(v)?.iter().map(nat).collect()
}

fn nat_grid(v: &V) -> Option<Vec<Vec<u64>>> {
    list1(v)?.iter().map(nat_list).collect()
}

fn perms_of(xs: &[V]) -> Option<Vec<V>> {
    if xs.len() > 7 {
        return None;
    }
    fn go(rest: Vec<V>, acc: &mut Vec<V>, out: &mut Vec<V>) {
        if rest.is_empty() {
            out.push(V::List(acc.clone()));
            return;
        }
        for i in 0..rest.len() {
            let mut r = rest.clone();
            let x = r.remove(i);
            acc.push(x);
            go(r, acc, out);
            acc.pop();
        }
    }
    let mut out = Vec::new();
    go(xs.to_vec(), &mut Vec::new(), &mut out);
    Some(out)
}

fn cycle_cost(m: &[Vec<u64>], p: &[u64]) -> Option<u64> {
    let n = p.len();
    let mut total = 0u64;
    for i in 0..n {
        let a = *p.get(i)? as usize;
        let b = p[(i + 1) % n] as usize;
        total = total.checked_add(*m.get(a)?.get(b)?)?;
    }
    Some(total)
}

/// CNF: list of clauses, each a list of nonzero ints (±(var+1)).
fn sat_cnf(clauses: &[Vec<i64>]) -> Option<bool> {
    let mut maxv = 0i64;
    for c in clauses {
        for &l in c {
            if l == 0 {
                return None;
            }
            maxv = maxv.max(l.abs());
        }
    }
    if maxv > 20 {
        return None;
    }
    let nv = maxv as u32;
    for asn in 0..(1u64 << nv) {
        let ok = clauses.iter().all(|c| {
            c.iter().any(|&l| {
                let bit = asn >> (l.abs() - 1) & 1 == 1;
                if l > 0 {
                    bit
                } else {
                    !bit
                }
            })
        });
        if ok {
            return Some(true);
        }
    }
    Some(false)
}

/// Bresenham restricted to x1≥x0, y1≥y0, endpoints inclusive.
/// Pixel k of the shallow case: y = y0 + floor((2k·dy + dx) / (2dx)).
fn line_raster(p0: (u64, u64), p1: (u64, u64)) -> Option<Vec<V>> {
    let (x0, y0) = p0;
    let (x1, y1) = p1;
    if x1 < x0 || y1 < y0 {
        return None;
    }
    let (dx, dy) = (x1 - x0, y1 - y0);
    if dx.max(dy) > 4096 {
        return None;
    }
    let mut out = Vec::new();
    if dx >= dy {
        for k in 0..=dx {
            let y = if dx == 0 { y0 } else { y0 + (2 * k * dy + dx - 1) / (2 * dx) };
            out.push(V::Tup(vec![V::Nat(x0 + k), V::Nat(y)]));
        }
    } else {
        for k in 0..=dy {
            let x = x0 + (2 * k * dx + dy - 1) / (2 * dy);
            out.push(V::Tup(vec![V::Nat(x), V::Nat(y0 + k)]));
        }
    }
    Some(out)
}

fn bool_grid(v: &V) -> Option<Vec<Vec<bool>>> {
    list1(v)?
        .iter()
        .map(|r| list1(r)?.iter().map(boolean).collect())
        .collect()
}

/// Shortest 4-dir path length (0,0) → (H-1,W-1) through `true` cells; 0 if
/// unreachable.
fn grid_bfs(g: &[Vec<bool>]) -> Option<u64> {
    let h = g.len();
    let w = g.first()?.len();
    if w == 0 || g.iter().any(|r| r.len() != w) {
        return None;
    }
    let mut dist = vec![vec![u64::MAX; w]; h];
    let mut queue = std::collections::VecDeque::new();
    if !g[0][0] {
        return Some(0);
    }
    dist[0][0] = 0;
    queue.push_back((0usize, 0usize));
    while let Some((i, j)) = queue.pop_front() {
        if i == h - 1 && j == w - 1 {
            return Some(dist[i][j]);
        }
        let d = dist[i][j];
        let mut push = |ni: usize, nj: usize, dist: &mut Vec<Vec<u64>>,
                        queue: &mut std::collections::VecDeque<(usize, usize)>| {
            if g[ni][nj] && dist[ni][nj] == u64::MAX {
                dist[ni][nj] = d + 1;
                queue.push_back((ni, nj));
            }
        };
        if i > 0 {
            push(i - 1, j, &mut dist, &mut queue);
        }
        if i + 1 < h {
            push(i + 1, j, &mut dist, &mut queue);
        }
        if j > 0 {
            push(i, j - 1, &mut dist, &mut queue);
        }
        if j + 1 < w {
            push(i, j + 1, &mut dist, &mut queue);
        }
    }
    Some(0)
}

/// Prim's MST total weight over an undirected edge list.
fn mst_w(n: u64, edges: &[(u64, u64, u64)]) -> Option<u64> {
    let n = n as usize;
    if n == 0 {
        return Some(0);
    }
    let mut intree = vec![false; n];
    intree[0] = true;
    let mut total = 0u64;
    for _ in 1..n {
        let mut best: Option<(u64, usize)> = None;
        for &(u, v, w) in edges {
            let (u, v) = (u as usize, v as usize);
            if u >= n || v >= n {
                return None;
            }
            let newv = match (intree[u], intree[v]) {
                (true, false) => v,
                (false, true) => u,
                _ => continue,
            };
            if best.map_or(true, |(bw, _)| w < bw) {
                best = Some((w, newv));
            }
        }
        let (w, v) = best?;
        intree[v] = true;
        total += w;
    }
    Some(total)
}

/// Andrew's monotone chain, strict vertices, CCW from the lex-min point.
fn hull(pts: &[(i64, i64)]) -> Vec<(i64, i64)> {
    let mut ps: Vec<(i64, i64)> = pts.to_vec();
    ps.sort();
    ps.dedup();
    if ps.len() <= 2 {
        return ps;
    }
    let cross = |o: (i64, i64), a: (i64, i64), b: (i64, i64)| -> i64 {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };
    let mut build = |it: &mut dyn Iterator<Item = &(i64, i64)>| -> Vec<(i64, i64)> {
        let mut st: Vec<(i64, i64)> = Vec::new();
        for &p in it {
            while st.len() >= 2 && cross(st[st.len() - 2], st[st.len() - 1], p) <= 0 {
                st.pop();
            }
            st.push(p);
        }
        st
    };
    let lower = build(&mut ps.iter());
    let mut upper = build(&mut ps.iter().rev());
    let mut out = lower;
    out.pop();
    upper.pop();
    out.extend(upper);
    out
}

/// Complete a k²×k² grid (0 = empty) so each row/col/box holds 1..k² once.
fn sudoku(g: &[Vec<u64>]) -> Option<Vec<Vec<u64>>> {
    let n = g.len();
    let k = (n as f64).sqrt() as usize;
    if k * k != n || g.iter().any(|r| r.len() != n) || n > 9 {
        return None;
    }
    let mut grid: Vec<Vec<u64>> = g.to_vec();
    fn valid(grid: &[Vec<u64>], k: usize, i: usize, j: usize, d: u64) -> bool {
        let n = k * k;
        for t in 0..n {
            if grid[i][t] == d || grid[t][j] == d {
                return false;
            }
        }
        let (bi, bj) = (i / k * k, j / k * k);
        for a in bi..bi + k {
            for b in bj..bj + k {
                if grid[a][b] == d {
                    return false;
                }
            }
        }
        true
    }
    fn go(grid: &mut Vec<Vec<u64>>, k: usize) -> bool {
        let n = k * k;
        for i in 0..n {
            for j in 0..n {
                if grid[i][j] == 0 {
                    for d in 1..=n as u64 {
                        if valid(grid, k, i, j, d) {
                            grid[i][j] = d;
                            if go(grid, k) {
                                return true;
                            }
                            grid[i][j] = 0;
                        }
                    }
                    return false;
                }
            }
        }
        true
    }
    if go(&mut grid, k) {
        Some(grid)
    } else {
        None
    }
}

// STLC terms: Ctr(0,[ty,body]) Lam, Ctr(1,[f,x]) App, Ctr(2,[Nat]) Var.
// Types: Ctr(0,[Nat]) Base, Ctr(1,[a,b]) Arr.
fn stlc_infer(ctx: &mut Vec<V>, t: &V) -> Option<V> {
    match t {
        V::Ctr(0, fs) if fs.len() == 2 => {
            ctx.push(fs[0].clone());
            let b = stlc_infer(ctx, &fs[1]);
            ctx.pop();
            Some(V::Ctr(1, vec![fs[0].clone(), b?]))
        }
        V::Ctr(1, fs) if fs.len() == 2 => {
            let f = stlc_infer(ctx, &fs[0])?;
            let x = stlc_infer(ctx, &fs[1])?;
            match f {
                V::Ctr(1, ab) if ab.len() == 2 && ab[0] == x => Some(ab[1].clone()),
                _ => None,
            }
        }
        V::Ctr(2, fs) if fs.len() == 1 => {
            let i = nat(&fs[0])? as usize;
            let l = ctx.len();
            if i < l {
                Some(ctx[l - 1 - i].clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

// de Bruijn λ terms: Ctr(0,[body]) Lam, Ctr(1,[f,a]) App, Ctr(2,[Nat]) Var.
fn db_shift(d: i64, c: u64, t: &V) -> Option<V> {
    match t {
        V::Ctr(0, fs) if fs.len() == 1 => Some(V::Ctr(0, vec![db_shift(d, c + 1, &fs[0])?])),
        V::Ctr(1, fs) if fs.len() == 2 => Some(V::Ctr(
            1,
            vec![db_shift(d, c, &fs[0])?, db_shift(d, c, &fs[1])?],
        )),
        V::Ctr(2, fs) if fs.len() == 1 => {
            let k = nat(&fs[0])?;
            let k2 = if k >= c { (k as i64 + d).try_into().ok()? } else { k };
            Some(V::Ctr(2, vec![V::Nat(k2)]))
        }
        _ => None,
    }
}

/// Substitute `s` for Var(j) in `t`, decrementing free vars above j
/// (single-pass β instantiation).
fn db_inst(t: &V, j: u64, s: &V) -> Option<V> {
    match t {
        V::Ctr(0, fs) if fs.len() == 1 => Some(V::Ctr(0, vec![db_inst(&fs[0], j + 1, s)?])),
        V::Ctr(1, fs) if fs.len() == 2 => Some(V::Ctr(
            1,
            vec![db_inst(&fs[0], j, s)?, db_inst(&fs[1], j, s)?],
        )),
        V::Ctr(2, fs) if fs.len() == 1 => {
            let k = nat(&fs[0])?;
            if k == j {
                db_shift(j as i64, 0, s)
            } else if k > j {
                Some(V::Ctr(2, vec![V::Nat(k - 1)]))
            } else {
                Some(t.clone())
            }
        }
        _ => None,
    }
}

fn db_nf(t: &V, fuel: &mut u32) -> Option<V> {
    if *fuel == 0 {
        return None;
    }
    *fuel -= 1;
    match t {
        V::Ctr(0, fs) if fs.len() == 1 => Some(V::Ctr(0, vec![db_nf(&fs[0], fuel)?])),
        V::Ctr(1, fs) if fs.len() == 2 => {
            let f = db_nf(&fs[0], fuel)?;
            if let V::Ctr(0, body) = &f {
                let red = db_inst(&body[0], 0, &fs[1])?;
                db_nf(&red, fuel)
            } else {
                Some(V::Ctr(1, vec![f, db_nf(&fs[1], fuel)?]))
            }
        }
        V::Ctr(2, _) => Some(t.clone()),
        _ => None,
    }
}

// Brainfuck: Ctr tags 0..5 = Right,Left,Inc,Dec,Out,In; Ctr(6,[List]) Loop.
struct BfState {
    left: Vec<u64>,
    cur: u64,
    right: Vec<u64>,
    inp: Vec<u64>,
    out: Vec<u64>,
    steps: u64,
}

fn bf_exec(prog: &[V], st: &mut BfState) -> Option<()> {
    for ins in prog {
        st.steps += 1;
        if st.steps > 200_000 {
            return None;
        }
        match ins {
            V::Ctr(0, _) => {
                st.left.push(st.cur);
                st.cur = st.right.pop().unwrap_or(0);
            }
            V::Ctr(1, _) => {
                st.right.push(st.cur);
                st.cur = st.left.pop().unwrap_or(0);
            }
            V::Ctr(2, _) => st.cur += 1,
            V::Ctr(3, _) => st.cur = st.cur.saturating_sub(1),
            V::Ctr(4, _) => st.out.push(st.cur),
            V::Ctr(5, _) => {
                st.cur = if st.inp.is_empty() { 0 } else { st.inp.remove(0) };
            }
            V::Ctr(6, body) => {
                let body = list1(&body[0])?;
                while st.cur != 0 {
                    bf_exec(&body, st)?;
                    st.steps += 1;
                    if st.steps > 200_000 {
                        return None;
                    }
                }
            }
            _ => return None,
        }
    }
    Some(())
}

// GN numbers: collapsed L/B tree — V::Node internal, V::Int leaves.
fn gn_neg(v: &V) -> Option<V> {
    match v {
        V::Int(n) => Some(V::Int(-n)),
        V::Node(a, b) => Some(V::Node(Box::new(gn_neg(a)?), Box::new(gn_neg(b)?))),
        _ => None,
    }
}

/// Multiply a GN(m) value by its own root w_m (w_m² = w_{m-1}, w_0 = −1).
fn gn_mulw(v: &V) -> Option<V> {
    match v {
        V::Int(n) => Some(V::Int(-n)),
        V::Node(a, b) => Some(V::Node(Box::new(gn_mulw(b)?), Box::new((**a).clone()))),
        _ => None,
    }
}

fn gn_add(a: &V, b: &V) -> Option<V> {
    match (a, b) {
        (V::Int(x), V::Int(y)) => Some(V::Int(x.checked_add(*y)?)),
        (V::Node(a1, a2), V::Node(b1, b2)) => Some(V::Node(
            Box::new(gn_add(a1, b1)?),
            Box::new(gn_add(a2, b2)?),
        )),
        _ => None,
    }
}

fn gn_dft_ord(t: &V, natural_out: bool) -> Option<V> {
    let v = gn_dft(t)?;
    if !natural_out {
        return Some(v);
    }
    // gn_dft returns bitrev-ordered storage; undo for natural order.
    let mut d = 0u32;
    let mut cur = &v;
    while let V::Node(a, _) = cur {
        d += 1;
        cur = a;
    }
    let k = (d + 1) / 2;
    fn flat(t: &V, depth: u32, out: &mut Vec<V>) {
        if depth == 0 {
            out.push(t.clone());
        } else if let V::Node(a, b) = t {
            flat(a, depth - 1, out);
            flat(b, depth - 1, out);
        }
    }
    let mut xs = Vec::new();
    flat(&v, k, &mut xs);
    let n = xs.len();
    let mut nat = xs.clone();
    for (i, x) in xs.into_iter().enumerate() {
        let mut j = 0usize;
        for b in 0..k {
            if i >> b & 1 == 1 {
                j |= 1 << (k - 1 - b);
            }
        }
        nat[j] = x;
    }
    let _ = n;
    fn rebuild2(xs: &[V]) -> V {
        if xs.len() == 1 {
            xs[0].clone()
        } else {
            let (a, b) = xs.split_at(xs.len() / 2);
            V::Node(Box::new(rebuild2(a)), Box::new(rebuild2(b)))
        }
    }
    Some(rebuild2(&nat))
}

fn gn_dft(t: &V) -> Option<V> {
    // k from the leftmost depth d of the collapsed tree: d = 2k − 1.
    let mut d = 0u32;
    let mut cur = t;
    while let V::Node(a, _) = cur {
        d += 1;
        cur = a;
    }
    if !matches!(cur, V::Int(_)) || d == 0 || d % 2 == 0 {
        return None;
    }
    let k = (d + 1) / 2;
    let n = 1usize << k;
    fn flatten(t: &V, depth: u32, out: &mut Vec<V>) -> Option<()> {
        if depth == 0 {
            out.push(t.clone());
            return Some(());
        }
        match t {
            V::Node(a, b) => {
                flatten(a, depth - 1, out)?;
                flatten(b, depth - 1, out)
            }
            _ => None,
        }
    }
    let mut elems = Vec::new();
    flatten(t, k, &mut elems)?;
    if elems.len() != n {
        return None;
    }
    // Undo bit-reversal storage order.
    let mut coef = elems.clone();
    for (i, v) in elems.into_iter().enumerate() {
        let mut j = 0usize;
        for b in 0..k {
            if i >> b & 1 == 1 {
                j |= 1 << (k - 1 - b);
            }
        }
        coef[j] = v;
    }
    // O(N²) DFT: X[m] = Σ_j w^(m·j) · c_j, w = one gn_mulw application.
    let mut out = Vec::new();
    for m in 0..n {
        let mut acc: Option<V> = None;
        for (j, c) in coef.iter().enumerate() {
            let mut v = c.clone();
            for _ in 0..(m * j) % n {
                v = gn_mulw(&v)?;
            }
            acc = Some(match acc {
                None => v,
                Some(a) => gn_add(&a, &v)?,
            });
        }
        out.push(acc?);
    }
    // The output storage tree is in bit-reversal order, like the input.
    let mut out_br = out.clone();
    for (i, v) in out.into_iter().enumerate() {
        let mut j = 0usize;
        for b in 0..k {
            if i >> b & 1 == 1 {
                j |= 1 << (k - 1 - b);
            }
        }
        out_br[j] = v;
    }
    let out = out_br;
    fn rebuild(xs: &[V]) -> V {
        if xs.len() == 1 {
            xs[0].clone()
        } else {
            let (a, b) = xs.split_at(xs.len() / 2);
            V::Node(Box::new(rebuild(a)), Box::new(rebuild(b)))
        }
    }
    Some(rebuild(&out))
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
                AdtLen => Some(V::Nat(adt_len(&eval(&args[0], env)?)?)),
                AdtIdx => match eval(&args[0], env)? {
                    V::Ctr(tag, _) => Some(V::Nat(tag as u64)),
                    _ => None,
                },
                AdtChi => match eval(&args[0], env)? {
                    V::Ctr(_, fields) => Some(V::List(fields)),
                    _ => None,
                },
                AdtRev => adt_rev(&eval(&args[0], env)?),
                AdtEq => {
                    let a = eval(&args[0], env)?;
                    let b = eval(&args[1], env)?;
                    if !matches!(a, V::Ctr(_, _)) {
                        return None;
                    }
                    Some(V::Bool(a == b))
                }
                AdtSer => {
                    let desc = desc_shape(&eval(&args[0], env)?)?;
                    let v = eval(&args[1], env)?;
                    let mut bits = Vec::new();
                    adt_ser(&desc, &v, &mut bits)?;
                    Some(V::List(bits))
                }
                AdtDes => {
                    let desc = desc_shape(&eval(&args[0], env)?)?;
                    let bits = list1(&eval(&args[1], env)?)?;
                    let mut pos = 0usize;
                    let v = adt_des(&desc, &bits, &mut pos)?;
                    if pos != bits.len() {
                        return None;
                    }
                    Some(v)
                }
                AdtMrg => {
                    let f = eval(&args[0], env)?;
                    adt_mrg(&f, &eval(&args[1], env)?, &eval(&args[2], env)?)
                }
                Fst => tup(&eval(&args[0], env)?)?.first().cloned(),
                Snd => tup(&eval(&args[0], env)?)?.get(1).cloned(),
                Range0 => {
                    let n = nat(&eval(&args[0], env)?)?;
                    if n as usize > LIST_CAP {
                        return None;
                    }
                    Some(V::List((0..n).map(V::Nat).collect()))
                }
                SumL => {
                    let xs = nat_list(&eval(&args[0], env)?)?;
                    let mut s = 0u64;
                    for x in xs {
                        s = s.checked_add(x)?;
                    }
                    Some(V::Nat(s))
                }
                MinL => {
                    let xs = nat_list(&eval(&args[0], env)?)?;
                    xs.into_iter().min().map(V::Nat)
                }
                Perms => {
                    let xs = list1(&eval(&args[0], env)?)?;
                    Some(V::List(perms_of(&xs)?))
                }
                MapB => {
                    let xs = list1(&eval(&args[0], env)?)?;
                    let E::Lam1(body) = &args[1] else { return None };
                    let mut env2 = env.to_vec();
                    env2.push(V::Nat(0));
                    let mut out = Vec::new();
                    for x in xs {
                        *env2.last_mut().unwrap() = x;
                        out.push(eval(body, &env2)?);
                    }
                    Some(V::List(out))
                }
                CycleCost => {
                    let m = nat_grid(&eval(&args[0], env)?)?;
                    let p = nat_list(&eval(&args[1], env)?)?;
                    Some(V::Nat(cycle_cost(&m, &p)?))
                }
                SatCnf => {
                    let cs: Option<Vec<Vec<i64>>> = list1(&eval(&args[0], env)?)?
                        .iter()
                        .map(|c| {
                            list1(c)?
                                .iter()
                                .map(|l| match l {
                                    V::Int(n) => Some(*n),
                                    _ => None,
                                })
                                .collect()
                        })
                        .collect();
                    Some(V::Bool(sat_cnf(&cs?)?))
                }
                LineRas => {
                    let p0 = eval(&args[0], env)?;
                    let p1 = eval(&args[1], env)?;
                    let get = |v: &V| -> Option<(u64, u64)> {
                        let xs = tup(v)?;
                        Some((nat(xs.first()?)?, nat(xs.get(1)?)?))
                    };
                    Some(V::List(line_raster(get(&p0)?, get(&p1)?)?))
                }
                GridBfs => Some(V::Nat(grid_bfs(&bool_grid(&eval(&args[0], env)?)?)?)),
                MstW => {
                    let n = nat(&eval(&args[0], env)?)?;
                    let es: Option<Vec<(u64, u64, u64)>> = list1(&eval(&args[1], env)?)?
                        .iter()
                        .map(|e| {
                            let xs = tup(e)?;
                            if xs.len() != 3 {
                                return None;
                            }
                            Some((nat(&xs[0])?, nat(&xs[1])?, nat(&xs[2])?))
                        })
                        .collect();
                    Some(V::Nat(mst_w(n, &es?)?))
                }
                Hull => {
                    let pts: Option<Vec<(i64, i64)>> = list1(&eval(&args[0], env)?)?
                        .iter()
                        .map(|p| {
                            let xs = tup(p)?;
                            if xs.len() != 2 {
                                return None;
                            }
                            Some((nat(&xs[0])? as i64, nat(&xs[1])? as i64))
                        })
                        .collect();
                    Some(V::List(
                        hull(&pts?)
                            .into_iter()
                            .map(|(x, y)| V::Tup(vec![V::Nat(x as u64), V::Nat(y as u64)]))
                            .collect(),
                    ))
                }
                Sudoku => {
                    let g = nat_grid(&eval(&args[0], env)?)?;
                    let sol = sudoku(&g)?;
                    Some(V::List(
                        sol.into_iter()
                            .map(|r| V::List(r.into_iter().map(V::Nat).collect()))
                            .collect(),
                    ))
                }
                StlcOk => {
                    let t = eval(&args[0], env)?;
                    if !matches!(t, V::Ctr(_, _)) {
                        return None;
                    }
                    Some(V::Bool(stlc_infer(&mut Vec::new(), &t).is_some()))
                }
                LamNf => {
                    let t = eval(&args[0], env)?;
                    if !matches!(t, V::Ctr(_, _)) {
                        return None;
                    }
                    db_nf(&t, &mut 100_000)
                }
                BfRun => {
                    let prog = list1(&eval(&args[0], env)?)?;
                    let inp = nat_list(&eval(&args[1], env)?)?;
                    let mut st = BfState {
                        left: Vec::new(),
                        cur: 0,
                        right: Vec::new(),
                        inp,
                        out: Vec::new(),
                        steps: 0,
                    };
                    bf_exec(&prog, &mut st)?;
                    Some(V::List(st.out.into_iter().map(V::Nat).collect()))
                }
                GnDft => gn_dft_ord(&eval(&args[0], env)?, false),
                GnDftN => gn_dft_ord(&eval(&args[0], env)?, true),
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
    pub budget_secs: u64,
}

impl Default for SemOptions {
    fn default() -> Self {
        SemOptions {
            max_size: 10,
            max_body_size: 6,
            max_entries: 200_000,
            budget_secs: 90,
        }
    }
}

/// Heavy algorithmic primitives are excluded from lambda bodies: they are
/// top-level shaped, and evaluating them per body candidate x probe grinds
/// the pre-pass.
fn body_op(op: Op) -> bool {
    !matches!(
        op,
        Op::SatCnf
            | Op::LineRas
            | Op::GridBfs
            | Op::MstW
            | Op::Hull
            | Op::Sudoku
            | Op::StlcOk
            | Op::LamNf
            | Op::BfRun
            | Op::GnDft
            | Op::Perms
    )
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
            V::Tup(xs) => xs.iter().any(|x| contains_kind(x, f)),
            _ => false,
        }
    }
    let all_vals = || inputs.iter().flatten().chain(outputs.iter());
    let has_nat = all_vals().any(|v| matches!(v, V::Nat(_)));
    let has_list = all_vals().any(|v| matches!(v, V::List(_)));
    let has_atom =
        all_vals().any(|v| contains_kind(v, &|x| matches!(x, V::Atom(_) | V::App(_, _))));
    let has_tree = all_vals().any(|v| contains_kind(v, &|x| matches!(x, V::Node(_, _))));
    let has_ctr = all_vals().any(|v| contains_kind(v, &|x| matches!(x, V::Ctr(_, _))));
    let has_tup = all_vals().any(|v| contains_kind(v, &|x| matches!(x, V::Tup(_))));
    let has_int = all_vals().any(|v| contains_kind(v, &|x| matches!(x, V::Int(_))));
    let has_bool_grid = all_vals().any(|v| bool_grid(v).is_some());
    let has_nat_grid = all_vals().any(|v| nat_grid(v).map_or(false, |g| g.len() > 1));
    if let Ok(probe_op) = std::env::var("SUP_PROBE") {
        let op = Op::all()
            .iter()
            .copied()
            .find(|o| format!("{o:?}") == probe_op);
        if let Some(op) = op {
            let (va, _) = op.sig();
            let cand = E::Prim(op, (0..va as u32).map(E::Var).collect());
            for (j, inp) in inputs.iter().enumerate() {
                eprintln!(
                    "    {probe_op} probe test{j}: got {:?} want {:?}",
                    eval(&cand, inp),
                    outputs.get(j)
                );
            }
        }
    }
    let noops = std::env::var("SUP_NOOPS").unwrap_or_default();
    let ops: Vec<Op> = Op::all()
        .iter()
        .copied()
        .filter(|op| !noops.split(',').any(|n| !n.is_empty() && n == format!("{op:?}")))
        .filter(|op| {
            use Op::*;
            match op {
                TFlat | TBfs | TMirror | TBuild | TMergeAp | TIdxAp | TScanAp | TBitRev => {
                    has_tree
                }
                AdtLen | AdtIdx | AdtChi | AdtRev | AdtEq | AdtSer | AdtDes | AdtMrg => has_ctr,
                MapAp | ZipAp | FoldrAp => has_atom && (has_list || has_tree),
                Head | Last | Nth | Rev | RotL | RotR | Len | AppendL | SortB => {
                    has_list || has_tree
                }
                Range1 | Count => has_nat,
                If | Eq | Not => true,
                Fst | Snd => has_tup,
                LineRas | Hull => has_tup,
                MstW => has_tup && has_nat,
                SatCnf => has_int && has_list && !has_tree,
                GnDft | GnDftN => has_int && has_tree,
                GridBfs => has_bool_grid,
                Sudoku => has_nat_grid,
                StlcOk | LamNf | BfRun => has_ctr,
                Range0 | SumL | MinL | Perms | MapB | CycleCost => has_list && has_nat,
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
        // List-valued probes: lambda params ranging over permutations or
        // sublists (MapB bodies). Rotations/reversals of small ranges keep
        // order-sensitive bodies (CycleCost) distinguishable.
        if std::env::var("SUP_NOPROBE").is_ok() {
            continue;
        }
        for v in inp {
            if let V::Nat(n) = v {
                if (2..=7).contains(n) {
                    let base: Vec<V> = (0..*n).map(V::Nat).collect();
                    let mut rot = base.clone();
                    rot.rotate_left(1);
                    let mut rev = base.clone();
                    rev.reverse();
                    let mut swapped = base.clone();
                    swapped.swap(0, 1);
                    probes.push((j, V::List(base)));
                    probes.push((j, V::List(rot)));
                    probes.push((j, V::List(rev)));
                    probes.push((j, V::List(swapped)));
                }
            }
            if let V::List(xs) = v {
                probes.push((j, v.clone()));
                let mut rev = xs.clone();
                rev.reverse();
                probes.push((j, V::List(rev)));
            }
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
    let budget = std::env::var("SUP_BUDGET")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(opts.budget_secs);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(budget);

    for s in 1..=opts.max_body_size {
        if std::time::Instant::now() > deadline {
            break;
        }
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
                if la > 0 || !body_op(*op) {
                    continue; // no nested lambdas / heavy primitives in bodies
                }
                if std::time::Instant::now() > deadline {
                    break;
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
        if std::time::Instant::now() > deadline {
            break;
        }
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
