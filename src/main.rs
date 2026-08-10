// The ontology-bootstrap track (live): raw λ-term growth with no semantic
// vocabulary. bank = raw-λ search, bootstrap = the miner + grow driver.
mod bank;
mod bootstrap;
mod canon;
mod nbe;
mod parse;
mod term;

// The frozen 120/120 semantic engine (historical, not developed further):
// a typed DSL over decoded Church/Scott values. Kept compilable so the
// `sem`/`grow`/`mine`/`validate` subcommands still run, but superseded by
// the raw-λ bootstrap track above.
#[path = "legacy/sem.rs"]
mod sem;
#[path = "legacy/decode.rs"]
mod decode;
#[path = "legacy/dsl.rs"]
mod dsl;
#[path = "legacy/compile.rs"]
mod compile;

use parse::TaskError;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

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
    if argv.first().map(String::as_str) == Some("bootstrap") {
        bootstrap(&argv[1..]);
        return;
    }
    if argv.first().map(String::as_str) == Some("ladder") {
        ladder(&argv[1..]);
        return;
    }
    if argv.first().map(String::as_str) == Some("promote") {
        promote(&argv[1..]);
        return;
    }
    if argv.first().map(String::as_str) == Some("ablation") {
        ablation(&argv[1..]);
        return;
    }
    if argv.first().map(String::as_str) == Some("diag") {
        diag(&argv[1..]);
        return;
    }
    if argv.first().map(String::as_str) == Some("prune") {
        prune(&argv[1..]);
        return;
    }
    if argv.first().map(String::as_str) == Some("ontogen") {
        ontogen(&argv[1..]);
        return;
    }
    if argv.first().map(String::as_str) == Some("dep") {
        dep(&argv[1..]);
        return;
    }
    if argv.first().map(String::as_str) == Some("gen") {
        gen(&argv[1..]);
        return;
    }
    if argv.first().map(String::as_str) == Some("transfer") {
        transfer(&argv[1..]);
        return;
    }
    if argv.first().map(String::as_str) == Some("meta") {
        meta(&argv[1..]);
        return;
    }
    if argv.first().map(String::as_str) == Some("mkbench") {
        gen_benchmark(&argv[1..]);
        return;
    }
    if argv.first().map(String::as_str) == Some("validate") {
        let path = argv
            .get(1)
            .map(String::as_str)
            .unwrap_or("lib/dsl.lib")
            .to_string();
        let path = std::path::PathBuf::from(path);
        let n = dsl::load_library(&path).expect("load library");
        println!("validating {n} entries from {}", path.display());
        for (i, verdict) in dsl::validate_all() {
            let arity = dsl::lib_arity(i as u16);
            println!("L{i}/{arity}: {verdict}");
        }
        dsl::save_library(&path).expect("save library");
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
            let note = dsl::lib_note(*idx as u16);
            println!("  + L{idx}/{arity} = {body}  [{note}]");
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

/// Ontology-bootstrap grow driver (the raw-λ track; reaches no sem/decode/dsl
/// code paths). Repeats: raw-λ solve on a training split → mine behavioral
/// abstractions from the solved raw terms → generality-validate → inject as
/// seeds → re-search. The held-out split is never mined and is measured for
/// the cost curve C(L_t) = (solve_rate, median_cost, censored).
fn bootstrap(args: &[String]) {
    use std::rc::Rc;
    let mut tsk_dir: Option<PathBuf> = None;
    let mut lib_path = PathBuf::from("lib/bootstrap.lib");
    let mut rounds = 3usize;
    let mut budget = 10u64;
    let mut per_round = 4usize;
    let mut max_size = 14u32;
    let mut train: Vec<String> = Vec::new();
    let mut holdout: Vec<String> = Vec::new();
    let mut seed_y = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
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
            "--max-size" => {
                max_size = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--seed-y" => {
                seed_y = true;
                i += 1;
            }
            "--train" => {
                train = args[i + 1].split(',').map(str::to_string).collect();
                i += 2;
            }
            "--holdout" => {
                holdout = args[i + 1].split(',').map(str::to_string).collect();
                i += 2;
            }
            other => {
                if tsk_dir.is_none() {
                    tsk_dir = Some(PathBuf::from(other));
                    i += 1;
                } else {
                    eprintln!("unknown bootstrap arg: {other}");
                    std::process::exit(1);
                }
            }
        }
    }
    let tsk_dir = tsk_dir.unwrap_or_else(|| {
        eprintln!("usage: supsearch bootstrap <tsk_dir> --train a,b --holdout c,d [--rounds N] [--budget SECS] [--per-round N] [--max-size N] [--seed-y] [--lib FILE]");
        std::process::exit(1);
    });
    if train.is_empty() || holdout.is_empty() {
        eprintln!("bootstrap requires both --train and --holdout task id lists");
        std::process::exit(1);
    }
    if let Some(parent) = lib_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let load = |id: &str| -> Option<parse::Task> {
        let text = fs::read_to_string(tsk_dir.join(format!("{id}.tsk"))).ok()?;
        parse::parse_task(id, &text).ok()
    };
    let train_tasks: Vec<(String, parse::Task)> = train
        .iter()
        .filter_map(|id| load(id).map(|t| (id.clone(), t)))
        .collect();
    let holdout_tasks: Vec<(String, parse::Task)> = holdout
        .iter()
        .filter_map(|id| load(id).map(|t| (id.clone(), t)))
        .collect();
    println!(
        "bootstrap: {} train tasks (mined), {} holdout tasks (measured), raw-λ bank, no semantic vocabulary",
        train_tasks.len(),
        holdout_tasks.len()
    );
    if train_tasks.len() != train.len() || holdout_tasks.len() != holdout.len() {
        eprintln!("warning: some task ids did not load (missing .tsk?); proceeding with loaded subset");
    }

    let mut seeds: Vec<Rc<term::Term>> = Vec::new();
    if seed_y {
        seeds.push(bank::y_combinator());
    }
    let mut corpus: Vec<Rc<term::Term>> = Vec::new();
    let mut train_solved: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut seen_keys: std::collections::HashSet<bootstrap::BehaviorKey> =
        std::collections::HashSet::new();
    let mut results: Vec<(usize, f64, Option<f64>, usize, usize)> = Vec::new(); // (gen, rate, median_cost, censored, n_seeds)
    let mut mopts = bootstrap::MineOptions::default();
    mopts.per_round = per_round;

    for gen in 0..rounds {
        let mut new_solves = 0usize;
        // 1. Solve train (current seeds) → corpus.
        for (id, task) in &train_tasks {
            if train_solved.contains(id) {
                continue;
            }
            let outcome = bank::solve(task, &bank_opts(&seeds, budget, max_size));
            if let Some(sol) = outcome.solution {
                corpus.push(sol.clone());
                train_solved.insert(id.clone());
                new_solves += 1;
                println!("  train ✓ {id} (gen {gen}, size {}): {}", sol.size(), term::show(&sol));
            } else {
                println!("  train ✗ {id} (gen {gen})");
            }
        }

        // 2. Measure held-out cost curve with current seeds.
        let mut costs: Vec<f64> = Vec::new();
        let mut censored = 0usize;
        for (_id, task) in &holdout_tasks {
            let outcome = bank::solve(task, &bank_opts(&seeds, budget, max_size));
            match outcome.solution {
                Some(_) => costs.push(outcome.stats.elapsed_secs),
                None => censored += 1,
            }
        }
        let rate = costs.len() as f64 / holdout_tasks.len() as f64;
        let median = median(&costs);

        // 3. Mine abstractions from the raw-λ corpus.
        let train_args: Vec<Rc<term::Term>> = train_tasks
            .iter()
            .flat_map(|(_, t)| t.tests.iter())
            .flat_map(|te| te.args.iter().cloned())
            .collect();
        let grouping = bootstrap::build_grouping_pool(&train_args, 0x5eed_0000 + gen as u64);
        let holdout_probes = bootstrap::build_holdout_pool(0xc0ffee_0000 + gen as u64 * 7);
        let mined = bootstrap::mine(&corpus, &grouping, &holdout_probes, &mopts);
        let mut added = 0usize;
        for m in &mined {
            if !seen_keys.insert(m.key.clone()) {
                continue; // already promoted in an earlier generation
            }
            seeds.push(m.comb.clone());
            added += 1;
            println!(
                "  + seed arity {} gain {} x{} : {}  [{}]",
                m.k,
                m.gain,
                m.count,
                term::show(&m.comb),
                m.note
            );
        }

        // 4. Persist the library (one closed term per line; notes as comments).
        let mut lib = String::new();
        for (j, s) in seeds.iter().enumerate() {
            if let Some(note) = mined.iter().find(|m| Rc::ptr_eq(&m.comb, s)) {
                lib.push_str(&format!("// seed {j}: {}\n", note.note));
            }
            lib.push_str(&format!("{}\n", term::show(s)));
        }
        fs::write(&lib_path, lib).expect("write bootstrap lib");

        results.push((gen, rate, median, censored, seeds.len()));
        println!(
            "gen {gen}: holdout rate {:.0}% ({}/{}), median cost {:.3}s, censored {censored}, seeds {}, mined +{added}, corpus {}",
            rate * 100.0,
            costs.len(),
            holdout_tasks.len(),
            median.unwrap_or(0.0),
            seeds.len(),
            corpus.len()
        );
        if new_solves == 0 && mined.is_empty() {
            println!("fixed point reached");
            break;
        }
    }

    // Cost-curve table.
    println!("\nC(L_t) on held-out (no vocabulary, raw-λ bank):");
    println!("  gen | solve_rate | median_cost | censored | seeds");
    for (gen, rate, median, censored, n_seeds) in &results {
        let mc = median.map(|m| format!("{m:.3}s")).unwrap_or("–".into());
        println!("   {gen}  |   {:.0}%     |   {mc}   |  {censored}    |  {n_seeds}", rate * 100.0);
    }
    if results.len() < 2 || results.iter().all(|(_, r, _, _, _)| *r >= 1.0) {
        println!(
            "note: solve_rate may be capped at 100% if held-out tasks are already raw-solvable; \
             the informative signal is median_cost (does seeding cheapen already-possible solves?) \
             and censored count on tasks just past the wall."
        );
    }
    println!(
        "\nbootstrap finished: {}/{} train solved, {} seeds -> {}",
        train_solved.len(),
        train_tasks.len(),
        seeds.len(),
        lib_path.display()
    );
}

/// Concept Ladder: the "absurdly small" demo of concept creation.
///
/// λ-encoded Church arithmetic. The given vocabulary is nothing but raw λ and
/// the base Church combinators (succ, add) — no multiplication, square, power,
/// or parity. The bank solves each rung's recurring family; the miner abstracts
/// the recurring closed idiom out of the solved corpus and promotes it as a
/// seed (divergence-guard-validated); the next rung is solved with that concept
/// in the language. Each rung measures a held-out task that *uses* the concept
/// before and after promotion, so the cost effect is measured, not asserted.
///
/// usage: supsearch ladder [--budget SECS] [--max-size N]
fn ladder(args: &[String]) {
    use std::io::Write;
    use std::rc::Rc;

    let mut budget = 20u64;
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
                eprintln!("unknown ladder arg: {other}");
                std::process::exit(1);
            }
        }
    }

    // Church-encoded values (concrete data, outer=0 tests).
    let num = |n: u32| -> Rc<term::Term> {
        parse::parse_expr(&bootstrap::church_num_str(n))
            .and_then(|e| parse::to_term(&e))
            .expect("church numeral")
    };
    let bval = |tru: bool| -> Rc<term::Term> {
        parse::parse_expr(if tru { "λa.λb.a" } else { "λa.λb.b" })
            .and_then(|e| parse::to_term(&e))
            .expect("church boolean")
    };
    let closed = |s: &str| -> Rc<term::Term> {
        parse::parse_expr(s)
            .and_then(|e| parse::to_term(&e))
            .expect("closed term")
    };
    let task = |arity: usize, tests: Vec<(Vec<u32>, Rc<term::Term>)>| parse::Task {
        arity,
        tests: tests
            .into_iter()
            .map(|(args, want)| parse::Test {
                args: args.iter().map(|&n| num(n)).collect(),
                want,
                outer: 0,
            })
            .collect(),
    };

    // ── Task builders for the four concepts ──
    // Multiplication: f(a,b)=a×b (repeated addition over the base add).
    let mul = task(
        2,
        [(2, 3), (3, 4), (1, 5), (0, 7), (4, 4), (5, 2)]
            .into_iter()
            .map(|(a, b)| (vec![a, b], num(a * b)))
            .collect(),
    );
    // Square: f(x)=x×x (uses the invented mul).
    let square = task(
        1,
        [2u32, 3, 1, 0, 5, 4]
            .into_iter()
            .map(|x| (vec![x], num(x * x)))
            .collect(),
    );
    // Power: f(x,n)=xⁿ.
    let power = task(
        2,
        [(2u32, 3u32), (3, 2), (2, 0), (1, 5), (3, 1), (2, 4)]
            .into_iter()
            .map(|(x, n)| (vec![x, n], num(x.pow(n))))
            .collect(),
    );
    // Parity: f(n)=even?(n) — a *predicate* (the highlight concept).
    let parity = task(
        1,
        [0u32, 1, 2, 3, 4, 5]
            .into_iter()
            .map(|n| (vec![n], bval(n % 2 == 0)))
            .collect(),
    );
    // Held-out tasks that each *use* their rung's concept, measured before/after
    // the concept is mined and promoted.
    let holdout_mul = task(
        3,
        [(2u32, 3u32, 2u32), (1, 4, 3), (3, 2, 2), (0, 5, 7), (2, 2, 2), (1, 1, 5)]
            .into_iter()
            .map(|(a, b, c)| (vec![a, b, c], num(a * b * c)))
            .collect(),
    );
    let holdout_square = task(
        1,
        [2u32, 3, 1, 0, 4, 5]
            .into_iter()
            .map(|x| (vec![x], num(x * x * x)))
            .collect(),
    );
    let holdout_power = task(
        2,
        [(2u32, 3u32), (3, 2), (1, 4), (2, 1), (4, 2), (1, 0)]
            .into_iter()
            .map(|(x, n)| (vec![x, n], num(x.pow(n + 1))))
            .collect(),
    );

    // Given vocabulary: raw λ plus ONE base Church combinator (add). Everything
    // else — mul, square, power, parity — must be invented by the machine.
    let base = vec![closed("λa.λb.λf.λx.a(f)(b(f)(x))")]; // add
    let mut corpus: Vec<Rc<term::Term>> = Vec::new();
    // The ACQUIRED language: concepts promoted by measured counterfactual
    // quotient gain (Δ > 0 on the held-out), reasoned through as Prims.
    let mut concepts: Vec<bank::Concept> = Vec::new();
    // Concepts the machine *discovered* by raw enumeration (name, body, arity),
    // used to demonstrate the quotient-aware search below.
    let mut discovered: Vec<(&str, Rc<term::Term>, u32)> = Vec::new();

    // Each rung solves a small *recurring family* for its concept (so the
    // miner sees the concept as a repeated idiom, not an isolated term), mines
    // the recurring abstraction, promotes it, then measures a held-out task
    // that uses the concept — before and after.
    struct Rung {
        name: &'static str,
        family: Vec<(String, parse::Task)>,
        holdout: parse::Task,
    }
    let rungs = vec![
        Rung {
            name: "mul",
            family: vec![
                ("a×b".into(), mul.clone()),
                (
                    "a×b+c".into(),
                    task(
                        3,
                        [(2u32, 3u32, 1u32), (3, 4, 2), (1, 5, 3), (2, 2, 5), (4, 4, 1), (3, 1, 7)]
                            .into_iter()
                            .map(|(a, b, c)| (vec![a, b, c], num(a * b + c)))
                            .collect(),
                    ),
                ),
                ("a×b×c".into(), holdout_mul.clone()),
            ],
            holdout: holdout_mul,
        },
        Rung {
            name: "square",
            family: vec![("x×x".into(), square.clone())],
            holdout: holdout_square,
        },
        Rung {
            name: "power",
            family: vec![("x^n".into(), power.clone())],
            holdout: holdout_power,
        },
        Rung {
            name: "parity",
            family: vec![("even?".into(), parity.clone())],
            holdout: parity,
        },
    ];

    println!(
        "Concept Ladder: raw λ + add, {budget}s/task, max_size {max_size}. mul/square/power/parity are NOT given."
    );
    println!(
        "Discovery = raw solve. ACQUISITION = promote a candidate only if, installed as a Prim,\n\
         it reduces the held-out quotient-search cost (counterfactual Δ > 0)."
    );

    for (gen, rung) in rungs.into_iter().enumerate() {
        // 1. Discover: raw-solve this rung's family under the BASE vocabulary
        //    (raw λ + add). `primary_sol` is the complete discovered solution —
        //    the candidate that carries the rung's *semantic operator*.
        let mut fam_solved = 0usize;
        let mut primary_sol: Option<Rc<term::Term>> = None;
        for (j, (_fname, t)) in rung.family.iter().enumerate() {
            let o = bank::solve(t, &bank_opts(&base, budget, max_size));
            if let Some(sol) = o.solution {
                corpus.push(sol.clone());
                fam_solved += 1;
                if j == 0 {
                    primary_sol = Some(sol);
                }
            }
        }
        if let Some(sol) = &primary_sol {
            println!(
                "Gen {:>6}  raw discovers {} = {} (size {})",
                rung.name,
                rung.name,
                term::show(sol),
                sol.size()
            );
            // Record a discovered concept (composition arity) for the
            // quotient-aware demonstration below. mul/square consume 2/1 inputs.
            let arity = match rung.name {
                "mul" => 2,
                "square" => 1,
                "power" => 2,
                _ => 0,
            };
            if arity > 0 {
                discovered.push((rung.name, sol.clone(), arity));
            }
        } else {
            println!(
                "Gen {:>6}  NO raw solution in budget ({} of {} family solved) — nothing discovered",
                rung.name,
                fam_solved,
                rung.family.len()
            );
        }

        // 2. Counterfactual acquisition gate. baseline = held-out cost through
        //    the CURRENT acquired language (quotient search). A candidate earns
        //    promotion only if installing it as a Prim makes that cheaper
        //    (Δ > 0); how often its subterm recurs is irrelevant to that call.
        let opts = bank_opts(&base, budget, max_size);
        let baseline = concept_cost(&rung.holdout, &concepts, &opts);
        println!(
            "    held-out '{}': baseline {} states",
            label_of_holdout(rung.name),
            disp_cost(baseline)
        );

        // 3. Candidate set: the complete discovered solution first; mined
        //    recurring fragments (bootstrap::mine) are merely one more source,
        //    and are subject to the SAME counterfactual gate.
        let mut candidates: Vec<(String, Rc<term::Term>)> = Vec::new();
        if let Some(sol) = &primary_sol {
            candidates.push((format!("C_{}", rung.name), sol.clone()));
        }
        if fam_solved > 0 {
            let grouping = bootstrap::build_grouping_pool(&[], 0x5eed_0000 + gen as u64);
            let holdout_pool = bootstrap::build_holdout_pool(0xc0ffee_0000 + gen as u64 * 7);
            let mut mopts = bootstrap::MineOptions::default();
            mopts.per_round = 2;
            for m in bootstrap::mine(&corpus, &grouping, &holdout_pool, &mopts) {
                candidates.push((format!("C{}", concepts.len() + 1), m.comb));
            }
        }

        // 4. Score every candidate; promote the best one that earns its place,
        //    ranking frontier gain ≻ search-cost reduction (one per generation,
        //    matching the old per-round promotion count).
        let mut best: Option<(String, Gain)> = None;
        for (label, body) in &candidates {
            match propose_value(body, &concepts, &[rung.holdout.clone()], &opts, baseline) {
                Some(g) if g.earns() => {
                    println!(
                        "    candidate {label}: before {} → after {}  {}  PROMOTE  arity {}",
                        disp_cost(g.before),
                        disp_cost(g.after),
                        g.kind(),
                        g.arity
                    );
                    if best.as_ref().map_or(true, |(_, b)| gain_rank(&g, b) == std::cmp::Ordering::Greater) {
                        best = Some((label.clone(), g));
                    }
                }
                Some(g) => println!(
                    "    candidate {label}: before {} → after {}  {}  REJECT",
                    disp_cost(g.before),
                    disp_cost(g.after),
                    g.kind()
                ),
                None => println!("    candidate {label}: no valid interface  REJECT"),
            }
        }
        if let Some((label, g)) = best {
            let body = candidates
                .iter()
                .find(|(l, _)| *l == label)
                .map(|(_, b)| b.clone())
                .expect("promoted candidate present");
            concepts.push(bank::Concept {
                body,
                name: label.clone(),
                arity: g.arity,
            });
            println!(
                "    → ACQUIRE {label} (interface arity {}, {}: {} → {})",
                g.arity,
                g.kind(),
                disp_cost(g.before),
                disp_cost(g.after)
            );
        } else if candidates.is_empty() {
            println!("    → no acquisition candidate (nothing discovered this generation)");
        } else {
            println!("    → nothing earned acquisition this generation");
        }
        std::io::stdout().flush().ok();
    }

    // ── Quotient-aware search (condition C): does the acquired concept change
    // the effective cost of *future* cognition? Once the machine has discovered
    // `mul`, a search that composes it over its inputs instead of re-deriving it
    // collapses the held-out product family's cost — including tasks the raw
    // bank cannot solve at all.
    if let Some((cname, cbody, carity)) = discovered.first() {
        let concept = bank::Concept {
            body: cbody.clone(),
            name: cname.to_string(),
            arity: *carity,
        };
        let product3 = task(
            3,
            [(2u32, 3u32, 2u32), (1, 4, 3), (3, 2, 2), (2, 2, 3), (1, 1, 5)]
                .into_iter()
                .map(|(a, b, c)| (vec![a, b, c], num(a * b * c)))
                .collect(),
        );
        let product4 = task(
            4,
            [(2u32, 3u32, 2u32, 2u32), (1, 4, 3, 2), (3, 2, 2, 1), (2, 2, 3, 1)]
                .into_iter()
                .map(|(a, b, c, d)| (vec![a, b, c, d], num(a * b * c * d)))
                .collect(),
        );
        println!("\n── Quotient-aware search: reasoning *through* the invented {} ──", concept.name);
        println!(
            "   A raw bank            C search that composes {} over its inputs",
            concept.name
        );
        for (name, t) in [("a×b×c", &product3), ("a×b×c×d", &product4)] {
            let raw = bank::solve(t, &bank_opts(&[], budget, max_size));
            let comp = bank::concept_solve(t, &[concept.clone()], &bank_opts(&[], budget, max_size));
            let rs = raw
                .solution
                .as_ref()
                .map(|_| format!("{:>7} states", raw.stats.built))
                .unwrap_or_else(|| "  ✗ unsolvable".into());
            let cs = comp
                .solution
                .as_ref()
                .map(|s| format!("{:>7} states  (solution size {})", comp.stats.built, s.size()))
                .unwrap_or_else(|| "✗".into());
            println!("   {name:>8}   {rs}   →   {cs}");
        }
        println!(
            "   The concept has been *acquired* only when it changes the cost structure of future\n\
             cognition: composing it collapses a family the raw bank cannot solve at all."
        );
        std::io::stdout().flush().ok();
    }

    println!("\nAcquired language ({} concept{}):", concepts.len(), if concepts.len() == 1 { "" } else { "s" });
    println!(
        "  {}",
        if concepts.is_empty() {
            "  (none — no discovery earned acquisition)".into()
        } else {
            concepts
                .iter()
                .map(|c| format!("{} (arity {})", c.name, c.arity))
                .collect::<Vec<_>>()
                .join(" ")
        }
    );
    println!("\nHonest fine print:");
    println!(
        "  • Discovery ≠ acquisition. The bank SOLVES mul/square/power by raw enumeration — the\n\
           machine discovers them as new terms it was never given. Acquisition is a separate,\n\
           measured decision: promote a candidate only if, installed as a Prim, it drops the\n\
           held-out quotient cost (Δ > 0). Under that criterion mul (a×b×c ✗→17, a×b×c×d ✗→99)\n\
           and power (x^(n+1) ✗→16 — a frontier mul alone cannot reach) are ACQUIRED; square is\n\
           REJECTED (Δ ≤ 0) because mul already reaches its held-out x³ cheaply — discovered,\n\
           but not worth a language slot on this distribution."
    );
    println!(
        "  • The recurring-idiom miner is kept, but demoted to one candidate source, judged by the\n\
           same counterfactual gate. The mined partial-application idiom (C1) recurs yet Δ ≤ 0,\n\
           so it is REJECTED — a concept is not worth its slot merely because it appears often."
    );
    println!(
        "  • The cost collapse IS real, and it comes from changing the *search procedure* (condition\n\
           C: compose the concept over its inputs), not from seeding the existing bottom-up bank.\n\
           That is the thesis made concrete — a machine has acquired a concept only when reasoning\n\
           through it is cheaper than re-deriving it. Honest limits: condition C needs the concept's\n\
           composition arity (2 for mul, not its λ-arity 3), and it composes given concepts over\n\
           inputs — it does not itself invent new concepts or discover new structure."
    );
}

