mod bank;
mod compile;
mod decode;
mod dsl;
mod sem;
mod nbe;
mod parse;
mod term;

use parse::TaskError;
use std::fs;
use std::path::PathBuf;

fn main() {
    // The evaluator recurses as deep as the fuel limit allows; give it room.
    let child = std::thread::Builder::new()
        .stack_size(1 << 30)
        .spawn(run)
        .expect("spawn worker");
    child.join().expect("worker panicked");
}

fn run() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if argv.first().map(String::as_str) == Some("mine") {
        mine(&argv[1..]);
        return;
    }
    if argv.first().map(String::as_str) == Some("grow") {
        grow(&argv[1..]);
        return;
    }
    if let Ok(path) = std::env::var("SUP_LIB") {
        let n = dsl::load_library(std::path::Path::new(&path)).expect("load SUP_LIB");
        eprintln!("loaded {n} library entries from {path}");
    }
    let mut args = argv.into_iter();
    let mut tsk_dir: Option<PathBuf> = None;
    let mut out_dir = PathBuf::from("out");
    let mut filter = String::new();
    let mut skip_existing = false;
    let mut opts = bank::Options::default();

    while let Some(a) = args.next() {
        match a.as_str() {
            "--out" => out_dir = PathBuf::from(args.next().expect("--out DIR")),
            "--filter" => filter = args.next().expect("--filter PREFIX"),
            "--max-size" => opts.max_size = args.next().unwrap().parse().unwrap(),
            "--max-depth" => opts.max_depth = args.next().unwrap().parse().unwrap(),
            "--fuel" => opts.fuel = args.next().unwrap().parse().unwrap(),
            "--timeout" => opts.time_budget_secs = args.next().unwrap().parse().unwrap(),
            "--seed-y" => opts.seeds.push(bank::y_combinator()),
            "--skip-existing" => skip_existing = true,
            "--lib" => {
                let path = args.next().expect("--lib FILE");
                let text = fs::read_to_string(&path).expect("read lib file");
                for line in text.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with("//") {
                        continue;
                    }
                    let t = parse::parse_expr(line)
                        .and_then(|e| parse::to_term(&e))
                        .unwrap_or_else(|e| panic!("bad lib term '{line}': {e}"));
                    opts.seeds.push(t);
                }
            }
            _ if tsk_dir.is_none() => tsk_dir = Some(PathBuf::from(a)),
            other => {
                eprintln!("unknown arg: {other}");
                std::process::exit(1);
            }
        }
    }

    let tsk_dir = tsk_dir.unwrap_or_else(|| {
        eprintln!("usage: supsearch <tsk_dir> [--out DIR] [--filter PREFIX] [--max-size N] [--max-depth N] [--fuel N] [--timeout SECS]");
        std::process::exit(1);
    });

    fs::create_dir_all(&out_dir).expect("create out dir");

    let mut files: Vec<PathBuf> = fs::read_dir(&tsk_dir)
        .expect("read tsk dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "tsk"))
        .collect();
    files.sort();

    let mut solved = 0usize;
    let mut attempted = 0usize;
    let mut skipped = 0usize;

    for path in &files {
        let id = path.file_stem().unwrap().to_string_lossy().to_string();
        if !filter.is_empty() && !id.starts_with(&filter) {
            continue;
        }
        if skip_existing && out_dir.join(format!("{id}.lam")).exists() {
            continue;
        }
        let text = fs::read_to_string(path).expect("read task");
        let task = match parse::parse_task(&id, &text) {
            Ok(t) => t,
            Err(TaskError::Unsupported(msg)) => {
                println!("- {id}: skipped ({msg})");
                skipped += 1;
                continue;
            }
            Err(TaskError::Parse(msg)) => {
                println!("- {id}: parse error ({msg})");
                skipped += 1;
                continue;
            }
        };
        attempted += 1;
        // Semantic track first: decode → DSL search → compile → verify.
        let sem_start = std::time::Instant::now();
        if let Some((src, e, n_args)) = try_semantic(&id, &task, &sem::SemOptions::default()) {
            solved += 1;
            let out_path = out_dir.join(format!("{id}.lam"));
            fs::write(&out_path, &src).expect("write solution");
            write_dsl(&out_dir, &id, n_args, &e);
            println!(
                "✓ {id}: semantic track in {:.3}s -> {}",
                sem_start.elapsed().as_secs_f64(),
                out_path.display()
            );
            continue;
        }
        let outcome = bank::solve(&task, &opts);
        match outcome.solution {
            Some(sol) => {
                solved += 1;
                let src = format!("@main = {}\n", term::show(&sol));
                let out_path = out_dir.join(format!("{id}.lam"));
                fs::write(&out_path, &src).expect("write solution");
                println!(
                    "✓ {id}: size {} in {:.2}s ({} built, {} classes) -> {}",
                    sol.size(),
                    outcome.stats.elapsed_secs,
                    outcome.stats.built,
                    outcome.stats.kept,
                    out_path.display()
                );
            }
            None => {
                println!(
                    "✗ {id}: no solution ≤ size {} in {:.2}s ({} built, {} classes, {} aborted)",
                    outcome.stats.reached_size,
                    outcome.stats.elapsed_secs,
                    outcome.stats.built,
                    outcome.stats.kept,
                    outcome.stats.aborted
                );
            }
        }
    }

    println!("\nsolved {solved}/{attempted} attempted ({skipped} skipped)");
}

