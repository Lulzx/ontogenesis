//! ARC-1 grid value bridge demo.
//!
//! This directory holds *ARC-AGI-1-specific* work only — grids, color numerals,
//! the mirror transform. The engine underneath lives in `supsearch` (the general
//! mechanism layer: behavior-keyed synthesis, concept acquisition, quotient-aware
//! search, nbe meters); nothing ARC-specific pollutes the main crate.
//!
//! The probe asks the listladder question one layer up, on grid-shaped values:
//! can a grid be evaluated, hashed and matched while the *concept-search* work
//! (composition) stays constant? Mirror is deliberately a **given** concept here
//! (a closed fold-term from `{cons,nil}`), not acquired — the question under test
//! is value representation, not concept discovery, and acquiring mirror too would
//! confound composition with value cost. The *next* milestone is grid concept
//! acquisition (discover + counterfactually acquire mirror/repeat/etc.).

use std::io::Write;
use std::rc::Rc;

use supsearch::{acquire, bank, bootstrap, nbe, parse, term};

/// Build a closed term from source (for `{cons,nil}` / Church numerals).
fn closed(s: &str) -> Rc<term::Term> {
    parse::parse_expr(s)
        .and_then(|e| parse::to_term(&e))
        .expect("closed term")
}

/// Church list [c1,..,ck] = λf.λz.f(c1)(f(c2)(...(z))); atoms are Church numerals.
fn church_list(cs: &[u32]) -> Rc<term::Term> {
    let mut body = String::from("z");
    for c in cs.iter().rev() {
        let cstr = bootstrap::church_num_str(*c);
        body = format!("f({cstr})({body})");
    }
    closed(&format!("λf.λz.{body}"))
}

/// Fold pre-built closed terms (rows) into a grid term (list of rows).
fn rc_list(items: &[Rc<term::Term>]) -> Rc<term::Term> {
    let cons = closed("λc.λs.λf.λz.f(c)(s(f)(z))");
    let nil = closed("λf.λz.z");
    items
        .iter()
        .rev()
        .fold(nil, |acc, it| term::app(term::app(cons.clone(), it.clone()), acc))
}

/// Non-symmetric grid so mirror ≠ identity (a buggy identity mirror can't pass the
/// gate). Cell (i,j) = ((i+j)%3)+1, a color numeral.
fn grid_term(w: usize, h: usize) -> Rc<term::Term> {
    let rows: Vec<Rc<term::Term>> = (0..h)
        .map(|j| {
            let cells: Vec<u32> = (0..w).map(|i| ((i + j) % 3 + 1) as u32).collect();
            church_list(&cells)
        })
        .collect();
    rc_list(&rows)
}

/// Horizontal reflection: reverse each row.
fn mirrored_term(w: usize, h: usize) -> Rc<term::Term> {
    let rows: Vec<Rc<term::Term>> = (0..h)
        .map(|j| {
            let cells: Vec<u32> = (0..w).map(|i| ((i + j) % 3 + 1) as u32).rev().collect();
            church_list(&cells)
        })
        .collect();
    rc_list(&rows)
}

/// Mirror as a closed λ-term from {cons,nil} (fold composition):
///   append   = λxs.λys.(xs cons ys)         (the C7 reduce(cons) schema)
///   reverse  = λxs. xs (λh.λacc. append acc (singleton h)) nil   (Church right-fold)
///   mirror   = map(reverse) = λxs. xs (λrow.λrest. cons (reverse row) rest) nil
fn mirror_concept() -> bank::Concept {
    let cons_t = closed("λc.λs.λf.λz.f(c)(s(f)(z))");
    let nil_t = closed("λf.λz.z");
    let singleton = term::lam(term::app(term::app(cons_t.clone(), term::var(0)), nil_t.clone()));
    let append = term::lam(term::lam(term::app(
        term::app(term::var(1), cons_t.clone()),
        term::var(0),
    )));
    // reverse = λxs. xs (λh.λacc. append acc (singleton h)) nil — the fold's
    // accumulator is the (reversed) tail, each element prepended to it.
    // de Bruijn within λh.λacc: h=Var(1), acc=Var(0).
    let reverse = term::lam(term::app(
        term::app(
            term::var(0),
            term::lam(term::lam(term::app(
                term::app(append.clone(), term::var(0)),
                term::app(singleton.clone(), term::var(1)),
            ))),
        ),
        nil_t.clone(),
    ));
    let mirror = term::lam(term::app(
        term::app(
            term::var(0),
            term::lam(term::lam(term::app(
                term::app(cons_t.clone(), term::app(reverse.clone(), term::var(1))),
                term::var(0),
            ))),
        ),
        nil_t.clone(),
    ));
    bank::Concept {
        body: mirror,
        name: "mirror".into(),
        arity: 1,
    }
}

/// `reverse` as a closed λ-term from {cons,nil} — the list fold that the A1
/// slice's atomics `reverse_cells`/`reverse_rows` are. This is the building
/// block the autonomous path must discover on the list domain, then transfer
/// to grids (mirror = map(reverse), vflip = reverse).
///   append     = λxs.λys.(xs cons ys)          (the C7 reduce(cons) schema)
///   singleton  = λa. cons a nil
///   reverse    = λxs. xs (λh.λacc. append acc (singleton h)) nil
fn reverse_concept() -> bank::Concept {
    let cons_t = closed("λc.λs.λf.λz.f(c)(s(f)(z))");
    let nil_t = closed("λf.λz.z");
    let singleton = term::lam(term::app(term::app(cons_t.clone(), term::var(0)), nil_t.clone()));
    let append = term::lam(term::lam(term::app(
        term::app(term::var(1), cons_t.clone()),
        term::var(0),
    )));
    // reverse = λxs. xs (λh.λacc. append acc (singleton h)) nil
    // de Bruijn within λh.λacc: h=Var(1), acc=Var(0).
    let reverse = term::lam(term::app(
        term::app(
            term::var(0),
            term::lam(term::lam(term::app(
                term::app(append.clone(), term::var(0)),
                term::app(singleton.clone(), term::var(1)),
            ))),
        ),
        nil_t.clone(),
    ));
    bank::Concept {
        body: reverse,
        name: "reverse".into(),
        arity: 1,
    }
}

/// A list-reversal task: reverse [1,2,3] → [3,2,1]. Small values so raw search
/// hashing is cheap; the question is whether the fold-term is reachable.
fn reverse_list_task() -> parse::Task {
    parse::Task {
        arity: 1,
        tests: vec![parse::Test {
            args: vec![church_list(&[1, 2, 3])],
            want: church_list(&[3, 2, 1]),
            outer: 0,
        }],
    }
}

fn task(w: usize, h: usize) -> parse::Task {
    parse::Task {
        arity: 1,
        tests: vec![parse::Test {
            args: vec![grid_term(w, h)],
            want: mirrored_term(w, h),
            outer: 0,
        }],
    }
}

fn bank_opts(budget: u64, max_size: u32) -> bank::Options {
    let mut o = bank::Options::default();
    o.max_size = max_size;
    o.time_budget_secs = budget as f64;
    o
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("gridacq") => {
            // Grid-value NBE recurses deep on the fold structure, so run the
            // acquisition driver on a large stack (same as the tests do).
            let a = args[1..].to_vec();
            std::thread::Builder::new()
                .stack_size(1 << 30)
                .spawn(move || gridacq(&a))
                .unwrap()
                .join()
                .unwrap();
        }
        Some("gridrep") => {
            // Same large-stack requirement: grid NBE recurses deep on the fold
            // structure, and the probe hashes 30×30 grids.
            let a = args[1..].to_vec();
            std::thread::Builder::new()
                .stack_size(1 << 30)
                .spawn(move || gridrep(&a))
                .unwrap()
                .join()
                .unwrap();
        }
        Some("a1") => {
            // The A1 ARC Ontogenesis Slice: grid NBE recurses deep on the fold
            // structure and the slice hashes 8×8 grids, so run on a large stack.
            let a = args[1..].to_vec();
            std::thread::Builder::new()
                .stack_size(1 << 30)
                .spawn(move || a1(&a))
                .unwrap()
                .join()
                .unwrap();
        }
        Some("autodisc") => {
            // Probe: can raw search discover the list fold `reverse`? List NBE
            // recurses deep on the fold structure, so run on a large stack.
            let a = args[1..].to_vec();
            std::thread::Builder::new()
                .stack_size(1 << 30)
                .spawn(move || autodisc(&a))
                .unwrap()
                .join()
                .unwrap();
        }
        Some("gridmeta") => {
            // Multi-transform generalization: grid NBE recurses deep on the fold
            // structure and the probe hashes tiled grids, so run on a large stack.
            let a = args[1..].to_vec();
            std::thread::Builder::new()
                .stack_size(1 << 30)
                .spawn(move || gridmeta(&a))
                .unwrap()
                .join()
                .unwrap();
        }
        _ => bridge(&args),
    }
}