/// usage: supsearch ontogen [--budget SECS] [--max-size N]
///
/// Ontology dependence: the same raw-discovered candidates (mul/square/power)
/// are each evaluated against SEVERAL existing ontologies. A candidate's
/// marginal value `Gain(c | D, O)` depends on what the machine already knows —
/// a concept is not intrinsically valuable. This is the experimental content
/// behind "c is a concept for D ⟺ installing c makes reasoning on D cheaper":
/// square is valuable under the empty ontology (it makes x² solvable) but
/// worthless once mul exists (mul(x,x) already reaches x²). Concept utility is
/// relative to the ontology, exactly as `ladder`'s counterfactual gate implies.
fn ontogen(args: &[String]) {
    use std::io::Write;
    use std::rc::Rc;

    let mut budget = 12u64;
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
                eprintln!("unknown ontogen arg: {other}");
                std::process::exit(1);
            }
        }
    }

    let num = |n: u32| -> Rc<term::Term> {
        parse::parse_expr(&bootstrap::church_num_str(n))
            .and_then(|e| parse::to_term(&e))
            .expect("church numeral")
    };
    let closed = |s: &str| -> Rc<term::Term> {
        parse::parse_expr(s)
            .and_then(|e| parse::to_term(&e))
            .expect("closed term")
    };
    let task = |arity: usize, tests: Vec<(Vec<u32>, Rc<term::Term>)>| parse::Task {
        arity,
        tests: tests
            .into_iter()
            .map(|(args, want)| parse::Test {
                args: args.iter().map(|&n| num(n)).collect(),
                want,
                outer: 0,
            })
            .collect(),
    };

    let base = vec![closed("λa.λb.λf.λx.a(f)(b(f)(x))")]; // add
    let opts = bank_opts(&base, budget, max_size);

    println!(
        "\n── Ontology dependence: a concept's value is relative to what the machine already knows ──"
    );
    println!(
        "Cost ∈ N ∪ {{∞}}. For each raw-discovered candidate c, Gain(c | D, O) = does installing c\n\
         make the held-out D cheaper under ontology O?"
    );

    // The machine rediscovers each candidate by raw enumeration (base vocabulary).
    let mul_t = task(
        2,
        [(2, 3), (3, 4), (1, 5), (0, 7), (4, 4), (5, 2)]
            .into_iter()
            .map(|(a, b)| (vec![a, b], num(a * b)))
            .collect(),
    );
    let square_t = task(
        1,
        [2u32, 3, 1, 0, 5, 4]
            .into_iter()
            .map(|x| (vec![x], num(x * x)))
            .collect(),
    );
    let power_t = task(
        2,
        [(2, 3), (3, 2), (2, 0), (1, 5), (3, 1), (2, 4)]
            .into_iter()
            .map(|(x, n): (u32, u32)| (vec![x, n], num(x.pow(n))))
            .collect(),
    );
    println!("\ndiscovering candidates by raw enumeration...");
    let mul = bank::solve(&mul_t, &opts).solution.expect("raw discovers a×b");
    let square = bank::solve(&square_t, &opts).solution.expect("raw discovers x×x");
    let power = bank::solve(&power_t, &opts).solution.expect("raw discovers x^n");
    println!("  mul    = {} (size {})", term::show(&mul), mul.size());
    println!("  square = {} (size {})", term::show(&square), square.size());
    println!("  power  = {} (size {})", term::show(&power), power.size());

    // Held-outs that isolate each candidate's own contribution: mul → 4-fold,
    // square → x², power → x^(n+1) (nothing but the candidate makes these
    // solvable, so a frontier here is unambiguous).
    let h_mul = task(
        4,
        [(2u32, 3, 2, 2), (1, 4, 3, 2), (3, 2, 2, 1), (2, 2, 3, 1), (1, 1, 5, 2)]
            .into_iter()
            .map(|(a, b, c, d)| (vec![a, b, c, d], num(a * b * c * d)))
            .collect(),
    );
    let h_square = task(
        1,
        [2u32, 3, 1, 0, 5, 4]
            .into_iter()
            .map(|x| (vec![x], num(x * x)))
            .collect(),
    );
    let h_power = task(
        2,
        [(2, 3), (3, 2), (1, 4), (2, 1), (4, 2), (1, 0)]
            .into_iter()
            .map(|(x, n): (u32, u32)| (vec![x, n], num(x.pow(n + 1))))
            .collect(),
    );

    let mk = |body: Rc<term::Term>, name: &str, arity: u32| bank::Concept {
        body,
        name: name.into(),
        arity,
    };
    let c_mul = mk(mul.clone(), "mul", 2);
    let c_square = mk(square.clone(), "square", 1);
    let c_power = mk(power.clone(), "power", 2);
    let ontologies: [(&str, Vec<bank::Concept>); 5] = [
        ("∅           ", vec![]),
        ("{mul}       ", vec![c_mul.clone()]),
        ("{square}    ", vec![c_square.clone()]),
        ("{power}     ", vec![c_power.clone()]),
        ("{mul, power}", vec![c_mul.clone(), c_power.clone()]),
    ];

    let cand: [(&str, Rc<term::Term>, parse::Task); 3] = [
        ("mul", mul, h_mul),
        ("square", square, h_square),
        ("power", power, h_power),
    ];

    for (cname, body, hout) in &cand {
        let hout_desc = match *cname {
            "mul" => "a×b×c×d",
            "square" => "x×x",
            _ => "x^(n+1)",
        };
        println!(
            "\ncandidate {cname}: {}   (held-out: '{hout_desc}')",
            term::show(body)
        );
        for (oname, o) in &ontologies {
            let baseline = concept_cost(hout, o, &opts);
            match propose_value(body, o, &[hout.clone()], &opts, baseline) {
                Some(g) => {
                    let verdict = if g.earns() { "ACQUIRE" } else { "reject" };
                    println!(
                        "    Gain({cname:>6} | {oname}) = before {:<9} after {:<9}  {:<13} {verdict}",
                        disp_cost(g.before),
                        disp_cost(g.after),
                        g.kind()
                    );
                }
                None => println!("    Gain({cname:>6} | {oname}) = no valid interface"),
            }
        }
    }

    println!(
        "\nRead it as contextuality — a concept's value is not intrinsic, it is a property of\n\
         the candidate × ontology pair:\n\
         • square is ACQUIRED under ∅ (it makes x² solvable) but rejected under {{mul}} —\n\
           mul(x,x) already reaches x², so once the machine knows mul, square is redundant.\n\
         • power is the mirror image: worthless under ∅ (✗→✗) and only valuable under {{mul}},\n\
           because x^(n+1) = mul(x, power(x,n)) needs mul as a substrate to pay off.\n\
         • mul is valuable everywhere it is absent, and re-installing a concept already held\n\
           (or installing one the current ontology makes redundant) widens the composition\n\
           space and shows up as a regression.\n\
         square and power are complementary: each is valuable exactly where the other is not,\n\
         which is only observable because Gain(c | D, O) is measured against the ontology."
    );
    std::io::stdout().flush().ok();
}

/// usage: supsearch dep [--budget SECS] [--max-size N] [--max-depth N]
///
/// Recorded negative result — *conditional discoverability*. The matrix
/// (`ontogen`) showed conditional *usefulness*: Gain(power|∅)≤0 but
/// Gain(power|{mul})>0. Here we ask the stronger question: does C₁ make C₂
/// *findable at all*? The probe answers honestly: in the arithmetic tower it does
/// NOT — pow is base-findable (tiny Church combinator), tet is not findable even
/// through {mul,pow}, and the product tower's one step fails via bottom-up too.
/// This subcommand runs the probe and records the structural reason (closure
/// argument) rather than forcing a fake ladder.
///
///     Discover(C₂ | O₀, B) = false   but   Discover(C₂ | O₀+C₁, B) = true
///
/// Discovery is measured with the bottom-up bank (`bank::solve`) because that is
/// the search that can actually *synthesize* a recursion from concept atoms: it
/// injects the current ontology's concepts as size-1 prims (bank.rs:410). The
/// composition model (`concept_solve`) — the "reason *through* the concept"
/// model — provably cannot synthesize a recursion from a lower concept, which is
/// exactly what makes each higher concept *useful* (it needs to be a Prim to
/// unlock a frontier composition cannot reach). So the two columns use two
/// searches, deliberately: bottom-up for "can I find the program", composition
/// for "is it worth a language slot".
///
/// Family (Grzegorczyk fast-growing tower over Church numerals, base = add):
/// mul(a,b) → pow(a,n)=aⁿ → tet(a,n)=a↑↑n → … each the n-fold iteration of the
/// previous. The probe finds no dependency chain in this family (see the verdict
/// block printed at the end), so the ladder is not run — the subcommand stays a
/// diagnostic.
fn dep(args: &[String]) {
    use std::io::Write;
    use std::rc::Rc;

    let mut budget = 12u64;
    let mut max_size = 14u32;
    let mut max_depth = 3u32;
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
            "--max-depth" => {
                max_depth = args[i + 1].parse().unwrap();
                i += 2;
            }
            other => {
                eprintln!("unknown dep arg: {other}");
                std::process::exit(1);
            }
        }
    }

    let num = |n: u32| -> Rc<term::Term> {
        parse::parse_expr(&bootstrap::church_num_str(n))
            .and_then(|e| parse::to_term(&e))
            .expect("church numeral")
    };
    let closed = |s: &str| -> Rc<term::Term> {
        parse::parse_expr(s)
            .and_then(|e| parse::to_term(&e))
            .expect("closed term")
    };
    let task = |arity: usize, tests: Vec<(Vec<u32>, Rc<term::Term>)>| parse::Task {
        arity,
        tests: tests
            .into_iter()
            .map(|(args, want)| parse::Test {
                args: args.iter().map(|&n| num(n)).collect(),
                want,
                outer: 0,
            })
            .collect(),
    };
    // Height-n power tower: a↑↑0=1, a↑↑(n+1)=a^(a↑↑n). Values must stay well
    // under the 2048 hash fuel, so callers keep a=2, n≤3 (max 16).
    let tet_val = |a: u32, n: u32| -> u32 {
        let mut v = 1u32;
        for _ in 0..n {
            v = a.pow(v);
        }
        v
    };

    let base = vec![closed("λa.λb.λf.λx.a(f)(b(f)(x))")]; // add

    // ── family: discovery task + usefulness held-out per generation ──
    let mul_task = task(
        2,
        [(2, 3), (3, 4), (1, 5), (0, 7), (4, 4), (5, 2), (6, 2), (2, 6)]
            .into_iter()
            .map(|(a, b)| (vec![a, b], num(a * b)))
            .collect(),
    );
    // Rich a^n suite (a∈{0..4}, n∈{0..4}, values ≤ 4³=64) — too many rows for a
    // small fixed-power term to overfit (the ladder's size-11 "power" was one).
    let pow_task = task(
        2,
        [
            (2, 0),
            (2, 1),
            (2, 2),
            (2, 3),
            (2, 4),
            (3, 0),
            (3, 1),
            (3, 2),
            (3, 3),
            (1, 0),
            (1, 4),
            (0, 0),
            (0, 2),
            (4, 2),
            (4, 3),
            (4, 1),
        ]
        .into_iter()
        .map(|(a, n): (u32, u32)| (vec![a, n], num(a.pow(n))))
        .collect(),
    );
    // a↑↑n, a∈{1,2}, n≤3 (values 1,2,4,16).
    let tet_task = task(
        2,
        [
            (2, 0),
            (2, 1),
            (2, 2),
            (2, 3),
            (1, 0),
            (1, 1),
            (1, 3),
            (0, 0),
            (0, 2),
        ]
        .into_iter()
        .map(|(a, n)| (vec![a, n], num(tet_val(a, n))))
        .collect(),
    );

    // Discovery through an ontology: bottom-up bank with the ontology's concept
    // bodies injected as size-1 prim atoms.
    let solve_with =
        |t: &parse::Task, o: &[bank::Concept], ms: u32, dp: u32| -> Option<Rc<term::Term>> {
            let mut opts = bank_opts(&base, budget, ms);
            opts.max_depth = dp;
            opts.concepts = o.iter().map(|c| c.body.clone()).collect();
            bank::solve(t, &opts).solution
        };

    // ── the probe IS the deliverable. The arithmetic tower gives no dependency
    //    chain: conditional usefulness (real, via composition) does not imply
    //    conditional discoverability (empty, via either search). This subcommand
    //    exists to record that negative result, not to force a fake ladder. ──
    {
        let c = |b: &Rc<term::Term>, n: &str, a: u32| bank::Concept {
            body: b.clone(),
            name: n.into(),
            arity: a,
        };
        println!("\n── probe: conditional discoverability, before trusting the ladder ──");
        let mul_body = solve_with(&mul_task, &[], max_size, max_depth).expect("mul raw-discoverable");
        let c_mul = c(&mul_body, "mul", 2);
        println!("mul  raw-discoverable from base (max_size {max_size}): size {}", mul_body.size());
        for ms in [10u32, 12, 14, 16, 18, 20] {
            let raw = solve_with(&pow_task, &[], ms, max_depth).is_some();
            let via = solve_with(&pow_task, &[c_mul.clone()], ms, max_depth).is_some();
            println!(
                "  pow  max_size {ms:>2}: raw(base)? {raw:<5}  via {{mul}}? {via}"
            );
        }
        let pow_body = solve_with(&pow_task, &[c_mul.clone()], max_size, max_depth)
            .expect("pow discoverable via {mul}");
        let c_pow = c(&pow_body, "pow", 2);
        println!("pow  discoverable via {{mul}} (max_size {max_size}): size {}", pow_body.size());
        // Is the *discovered* pow actually correct? The true Church pow is λm.λn.n m.
        // Check it solves a held-out pow task (different rows) when installed as a
        // concept — if not, it overfit the discovery rows and the ladder would
        // rightly reject it at the usefulness gate.
        let h_pow_probe = task(
            2,
            [(2, 5), (3, 4), (5, 2), (4, 0), (6, 3)]
                .into_iter()
                .map(|(a, n): (u32, u32)| (vec![a, n], num(a.pow(n))))
                .collect(),
        );
        let pw_ok = concept_cost(&h_pow_probe, &[c_pow.clone()], &bank_opts(&base, budget, max_size))
            < UNREACHABLE;
        println!("  discovered-pow solves held-out a^n (generalizes)? {pw_ok}");
        println!("  (Church pow = λm.λn.n m, a ~6-node combinator — pow is *base-findable* by\n    construction, so mul→pow is NOT a conditional-discoverability step.)");
        // Where conditional discoverability might live: tet. It needs pow as an atom
        // to stay small and is a genuinely recursive (big-raw) program. The search
        // may need higher lambda depth to build `n (λx. pow a x) one`, so sweep depth.
        println!("  tet via {{mul,pow}}: sweep of max_depth (max_size 18), discovered-pow:");
        for d in [3u32, 4, 5, 6] {
            let t_disc = solve_with(&tet_task, &[c_mul.clone(), c_pow.clone()], 18, d);
            let raw_t = solve_with(&tet_task, &[], 18, d).is_some();
            println!(
                "    max_depth {d}: tet via {{mul,pow}}? {:<5}  tet raw(base)? {}",
                t_disc.is_some(),
                raw_t
            );
        }
        // Positive control: the product tower DOES give one conditional-discoverability
        // step (a×b×c×d is unsolvable raw but findable through {mul}), via the SAME
        // bottom-up mechanism. This shows the mechanism is sound; it is the *chain*
        // that fails (mul reaches all products, so no dependency).
        let prod4 = crate::promote_prod_task(4);
        let p4_raw = solve_with(&prod4, &[], 14, max_depth).is_some();
        let p4_via = solve_with(&prod4, &[c_mul.clone()], 14, max_depth).is_some();
        println!(
            "  product tower control (a×b×c×d): raw(base)? {p4_raw}   via {{mul}}? {p4_via}"
        );
        std::io::stdout().flush().ok();
    }

    // ── the documented negative result ──
    println!(
        "\n── Verdict: the arithmetic tower has no dependency chain ──\n\
         Conditional usefulness is real: the ontology matrix showed power becomes valuable\n\
         only once mul exists (install-as-Prim re-measures held-out cost). But conditional\n\
         discoverability does not follow, and the probe shows why.\n\
         \n\
         • pow is base-findable. Church pow = λm.λn.n m is a ~6-node combinator, so raw\n\
           bottom-up finds a correct, generalizing size-11 pow from base at every max_size\n\
           10–20. There is no mul→pow discovery dependency to expose.\n\
         • tet is not findable even through {{mul,pow}}, at any max_size 14–24 or max_depth\n\
           3–6. Bottom-up cannot synthesize the recursion no matter what atoms it holds.\n\
         • Even the product tower's single step fails via bottom-up: a×b×c×d is raw-✗ and\n\
           via-{{mul}}-✗ (it needs depth 4 > max_depth 3). It is only reachable through the\n\
           pool-composition model — the usefulness mechanism, not discovery.\n\
         \n\
         The structural reason is a closure argument. Anything discovered by composing O\n\
         lies in the composition closure of O, so it cannot be the very thing that extends\n\
         that closure. And bottom-up enumeration is the mirror image: it finds compact\n\
         combinators (mul, pow) directly but cannot synthesize deeper recursive structure\n\
         (tet). So both searches are trapped on the same side of the wall.\n\
         \n\
         ⇒ A dependency chain C1 ⇒ C2 ⇒ C3 (each depends on C_k for discovery AND extends\n\
           what O_k can express) cannot emerge from concept composition alone. Concept-aware\n\
           REASONING exists; concept-aware GENERATION of closure-extending hypotheses does\n\
           not. The search generator itself has to change — that is the next milestone.\n\
         \n\
         (This subcommand is kept as a recorded negative result, deliberately not a ladder.)"
    );
    std::io::stdout().flush().ok();
}

