# Legacy: the 120/120 semantic engine (frozen)

Before the [ontology-bootstrap](../docs/README.md) pivot, supsearch was a program
synthesis engine that solved all 120 tasks of
[LamBench](https://github.com/VictorTaelin/LamBench) with a hand-built
semantic vocabulary: a typed DSL (~70 operations) over decoded Church/Scott
values, compiled through a hand-written λ standard library, verified against
the benchmark's oracle. It scored 120/120, certified by LamBench's own
referee (best LLMs: 108/120).

That work is **frozen** — kept compilable and runnable, but not developed
further. The 9→120 score jump is the payoff of the human vocabulary, not an
invention of it; the live project now asks whether the useful abstractions
can be re-derived from raw λ + behavior + compression, with no vocabulary.

## Contents

- `outsem/` — the 120 emitted `.lam` solutions, as found by the engine.
- `final/` — the certified final run (120/120).
- `out2/` — an earlier output run.
- `solutions_semantic/` — 108 `.lam` reference solutions.
- `RESULTS.md` — timings, per-task attribution, and the complete honesty
  audit of the 120/120 run.
- `certify.sh` — certify emitted solutions with LamBench's own harness
  (`cd lambench && bun src/check.ts ../out`).

## The code

The modules `src/legacy/*.rs` (`sem`, `decode`, `dsl`, `compile`) still
compile and the `sem` / `grow` / `mine` / `validate` subcommands still run —
they are the frozen track's interface. The shared raw-λ core (`bank`, `nbe`,
`term`, `parse`) is live and used by the bootstrap track too.

This directory does **not** vendor `lambench/`; to re-certify you need a
checkout of [LamBench](https://github.com/VictorTaelin/LamBench) in the
repo root, plus [Bun](https://bun.sh).