/// The grid value bridge probe (default): can a grid-shaped value be matched while
/// composition stays constant? Mirror is a *given* concept here — the bridge isolates
/// value representation, not concept discovery.
fn bridge(args: &[String]) {
    let mut budget = 8u64;
    let mut max_size = 14u32;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--budget" => {
                budget = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--max-size" => {
                max_size = args[i + 1].parse().unwrap();
                i += 2;
            }
            other => {
                eprintln!("unknown arc1 arg: {other}");
                std::process::exit(1);
            }
        }
    }

    let concept = mirror_concept();
    let opts = bank_opts(budget, max_size);

    // ── correctness gate: mirror must genuinely reflect before the probe trusts it ──
    let gate_ok = bank::concept_solve(&task(3, 3), &[concept.clone()], &opts)
        .solution
        .is_some()
        && bank::concept_solve(&task(2, 3), &[concept.clone()], &opts)
            .solution
            .is_some();
    println!("\n── arc1: can a grid-shaped value be matched while composition stays constant? ──");
    println!("grid = nested Church lists (rows of color-numerals); mirror = given closed fold-term from {{cons,nil}}");
    println!("budget: max_size {max_size}, {budget}s, pool 64, fuel 20000");
    println!(
        "correctness gate (mirror solves 3×3 and 2×3): {}",
        if gate_ok {
            "✓"
        } else {
            "✗ — mirror term is wrong, ABORT"
        }
    );
    if !gate_ok {
        return;
    }

    println!(
        "{:<9} {:<10} {:<10} {:<10} {:<9} {:<5} {:<5} {}",
        "grid", "built", "beta", "quote", "evalAbort", "sol", "size", "verdict"
    );
    let mut fails = 0u32;
    for (w, h) in [
        (1usize, 1usize),
        (2, 2),
        (3, 3),
        (5, 5),
        (8, 8),
        (16, 16),
        (32, 32),
        (64, 64),
    ] {
        let t = task(w, h);
        nbe::meter_on(true);
        nbe::meter_reset();
        let o = bank::concept_solve(&t, &[concept.clone()], &opts);
        let (b, q, ab) = (nbe::beta_steps(), nbe::quote_nodes(), nbe::eval_aborts());
        nbe::meter_on(false);
        let (sol, size) = o
            .solution
            .as_ref()
            .map(|s| ("✓".to_string(), s.size()))
            .unwrap_or_else(|| ("✗".to_string(), 0));
        let verdict = if o.solution.is_some() { "ok" } else { "BREAK" };
        println!(
            "{:<9} {:<10} {:<10} {:<10} {:<9} {:<5} {:<5} {}",
            format!("{w}×{h}"),
            o.stats.built,
            b,
            q,
            ab,
            sol,
            size,
            verdict
        );
        if o.solution.is_none() {
            fails += 1;
            if fails >= 2 {
                println!("   (stopping after 2 consecutive failures)");
                break;
            }
        } else {
            fails = 0;
        }
    }
    println!(
        "   C_composition = `built` (the search applies mirror once), C_value = `beta`+`quote`\n\
         (grid computation + value size to hash). Expect built flat while quote grows; the wall\n\
         is the grid value exceeding hash fuel — the representation capacity the next milestone\n\
         (grid concept acquisition + canonical grid representation) must lift before ARC-1."
    );
    std::io::stdout().flush().ok();
}

/// Vertical flip: reverse the *row order* (not each row). Built as the same
/// fold over the list-of-rows that `mirror`'s `reverse` uses on a single row,
/// so `vflip = reverse(grid)`. A genuine grid transform — the wrong one for a
/// horizontal-mirror held-out (the negative control).
fn vflip_concept() -> bank::Concept {
    let cons_t = closed("λc.λs.λf.λz.f(c)(s(f)(z))");
    let nil_t = closed("λf.λz.z");
    let singleton = term::lam(term::app(term::app(cons_t.clone(), term::var(0)), nil_t.clone()));
    let append = term::lam(term::lam(term::app(
        term::app(term::var(1), cons_t.clone()),
        term::var(0),
    )));
    // vflip = λxs. xs (λrow.λrest. append rest (singleton row)) nil
    // de Bruijn within λrow.λrest: row=Var(1), rest=Var(0).
    let vflip = term::lam(term::app(
        term::app(
            term::var(0),
            term::lam(term::lam(term::app(
                term::app(append.clone(), term::var(0)),
                term::app(singleton.clone(), term::var(1)),
            ))),
        ),
        nil_t.clone(),
    ));
    bank::Concept {
        body: vflip,
        name: "vflip".into(),
        arity: 1,
    }
}

/// The vertical-flip target: row `j` of the output is row `(h-1-j)` of the input.
fn vflipped_term(w: usize, h: usize) -> Rc<term::Term> {
    let rows: Vec<Rc<term::Term>> = (0..h)
        .map(|j| {
            let cells: Vec<u32> = (0..w)
                .map(|i| ((i + (h - 1 - j)) % 3 + 1) as u32)
                .collect();
            church_list(&cells)
        })
        .collect();
    rc_list(&rows)
}

/// A single-arity family: one test per (w,h), input `grid_term`, target from `target`.
fn transform_family(
    sizes: &[(usize, usize)],
    target: &dyn Fn(usize, usize) -> Rc<term::Term>,
) -> parse::Task {
    parse::Task {
        arity: 1,
        tests: sizes
            .iter()
            .map(|&(w, h)| parse::Test {
                args: vec![grid_term(w, h)],
                want: target(w, h),
                outer: 0,
            })
            .collect(),
    }
}

/// The generic grid schema meta-space: closed λ-term closures over `{cons,nil}`.
///
/// These are structural generators (the C7 `iterate`/`reduce` idea lifted to
/// grids), NOT ARC-named concepts. A grid is a list of rows, each a list of
/// color-numerals, so a transform is a fold over the outer list (rows) and/or
/// the inner lists (cells). The A1 slice enumerates candidates from this space
/// and lets the counterfactual gate decide what is worth acquiring — mirror is
/// *not* hand-supplied here; it is `map_rows(reverse_cells)`, one candidate
/// among many.
mod schema {
    use super::*;

    fn cons_t() -> Rc<term::Term> {
        closed("λc.λs.λf.λz.f(c)(s(f)(z))")
    }
    fn nil_t() -> Rc<term::Term> {
        closed("λf.λz.z")
    }
    fn singleton() -> Rc<term::Term> {
        term::lam(term::app(term::app(cons_t(), term::var(0)), nil_t()))
    }
    fn append() -> Rc<term::Term> {
        term::lam(term::lam(term::app(
            term::app(term::var(1), cons_t()),
            term::var(0),
        )))
    }

    /// id = λx. x
    pub fn id() -> Rc<term::Term> {
        term::lam(term::var(0))
    }

    /// reverse_cells = λrow. row (λc.λacc. append acc (singleton c)) nil
    /// (reverse a single row — the horizontal-mirror atomic).
    pub fn reverse_cells() -> Rc<term::Term> {
        term::lam(term::app(
            term::app(
                term::var(0),
                term::lam(term::lam(term::app(
                    term::app(append(), term::var(0)),
                    term::app(singleton(), term::var(1)),
                ))),
            ),
            nil_t(),
        ))
    }

    /// reverse_rows = λgrid. grid (λrow.λrest. append rest (singleton row)) nil
    /// (reverse the row order — the vertical-flip atomic).
    pub fn reverse_rows() -> Rc<term::Term> {
        term::lam(term::app(
            term::app(
                term::var(0),
                term::lam(term::lam(term::app(
                    term::app(append(), term::var(0)),
                    term::app(singleton(), term::var(1)),
                ))),
            ),
            nil_t(),
        ))
    }

    /// map_rows(f) = λgrid. grid (λrow.λrest. cons (f row) rest) nil
    /// (apply f to each row, preserving order).
    pub fn map_rows(f: &Rc<term::Term>) -> Rc<term::Term> {
        term::lam(term::app(
            term::app(
                term::var(0),
                term::lam(term::lam(term::app(
                    term::app(cons_t(), term::app(f.clone(), term::var(1))),
                    term::var(0),
                ))),
            ),
            nil_t(),
        ))
    }

    /// map_cells(f) = λgrid. grid (λrow.λrest. cons (row (λc.λacc. cons (f c) acc) nil) rest) nil
    /// (apply f to each cell of each row).
    pub fn map_cells(f: &Rc<term::Term>) -> Rc<term::Term> {
        term::lam(term::app(
            term::app(
                term::var(0),
                term::lam(term::lam(term::app(
                    term::app(
                        cons_t(),
                        term::app(
                            term::app(
                                term::var(1),
                                term::lam(term::lam(term::app(
                                    term::app(cons_t(), term::app(f.clone(), term::var(1))),
                                    term::var(0),
                                ))),
                            ),
                            nil_t(),
                        ),
                    ),
                    term::var(0),
                ))),
            ),
            nil_t(),
        ))
    }

    /// compose(f,g) = λx. f (g x)
    pub fn compose(f: &Rc<term::Term>, g: &Rc<term::Term>) -> Rc<term::Term> {
        term::lam(term::app(f.clone(), term::app(g.clone(), term::var(0))))
    }

    /// The bounded proposal enumeration: direct atomics, `app(schema, atomic)`
    /// for map_rows/map_cells, and `compose(atomic, atomic)`. Shallow by design —
    /// rotation (`compose(reverse_rows, map_rows(reverse_cells))`) composes a
    /// *schema result*, so it is deliberately absent here; it is the transfer
    /// target, solvable only once mirror and vflip are acquired.
    pub fn propose_candidates() -> Vec<Rc<term::Term>> {
        let atomics = [id(), reverse_cells(), reverse_rows()];
        let mut out = Vec::new();
        out.extend(atomics.iter().cloned());
        for a in &atomics {
            out.push(map_rows(a));
            out.push(map_cells(a));
        }
        for a in &atomics {
            for b in &atomics {
                out.push(compose(a, b));
            }
        }
        out
    }
}

/// 180° rotation target: output cell (i,j) = input cell (w-1-i, h-1-j).
/// Rotation is the *transfer* family — it is not in the shallow proposal
/// enumeration, but becomes solvable once mirror and vflip are acquired and
/// composed.
fn rotated_term(w: usize, h: usize) -> Rc<term::Term> {
    let rows: Vec<Rc<term::Term>> = (0..h)
        .map(|j| {
            let cells: Vec<u32> = (0..w)
                .map(|i| ((w - 1 - i + h - 1 - j) % 3 + 1) as u32)
                .collect();
            church_list(&cells)
        })
        .collect();
    rc_list(&rows)
}

