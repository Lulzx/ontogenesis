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
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if argv.first().map(String::as_str) == Some("mine") {
        mine(&argv[1..]);
        return;
    }
    let mut args = argv.into_iter();
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
            "--seed-y" => opts.seeds.push(bank::y_combinator()),
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
