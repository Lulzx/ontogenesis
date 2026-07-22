# Results — LamBench track

**108/120 tasks certified** by LamBench's own harness (`bun src/check.ts`), tying the
leaderboard leaders. No LLM, no learned parameters, no reasoning tokens.

| Model / system | Score |
|---|---|
| GPT-5.3 Codex | 108/120 |
| Opus 4.6 | 108/120 |
| **supsearch (enumerative + generic stdlib, no model)** | **108/120** |
| Opus 4.7 / Gemini 3.1 Pro | 106/120 |
| GPT-5.4 | 96/120 |
| GPT-5.5 | 89/120 |
| Sonnet 4.6 | 87/120 |

## Wall-clock

Single machine (Apple Silicon), single process, one clean run over all 120 tasks:

- **3 min 27 s** total, including the fallback budgets burned on the 12 unsolved tasks
- **144 s** cumulative search+verify for the 108 solved tasks (~1.3 s average; slowest 14 s)
- Most tasks solve in **milliseconds**; the time is dominated by internal verification
  of division/gcd-style compiled programs

Reproduce: `cargo build --release && ./target/release/supsearch lambench/tsk --out final --timeout 5`
then `cd lambench && bun src/check.ts ../final`.

## Per family

| Families | Score | Mechanism |
|---|---|---|
| cnat, snat, cbin, sbin | 40/40 | arithmetic DSL search over decoded values; affine unary core; binary boundary adapters |
| clst, slst, ntup | 30/30 | list-structure ops + uninterpreted-function atoms (`F(x)` tracked symbolically) |
| ctre, stre | 18/20 | Scott-tree internal representation; CPS-affine stateful traversals; BFS queue; bit-reversal via deinterleave |
| cadt, sadt | 20/20 | generic descriptor interpreter: variadic collectors, dynamic case analysis, generic fold/unfold; two verified templates (ctr, mrg) |
| algo_, \*_fft | 0/12 | **the wall**: BF interpreter, SAT, Sudoku, maze, MST, hull, rasterization, λ-evaluator, type checker, TSP, balanced-ternary DFT |

Attribution chain from the frozen baseline (git tag `baseline-raw-search-50`):
50 (raw λ-enumeration era: 9 → semantic pivot: 50) + 20 binaries + 18 trees + 20 ADTs = 108.

## How it works

1. **Decode** — test inputs and expected outputs are λ-normal-forms whose shape *is* the
   encoding grammar; ~600 lines of recognizers turn them into native values (nats, lists,
   trees, constructor trees, opaque atoms).
2. **Search** — a bottom-up enumerator with behavioral deduplication finds the connecting
   function in a ~40-op typed DSL over native values. Division is found in milliseconds;
   totient is found as `Count(Range1(n), λd. Eq(Gcd(d, n), 1))` and generalizes beyond
   the examples.
3. **Compile** — each DSL op has a handwritten implementation in a ~90-definition Lamb
   standard library (affine style, Scott-encoded internal representations, explicit
   `@sdup` duplication); per-encoding adapters sit at the boundary.
4. **Verify** — the compiled program is checked against every test with an internal
   normalizer, then certified by LamBench's own referee. Nothing unsound can escape.

## Honesty notes

- **The stdlib is hand-primed, not mined.** The compounding-library claim (abstractions
  mined from the system's own solutions) is demonstrated only in miniature (round-1
  mining sped re-solves ~50×); the ADT/tree generic machinery was written by hand, as
  any compiler backend is. The synthesis contribution is *which composition* of ops
  matches the examples — decided by search alone.
- **Two tasks per ADT family use verified templates** (`ctr`: outputs are functions, not
  data; `mrg`: F arrives concrete with pre-reduced expectations). The template is a fixed
  generic program accepted only after passing every test.
- **Clean room**: the search never reads `lambench/lam/` (reference solutions). Inputs
  are task descriptions and test cases only. Reference solutions are used solely by
  LamBench's own scorer to compute its size-score denominator.
- **The oracle is finite.** For every task with concrete (non-free-variable) test inputs,
  a Böhm-style lookup table would pass without solving anything — roughly 70 tasks are
  memorizable in principle. We do not do this; we note it because it marks the exact
  boundary of what "incorruptible verification" means: an oracle is only as strong as
  the spec it checks. Taelin's higher-order tests, which pass free variables, are
  immune — universally-quantified tests are the better oracle design.

## Findings

1. **The referee's cost model is part of the spec.** `lam` evaluates call-by-name with
   no sharing; any value consumed twice is re-evaluated, and iteration state chained
   through recursion goes exponential. The stdlib therefore had to be written in affine
   style with explicit duplication nodes — *the exact bookkeeping interaction nets
   automate*. Taelin's benchmark quietly contains an argument for Taelin's runtime; the
   counter-argument is quantitative: confronted as an engineering discipline rather than
   a foundational problem, it cost ~40 lines of stdlib.
2. **Search in the semantic space; verify in the wire format.** Direct λ-term enumeration
   stalled at term size ~13 (9/120 after CPU-hours). Decoding to native values collapsed
   whole categories to milliseconds — representation choice dominates raw search power.
3. **Most apparent search failures are specification-parsing failures.** The single bug
   that hid every list task was in the spec decoder (outer-binder stripping), not the
   search. Oracle-side code deserves the same rigor as the enumerator.
4. **Some benchmark tasks have zero semantic content.** The encoding-conversion tasks
   (`fol`) are natively the identity function — all the work lives in the adapters.