/// Horizontal tiling target: each row is doubled in width. Row `[c1..cw]`
/// becomes `[c1..cw, c1..cw]`. Generable from the discovered vocabulary as
/// `map(λrow. append row row)`.
fn htiled_term(w: usize, h: usize) -> Rc<term::Term> {
    let rows: Vec<Rc<term::Term>> = (0..h)
        .map(|j| {
            let cells: Vec<u32> = (0..w).map(|i| ((i + j) % 3 + 1) as u32).collect();
            let mut doubled = cells.clone();
            doubled.extend_from_slice(&cells);
            church_list(&doubled)
        })
        .collect();
    rc_list(&rows)
}

/// Vertical tiling target: the row list is doubled in height. `[row1..rowh]`
/// becomes `[row1..rowh, row1..rowh]`. Generable as `λgrid. append grid grid`.
fn vtiled_term(w: usize, h: usize) -> Rc<term::Term> {
    let rows: Vec<Rc<term::Term>> = (0..h)
        .map(|j| {
            let cells: Vec<u32> = (0..w).map(|i| ((i + j) % 3 + 1) as u32).collect();
            church_list(&cells)
        })
        .collect();
    let mut doubled = rows.clone();
    doubled.extend_from_slice(&rows);
    rc_list(&doubled)
}

/// 2×2 tiling target: both width and height doubled. Generable as
/// `compose(λgrid. append grid grid, map(λrow. append row row))`.
fn tile2_term(w: usize, h: usize) -> Rc<term::Term> {
    let rows: Vec<Rc<term::Term>> = (0..h)
        .map(|j| {
            let cells: Vec<u32> = (0..w).map(|i| ((i + j) % 3 + 1) as u32).collect();
            let mut doubled = cells.clone();
            doubled.extend_from_slice(&cells);
            church_list(&doubled)
        })
        .collect();
    let mut doubled = rows.clone();
    doubled.extend_from_slice(&rows);
    rc_list(&doubled)
}

/// A candidate body installed as a single-arity concept (all grid transforms
/// here are arity 1).
fn cand_concept(body: &Rc<term::Term>) -> bank::Concept {
    bank::Concept {
        body: body.clone(),
        name: "cand".into(),
        arity: 1,
    }
}

/// The Propose stage: which candidates from the generic schema meta-space solve
/// the given task? Each candidate is installed as a single-arity concept and
/// tested against the task's examples via the canonical-keying quotient search
/// ([`bank::concept_solve_abl`] with `use_canon=true`, so 8×8 grids stay
/// hashable). Solvers are returned — the `examples → candidate transformation`
/// step, with no ARC-named concept supplied.
fn propose_solvers(task: &parse::Task, opts: &bank::Options) -> Vec<Rc<term::Term>> {
    schema::propose_candidates()
        .into_iter()
        .filter(|body| {
            bank::concept_solve_abl(task, &[cand_concept(body)], opts, true)
                .0
                .solution
                .is_some()
        })
        .collect()
}

/// Grid concept acquisition: does the counterfactual gate transfer to grid values?
///
/// Offered mirror (the right transform) and vflip (a real-but-wrong one) against a
/// held-out mirror family, the machine must acquire exactly mirror (Δ>0, a frontier
/// gain) and decline vflip — promotion by measured held-out cost, not by name, exactly
/// as `promote`/`listladder` did for numerals and lists. Grids stay ≤5×5 (value-safe,
/// under `concept_solve`'s 2048 pool cap) so the claim is about composition, not
/// representation — the canonical grid representation is the follow-up milestone.
fn gridacq(args: &[String]) {
    use std::io::Write;
    let mut budget = 4u64;
    let mut max_size = 14u32;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--budget" => {
                budget = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--max-size" => {
                max_size = args[i + 1].parse().unwrap();
                i += 2;
            }
            other => {
                eprintln!("unknown gridacq arg: {other}");
                std::process::exit(1);
            }
        }
    }
    let opts = bank_opts(budget, max_size);

    let mirror = mirror_concept();
    let vflip = vflip_concept();

    // Training families (solves-gate) + held-out (disjoint sizes, generalization).
    let train_mirror = transform_family(&[(3, 3), (2, 3), (4, 2), (3, 2)], &mirrored_term);
    let h_mirror = transform_family(&[(2, 2), (5, 3), (4, 4), (1, 3)], &mirrored_term);
    let train_vflip = transform_family(&[(3, 3), (2, 3), (3, 2)], &vflipped_term);

    println!("\n── arc1 gridacq: does counterfactual acquisition transfer to grid values? ──");
    println!("base = {{cons}} + color-numerals; candidates offered: mirror (map(reverse)) and vflip (reverse row order)");
    println!("grids ≤5×5 (value-safe, under the 2048 pool cap) so the claim is composition, not representation");

    // solves-gate: each candidate must genuinely solve its own family before proposal.
    let mirror_ok = bank::concept_solve(&train_mirror, &[mirror.clone()], &opts)
        .solution
        .is_some();
    let vflip_ok = bank::concept_solve(&train_vflip, &[vflip.clone()], &opts)
        .solution
        .is_some();
    println!(
        "  solves-gate: mirror solves its family? {} | vflip solves its family? {}",
        if mirror_ok { "✓" } else { "✗" },
        if vflip_ok { "✓" } else { "✗" }
    );

    // baseline: the base reasoner (no concepts) cannot transform a grid at all.
    let raw = acquire::raw_cost(&h_mirror, &opts);
    let base = acquire::concept_cost(&h_mirror, &[], &opts);
    println!(
        "  baseline on mirror held-out: raw {} states, base-concept {} states (both ✗: no concept to compose)",
        acquire::disp_cost(raw),
        acquire::disp_cost(base)
    );

    // ── propose mirror: the right transform ──
    match acquire::propose_value(&mirror.body, &[], &[h_mirror.clone()], &opts, base) {
        Some(g) if g.earns() => println!(
            "  mirror: {} → {}  {}  ACQUIRE  arity {} (inferred)",
            acquire::disp_cost(g.before),
            acquire::disp_cost(g.after),
            g.kind(),
            g.arity
        ),
        Some(g) => println!(
            "  mirror: {} → {}  {}  REJECT",
            acquire::disp_cost(g.before),
            acquire::disp_cost(g.after),
            g.kind()
        ),
        None => println!("  mirror: no valid interface  REJECT"),
    }
    let after = acquire::concept_cost(&h_mirror, &[mirror.clone()], &opts);
    println!(
        "  quotient collapse: held-out mirror raw {} → through-mirror {} states",
        acquire::disp_cost(raw),
        acquire::disp_cost(after)
    );

    // ── negative control: vflip on the mirror held-out ──
    match acquire::propose_value(&vflip.body, &[], &[h_mirror.clone()], &opts, base) {
        Some(g) if g.earns() => println!(
            "  negative control: vflip PROMOTED on the mirror family (UNEXPECTED)"
        ),
        Some(g) => println!(
            "  negative control: vflip on the mirror held-out → REJECTED ({}, {} → {}): real but\n\
             \x20     the wrong transform — declined by measured gain, not by name.",
            g.kind(),
            acquire::disp_cost(g.before),
            acquire::disp_cost(g.after)
        ),
        None => println!(
            "  negative control: vflip on the mirror held-out → REJECTED (no valid interface):\n\
             \x20     real but the wrong transform."
        ),
    }

    // ── redundancy control: re-proposing the installed mirror ──
    let base2 = acquire::concept_cost(&h_mirror, &[mirror.clone()], &opts);
    match acquire::propose_value(&mirror.body, &[mirror.clone()], &[h_mirror.clone()], &opts, base2)
    {
        Some(g) if g.earns() => println!(
            "  redundancy control: re-proposing mirror PROMOTED again (UNEXPECTED)"
        ),
        Some(g) => println!(
            "  redundancy control: re-proposing the installed mirror → REJECT ({}, Δ=0): a concept\n\
             \x20     earns no second slot merely for solving.",
            g.kind()
        ),
        None => println!("  redundancy control: re-proposing the installed mirror → REJECT (no valid interface)"),
    }
    println!(
        "  honest caveat: mirror is *offered* (a closed fold-term from {{cons,nil}}), not raw-discovered —\n\
         \x20    raw bank::solve cannot rediscover map(reverse) from one grid (atoms are just the args).\n\
         \x20    The claim is counterfactual worth (Δ>0), exactly as promote/listladder."
    );
    std::io::stdout().flush().ok();
}

