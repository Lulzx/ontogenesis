mod bank;
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
    let mut args = std::env::args().skip(1);
    let mut tsk_dir: Option<PathBuf> = None;
    let mut out_dir = PathBuf::from("out");
    let mut filter = String::new();
    let mut opts = bank::Options::default();

    while let Some(a) = args.next() {
        match a.as_str() {
            "--out" => out_dir = PathBuf::from(args.next().expect("--out DIR")),
            "--filter" => filter = args.next().expect("--filter PREFIX"),
            "--max-size" => opts.max_size = args.next().unwrap().parse().unwrap(),
            "--max-depth" => opts.max_depth = args.next().unwrap().parse().unwrap(),
            "--fuel" => opts.fuel = args.next().unwrap().parse().unwrap(),
            "--timeout" => opts.time_budget_secs = args.next().unwrap().parse().unwrap(),
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
