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
}