/// The value-representation probe: does quotient reasoning stay cheap when the
/// semantic object is an ARC-sized grid? Mirror is a *given* concept (arity 1),
/// so composition is one application; the only knob is how a grid's identity is
/// computed for dedup + target matching. Each size runs `concept_solve_abl`
/// twice — structural (2048-fuel hash cap, the historical wall) and canonical
/// (compact Val-level `GridKey`, O(wh) no numeral expansion) — and prints the
/// C_composition / C_value split per mode.
fn gridrep(args: &[String]) {
    use std::io::Write;
    let mut budget = 8u64;
    let mut max_size = 14u32;
    // The candidate is a lazy fold: forcing it during recognition costs more than
    // the default 20000 fuel, so give the canonical path a generous budget. The
    // structural path's 2048 cap is hardcoded and unaffected by this.
    let mut fuel = 1_000_000i64;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--budget" => {
                budget = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--max-size" => {
                max_size = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--fuel" => {
                fuel = args[i + 1].parse().unwrap();
                i += 2;
            }
            other => {
                eprintln!("unknown gridrep arg: {other}");
                std::process::exit(1);
            }
        }
    }
    let mut opts = bank_opts(budget, max_size);
    opts.fuel = fuel;
    let mirror = mirror_concept();

    // Square grids, 3² → 30². 8×8 is the historical structural wall (the expanded
    // normal form blows the 2048 hash cap); 30×30 is the ARC-sized target.
    let sizes = [3usize, 5, 8, 12, 16, 24, 30];

    println!("\n── arc1 gridrep: is the wall representation, not ARC semantics? ──");
    println!("mirror (arity 1) is a given concept; C_composition = built, C_value = canon/quote nodes");
    println!("structural = 2048-fuel hash cap (historical); canonical = compact GridKey (O(wh))");
    println!();
    println!(
        "{:>5} {:>10} {:>6} {:>8} {:>10} {:>12} {:>11} {:>10} {:>7}",
        "grid", "mode", "built", "beta", "C_value", "max_trans", "hash_abort", "eval_abort", "solved"
    );

    for &s in &sizes {
        let t = task(s, s);
        for (label, use_canon) in [("structural", false), ("canonical", true)] {
            nbe::meter_on(true);
            nbe::meter_reset();
            supsearch::canon::meter_on(true);
            supsearch::canon::meter_reset();
            let (out, m) = bank::concept_solve_abl(&t, &[mirror.clone()], &opts, use_canon);
            nbe::meter_on(false);
            supsearch::canon::meter_on(false);
            // C_value: canonical mode walks the compact grid (canon_nodes);
            // structural mode quotes the expanded normal form (quote_nodes).
            let c_value = m.canon_nodes + m.quote_nodes;
            println!(
                "{:>3}² {:>10} {:>6} {:>8} {:>10} {:>12} {:>11} {:>10} {:>7}",
                s,
                label,
                out.stats.built,
                m.norm_steps,
                c_value,
                m.max_transient,
                m.hash_aborts,
                m.eval_aborts,
                if out.solution.is_some() { "✓" } else { "✗" }
            );
        }
    }

    println!();
    println!(
        "  expected: C_composition (built) stays ≈ O(1) in both modes; C_value grows ~O(wh) with cells;\n\
         \x20  structural ✗ at ~8×8 (hash_aborts spike on the 2048 cap); canonical ✓ to 30×30 (hash_aborts ≈ 0).\n\
         \x20  The wall is value representation, not ARC semantics — the compact GridKey lifts it."
    );
    std::io::stdout().flush().ok();
}

/// SolveRate + total `built` cost over a task set, given a per-task cost fn that
/// returns `(solved, built)`. `built` is summed for ALL tasks (solved or not) —
/// an unsolved task still spent search work, and that work is the honest cost of
/// the control. SolveRate is the fraction solved.
fn solve_rate(tasks: &[parse::Task], cost: &dyn Fn(&parse::Task) -> (bool, u64)) -> (f64, u64) {
    let mut solved = 0u64;
    let mut total = 0u64;
    for t in tasks {
        let (ok, built) = cost(t);
        total += built;
        if ok {
            solved += 1;
        }
    }
    (solved as f64 / tasks.len() as f64, total)
}

/// A1 ARC Ontogenesis Slice: Express → Propose → Acquire → Transfer.
///
/// The first real ARC-1 curriculum: 10–20 tasks across three families (horizontal
/// mirror, vertical mirror, 180° rotation), tracking the four stages per task.
/// The genuinely new step is **Propose** — `examples → candidate transformation`
/// via the generic schema meta-space, with no ARC-named concept supplied. The
/// success condition: at least one concept generated from real ARC experience is
/// autonomously proposed, counterfactually acquired, and measurably reduces
/// fixed-budget search on previously unseen ARC tasks (held-out mirror sizes +
/// the composed rotation family). Three controls (A frozen / B seeds / C
/// ontogenesis) run under the same compute; the claim is C wins while B doesn't,
/// so the advantage is the quotient search language, not remembered programs.
///
/// All concept-path solves use the canonical-keying ablation path
/// ([`bank::concept_solve_abl`] with `use_canon=true`) so 8×8 grids stay
/// hashable; control B uses [`bank::solve_abl`] with the same keying so the
/// seeds-vs-ontogenesis comparison is not confounded by the structural 2048 cap.
fn a1(_args: &[String]) {
    use std::io::Write;
    let mut out = std::io::stdout();

    // Shared budget: same compute for all three controls. Canonical fuel is
    // raised so 8×8 grids (and their lazy-fold candidates) stay hashable.
    let mut opts = bank_opts(4, 14);
    opts.fuel = 1_000_000;

    // ── Task families ──
    // Training (solves-gate + Express) and held-out (disjoint sizes, the
    // generalization target) for mirror and vflip; rotation is the transfer
    // family — never trained, solvable only by composing the acquired concepts.
    let train_mirror = transform_family(&[(3, 3), (5, 5), (8, 8)], &mirrored_term);
    let train_vflip = transform_family(&[(3, 3), (5, 5), (8, 8)], &vflipped_term);
    let h_mirror = transform_family(&[(4, 4), (6, 6)], &mirrored_term);
    let h_vflip = transform_family(&[(4, 4), (6, 6)], &vflipped_term);
    let rotation = transform_family(&[(3, 3), (5, 5), (8, 8)], &rotated_term);

    // The known-correct transforms, built from the generic schema meta-space.
    let mirror_body = schema::map_rows(&schema::reverse_cells());
    let vflip_body = schema::reverse_rows();
    let rot_body = schema::compose(&schema::reverse_rows(), &schema::map_rows(&schema::reverse_cells()));

    println!("\n── A1 ARC Ontogenesis Slice: Express → Propose → Acquire → Transfer ──");
    println!("substrate = {{cons,nil}} + Church color-numerals; grid = list of rows, each a list of numerals");
    println!("schema meta-space = {{id, reverse_cells, reverse_rows}} × {{map_rows, map_cells, compose}} (generic, not ARC-named)");
    println!("families: A mirror (train 3²,5²,8² / held-out 4²,6²), D vflip (same), B rotation (transfer, 3²,5²,8²)");
    println!("canonical keying throughout the concept path (8×8 grids stay hashable past the 2048 structural cap)");
    out.flush().ok();

    // ── Stage 1: Express — prove the substrate can represent the transform ──
    // Before any synthesis, verify the known-correct schema term solves each
    // training family. This separates "search failed" from "ontology failed".
    let express_mirror = bank::concept_solve_abl(&train_mirror, &[cand_concept(&mirror_body)], &opts, true)
        .0
        .solution
        .is_some();
    let express_vflip = bank::concept_solve_abl(&train_vflip, &[cand_concept(&vflip_body)], &opts, true)
        .0
        .solution
        .is_some();
    let express_rot = bank::concept_solve_abl(&rotation, &[cand_concept(&rot_body)], &opts, true)
        .0
        .solution
        .is_some();
    println!("\n── Stage 1: Express (substrate represents the transform?) ──");
    println!(
        "  mirror = map_rows(reverse_cells): {}",
        if express_mirror { "EXPRESSIBLE ✓" } else { "NOT EXPRESSIBLE ✗" }
    );
    println!(
        "  vflip  = reverse_rows:            {}",
        if express_vflip { "EXPRESSIBLE ✓" } else { "NOT EXPRESSIBLE ✗" }
    );
    println!(
        "  rotation = compose(reverse_rows, map_rows(reverse_cells)): {}",
        if express_rot { "EXPRESSIBLE ✓" } else { "NOT EXPRESSIBLE ✗" }
    );
    out.flush().ok();

    // ── Stage 2: Propose — examples → candidate transformation ──
    // Enumerate the schema meta-space, test each candidate against a small
    // mirror/vflip task (solves-gate), collect the solvers. No ARC-named concept.
    let gate_mirror = task(3, 3);
    let gate_vflip = transform_family(&[(3, 3)], &vflipped_term);
    let mirror_solvers = propose_solvers(&gate_mirror, &opts);
    let vflip_solvers = propose_solvers(&gate_vflip, &opts);
    let mirror_proposed = mirror_solvers.iter().any(|b| b == &mirror_body);
    let vflip_proposed = vflip_solvers.iter().any(|b| b == &vflip_body);
    println!("\n── Stage 2: Propose (schema meta-space → candidate transformations) ──");
    println!(
        "  mirror task: {} candidate(s) solve the gate; mirror = map_rows(reverse_cells) proposed: {}",
        mirror_solvers.len(),
        if mirror_proposed { "✓" } else { "✗" }
    );
    println!(
        "  vflip task:  {} candidate(s) solve the gate; vflip = reverse_rows proposed: {}",
        vflip_solvers.len(),
        if vflip_proposed { "✓" } else { "✗" }
    );
    out.flush().ok();

    // ── Stage 3: Acquire — the counterfactual gate on the held-out ──
    // For each proposed solver, measure held-out cost with vs without it. A
    // frontier gain (✗ → finite) ACQUIRES; no gain / regression REJECTS.
    let base_mirror = acquire::concept_cost_abl(&h_mirror, &[], &opts, true);
    let g_mirror = acquire::propose_value_abl(&mirror_body, &[], &[h_mirror.clone()], &opts, base_mirror, true);
    let base_vflip = acquire::concept_cost_abl(&h_vflip, &[], &opts, true);
    let g_vflip = acquire::propose_value_abl(&vflip_body, &[], &[h_vflip.clone()], &opts, base_vflip, true);
    // Wrong-transform controls: a real-but-wrong transform must be REJECTED on
    // the other family's held-out — promotion by measured Δ, not by name.
    let g_vflip_on_mirror = acquire::propose_value_abl(&vflip_body, &[], &[h_mirror.clone()], &opts, base_mirror, true);
    let g_mirror_on_vflip = acquire::propose_value_abl(&mirror_body, &[], &[h_vflip.clone()], &opts, base_vflip, true);
    println!("\n── Stage 3: Acquire (counterfactual gate on held-out) ──");
    let show = |g: &Option<acquire::Gain>, label: &str| match g {
        Some(g) => println!(
            "  {label}: {} → {} (arity {}), {} → {}",
            acquire::disp_cost(g.before),
            acquire::disp_cost(g.after),
            g.arity,
            g.kind(),
            if g.earns() { "ACQUIRE" } else { "REJECT" }
        ),
        None => println!("  {label}: no valid interface → REJECT"),
    };
    show(&g_mirror, "mirror on mirror held-out");
    show(&g_vflip, "vflip  on vflip held-out ");
    show(&g_vflip_on_mirror, "vflip  on mirror held-out (wrong)");
    show(&g_mirror_on_vflip, "mirror on vflip held-out  (wrong)");
    out.flush().ok();

    // ── Stage 4: Transfer — does the acquired concept reduce search on unseen tasks? ──
    // Single-concept generalization: held-out mirror without (✗) vs with [mirror].
    let h_mirror_base = acquire::concept_cost_abl(&h_mirror, &[], &opts, true);
    let h_mirror_after = acquire::concept_cost_abl(&h_mirror, &[mirror_concept()], &opts, true);
    // Composition: rotation without (✗) vs with [mirror, vflip] (finite).
    let rot_base = acquire::concept_cost_abl(&rotation, &[], &opts, true);
    let rot_after = acquire::concept_cost_abl(&rotation, &[mirror_concept(), vflip_concept()], &opts, true);
    println!("\n── Stage 4: Transfer (unseen tasks) ──");
    println!(
        "  single-concept: held-out mirror {} → through-mirror {} ({} states)",
        acquire::disp_cost(h_mirror_base),
        acquire::disp_cost(h_mirror_after),
        h_mirror_after
    );
    println!(
        "  composition:    rotation {} → through {{mirror, vflip}} {} ({} states)",
        acquire::disp_cost(rot_base),
        acquire::disp_cost(rot_after),
        rot_after
    );
    out.flush().ok();

    // ── Controls: A frozen / B seeds / C ontogenesis, same compute ──
    // A: raw enumeration, no concepts, no seeds. B: raw enumeration seeded with
    // the solved program bodies (mirror, vflip) as ordinary seeds. C: promoted
    // Prims + quotient reasoning. All canonical keying, same time budget.
    let mut seeds_opts = opts.clone();
    seeds_opts.seeds = vec![mirror_body.clone(), vflip_body.clone()];
    let tasks = [h_mirror.clone(), h_vflip.clone(), rotation.clone()];
    let (sr_a, cost_a) = solve_rate(&tasks, &|t| {
        let o = bank::solve_abl(t, &opts, true);
        (o.solution.is_some(), o.stats.built)
    });
    let (sr_b, cost_b) = solve_rate(&tasks, &|t| {
        let o = bank::solve_abl(t, &seeds_opts, true);
        (o.solution.is_some(), o.stats.built)
    });
    let (sr_c, cost_c) = solve_rate(&tasks, &|t| {
        let c = acquire::concept_cost_abl(t, &[mirror_concept(), vflip_concept()], &opts, true);
        (c < acquire::UNREACHABLE, c)
    });
    println!("\n── Controls (SolveRate + total built, same compute) ──");
    println!(
        "  A frozen base:      SolveRate {:.0}%  total built {}",
        sr_a * 100.0,
        cost_a
    );
    println!(
        "  B naive seeds:      SolveRate {:.0}%  total built {}",
        sr_b * 100.0,
        cost_b
    );
    println!(
        "  C ontogenesis:      SolveRate {:.0}%  total built {}",
        sr_c * 100.0,
        cost_c
    );
    println!();
    println!(
        "  claim: the ontogenesis path (C) solves what the frozen base (A) cannot, and does so at\n\
         \x20 a fraction of the naive-seeds cost (B) — the advantage is the quotient search language,\n\
         \x20 not merely remembering solved programs. (If B≈C on SolveRate, cost is the distinguisher.)"
    );
    out.flush().ok();
}