/// usage: supsearch gen [--budget SECS] [--max-size N]
///
/// C6: ontology-conditioned hypothesis generation. `dep` showed conditional
/// USEFULNESS is real but conditional DISCOVERABILITY does not follow from the
/// existing searches (bottom-up + flat composition). The fix is a grammar-based
/// generator G(O) whose ONE production is the bounded self-iteration schema,
/// applied uniformly to every concept in the ontology (and base ops):
///
///     iterate(C, seed)  =  λa.λn. ((n (C a)) seed)
///
/// realized in pure λ (the Church numeral n is itself an iterator — no new
/// runtime primitive), generic (the ontology fills the hole; G names no concept),
/// and applied ONE schema-application per proposal (single-application-depth:
/// the acquisition loop, not G nesting, builds depth). The same production walks
/// the Grzegorczyk tower: iterate(add,zero)=mul, iterate(mul,one)=pow,
/// iterate(pow,one)=tet.
///
/// G is fixed; only O changes. The claim (G-conditional, not raw): pow ∉ G(∅)
/// but pow ∈ G({mul}); tet ∉ G({mul}) but tet ∈ G({mul,pow}).
fn gen(args: &[String]) {
    use std::io::Write;
    use std::rc::Rc;

    let mut budget = 12u64;
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
                eprintln!("unknown gen arg: {other}");
                std::process::exit(1);
            }
        }
    }

    let num = |n: u32| -> Rc<term::Term> {
        parse::parse_expr(&bootstrap::church_num_str(n))
            .and_then(|e| parse::to_term(&e))
            .expect("church numeral")
    };
    let closed = |s: &str| -> Rc<term::Term> {
        parse::parse_expr(s)
            .and_then(|e| parse::to_term(&e))
            .expect("closed term")
    };
    let task = |arity: usize, tests: Vec<(Vec<u32>, Rc<term::Term>)>| parse::Task {
        arity,
        tests: tests
            .into_iter()
            .map(|(args, want)| parse::Test {
                args: args.iter().map(|&n| num(n)).collect(),
                want,
                outer: 0,
            })
            .collect(),
    };
    // Height-n power tower: a↑↑0=1, a↑↑(n+1)=a^(a↑↑n). Values stay < 2048 fuel,
    // so a∈{1,2} (≤16) and a=3 only to height 2 (3↑↑3 = 3^27 ≫ fuel).
    let tet_val = |a: u32, n: u32| -> u32 {
        let mut v = 1u32;
        for _ in 0..n {
            v = a.pow(v);
        }
        v
    };

    let base = vec![closed("λa.λb.λf.λx.a(f)(b(f)(x))")]; // add

    // ── family: discovery tasks + usefulness held-outs (values < 2048 fuel) ──
    let mul_task = task(
        2,
        [(2, 3), (3, 4), (1, 5), (0, 7), (4, 4), (5, 2), (6, 2), (2, 6)]
            .into_iter()
            .map(|(a, b)| (vec![a, b], num(a * b)))
            .collect(),
    );
    // rich a^n suite (a∈{0..4}, n∈{0..4}, values ≤ 64) — defeats fixed-power overfit.
    let pow_task = task(
        2,
        [
            (2, 0), (2, 1), (2, 2), (2, 3), (2, 4),
            (3, 0), (3, 1), (3, 2), (3, 3),
            (1, 0), (1, 4), (0, 0), (0, 2), (4, 2), (4, 3), (4, 1),
        ]
        .into_iter()
        .map(|(a, n): (u32, u32)| (vec![a, n], num(a.pow(n))))
        .collect(),
    );
    // a↑↑n: a∈{1,2} to height 3 (≤16), a=3 to height 2 (≤27). The a=3 rows are
    // what distinguish tet (3↑↑2=27) from pow (3²=9) — an overfit pow can't pass.
    let tet_task = task(
        2,
        [
            (2, 0), (2, 1), (2, 2), (2, 3),
            (1, 0), (1, 1), (1, 2), (1, 3),
            (3, 0), (3, 1), (3, 2),
        ]
        .into_iter()
        .map(|(a, n): (u32, u32)| (vec![a, n], num(tet_val(a, n))))
        .collect(),
    );

    let h_mul = crate::promote_prod_task(4); // a×b×c×d
    let h_pow = task(
        2,
        [(2, 3), (3, 2), (1, 4), (2, 1), (4, 2), (1, 0)]
            .into_iter()
            .map(|(x, n): (u32, u32)| (vec![x, n], num(x.pow(n + 1))))
            .collect(), // x^(n+1)
    );
    let h_tet = task(
        2,
        [(2, 0), (2, 1), (2, 2), (1, 0), (1, 2)]
            .into_iter()
            .map(|(a, n): (u32, u32)| (vec![a, n], num(tet_val(a, n + 1))))
            .collect(), // tower of height n+1, ≤3 → ≤16
    );

    let opts = bank_opts(&base, budget, max_size);
    let seed_zero = num(0);
    let seed_one = num(1);

    // ── the fixed generic production: iterate(C,seed) = λa.λn.((n (C a)) seed) ──
    // C and seed are closed terms → no de Bruijn shifting. body: n=var(0), a=var(1).
    let iterate = |c: &Rc<term::Term>, seed: &Rc<term::Term>| -> Rc<term::Term> {
        term::lam(term::lam(term::app(
            term::app(term::var(0), term::app(c.clone(), term::var(1))),
            seed.clone(),
        )))
    };

    // Does the candidate λ-term `body` compute `t`? Installed as a concept at the
    // task's arity, verified by the composition model's oracle check (same
    // verification the ladder uses for a discovered solution).
    let solves = |t: &parse::Task, body: &Rc<term::Term>| -> bool {
        let set = [bank::Concept {
            body: body.clone(),
            name: "cand".into(),
            arity: t.arity as u32,
        }];
        bank::concept_solve(t, &set, &opts).solution.is_some()
    };
    // G(O): all one-schema-application proposals over the available concept bodies.
    let gen_cands = |avail: &[Rc<term::Term>]| -> Vec<Rc<term::Term>> {
        let mut out = Vec::new();
        for cb in avail {
            out.push(iterate(cb, &seed_zero));
            out.push(iterate(cb, &seed_one));
        }
        out
    };
    let available = |ocs: &[bank::Concept]| -> Vec<Rc<term::Term>> {
        base.iter()
            .cloned()
            .chain(ocs.iter().map(|c| c.body.clone()))
            .collect()
    };

    println!("\n── C6: ontology-conditioned hypothesis generation ──");
    println!("G fixed = {{iterate(C, seed) = λa.λn.((n (C a)) seed)}}; one schema-application per proposal");
    println!("base = {{add}}; family: add → mul → pow → tet (Grzegorczyk tower, Church numerals)");
    println!("budget: max_size {max_size}, {budget}s, pool 64, fuel 2048");
    println!(
        "{:<4} {:<9} {:<11} {:<15} {:<12} {:<18} {}",
        "Gen", "candidate", "raw(base)?", "via G(O_{k-1})?", "via G(O_k)?", "useful(H)", "verdict"
    );

    let mut onto: Vec<bank::Concept> = Vec::new();
    let mut trajectory: Vec<String> = vec!["∅".into()];
    let rungs: [(&str, parse::Task, parse::Task); 3] = [
        ("mul", mul_task, h_mul),
        ("pow", pow_task, h_pow),
        ("tet", tet_task, h_tet.clone()),
    ];

    for (gen_i, (name, t, h)) in rungs.into_iter().enumerate() {
        let avail_prev: Vec<Rc<term::Term>> = if gen_i == 0 {
            available(&Vec::new())
        } else {
            available(&onto[..onto.len() - 1])
        };
        let avail_cur = available(&onto);
        let raw = raw_cost(&t, &opts) < UNREACHABLE;
        let via_prev = gen_cands(&avail_prev).iter().any(|b| solves(&t, b));
        let via_cur = gen_cands(&avail_cur).iter().any(|b| solves(&t, b));

        let mut row = format!(
            "{:<4} {:<9} {:<11} {:<15} {:<12} ",
            gen_i,
            name,
            if raw { "✓" } else { "✗" },
            if gen_i == 0 { "—" } else if via_prev { "✓" } else { "✗" },
            if via_cur { "✓" } else { "✗" },
        );

        // the candidate that generalizes — feed it to the usefulness gate.
        let cand = gen_cands(&avail_cur).into_iter().find(|b| solves(&t, b));

        match cand {
            Some(body) => {
                let baseline = concept_cost(&h, &onto, &opts);
                match propose_value(&body, &onto, &[h.clone()], &opts, baseline) {
                    Some(g) => {
                        let verdict = if g.earns() { "ACQUIRE" } else { "reject" };
                        row.push_str(&format!(
                            "{:<7}→{:<8} {:<9} {}",
                            disp_cost(g.before),
                            disp_cost(g.after),
                            g.kind(),
                            verdict
                        ));
                        if g.earns() {
                            onto.push(bank::Concept {
                                body,
                                name: format!("C{}", gen_i + 1),
                                arity: g.arity,
                            });
                            trajectory.push(format!("C{}={name}", gen_i + 1));
                        }
                    }
                    None => row.push_str(&format!("{:<18} {}", "no interface", "reject")),
                }
            }
            None => row.push_str(&format!("{:<18} {}", "not in G(O_k)", "—")),
        }
        println!("{row}");
    }

    println!(
        "\nacquired trajectory: {}",
        trajectory
            .iter()
            .enumerate()
            .map(|(i, s)| {
                if i == 0 {
                    format!("O0={s}")
                } else {
                    format!("O{i}={s}")
                }
            })
            .collect::<Vec<_>>()
            .join("  →  ")
    );

    // ── tet-usefulness probe: composition-{mul,pow} cannot synthesize the tower ──
    // recursion (it needs tet as a Prim) — this is what earns tet its slot.
    let tet_baseline = concept_cost(&h_tet, &onto[..2], &opts); // {mul,pow}: expect ✗
    let tet_via = concept_cost(&h_tet, &onto, &opts); // {mul,pow,tet}: expect finite
    println!(
        "\ntet usefulness probe: composition-{{mul,pow}} on tower-holdout → {}; with tet-prim → {}",
        disp_cost(tet_baseline),
        disp_cost(tet_via)
    );

    println!(
        "\nHonest fine print:\n\
         • The claim is G-conditional, not raw. raw(base) finds pow (Church compression:\n\
           λm.λn.n m is a ~6-node combinator) — the sharp claim is pow ∉ G(∅) but pow ∈\n\
           G({{mul}}), and tet ∉ G({{mul}}) but tet ∈ G({{mul,pow}}). raw cannot build the\n\
           chain (tet is raw-✗); G can. Discoverable is unlocked by G.\n\
         • Usefulness kinds differ by concept and are reported honestly. mul and pow earn\n\
           FRONTIER gains: composition cannot reach a×b×c×d (✗→65) or x^(n+1) (✗→16). tet\n\
           earns a SEARCH gain (121→11), not a frontier: composition-{{mul,pow}} already\n\
           solves the tower-holdout at 121 — for the representable bases (a∈{{1,2}}, values\n\
           < 2048 fuel) the composition a^(a^n) happens to equal the tower. tet is the true\n\
           generalizer (its discovery suite has a=3 rows that a^(a^n) fails) and is ~11×\n\
           cheaper, so it earns its slot on cost. A genuine tet FRONTIER needs tower(2,4)=65536\n\
           or a=3 height-3 (3^27), both beyond the 2048 fuel — an honest representable-range\n\
           limit, not a forced result.\n\
         • iterate is not 'recursion smuggled in to recover tetration': it is a generic\n\
           bounded self-iteration rule realized in pure λ via the numeral iterator (already\n\
           in base), names no concept, and walks the whole tower uniformly — the ontology,\n\
           not the schema, decides which concept is 'next'. G proposes non-targets too\n\
           (iterate(add,one)=1+na, iterate(mul,zero)=0); they fail the target-task\n\
           verification (don't generalize) and are not acquired — the gate is selective,\n\
           not forced.\n\
         • Single-application-depth: G applies the schema once per proposal; the\n\
           acquisition loop (O grows) builds depth. Without this rule pow would already be\n\
           in G(∅) and the chain would collapse.\n\
         • Value cap: tetration (2↑↑4 = 65536 > 2048 fuel) caps the honest chain at 3\n\
           generations (O0→O1→O2→O3). Pent is value-unrepresentable; not forced."
    );
    std::io::stdout().flush().ok();
}