/// Persist the winning DSL term next to the compiled solution — the mining
/// corpus. Format: `args=N` then the S-expression.
fn write_dsl(out_dir: &std::path::Path, id: &str, n_args: usize, e: &sem::E) {
    let path = out_dir.join(format!("{id}.dsl"));
    let text = format!("args={n_args}\n{}\n", dsl::print_e(e));
    fs::write(path, text).expect("write dsl");
}

/// The semantic track: decode test I/O to native values, search the DSL,
/// compile through the Lamb stdlib, verify internally against every test.
/// Any failure at any stage returns None and the λ-bank takes over.
/// On success returns (lam source, DSL term, task arity).
fn try_semantic(
    id: &str,
    task: &parse::Task,
    opts: &sem::SemOptions,
) -> Option<(String, sem::E, usize)> {
    let family = compile::Family::of_task(id)?;
    if matches!(family, compile::Family::CAdt | compile::Family::SAdt) {
        return try_semantic_adt(id, family, task, opts);
    }
    let Some((inputs, outputs, kinds, out_kind)) = compile::decode_task(family, task) else {
        eprintln!("  {id}: semantic decode failed");
        return None;
    };
    if std::env::var("SUP_DEBUG").is_ok() {
        for (j, (i, o)) in inputs.iter().zip(outputs.iter()).enumerate().take(3) {
            eprintln!("  {id} test{j}: in={i:?} out={o:?}");
        }
    }
    let dbg = std::env::var("SUP_DEBUG").is_ok();
    if dbg {
        eprintln!("  {id}: searching...");
    }
    let Some(e) = sem::solve(&inputs, &outputs, opts) else {
        eprintln!("  {id}: no DSL expression found (kinds {kinds:?}, out {out_kind:?})");
        return None;
    };
    if dbg {
        eprintln!("  {id}: candidate found: {e:?}");
    }
    // The tuple adapter needs to know which Nat argument is the size; its
    // position varies per task, so try each Nat position until one verifies.
    let nat_positions: Vec<usize> = kinds
        .iter()
        .enumerate()
        .filter(|(_, k)| **k == compile::ArgKind::Nat)
        .map(|(i, _)| i)
        .collect();
    let size_choices = if nat_positions.is_empty() {
        vec![0]
    } else {
        nat_positions
    };
    // Fuel bounds recursion depth too: keep it under what the 1GB worker
    // stack can absorb when a bad candidate diverges. Algo/tree solutions
    // run real algorithms (backtracking, BFS, DFT), so they get more.
    let fuel = match family {
        compile::Family::Algo => 3_000_000,
        compile::Family::CTre | compile::Family::STre => 3_000_000,
        _ => 2_000_000,
    };
    for size_idx in size_choices {
        let src = compile::program(family, out_kind, &e, &kinds, size_idx);
        let Ok(main) = compile::inline_main(&src) else {
            continue;
        };
        if compile::verify(&main, task, fuel) {
            let n_args = kinds.len();
            return Some((src, e, n_args));
        }
    }
    eprintln!("  {id}: semantic candidate failed internal verification");
    None
}