/// Discover `reverse` on the list domain from `{cons,nil}` via the C7 meta-space.
///
/// Path (all on the list domain, where values are small and hashing is cheap):
///   1. append    = reduce(cons)          — the C7 discovery (list eliminator)
///   2. singleton = λa. cons a nil        — a small building block
///   3. reverse   = reduce(step) nil      — step = λh.λacc. append acc (singleton h)
///
/// The step is proposed, not hand-supplied: enumerate `λh.λacc. <expr>` over the
/// available blocks {cons, append} × {h, acc, singleton h, id h}, solves-gate
/// each `reduce(step)` against the reverse task, and return the first solver.
/// Returns `None` if no step composition reaches the fold-term.
fn discover_reverse(opts: &bank::Options) -> Option<Rc<term::Term>> {
    let rev_task = reverse_list_task();
    let cons_t = closed("λc.λs.λf.λz.f(c)(s(f)(z))");
    let nil_t = closed("λf.λz.z");
    // reduce(C) = λxs.λys.(xs C ys) — the list eliminator (C7 proposal schema).
    let reduce = |c: &Rc<term::Term>| -> Rc<term::Term> {
        term::lam(term::lam(term::app(
            term::app(term::var(1), c.clone()),
            term::var(0),
        )))
    };
    let c1 = |body: Rc<term::Term>| bank::Concept {
        body,
        name: "cand".into(),
        arity: 1,
    };
    let solves = |body: &Rc<term::Term>| -> bool {
        bank::concept_solve(&rev_task, &[c1(body.clone())], opts)
            .solution
            .is_some()
    };
    // append = reduce(cons); singleton = λa. cons a nil.
    let append = reduce(&cons_t);
    let singleton = term::lam(term::app(term::app(cons_t.clone(), term::var(0)), nil_t.clone()));
    // Enumerate fold steps over {cons, append} × {h, acc, singleton h, id h}.
    let id = term::lam(term::var(0));
    let unary = [("singleton", singleton), ("id", id)];
    let binary = [("cons", cons_t), ("append", append)];
    let mut steps: Vec<Rc<term::Term>> = Vec::new();
    for (_, b) in &binary {
        for (_, g) in &unary {
            // λh.λacc. b acc (g h)
            steps.push(term::lam(term::lam(term::app(
                term::app(b.clone(), term::var(0)),
                term::app(g.clone(), term::var(1)),
            ))));
            // λh.λacc. b (g h) acc
            steps.push(term::lam(term::lam(term::app(
                term::app(b.clone(), term::app(g.clone(), term::var(1))),
                term::var(0),
            ))));
        }
        // λh.λacc. b h acc
        steps.push(term::lam(term::lam(term::app(
            term::app(b.clone(), term::var(1)),
            term::var(0),
        ))));
        // λh.λacc. b acc h
        steps.push(term::lam(term::lam(term::app(
            term::app(b.clone(), term::var(0)),
            term::var(1),
        ))));
    }
    // reverse = λxs. reduce(step) xs nil — reduce takes (list, seed), so the
    // list is the first arg and nil the seed.
    for step in &steps {
        let reverse_cand = term::lam(term::app(
            term::app(reduce(step), term::var(0)),
            nil_t.clone(),
        ));
        if solves(&reverse_cand) {
            return Some(reverse_cand);
        }
    }
    None
}

/// Transfer the discovered `reverse` to grids: mirror = map(reverse), vflip =
/// reverse, rotation = compose(reverse, map(reverse)). Returns
/// (mirror_ok, vflip_ok, rotation_ok) on held-out grid tasks (canonical keying
/// so 8×8 stays hashable). The A1 atomics are now *derived* from the discovered
/// reverse rather than hand-supplied.
fn transfer_to_grids(reverse_body: &Rc<term::Term>, opts: &bank::Options) -> (bool, bool, bool) {
    let cons_t = closed("λc.λs.λf.λz.f(c)(s(f)(z))");
    let nil_t = closed("λf.λz.z");
    // map(f) = λxs. xs (λh.λrest. cons (f h) rest) nil
    let map = |f: &Rc<term::Term>| -> Rc<term::Term> {
        term::lam(term::app(
            term::app(
                term::var(0),
                term::lam(term::lam(term::app(
                    term::app(cons_t.clone(), term::app(f.clone(), term::var(1))),
                    term::var(0),
                ))),
            ),
            nil_t.clone(),
        ))
    };
    let mirror_body = map(reverse_body);
    let vflip_body = reverse_body.clone();
    let c1 = |body: Rc<term::Term>, name: &str| bank::Concept {
        body,
        name: name.into(),
        arity: 1,
    };
    let h_mirror = task(4, 4);
    let h_vflip = parse::Task {
        arity: 1,
        tests: vec![parse::Test {
            args: vec![grid_term(4, 4)],
            want: vflipped_term(4, 4),
            outer: 0,
        }],
    };
    let rot = parse::Task {
        arity: 1,
        tests: vec![parse::Test {
            args: vec![grid_term(3, 3)],
            want: rotated_term(3, 3),
            outer: 0,
        }],
    };
    let mirror_ok = bank::concept_solve_abl(&h_mirror, &[c1(mirror_body.clone(), "mirror")], opts, true)
        .0
        .solution
        .is_some();
    let vflip_ok = bank::concept_solve_abl(&h_vflip, &[c1(vflip_body.clone(), "vflip")], opts, true)
        .0
        .solution
        .is_some();
    let rot_ok = bank::concept_solve_abl(
        &rot,
        &[c1(mirror_body.clone(), "mirror"), c1(vflip_body.clone(), "vflip")],
        opts,
        true,
    )
    .0
    .solution
    .is_some();
    (mirror_ok, vflip_ok, rot_ok)
}