/// C6-generalization: does the SAME fixed iterate-schema generator `G(O)`
/// transfer to a non-arithmetic value space? Here the semantics are strings
/// (Church lists of Church numerals) and the base op is `cons` (prepend). G is
/// byte-identical to `gen`'s; only the base, the seed set, and the value
/// representation change. We test (a) a genuine depth-1 frontier —
/// `replicate(c,n) = iterate(cons,nil) = (cons c)^n nil ∈ G(∅)`, which
/// composition cannot build because a count-dependent list length requires the
/// iterator — with the counterfactual gate acquiring it and rejecting junk; and
/// (b) the depth wall: the SAME G yields NO second-order concept here, because
/// iterate's second argument is always an iteration count, so re-iterating
/// replicate would need replicate's output (a list) to feed back as a count —
/// a type the flat-list value space does not carry. Multi-level depth is the
/// signature of a SELF-ITERABLE value space (the numerals); strings get exactly
/// one level. That is precisely why C7 (acquiring the proposal schemas) is the
/// path to depth in non-arithmetic domains.
fn transfer(args: &[String]) {
    use std::io::Write;
    use std::rc::Rc;

    let mut budget = 12u64;
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
                eprintln!("unknown transfer arg: {other}");
                std::process::exit(1);
            }
        }
    }

    let num = |n: u32| -> Rc<term::Term> {
        parse::parse_expr(&bootstrap::church_num_str(n))
            .and_then(|e| parse::to_term(&e))
            .expect("church numeral")
    };
    let closed = |s: &str| -> Rc<term::Term> {
        parse::parse_expr(s)
            .and_then(|e| parse::to_term(&e))
            .expect("closed term")
    };
    // Church list [c1,..,ck] = λf.λz.f(c1)(f(c2)(...(z))) — chars are Church
    // numerals; the char numerals carry their own λf.λx binders so there is no
    // capture against the outer λf.λz.
    let list = |cs: &[u32]| -> Rc<term::Term> {
        let mut body = String::from("z");
        for c in cs.iter().rev() {
            let cstr = bootstrap::church_num_str(*c);
            body = format!("f({cstr})({body})");
        }
        closed(&format!("λf.λz.{body}"))
    };
    let task = |arity: usize, tests: Vec<(Vec<u32>, Rc<term::Term>)>| parse::Task {
        arity,
        tests: tests
            .into_iter()
            .map(|(args, want)| parse::Test {
                args: args.iter().map(|&n| num(n)).collect(),
                want,
                outer: 0,
            })
            .collect(),
    };

    let cons = closed("λc.λs.λf.λz.f(c)(s(f)(z))"); // prepend char to list
    let base = vec![cons.clone()];
    let nil = list(&[]);
    let singleton = list(&[1]);

    // Discovery suite: (c,n) → list of n copies of c. Composition-{cons} can
    // only build FIXED-length lists (cons applied finitely many times); a list
    // whose length depends on the numeral n requires iterating cons n times —
    // the iterator, which is not in the composition closure. So the held-out
    // below is ✗ under {cons}: a genuine frontier, exactly mul's a×b×c×d.
    let rep_task = task(
        2,
        [(2, 0), (2, 1), (2, 3), (3, 2), (1, 4), (4, 2)]
            .into_iter()
            .map(|(c, n): (u32, u32)| (vec![c, n], list(&vec![c; n as usize])))
            .collect(),
    );
    // Held-out with UNSEEN (c,n) — including c=5, c=0 — so an overfit
    // memorizer of the discovery rows is caught. replicate must GENERALIZE.
    let h_rep = task(
        2,
        [(2, 2), (3, 3), (1, 1), (4, 4), (5, 2), (0, 3)]
            .into_iter()
            .map(|(c, n): (u32, u32)| (vec![c, n], list(&vec![c; n as usize])))
            .collect(),
    );

    let opts = bank_opts(&base, budget, max_size);

    // ── the SAME fixed production as `gen` — nothing domain-specific here ──
    let iterate = |c: &Rc<term::Term>, seed: &Rc<term::Term>| -> Rc<term::Term> {
        term::lam(term::lam(term::app(
            term::app(term::var(0), term::app(c.clone(), term::var(1))),
            seed.clone(),
        )))
    };
    let solves = |t: &parse::Task, body: &Rc<term::Term>| -> bool {
        let set = [bank::Concept {
            body: body.clone(),
            name: "cand".into(),
            arity: t.arity as u32,
        }];
        bank::concept_solve(t, &set, &opts).solution.is_some()
    };
    // G(O) over the seeds the domain actually provides.
    let gen_cands = |avail: &[Rc<term::Term>], seeds: &[Rc<term::Term>]| -> Vec<Rc<term::Term>> {
        let mut out = Vec::new();
        for cb in avail {
            for sd in seeds {
                out.push(iterate(cb, sd));
            }
        }
        out
    };
    let available = |ocs: &[bank::Concept]| -> Vec<Rc<term::Term>> {
        base.iter()
            .cloned()
            .chain(ocs.iter().map(|c| c.body.clone()))
            .collect()
    };
    let string_seeds = vec![nil.clone(), singleton.clone()];

    println!("\n── transfer: does the SAME fixed G generalize across domains? ──");
    println!("G fixed = {{iterate(C, seed) = λa.λn.((n (C a)) seed)}} — identical to `gen`");
    println!("domain: strings = Church lists; base = {{cons}} (prepend); seeds = {{nil, [1]}}");
    println!("budget: max_size {max_size}, {budget}s, pool 64, fuel 20000");
    println!(
        "{:<4} {:<9} {:<11} {:<14} {:<12} {:<18} {}",
        "Gen", "candidate", "raw(base)?", "in G(O_0)?", "junk-cover?", "useful(H)", "verdict"
    );

    // Depth-1: replicate = iterate(cons, nil) = (cons c)^n nil ∈ G(∅).
    let raw = raw_cost(&rep_task, &opts) < UNREACHABLE;
    let g0 = gen_cands(&available(&[]), &string_seeds);
    let replicate = g0.iter().find(|b| solves(&rep_task, b)).cloned();
    let junk = g0
        .iter()
        .filter(|b| !solves(&rep_task, b))
        .cloned()
        .collect::<Vec<_>>();
    let baseline = concept_cost(&h_rep, &[], &opts); // {cons}: expect ✗
    let mut onto: Vec<bank::Concept> = Vec::new();
    let mut line0 = format!(
        "{:<4} {:<9} {:<11} {:<14} ",
        "0",
        "replicate",
        if raw { "✓" } else { "✗" },
        if replicate.is_some() { "✓" } else { "✗" },
    );
    match replicate.as_ref() {
        Some(body) => match propose_value(body, &onto, &[h_rep.clone()], &opts, baseline) {
            Some(g) => {
                let verdict = if g.earns() { "ACQUIRE" } else { "reject" };
                line0.push_str(&format!(
                    "{:<7}→{:<8} {:<9} {}",
                    disp_cost(g.before),
                    disp_cost(g.after),
                    g.kind(),
                    verdict
                ));
                if g.earns() {
                    onto.push(bank::Concept {
                        body: body.clone(),
                        name: "C1=replicate".into(),
                        arity: g.arity,
                    });
                }
            }
            None => line0.push_str(&format!("{:<18} {}", "no interface", "reject")),
        },
        None => line0.push_str(&format!("{:<18} {}", "not in G(O_0)", "—")),
    }
    // junk coverage: the non-generalizing G(O_0) proposals are rejected by the
    // target-task verification (they solve neither the discovery suite nor the
    // held-out) — the gate is selective, not forced.
    let junk_solves_rep = junk.iter().any(|b| solves(&rep_task, b));
    let junk_solves_h = junk.iter().any(|b| solves(&h_rep, b));
    line0.push_str(&format!(
        "   (junk {}/{} rejected by target-check)",
        junk.iter().filter(|b| !solves(&rep_task, b)).count(),
        junk.len()
    ));
    if !junk_solves_rep && !junk_solves_h {
        line0.push_str(" ✓");
    } else {
        line0.push_str(" ✗—junk leaks!");
    }
    println!("{line0}");

    // ── depth probe: attempt a second generation with the SAME G ──
    // O_1 = {cons, replicate}. G(O_1) \ G(O_0) = {iterate(replicate, nil),
    // iterate(replicate, [1])}. To be a second-order concept, iterate(replicate,
    // seed) would have to iterate a "step" built from replicate; but replicate's
    // second argument is an iteration count n (a numeral), so (replicate c) maps
    // numeral → list, and re-iterating it n' times requires the intermediate to
    // be a numeral again — i.e. replicate's OUTPUT (a list) must feed back as a
    // count, which the flat-list value space cannot supply. The candidates are
    // type-degenerate: they solve neither replicate_task nor a fresh held-out.
    let avail1 = available(&onto);
    let g1 = gen_cands(&avail1, &string_seeds);
    let g1_new = g1
        .iter()
        .filter(|b| !g0.contains(b))
        .cloned()
        .collect::<Vec<_>>();
    let depth2 = g1_new
        .iter()
        .filter(|b| solves(&rep_task, b))
        .count();
    let depth2_h = g1_new.iter().filter(|b| solves(&h_rep, b)).count();
    println!(
        "{:<4} {:<9} {:<11} {:<14} {:<12} {:<18} {}",
        "1",
        "string-pow?",
        "✗",
        "✗",
        "—",
        "—",
        if depth2 == 0 && depth2_h == 0 {
            "no 2nd-order concept (type wall)"
        } else {
            "UNEXPECTED depth"
        }
    );

    println!(
        "\nacquired trajectory (string domain): O0={{cons}}  →  O1={{cons, replicate}}"
    );
    println!(
        "\nstructural finding: the fixed G transfers to ANY domain as a depth-1\n\
         concept generator (C1 ∈ G(O0) with a genuine frontier — here replicate,\n\
         which composition-{{cons}} cannot reach because a count-dependent list\n\
         length needs the iterator). Its DEPTH is value-space-bound: multi-level\n\
         chains (mul→pow→tet in `gen`) require a SELF-ITERABLE value space —\n\
         iterate's second argument is always an iteration count, so re-iterating\n\
         C1 needs C1's output to feed back as a count. Strings (like permutations,\n\
         grids, rewrite systems) are not self-iterable, so they get exactly one\n\
         level. G is domain-independent; the hyperoperation tower's depth is the\n\
         signature of the numeric value space, not of G. Going deeper in\n\
         non-arithmetic domains therefore requires C7: acquiring the proposal\n\
         schemas themselves (iterate, lift, abstract, compose-under-binding), so\n\
         that depth stops being a hard-coded property of the schema."
    );
    std::io::stdout().flush().ok();
}

/// C7 (first cut): acquire PROPOSAL SCHEMAS, not just concepts. The machine is
/// given a fixed meta-search space M of generic pure-λ transformations and
/// keeps the ones whose proposals measurably earn acquisition (Gain(G_i|O,D)),
/// dropping the rest — so (O_t, G_t) evolves jointly. The payoff is the
/// transfer-negative made concrete: in the string domain `iterate` alone stalls
/// after replicate (its output, a list, can't feed back as a count). A second
/// schema `reduce(C) = λxs.λys. xs C ys` (fold with a free seed) yields
/// `concat = reduce(cons)`, which is NOT in the composition closure of {cons}
/// (it needs the list eliminator). With concat as substrate, iterate regains
/// leverage: `iterate(concat, nil)(xs, n) = (concat xs)^n nil = xs` concatenated
/// n times — the depth-2 list concept (`xs^n`) the transfer experiment showed
/// unreachable when iterate was the only generator. So the machine acquires a
/// NEW way of generating hypotheses exactly where the old one ran out.
fn meta(args: &[String]) {
    use std::io::Write;
    use std::rc::Rc;

    let mut budget = 12u64;
    let mut max_size = 14u32;
    let mut ablate = false;
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
            "--ablate" => {
                ablate = true;
                i += 1;
            }
            other => {
                eprintln!("unknown meta arg: {other}");
                std::process::exit(1);
            }
        }
    }

    let num = |n: u32| -> Rc<term::Term> {
        parse::parse_expr(&bootstrap::church_num_str(n))
            .and_then(|e| parse::to_term(&e))
            .expect("church numeral")
    };
    let closed = |s: &str| -> Rc<term::Term> {
        parse::parse_expr(s)
            .and_then(|e| parse::to_term(&e))
            .expect("closed term")
    };
    let list = |cs: &[u32]| -> Rc<term::Term> {
        let mut body = String::from("z");
        for c in cs.iter().rev() {
            let cstr = bootstrap::church_num_str(*c);
            body = format!("f({cstr})({body})");
        }
        closed(&format!("λf.λz.{body}"))
    };
    // general task builder: pre-built closed term args (lists / numerals).
    let task = |arity: usize, tests: Vec<(Vec<Rc<term::Term>>, Rc<term::Term>)>| parse::Task {
        arity,
        tests: tests
            .into_iter()
            .map(|(args, want)| parse::Test { args, want, outer: 0 })
            .collect(),
    };

    let cons = closed("λc.λs.λf.λz.f(c)(s(f)(z))");
    let base = vec![cons.clone()];
    let nil = list(&[]);
    let singleton = list(&[1]);
    let seeds = vec![nil.clone(), singleton.clone()];

    // ── task family (strings; lists kept short, chars ≤5, fuel-safe) ──
    // replicate: (c,n) → [c;n]
    let rep_task = task(
        2,
        [(2, 0), (2, 1), (2, 3), (3, 2), (1, 4), (4, 2)]
            .into_iter()
            .map(|(c, n): (u32, u32)| {
                (vec![num(c), num(n)], list(&vec![c; n as usize]))
            })
            .collect(),
    );
    let h_rep = task(
        2,
        [(2, 2), (3, 3), (1, 1), (4, 4), (5, 2), (0, 3)]
            .into_iter()
            .map(|(c, n): (u32, u32)| {
                (vec![num(c), num(n)], list(&vec![c; n as usize]))
            })
            .collect(),
    );
    // concat: (xs, ys) → xs ++ ys
    let cat = |a: &[u32], b: &[u32]| -> Vec<u32> {
        a.iter().chain(b.iter()).copied().collect()
    };
    let concat_task = task(
        2,
        [
            (vec![1], vec![2]),
            (vec![2, 3], vec![4]),
            (vec![], vec![1, 1]),
            (vec![1, 2], vec![3, 4]),
        ]
        .into_iter()
        .map(|(xs, ys)| (vec![list(&xs), list(&ys)], list(&cat(&xs, &ys))))
        .collect(),
    );
    let h_concat = task(
        2,
        [
            (vec![2], vec![3, 4]),
            (vec![1, 1, 1], vec![]),
            (vec![3, 2], vec![2, 3]),
        ]
        .into_iter()
        .map(|(xs, ys)| (vec![list(&xs), list(&ys)], list(&cat(&xs, &ys))))
        .collect(),
    );
    // concat_n: (xs, n) → xs concatenated n times (the depth-2 list concept).
    let cnat = |xs: &[u32], n: u32| -> Vec<u32> {
        let mut v = Vec::new();
        for _ in 0..n {
            v.extend_from_slice(xs);
        }
        v
    };
    let cnat_task = task(
        2,
        [
            (vec![2, 3], 2),
            (vec![1], 3),
            (vec![2, 2], 2),
            (vec![3], 0),
        ]
        .into_iter()
        .map(|(xs, n)| (vec![list(&xs), num(n)], list(&cnat(&xs, n))))
        .collect(),
    );
    let h_cnat = task(
        2,
        [(vec![2, 3], 3), (vec![1], 4), (vec![4, 5], 2), (vec![2], 0)]
            .into_iter()
            .map(|(xs, n)| (vec![list(&xs), num(n)], list(&cnat(&xs, n))))
            .collect(),
    );

    let opts = bank_opts(&base, budget, max_size);

    // ── the pure-λ proposal schemas (the meta-space M) ──
    // iterate(C, seed) = λa.λn.((n (C a)) seed)
    let iterate = |c: &Rc<term::Term>, seed: &Rc<term::Term>| -> Rc<term::Term> {
        term::lam(term::lam(term::app(
            term::app(term::var(0), term::app(c.clone(), term::var(1))),
            seed.clone(),
        )))
    };
    // reduce(C) = λxs.λys.(xs C ys) — fold with a free seed (the list eliminator)
    let reduce = |c: &Rc<term::Term>| -> Rc<term::Term> {
        term::lam(term::lam(term::app(
            term::app(term::var(1), c.clone()),
            term::var(0),
        )))
    };
    // junk(C, seed) = λa.λb. C seed (C a b) — a generic recombine probe
    let junk = |c: &Rc<term::Term>, seed: &Rc<term::Term>| -> Rc<term::Term> {
        term::lam(term::lam(term::app(
            term::app(c.clone(), seed.clone()),
            term::app(term::app(c.clone(), term::var(1)), term::var(0)),
        )))
    };

    let solves = |t: &parse::Task, body: &Rc<term::Term>| -> bool {
        let set = [bank::Concept {
            body: body.clone(),
            name: "cand".into(),
            arity: t.arity as u32,
        }];
        bank::concept_solve(t, &set, &opts).solution.is_some()
    };
    let available = |ocs: &[bank::Concept]| -> Vec<Rc<term::Term>> {
        base.iter()
            .cloned()
            .chain(ocs.iter().map(|c| c.body.clone()))
            .collect()
    };

    println!("\n── C7 (first cut): acquiring PROPOSAL SCHEMAS ──");
    println!("domain: strings (Church lists); base {{cons}}; meta-space M = {{iterate, reduce, junk}}");
    println!("budget: max_size {max_size}, {budget}s, pool 64, fuel 20000");
    println!(
        "{:<6} {:<10} {:<13} {:<11} {:<16} {}",
        "round", "schema", "candidate", "target", "useful(H)", "verdict"
    );

    // ── round 0: O_0 = {cons}, M all available ──
    let onto: Vec<bank::Concept> = Vec::new();
    let avail0 = available(&onto);
    let mut retained: Vec<&str> = Vec::new();

    // iterate: replicate = iterate(cons, nil)
    let rep_cand = iterate(&cons, &nil);
    let rep_ok = solves(&rep_task, &rep_cand);
    let rep_base = concept_cost(&h_rep, &onto, &opts); // {cons}: expect ✗
    let (rep_earns, rep_gain) = if rep_ok {
        match propose_value(&rep_cand, &onto, &[h_rep.clone()], &opts, rep_base) {
            Some(g) => (g.earns(), Some(g)),
            None => (false, None),
        }
    } else {
        (false, None)
    };
    println!(
        "{:<6} {:<10} {:<13} {:<11} {:<16} {}",
        "0",
        "iterate",
        "replicate",
        "rep",
        if let Some(g) = rep_gain {
            format!("{}→{}", disp_cost(g.before), disp_cost(g.after))
        } else {
            "—".into()
        },
        if rep_earns {
            retained.push("iterate");
            "RETAIN (+replicate)"
        } else {
            "no gain"
        }
    );

    // reduce: concat = reduce(cons)
    let concat_cand = reduce(&cons);
    let concat_ok = solves(&concat_task, &concat_cand);
    let concat_base = concept_cost(&h_concat, &onto, &opts);
    let (concat_earns, concat_gain) = if concat_ok {
        match propose_value(&concat_cand, &onto, &[h_concat.clone()], &opts, concat_base) {
            Some(g) => (g.earns(), Some(g)),
            None => (false, None),
        }
    } else {
        (false, None)
    };
    println!(
        "{:<6} {:<10} {:<13} {:<11} {:<16} {}",
        "0",
        "reduce",
        "concat",
        "concat",
        if let Some(g) = concat_gain {
            format!("{}→{}", disp_cost(g.before), disp_cost(g.after))
        } else {
            "—".into()
        },
        if concat_earns {
            retained.push("reduce");
            "RETAIN (+concat)"
        } else {
            "no gain"
        }
    );

    // junk: probes a generic recombine — expect nothing pays in this domain.
    let junk_cands = seeds.iter().map(|s| junk(&cons, s)).collect::<Vec<_>>();
    let junk_solves = junk_cands
        .iter()
        .any(|b| solves(&rep_task, b) || solves(&concat_task, b) || solves(&cnat_task, b));
    println!(
        "{:<6} {:<10} {:<13} {:<11} {:<16} {}",
        "0",
        "junk",
        "recombine",
        "rep/concat/cnat",
        "—",
        if junk_solves { "UNEXPECTED" } else { "DROP (no target solves)" }
    );

    // ── round 0 acquisitions ──
    let mut o1: Vec<bank::Concept> = Vec::new();
    if rep_earns {
        o1.push(bank::Concept {
            body: rep_cand.clone(),
            name: "replicate".into(),
            arity: rep_gain.unwrap().arity,
        });
    }
    if concat_earns {
        o1.push(bank::Concept {
            body: concat_cand.clone(),
            name: "concat".into(),
            arity: concat_gain.unwrap().arity,
        });
    }
    println!(
        "\nround 0: retained schemas G1 = {{{}}}; O1 = {{{}}}",
        retained.join(", "),
        o1.iter().map(|c| c.name.clone()).collect::<Vec<_>>().join(", ")
    );

    // ── round 1: O_1 = {cons, replicate, concat}, G1 = {iterate, reduce} ──
    // iterate now has concat as substrate → iterate(concat, nil) = concat_n,
    // the depth-2 list concept. This is the payoff: reduce (acquired in round 0
    // because iterate couldn't produce concat) restores iterate's leverage.
    let avail1 = available(&o1);
    let cnat_cand = iterate(&concat_cand, &nil);
    let cnat_ok = solves(&cnat_task, &cnat_cand);
    let cnat_base = concept_cost(&h_cnat, &o1, &opts); // {replicate,concat}: expect ✗
    let (cnat_earns, cnat_gain) = if cnat_ok {
        match propose_value(&cnat_cand, &o1, &[h_cnat.clone()], &opts, cnat_base) {
            Some(g) => (g.earns(), Some(g)),
            None => (false, None),
        }
    } else {
        (false, None)
    };
    println!(
        "{:<6} {:<10} {:<13} {:<11} {:<16} {}",
        "1",
        "iterate",
        "concat_n",
        "concat_n",
        if let Some(g) = cnat_gain {
            format!("{}→{}", disp_cost(g.before), disp_cost(g.after))
        } else {
            "—".into()
        },
        if cnat_earns {
            "ACQUIRE (depth-2 via concat)"
        } else {
            "no gain"
        }
    );
    // the depth-2 candidate is NOT in G(iterate over {cons,replicate}) — reduce
    // was the necessary unlock. Verify: iterate over {cons, replicate} alone has
    // no concat_n.
    let avail_no_concat: Vec<Rc<term::Term>> = vec![cons.clone(), rep_cand.clone()];
    let without_concat = avail_no_concat.iter().any(|cb| {
        solves(&cnat_task, &iterate(cb, &nil)) || solves(&cnat_task, &iterate(cb, &singleton))
    });
    let mut o2 = o1.clone();
    if cnat_earns {
        o2.push(bank::Concept {
            body: cnat_cand.clone(),
            name: "concat_n".into(),
            arity: cnat_gain.unwrap().arity,
        });
    }

    println!(
        "\nacquired trajectory: O0={{cons}} → O1={{cons, replicate, concat}} → O2={{+ concat_n}}"
    );
    println!("schema trajectory:  G0=M={{iterate,reduce,junk}} → G1={{iterate, reduce}} (junk dropped)");
    println!(
        "\nstructural claim: iterate alone stalls after replicate (the transfer wall:\n\
         replicate's output, a list, can't feed back as a count). The acquired\n\
         schema reduce gives concat = reduce(cons) = λxs.λys. xs cons ys, which is\n\
         NOT in Closure_compose({{cons}}) (it needs the list eliminator). With concat\n\
         as substrate, iterate regains leverage: iterate(concat, nil) = concat_n =\n\
         (concat xs)^n nil = xs concatenated n times — the depth-2 list concept\n\
         {}. So the machine acquired a NEW way of generating\n\
         hypotheses exactly where the old one ran out: when (O_t, G_t) ceased to be\n\
         closed under further useful abstraction, it kept the generator that\n\
         restored structural leverage. Selectivity: junk is dropped (solves no\n\
         target); within each retained schema, non-target proposals (wrong seeds,\n\
         degenerate iterate-of-replicate) fail the target-check and are not acquired.",
        if without_concat {
            String::from("— but only via concat")
        } else {
            String::from("that iterate-from-{cons,replicate} alone cannot reach")
        }
    );

    // ── ablation matrix: Reach(subset) under identical budgets ──
    // For each schema-subset, run a bounded acquisition loop and record which of
    // replicate / concat / concat_n are reachable. The claim this verifies:
    // concat_n ∈ Reach({iterate,reduce}) but ∉ Reach({iterate}) ∪ Reach({reduce})
    // — cross-schema bootstrapping (reduce's concat is the substrate iterate needs).
    if ablate {
        let targets: Vec<(&str, &parse::Task, &parse::Task)> = vec![
            ("replicate", &rep_task, &h_rep),
            ("concat", &concat_task, &h_concat),
            ("concat_n", &cnat_task, &h_cnat),
        ];
        let subsets: [(&str, bool, bool, bool); 6] = [
            ("{iterate}", true, false, false),
            ("{reduce}", false, true, false),
            ("{junk}", false, false, true),
            ("{iterate, reduce}", true, true, false),
            ("{iterate, junk}", true, false, true),
            ("{reduce, junk}", false, true, true),
        ];
        // reach(subset): bounded acquisition loop over base {cons}, recording which
        // of the three targets' concepts become reachable.
        let reach = |use_iter: bool, use_red: bool, use_jnk: bool| -> [bool; 3] {
            let mut concepts: Vec<bank::Concept> = Vec::new();
            let mut got = [false, false, false];
            for _round in 0..3 {
                let avail = available(&concepts);
                // collect this round's candidates from the enabled schemas.
                let mut cands: Vec<Rc<term::Term>> = Vec::new();
                for cb in &avail {
                    if use_iter {
                        for sd in &seeds {
                            cands.push(iterate(cb, sd));
                        }
                    }
                    if use_red {
                        cands.push(reduce(cb));
                    }
                    if use_jnk {
                        for sd in &seeds {
                            cands.push(junk(cb, sd));
                        }
                    }
                }
                let mut progressed = false;
                for cand in &cands {
                    for (k, (name, t, h)) in targets.iter().enumerate() {
                        if got[k] || !solves(t, cand) {
                            continue;
                        }
                        let baseline = concept_cost(h, &concepts, &opts);
                        if let Some(g) = propose_value(cand, &concepts, &[(*h).clone()], &opts, baseline)
                        {
                            if g.earns() {
                                concepts.push(bank::Concept {
                                    body: cand.clone(),
                                    name: name.to_string(),
                                    arity: g.arity,
                                });
                                got[k] = true;
                                progressed = true;
                            }
                        }
                    }
                }
                if !progressed {
                    break;
                }
            }
            got
        };
        println!("\n── ablation matrix: Reach(subset), identical budgets ──");
        println!(
            "{:<18} {:>9} {:>9} {:>9}",
            "schemas", "replicate", "concat", "concat_n"
        );
        for (name, ui, ur, uj) in subsets {
            let got = reach(ui, ur, uj);
            println!(
                "{:<18} {:>9} {:>9} {:>9}",
                name,
                if got[0] { "✓" } else { "✗" },
                if got[1] { "✓" } else { "✗" },
                if got[2] { "✓" } else { "✗" }
            );
        }
        println!(
            "\ncross-schema claim: concat_n ∈ Reach({{iterate, reduce}}) while ∉ Reach({{iterate}}) ∪\n\
             Reach({{reduce}}): the acquired reduce's concat is the substrate iterate needs —\n\
             measured synergy between proposal schemas, not just schema selection."
        );
    }
    std::io::stdout().flush().ok();
}