/// ADT families: multi-candidate decode (Church/Scott zero-field values are
/// syntactically identical, so output-kind choice is settled by verifying),
/// plus template fallback for the constructor-builder tasks whose outputs
/// are functions rather than data.
fn try_semantic_adt(
    id: &str,
    family: compile::Family,
    task: &parse::Task,
    opts: &sem::SemOptions,
) -> Option<(String, sem::E, usize)> {
    let cands = compile::decode_task_adt(family, task);
    if std::env::var("SUP_DEBUG").is_ok() {
        eprintln!("  {id}: {} adt candidates", cands.len());
        for (inputs, outputs, kinds, out_kind, _) in &cands {
            eprintln!("    kinds {kinds:?} out {out_kind:?} in0 {:?} out0 {:?}", inputs[0], outputs[0]);
        }
    }
    for (inputs, outputs, kinds, out_kind, desc_pos) in cands
    {
        let Some(e) = sem::solve(&inputs, &outputs, opts) else {
            continue;
        };
        let src = compile::program(family, out_kind, &e, &kinds, desc_pos);
        let Ok(main) = compile::inline_main(&src) else {
            continue;
        };
        if compile::verify(&main, task, 2_000_000) {
            let n_args = kinds.len();
            return Some((src, e, n_args));
        }
    }
    eprintln!("  {id}: semantic track exhausted");
    None
}

/// The library-growth experiment: solve with the seed tier only, compress
/// the solved corpus into library abstractions, re-search with the grown
/// library, and repeat to a fixed point. Nothing here touches the certified
/// full-tier configuration.
fn grow(args: &[String]) {
    let mut tsk_dir: Option<PathBuf> = None;
    let mut out_dir = PathBuf::from("outgrow");
    let mut lib_path = PathBuf::from("lib/dsl.lib");
    let mut rounds = 8usize;
    let mut budget = 20u64;
    let mut per_round = 6usize;
    let mut filter = String::new();
    let mut fresh = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                out_dir = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--lib" => {
                lib_path = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--rounds" => {
                rounds = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--budget" => {
                budget = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--per-round" => {
                per_round = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--filter" => {
                filter = args[i + 1].clone();
                i += 2;
            }
            "--fresh" => {
                fresh = true;
                i += 1;
            }
            other => {
                if tsk_dir.is_none() {
                    tsk_dir = Some(PathBuf::from(other));
                    i += 1;
                } else {
                    eprintln!("unknown grow arg: {other}");
                    std::process::exit(1);
                }
            }
        }
    }
    let tsk_dir = tsk_dir.unwrap_or_else(|| {
        eprintln!("usage: supsearch grow <tsk_dir> [--out DIR] [--lib FILE] [--rounds N] [--budget SECS] [--per-round N] [--filter PREFIX] [--fresh]");
        std::process::exit(1);
    });
    fs::create_dir_all(&out_dir).expect("create out dir");
    if let Some(parent) = lib_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if !fresh && lib_path.exists() {
        let n = dsl::load_library(&lib_path).expect("load library");
        println!("loaded {n} library entries from {}", lib_path.display());
    }

    // Parse every task once.
    let mut files: Vec<PathBuf> = fs::read_dir(&tsk_dir)
        .expect("read tsk dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "tsk"))
        .collect();
    files.sort();
    let mut tasks: Vec<(String, parse::Task)> = Vec::new();
    for path in &files {
        let id = path.file_stem().unwrap().to_string_lossy().to_string();
        if !filter.is_empty() && !id.starts_with(&filter) {
            continue;
        }
        let text = fs::read_to_string(path).expect("read task");
        if let Ok(t) = parse::parse_task(&id, &text) {
            tasks.push((id, t));
        }
    }
    println!("grow: {} tasks, seed tier, {budget}s budget/task", tasks.len());

    let mut corpus: Vec<dsl::CorpusEntry> = Vec::new();
    let mut solved: std::collections::HashSet<String> = std::collections::HashSet::new();
    for round in 0..rounds {
        let opts = sem::SemOptions {
            tier: sem::Tier::Seed,
            budget_secs: budget,
            ..Default::default()
        };
        let round_start = std::time::Instant::now();
        let mut new_solves = 0usize;
        for (id, task) in &tasks {
            if solved.contains(id) {
                continue;
            }
            let t0 = std::time::Instant::now();
            if let Some((src, e, n_args)) = try_semantic(id, task, &opts) {
                fs::write(out_dir.join(format!("{id}.lam")), &src).expect("write solution");
                write_dsl(&out_dir, id, n_args, &e);
                println!(
                    "✓ {id} (round {round}, {:.2}s): {}",
                    t0.elapsed().as_secs_f64(),
                    dsl::print_e(&e)
                );
                corpus.push(dsl::CorpusEntry {
                    id: id.clone(),
                    n_args: n_args as u32,
                    e,
                });
                solved.insert(id.clone());
                new_solves += 1;
            }
        }
        let added = dsl::mine_round(&mut corpus, per_round);
        for (idx, body) in &added {
            let arity = dsl::lib_arity(*idx as u16);
            println!("  + L{idx}/{arity} = {body}");
        }
        dsl::save_library(&lib_path).expect("save library");
        // Refresh persisted corpus (rewritten by compression).
        for entry in &corpus {
            write_dsl(&out_dir, &entry.id, entry.n_args as usize, &entry.e);
        }
        println!(
            "round {round}: solved {}/{} (+{new_solves}), library {} entries, {:.1}s",
            solved.len(),
            tasks.len(),
            dsl::lib_len(),
            round_start.elapsed().as_secs_f64()
        );
        if new_solves == 0 && added.is_empty() {
            println!("fixed point reached");
            break;
        }
    }
    println!(
        "\ngrow finished: {}/{} solved with seed tier + mined library ({} entries -> {})",
        solved.len(),
        tasks.len(),
        dsl::lib_len(),
        lib_path.display()
    );
}