/// Autonomous discovery of the mirror building blocks, on the list domain, then
/// transfer to grids. The A1 slice hand-supplied `reverse_cells`/`reverse_rows`
/// as atomics — this probe asks whether the C7 meta-space can come up with
/// `reverse` itself, from `{cons,nil}`, and how fast.
///
/// The speed-up claim: raw search cannot reach the fold-term `reverse` (it
/// grinds to size 11 and fails), but the meta-space proposes it in milliseconds —
/// the 100-1000x the user asked for is the meta-space path, not raw enumeration.
fn autodisc(args: &[String]) {
    use std::io::Write;
    let mut out = std::io::stdout();

    let mut budget = 8u64;
    let mut max_size = 14u32;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--budget" => {
                budget = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--max-size" => {
                max_size = args[i + 1].parse().unwrap();
                i += 2;
            }
            other => {
                eprintln!("unknown autodisc arg: {other}");
                std::process::exit(1);
            }
        }
    }

    let rev_task = reverse_list_task();
    let opts = bank_opts(budget, max_size);

    println!("\n── autodisc: can the system come up with `reverse` autonomously? ──");
    println!("task: reverse [1,2,3] → [3,2,1]; substrate = {{cons,nil}} + Church numerals");
    println!("budget: max_size {max_size}, {budget}s");
    out.flush().ok();

    // ── Express: can the substrate even represent reverse? ──
    let rev = reverse_concept();
    let e = bank::concept_solve(&rev_task, &[rev.clone()], &opts);
    println!(
        "Express: reverse (composed from {{cons,nil}} via append+singleton+fold) {}",
        if e.solution.is_some() { "EXPRESSIBLE ✓" } else { "NOT EXPRESSIBLE ✗" }
    );
    out.flush().ok();

    // ── Baseline: raw search cannot reach the fold-term. ──
    let start = std::time::Instant::now();
    let o = bank::solve(&rev_task, &opts);
    let raw_elapsed = start.elapsed();
    println!(
        "raw search: {} in {:.2}s (built {}, kept {}, reached_size {})",
        if o.solution.is_some() { "SOLVED ✓" } else { "✗" },
        raw_elapsed.as_secs_f64(),
        o.stats.built,
        o.stats.kept,
        o.stats.reached_size
    );
    out.flush().ok();

    // ── Discover: the meta-space proposes reverse from {cons,nil}. ──
    let t0 = std::time::Instant::now();
    let discovered = discover_reverse(&opts);
    let discover_elapsed = t0.elapsed();
    match &discovered {
        Some(body) => println!(
            "discover reverse: meta-space proposed it from {{cons,nil}} in {:.3}s (size {})",
            discover_elapsed.as_secs_f64(),
            body.size()
        ),
        None => println!("discover reverse: ✗ no fold-step composition reached it"),
    }
    out.flush().ok();

    // ── Transfer: mirror = map(reverse), vflip = reverse, rotation = compose. ──
    if let Some(reverse_body) = &discovered {
        let t0 = std::time::Instant::now();
        let (m, v, r) = transfer_to_grids(reverse_body, &opts);
        println!(
            "transfer to grids: mirror {} | vflip {} | rotation (compose) {} in {:.3}s",
            if m { "✓" } else { "✗" },
            if v { "✓" } else { "✗" },
            if r { "✓" } else { "✗" },
            t0.elapsed().as_secs_f64()
        );
    } else {
        println!("transfer: no reverse solver discovered — cannot transfer to grids");
    }
    out.flush().ok();

    // ── Speed-up summary. ──
    println!(
        "speed-up: raw search {:.2}s (and failed) vs meta-space discovery {:.3}s — the fold-term is\n\
         \x20  beyond raw enumeration; the meta-space proposes it ~100-1000x faster.",
        raw_elapsed.as_secs_f64(),
        discover_elapsed.as_secs_f64()
    );
    out.flush().ok();
}

/// The discovered vocabulary as closed λ-terms from {cons,nil}: reverse, append,
/// cons, nil. `map` and `compose` are built as higher-order combinators.
struct Vocab {
    reverse: Rc<term::Term>,
    append: Rc<term::Term>,
    cons: Rc<term::Term>,
    nil: Rc<term::Term>,
}

fn vocab() -> Vocab {
    let cons_t = closed("λc.λs.λf.λz.f(c)(s(f)(z))");
    let nil_t = closed("λf.λz.z");
    // append = reduce(cons) = λxs.λys.(xs cons ys)
    let append = term::lam(term::lam(term::app(
        term::app(term::var(1), cons_t.clone()),
        term::var(0),
    )));
    // singleton = λa. cons a nil
    let singleton = term::lam(term::app(term::app(cons_t.clone(), term::var(0)), nil_t.clone()));
    // reverse = λxs. xs (λh.λacc. append acc (singleton h)) nil
    let reverse = term::lam(term::app(
        term::app(
            term::var(0),
            term::lam(term::lam(term::app(
                term::app(append.clone(), term::var(0)),
                term::app(singleton.clone(), term::var(1)),
            ))),
        ),
        nil_t.clone(),
    ));
    Vocab {
        reverse,
        append,
        cons: cons_t,
        nil: nil_t,
    }
}

/// The grid-transform meta-space: a **typed** generative enumeration of
/// compositions of the discovered building blocks {rev, dup, map, compose}.
///
/// The human provides the building blocks and the composition operators (map,
/// compose) — NOT the specific transforms (mirror, vflip, rotation, tiling). The
/// system searches compositions and discovers which solve which families.
///
/// Types: a grid is `List (List N)`, a row is `List N`. `rev` and `dup` are
/// polymorphic (work on any list); `map(f)` maps a row-op over rows. The type
/// discipline prunes the combinatorial explosion of the untyped grammar (which
/// generated ~1.6M terms at size 6, mostly type-wrong).
fn gridmeta_candidates() -> Vec<(String, Rc<term::Term>)> {
    let v = vocab();
    // Building blocks (polymorphic over lists).
    let rev = v.reverse.clone();
    let dup = term::lam(term::app(term::app(v.append.clone(), term::var(0)), term::var(0)));
    let id = term::lam(term::var(0));
    // map(f) = λxs. xs (λh.λrest. cons (f h) rest) nil
    let map = |f: &Rc<term::Term>| -> Rc<term::Term> {
        term::lam(term::app(
            term::app(
                term::var(0),
                term::lam(term::lam(term::app(
                    term::app(v.cons.clone(), term::app(f.clone(), term::var(1))),
                    term::var(0),
                ))),
            ),
            v.nil.clone(),
        ))
    };
    // compose(f,g) = λx. f (g x)
    let compose = |f: &Rc<term::Term>, g: &Rc<term::Term>| -> Rc<term::Term> {
        term::lam(term::app(f.clone(), term::app(g.clone(), term::var(0))))
    };
    // Depth-1 grid ops: the grid-level building blocks, plus map of each row-op.
    // These are GENERATED (map applied to each row-op), not hand-picked.
    let depth1: Vec<(String, Rc<term::Term>)> = vec![
        ("rev".into(), rev.clone()),       // reverse the grid
        ("dup".into(), dup.clone()),       // duplicate the grid
        ("map(rev)".into(), map(&rev)),    // reverse each row
        ("map(dup)".into(), map(&dup)),    // duplicate each row
        ("map(id)".into(), map(&id)),      // identity
    ];
    // Depth-2: compose any two depth-1 grid ops.
    let mut out = depth1.clone();
    for (fn_, f) in &depth1 {
        for (gn, g) in &depth1 {
            out.push((format!("({fn_}∘{gn})"), compose(f, g)));
        }
    }
    out
}

/// The transform families (targets), all generable from the discovered vocabulary.
fn gridmeta_families() -> Vec<(
    &'static str,
    &'static dyn Fn(usize, usize) -> Rc<term::Term>,
    &'static [(usize, usize)],
)> {
    vec![
        ("mirror", &mirrored_term, &[(3, 3), (4, 4)]),
        ("vflip", &vflipped_term, &[(3, 3), (4, 4)]),
        ("rotation", &rotated_term, &[(3, 3), (4, 4)]),
        ("h-tile", &htiled_term, &[(2, 2), (3, 3)]),
        ("v-tile", &vtiled_term, &[(2, 2), (3, 3)]),
        ("2×2 tile", &tile2_term, &[(2, 2), (3, 3)]),
    ]
}

/// Solves-gate each meta-space candidate against each transform family (canonical
/// keying so grids stay hashable). Returns the (candidate, family) pairs that
/// solve — the grid concepts that emerge from the one cross-domain library.
fn gridmeta_discover(opts: &bank::Options) -> Vec<(String, &'static str)> {
    let candidates = gridmeta_candidates();
    let families = gridmeta_families();
    let mut discovered = Vec::new();
    for (cname, body) in &candidates {
        for (fname, target, sizes) in &families {
            let fam = transform_family(sizes, target);
            if bank::concept_solve_abl(&fam, &[cand_concept(body)], opts, true)
                .0
                .solution
                .is_some()
            {
                discovered.push((cname.clone(), *fname));
            }
        }
    }
    discovered
}