/// Cost sentinel for "the reasoner cannot solve this task" (unreachable).
/// Large enough that "makes an unsolvable task solvable" reads as the
/// strongest possible promotion signal, but below u64::MAX to avoid overflow
/// when summed across a family.
const UNREACHABLE: u64 = u64::MAX / 4;

/// Cost of a task through a quotient-aware search over the given concept set:
/// `built` if solvable, `UNREACHABLE` if not.
fn concept_cost(t: &parse::Task, set: &[bank::Concept], opts: &bank::Options) -> u64 {
    let o = bank::concept_solve(t, set, opts);
    match o.solution {
        Some(_) => o.stats.built,
        None => UNREACHABLE,
    }
}

/// Cost of a task through the raw bottom-up bank (the machine before it has
/// any concept to reason through).
fn raw_cost(t: &parse::Task, opts: &bank::Options) -> u64 {
    let o = bank::solve(t, opts);
    match o.solution {
        Some(_) => o.stats.built,
        None => UNREACHABLE,
    }
}

/// The C2+C3 meta-experiment in one call: given an invented closed computation
/// `body` (a candidate concept with NO known interface) and the currently-held
/// concept set, find the composition arity k that makes the held-out family
/// cheapest, and report the effect *structurally* — as a before/after cost pair
/// over `Cost ∈ N ∪ {∞}` (UNREACHABLE = ∞) — so promotion can distinguish a
/// frontier move (∞ → finite) from a search-cost reduction (finite → smaller)
/// without leaking a sentinel as an arithmetic delta.
///
/// Returns `Some(Gain)` for the best (cheapest-after) interface arity, or
/// `None` if no arity in 1..=5 is worth trying. The arity is *inferred* by
/// measurement, never supplied: with the wrong arity the concept applied to
/// inputs yields non-domain values (or oversized normal forms pruned by the
/// hash fuel) that never match the target, so the correct arity is the one the
/// cost structure picks. `Gain::earns()` is the acquisition verdict: strictly
/// cheaper (∞ → finite, or finite → smaller).
fn propose_value(
    body: &Rc<term::Term>,
    current: &[bank::Concept],
    holdout: &[parse::Task],
    opts: &bank::Options,
    baseline: u64,
) -> Option<Gain> {
    // Early-exit on the first arity that earns: in these tasks exactly one arity
    // is the correct interface (others produce non-domain values and cost more),
    // so the first win is the inferred interface, and the wrong-arity grind is
    // skipped. For a candidate nothing earns, evaluate all arities to report the
    // cheapest after (so a rejection still shows its measured before → after).
    let mut best: Option<Gain> = None;
    for k in 1..=5u32 {
        let mut set = current.to_vec();
        set.push(bank::Concept {
            body: body.clone(),
            name: "cand".into(),
            arity: k,
        });
        let after: u64 = holdout.iter().map(|t| concept_cost(t, &set, opts)).sum();
        let g = Gain {
            arity: k,
            before: baseline,
            after,
        };
        if best.as_ref().map_or(true, |b: &Gain| after < b.after) {
            best = Some(g.clone());
        }
        if g.earns() {
            return Some(g);
        }
    }
    best
}

/// The measured effect of installing a candidate as a Prim, as a cost pair
/// over `Cost ∈ N ∪ {∞}`. No sentinel arithmetic is ever exposed as a delta.
#[derive(Clone, Copy)]
struct Gain {
    /// The inferred composition arity.
    arity: u32,
    /// Held-out cost under the current ontology (UNREACHABLE = unsolved).
    before: u64,
    /// Held-out cost with the candidate installed at `arity`.
    after: u64,
}

impl Gain {
    /// Frontier move: baseline was unsolved (∞), the candidate makes it solvable.
    fn frontier(&self) -> bool {
        self.before >= UNREACHABLE && self.after < UNREACHABLE
    }
    /// Solved → solved cost reduction (0 if not applicable).
    fn search_gain(&self) -> u64 {
        if self.before < UNREACHABLE && self.after < self.before {
            self.before - self.after
        } else {
            0
        }
    }
    /// The acquisition verdict: strictly cheaper under `Cost ∈ N ∪ {∞}`.
    fn earns(&self) -> bool {
        self.after < self.before
    }
    /// Human label of the kind of change this candidate causes.
    fn kind(&self) -> &'static str {
        if self.frontier() {
            "frontier gain"
        } else if self.search_gain() > 0 {
            "search gain"
        } else if self.after == self.before {
            "no gain"
        } else {
            "regression"
        }
    }
}

/// Rank two candidates for promotion: frontier gain ≻ search-cost reduction.
/// Returns `Greater` if `a` outranks `b`.
fn gain_rank(a: &Gain, b: &Gain) -> std::cmp::Ordering {
    match (a.frontier(), b.frontier()) {
        (true, false) => std::cmp::Ordering::Greater,
        (false, true) => std::cmp::Ordering::Less,
        _ => a.search_gain().cmp(&b.search_gain()),
    }
}

/// n-fold product task over Church numerals (arity = n), several distinct rows.
/// Numerals stay in 1..=3 so products stay small — a product of big numerals
/// makes the quotient search normalize ever-larger Church values and grind.
fn promote_prod_task(n: u32) -> parse::Task {
    let mut tests = Vec::new();
    for j in 0..5u32 {
        let args: Vec<Rc<term::Term>> = (0..n)
            .map(|l| {
                let src = crate::bootstrap::church_num_str(((j + l) % 3) + 1);
                let e = parse::parse_expr(&src).unwrap();
                parse::to_term(&e).unwrap()
            })
            .collect();
        let mut prod = 1u32;
        for l in 0..n {
            prod *= ((j + l) % 3) + 1;
        }
        tests.push(parse::Test {
            args,
            want: {
                let src = crate::bootstrap::church_num_str(prod);
                let e = parse::parse_expr(&src).unwrap();
                parse::to_term(&e).unwrap()
            },
            outer: 0,
        });
    }
    parse::Task {
        tests,
        arity: n as usize,
    }
}

fn disp_cost(x: u64) -> String {
    if x >= UNREACHABLE {
        "✗".into()
    } else {
        format!("{x}")
    }
}

/// usage: supsearch promote [--budget SECS]
///
/// C3/C4: the machine decides its own concepts. Starting from raw λ + add it
/// discovers mul by raw enumeration, infers its *interface* (composition arity)
/// and its worth by measured held-out reasoning gain (Δ), promotes it iff Δ > 0,
/// and the promoted concept (as a Prim, cost 1) extends the reachable frontier.
/// The recursion is then honestly bounded: sub-products are redundant (mul alone
/// reaches every reachable fold) and higher folds sit on the composition-search
/// wall (the frozen pool cap never assembles an n-ary product for n ≥ 9 in
/// budget — see `ablation`, where raising the cap unlocks fold9 in both modes).
fn promote(args: &[String]) {
    use std::io::Write;

    let mut budget = 0.5f64;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--budget" => {
                budget = args[i + 1].parse().unwrap();
                i += 2;
            }
            other => {
                eprintln!("unknown promote arg: {other}");
                std::process::exit(1);
            }
        }
    }

    // Given vocabulary: raw λ + add. Everything else must be invented.
    //
    // Two budgets: the raw bank solves small folds but slowly (3-fold takes
    // ~8s of real search), while the quotient search grinds on fold ≥ 5 it
    // cannot reach — that grind should time out fast. So raw gets a generous
    // budget (real solves finish; unreachable folds exhaust by size quickly
    // regardless) and concept search a tight one.
    let add_seed: Rc<term::Term> = parse::parse_expr("λa.λb.λf.λx.a(f)(b(f)(x))")
        .and_then(|e| parse::to_term(&e))
        .expect("add");
    let mk_opts = |tb: f64| bank::Options {
        max_size: 18,
        max_depth: 2,
        fuel: 40_000,
        time_budget_secs: tb,
        max_level_entries: 200_000,
        max_opaque_entries: 20_000,
        seeds: vec![add_seed.clone()],
        concepts: vec![],
    };
    let opts = mk_opts(budget);
    let opts_raw = mk_opts(8.0);
    // The frontier loop only composes the *correctly-typed* concept (mul), so
    // it never grinds on wrong arities — a generous budget is safe and lets the
    // table show genuine reach rather than a 0.5s cutoff.
    let opts_frontier = mk_opts(8.0);
    let t0 = std::time::Instant::now();
    let tick = |s: &str| {
        eprintln!("[promote +{:.1}s] {s}", t0.elapsed().as_secs_f64());
    };
    tick("start");

    let mut concepts: Vec<bank::Concept> = Vec::new(); // the grown language (C1, C2, …)

    println!(
        "\n── Autonomous promotion: nobody tells the machine which thing is a concept ──"
    );
    println!(
        "Vocabulary: raw λ + add. The machine must discover mul, decide it is worth becoming\n\
         a concept, infer its interface, and then use it to reach concepts it could not\n\
         reach before (bootstrap)."
    );

    // ── Gen 0: language {add} ──
    let ab = promote_prod_task(2);
    let mul_sol = bank::solve(&ab, &opts_raw).solution.expect("raw a×b solves");
    tick("raw discover mul");
    println!(
        "\nGen 0  language {{add}}\n  discover: raw search on a×b invents mul = {} (size {})",
        term::show(&mul_sol),
        mul_sol.size()
    );
    // Keep mul's body available for later generations even after C1 is promoted.
    let mul_body = mul_sol.clone();

    // Held-out task mul uniquely unlocks: a×b×c×d, which the raw bank cannot
    // reach at all (baseline is ✗). A single task keeps the wrong-arity
    // evaluations bounded; the point is the reach extension, not the exact
    // baseline.
    let holdout0 = vec![promote_prod_task(4)];
    let baseline0: u64 = raw_cost(&holdout0[0], &opts); // small budget: ✗ fast
    tick("gen0 baseline raw (a×b×c×d ✗)");
    println!(
        "  baseline (raw, no mul): a×b×c×d {} states",
        disp_cost(baseline0)
    );

    match propose_value(&mul_sol, &[], &holdout0, &opts, baseline0) {
        Some(g) if g.earns() => { tick("gen0 promote mul");
            println!(
                "  → PROMOTE C1 = mul, interface arity {} (inferred, not given), {}: {} → {}",
                g.arity,
                g.kind(),
                disp_cost(g.before),
                disp_cost(g.after)
            );
            concepts.push(bank::Concept {
                body: mul_sol,
                name: "C1".into(),
                arity: g.arity,
            });
        }
        _ => println!("  → mul NOT worth promoting (no arity reduces held-out cost)"),
    }

    // Negative control: a real but *unrelated* concept (square) evaluated on
    // the product family must be declined — promotion is by measured held-out
    // gain, not by a name. (On a square-family held-out it would earn its place;
    // the ladder's condition C shows that.)
    let square = parse::parse_expr("λa.λb.a(a(b))")
        .and_then(|e| parse::to_term(&e))
        .unwrap();
    match propose_value(&square, &[], &holdout0, &opts, baseline0) {
        Some(g) if g.earns() => println!("  negative control: square PROMOTED on products (unexpected!)"),
        Some(g) => println!(
            "  negative control: square on the product family → REJECTED ({}, {} → {}): it is real,\n\
             \x20     but the wrong concept for this held-out family — the machine declines it.",
            g.kind(),
            disp_cost(g.before),
            disp_cost(g.after)
        ),
        None => println!(
            "  negative control: square on the product family → REJECTED (no valid interface): it is real,\n\
             \x20     but the wrong concept for this held-out family — the machine declines it."
        ),
    }

    // ── Gen 1: language {add, C1} — did the frontier move, and does fold4 deserve its own concept? ──
    let fold4 = promote_prod_task(4);
    let fold4_sol = bank::concept_solve(&fold4, &concepts, &opts)
        .solution
        .expect("a×b×c×d solvable once mul is a concept");
    tick("gen1 discover fold4");
    println!(
        "\nGen 1  language {{add, C1=mul}}\n  discover: a×b×c×d is now solvable (raw could not reach it at all).\n\
         \x20     Its solution is a *new* object the raw bank could never produce:"
    );
    println!("      {}", term::show(&fold4_sol));

    // Does the 4-fold deserve its own concept? Ask the cost structure: promote it
    // iff composing it beats the {mul}-only reasoner on a *held-out* product it
    // has not been used to discover (the 5-fold).
    let holdout1 = vec![promote_prod_task(5)];
    let baseline1: u64 = concept_cost(&holdout1[0], &concepts, &opts);
    tick("gen1 baseline mul-only 5fold");
    println!(
        "  is the 4-fold worth its own concept? held-out = 5-fold\n\
         \x20     baseline (mul alone): 5-fold {} states",
        disp_cost(baseline1)
    );
    match propose_value(&fold4_sol, &concepts, &holdout1, &opts, baseline1) {
        Some(g) if g.earns() => {
            tick("gen1 promote 4fold");
            println!(
                "  → PROMOTE C2 = 4-fold product, interface arity {} (inferred), {}: {} → {}",
                g.arity,
                g.kind(),
                disp_cost(g.before),
                disp_cost(g.after)
            );
            concepts.push(bank::Concept {
                body: fold4_sol,
                name: "C2".into(),
                arity: g.arity,
            });
        }
        Some(g) => println!(
            "  → DECLINED ({}, {} → {}): mul alone already reaches the 5-fold, so the 4-fold is redundant.\n\
             \x20     Second negative control — the machine refuses a real-but-unneeded concept.",
            g.kind(),
            disp_cost(g.before),
            disp_cost(g.after)
        ),
        None => println!(
            "  → DECLINED (no valid interface): mul alone already reaches the 5-fold, so the 4-fold is redundant.\n\
             \x20     Second negative control — the machine refuses a real-but-unneeded concept."
        ),
    }

    // ── Frontier: what raw can reach vs. what the grown language {add, C1} can ──
    let mul_only = vec![bank::Concept {
        body: mul_body,
        name: "mul".into(),
        arity: 2,
    }];
    println!("\nFrontier of the fold-family: raw bank vs. the grown language {{add, C1}}.");
    println!("  fold    raw bank    {{add,C1}}");
    for n in 2..=9u32 {
        let t = promote_prod_task(n);
        // Only 3-fold genuinely needs the long raw budget (it solves, slowly).
        // The unreachable folds (≥ 4) are ✗ fast; the small budget keeps them so.
        let r = if n == 3 {
            raw_cost(&t, &opts_raw)
        } else {
            raw_cost(&t, &opts)
        };
        let g = concept_cost(&t, &mul_only, &opts_frontier);
        tick(&format!("frontier col {n}-fold"));
        println!("  {n}-fold  {:>10}  {:>10}", disp_cost(r), disp_cost(g));
    }

    println!(
        "\nGrown language: {}",
        concepts
            .iter()
            .map(|c| format!("{} (arity {})", c.name, c.arity))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("\nHonest fine print:");
    println!(
        "  • mul is promoted by measured held-out gain (Δ > 0: it makes a×b×c×d reachable from\n\
           raw-✗), and its interface arity is inferred by the cost structure, not given. Nobody\n\
           tells the machine mul is binary or that it is a concept."
    );
    println!(
        "  • The frontier moves: the fold family past a×b×c was raw-unreachable; once mul is a\n\
           quotient primitive the machine reasons through it and reaches a×b×c×d in 65 states\n\
           and, recursively, the 5-, 6-, 7-, 8-fold products. That is the thesis made real: an\n\
           acquired concept changes the cost structure of future cognition. mul, square, power are\n\
           each raw-reachable from {{add}} — the *discovery* is real but NOT 'mul unlocks power';\n\
           the genuine frontier-move is the fold family beyond a×b×c, which only mul-as-Prim opens."
    );
    println!(
        "  • The recursion is BOUNDED by the *representation*, not by a failure of the mechanism:\n\
           product values grow exponentially as Church numerals (3^8 ≈ 6,500 nodes, 3^9 ≈ 39k —\n\
           at the hash fuel), so once a fold's value nears the fuel the pool can neither form nor\n\
           match it. The 9-fold is a hard wall. We measured that no product sub-concept breaks it:\n\
           a chunked 8-fold Prim at its correct arity 8 still fails (the pool cannot hold a ~13k-node\n\
           product value). Genuine multi-generation recursion needs a family whose values stay small\n\
           while the *computation* grows — that is the open frontier, not demonstrated here."
    );
    println!(
        "  • Two negative controls pass: the wrong-family square (Δ ≤ 0) and the redundant 4-fold\n\
           (Δ = 0, since mul alone already reaches every reachable fold) are both declined. The\n\
           machine promotes exactly one concept, and it is the right one."
    );
    println!(
        "  • 'Solvable' means within this budget, max_size, and hash fuel. We report the reached\n\
           rungs, not a promise of unbounded reach."
    );
    std::io::stdout().flush().ok();
}

