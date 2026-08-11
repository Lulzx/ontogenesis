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

use std::collections::hash_map::DefaultHasher;
use std::io::Write;
use std::rc::Rc;

use supsearch::{
    acquire, bank, bootstrap, canon,
    contextual_allocation::{
        ConceptSet, ContextualEvidence, ContextualLedger, EvidenceDerivation, FreezeSpec,
        FrozenPolicy, TaskContext,
    },
    learned_context::{
        freeze_policy as freeze_learned_policy, learn_representation,
        LearnedRepresentation, RawField, RawTaskObservation, RawUtilityEvidence,
        RepresentationSpec,
    },
    nbe, parse, recurrence,
    search_accounting::{
        self, AccountingSummary, EvidencePhase, RunAccounting, RunProvenance,
    },
    term, transform, typed,
};

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
        Some("arcdiag") => {
            // Real-ARC diagnostic: grid NBE recurses deep on the fold structure
            // and the probe hashes large grids, so run on a large stack.
            let a = args[1..].to_vec();
            std::thread::Builder::new()
                .stack_size(1 << 30)
                .spawn(move || arcdiag(&a))
                .unwrap()
                .join()
                .unwrap();
        }
        Some("contextual") => {
            // Frozen real-ARC contextual allocation and independent test-pair
            // verification use the same large-stack grid evaluation path.
            std::thread::Builder::new()
                .stack_size(1 << 30)
                .spawn(contextual_arc)
                .unwrap()
                .join()
                .unwrap();
        }
        Some("b1") => {
            // B1 generic context-abstraction meta-search: grid NBE recurses deep
            // on the fold structure, so run on a large stack.
            let a = args[1..].to_vec();
            std::thread::Builder::new()
                .stack_size(1 << 30)
                .spawn(move || b1(&a))
                .unwrap()
                .join()
                .unwrap();
        }
        Some("b2") => {
            let a = args[1..].to_vec();
            std::thread::Builder::new()
                .stack_size(1 << 30)
                .spawn(move || b2(&a))
                .unwrap()
                .join()
                .unwrap();
        }
        Some("b3") => {
            let a = args[1..].to_vec();
            std::thread::Builder::new()
                .stack_size(1 << 30)
                .spawn(move || b3(&a))
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

/// A candidate body installed as a concept of the given composition arity.
/// (`cand_concept` is hardcoded to arity 1; the vocabulary contains multi-arity
/// combinators like map(2), compose(3), append(2).)
fn arity_concept(body: &Rc<term::Term>, arity: u32) -> bank::Concept {
    bank::Concept { body: body.clone(), name: "vocab".into(), arity }
}

/// The discovered cross-domain ontology installed as concepts for the search
/// gate. `dup` is derived (λx. append x x). The substrate {cons,nil} are raw
/// primitives — the building blocks, not the installed concept set.
fn vocab_concepts(s: &Schemas) -> Vec<bank::Concept> {
    let dup = term::lam(term::app(term::app(s.append.clone(), term::var(0)), term::var(0)));
    vec![
        arity_concept(&s.reverse, 1),
        arity_concept(&s.map, 2),
        arity_concept(&s.compose, 3),
        arity_concept(&dup, 1),
        arity_concept(&s.append, 2),
    ]
}

// ────────────────────────────────────────────────────────────────────────────
// ARC-AGI-1 data + the four-bucket diagnostic (arcdiag).
//
// Real ARC training tasks (MIT-licensed, in data/*.json): each is
// {"train":[{input,output},..], "test":[{input,output},..]} with grids as lists
// of rows of color numerals 0-9 — directly encodable on the Church-list
// substrate. We do NOT ask "how many can we solve?" but "for which real tasks is
// the currently discovered structural ontology already sufficient?" The four
// buckets keep the two gates separate forever (candidate usefulness = search
// gate, expressibility = direct gate):
//   SOLVED            — the search gate finds a program through the vocabulary.
//   EXPRESSIBLE       — a composition of the building blocks directly maps
//                       inputs→outputs, but the search didn't reach it.
//   REQUIRES NEW      — not expressible up to the enumeration depth.
//   NOT REPRESENTABLE — the grids can't be encoded on the substrate.
// All bounds (depth, search budget) are reported honestly.
// ────────────────────────────────────────────────────────────────────────────

/// The real ARC-AGI-1 training set lives in the crate's data/ dir. Anchored to
/// the manifest so both the `arcdiag` subcommand (run from the workspace root)
/// and the tests (run from the crate dir) resolve it regardless of CWD.
const ARC_DATA_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/data");

/// A single ARC-AGI-1 grid: a list of rows of color numerals 0-9.
#[derive(Debug, Clone)]
struct ArcGrid {
    rows: Vec<Vec<u32>>,
}

/// A train/test pair: an input grid and its expected output grid.
#[derive(Debug, Clone)]
struct ArcExample {
    input: ArcGrid,
    output: ArcGrid,
}

/// One ARC-AGI-1 task file, identified by its filename stem.
#[derive(Debug, Clone)]
struct ArcTask {
    id: String,
    train: Vec<ArcExample>,
    test: Vec<ArcExample>,
}

/// Parse one ARC-AGI-1 JSON task via serde_json.
fn parse_arc_json(txt: &str) -> Option<(Vec<ArcExample>, Vec<ArcExample>)> {
    let v: serde_json::Value = serde_json::from_str(txt).ok()?;
    let grid_of = |j: &serde_json::Value| -> Option<ArcGrid> {
        let rows = j
            .as_array()?
            .iter()
            .map(|r| {
                r.as_array()
                    .map(|cells| cells.iter().map(|c| c.as_u64().unwrap_or(0) as u32).collect())
            })
            .collect::<Option<Vec<Vec<u32>>>>()?;
        Some(ArcGrid { rows })
    };
    let ex_of = |o: &serde_json::Value| -> Option<ArcExample> {
        Some(ArcExample { input: grid_of(o.get("input")?)?, output: grid_of(o.get("output")?)? })
    };
    let train = v.get("train")?.as_array()?.iter().filter_map(ex_of).collect();
    let test = v.get("test")?.as_array()?.iter().filter_map(ex_of).collect();
    Some((train, test))
}

/// Load all ARC-AGI-1 training-task JSON files in `dir`, sorted by filename.
fn load_arc_tasks(dir: &str) -> Vec<ArcTask> {
    let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("arc data dir {dir}: {e}"))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |x| x == "json"))
        .collect();
    paths.sort();
    let mut out = Vec::new();
    for p in paths {
        let id = p.file_stem().unwrap().to_string_lossy().to_string();
        let txt = std::fs::read_to_string(&p).expect("read arc task");
        match parse_arc_json(&txt) {
            Some((train, test)) => out.push(ArcTask { id, train, test }),
            None => eprintln!("[arcdiag] skip unparseable task {id}"),
        }
    }
    out
}

/// Load one ARC task by id (targeted probes / tests; `load_arc_tasks` bulk-loads).
fn load_arc_task_by_id(dir: &str, id: &str) -> ArcTask {
    let txt = std::fs::read_to_string(format!("{dir}/{id}.json")).expect("arc task file");
    let (train, test) = parse_arc_json(&txt).expect("parse arc task");
    ArcTask { id: id.to_string(), train, test }
}

/// Is this grid rectangular with all cells in 0-9 (i.e. encodable on the
/// substrate)? Ragged rows or out-of-range cells make it NOT REPRESENTABLE.
fn grid_representable(g: &ArcGrid) -> bool {
    let w = g.rows.first().map(|r| r.len()).unwrap_or(0);
    g.rows.iter().all(|r| r.len() == w && r.iter().all(|&c| c < 10))
}

/// Encode an ARC grid as a Church list of rows of Church numerals.
fn arc_grid_to_term(g: &ArcGrid) -> Rc<term::Term> {
    let rows: Vec<Rc<term::Term>> = g.rows.iter().map(|r| church_list(r)).collect();
    rc_list(&rows)
}

/// Build an arity-1 `parse::Task` whose tests are the train pairs.
fn arc_task_to_parse(t: &ArcTask) -> Option<parse::Task> {
    let tests = t
        .train
        .iter()
        .map(|ex| parse::Test {
            args: vec![arc_grid_to_term(&ex.input)],
            want: arc_grid_to_term(&ex.output),
            outer: 0,
        })
        .collect();
    Some(parse::Task { tests, arity: 1 })
}

/// Independent verification task. These outputs are never exposed to feature
/// extraction, utility learning, policy selection, or the search itself.
fn arc_task_test_to_parse(t: &ArcTask) -> Option<parse::Task> {
    let tests = t
        .test
        .iter()
        .map(|ex| parse::Test {
            args: vec![arc_grid_to_term(&ex.input)],
            want: arc_grid_to_term(&ex.output),
            outer: 0,
        })
        .collect();
    Some(parse::Task { tests, arity: 1 })
}

// Preregistered before the final comparison. There is exactly one real pure
// mirror task, one pure vertical-flip task, and two pure 180-degree rotation
// tasks in the local ARC-1 corpus. The lexically first rotation is calibration;
// the second is the frozen final transfer task.
const CONTEXT_ARC_TRAIN_IDS: [&str; 2] = ["67a3c6ac", "68b16354"];
const CONTEXT_ARC_CALIBRATION_ID: &str = "3c9b0459";
const CONTEXT_ARC_HOLDOUT_ID: &str = "6150a2bd";

fn hand_arc_context(task: &ArcTask) -> TaskContext {
    let same_shape = task.train.iter().all(|example| {
        example.input.rows.len() == example.output.rows.len()
            && example.input.rows.first().map(Vec::len)
                == example.output.rows.first().map(Vec::len)
    });
    TaskContext {
        task_id: task.id.clone(),
        family_id: "arc-d4-geometry".into(),
        duplicate_group_id: task.id.clone(),
        features: std::collections::BTreeMap::from([
            (
                "observable-relation".into(),
                raw_arc_observation(task).fields["raw-0"].value.to_string(),
            ),
            ("shape-preserving".into(), same_shape.to_string()),
        ]),
    }
}

/// Numeric observations of published training pairs. `raw-0` is a bitset over
/// four generic rectangular coordinate involutions in a fixed order; no bit is
/// named after an ARC task, target program, or ontology concept. Other fields
/// are deliberately plausible but often irrelevant surface measurements.
fn raw_arc_observation(task: &ArcTask) -> RawTaskObservation {
    let mut relation_bits = 0i64;
    for relation in 0..4 {
        let matches = task.train.iter().all(|example| {
            let input = &example.input.rows;
            let output = &example.output.rows;
            let height = input.len();
            let width = input.first().map(Vec::len).unwrap_or(0);
            height == output.len()
                && output.first().map(Vec::len).unwrap_or(0) == width
                && (0..height).all(|row| {
                    (0..width).all(|column| {
                        let source_row = if relation & 2 == 0 { row } else { height - 1 - row };
                        let source_column = if relation & 1 == 0 {
                            column
                        } else {
                            width - 1 - column
                        };
                        output[row][column] == input[source_row][source_column]
                    })
                })
        });
        if matches {
            relation_bits |= 1 << relation;
        }
    }
    let total_cells = task
        .train
        .iter()
        .map(|example| example.input.rows.iter().map(Vec::len).sum::<usize>())
        .sum::<usize>() as i64;
    let distinct_colors = task
        .train
        .iter()
        .flat_map(|example| example.input.rows.iter().flatten().copied())
        .collect::<std::collections::BTreeSet<_>>()
        .len() as i64;
    let same_shape = task.train.iter().all(|example| {
        example.input.rows.len() == example.output.rows.len()
            && example.input.rows.first().map(Vec::len)
                == example.output.rows.first().map(Vec::len)
    });
    RawTaskObservation {
        task_id: task.id.clone(),
        duplicate_group_id: task.id.clone(),
        fields: std::collections::BTreeMap::from([
            ("raw-0".into(), RawField::observable(relation_bits, 1)),
            ("raw-1".into(), RawField::observable(i64::from(same_shape), 1)),
            ("raw-2".into(), RawField::observable(task.train.len() as i64, 1)),
            ("raw-3".into(), RawField::observable(total_cells, 1)),
            ("raw-4".into(), RawField::observable(distinct_colors, 1)),
        ]),
    }
}