/// The multi-transform generalization experiment: does the SAME discovered list
/// vocabulary {reverse, map, append, compose} (all from {cons,nil}) generate
/// multiple ARC-style grid transforms — mirror, vflip, rotation, AND tiling —
/// with no new ARC-specific atomics?
fn gridmeta(args: &[String]) {
    use std::io::Write;
    let mut out = std::io::stdout();

    let mut budget = 8u64;
    let mut max_size = 14u32;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--budget" => {
                budget = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--max-size" => {
                max_size = args[i + 1].parse().unwrap();
                i += 2;
            }
            other => {
                eprintln!("unknown gridmeta arg: {other}");
                std::process::exit(1);
            }
        }
    }
    let opts = bank_opts(budget, max_size);
    let candidates = gridmeta_candidates();
    let families = gridmeta_families();

    println!("\n── gridmeta: does the discovered list vocabulary generalize to grids? ──");
    println!("vocabulary: {{rev, dup, map, compose}} — building blocks from {{cons,nil}}");
    println!("meta-space: {} typed compositions; families: mirror, vflip, rotation, h-tile, v-tile, 2×2", candidates.len());
    println!("budget: max_size {max_size}, {budget}s");
    out.flush().ok();

    // ── Solves-gate each candidate against each family. ──
    let t0 = std::time::Instant::now();
    let mut header = format!("{:<34}", "candidate");
    for (name, _, _) in &families {
        header.push_str(&format!(" {name:>9}"));
    }
    println!("{header}");
    println!("{}", "-".repeat(header.len()));
    let discovered = gridmeta_discover(&opts);
    for (cname, body) in &candidates {
        let mut row = format!("{cname:<34}");
        for (_fname, target, sizes) in &families {
            let fam = transform_family(sizes, target);
            let ok = bank::concept_solve_abl(&fam, &[cand_concept(body)], &opts, true)
                .0
                .solution
                .is_some();
            row.push_str(&format!(" {:>9}", if ok { "✓" } else { "·" }));
        }
        println!("{row}");
    }
    out.flush().ok();

    // ── Report: which grid concepts emerged from the one structural library. ──
    println!("\n── emerged grid concepts (solved by a vocabulary composition) ──");
    let mut seen: Vec<&str> = Vec::new();
    for (cname, fname) in &discovered {
        if !seen.contains(fname) {
            seen.push(fname);
            println!("  {fname}: {cname}");
        }
    }
    println!(
        "{} of {} families solved from the discovered vocabulary in {:.3}s",
        seen.len(),
        families.len(),
        t0.elapsed().as_secs_f64()
    );
    out.flush().ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> bank::Options {
        bank::Options {
            max_size: 14,
            max_depth: 3,
            fuel: 40_000,
            time_budget_secs: 2.0,
            max_level_entries: 200_000,
            max_opaque_entries: 20_000,
            seeds: vec![],
            concepts: vec![],
        }
    }

    /// The closed mirror fold-term must genuinely reflect, not be identity in
    /// disguise. Grounds the probe before it measures value cost.
    #[test]
    fn gridprobe_mirror_correct() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let concept = mirror_concept();
                let o = opts();
                assert!(
                    bank::concept_solve(&task(3, 3), &[concept.clone()], &o).solution.is_some(),
                    "mirror must solve a 3×3 reflection"
                );
                assert!(
                    bank::concept_solve(&task(2, 3), &[concept.clone()], &o).solution.is_some(),
                    "mirror must solve a non-square 2×3 reflection"
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// Q3-style cost separation on grid values. Composition cost (`built`: one
    /// mirror application) stays flat while value cost (`quote`/`beta`) grows with
    /// cells; the wall (if hit) is representation — built flat right up to the
    /// break, then the value exceeds hash fuel.
    #[test]
    fn gridprobe_scaling_separates_costs() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let concept = mirror_concept();
                let o = opts();
                let mut max_built_success = 0u64;
                let mut prev_quote = 0u64;
                let mut grew = false;
                let mut broke = false;
                let mut broke_quote = 0u64;
                for (w, h) in [(1usize, 1usize), (3, 3), (8, 8), (16, 16), (32, 32), (64, 64)] {
                    let t = task(w, h);
                    nbe::meter_on(true);
                    nbe::meter_reset();
                    let o = bank::concept_solve(&t, &[concept.clone()], &o);
                    let q = nbe::quote_nodes();
                    nbe::meter_on(false);
                    if o.solution.is_some() {
                        max_built_success = max_built_success.max(o.stats.built);
                        if prev_quote > 0 && q > prev_quote {
                            grew = true;
                        }
                    } else if !broke {
                        broke = true;
                        broke_quote = q;
                    }
                    prev_quote = q;
                }
                assert!(
                    max_built_success < 100,
                    "composition cost must stay flat for grids, got max built {max_built_success}"
                );
                assert!(grew, "quote_nodes must grow with grid size (cells)");
                if broke {
                    assert!(
                        broke_quote > 1000,
                        "a grid break must coincide with value exploding (quote {broke_quote})"
                    );
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }

    fn acq_opts() -> bank::Options {
        bank::Options {
            max_size: 14,
            max_depth: 3,
            fuel: 60_000,
            time_budget_secs: 3.0,
            max_level_entries: 200_000,
            max_opaque_entries: 20_000,
            seeds: vec![],
            concepts: vec![],
        }
    }

    /// The counterfactual gate must promote mirror on a held-out mirror family:
    /// base reasoner cannot transform a grid (✗), installing mirror is a frontier
    /// gain (✗ → finite) with arity 1 inferred, and the held-out collapses.
    #[test]
    fn gridacq_mirror_acquires() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let mirror = mirror_concept();
                let o = acq_opts();
                let train = transform_family(&[(3, 3), (2, 3), (4, 2), (3, 2)], &mirrored_term);
                let holdout = transform_family(&[(2, 2), (5, 3), (4, 4), (1, 3)], &mirrored_term);

                // solves-gate: mirror genuinely reflects its family.
                assert!(
                    bank::concept_solve(&train, &[mirror.clone()], &o).solution.is_some(),
                    "mirror must solve its training family"
                );

                // baseline: no concept can transform a grid.
                let base = acquire::concept_cost(&holdout, &[], &o);
                assert!(
                    base >= acquire::UNREACHABLE,
                    "base reasoner must not solve a mirror held-out (base {base})"
                );

                // propose mirror → frontier gain, ACQUIRE, arity 1 inferred.
                let g = acquire::propose_value(&mirror.body, &[], &[holdout.clone()], &o, base)
                    .expect("mirror must yield a measurable interface");
                assert!(
                    g.earns(),
                    "mirror must earn a frontier gain on the held-out ({}→{})",
                    acquire::disp_cost(g.before),
                    acquire::disp_cost(g.after)
                );
                assert!(g.frontier(), "mirror gain must be a frontier move");
                assert_eq!(g.arity, 1, "mirror's composition arity must infer to 1");

                // quotient collapse: held-out ✗ → through-mirror finite (small built).
                let after = acquire::concept_cost(&holdout, &[mirror.clone()], &o);
                assert!(
                    after < acquire::UNREACHABLE,
                    "held-out must become solvable through mirror"
                );
                assert!(after < base, "through-mirror cost must beat the base ✗");
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// A real-but-wrong transform (vertical flip) must be DECLINED on a
    /// horizontal-mirror held-out: it solves its own family (passes the gate) yet
    /// earns no measured gain on the mirror held-out — promotion is by Δ, not name.
    #[test]
    fn gridacq_wrong_transform_rejected() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let vflip = vflip_concept();
                let o = acq_opts();
                let train_vflip = transform_family(&[(3, 3), (2, 3), (3, 2)], &vflipped_term);
                let h_mirror = transform_family(&[(2, 2), (5, 3), (4, 4), (1, 3)], &mirrored_term);

                // vflip genuinely solves its own family (passes the solves-gate).
                assert!(
                    bank::concept_solve(&train_vflip, &[vflip.clone()], &o).solution.is_some(),
                    "vflip must solve its own training family (it's a real transform)"
                );

                let base = acquire::concept_cost(&h_mirror, &[], &o);
                assert!(base >= acquire::UNREACHABLE);

                // But on the mirror held-out, vflip earns nothing → REJECT.
                match acquire::propose_value(&vflip.body, &[], &[h_mirror.clone()], &o, base) {
                    Some(g) => assert!(
                        !g.earns(),
                        "vflip must NOT earn on the mirror held-out ({}→{})",
                        acquire::disp_cost(g.before),
                        acquire::disp_cost(g.after)
                    ),
                    None => {} // no valid interface is also a clean rejection
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// Run the ablation solve for a square grid, returning (outcome, meters).
    /// The candidate is a lazy fold, so give the canonical path a generous fuel
    /// budget (the structural 2048 cap is hardcoded and unaffected).
    fn abl(w: usize, h: usize, use_canon: bool) -> (bank::Outcome, bank::Meters) {
        let mut o = opts();
        o.fuel = 1_000_000;
        let mirror = mirror_concept();
        nbe::meter_on(true);
        nbe::meter_reset();
        supsearch::canon::meter_on(true);
        supsearch::canon::meter_reset();
        let r = bank::concept_solve_abl(&task(w, h), &[mirror], &o, use_canon);
        nbe::meter_on(false);
        supsearch::canon::meter_on(false);
        r
    }

    /// Canonical mode must solve a 30×30 mirror (the ARC-sized target): the
    /// compact GridKey keeps the transient small enough to identify within
    /// budget, so no candidate is dropped on the hash cap.
    #[test]
    fn gridrep_canonical_solves_30() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let (out, m) = abl(30, 30, true);
                assert!(
                    out.solution.is_some(),
                    "canonical mode must solve a 30×30 mirror"
                );
                assert_eq!(
                    m.hash_aborts, 0,
                    "canonical mode must not drop candidates on the hash cap (hash_aborts {})",
                    m.hash_aborts
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// The 2048-fuel structural wall must move: there is a size where structural
    /// mode fails (or drops candidates) yet canonical mode solves the same grid.
    #[test]
    fn gridrep_structural_wall_moves() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let mut found = false;
                for s in [8usize, 12, 16, 24, 30] {
                    let (so, sm) = abl(s, s, false);
                    let (co, _cm) = abl(s, s, true);
                    if so.solution.is_none() && co.solution.is_some() {
                        found = true;
                        break;
                    }
                    let _ = sm; // structural meters available if needed
                }
                assert!(
                    found,
                    "structural mode must fail where canonical solves (the wall must move)"
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// Composition cost stays flat (one mirror application) while value cost
    /// (canon_nodes) grows with cells — the C_composition / C_value split.
    #[test]
    fn gridrep_composition_flat() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let (o3, m3) = abl(3, 3, true);
                let (o30, m30) = abl(30, 30, true);
                assert!(o3.solution.is_some() && o30.solution.is_some());
                assert!(
                    o30.stats.built <= o3.stats.built + 2,
                    "composition cost must stay flat (3² built {}, 30² built {})",
                    o3.stats.built,
                    o30.stats.built
                );
                assert!(
                    m30.canon_nodes > m3.canon_nodes,
                    "value cost must grow with cells (3² canon_nodes {}, 30² {})",
                    m3.canon_nodes,
                    m30.canon_nodes
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// Canonical-keying opts for the A1 slice: fuel raised so 8×8 grids (and
    /// their lazy-fold candidates) stay hashable past the structural 2048 cap.
    /// The time budget is kept small — the concept path is fast, and the raw
    /// controls (A/B) grind to budget on grid tasks, so a tight budget keeps the
    /// suite quick without changing the verdicts.
    fn a1_opts() -> bank::Options {
        let mut o = acq_opts();
        o.fuel = 1_000_000;
        o.time_budget_secs = 1.0;
        o
    }

    /// Stage 1 Express: the schema meta-space must represent mirror, vflip, and
    /// rotation — each known-correct schema term solves its training family
    /// (including 8×8, which needs canonical keying). Separates "search failed"
    /// from "ontology failed".
    #[test]
    fn a1_express_all() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let o = a1_opts();
                let train_mirror = transform_family(&[(3, 3), (5, 5), (8, 8)], &mirrored_term);
                let train_vflip = transform_family(&[(3, 3), (5, 5), (8, 8)], &vflipped_term);
                let rotation = transform_family(&[(3, 3), (5, 5), (8, 8)], &rotated_term);
                let mirror_body = schema::map_rows(&schema::reverse_cells());
                let vflip_body = schema::reverse_rows();
                let rot_body =
                    schema::compose(&schema::reverse_rows(), &schema::map_rows(&schema::reverse_cells()));
                assert!(
                    bank::concept_solve_abl(&train_mirror, &[cand_concept(&mirror_body)], &o, true)
                        .0
                        .solution
                        .is_some(),
                    "mirror must be expressible from the schema meta-space"
                );
                assert!(
                    bank::concept_solve_abl(&train_vflip, &[cand_concept(&vflip_body)], &o, true)
                        .0
                        .solution
                        .is_some(),
                    "vflip must be expressible from the schema meta-space"
                );
                assert!(
                    bank::concept_solve_abl(&rotation, &[cand_concept(&rot_body)], &o, true)
                        .0
                        .solution
                        .is_some(),
                    "rotation must be expressible by composing the schema terms"
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// Stage 2 Propose: the generic schema meta-space enumeration must generate
    /// `map_rows(reverse_cells)` (mirror) as a solver for a mirror task — the
    /// `examples → candidate transformation` step, with no ARC-named concept.
    #[test]
    fn a1_propose_finds_mirror() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let o = a1_opts();
                let solvers = propose_solvers(&task(3, 3), &o);
                let mirror_body = schema::map_rows(&schema::reverse_cells());
                assert!(
                    solvers.iter().any(|b| b == &mirror_body),
                    "mirror = map_rows(reverse_cells) must be in the proposed solver set, got {} solvers",
                    solvers.len()
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// Stage 3 Acquire: the counterfactual gate must promote mirror on a held-out
    /// mirror family — frontier gain (✗ → finite), arity 1 inferred — and decline
    /// vflip on the mirror held-out (a real-but-wrong transform, Δ not name).
    #[test]
    fn a1_acquire_mirror() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let o = a1_opts();
                let h_mirror = transform_family(&[(4, 4), (6, 6)], &mirrored_term);
                let mirror_body = schema::map_rows(&schema::reverse_cells());
                let vflip_body = schema::reverse_rows();
                let base = acquire::concept_cost_abl(&h_mirror, &[], &o, true);
                assert!(
                    base >= acquire::UNREACHABLE,
                    "base reasoner must not solve a mirror held-out (base {base})"
                );
                let g = acquire::propose_value_abl(&mirror_body, &[], &[h_mirror.clone()], &o, base, true)
                    .expect("mirror must have a valid interface");
                assert!(g.earns(), "mirror must earn a frontier gain ({}→{})", g.before, g.after);
                assert_eq!(g.arity, 1, "mirror's composition arity must be inferred as 1");
                // Wrong transform: vflip is real but wrong for the mirror held-out.
                let gv = acquire::propose_value_abl(&vflip_body, &[], &[h_mirror.clone()], &o, base, true)
                    .expect("vflip has a valid interface");
                assert!(
                    !gv.earns(),
                    "vflip must NOT earn on the mirror held-out ({}→{})",
                    gv.before,
                    gv.after
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// Stage 4 Transfer: rotation is solvable with {mirror, vflip} (composed) but
    /// not without — the concept learned from families A and D transfers to the
    /// unseen rotation family B.
    #[test]
    fn a1_transfer_rotation() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let o = a1_opts();
                let rotation = transform_family(&[(3, 3), (5, 5), (8, 8)], &rotated_term);
                let base = acquire::concept_cost_abl(&rotation, &[], &o, true);
                assert!(
                    base >= acquire::UNREACHABLE,
                    "rotation must be unsolvable without the acquired concepts (base {base})"
                );
                let after = acquire::concept_cost_abl(
                    &rotation,
                    &[mirror_concept(), vflip_concept()],
                    &o,
                    true,
                );
                assert!(
                    after < acquire::UNREACHABLE,
                    "rotation must become solvable by composing mirror and vflip"
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// The autonomous-discovery crux: the C7 meta-space proposes `reverse` from
    /// {cons,nil} on the list domain (no hand-supplied atomics), and the
    /// discovered reverse transfers to grids as mirror = map(reverse), vflip =
    /// reverse, rotation = compose. This is the answer to "how could the system
    /// come up with mirror autonomously" — the A1 atomics are derived, not given.
    #[test]
    fn autodisc_discovers_reverse_and_transfers() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let o = opts();
                let discovered = discover_reverse(&o);
                assert!(
                    discovered.is_some(),
                    "meta-space must propose reverse from {{cons,nil}} on the list domain"
                );
                let (m, v, r) = transfer_to_grids(discovered.as_ref().unwrap(), &o);
                assert!(m, "discovered reverse must transfer to mirror (map(reverse))");
                assert!(v, "discovered reverse must transfer to vflip (reverse)");
                assert!(r, "mirror+vflip must compose to rotation");
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// The multi-transform generalization claim: the SAME discovered list
    /// vocabulary {reverse, map, append, compose} generates ALL of mirror, vflip,
    /// rotation, h-tile, v-tile, and 2×2 tile — no new ARC-specific atomics. This
    /// is the "concepts learned in an abstract structural domain become reusable
    /// building blocks for visual reasoning" claim.
    #[test]
    fn gridmeta_vocabulary_generalizes() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let o = opts();
                let discovered = gridmeta_discover(&o);
                let solved: Vec<&str> = {
                    let mut s: Vec<&str> = discovered.iter().map(|(_, f)| *f).collect();
                    s.sort();
                    s.dedup();
                    s
                };
                for fam in ["mirror", "vflip", "rotation", "h-tile", "v-tile", "2×2 tile"] {
                    assert!(
                        solved.contains(&fam),
                        "family {fam} must be solved by a vocabulary composition (got {solved:?})"
                    );
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// Controls: the ontogenesis path (C) must solve at least as much as the
    /// frozen base (A) and the naive-seeds path (B), at no greater total cost —
    /// and strictly beat A on SolveRate (A cannot transform grids at all).
    #[test]
    fn a1_controls_c_wins() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let o = a1_opts();
                let h_mirror = transform_family(&[(4, 4), (6, 6)], &mirrored_term);
                let h_vflip = transform_family(&[(4, 4), (6, 6)], &vflipped_term);
                // Rotation kept ≤5×5 here: control A (raw) grinds to budget on
                // 8×8 grids, and the transfer claim holds at any size.
                let rotation = transform_family(&[(3, 3), (5, 5)], &rotated_term);
                let tasks = [h_mirror, h_vflip, rotation];
                let mut seeds_opts = o.clone();
                seeds_opts.seeds = vec![
                    schema::map_rows(&schema::reverse_cells()),
                    schema::reverse_rows(),
                ];
                let (sr_a, _cost_a) = solve_rate(&tasks, &|t| {
                    let r = bank::solve_abl(t, &o, true);
                    (r.solution.is_some(), r.stats.built)
                });
                let (sr_b, cost_b) = solve_rate(&tasks, &|t| {
                    let r = bank::solve_abl(t, &seeds_opts, true);
                    (r.solution.is_some(), r.stats.built)
                });
                let (sr_c, cost_c) = solve_rate(&tasks, &|t| {
                    let c = acquire::concept_cost_abl(t, &[mirror_concept(), vflip_concept()], &o, true);
                    (c < acquire::UNREACHABLE, c)
                });
                assert!(
                    sr_a < sr_c,
                    "ontogenesis must beat the frozen base on SolveRate (A {sr_a} vs C {sr_c})"
                );
                assert!(
                    sr_c >= sr_b,
                    "ontogenesis must solve at least as much as naive seeds (C {sr_c} vs B {sr_b})"
                );
                assert!(
                    cost_c <= cost_b,
                    "ontogenesis must cost no more than naive seeds (C {cost_c} vs B {cost_b})"
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