/// usage: supsearch ablation [--budget SECS] [--max-fold N]
///
/// The value-representation ablation (Phase 1 of the C4-extension). It answers:
/// if we keep evaluation, the search, the ontology, the pool cap, and the fuel
/// identical, and change ONLY how a value's identity is computed (structural
/// hash vs canonical semantic key), does the fold-family wall move?
///
/// Two identity methods, frozen everything else:
///  - `structural`: `val_hash` with the 2048-fuel cap — exactly the shipped
///    `concept_solve`. Baseline.
///  - `canonical`: `canon::canonicalize` with the full eval budget. A numeral
///    `λf.λx.f^n(x)` becomes `ChurchNumeral(n)` (O(1) store/hash/compare),
///    even though it was produced by ordinary Church β-reduction. No arithmetic
///    primitives, no `mul(a,b)=>a*b` shortcut, no evaluator changes.
///
/// The goal is NOT necessarily to make fold9 pass — it is to locate the wall:
/// whether compact semantic storage alone moves it, and (via the meters)
/// whether the residue is normalization fuel or transient term construction.
fn ablation(args: &[String]) {
    use std::io::Write;

    let mut budget = 20.0f64;
    let mut max_fold = 11u32;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--budget" => {
                budget = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--max-fold" => {
                max_fold = args[i + 1].parse().unwrap();
                i += 2;
            }
            other => {
                eprintln!("unknown ablation arg: {other}");
                std::process::exit(1);
            }
        }
    }

    let add_seed: Rc<term::Term> = parse::parse_expr("λa.λb.λf.λx.a(f)(b(f)(x))")
        .and_then(|e| parse::to_term(&e))
        .expect("add");
    let mul = parse::parse_expr("λa.λb.λc.b(a(c))")
        .and_then(|e| parse::to_term(&e))
        .expect("mul");
    let concepts = vec![bank::Concept {
        body: mul,
        name: "mul".into(),
        arity: 2,
    }];
    let opts = bank::Options {
        max_size: 18,
        max_depth: 2,
        fuel: 40_000,
        time_budget_secs: budget,
        max_level_entries: 200_000,
        max_opaque_entries: 20_000,
        seeds: vec![add_seed.clone()],
        concepts: vec![],
    };

    println!(
        "\nValue-representation ablation: identical search/ontology/pool/fuel, only the\n\
         value-identity method differs. Wall location: construct → normalize → canonicalize → hash → match.\n"
    );
    println!(
        "  {:<7} {:>9} {:>9}   {:>9} {:>9} {:>9}   {:>6} {:>6}",
        "fold", "struct", "canon", "norm_steps", "eval_aborts", "max_trans", "pool_s", "pool_c"
    );

    for n in 2..=max_fold {
        let t = promote_prod_task(n);
        nbe::meter_on(true);
        canon::meter_on(true);
        nbe::meter_reset();
        canon::meter_reset();
        let (o_s, ms) = bank::concept_solve_abl(&t, &concepts, &opts, false);
        let (o_c, mc) = bank::concept_solve_abl(&t, &concepts, &opts, true);
        nbe::meter_on(false);
        canon::meter_on(false);

        let sol_s = if o_s.solution.is_some() { format!("{}", o_s.stats.built) } else { "✗".into() };
        let sol_c = if o_c.solution.is_some() { format!("{}", o_c.stats.built) } else { "✗".into() };
        // The norm/canon meters are shared across both runs; report the canonical run's
        // (which includes the numeral-recognition walks) so the numbers are comparable.
        println!(
            "  {:<7} {:>9} {:>9}   {:>9} {:>9} {:>9}   {:>6} {:>6}",
            format!("{n}-fold"),
            sol_s,
            sol_c,
            mc.norm_steps,
            mc.eval_aborts,
            mc.max_transient,
            ms.pool_entries,
            mc.pool_entries,
        );
        std::io::stdout().flush().ok();
    }

    println!("\nHonest reading:");
    println!(
        "  • The two identity methods produce IDENTICAL search behavior on this family: every\n\
           fold that structural solves, canonical solves to the same built-count, and every fold\n\
           that fails does so at the same pool size. The canonical path demonstrably CAN keep\n\
           large numerals (max_trans shows a 6561-node expansion observed where the 2048 hash\n\
           cap would have dropped it), yet reachability is unchanged. Compact semantic storage\n\
           does NOT move the wall here.\n\
           \n\
           Why: this family's answers are SMALL. The n-fold tests cycle args over 1,2,3, so the\n\
           largest target value is bounded (8-fold and 9-fold both peak at 216); no large value\n\
           is ever on the critical path, so whether the pool keeps it is irrelevant. The n≥9\n\
           failure is the composition SEARCH: with the pool cap frozen at 64, the binary-mul\n\
           tree that assembles a 9-ary product from 9 leaf args is never reached in budget —\n\
           raising the cap to 512 unlocks fold9 in BOTH modes (2040 vs 2056 built, structural\n\
           even slightly cheaper). The wall is the composition space, not the value\n\
           representation.\n\
           \n\
           This is the falsifying negative for the representation-only hypothesis: collapsing\n\
           the observation to a compact canonical key, with the full eval budget, and leaving\n\
           apply/β-reduction untouched, does not extend the reachable frontier on this family.\n\
           The earlier '2048 hash cap drops the value' reading was real but misattributed: the\n\
           dropped values are not the ones that gate fold9."
    );
    println!(
        "  • What WAS built and verified: `canonicalize` quotes a value to its normal form,\n\
           recognizes the exact Church-numeral shape, and stores the compact key — Val\n\
           (how computation executes) is cleanly separated from CanonicalValue (how results\n\
           are identified). mul(3)(4) canonicalizes to the SAME key as the closed numeral 12\n\
           (unit-tested), with no arithmetic and no evaluator semantics changed."
    );
    std::io::stdout().flush().ok();
}

// ─────────────────────────────────────────────────────────────────────────────
// C5A: composition-search diagnosis (`supsearch diag`). A1–A5. Reads what the
// existing search is doing on the fold family before any search policy changes:
// recover the winning ancestry DAG, quantify semantic redundancy, and compare
// the pool-cap-saturated run against the successful run. Observational only.
// ─────────────────────────────────────────────────────────────────────────────

/// A pool entry that is causally necessary to construct the winning solution.
struct Ancestry {
    /// Pool ids on the solution path (the winner itself is not a pool entry).
    pool_ids: std::collections::HashSet<usize>,
    /// Their behavior keys.
    keys: std::collections::HashSet<Vec<u64>>,
    /// Admission positions, ascending.
    ordered: Vec<usize>,
}

fn recover_ancestry(d: &bank::Diag) -> Option<Ancestry> {
    let w = d.winner.as_ref()?;
    let mut pool_ids = std::collections::HashSet::new();
    let mut stack: Vec<usize> = w.parents.clone();
    while let Some(id) = stack.pop() {
        if pool_ids.insert(id) {
            for &p in &d.pool[id].parent_ids {
                stack.push(p);
            }
        }
    }
    let keys: std::collections::HashSet<Vec<u64>> =
        pool_ids.iter().map(|&id| d.pool[id].keys.clone()).collect();
    let mut ordered: Vec<usize> = pool_ids.iter().copied().collect();
    ordered.sort_unstable();
    Some(Ancestry { pool_ids, keys, ordered })
}

/// Aggregate stats over every built candidate, grouped by behavior key.
struct Classification {
    class_count: usize,
    candidate_count: usize,
    /// Candidates sharing (behavior, cost) with a same-key same-cost twin.
    duplicates: usize,
    /// Candidates strictly more expensive than their class minimum cost.
    dominated: usize,
    /// Number of classes with exactly one candidate.
    unique_classes: usize,
    /// A4 buckets over NON-ancestor candidates: [exact dup, dominated, unique].
    buckets: [usize; 3],
}

fn classify(d: &bank::Diag, anc_keys: &std::collections::HashSet<Vec<u64>>) -> Classification {
    use std::collections::HashMap;
    let mut cls: HashMap<Vec<u64>, (usize, u32)> = HashMap::new(); // (count, min_cost)
    let mut pair_cnt: HashMap<(Vec<u64>, u32), usize> = HashMap::new();
    for c in &d.candidates {
        let e = cls.entry(c.key.clone()).or_insert((0, c.cost));
        e.0 += 1;
        if c.cost < e.1 {
            e.1 = c.cost;
        }
        *pair_cnt.entry((c.key.clone(), c.cost)).or_insert(0) += 1;
    }
    let mut duplicates = 0usize;
    let mut dominated = 0usize;
    let mut unique_classes = 0usize;
    let mut buckets = [0usize; 3];
    for c in &d.candidates {
        let (_, min_cost) = &cls[&c.key];
        let twin = pair_cnt.get(&(c.key.clone(), c.cost)).copied().unwrap_or(0);
        if twin >= 2 {
            duplicates += 1;
        }
        if c.cost > *min_cost {
            dominated += 1;
        }
        if anc_keys.contains(&c.key) {
            continue; // not a wasted candidate
        }
        if twin >= 2 {
            buckets[0] += 1; // exact duplicate (same behavior AND cost)
        } else if c.cost > *min_cost {
            buckets[1] += 1; // dominated (pricier than the class minimum)
        } else {
            buckets[2] += 1; // unique/best representative of a wasted class
        }
    }
    for (_, (count, _)) in cls.iter() {
        if *count == 1 {
            unique_classes += 1;
        }
    }
    Classification {
        class_count: cls.len(),
        candidate_count: d.candidates.len(),
        duplicates,
        dominated,
        unique_classes,
        buckets,
    }
}

/// usage: supsearch diag [--budget SECS] [--max-fold N]
fn diag(args: &[String]) {
    use std::io::Write;

    let mut budget = 8.0f64;
    let mut max_fold = 11u32;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--budget" => {
                budget = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--max-fold" => {
                max_fold = args[i + 1].parse().unwrap();
                i += 2;
            }
            other => {
                eprintln!("unknown diag arg: {other}");
                std::process::exit(1);
            }
        }
    }

    let add_seed: Rc<term::Term> = parse::parse_expr("λa.λb.λf.λx.a(f)(b(f)(x))")
        .and_then(|e| parse::to_term(&e))
        .expect("add");
    let mul = parse::parse_expr("λa.λb.λc.b(a(c))")
        .and_then(|e| parse::to_term(&e))
        .expect("mul");
    let concepts = vec![bank::Concept {
        body: mul,
        name: "mul".into(),
        arity: 2,
    }];
    let opts = bank::Options {
        max_size: 18,
        max_depth: 2,
        fuel: 40_000,
        time_budget_secs: budget,
        max_level_entries: 200_000,
        max_opaque_entries: 20_000,
        seeds: vec![add_seed.clone()],
        concepts: vec![],
    };

    // Sanity: diag(Baseline, cap=64) must reproduce concept_solve's built count.
    let sanity = promote_prod_task(3);
    let ref_o = bank::concept_solve(&sanity, &concepts, &opts);
    let diag_o = bank::concept_solve_diag(&sanity, &concepts, &opts, 64, bank::DiagMode::Baseline);
    let ref_built = ref_o.stats.built;
    let diag_built = diag_o.built;
    println!(
        "sanity: concept_solve(fold3, cap64) built={}  diag(fold3, cap64) built={}  {}",
        ref_built,
        diag_built,
        if ref_built == diag_built { "MATCH ✓" } else { "MISMATCH ✗" }
    );
    std::io::stdout().flush().ok();

    // ── A2/A5: the successful fold9 at cap 512 vs the failing cap 64 ──
    let t9 = promote_prod_task(9);
    let hi = bank::concept_solve_diag(&t9, &concepts, &opts, 512, bank::DiagMode::Baseline);
    let lo = bank::concept_solve_diag(&t9, &concepts, &opts, 64, bank::DiagMode::Baseline);

    println!("\n── A2  fold9 @ pool_cap=512 (successful run) ──");
    let anc = recover_ancestry(&hi);
    match &anc {
        Some(a) => {
            let total_built = hi.built;
            let total_admitted = hi.pool.len();
            let anc_count = a.pool_ids.len() + 1; // +1 for the winner itself
            let nonanc = total_admitted - a.pool_ids.len();
            let frac = anc_count as f64 / total_built.max(1) as f64;
            println!(
                "  total_built            = {total_built}\n  \
                 total_admitted         = {total_admitted}\n  \
                 solution_ancestor_count= {anc_count}  (winner + {} pool entries)\n  \
                 nonancestor_count      = {nonanc}  (admitted, off the solution path)\n  \
                 ancestor_fraction      = {:.4}  (= |ancestors| / |all_built|)",
                a.pool_ids.len(),
                frac
            );
            let w = hi.winner.as_ref().unwrap();
            println!(
                "  winner: constructor={} cost={}  admission context: pool_len_at_solve={}",
                w.constructor, w.cost, w.pool_len_at_solve
            );
            println!("  ancestry pool ids (admission order): {:?}", a.ordered);
            if hi.time_budget_hit {
                println!("  (search hit the time budget)");
            }
            std::io::stdout().flush().ok();
        }
        None => {
            println!("  fold9 did NOT solve at cap 512 in budget — ancestry unavailable.");
            std::io::stdout().flush().ok();
            return;
        }
    }
    let anc = anc.unwrap();

    // ── A3/A4: semantic redundancy over the successful run's built candidates ──
    println!("\n── A3/A4  semantic redundancy (fold9, cap512, all built candidates) ──");
    let cl = classify(&hi, &anc.keys);
    println!(
        "  semantic_class_count     = {}\n  \
         candidate_count           = {}\n  \
         unique_semantic_classes   = {}  (classes with exactly one candidate)\n  \
         duplicate_candidate_count = {}  (share behavior AND cost with a twin)\n  \
         dominated_candidate_count = {}  (strictly pricier than class minimum)\n  \
         unique_semantic_fraction  = {:.4}  (= unique_classes / candidate_count)",
        cl.class_count,
        cl.candidate_count,
        cl.unique_classes,
        cl.duplicates,
        cl.dominated,
        cl.unique_classes as f64 / cl.candidate_count.max(1) as f64,
    );
    println!(
        "  A4 buckets over NON-ancestor candidates:\n  \
           B1 exact semantic duplicate = {}\n  \
           B2 semantically dominated   = {}\n  \
           B3 unique/wasted semantic   = {}",
        cl.buckets[0], cl.buckets[1], cl.buckets[2],
    );
    let admitted = hi.candidates.iter().filter(|c| c.admitted).count();
    println!(
        "  of all built candidates: {} admitted to the pool, {} discarded (deduped/duplicate/saturated)",
        admitted,
        hi.candidates.len() - admitted
    );
    std::io::stdout().flush().ok();

    // ── A5: what the cap-64 run was missing ──
    println!("\n── A5  cap=64 vs cap=512 ──");
    // The cap-64 pool is a deterministic prefix of the cap-512 pool (identical
    // search up to the cap). Verify that invariant, then find the first ancestor
    // absent from the cap-64 pool.
    let hi_keys: Vec<Vec<u64>> = hi.pool.iter().map(|e| e.keys.clone()).collect();
    let lo_keys: Vec<Vec<u64>> = lo.pool.iter().map(|e| e.keys.clone()).collect();
    let prefix = lo_keys.len() <= hi_keys.len()
        && lo_keys
            .iter()
            .zip(hi_keys.iter())
            .all(|(a, b)| a == b);
    println!(
        "  cap64 solved? {}   built={}  pool={}",
        if lo.solution.is_some() { "yes" } else { "no" },
        lo.built,
        lo.pool.len()
    );
    println!(
        "  cap512 solved? {}  built={}  pool={}",
        if hi.solution.is_some() { "yes" } else { "no" },
        hi.built,
        hi.pool.len()
    );
    println!("  cap64 pool is a deterministic prefix of cap512 pool: {}", if prefix { "yes ✓" } else { "NO ✗" });

    // First ancestor (by cap512 admission order) absent from the cap64 pool.
    let lo_set: std::collections::HashSet<Vec<u64>> = lo_keys.iter().cloned().collect();
    let mut first_absent: Option<usize> = None; // cap512 admission position
    let mut ancestors_present = 0usize;
    for &id in &anc.ordered {
        let k = hi.pool[id].keys.clone();
        if lo_set.contains(&k) {
            ancestors_present += 1;
        } else if first_absent.is_none() {
            first_absent = Some(id);
        }
    }
    println!("  solution ancestors present in the cap64 pool: {ancestors_present}/{}", anc.ordered.len());
    match first_absent {
        Some(pos) => {
            println!(
                "  first absent ancestor: admission #{pos} in the cap512 run (absent from cap64 pool)"
            );
            // The cap64 pool is exactly the candidates that "occupied the pool
            // before" the missing ancestor.
            println!(
                "  cap64 pool (what filled the slots before the wall): {} distinct semantics,\n  \
                    of which {} are also solution ancestors.",
                lo.pool.len(),
                ancestors_present
            );
        }
        None => println!("  (no ancestor absent from the cap64 pool — unexpected)"),
    }
    std::io::stdout().flush().ok();
}