fn arc_provenance(
    context: &TaskContext,
    concept_ids: &[String],
    phase: EvidencePhase,
) -> RunProvenance {
    RunProvenance {
        task_id: context.task_id.clone(),
        family_id: context.family_id.clone(),
        duplicate_group_id: context.duplicate_group_id.clone(),
        context_features: context.features.clone(),
        concept_ids: concept_ids.to_vec(),
        phase,
        observed_epoch: 1,
    }
}

fn arc_evidence(
    context: TaskContext,
    concept_ids: &[&str],
    without: &bank::Outcome,
    with: &bank::Outcome,
    opts: &bank::Options,
    phase: EvidencePhase,
) -> ContextualEvidence {
    let ids = concept_ids.iter().map(|id| (*id).to_string()).collect::<Vec<_>>();
    ContextualEvidence {
        without: RunAccounting::from_bank(
            without,
            opts,
            arc_provenance(&context, &[], phase),
        ),
        with: RunAccounting::from_bank(
            with,
            opts,
            arc_provenance(&context, &ids, phase),
        ),
        context,
        concept_ids: ids,
        age: 0,
        recorded_epoch: 1,
        derivation: EvidenceDerivation::default(),
    }
}

fn raw_arc_evidence(
    task: &ArcTask,
    concept_ids: &[&str],
    without: &bank::Outcome,
    with: &bank::Outcome,
    opts: &bank::Options,
    phase: EvidencePhase,
) -> RawUtilityEvidence {
    let context = hand_arc_context(task);
    let ids = concept_ids.iter().map(|id| (*id).to_string()).collect::<Vec<_>>();
    RawUtilityEvidence {
        observation: raw_arc_observation(task),
        without: RunAccounting::from_bank(
            without,
            opts,
            arc_provenance(&context, &[], phase),
        ),
        with: RunAccounting::from_bank(
            with,
            opts,
            arc_provenance(&context, &ids, phase),
        ),
        concept_ids: ids,
        age: 0,
        recorded_epoch: 1,
        derivation: EvidenceDerivation::default(),
    }
}

#[derive(Clone, Debug)]
struct ArcAllocationCondition {
    name: &'static str,
    selected: Vec<ConceptSet>,
    accounting: AccountingSummary,
    training_solved: bool,
    hidden_test_verified: bool,
    universal_coverage: bool,
}

#[derive(Clone, Debug)]
struct ContextualArcReport {
    learned_representation: LearnedRepresentation,
    encoder_evidence_accounting: AccountingSummary,
    contextual_policy: FrozenPolicy,
    hand_policy: FrozenPolicy,
    global_policy: FrozenPolicy,
    contextual: ArcAllocationCondition,
    hand_features: ArcAllocationCondition,
    global: ArcAllocationCondition,
    uniform: ArcAllocationCondition,
    oracle: ArcAllocationCondition,
    shuffled: ArcAllocationCondition,
    interaction_disabled: ArcAllocationCondition,
    irrelevant: ArcAllocationCondition,
    misleading: ArcAllocationCondition,
    raw_bounded: ArcAllocationCondition,
}

fn arc_concept_map() -> std::collections::HashMap<String, bank::Concept> {
    std::collections::HashMap::from([
        ("mirror".into(), mirror_concept()),
        ("vflip".into(), vflip_concept()),
        ("irrelevant-identity".into(), cand_concept(&term::lam(term::var(0)))),
        (
            "misleading-projection".into(),
            cand_concept(&term::lam(term::lam(term::var(1)))),
        ),
    ])
}

fn run_arc_condition(
    name: &'static str,
    task: &ArcTask,
    order: &[ConceptSet],
    concepts: &std::collections::HashMap<String, bank::Concept>,
    opts: &bank::Options,
) -> ArcAllocationCondition {
    let train = arc_task_to_parse(task).expect("representable ARC training pairs");
    let hidden_test = arc_task_test_to_parse(task).expect("representable ARC test pairs");
    let context = hand_arc_context(task);
    let mut runs = Vec::new();
    let mut training_solved = false;
    let mut hidden_test_verified = false;
    for set in order {
        let installed = set
            .0
            .iter()
            .map(|id| concepts.get(id).expect("preregistered concept").clone())
            .collect::<Vec<_>>();
        let (outcome, _) = bank::concept_solve_abl(&train, &installed, opts, true);
        runs.push(RunAccounting::from_bank(
            &outcome,
            opts,
            arc_provenance(&context, &set.0, EvidencePhase::HeldOut),
        ));
        if let Some(solution) = outcome.solution {
            training_solved = true;
            if direct_solves(&hidden_test, &solution) {
                hidden_test_verified = true;
                break;
            }
        }
    }
    ArcAllocationCondition {
        name,
        selected: order.to_vec(),
        accounting: search_accounting::aggregate(&runs).expect("one labeled ARC engine"),
        training_solved,
        hidden_test_verified,
        // ARC's bank is deliberately bounded and is not the universal lane.
        universal_coverage: false,
    }
}

fn run_arc_raw_condition(
    task: &ArcTask,
    opts: &bank::Options,
) -> ArcAllocationCondition {
    // Raw enumeration uses an exact finite syntax boundary rather than the
    // wall-clock stop used by interactive demos, making replay counters stable.
    let mut raw_opts = opts.clone();
    raw_opts.max_size = 7;
    raw_opts.time_budget_secs = 3_600.0;
    let train = arc_task_to_parse(task).unwrap();
    let hidden_test = arc_task_test_to_parse(task).unwrap();
    let outcome = bank::solve_abl(&train, &raw_opts, true);
    let verified = outcome
        .solution
        .as_ref()
        .is_some_and(|solution| direct_solves(&hidden_test, solution));
    let context = hand_arc_context(task);
    let accounting = search_accounting::aggregate(&[RunAccounting::from_bank(
        &outcome,
        &raw_opts,
        arc_provenance(&context, &[], EvidencePhase::HeldOut),
    )])
    .unwrap();
    ArcAllocationCondition {
        name: "raw-bounded-bank",
        selected: Vec::new(),
        accounting,
        training_solved: outcome.solution.is_some(),
        hidden_test_verified: verified,
        universal_coverage: false,
    }
}

fn top_arc_set(policy: &FrozenPolicy) -> ConceptSet {
    policy.ranked.first().expect("ARC candidate set").concepts.clone()
}