/// Library mining: extract closed subterms recurring across solved programs
/// and rank them by compression value (occurrences × size). The output lines
/// feed back into search via --lib.
fn mine(args: &[String]) {
    let mut dir: Option<PathBuf> = None;
    let mut top = 16usize;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--top" => {
                top = args[i + 1].parse().unwrap();
                i += 2;
            }
            other => {
                dir = Some(PathBuf::from(other));
                i += 1;
            }
        }
    }
    let dir = dir.expect("usage: supsearch mine <solutions_dir> [--top N]");

    use std::collections::HashMap;
    use std::rc::Rc;
    let mut counts: HashMap<String, (Rc<term::Term>, usize)> = HashMap::new();

    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read solutions dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "lam"))
        .collect();
    files.sort();

    for path in &files {
        let text = fs::read_to_string(path).expect("read solution");
        for line in text.lines() {
            let line = line.trim();
            let Some(rhs) = line.strip_prefix("@main = ") else {
                continue;
            };
            let Ok(t) = parse::parse_expr(rhs).and_then(|e| parse::to_term(&e)) else {
                continue;
            };
            collect_closed_subterms(&t, 0, &mut |sub| {
                if sub.size() >= 3 {
                    let key = term::show(sub);
                    counts
                        .entry(key)
                        .and_modify(|e| e.1 += 1)
                        .or_insert_with(|| (sub.clone(), 1));
                }
            });
        }
    }

    let mut ranked: Vec<(&String, &(Rc<term::Term>, usize))> = counts.iter().collect();
    ranked.sort_by_key(|(_, (t, n))| std::cmp::Reverse(n * t.size() as usize));
    for (src, (t, n)) in ranked.into_iter().take(top) {
        println!("// x{n}, size {}", t.size());
        println!("{src}");
    }
}

/// Visit every subterm that is closed as a standalone term (no de Bruijn
/// index escaping it). `d` tracks binders above the current node.
fn collect_closed_subterms(
    t: &std::rc::Rc<term::Term>,
    d: u32,
    f: &mut impl FnMut(&std::rc::Rc<term::Term>),
) {
    let _ = d;
    if closed_above(t, 0) {
        f(t);
    }
    match t.as_ref() {
        term::Term::Lam(b) => collect_closed_subterms(b, d + 1, f),
        term::Term::App(x, a) => {
            collect_closed_subterms(x, d, f);
            collect_closed_subterms(a, d, f);
        }
        _ => {}
    }
}

/// True if no variable in `t` escapes `depth` binders.
fn closed_above(t: &term::Term, depth: u32) -> bool {
    match t {
        term::Term::Var(i) => *i < depth,
        term::Term::Free(_) => false,
        term::Term::Lam(b) => closed_above(b, depth + 1),
        term::Term::App(f, a) => closed_above(f, depth) && closed_above(a, depth),
    }
}