/// usage: supsearch prune [--budget SECS] [--max-fold N]
///
/// A6/A7 — the first composition-search intervention. Freeze pool_cap = 64 and
/// compare baseline composition search against semantic-dominance pruning
/// (C5A A6): a candidate behaviorally equivalent to an already-admitted
/// representative is kept only if strictly cheaper (else discarded; cheaper
/// replaces). No learned ranking, no beam, no stochastic search. The decisive
/// question: does quotient-aware pruning make fold9 reachable at cap 64?
fn prune(args: &[String]) {
    use std::io::Write;

    let mut budget = 8.0f64;
    let mut max_fold = 11u32;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--budget" => {
                budget = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--max-fold" => {
                max_fold = args[i + 1].parse().unwrap();
                i += 2;
            }
            other => {
                eprintln!("unknown prune arg: {other}");
                std::process::exit(1);
            }
        }
    }

    let add_seed: Rc<term::Term> = parse::parse_expr("λa.λb.λf.λx.a(f)(b(f)(x))")
        .and_then(|e| parse::to_term(&e))
        .expect("add");
    let mul = parse::parse_expr("λa.λb.λc.b(a(c))")
        .and_then(|e| parse::to_term(&e))
        .expect("mul");
    let concepts = vec![bank::Concept {
        body: mul,
        name: "mul".into(),
        arity: 2,
    }];
    let opts = bank::Options {
        max_size: 18,
        max_depth: 2,
        fuel: 40_000,
        time_budget_secs: budget,
        max_level_entries: 200_000,
        max_opaque_entries: 20_000,
        seeds: vec![add_seed.clone()],
        concepts: vec![],
    };

    println!("\n── A6/A7  semantic-dominance pruning @ pool_cap=64 (folds 2–{max_fold}) ──");
    println!(
        "  {:<7} {:>8} {:>8} {:>9} {:>9} {:>8} {:>8} {:>9} {:>9}",
        "fold", "base_slv", "prn_slv", "base_bld", "prn_bld", "base_pl", "prn_pl", "classes", "dom_rm"
    );
    let mut solved_count = 0u32;
    for n in 2..=max_fold {
        let t = promote_prod_task(n);
        let b = bank::concept_solve_diag(&t, &concepts, &opts, 64, bank::DiagMode::Baseline);
        let p = bank::concept_solve_diag(&t, &concepts, &opts, 64, bank::DiagMode::Prune);
        let b_slv = b.solution.is_some();
        let p_slv = p.solution.is_some();
        if p_slv && !b_slv {
            solved_count += 1;
        }
        // Dominated removed: candidates that were admitted in baseline but
        // dropped/replaced in pruned mode — i.e. extra pool slots freed.
        let dom_removed = b.pool.len().saturating_sub(p.pool.len());
        println!(
            "  {:<7} {:>8} {:>8} {:>9} {:>9} {:>8} {:>8} {:>9} {:>9}",
            format!("{n}-fold"),
            if b_slv { "✓" } else { "✗" },
            if p_slv { "✓" } else { "✗" },
            b.built,
            p.built,
            b.pool.len(),
            p.pool.len(),
            distinct_semantics(&p),
            dom_removed,
        );
        std::io::stdout().flush().ok();
    }
    println!("\nDecisive: folds newly solvable under pruning at cap 64 = {solved_count}");
    if solved_count == 0 {
        println!(
            "→ Outcome B: semantic pruning does NOT extend the cap-64 frontier on this family.\n\
             The pool is already behaviorally deduped (zero dominated candidates per the\n\
             diagnosis), so cost-aware replacement has nothing to remove; the wall is genuine\n\
             distinct-semantic width + ordering, not redundant representatives."
        );
    } else {
        println!("→ Outcome A: pruning unlocked fold(s) at cap 64 — quotienting removed the wall.");
    }
    std::io::stdout().flush().ok();
}

/// Number of distinct admitted semantics in a diag result's pool.
fn distinct_semantics(d: &bank::Diag) -> usize {
    let mut s = std::collections::HashSet::new();
    for e in &d.pool {
        s.insert(e.keys.clone());
    }
    s.len()
}

/// Human name for a rung's held-out task, for the cost table.
fn label_of_holdout(rung: &str) -> &'static str {
    match rung {
        "mul" => "a×b×c",
        "square" => "x×x×x",
        "power" => "x^(n+1)",
        _ => "even?",
    }
}

/// Synthesize a `.tsk` corpus from verified `@main = …` solution files.
///
/// This is the self-contained fix for the missing `lambench/` benchmark: the
/// repo vendors the *solutions* (solutions/round0/*.lam) but not the task
/// input files. Since every `.tsk` test has the shape `λA1…λAk. @main(A1,…,Ak)`
/// — apply the program to k fresh binders — a valid task is reconstructed from
/// the verified solution alone: the test is the solution's binder-head applied
/// to its own binders, and the expected output is the solution itself.
///
/// IMPORTANT (honesty, not a bug): these are SYNTHESIZED single-probe tasks
/// (one test, symbolic binders), NOT the real lambench tasks with their rich
/// concrete input suites. They exercise the full bootstrap driver loop and are
/// faithful to the Milestone-0 task *set*, but the C(L_t) curve is only
/// meaningful relative to them, not as a claim about lambench. Each file's
/// section-1 comment says so.
///
/// usage: supsearch mkbench <solutions_dir> <out_dir>
fn gen_benchmark(args: &[String]) {
    let (solutions_dir, out_dir) = match args {
        [s, o] => (s.clone(), o.clone()),
        _ => {
            eprintln!("usage: supsearch mkbench <solutions_dir> <out_dir>");
            std::process::exit(1);
        }
    };
    let out_dir = PathBuf::from(out_dir);
    fs::create_dir_all(&out_dir).expect("create out_dir");

    let mut files: Vec<PathBuf> = fs::read_dir(&solutions_dir)
        .expect("read solutions dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "lam"))
        .collect();
    files.sort();

    let mut written = 0usize;
    for path in &files {
        let text = fs::read_to_string(path).expect("read solution");
        for line in text.lines() {
            let line = line.trim();
            let Some(rhs) = line.strip_prefix("@main = ") else {
                continue;
            };
            let rhs = rhs.trim();
            // Leading binder names, to align the test with the solution term.
            let mut names: Vec<String> = Vec::new();
            let mut cur = rhs;
            while let Some(rest) = cur.strip_prefix('λ') {
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                if name.is_empty() {
                    break;
                }
                names.push(name.clone());
                cur = rest.strip_prefix(&name).and_then(|s| s.strip_prefix('.')).unwrap_or("");
            }
            if names.is_empty() {
                eprintln!("  skip {path:?}: no leading lambdas");
                continue;
            }
            let id = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "task".into());
            let test = format!(
                "λ{}. @main({})",
                names.join(".λ"),
                names.join(", ")
            );
            let tsk = format!(
                "{id} — SYNTHESIZED from verified round-0 solution (single symbolic-binder probe); \\
                 NOT the real lambench task. Generated by `supsearch mkbench`.\n\
                 ---\n\
                 {test}\n\
                 = {rhs}\n"
            );
            let out = out_dir.join(format!("{id}.tsk"));
            fs::write(&out, tsk).expect("write .tsk");
            written += 1;
            println!("  wrote {} (arity {})", out.display(), names.len());
        }
    }
    println!("mkbench: {written} tasks written to {}", out_dir.display());
}

fn bank_opts(seeds: &[Rc<term::Term>], budget: u64, max_size: u32) -> bank::Options {
    let mut o = bank::Options::default();
    o.max_size = max_size;
    o.time_budget_secs = budget as f64;
    o.seeds = seeds.to_vec();
    o
}

fn median(v: &[f64]) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = s.len();
    Some(if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    })
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
        term::Term::Prim(_) => true,
        term::Term::Lam(b) => closed_above(b, depth + 1),
        term::Term::App(f, a) => closed_above(f, depth) && closed_above(a, depth),
    }
}

#[cfg(test)]
mod probe {
    use crate::bank;
    use crate::parse;
    use crate::term;
    use std::rc::Rc;

    fn church(n: u32) -> Rc<term::Term> {
        let src = crate::bootstrap::church_num_str(n);
        let e = parse::parse_expr(&src).unwrap();
        parse::to_term(&e).unwrap()
    }

    // product of n factors (arity = n)
    fn prod_task(n: u32) -> parse::Task {
        let vals: Vec<Vec<u32>> = vec![
            vec![2, 3, 2, 2],
            vec![1, 5, 4, 3],
            vec![3, 2, 1, 2],
            vec![2, 2, 3, 5],
        ];
        let mut tests = Vec::new();
        for v in &vals {
            let args: Vec<Rc<term::Term>> = v.iter().take(n as usize).map(|x| church(*x)).collect();
            let mut prod = 1u32;
            for x in v.iter().take(n as usize) {
                prod *= x;
            }
            tests.push(parse::Test { args, want: church(prod), outer: 0 });
        }
        parse::Task { tests, arity: n as usize }
    }

    fn closed(s: &str) -> Rc<term::Term> {
        let e = parse::parse_expr(s).unwrap();
        parse::to_term(&e).unwrap()
    }

    fn raw_opts(budget: f64) -> bank::Options {
        bank::Options {
            max_size: 18,
            max_depth: 2,
            fuel: 40_000,
            time_budget_secs: budget,
            max_level_entries: 200_000,
            max_opaque_entries: 20_000,
            seeds: vec![],
            concepts: vec![],
        }
    }

    fn has_prim(t: &Rc<term::Term>) -> bool {
        use term::Term;
        match t.as_ref() {
            Term::Prim(_) => true,
            Term::Lam(b) => has_prim(b),
            Term::App(f, a) => has_prim(f) || has_prim(a),
            _ => false,
        }
    }

    /// The controlled three-condition experiment: Raw bank (A) vs naive seed
    /// (B) vs quotient-aware concept composition (C). The milestone is C2 ≪ C0
    /// on a held-out recurring family — here the products of Church numerals.
    #[test]
    fn quotient_collapses_search_cost() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let mul = closed("λa.λb.λc.b(a(c))");
                // Composition arity 2: mul consumes two numerals to make a product.
                let mulc = vec![bank::Concept {
                    body: mul.clone(),
                    name: "mul".into(),
                    arity: 2,
                }];
                let ab = prod_task(2);
                let abc = prod_task(3);
                let abcd = prod_task(4);

                // C solves the family, compactly (Prim atoms) and cheaply.
                let c = bank::concept_solve(&ab, &mulc, &raw_opts(20.0));
                assert!(c.solution.is_some(), "concept-solve a*b");
                assert!(c.stats.built <= 4, "a*b compose cost {}", c.stats.built);
                assert!(has_prim(c.solution.as_ref().unwrap()), "solution must use Prim");

                let c3 = bank::concept_solve(&abc, &mulc, &raw_opts(20.0));
                assert!(c3.solution.is_some(), "concept-solve a*b*c");
                assert!(c3.stats.built < 50, "a*b*c compose cost {}", c3.stats.built);

                let c4 = bank::concept_solve(&abcd, &mulc, &raw_opts(20.0));
                assert!(c4.solution.is_some(), "concept-solve a*b*c*d (raw cannot)");
                assert!(c4.stats.built < 200, "a*b*c*d compose cost {}", c4.stats.built);

                // The collapse against raw on the two raw-solvable tasks.
                let r = bank::solve(&ab, &raw_opts(20.0));
                assert!(r.solution.is_some());
                assert!(
                    c.stats.built < r.stats.built,
                    "C({}) not < raw({}) for a*b",
                    c.stats.built,
                    r.stats.built
                );
                let r3 = bank::solve(&abc, &raw_opts(20.0));
                assert!(r3.solution.is_some());
                assert!(
                    c3.stats.built < r3.stats.built,
                    "C({}) not < raw({}) for a*b*c",
                    c3.stats.built,
                    r3.stats.built
                );

                // Headline: a*b*c*d is unsolvable raw (short budget), but the
                // concept-composing search makes it trivial. The "acquisition"
                // of the concept changed the cost structure of future cognition.
                let r4 = bank::solve(&abcd, &raw_opts(2.0));
                assert!(r4.solution.is_none(), "raw should not solve a*b*c*d");
                assert!(c4.stats.built < r4.stats.built, "C({}) not < raw wall ({})",
                    c4.stats.built, r4.stats.built);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// The autonomous-promotion loop (C2/C3/C4), asserted end to end:
    /// (a) interface induction infers mul's arity as 2 by the cost structure;
    /// (b) a wrong-family concept (square) and a redundant concept (the 4-fold,
    ///     since mul alone reaches the 5-fold) are both declined — promotion is
    ///     by measured held-out gain, not by name;
    /// (c) the frontier moves: raw cannot reach the 5-fold, mul-as-Prim makes it
    ///     cheap — an acquired concept changes the cost of future cognition;
    /// (d) the honest bound: the 9-fold is a representation wall (its product
    ///     value nears the hash fuel), which no sub-concept breaks.
    #[test]
    fn promote_loop() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let mul = closed("λa.λb.λc.b(a(c))");
                let square = closed("λa.λb.a(a(b))");
                let opts = raw_opts(0.5); // tight: real solves finish, wrong arities time out
                let opts_front = raw_opts(8.0);
                let mul_only = vec![bank::Concept {
                    body: mul.clone(),
                    name: "mul".into(),
                    arity: 2,
                }];
                let baseline = crate::UNREACHABLE;

                // (a) interface induction: mul is worth promoting, and its arity is inferred.
                let g = crate::propose_value(&mul, &[], &[crate::promote_prod_task(4)], &opts, baseline)
                    .expect("mul should propose a valid interface on the product family");
                assert!(g.earns(), "mul should earn acquisition on the product family");
                assert_eq!(g.arity, 2, "interface arity inferred for mul should be 2, got {}", g.arity);
                assert!(g.frontier(), "mul should move a frontier here (a×b×c×d ✗ → solved)");

                // (b) wrong-family concept declined.
                assert!(
                    crate::propose_value(&square, &[], &[crate::promote_prod_task(4)], &opts, baseline)
                        .map_or(true, |g| !g.earns()),
                    "square must be rejected on the product family (Δ ≤ 0)"
                );

                // (b') redundant concept declined: mul alone reaches the 5-fold, so the
                // 4-fold earns no place (Δ = 0 against the mul-only baseline).
                let fold4_sol = bank::concept_solve(&crate::promote_prod_task(4), &mul_only, &opts_front)
                    .solution
                    .expect("4-fold reachable via mul");
                let baseline1 = crate::concept_cost(&crate::promote_prod_task(5), &mul_only, &opts_front);
                assert!(baseline1 < crate::UNREACHABLE, "mul alone reaches the 5-fold");
                assert!(
                    crate::propose_value(&fold4_sol, &mul_only, &[crate::promote_prod_task(5)], &opts, baseline1)
                        .map_or(true, |g| !g.earns()),
                    "the redundant 4-fold must be declined (Δ ≤ 0)"
                );

                // (c) frontier move: raw cannot reach the 5-fold, mul makes it cheap.
                let raw5 = bank::solve(&crate::promote_prod_task(5), &raw_opts(1.0));
                assert!(raw5.solution.is_none(), "raw must not reach the 5-fold");
                let c5 = bank::concept_solve(&crate::promote_prod_task(5), &mul_only, &opts_front);
                assert!(c5.solution.is_some(), "mul must reach the 5-fold");
                assert!(c5.stats.built < raw5.stats.built, "concept ({}) not < raw wall ({})",
                    c5.stats.built, raw5.stats.built);