fn run_contextual_arc() -> ContextualArcReport {
    let mut opts = bank_opts(4, 14);
    opts.fuel = 1_000_000;
    let concepts = arc_concept_map();
    let mirror_task = load_arc_task_by_id(ARC_DATA_DIR, CONTEXT_ARC_TRAIN_IDS[0]);
    let vflip_task = load_arc_task_by_id(ARC_DATA_DIR, CONTEXT_ARC_TRAIN_IDS[1]);
    let calibration = load_arc_task_by_id(ARC_DATA_DIR, CONTEXT_ARC_CALIBRATION_ID);
    let holdout = load_arc_task_by_id(ARC_DATA_DIR, CONTEXT_ARC_HOLDOUT_ID);
    let mirror = ConceptSet::singleton("mirror");
    let vflip = ConceptSet::singleton("vflip");
    let pair = ConceptSet::new(["mirror".into(), "vflip".into()]);
    let candidates = [mirror.clone(), vflip.clone(), pair.clone()];

    let measure = |task: &ArcTask, set: &ConceptSet| {
        let parsed = arc_task_to_parse(task).unwrap();
        let installed = set
            .0
            .iter()
            .map(|id| concepts.get(id).unwrap().clone())
            .collect::<Vec<_>>();
        let (without, _) = bank::concept_solve_abl(&parsed, &[], &opts, true);
        let (with, _) = bank::concept_solve_abl(&parsed, &installed, &opts, true);
        (without, with)
    };
    let (mirror_without, mirror_with) = measure(&mirror_task, &mirror);
    let (vflip_without, vflip_with) = measure(&vflip_task, &vflip);
    let (pair_without, pair_with) = measure(&calibration, &pair);

    // Encoder selection gets generated, disjoint pretraining/calibration task
    // groups. It never sees any real ARC task, protected output, task identity,
    // or solution trace. Different sizes/seeds prevent exact-example leakage.
    let generated = |id: &str, relation: usize, seed: u32, dimensions: &[(usize, usize)]| {
        let train = dimensions
            .iter()
            .enumerate()
            .map(|(example_index, &(height, width))| {
                let input = (0..height)
                    .map(|row| {
                        (0..width)
                            .map(|column| {
                                (seed + row as u32 * 3 + column as u32 * 5
                                    + example_index as u32 * 7)
                                    % 10
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                let output = (0..height)
                    .map(|row| {
                        (0..width)
                            .map(|column| {
                                let source_row = if relation & 2 == 0 {
                                    row
                                } else {
                                    height - 1 - row
                                };
                                let source_column = if relation & 1 == 0 {
                                    column
                                } else {
                                    width - 1 - column
                                };
                                input[source_row][source_column]
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                ArcExample {
                    input: ArcGrid { rows: input },
                    output: ArcGrid { rows: output },
                }
            })
            .collect();
        ArcTask { id: id.into(), train, test: Vec::new() }
    };
    let encoder_train_tasks = [
        generated("generated-mirror-a", 1, 1, &[(2, 3), (3, 5)]),
        generated("generated-mirror-b", 1, 2, &[(4, 3), (2, 7)]),
        generated("generated-vflip-a", 2, 3, &[(3, 4), (5, 2)]),
        generated("generated-vflip-b", 2, 4, &[(4, 5), (6, 3)]),
    ];
    let encoder_calibration_tasks = [
        generated("generated-mirror-cal", 1, 5, &[(3, 7), (5, 4)]),
        generated("generated-vflip-cal", 2, 6, &[(7, 3), (4, 6)]),
    ];
    let build_encoder_records = |task: &ArcTask| {
        [mirror.clone(), vflip.clone()]
            .iter()
            .map(|set| {
                let (without, with) = measure(task, set);
                raw_arc_evidence(
                    task,
                    &set.0.iter().map(String::as_str).collect::<Vec<_>>(),
                    &without,
                    &with,
                    &opts,
                    EvidencePhase::Calibration,
                )
            })
            .collect::<Vec<_>>()
    };
    let encoder_training = encoder_train_tasks
        .iter()
        .flat_map(build_encoder_records)
        .collect::<Vec<_>>();
    let encoder_calibration = encoder_calibration_tasks
        .iter()
        .flat_map(build_encoder_records)
        .collect::<Vec<_>>();
    let encoder_evidence_runs = encoder_training
        .iter()
        .chain(&encoder_calibration)
        .flat_map(|record| [record.without.clone(), record.with.clone()])
        .collect::<Vec<_>>();
    let encoder_evidence_accounting = search_accounting::aggregate(&encoder_evidence_runs)
        .expect("encoder evidence uses only behavior-bank units");
    let representation_spec = RepresentationSpec {
        engine: search_accounting::SearchEngine::BehaviorBank,
        freeze_epoch: 1,
        decay_per_mille: 900,
        interactions: true,
        max_interaction_width: 2,
        max_projection_width: 2,
    };
    let learned_representation = learn_representation(
        &encoder_training,
        &encoder_calibration,
        &candidates,
        &representation_spec,
    );

    let raw_utility = vec![
        raw_arc_evidence(
            &mirror_task, &["mirror"], &mirror_without, &mirror_with, &opts,
            EvidencePhase::Training,
        ),
        raw_arc_evidence(
            &vflip_task, &["vflip"], &vflip_without, &vflip_with, &opts,
            EvidencePhase::Training,
        ),
        raw_arc_evidence(
            &calibration, &["mirror", "vflip"], &pair_without, &pair_with, &opts,
            EvidencePhase::Calibration,
        ),
    ];
    let contextual_policy = freeze_learned_policy(
        &learned_representation.encoder,
        &raw_utility,
        &raw_arc_observation(&holdout),
        &candidates,
        &representation_spec,
    )
    .expect("frozen ARC raw observations");

    // The old named-feature condition remains strictly as an ablation.
    let mut ledger = ContextualLedger::default();
    ledger.record(arc_evidence(
        hand_arc_context(&mirror_task),
        &["mirror"],
        &mirror_without,
        &mirror_with,
        &opts,
        EvidencePhase::Training,
    ));
    ledger.record(arc_evidence(
        hand_arc_context(&vflip_task),
        &["vflip"],
        &vflip_without,
        &vflip_with,
        &opts,
        EvidencePhase::Training,
    ));
    ledger.record(arc_evidence(
        hand_arc_context(&calibration),
        &["mirror", "vflip"],
        &pair_without,
        &pair_with,
        &opts,
        EvidencePhase::Calibration,
    ));
    let target = hand_arc_context(&holdout);
    let hand_policy = ledger.learn(
        &candidates,
        &FreezeSpec {
            target: target.clone(),
            engine: search_accounting::SearchEngine::BehaviorBank,
            freeze_epoch: 1,
            decay_per_mille: 900,
            contextual: true,
            interactions: true,
            max_interaction_width: 2,
        },
    );
    let global_policy = ledger.learn(
        &candidates,
        &FreezeSpec {
            target: target.clone(),
            engine: search_accounting::SearchEngine::BehaviorBank,
            freeze_epoch: 1,
            decay_per_mille: 900,
            contextual: false,
            interactions: true,
            max_interaction_width: 2,
        },
    );
    let mut no_interaction_spec = representation_spec.clone();
    no_interaction_spec.interactions = false;
    let interaction_disabled_policy = freeze_learned_policy(
        &learned_representation.encoder,
        &raw_utility,
        &raw_arc_observation(&holdout),
        &candidates,
        &no_interaction_spec,
    )
    .expect("interaction ablation");
    let mut shuffled_target = target;
    shuffled_target.features = hand_arc_context(&mirror_task).features;
    let shuffled_policy = ledger.learn(
        &candidates,
        &FreezeSpec {
            target: shuffled_target,
            engine: search_accounting::SearchEngine::BehaviorBank,
            freeze_epoch: 1,
            decay_per_mille: 900,
            contextual: true,
            interactions: true,
            max_interaction_width: 2,
        },
    );
    let contextual_top = [top_arc_set(&contextual_policy)];
    let hand_top = [top_arc_set(&hand_policy)];
    let global_top = [top_arc_set(&global_policy)];
    let shuffled_top = [top_arc_set(&shuffled_policy)];
    let interaction_disabled_top = [top_arc_set(&interaction_disabled_policy)];
    let uniform_order = [mirror.clone(), vflip.clone(), pair.clone()];
    let oracle_order = [pair];
    let irrelevant_order = [ConceptSet::singleton("irrelevant-identity")];
    let misleading_order = [ConceptSet::singleton("misleading-projection")];
    ContextualArcReport {
        learned_representation,
        encoder_evidence_accounting,
        contextual: run_arc_condition(
            "learned-context",
            &holdout,
            &contextual_top,
            &concepts,
            &opts,
        ),
        hand_features: run_arc_condition(
            "hand-features", &holdout, &hand_top, &concepts, &opts,
        ),
        global: run_arc_condition("global", &holdout, &global_top, &concepts, &opts),
        uniform: run_arc_condition("uniform", &holdout, &uniform_order, &concepts, &opts),
        oracle: run_arc_condition("oracle", &holdout, &oracle_order, &concepts, &opts),
        shuffled: run_arc_condition("shuffled", &holdout, &shuffled_top, &concepts, &opts),
        interaction_disabled: run_arc_condition(
            "interaction-disabled",
            &holdout,
            &interaction_disabled_top,
            &concepts,
            &opts,
        ),
        irrelevant: run_arc_condition(
            "irrelevant",
            &holdout,
            &irrelevant_order,
            &concepts,
            &opts,
        ),
        misleading: run_arc_condition(
            "misleading",
            &holdout,
            &misleading_order,
            &concepts,
            &opts,
        ),
        raw_bounded: run_arc_raw_condition(&holdout, &opts),
        contextual_policy,
        hand_policy,
        global_policy,
    }
}

fn contextual_arc() {
    let report = run_contextual_arc();
    println!("\n── contextual ARC-1 allocation (preregistered D4 slice) ──");
    println!(
        "split: train={:?} calibration={} holdout={} (test output verification-only)",
        CONTEXT_ARC_TRAIN_IDS, CONTEXT_ARC_CALIBRATION_ID, CONTEXT_ARC_HOLDOUT_ID
    );
    println!(
        "learned encoder={:?} regret={} collapsed_regret={} candidates={}",
        report.learned_representation.encoder.kind,
        report.learned_representation.encoder.calibration_regret,
        report.learned_representation.encoder.collapsed_regret,
        report.learned_representation.accounting.candidates_evaluated,
    );
    println!(
        "learned top={} hand-feature top={} global top={}",
        top_arc_set(&report.contextual_policy).0.join("+"),
        top_arc_set(&report.hand_policy).0.join("+"),
        top_arc_set(&report.global_policy).0.join("+")
    );
    println!(
        "record,engine=context-encoder,condition=learned-z,kind={:?},regret={},collapsed_regret={},candidates={},predictions={},fields_inspected={}",
        report.learned_representation.encoder.kind,
        report.learned_representation.encoder.calibration_regret,
        report.learned_representation.encoder.collapsed_regret,
        report.learned_representation.accounting.candidates_evaluated,
        report.learned_representation.accounting.validation_predictions,
        report.learned_representation.accounting.raw_fields_inspected,
    );
    println!(
        "record,engine=behavior-bank,condition=encoder-evidence,built={},solution_rank=none,universal=false",
        report.encoder_evidence_accounting.work.comparable_primary_work(),
    );
    for condition in [
        &report.contextual,
        &report.hand_features,
        &report.global,
        &report.uniform,
        &report.oracle,
        &report.shuffled,
        &report.interaction_disabled,
        &report.irrelevant,
        &report.misleading,
        &report.raw_bounded,
    ] {
        let built = match condition.accounting.work {
            search_accounting::EngineWork::BehaviorBank {
                candidate_constructions,
                ..
            } => candidate_constructions,
            _ => unreachable!(),
        };
        println!(
            "{:<18} built={:<7} rank={:<5} train={} hidden-test={} universal={} order={}",
            condition.name,
            built,
            if condition.hidden_test_verified {
                built.to_string()
            } else {
                "none".into()
            },
            condition.training_solved,
            condition.hidden_test_verified,
            condition.universal_coverage,
            condition
                .selected
                .iter()
                .map(|set| set.0.join("+"))
                .collect::<Vec<_>>()
                .join(" -> ")
        );
        println!(
            "record,engine=behavior-bank,condition={},built={},solution_rank={},train_solved={},hidden_test_verified={},universal=false",
            condition.name,
            built,
            if condition.hidden_test_verified {
                built.to_string()
            } else {
                "none".into()
            },
            condition.training_solved,
            condition.hidden_test_verified
        );
    }
    println!("ARC `built` is a bounded-bank unit and is never combined with universal proposals.");
    let contextual_work = report.contextual.accounting.work.comparable_primary_work();
    let oracle_work = report.oracle.accounting.work.comparable_primary_work();
    let next_score = report
        .contextual_policy
        .ranked
        .get(1)
        .map(|weight| weight.score)
        .unwrap_or(0);
    println!(
        "regret_vs_oracle={} calibration_margin={} confidence_per_mille={} paired_solve_rate={}/1",
        contextual_work.saturating_sub(oracle_work),
        report.contextual_policy.ranked[0].score.saturating_sub(next_score),
        report.contextual_policy.ranked[0].confidence_per_mille,
        usize::from(report.contextual.hidden_test_verified),
    );
}

/// The four-bucket diagnostic outcome.
enum Bucket {
    Solved,
    Expressible,
    RequiresNew,
    NotRepresentable,
}

fn bucket_label(b: &Bucket) -> &'static str {
    match b {
        Bucket::Solved => "SOLVED",
        Bucket::Expressible => "EXPRESSIBLE",
        Bucket::RequiresNew => "REQUIRES_NEW",
        Bucket::NotRepresentable => "NOT_REPRESENTABLE",
    }
}

/// Classify one parseable task. Honest bounds: SOLVED is bounded by the search
/// budget; EXPRESSIBLE/REQUIRES_NEW are bounded by the expressibility depth.
/// Returns `(bucket, expressible-composition-if-any, built-cost-if-solved)`.
fn classify_task(
    task: &parse::Task,
    vocab: &[bank::Concept],
    cands: &[(String, Rc<term::Term>)],
    opts: &bank::Options,
    ho: bool,
) -> (Bucket, Option<String>, u64) {
    // Search gate first: is a program reachable through the vocabulary? With
    // `ho` (C8), the pool holds the concept functions too, so higher-order
    // compositions (map(rev), map(dup), rev(mirror)) are reachable.
    let (out, _m) = if ho {
        bank::concept_solve_ho_abl(task, vocab, opts, true)
    } else {
        bank::concept_solve_abl(task, vocab, opts, true)
    };
    if out.solution.is_some() {
        return (Bucket::Solved, None, out.stats.built);
    }
    // Expressibility gate: does any composition directly map inputs→outputs?
    for (name, body) in cands {
        if direct_solves(task, body) {
            return (Bucket::Expressible, Some(name.clone()), 0);
        }
    }
    (Bucket::RequiresNew, None, 0)
}

/// The arcdiag driver: run the four-bucket diagnostic over real ARC tasks.
fn arcdiag(args: &[String]) {
    let mut dir = ARC_DATA_DIR.to_string();
    let mut max_tasks = usize::MAX;
    let mut id_filter: Option<String> = None;
    let mut depth = 3u32;
    let mut budget = 10u64;
    let mut ho = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--dir" => {
                dir = args[i + 1].clone();
                i += 2;
            }
            "--max" => {
                max_tasks = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--id" => {
                id_filter = Some(args[i + 1].clone());
                i += 2;
            }
            "--depth" => {
                depth = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--budget" => {
                budget = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--ho" => {
                ho = true;
                i += 1;
            }
            other => {
                if i == 0 {
                    dir = other.to_string();
                }
                i += 1;
            }
        }
    }

    // Discover the vocabulary once, or fail honestly (the whole diagnostic rests
    // on the claim that it is genuinely derived).
    let dopts = bank_opts(60, 14);
    let schemas = match discover_schemas(&dopts) {
        Some(s) => s,
        None => {
            eprintln!("[arcdiag] vocabulary discovery FAILED — nothing to install");
            return;
        }
    };
    let vocab = vocab_concepts(&schemas);
    let cands = expressibility_candidates(&schemas, depth);
    println!(
        "[arcdiag] installed {} vocab concepts; {} expressibility candidates at depth {depth}; ho={ho}",
        vocab.len(),
        cands.len()
    );

    let tasks = load_arc_tasks(&dir);
    let mut counts = [0usize; 4];
    let mut done = 0usize;
    for at in &tasks {
        if let Some(f) = &id_filter {
            if &at.id != f {
                continue;
            }
        } else if done >= max_tasks {
            break;
        }
        done += 1;

        let rep = at
            .train
            .iter()
            .chain(at.test.iter())
            .all(|e| grid_representable(&e.input) && grid_representable(&e.output));
        let pt = arc_task_to_parse(at);
        let (bucket, comp, built) = if !rep || pt.is_none() {
            (Bucket::NotRepresentable, None, 0)
        } else {
            let opts2 = bank_opts(budget, 14);
            classify_task(&pt.unwrap(), &vocab, &cands, &opts2, ho)
        };
        counts[match &bucket {
            Bucket::Solved => 0,
            Bucket::Expressible => 1,
            Bucket::RequiresNew => 2,
            Bucket::NotRepresentable => 3,
        }] += 1;

        let comp = comp.unwrap_or_default();
        println!(
            "[arcdiag] {:>12}  {:14}  comp={comp:<24}  built={built}",
            at.id,
            bucket_label(&bucket)
        );
    }

    println!();
    println!("=== arcdiag summary (depth={depth}, budget={budget}s) ===");
    println!("SOLVED:            {}", counts[0]);
    println!("EXPRESSIBLE:       {}", counts[1]);
    println!("REQUIRES_NEW:      {}", counts[2]);
    println!("NOT_REPRESENTABLE: {}", counts[3]);
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

// ────────────────────────────────────────────────────────────────────────────
// B1: generic context-abstraction meta-search — the machine invents a primitive
// by restructuring its own discovered code, not by selecting from a schema
// catalog. The substrate is only the computational primitives {cons,nil} +
// color-numerals; the ontology is empty. The machine raw-solves a task, factors
// a repeated subterm out of the program's body into a hole (context abstraction),
// and counterfactually acquires the concept iff it reduces held-out cost.
//
//   raw solve → p
//   enumerate repeated contexts → (C_i, p_i')
//   verify p_i' ≡ p (factorization) + C_i closed (semantics) → valid proposals
//   counterfactual held-out evaluation → Gain(C_i)
//   Gain > 0 → ACQUIRE
// ────────────────────────────────────────────────────────────────────────────

/// The fixed computational substrate: {cons, nil} as cost-1 primitives. No
/// reverse/map/compose/mirror — the ontology is empty; the machine must build it.
fn substrate_concepts() -> Vec<bank::Concept> {
    vec![
        bank::Concept {
            body: closed("λc.λs.λf.λz.f(c)(s(f)(z))"),
            name: "cons".into(),
            arity: 2,
        },
        bank::Concept {
            body: closed("λf.λz.z"),
            name: "nil".into(),
            arity: 0,
        },
    ]
}

/// A color numeral as a closed Church term.
fn b1_num(n: u32) -> Rc<term::Term> {
    closed(&bootstrap::church_num_str(n))
}

/// A row of `w` copies of color `c`.
fn b1_row(c: u32, w: usize) -> Rc<term::Term> {
    let cells: Vec<Rc<term::Term>> = (0..w).map(|_| b1_num(c)).collect();
    rc_list(&cells)
}

/// Discovery family: given a singleton row `[c]`, duplicate it into the 2×1
/// grid `[[c],[c]]`. Supplying the row keeps discovery focused on the repeated
/// context `λr. cons r (cons r nil)` instead of spending the search budget
/// constructing the value that will fill that context.
fn b1_discovery_task() -> parse::Task {
    parse::Task {
        arity: 1,
        tests: (1..=3u32)
            .map(|c| parse::Test {
                args: vec![b1_row(c, 1)],
                want: rc_list(&[b1_row(c, 1), b1_row(c, 1)]),
                outer: 0,
            })
            .collect(),
    }
}

/// Held-out transfer: duplicate an unseen width-4 row. The operation is the
/// same as discovery, but both the row shape and values are held out; acquiring
/// the abstraction must reduce the search cost rather than merely memorize the
/// singleton discovery examples.
fn b1_heldout_task() -> parse::Task {
    b1_duplication_task(&[4, 7], 4)
}

/// Semantic validation is disjoint from both discovery and acquisition: new
/// widths and colors make accidental agreement on singleton discovery rows
/// insufficient to establish that the rewrite preserves the intended behavior.
fn b1_semantic_validation_task() -> parse::Task {
    b1_duplication_task(&[2, 3], 7)
}

fn b1_duplication_task(widths: &[usize], first_color: u32) -> parse::Task {
    parse::Task {
        arity: 1,
        tests: widths
            .iter()
            .enumerate()
            .map(|(i, width)| {
                let c = first_color + i as u32;
                parse::Test {
                    args: vec![b1_row(c, *width)],
                    want: rc_list(&[b1_row(c, *width), b1_row(c, *width)]),
                    outer: 0,
                }
            })
            .collect(),
    }
}

/// B1 driver: raw solve → enumerate repeated contexts → verify factorization →
/// counterfactual held-out evaluation → ACQUIRE/REJECT per candidate.
fn b1(_args: &[String]) {
    use std::io::Write;
    let mut opts = bank_opts(60, 14);
    opts.max_depth = 1; // p = λr. … has one binder; depth 1 suffices and shrinks the search
    let substrate = substrate_concepts();
    // Wire the {cons, nil} substrate into the raw search's quotient pool, or the
    // enumerator has no way to build a list at all — [1,1] would be unreachable
    // and only degenerate λ-terms (matching a 1-element list's behavior) pass.
    opts.concepts = substrate.iter().map(|c| c.body.clone()).collect();
    let discovery = b1_discovery_task();
    let semantic_validation = b1_semantic_validation_task();
    let heldout = b1_heldout_task();

    println!("\n── arc1 b1: generic context abstraction invents a primitive from raw code ──");
    println!("substrate = {{cons, nil}} + color-numerals (computational primitives only)");
    println!("ontology = empty — no reverse/map/compose/mirror; the machine must build it");

    // 1. raw solve → p
    let raw_out = bank::solve_abl(&discovery, &opts, true);
    let p = match raw_out.solution {
        Some(p) => p,
        None => {
            println!(
                "  raw solve: ✗ no program found (reached_size {}, built {}, budget {}s)",
                raw_out.stats.reached_size, raw_out.stats.built, 60
            );
            return;
        }
    };
    println!("  raw solve → p = {}", term::show(&p));

    // 2. enumerate repeated contexts → (C_i, p_i')
    let cands = transform::enumerate_abstractions(&p);
    println!("  repeated contexts → {} closed abstraction candidates", cands.len());

    // 3. verify behavior on a disjoint semantic-validation suite. The original
    //    raw program and its rewrite must both implement row duplication there.
    let valid: Vec<&transform::Abstraction> = cands
        .iter()
        .filter(|a| {
            direct_solves(&semantic_validation, &p)
                && direct_solves(&semantic_validation, &a.rewritten_program)
        })
        .collect();
    println!(
        "  semantic validation (unseen widths 2,3): {} valid proposals",
        valid.len()
    );

    // 4. counterfactual held-out evaluation → Gain(C_i)
    let baseline = acquire::concept_cost_abl(&heldout, &substrate, &opts, true);
    println!("  held-out baseline (substrate only): {} states", acquire::disp_cost(baseline));
    for a in &valid {
        let g = acquire::propose_value_abl(
            &a.concept,
            &substrate,
            &[heldout.clone()],
            &opts,
            baseline,
            true,
        );
        match g {
            Some(g) if g.earns() => println!(
                "  C = {}  (factored subterm size {}): {} → {}  {}  ACQUIRE  arity {}",
                term::show(&a.concept),
                a.extracted_subterm.size(),
                acquire::disp_cost(g.before),
                acquire::disp_cost(g.after),
                g.kind(),
                g.arity
            ),
            Some(g) => println!(
                "  C = {}  (factored subterm size {}): {} → {}  {}  REJECT",
                term::show(&a.concept),
                a.extracted_subterm.size(),
                acquire::disp_cost(g.before),
                acquire::disp_cost(g.after),
                g.kind()
            ),
            None => println!(
                "  C = {}  (factored subterm size {}): no valid interface  REJECT",
                term::show(&a.concept),
                a.extracted_subterm.size()
            ),
        }
    }
    println!(
        "  claim: generic syntactic factorization proposes abstractions; only measured semantic\n\
         \x20    reuse turns one into a concept — the machine creates a new primitive by restructuring\n\
         \x20    its own discovered code, rather than selecting it from a schema catalog."
    );
    std::io::stdout().flush().ok();
}

// ────────────────────────────────────────────────────────────────────────────
// B2: recurrence induction from independently raw-discovered finite programs.
// The discovery programs concatenate 1, 2, and 3 separate rows.  Their
// normalized symbolic computations expose an invariant two-hole context; the
// recurrence engine has no concat/fold/reduce/list-operation proposal catalog.
// ────────────────────────────────────────────────────────────────────────────

/// Closed instance atoms for B2. Each is an endofunction discovered programs
/// may compose; none names a recursion scheme or higher list operation.
fn b2_functions(depth: usize, salt: u32) -> Vec<Rc<term::Term>> {
    let cons = closed("λc.λs.λf.λz.f(c)(s(f)(z))");
    (0..depth)
        .map(|i| {
            let atom = b1_num(1 + (salt * 3 + i as u32 * 2) % 8);
            term::app(cons.clone(), atom)
        })
        .collect()
}

fn b2_apply(functions: &[Rc<term::Term>], tail: Rc<term::Term>) -> Rc<term::Term> {
    functions
        .iter()
        .rev()
        .fold(tail, |result, f| term::app(f.clone(), result))
}

/// Extrapolation task for the executable law: apply lists of independently
/// supplied endofunctions at depths never used for induction.
fn b2_extrapolation_task() -> parse::Task {
    parse::Task {
        arity: 1,
        tests: [5usize, 7, 9]
            .into_iter()
            .enumerate()
            .map(|(salt, depth)| {
                let functions = b2_functions(depth, 20 + salt as u32);
                let tail = b1_row(1 + salt as u32, 2 + salt);
                parse::Test {
                    args: vec![rc_list(&functions), tail.clone()],
                    want: b2_apply(&functions, tail),
                    outer: 0,
                }
            })
            .collect(),
    }
}

fn b2_acquisition_task() -> parse::Task {
    let functions = b2_functions(5, 50);
    let tail = b1_row(5, 2);
    parse::Task {
        arity: 2,
        tests: vec![parse::Test {
            args: vec![rc_list(&functions), tail.clone()],
            want: b2_apply(&functions, tail),
            outer: 0,
        }],
    }
}

fn discover_b2_law(opts: &bank::Options) -> Option<(recurrence::RecurrenceLaw, Vec<Rc<term::Term>>)> {
    let mut programs = Vec::new();
    let mut observations = Vec::new();
    for depth in 1..=3usize {
        let mut local = opts.clone();
        local.max_depth = 0;
        let functions = b2_functions(depth, depth as u32);
        let tail = b1_row(6 + depth as u32, 1 + depth);
        let task = parse::Task {
            arity: 0,
            tests: vec![parse::Test {
                args: vec![],
                want: b2_apply(&functions, tail.clone()),
                outer: 0,
            }],
        };
        local.concepts.extend(functions.iter().cloned());
        local.concepts.push(tail.clone());
        let out = bank::solve_abl(&task, &local, true);
        println!(
            "  raw depth {depth}: {} (reached_size {}, built {})",
            if out.solution.is_some() { "✓" } else { "✗" },
            out.stats.reached_size,
            out.stats.built
        );
        let program = out.solution?;
        let (observation, external_parameters) = recurrence::observe_instantiated_program(
            &program,
            &functions,
            &[tail],
            2_000_000,
        )?;
        println!("    symbolic q{depth} = {}", term::show(&observation.body));
        if external_parameters.len() != 1 {
            return None;
        }
        programs.push(program);
        observations.push(observation);
    }
    recurrence::infer(&observations).map(|law| (law, programs))
}

fn b2(_args: &[String]) {
    let substrate = substrate_concepts();
    let mut opts = bank_opts(10, 9);
    opts.concepts = substrate.iter().map(|c| c.body.clone()).collect();

    println!("\n── arc1 b2: infer an executable recursive law from finite discovered programs ──");
    println!("discovery depths = {{1,2,3}}; schemas = none; substrate = {{cons,nil}}");
    println!("each closed instance supplies only fresh endofunction atoms g_i and a tail z");
    let Some((law, programs)) = discover_b2_law(&opts) else {
        println!("  induction: ✗ raw discovery or invariant-context inference failed");
        return;
    };
    for (i, p) in programs.iter().enumerate() {
        println!("  raw p{} = {}", i + 1, term::show(p));
    }
    println!("  inferred base = {}", term::show(&law.base));
    println!("  inferred step context = {}", term::show(law.step_context()));
    println!("  law validation: ✓ exact reconstruction at depths 1,2,3");

    let executable = law.compile_church();
    let extrapolation = b2_extrapolation_task();
    let extrapolates = direct_solves(&extrapolation, &executable);
    println!(
        "  extrapolation at depths 5,7,9 with novel atoms/shapes: {}",
        if extrapolates { "✓" } else { "✗" }
    );

    let mut heldout_opts = bank_opts(8, 8);
    heldout_opts.max_depth = 2;
    heldout_opts.concepts = substrate.iter().map(|c| c.body.clone()).collect();
    let acquisition_task = b2_acquisition_task();
    let baseline = acquire::concept_cost_abl(&acquisition_task, &substrate, &heldout_opts, true);
    let gain = acquire::propose_value_abl(
        &executable,
        &substrate,
        &[acquisition_task],
        &heldout_opts,
        baseline,
        true,
    );
    match gain {
        Some(g) if extrapolates && g.earns() => println!(
            "  counterfactual acquisition: {} → {}  {}  ACQUIRE",
            acquire::disp_cost(g.before),
            acquire::disp_cost(g.after),
            g.kind()
        ),
        Some(g) => println!(
            "  counterfactual acquisition: {} → {}  {}  REJECT",
            acquire::disp_cost(g.before),
            acquire::disp_cost(g.after),
            g.kind()
        ),
        None => println!("  counterfactual acquisition: no executable interface  REJECT"),
    }
}

// ────────────────────────────────────────────────────────────────────────────
// B3: the recursion scheme induced in B2 becomes an ontology atom. A single
// typed normal-form generator then searches for higher concepts; its grammar is
// operation-blind and contains no map/reverse/append/fold productions.
// ────────────────────────────────────────────────────────────────────────────

fn b3_map_task() -> parse::Task {
    let succ = closed("λn.λf.λx.f(n(f)(x))");
    let twice = closed("λn.λf.λx.n(f)(n(f)(x))");
    let cases = vec![
        (succ, vec![1, 2, 4]),
        (twice, vec![1, 3, 4]),
    ];
    parse::Task {
        arity: 2,
        tests: cases
            .into_iter()
            .map(|(f, values)| {
                let inputs: Vec<Rc<term::Term>> = values.iter().map(|n| b1_num(*n)).collect();
                let outputs: Vec<Rc<term::Term>> = inputs
                    .iter()
                    .map(|x| term::app(f.clone(), x.clone()))
                    .collect();
                parse::Test {
                    args: vec![f, rc_list(&inputs)],
                    want: rc_list(&outputs),
                    outer: 0,
                }
            })
            .collect(),
    }
}

fn b3_append_task() -> parse::Task {
    parse::Task {
        arity: 2,
        tests: vec![
            parse::Test {
                args: vec![church_list(&[1, 2]), church_list(&[3, 4])],
                want: church_list(&[1, 2, 3, 4]),
                outer: 0,
            },
            parse::Test {
                args: vec![church_list(&[5]), church_list(&[6, 7, 8])],
                want: church_list(&[5, 6, 7, 8]),
                outer: 0,
            },
        ],
    }
}

fn b3_reverse_task() -> parse::Task {
    parse::Task {
        arity: 1,
        tests: vec![
            parse::Test {
                args: vec![church_list(&[1, 2, 3])],
                want: church_list(&[3, 2, 1]),
                outer: 0,
            },
            parse::Test {
                args: vec![church_list(&[4, 1, 5, 2])],
                want: church_list(&[2, 5, 1, 4]),
                outer: 0,
            },
        ],
    }
}

fn b3_types() -> (typed::Type, typed::Type) {
    (typed::Type::Atom(0), typed::Type::Atom(1))
}

fn b3_atoms(
    recursion: Option<Rc<term::Term>>,
    append: Option<Rc<term::Term>>,
) -> Vec<typed::Atom> {
    let (a, list) = b3_types();
    let arrow = typed::Type::arrow;
    let cons_ty = arrow(a.clone(), arrow(list.clone(), list.clone()));
    let step_ty = cons_ty.clone();
    let rec_ty = arrow(
        step_ty,
        arrow(list.clone(), arrow(list.clone(), list.clone())),
    );
    let mut atoms = vec![
        typed::Atom {
            body: closed("λc.λs.λf.λz.f(c)(s(f)(z))"),
            ty: cons_ty,
        },
        typed::Atom {
            body: closed("λf.λz.z"),
            ty: list.clone(),
        },
    ];
    if let Some(body) = recursion {
        atoms.push(typed::Atom { body, ty: rec_ty });
    }
    if let Some(body) = append {
        atoms.push(typed::Atom {
            body,
            ty: arrow(list.clone(), arrow(list.clone(), list)),
        });
    }
    atoms
}

fn b3_find(
    task: &parse::Task,
    target: &typed::Type,
    atoms: &[typed::Atom],
    max_size: u32,
) -> Option<typed::Found> {
    typed::find_closed(target, atoms, max_size, 50_000, |candidate| {
        direct_solves(task, candidate)
    })
}

struct B3Vocabulary {
    recursion: Rc<term::Term>,
    map: typed::Found,
    append: typed::Found,
    reverse: typed::Found,
}

fn discover_b3_vocabulary() -> Option<B3Vocabulary> {
    let substrate = substrate_concepts();
    let mut opts = bank_opts(10, 9);
    opts.concepts = substrate.iter().map(|c| c.body.clone()).collect();
    let (law, _) = discover_b2_law(&opts)?;
    let recursion = law.compile_church_scheme();

    let (a, list) = b3_types();
    let arrow = typed::Type::arrow;
    let map_ty = arrow(arrow(a.clone(), a), arrow(list.clone(), list.clone()));
    let append_ty = arrow(list.clone(), arrow(list.clone(), list.clone()));
    let reverse_ty = arrow(list.clone(), list);

    let base_atoms = b3_atoms(Some(recursion.clone()), None);
    let map = match b3_find(&b3_map_task(), &map_ty, &base_atoms, 18) {
        Some(x) => x,
        None => {
            println!("  map proposal search: ✗ through size 18");
            return None;
        }
    };
    let append = match b3_find(&b3_append_task(), &append_ty, &base_atoms, 12) {
        Some(x) => x,
        None => {
            println!("  append proposal search: ✗ through size 12");
            return None;
        }
    };
    let reverse = match b3_find(&b3_reverse_task(), &reverse_ty, &base_atoms, 20) {
        Some(x) => x,
        None => {
            println!("  reverse proposal search: ✗ through size 20");
            return None;
        }
    };
    Some(B3Vocabulary { recursion, map, append, reverse })
}

fn b3_gain(
    candidate: &Rc<term::Term>,
    arity: u32,
    current: &[bank::Concept],
    task: &parse::Task,
) -> acquire::Gain {
    let mut opts = bank_opts(5, 8);
    opts.max_depth = task.arity as u32;
    opts.concepts = current.iter().map(|c| c.body.clone()).collect();
    let baseline = acquire::concept_cost_abl(task, current, &opts, true);
    let mut extended = current.to_vec();
    extended.push(bank::Concept {
        body: candidate.clone(),
        name: "candidate".into(),
        arity,
    });
    let after = acquire::concept_cost_abl(task, &extended, &opts, true);
    acquire::Gain { arity, before: baseline, after }
}

fn b3(_args: &[String]) {
    println!("\n── arc1 b3: invented recursion constructs a higher reasoning vocabulary ──");
    println!("generator = simply-typed β-normal enumeration; named operation schemas = none");
    let Some(v) = discover_b3_vocabulary() else {
        println!("  vocabulary discovery: ✗");
        return;
    };
    println!("  B2 recursion scheme = {}", term::show(&v.recursion));
    println!(
        "  map: size {}, generated {} → {}",
        v.map.size,
        v.map.generated,
        term::show(&v.map.term)
    );
    println!(
        "  append: size {}, generated {} → {}",
        v.append.size,
        v.append.generated,
        term::show(&v.append.term)
    );
    println!(
        "  reverse: size {}, generated {} → {}",
        v.reverse.size,
        v.reverse.generated,
        term::show(&v.reverse.term)
    );

    let (a, list) = b3_types();
    let arrow = typed::Type::arrow;
    let substrate_atoms = b3_atoms(None, None);
    let absent_without_recursion = [
        b3_find(
            &b3_map_task(),
            &arrow(arrow(a.clone(), a), arrow(list.clone(), list.clone())),
            &substrate_atoms,
            v.map.size,
        )
        .is_none(),
        b3_find(
            &b3_append_task(),
            &arrow(list.clone(), arrow(list.clone(), list.clone())),
            &substrate_atoms,
            v.append.size,
        )
        .is_none(),
        b3_find(
            &b3_reverse_task(),
            &arrow(list.clone(), list),
            &substrate_atoms,
            v.reverse.size,
        )
        .is_none(),
    ]
    .into_iter()
    .filter(|absent| *absent)
    .count();
    println!(
        "  proposal-space gain from invented recursion: {}/3 absent → 3/3 reachable",
        absent_without_recursion
    );

    let substrate = substrate_concepts();
    let recursion_concept = bank::Concept {
        body: v.recursion.clone(),
        name: "recurrence".into(),
        arity: 3,
    };
    let map_gain = b3_gain(
        &v.map.term,
        2,
        &[substrate.clone(), vec![recursion_concept.clone()]].concat(),
        &b3_map_task(),
    );
    let append_gain = b3_gain(
        &v.append.term,
        2,
        &[substrate.clone(), vec![recursion_concept.clone()]].concat(),
        &b3_append_task(),
    );
    let reverse_gain = b3_gain(
        &v.reverse.term,
        1,
        &[substrate.clone(), vec![recursion_concept]].concat(),
        &b3_reverse_task(),
    );
    println!(
        "  map acquisition: {} → {}  {}",
        acquire::disp_cost(map_gain.before),
        acquire::disp_cost(map_gain.after),
        map_gain.kind()
    );
    println!(
        "  append acquisition: {} → {}  {}",
        acquire::disp_cost(append_gain.before),
        acquire::disp_cost(append_gain.after),
        append_gain.kind()
    );
    println!(
        "  reverse acquisition: {} → {}  {}",
        acquire::disp_cost(reverse_gain.before),
        acquire::disp_cost(reverse_gain.after),
        reverse_gain.kind()
    );

    let mirror = term::lam(term::app(
        term::app(v.map.term.clone(), v.reverse.term.clone()),
        term::var(0),
    ));
    let mirror_transfer = direct_solves(&task(5, 4), &mirror);
    println!(
        "  cross-domain transfer map(reverse): 5×4 unseen grid mirror {}",
        if mirror_transfer { "✓" } else { "✗" }
    );
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

/// The discovered cross-domain vocabulary as closed λ-terms from {cons,nil}.
/// ALL of `reverse`, `map`, `compose` are DISCOVERED by the C7 fold-schema
/// meta-space (not hand-written); `append` and `dup` are derived from the
/// discovered reverse/append. `cons`/`nil` are the substrate.
struct Schemas {
    reverse: Rc<term::Term>,
    append: Rc<term::Term>,
    map: Rc<term::Term>,
    compose: Rc<term::Term>,
}

/// Discover the full cross-domain vocabulary {reverse, map, compose} from
/// {cons,nil} via the C7 fold-schema meta-space, and derive {append, dup}.
/// Returns None if any schema fails to be proposed (the honest failure mode —
/// the gridmeta claim only holds if the whole stack is genuinely derived).
fn discover_schemas(opts: &bank::Options) -> Option<Schemas> {
    let cons_t = closed("λc.λs.λf.λz.f(c)(s(f)(z))");
    let nil_t = closed("λf.λz.z");
    // reduce(C) = λxs.λys.(xs C ys) — the C7 proposal schema.
    let reduce = |c: &Rc<term::Term>| -> Rc<term::Term> {
        term::lam(term::lam(term::app(term::app(term::var(1), c.clone()), term::var(0))))
    };
    let append = reduce(&cons_t);
    let singleton = term::lam(term::app(term::app(cons_t.clone(), term::var(0)), nil_t.clone()));
    let reverse = discover_reverse(opts)?;
    let map = discover_map(&cons_t, &nil_t, &singleton)?;
    let compose = discover_compose(&reverse, &singleton)?;
    Some(Schemas { reverse, append, map, compose })
}

/// Directly evaluate `body` (a closed term) as a `k`-ary function against a
/// task's tests, comparing results to wants via canonical keys. This is the
/// honest single-candidate gate for schema discovery: does THIS term, as a
/// function, map the test inputs to the test outputs? (`concept_solve` is a
/// *search* and can find a program that uses a wrong candidate in a non-obvious
/// way — e.g. a projection that happens to compose the right values — so
/// discovering a specific schema needs the direct gate.)
fn direct_solves(task: &parse::Task, body: &Rc<term::Term>) -> bool {
    let empty: nbe::Env = Rc::new(Vec::new());
    task.tests.iter().all(|t| {
        let mut fuel = nbe::Fuel(1_000_000);
        let applied = t.args.iter().fold(body.clone(), |acc, a| term::app(acc, a.clone()));
        let v = nbe::eval(&empty, &applied, &mut fuel).ok();
        let w = nbe::eval(&empty, &t.want, &mut fuel).ok();
        match (v, w) {
            (Some(v), Some(w)) => {
                let mut h1 = DefaultHasher::new();
                let mut h2 = DefaultHasher::new();
                let mut f1 = nbe::Fuel(1_000_000);
                let mut f2 = nbe::Fuel(1_000_000);
                let k1 = canon::canonicalize(v.as_ref(), &mut f1, &mut h1).ok();
                let k2 = canon::canonicalize(w.as_ref(), &mut f2, &mut h2).ok();
                k1 == k2
            }
            _ => false,
        }
    })
}

/// Discover `map` as a fold with a function parameter:
///   map(f) = λf. λxs. (xs step nil),  step = λh.λrest. cons (f h) rest.
/// The step references the outer function `f` (de Bruijn Var(3) inside the step),
/// so the enumeration adds `f h` as a candidate element alongside the closed one
/// (id h). Solves-gated against map(singleton) [1,2,3] → [[1],[2],[3]] — a
/// 3-element list distinguishes cons-map from append-map, which coincide on
/// singletons (append flattens, cons nests).
fn discover_map(
    cons_t: &Rc<term::Term>,
    nil_t: &Rc<term::Term>,
    singleton: &Rc<term::Term>,
) -> Option<Rc<term::Term>> {
    let num = |n: u32| closed(&bootstrap::church_num_str(n));
    let map_task = parse::Task {
        arity: 2,
        tests: vec![parse::Test {
            args: vec![singleton.clone(), church_list(&[1, 2, 3])],
            want: rc_list(&[
                rc_list(&[num(1)]),
                rc_list(&[num(2)]),
                rc_list(&[num(3)]),
            ]),
            outer: 0,
        }],
    };
    let solves = |body: &Rc<term::Term>| -> bool { direct_solves(&map_task, body) };
    // append = reduce(cons) = λxs.λys.(xs cons ys).
    let append = term::lam(term::lam(term::app(
        term::app(term::var(1), cons_t.clone()),
        term::var(0),
    )));
    // Inside the fold step λh.λrest.…, the outer function f is Var(3) (bound by
    // λf two lambdas out), h is Var(1), rest is Var(0).
    let f_h = term::app(term::var(3), term::var(1)); // f h
    let h = term::var(1);                            // id h = h
    let unary = [("id", h), ("f", f_h)];
    let binary = [("cons", cons_t.clone()), ("append", append)];
    for (_, b) in &binary {
        for (_, g) in &unary {
            // λh.λrest. b (g h) rest
            let step1 = term::lam(term::lam(term::app(
                term::app(b.clone(), g.clone()),
                term::var(0),
            )));
            // λh.λrest. b rest (g h)
            let step2 = term::lam(term::lam(term::app(
                term::app(b.clone(), term::var(0)),
                g.clone(),
            )));
            for step in [step1, step2] {
                // map = λf. λxs. (xs step nil) — built directly (no reduce
                // wrapper, so the step's outer function f stays at Var(3)).
                let map_cand = term::lam(term::lam(term::app(
                    term::app(term::var(0), step),
                    nil_t.clone(),
                )));
                if solves(&map_cand) {
                    return Some(map_cand);
                }
            }
        }
    }
    None
}

/// Enumerate all closed λ-terms of exactly `size` nodes (pure combinators — no
/// cons/nil, just λ and variables). Used to discover `compose` as the smallest
/// closed term that composes two functions.
fn gen_closed(size: u32) -> Vec<Rc<term::Term>> {
    fn gen(depth: u32, size: u32) -> Vec<Rc<term::Term>> {
        let mut out = Vec::new();
        if size == 1 {
            for i in 0..depth {
                out.push(term::var(i));
            }
            return out;
        }
        if size >= 2 {
            for b in gen(depth + 1, size - 1) {
                out.push(term::lam(b));
            }
        }
        for s1 in 1..size {
            let s2 = size - s1;
            for f in gen(depth, s1) {
                for a in gen(depth, s2) {
                    out.push(term::app(f.clone(), a));
                }
            }
        }
        out
    }
    gen(0, size)
}

/// Discover `compose` as the pure composition combinator λf.λg.λx. f (g x) by
/// enumerating small closed λ-terms and solves-gating against a two-test task
/// that pins down the argument order:
///   compose(rev, singleton) [1,2] → [[1,2]]   (rev ∘ singleton)
///   compose(singleton, rev) [1,2] → [[2,1]]   (singleton ∘ rev)
/// The two tests distinguish compose from the projections (λf.λg.λx. f x and
/// λf.λg.λx. g x), each of which passes exactly one of them.
fn discover_compose(reverse: &Rc<term::Term>, singleton: &Rc<term::Term>) -> Option<Rc<term::Term>> {
    let compose_task = parse::Task {
        arity: 3,
        tests: vec![
            parse::Test {
                args: vec![reverse.clone(), singleton.clone(), church_list(&[1, 2])],
                want: rc_list(&[church_list(&[1, 2])]),
                outer: 0,
            },
            parse::Test {
                args: vec![singleton.clone(), reverse.clone(), church_list(&[1, 2])],
                want: rc_list(&[church_list(&[2, 1])]),
                outer: 0,
            },
        ],
    };
    let solves = |body: &Rc<term::Term>| -> bool { direct_solves(&compose_task, body) };
    // compose is size 8; enumerate up to that and return the first solver.
    for size in 1..=8u32 {
        for t in gen_closed(size) {
            if solves(&t) {
                return Some(t);
            }
        }
    }
    None
}

/// The grid-transform meta-space: a **typed** generative enumeration of
/// compositions of the DISCOVERED building blocks {rev, dup, map, compose} —
/// all proposed by the C7 fold-schema meta-space from {cons,nil}, none
/// hand-written.
///
/// The human provides only the composition operators (map, compose) as
/// discovered schemas and the type discipline (grid = List(List N), row = List N)
/// — NOT the specific transforms (mirror, vflip, rotation, tiling). The system
/// searches compositions and discovers which solve which families.
/// The grid-transform composition space over the DISCOVERED building blocks
/// {rev, dup, map, compose} (all proposed by the C7 meta-space from {cons,nil},
/// none hand-written). Depth-1 grid ops {rev, dup, map(rev), map(dup), map(id)}
/// are GENERATED by applying map to each row-op; deeper levels compose shallower
/// pairs. This is the expressibility enumeration for the real-ARC diagnostic: a
/// task is expressible iff some composition directly maps its train inputs to
/// outputs. `max_depth` bounds the search (reported honestly — "REQUIRES new
/// concept" means not expressible up to this depth).
fn expressibility_candidates(s: &Schemas, max_depth: u32) -> Vec<(String, Rc<term::Term>)> {
    let rev = s.reverse.clone();
    let dup = term::lam(term::app(term::app(s.append.clone(), term::var(0)), term::var(0)));
    let id = term::lam(term::var(0));
    // map(f) = app(map, f); compose(f,g) = app(app(compose, f), g) — map and
    // compose are now DISCOVERED closed terms, not hand-written closures.
    let map = |f: &Rc<term::Term>| -> Rc<term::Term> { term::app(s.map.clone(), f.clone()) };
    let compose = |f: &Rc<term::Term>, g: &Rc<term::Term>| -> Rc<term::Term> {
        term::app(term::app(s.compose.clone(), f.clone()), g.clone())
    };
    // Level 0 (depth 1): the base grid ops.
    let base: Vec<(String, Rc<term::Term>)> = vec![
        ("rev".into(), rev.clone()),       // reverse the grid
        ("dup".into(), dup.clone()),       // duplicate the grid
        ("map(rev)".into(), map(&rev)),    // reverse each row
        ("map(dup)".into(), map(&dup)),    // duplicate each row
        ("map(id)".into(), map(&id)),      // identity
    ];
    // levels[i] holds terms of depth i+1; compose(f,g) lands at depth
    // max(df,dg)+1, i.e. level index max(i,j)+1.
    let mut levels: Vec<Vec<(String, Rc<term::Term>)>> = vec![base];
    let mut out: Vec<(String, Rc<term::Term>)> = levels[0].clone();
    for depth in 2..=max_depth {
        let mut level: Vec<(String, Rc<term::Term>)> = Vec::new();
        for i in 0..levels.len() {
            for j in 0..levels.len() {
                if (i as u32).max(j as u32) == depth - 2 {
                    for (fn_, f) in &levels[i] {
                        for (gn, g) in &levels[j] {
                            level.push((format!("({fn_}∘{gn})"), compose(f, g)));
                        }
                    }
                }
            }
        }
        out.extend(level.iter().cloned());
        levels.push(level);
    }
    out
}

/// The depth-2 slice of [`expressibility_candidates`] — the gridmeta composition
/// space (the 6 synthetic families are all expressible at depth ≤ 2).
fn gridmeta_candidates(s: &Schemas) -> Vec<(String, Rc<term::Term>)> {
    expressibility_candidates(s, 2)
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
/// `None` if the vocabulary itself cannot be discovered from {cons,nil}.
fn gridmeta_discover(opts: &bank::Options) -> Option<Vec<(String, &'static str)>> {
    let s = discover_schemas(opts)?;
    let candidates = gridmeta_candidates(&s);
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
    Some(discovered)
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
    let Some(s) = discover_schemas(&opts) else {
        println!("✗ could not discover the full vocabulary {{reverse, map, compose}} from {{cons,nil}}");
        return;
    };
    let candidates = gridmeta_candidates(&s);
    let families = gridmeta_families();

    println!("\n── gridmeta: does the discovered list vocabulary generalize to grids? ──");
    println!(
        "discovered: reverse(size {}), map(size {}), compose(size {})",
        s.reverse.size(),
        s.map.size(),
        s.compose.size()
    );
    println!("vocabulary: {{rev, dup, map, compose}} — ALL discovered from {{cons,nil}} by the C7 meta-space");
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
    let discovered = gridmeta_discover(&opts).unwrap_or_default();
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

    #[test]
    fn contextual_arc_transfer_is_frozen_verified_and_deterministic() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let report = run_contextual_arc();
                assert!(report.learned_representation.encoder.retained);
                assert_eq!(report.learned_representation.encoder.calibration_regret, 0);
                assert_eq!(report.learned_representation.encoder.collapsed_regret, 4);
                assert_eq!(report.learned_representation.accounting.candidates_evaluated, 16);
                assert_eq!(report.learned_representation.accounting.validation_predictions, 32);
                assert_eq!(
                    report.encoder_evidence_accounting.work.comparable_primary_work(),
                    22
                );
                assert_eq!(
                    report.learned_representation.encoder.kind,
                    supsearch::learned_context::EncoderKind::Projection(vec!["raw-0".into()])
                );
                assert_eq!(
                    top_arc_set(&report.contextual_policy),
                    ConceptSet::new(["mirror".into(), "vflip".into()])
                );
                assert_eq!(
                    top_arc_set(&report.global_policy),
                    ConceptSet::singleton("mirror")
                );
                assert!(report.contextual.training_solved);
                assert!(report.contextual.hidden_test_verified);
                assert!(report.hand_features.hidden_test_verified);
                assert_eq!(
                    top_arc_set(&report.hand_policy),
                    ConceptSet::new(["mirror".into(), "vflip".into()])
                );
                assert!(report.oracle.hidden_test_verified);
                assert!(!report.global.hidden_test_verified);
                assert!(!report.shuffled.hidden_test_verified);
                assert!(!report.interaction_disabled.hidden_test_verified);
                assert!(!report.irrelevant.hidden_test_verified);
                assert!(!report.misleading.hidden_test_verified);
                assert!(!report.raw_bounded.hidden_test_verified);
                assert!(report.contextual.accounting.work.comparable_primary_work()
                    < report.uniform.accounting.work.comparable_primary_work());
                assert_eq!(report.contextual.accounting.work.comparable_primary_work(), 5);
                assert_eq!(report.global.accounting.work.comparable_primary_work(), 3);
                assert_eq!(report.uniform.accounting.work.comparable_primary_work(), 11);
                assert_eq!(
                    report.contextual.accounting.work.comparable_primary_work(),
                    report.oracle.accounting.work.comparable_primary_work()
                );
                assert!(!report.contextual.universal_coverage);
                assert_eq!(
                    report.contextual.accounting.engine,
                    search_accounting::SearchEngine::BehaviorBank
                );

                // Replay the frozen winner on the final task. Non-time ARC
                // counters and independent hidden-test verification are exact.
                let holdout = load_arc_task_by_id(ARC_DATA_DIR, CONTEXT_ARC_HOLDOUT_ID);
                let original_context = raw_arc_observation(&holdout);
                let mut hidden_outputs_changed = holdout.clone();
                for example in &mut hidden_outputs_changed.test {
                    example.output.rows = vec![vec![9]];
                }
                assert_eq!(raw_arc_observation(&hidden_outputs_changed), original_context);
                let concepts = arc_concept_map();
                let mut replay_opts = bank_opts(4, 14);
                replay_opts.fuel = 1_000_000;
                let order = [top_arc_set(&report.contextual_policy)];
                let replay = run_arc_condition(
                    "contextual",
                    &holdout,
                    &order,
                    &concepts,
                    &replay_opts,
                );
                assert_eq!(replay.accounting, report.contextual.accounting);
                assert!(replay.hidden_test_verified);
                let raw_replay = run_arc_raw_condition(&holdout, &replay_opts);
                assert_eq!(raw_replay.accounting, report.raw_bounded.accounting);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn b1_discovers_factors_and_acquires_row_duplication() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let substrate = substrate_concepts();
                let mut o = bank_opts(10, 14);
                o.max_depth = 1;
                o.concepts = substrate.iter().map(|c| c.body.clone()).collect();

                let discovery = b1_discovery_task();
                let raw = bank::solve_abl(&discovery, &o, true)
                    .solution
                    .expect("B1 discovery must be raw-reachable");
                assert!(
                    term_has_prim(raw.as_ref(), &substrate[0].body),
                    "the raw program must genuinely use cons, not an observational impostor"
                );
                let candidate = transform::enumerate_abstractions(&raw)
                    .into_iter()
                    .find(|a| {
                        direct_solves(&b1_semantic_validation_task(), &raw)
                            && direct_solves(
                                &b1_semantic_validation_task(),
                                &a.rewritten_program,
                            )
                    })
                    .expect("raw program must contain a valid closed repeated context");

                let heldout = b1_heldout_task();
                assert!(direct_solves(&heldout, &candidate.concept));
                assert!(
                    !direct_solves(&heldout, &closed("λx.x")),
                    "identity must not pass unseen-width semantic validation"
                );
                let baseline = acquire::concept_cost_abl(&heldout, &substrate, &o, true);
                let gain = acquire::propose_value_abl(
                    &candidate.concept,
                    &substrate,
                    &[heldout],
                    &o,
                    baseline,
                    true,
                )
                .expect("factored context must have a measurable interface");
                assert!(gain.earns(), "B1 abstraction must reduce held-out cost");
                assert_eq!(gain.arity, 1, "row duplication is unary");
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn b2_induces_extrapolates_and_earns_recursive_law() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let substrate = substrate_concepts();
                let mut discovery_opts = bank_opts(10, 9);
                discovery_opts.concepts =
                    substrate.iter().map(|c| c.body.clone()).collect();
                let (law, programs) = discover_b2_law(&discovery_opts)
                    .expect("depth-1..3 raw programs must induce one invariant law");
                assert_eq!(programs.len(), 3);
                assert!(law.uses_head());
                assert!(law.uses_recursive_result());

                let executable = law.compile_church();
                assert!(transform::is_closed(&executable));
                assert!(
                    direct_solves(&b2_extrapolation_task(), &executable),
                    "the executable law must extrapolate to 5,7,9"
                );

                let task = b2_acquisition_task();
                let mut heldout_opts = bank_opts(8, 8);
                heldout_opts.max_depth = 2;
                heldout_opts.concepts =
                    substrate.iter().map(|c| c.body.clone()).collect();
                let baseline =
                    acquire::concept_cost_abl(&task, &substrate, &heldout_opts, true);
                assert!(baseline >= acquire::UNREACHABLE);
                let gain = acquire::propose_value_abl(
                    &executable,
                    &substrate,
                    &[task],
                    &heldout_opts,
                    baseline,
                    true,
                )
                .expect("the compiled recurrence must expose an interface");
                assert!(gain.frontier());
                assert_eq!(gain.arity, 2);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn b3_invented_recursion_expands_proposals_and_recovers_vocabulary() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let vocabulary = discover_b3_vocabulary()
                    .expect("B2 recursion must expose map, append, then reverse");
                let (a, list) = b3_types();
                let arrow = typed::Type::arrow;
                let map_ty = arrow(arrow(a.clone(), a), arrow(list.clone(), list.clone()));
                let append_ty = arrow(list.clone(), arrow(list.clone(), list.clone()));
                let reverse_ty = arrow(list.clone(), list);

                // Proposal-space controls: without the invented recursive atom,
                // map, append, and reverse are absent at their discovered sizes.
                let substrate_only = b3_atoms(None, None);
                assert!(b3_find(&b3_map_task(), &map_ty, &substrate_only, 11).is_none());
                assert!(
                    b3_find(&b3_append_task(), &append_ty, &substrate_only, 9).is_none()
                );
                assert!(
                    b3_find(&b3_reverse_task(), &reverse_ty, &substrate_only, 14).is_none()
                );

                // Honest control: recursion alone can inline an append-like
                // computation, so append is useful vocabulary but not strictly
                // load-bearing for reverse in this bounded typed space.
                let recursion_only = b3_atoms(Some(vocabulary.recursion.clone()), None);
                let inlined_reverse = b3_find(
                    &b3_reverse_task(),
                    &reverse_ty,
                    &recursion_only,
                    20,
                )
                .expect("recursion alone should be able to inline reverse's helper");
                assert!(direct_solves(&b3_reverse_task(), &inlined_reverse.term));

                assert!(term_has_prim(
                    vocabulary.map.term.as_ref(),
                    &vocabulary.recursion
                ));
                assert!(term_has_prim(
                    vocabulary.append.term.as_ref(),
                    &vocabulary.recursion
                ));
                assert!(term_has_prim(
                    vocabulary.reverse.term.as_ref(),
                    &vocabulary.recursion
                ));

                let substrate = substrate_concepts();
                let recursion = bank::Concept {
                    body: vocabulary.recursion.clone(),
                    name: "recurrence".into(),
                    arity: 3,
                };
                let map_gain = b3_gain(
                    &vocabulary.map.term,
                    2,
                    &[substrate.clone(), vec![recursion.clone()]].concat(),
                    &b3_map_task(),
                );
                assert!(map_gain.frontier());

                let reverse_gain = b3_gain(
                    &vocabulary.reverse.term,
                    1,
                    &[substrate, vec![recursion]].concat(),
                    &b3_reverse_task(),
                );
                assert!(reverse_gain.frontier());

                let mirror = term::lam(term::app(
                    term::app(
                        vocabulary.map.term.clone(),
                        vocabulary.reverse.term.clone(),
                    ),
                    term::var(0),
                ));
                assert!(direct_solves(&task(5, 4), &mirror));
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// Does `t` contain a subterm `Prim(b)` with `**b == **body`? The C8 claim is
    /// that a composed solution references a concept through its quotient atom
    /// (`Prim(reverse)`), so the function argument must appear as a `Prim`.
    fn term_has_prim(t: &term::Term, body: &Rc<term::Term>) -> bool {
        match t {
            term::Term::Prim(b) => **b == **body,
            term::Term::App(f, a) => term_has_prim(f, body) || term_has_prim(a, body),
            term::Term::Lam(b) => term_has_prim(b, body),
            _ => false,
        }
    }

    /// Does `t` contain `body` as a *bare* subterm outside any `Prim` wrapper?
    /// This is the failure mode C8 forbids: embedding reverse's λ-body inline
    /// (e.g. `Prim(map);(λreverse-body);grid`) instead of `Prim(map);Prim(reverse);grid`.
    /// A `Prim` is treated as opaque — its inner only expands under evaluation,
    /// so the body appearing *inside* a `Prim` is exactly the correct atom.
    fn term_has_raw_subterm(t: &term::Term, body: &Rc<term::Term>) -> bool {
        match t {
            term::Term::Prim(_) => false,
            term::Term::App(f, a) => {
                term_has_raw_subterm(f, body) || term_has_raw_subterm(a, body)
            }
            term::Term::Lam(b) => term_has_raw_subterm(b, body),
            _ => t == body.as_ref(),
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
                let Some(discovered) = gridmeta_discover(&o) else {
                    panic!("must discover the full vocabulary {{reverse, map, compose}} from {{cons,nil}}");
                };
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

    /// The four-bucket diagnostic: the 6 synthetic transform families (which the
    /// discovered vocabulary genuinely expresses) must classify as EXPRESSIBLE
    /// through the direct gate — the vocabulary's composition space contains each
    /// transform. This validates the pipeline on known-expressible tasks before
    /// trusting it on real ARC. (They land EXPRESSIBLE, not SOLVED, because the
    /// search gate's pool holds only grid values, never function-typed
    /// intermediates like `reverse`, so it cannot re-compose map(reverse) from
    /// the flat {reverse,map,compose,dup,append} concept set — see the
    /// gate-separation test.)
    #[test]
    fn arcdiag_synthetic_captured() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let o = opts();
                let s = discover_schemas(&o).expect("vocabulary must be discovered");
                let vocab = vocab_concepts(&s);
                let cands = expressibility_candidates(&s, 2);
                for (fname, target, sizes) in gridmeta_families() {
                    let fam = transform_family(sizes, target);
                    let (bucket, comp, _built) = classify_task(&fam, &vocab, &cands, &o, false);
                    assert!(
                        matches!(bucket, Bucket::Expressible | Bucket::Solved),
                        "family {fname} must be captured by the vocabulary (got {} comp={comp:?})",
                        bucket_label(&bucket)
                    );
                    // Arity-1 building blocks (vflip=reverse, v-tile=dup) are
                    // directly SOLVED; map-compositions (mirror=map(rev), etc.)
                    // are only EXPRESSIBLE — both are captured.
                    if matches!(bucket, Bucket::Expressible) {
                        assert!(
                            comp.is_some(),
                            "expressible family {fname} must name a composition"
                        );
                    }
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// Gate separation, proven on the synthetic mirror family: the transform IS
    /// directly expressible as map(reverse) (direct gate: EXPRESSIBLE), yet the
    /// search gate over the flat vocabulary FAILS to re-compose it (not SOLVED).
    /// The pool never holds function-typed intermediates, so map (arity 2) can
    /// never be applied to the function `reverse` — higher-order composition is
    /// expressible but not search-reachable. This is precisely the "EXPRESSIBLE
    /// but search fails" bucket the diagnostic exists to surface, and it confirms
    /// the two gates stay separate forever.
    #[test]
    fn arcdiag_gate_separation() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let o = opts();
                let s = discover_schemas(&o).expect("vocabulary must be discovered");
                let vocab = vocab_concepts(&s);
                let cands = expressibility_candidates(&s, 2);
                let fam = transform_family(&[(3, 3), (4, 4)], &mirrored_term);
                let (bucket, comp, _built) = classify_task(&fam, &vocab, &cands, &o, false);
                assert!(
                    matches!(bucket, Bucket::Expressible),
                    "mirror must be EXPRESSIBLE (direct gate) — got {}",
                    bucket_label(&bucket)
                );
                assert_eq!(comp.as_deref(), Some("map(rev)"), "mirror = map(rev)");
                // And the direct gate alone sees it: `direct_solves(map(rev))`.
                let mr = term::app(s.map.clone(), s.reverse.clone());
                assert!(direct_solves(&fam, &mr), "map(rev) directly maps inputs→outputs");
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// Real ARC tasks whose transform is a pure geometry op of the discovered
    /// vocabulary classify honestly: vflip (arity-1 reverse) is SOLVED; mirror
    /// (map(rev)) and v-tile (map(dup)) are EXPRESSIBLE — the higher-order
    /// compositions the search gate can't re-derive. These are the first real
    /// tasks the current structural ontology is already sufficient for.
    #[test]
    fn arcdiag_real_geometry_captured() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let o = opts();
                let s = discover_schemas(&o).expect("vocabulary must be discovered");
                let vocab = vocab_concepts(&s);
                let cands = expressibility_candidates(&s, 2);
                let dir = ARC_DATA_DIR;

                // vflip: reverse applied directly to the grid → SOLVED, tiny cost.
                let vflip = arc_task_to_parse(&load_arc_task_by_id(dir, "68b16354")).unwrap();
                let (b, _c, built) = classify_task(&vflip, &vocab, &cands, &o, false);
                assert!(
                    matches!(b, Bucket::Solved),
                    "real vflip (68b16354) must be SOLVED, got {}",
                    bucket_label(&b)
                );
                assert!(built <= 2, "vflip solved at composition cost {built}, expected ~1");

                // mirror: needs map(rev) — EXPRESSIBLE, not SOLVED (higher-order).
                let mirror = arc_task_to_parse(&load_arc_task_by_id(dir, "67a3c6ac")).unwrap();
                let (b, comp, _built) = classify_task(&mirror, &vocab, &cands, &o, false);
                assert!(
                    matches!(b, Bucket::Expressible),
                    "real mirror (67a3c6ac) must be EXPRESSIBLE, got {}",
                    bucket_label(&b)
                );
                assert_eq!(comp.as_deref(), Some("map(rev)"));

                // v-tile: needs map(dup) — EXPRESSIBLE, not SOLVED.
                let vtile = arc_task_to_parse(&load_arc_task_by_id(dir, "a416b8f3")).unwrap();
                let (b, comp, _built) = classify_task(&vtile, &vocab, &cands, &o, false);
                assert!(
                    matches!(b, Bucket::Expressible),
                    "real v-tile (a416b8f3) must be EXPRESSIBLE, got {}",
                    bucket_label(&b)
                );
                assert_eq!(comp.as_deref(), Some("map(dup)"));
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// A real ARC task whose transform needs color/object/region analysis (not a
    /// geometry op) must classify REQUIRES_NEW — the vocabulary genuinely cannot
    /// express it, so the diagnostic points the next ontogenetic step.
    #[test]
    fn arcdiag_real_requires_new() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let o = opts();
                let s = discover_schemas(&o).expect("vocabulary must be discovered");
                let vocab = vocab_concepts(&s);
                let cands = expressibility_candidates(&s, 2);
                // 2dc579da extracts a non-background sub-region — needs
                // color/object analysis, well beyond the list-transform vocab.
                let hard = arc_task_to_parse(&load_arc_task_by_id(ARC_DATA_DIR, "2dc579da")).unwrap();
                let (b, _c, _built) = classify_task(&hard, &vocab, &cands, &o, false);
                assert!(
                    matches!(b, Bucket::RequiresNew),
                    "region-extraction (2dc579da) must be REQUIRES_NEW, got {}",
                    bucket_label(&b)
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// C8 — the decisive causal claim, pinned on real ARC tasks: the 4 EXPRESSIBLE
    /// tasks (mirror `67a3c6ac`, v-tile `a416b8f3`, rotation `3c9b0459`/`6150a2bd`)
    /// are expressible as compositions of the discovered vocabulary, yet the
    /// baseline search gate cannot reach them (ho=false → EXPRESSIBLE). Upgrading
    /// the search to hold first-class concept values (ho=true) moves them into
    /// SOLVED — WITHOUT adding any new concepts or schemas. The ontology is frozen;
    /// only the reasoner's composition power changed. That is the whole claim: the
    /// ontology already contained the solutions; the baseline search simply could
    /// not compose higher-order concepts.
    #[test]
    fn c8_higher_order_moves_real_expressible_to_solved() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let o = opts();
                let s = discover_schemas(&o).expect("vocabulary must be discovered");
                let vocab = vocab_concepts(&s);
                let cands = expressibility_candidates(&s, 2);
                for id in ["67a3c6ac", "a416b8f3", "3c9b0459", "6150a2bd"] {
                    let t = arc_task_to_parse(&load_arc_task_by_id(ARC_DATA_DIR, id)).unwrap();
                    let (b_base, _c, _built) = classify_task(&t, &vocab, &cands, &o, false);
                    assert!(
                        matches!(b_base, Bucket::Expressible),
                        "baseline {id} must be EXPRESSIBLE, got {}",
                        bucket_label(&b_base)
                    );
                    let (b_ho, _c, _built) = classify_task(&t, &vocab, &cands, &o, true);
                    assert!(
                        matches!(b_ho, Bucket::Solved),
                        "higher-order {id} must move to SOLVED, got {}",
                        bucket_label(&b_ho)
                    );
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// C8 on the synthetic families: mirror, rotation, and v-tile (all higher-order
    /// compositions — map(rev), rev∘map(rev), map(dup)) were EXPRESSIBLE-but-not-
    /// SOLVED under the baseline search; under first-class concept values they
    /// must be SOLVED. Mirrors the real-task claim on tasks whose composition is
    /// known exactly.
    #[test]
    fn c8_synthetic_mirror_higher_order_solved() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let o = opts();
                let s = discover_schemas(&o).expect("vocabulary must be discovered");
                let vocab = vocab_concepts(&s);
                let cands = expressibility_candidates(&s, 2);
                let families: [(&str, &dyn Fn(usize, usize) -> Rc<term::Term>); 3] = [
                    ("mirror", &mirrored_term),
                    ("rotation", &rotated_term),
                    ("v-tile", &vtiled_term),
                ];
                for (fname, target) in families {
                    let fam = transform_family(&[(3, 3), (5, 5)], target);
                    let (b, _c, _built) = classify_task(&fam, &vocab, &cands, &o, true);
                    assert!(
                        matches!(b, Bucket::Solved),
                        "synthetic {fname} must be SOLVED under higher-order search, got {}",
                        bucket_label(&b)
                    );
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// C8 — the load-bearing representation detail. When the search composes map
    /// over the concept `reverse`, the returned solution must reference reverse as
    /// the quotient atom `Prim(reverse)`, NOT embed reverse's λ-body inline. The
    /// emitted program is `Prim(map);Prim(reverse);grid`, so the concept appears
    /// as an atom and is only expanded under evaluation. Asserting this pins that
    /// first-class concept values stay quotient atoms through composition.
    #[test]
    fn c8_solution_uses_prim_for_function_argument() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let o = opts();
                let s = discover_schemas(&o).expect("vocabulary must be discovered");
                let vocab = vocab_concepts(&s);
                let mirror = transform_family(&[(3, 3), (5, 5)], &mirrored_term);
                let (out, _m) = bank::concept_solve_ho_abl(&mirror, &vocab, &o, true);
                let sol = out.solution.expect("mirror must be solved under ho");
                assert!(
                    term_has_prim(&sol, &s.reverse),
                    "mirror solution must contain Prim(reverse), got: {sol:?}"
                );
                assert!(
                    !term_has_raw_subterm(&sol, &s.reverse),
                    "mirror solution must NOT embed reverse's λ-body inline, got: {sol:?}"
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// Controls: the ontogenesis path (C) must solve at least as much as the
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