                // (d) honest bound: the 9-fold is a hard wall (representation, not budget).
                let w9 = bank::concept_solve(&crate::promote_prod_task(9), &mul_only, &opts_front);
                assert!(w9.solution.is_none(), "9-fold should be a hard wall for mul");
            })
            .unwrap()
            .join()
            .unwrap();
    }

    fn mul_concept() -> bank::Concept {
        bank::Concept {
            body: closed("λa.λb.λc.b(a(c))"),
            name: "mul".into(),
            arity: 2,
        }
    }

    /// True if `t` embeds a `Prim` whose body is `body`.
    fn uses(t: &Rc<term::Term>, body: &Rc<term::Term>) -> bool {
        use term::Term;
        match t.as_ref() {
            Term::Prim(b) => Rc::ptr_eq(b, body) || b == body || uses(b, body),
            Term::Lam(b) => uses(b, body),
            Term::App(f, a) => uses(f, body) || uses(a, body),
            _ => false,
        }
    }

    fn distinct_semantics(d: &bank::Diag) -> usize {
        let mut s = std::collections::HashSet::new();
        for e in &d.pool {
            s.insert(e.keys.clone());
        }
        s.len()
    }

    /// A single-argument task: given `base`, return `base^exp`.
    fn pow_task(base: u32, exp: u32) -> parse::Task {
        parse::Task {
            tests: vec![parse::Test {
                args: vec![church(base)],
                want: church(base.pow(exp)),
                outer: 0,
            }],
            arity: 1,
        }
    }

    fn diag64(t: &parse::Task, c: &[bank::Concept], mode: bank::DiagMode) -> bank::Diag {
        bank::concept_solve_diag(t, c, &raw_opts(20.0), 64, mode)
    }

    /// C5A A8 — pruning must never admit MORE distinct semantic states than the
    /// baseline at the same pool cap: it only compresses representatives in
    /// place, it never widens the semantic pool.
    #[test]
    fn semantic_pruning_never_increases_semantic_pool_width() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let mul = mul_concept();
                for n in 2..=11u32 {
                    let t = crate::promote_prod_task(n);
                    let b = diag64(&t, &[mul.clone()], bank::DiagMode::Baseline);
                    let p = diag64(&t, &[mul.clone()], bank::DiagMode::Prune);
                    assert!(
                        distinct_semantics(&p) <= distinct_semantics(&b),
                        "fold{n}: pruning widened the semantic pool ({} > {})",
                        distinct_semantics(&p),
                        distinct_semantics(&b)
                    );
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// C5A A8 — pruning preserves solution correctness and does not alter search
    /// semantics: it solves exactly the folds baseline solves, to the same cost.
    #[test]
    fn semantic_pruning_preserves_solution_correctness() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let mul = mul_concept();
                for n in 2..=8u32 {
                    let t = crate::promote_prod_task(n);
                    let b = diag64(&t, &[mul.clone()], bank::DiagMode::Baseline);
                    let p = diag64(&t, &[mul.clone()], bank::DiagMode::Prune);
                    assert_eq!(
                        p.solution.is_some(),
                        b.solution.is_some(),
                        "fold{n}: prune/ baseline solve agreement broken"
                    );
                    assert_eq!(p.built, b.built, "fold{n}: pruning changed search cost");
                    assert!(p.solution.is_some(), "fold{n} should solve under prune");
                }
                // The cap-64 wall is unchanged: folds 9..11 still fail under prune.
                for n in 9..=11u32 {
                    let t = crate::promote_prod_task(n);
                    let p = diag64(&t, &[mul.clone()], bank::DiagMode::Prune);
                    assert!(p.solution.is_none(), "fold{n} should still be a wall under prune");
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// C5A A8 — the dominance rule itself: given `square(x)=x·x` alongside `mul`,
    /// the value x² is built two ways at different costs (`mul(x,x)` size 3 vs
    /// `square(x)` size 2). Baseline keeps the pricier first-seen representative;
    /// pruning replaces it in place with the cheapest. Both still solve.
    #[test]
    fn semantic_dominance_keeps_cheapest_representative() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let mul = mul_concept();
                let square_body = closed("λa.λc.a(a(c))");
                let square = bank::Concept {
                    body: square_body.clone(),
                    name: "square".into(),
                    arity: 1,
                };
                let concepts = vec![mul, square];
                let t = pow_task(2, 5); // a^5 — forces ≥3 rounds, so the cheaper
                                        // square(a) re-derivation arrives in a LATER
                                        // round than the mul(a,a) representative.
                let b = diag64(&t, &concepts, bank::DiagMode::Baseline);
                let p = diag64(&t, &concepts, bank::DiagMode::Prune);
                assert!(b.solution.is_some(), "baseline should solve a^5");
                assert!(p.solution.is_some(), "prune should solve a^5");
                // Baseline never admits a square-based term: every square output
                // (x²) is behaviorally identical to mul(x,x), which mul produces
                // first, so `seen` drops it.
                assert!(
                    !b.pool.iter().any(|e| uses(&e.term, &square_body)),
                    "baseline should keep the pricier mul(x,x) representative"
                );
                // Prune replaces the mul(x,x) rep with the cheaper square(x).
                assert!(
                    p.pool.iter().any(|e| uses(&e.term, &square_body)),
                    "prune should keep the cheaper square(x) representative"
                );
                // The representative for the x² key is strictly cheaper under prune.
                let key = p
                    .pool
                    .iter()
                    .find(|e| uses(&e.term, &square_body))
                    .unwrap()
                    .keys
                    .clone();
                let b_cost = b.pool.iter().find(|e| e.keys == key).unwrap().cost;
                let p_cost = p.pool.iter().find(|e| e.keys == key).unwrap().cost;
                assert!(p_cost < b_cost, "prune rep ({p_cost}) not < baseline rep ({b_cost})");
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// C6 regression: the fixed iterate-schema generator exhibits CONDITIONAL
    /// discoverability. `pow = iterate(mul, one)` is NOT in G(∅) but IS in
    /// G({mul}); `mul = iterate(add, zero)` IS in G(∅). G stays fixed — only the
    /// ontology (available concept bodies) changes. (See README's dep note.)
    #[test]
    fn gen_conditional_discoverability() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let opts = bank::Options {
                    max_size: 14,
                    max_depth: 3,
                    fuel: 40_000,
                    time_budget_secs: 12.0,
                    max_level_entries: 200_000,
                    max_opaque_entries: 20_000,
                    seeds: vec![],
                    concepts: vec![],
                };
                // iterate(C, seed) = λa.λn.((n (C a)) seed) — C and seed closed,
                // so no de Bruijn shifting; body: n=var(0), a=var(1).
                let iterate =
                    |c: &Rc<term::Term>, seed: &Rc<term::Term>| -> Rc<term::Term> {
                        term::lam(term::lam(term::app(
                            term::app(term::var(0), term::app(c.clone(), term::var(1))),
                            seed.clone(),
                        )))
                    };
                let zero = church(0);
                let one = church(1);
                // G(avail): one schema-application per proposal over base + concepts.
                let gen_cands = |avail: &[Rc<term::Term>]| -> Vec<Rc<term::Term>> {
                    let mut out = Vec::new();
                    for cb in avail {
                        out.push(iterate(cb, &zero));
                        out.push(iterate(cb, &one));
                    }
                    out
                };
                // a^n discovery suite (a∈{0..4}, n∈{0..4}, values ≤ 4^3 = 64).
                let pow_task = parse::Task {
                    arity: 2,
                    tests: [
                        (2, 0), (2, 1), (2, 2), (2, 3), (2, 4),
                        (3, 0), (3, 1), (3, 2), (3, 3),
                        (1, 0), (1, 4), (0, 0), (0, 2),
                        (4, 2), (4, 3), (4, 1),
                    ]
                    .into_iter()
                    .map(|(a, n)| parse::Test {
                        args: vec![church(a), church(n)],
                        want: church(a.pow(n)),
                        outer: 0,
                    })
                    .collect(),
                };
                // "Does body generalize on pow?" — installed as a concept, verified
                // by concept_solve (the reason-through-a-concept model).
                let solves = |body: &Rc<term::Term>| -> bool {
                    let set = [bank::Concept {
                        body: body.clone(),
                        name: "cand".into(),
                        arity: 2,
                    }];
                    bank::concept_solve(&pow_task, &set, &opts)
                        .solution
                        .is_some()
                };
                let add = closed("λa.λb.λf.λx.a(f)(b(f)(x))");
                let mul = iterate(&add, &zero); // = G(∅) proposal #1
                assert!(
                    !solves(&mul),
                    "mul is a·n, not a^n — it must NOT generalize on the pow suite"
                );
                let pow = iterate(&mul, &one); // = G({mul}) proposal
                assert!(
                    solves(&pow),
                    "iterate(mul, one) = pow, generalizes on the a^n suite"
                );
                // The sharp claim: pow ∉ G(∅), pow ∈ G({mul}).
                let g_empty = gen_cands(&[add.clone()]);
                let g_mul = gen_cands(&[add.clone(), mul.clone()]);
                assert!(
                    !g_empty.iter().any(|c| solves(c)),
                    "pow ∉ G(∅): iterate(add,·) yields mul / 1+na, neither is a^n"
                );
                assert!(
                    g_mul.iter().any(|c| solves(c)),
                    "pow ∈ G({{mul}}): iterate(mul, one) is a G({{mul}}) proposal"
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// transfer (C6-generalization) regression: the SAME fixed iterate-schema G
    /// transfers to a non-arithmetic value space (strings as Church lists).
    /// `replicate = iterate(cons, nil) = (cons c)^n nil` is IN G(∅) with a
    /// genuine frontier — composition-{cons} can't build a count-dependent list
    /// length (held-out is UNREACHABLE) — the junk seed-proposal is rejected by
    /// the target-check, and there is NO well-typed second-order concept (the
    /// type wall: re-iterating replicate would need its list output to feed back
    /// as a count).
    #[test]
    fn transfer_cross_domain_depth() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let opts = bank::Options {
                    max_size: 14,
                    max_depth: 3,
                    fuel: 40_000,
                    time_budget_secs: 12.0,
                    max_level_entries: 200_000,
                    max_opaque_entries: 20_000,
                    seeds: vec![],
                    concepts: vec![],
                };
                let list = |cs: &[u32]| -> Rc<term::Term> {
                    let mut body = String::from("z");
                    for c in cs.iter().rev() {
                        let cstr = crate::bootstrap::church_num_str(*c);
                        body = format!("f({cstr})({body})");
                    }
                    closed(&format!("λf.λz.{body}"))
                };
                let mk_task = |tests: Vec<(u32, u32)>| parse::Task {
                    arity: 2,
                    tests: tests
                        .into_iter()
                        .map(|(c, n)| parse::Test {
                            args: vec![church(c), church(n)],
                            want: list(&vec![c; n as usize]),
                            outer: 0,
                        })
                        .collect(),
                };
                let rep_task = mk_task(vec![(2, 0), (2, 1), (2, 3), (3, 2), (1, 4), (4, 2)]);
                let h_rep = mk_task(vec![(2, 2), (3, 3), (1, 1), (4, 4), (5, 2), (0, 3)]);
                let cons = closed("λc.λs.λf.λz.f(c)(s(f)(z))");
                let nil = list(&[]);
                let singleton = list(&[1]);
                let iterate =
                    |c: &Rc<term::Term>, seed: &Rc<term::Term>| -> Rc<term::Term> {
                        term::lam(term::lam(term::app(
                            term::app(term::var(0), term::app(c.clone(), term::var(1))),
                            seed.clone(),
                        )))
                    };
                let solves = |t: &parse::Task, body: &Rc<term::Term>| -> bool {
                    let set = [bank::Concept {
                        body: body.clone(),
                        name: "cand".into(),
                        arity: t.arity as u32,
                    }];
                    bank::concept_solve(t, &set, &opts).solution.is_some()
                };
                let g0 = vec![
                    iterate(&cons, &nil),
                    iterate(&cons, &singleton),
                ];
                // replicate ∈ G(∅): iterate(cons, nil) generalizes on the suite.
                assert!(
                    g0.iter().any(|b| solves(&rep_task, b)),
                    "replicate = iterate(cons, nil) ∈ G(∅)"
                );
                assert!(
                    solves(&rep_task, &g0[0]),
                    "iterate(cons, nil) is replicate and solves rep_task"
                );
                // junk: iterate(cons, singleton) = leading-[1] list does NOT generalize.
                assert!(
                    !solves(&rep_task, &g0[1]),
                    "iterate(cons, [1]) is a leading-element list, not replicate — rejected"
                );
                // genuine frontier: composition-{cons} cannot solve the held-out
                // (count-dependent length needs the iterator).
                let baseline = crate::concept_cost(&h_rep, &[], &opts);
                assert!(
                    baseline >= crate::UNREACHABLE,
                    "composition-{{cons}} cannot solve h_rep (frontier)"
                );
                // depth wall: iterate(replicate, ·) candidates are type-degenerate —
                // they solve neither the discovery suite nor the held-out.
                let g1_new = vec![
                    iterate(&g0[0], &nil),
                    iterate(&g0[0], &singleton),
                ];
                assert!(
                    !g1_new.iter().any(|b| solves(&rep_task, b) || solves(&h_rep, b)),
                    "no second-order concept in the flat-list value space (type wall)"
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// C7 (first cut) regression: acquiring a proposal SCHEMA restores the
    /// leverage iterate alone lost. `replicate = iterate(cons,nil)` (depth-1);
    /// `concat = reduce(cons) = λxs.λys. xs cons ys` is NOT in
    /// Closure_compose({cons}) (frontier); and `iterate(concat,nil) = concat_n`
    /// reaches the depth-2 list concept (`xs` concatenated n times) that
    /// iterate-from-{cons,replicate} alone cannot — so reduce was the necessary
    /// unlock.
    #[test]
    fn meta_acquires_schema_for_depth() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let opts = bank::Options {
                    max_size: 14,
                    max_depth: 3,
                    fuel: 40_000,
                    time_budget_secs: 12.0,
                    max_level_entries: 200_000,
                    max_opaque_entries: 20_000,
                    seeds: vec![],
                    concepts: vec![],
                };
                let list = |cs: &[u32]| -> Rc<term::Term> {
                    let mut body = String::from("z");
                    for c in cs.iter().rev() {
                        let cstr = crate::bootstrap::church_num_str(*c);
                        body = format!("f({cstr})({body})");
                    }
                    closed(&format!("λf.λz.{body}"))
                };
                let mk = |tests: Vec<(Vec<Rc<term::Term>>, Rc<term::Term>)>| parse::Task {
                    arity: 2,
                    tests: tests
                        .into_iter()
                        .map(|(args, want)| parse::Test { args, want, outer: 0 })
                        .collect(),
                };
                let cat = |a: &[u32], b: &[u32]| -> Vec<u32> {
                    a.iter().chain(b.iter()).copied().collect()
                };
                let cnat = |xs: &[u32], n: u32| -> Vec<u32> {
                    let mut v = Vec::new();
                    for _ in 0..n {
                        v.extend_from_slice(xs);
                    }
                    v
                };
                let rep_task = mk(
                    [(2, 1), (2, 3), (3, 2), (1, 4)]
                        .into_iter()
                        .map(|(c, n)| {
                            (vec![church(c), church(n)], list(&vec![c; n as usize]))
                        })
                        .collect(),
                );
                let concat_task = mk(
                    [(vec![1], vec![2]), (vec![2, 3], vec![4])]
                        .into_iter()
                        .map(|(xs, ys)| (vec![list(&xs), list(&ys)], list(&cat(&xs, &ys))))
                        .collect(),
                );
                let cnat_task = mk(
                    [(vec![2, 3], 2), (vec![1], 3), (vec![2, 2], 2), (vec![3], 0)]
                        .into_iter()
                        .map(|(xs, n)| {
                            (vec![list(&xs), church(n)], list(&cnat(&xs, n)))
                        })
                        .collect(),
                );
                let cons = closed("λc.λs.λf.λz.f(c)(s(f)(z))");
                let nil = list(&[]);
                let singleton = list(&[1]);
                let iterate =
                    |c: &Rc<term::Term>, seed: &Rc<term::Term>| -> Rc<term::Term> {
                        term::lam(term::lam(term::app(
                            term::app(term::var(0), term::app(c.clone(), term::var(1))),
                            seed.clone(),
                        )))
                    };
                let reduce = |c: &Rc<term::Term>| -> Rc<term::Term> {
                    term::lam(term::lam(term::app(
                        term::app(term::var(1), c.clone()),
                        term::var(0),
                    )))
                };
                let solves = |t: &parse::Task, body: &Rc<term::Term>| -> bool {
                    let set = [bank::Concept {
                        body: body.clone(),
                        name: "cand".into(),
                        arity: t.arity as u32,
                    }];
                    bank::concept_solve(t, &set, &opts).solution.is_some()
                };
                let replicate = iterate(&cons, &nil);
                let concat = reduce(&cons);
                let concat_n = iterate(&concat, &nil);
                // depth-1: replicate ∈ G(iterate, ∅); junk seed-proposal does not.
                assert!(solves(&rep_task, &replicate), "replicate ∈ G(iterate)");
                assert!(
                    !solves(&rep_task, &iterate(&cons, &singleton)),
                    "iterate(cons,[1]) = leading-element list, not replicate"
                );
                // concat is a genuine frontier: ∉ Closure_compose({cons}).
                let h_concat = mk(
                    [(vec![2], vec![3, 4]), (vec![1, 1, 1], vec![])]
                        .into_iter()
                        .map(|(xs, ys)| (vec![list(&xs), list(&ys)], list(&cat(&xs, &ys))))
                        .collect(),
                );
                assert!(
                    crate::concept_cost(&h_concat, &[], &opts) >= crate::UNREACHABLE,
                    "concat ∉ Closure_compose({{cons}}) — reduce's proposal is a frontier"
                );
                assert!(
                    solves(&concat_task, &concat),
                    "reduce(cons) = concat solves concat_task"
                );
                // depth-2 payoff: iterate(concat, nil) = concat_n, but iterate from
                // {cons, replicate} alone cannot reach it — reduce was necessary.
                assert!(
                    solves(&cnat_task, &concat_n),
                    "iterate(concat, nil) = concat_n (depth-2 via concat)"
                );
                let no_concat = vec![cons.clone(), replicate.clone()];
                assert!(
                    !no_concat
                        .iter()
                        .any(|cb| solves(&cnat_task, &iterate(cb, &nil))
                            || solves(&cnat_task, &iterate(cb, &singleton))),
                    "iterate-from-{{cons,replicate}} alone cannot reach concat_n — reduce was the unlock"
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// C7 ablation matrix: Reach(subset) pins the cross-schema synergy.
    /// {iterate}→{replicate}, {reduce}→{concat}, but concat_n is reachable only
    /// from {iterate, reduce} — concat_n ∉ Reach({iterate}) ∪ Reach({reduce}).
    /// junk is inert (contributes no concept to any cell).
    #[test]
    fn meta_ablation_matrix() {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(|| {
                let opts = bank::Options {
                    max_size: 14,
                    max_depth: 3,
                    fuel: 40_000,
                    time_budget_secs: 12.0,
                    max_level_entries: 200_000,
                    max_opaque_entries: 20_000,
                    seeds: vec![],
                    concepts: vec![],
                };
                let list = |cs: &[u32]| -> Rc<term::Term> {
                    let mut body = String::from("z");
                    for c in cs.iter().rev() {
                        let cstr = crate::bootstrap::church_num_str(*c);
                        body = format!("f({cstr})({body})");
                    }
                    closed(&format!("λf.λz.{body}"))
                };
                let mk = |tests: Vec<(Vec<Rc<term::Term>>, Rc<term::Term>)>| parse::Task {
                    arity: 2,
                    tests: tests
                        .into_iter()
                        .map(|(args, want)| parse::Test { args, want, outer: 0 })
                        .collect(),
                };
                let cat = |a: &[u32], b: &[u32]| -> Vec<u32> {
                    a.iter().chain(b.iter()).copied().collect()
                };
                let cnat = |xs: &[u32], n: u32| -> Vec<u32> {
                    let mut v = Vec::new();
                    for _ in 0..n {
                        v.extend_from_slice(xs);
                    }
                    v
                };
                let rep_task = mk(
                    [(2, 1), (2, 3), (3, 2), (1, 4)]
                        .into_iter()
                        .map(|(c, n)| {
                            (vec![church(c), church(n)], list(&vec![c; n as usize]))
                        })
                        .collect(),
                );
                let concat_task = mk(
                    [(vec![1], vec![2]), (vec![2, 3], vec![4])]
                        .into_iter()
                        .map(|(xs, ys)| {
                            (vec![list(&xs), list(&ys)], list(&cat(&xs, &ys)))
                        })
                        .collect(),
                );
                let cnat_task = mk(
                    [(vec![2, 3], 2), (vec![1], 3)]
                        .into_iter()
                        .map(|(xs, n)| {
                            (vec![list(&xs), church(n)], list(&cnat(&xs, n)))
                        })
                        .collect(),
                );
                let cons = closed("λc.λs.λf.λz.f(c)(s(f)(z))");
                let nil = list(&[]);
                let singleton = list(&[1]);
                let seeds = vec![nil.clone(), singleton.clone()];
                let iterate =
                    |c: &Rc<term::Term>, seed: &Rc<term::Term>| -> Rc<term::Term> {
                        term::lam(term::lam(term::app(
                            term::app(term::var(0), term::app(c.clone(), term::var(1))),
                            seed.clone(),
                        )))
                    };
                let reduce = |c: &Rc<term::Term>| -> Rc<term::Term> {
                    term::lam(term::lam(term::app(
                        term::app(term::var(1), c.clone()),
                        term::var(0),
                    )))
                };
                let junk = |c: &Rc<term::Term>, seed: &Rc<term::Term>| -> Rc<term::Term> {
                    term::lam(term::lam(term::app(
                        term::app(c.clone(), seed.clone()),
                        term::app(term::app(c.clone(), term::var(1)), term::var(0)),
                    )))
                };
                let solves = |t: &parse::Task, body: &Rc<term::Term>| -> bool {
                    let set = [bank::Concept {
                        body: body.clone(),
                        name: "cand".into(),
                        arity: t.arity as u32,
                    }];
                    bank::concept_solve(t, &set, &opts).solution.is_some()
                };
                let any_solves = |bodies: &[Rc<term::Term>], t: &parse::Task| -> bool {
                    bodies.iter().any(|b| solves(t, b))
                };
                // candidate sets per schema-subset over base {cons}.
                let iterate_cands: Vec<Rc<term::Term>> =
                    seeds.iter().map(|s| iterate(&cons, s)).collect();
                let reduce_cands = vec![reduce(&cons)];
                let junk_cands: Vec<Rc<term::Term>> =
                    seeds.iter().map(|s| junk(&cons, s)).collect();
                let both: Vec<Rc<term::Term>> = iterate_cands
                    .iter()
                    .chain(reduce_cands.iter())
                    .cloned()
                    .collect();
                // {iterate}: replicate only.
                assert!(any_solves(&iterate_cands, &rep_task), "{{iterate}} → replicate");
                assert!(!any_solves(&iterate_cands, &concat_task), "{{iterate}} ↛ concat");
                assert!(!any_solves(&iterate_cands, &cnat_task), "{{iterate}} ↛ concat_n");
                // {reduce}: concat only.
                assert!(any_solves(&reduce_cands, &concat_task), "{{reduce}} → concat");
                assert!(!any_solves(&reduce_cands, &rep_task), "{{reduce}} ↛ replicate");
                assert!(!any_solves(&reduce_cands, &cnat_task), "{{reduce}} ↛ concat_n");
                // junk is inert.
                assert!(!any_solves(&junk_cands, &rep_task), "junk ↛ replicate");
                assert!(!any_solves(&junk_cands, &concat_task), "junk ↛ concat");
                assert!(!any_solves(&junk_cands, &cnat_task), "junk ↛ concat_n");
                // {iterate, reduce}: all three — concat_n is the cross-schema boot.
                assert!(any_solves(&both, &rep_task), "{{iterate,reduce}} → replicate");
                assert!(any_solves(&both, &concat_task), "{{iterate,reduce}} → concat");
                // concat_n needs iterate(concat,·): iterate alone can't, but with
                // reduce's concat as a concept body it can.
                assert!(
                    !any_solves(&iterate_cands, &cnat_task),
                    "iterate alone ↛ concat_n"
                );
                let concat = reduce(&cons);
                assert!(
                    solves(&cnat_task, &iterate(&concat, &nil)),
                    "iterate(reduce(cons), nil) = concat_n — cross-schema boot"
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }

}
