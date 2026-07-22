# supsearch

supsearch is a program synthesis engine. It solves all 120 tasks of
[LamBench](https://github.com/VictorTaelin/LamBench), Victor Taelin's pure
λ-calculus benchmark, scoring 120/120 as certified by the benchmark's own
referee. The best LLMs score 108/120.

It is about 2,500 lines of Rust with no dependencies. There is no neural
network, no new language and no special runtime: candidate programs are
enumerated bottom-up, deduplicated by their behavior on the test inputs,
compiled to λ-calculus through a small hand-written standard library, and
verified against the benchmark's oracle. Every solution in `outsem/` was
produced by the engine; none was written by a human or an LLM.

## News

- 2026-07-22: 120/120. The `algo_` category (brainfuck interpreter, SAT,
  Sudoku, TSP, convex hull, maze, MST, λ-evaluator, type checker, Bresenham)
  and both FFT tasks solved; the four remaining template-solved ADT tasks
  (`ctr`, `mrg`) re-derived by search. TSP was found as a composition:
  `MinL(MapB(Perms(Range0 n), λp.CycleCost(D, p)))`.
- earlier: 108/120 with the semantic-search pipeline (ties the LLM
  leaderboard leaders); 9/120 with raw λ-term enumeration.

## Results

| Score | Entry |
|-------|-------|
| **120/120 (100%)** | **supsearch (this repo — a search engine, not a model)** |
| 108/120 (90.0%) | GPT-5.3 Codex |
| 108/120 (90.0%) | Opus 4.6 |
| 106/120 (88.3%) | Opus 4.7 |
| 106/120 (88.3%) | Gemini 3.1 Pro |
| 96/120 (80.0%) | GPT-5.4 |
| 89/120 (74.2%) | GPT-5.5 |
| 88/120 (73.3%) | GPT-5.2 |
| 87/120 (72.5%) | Sonnet 4.6 |
| 72/120 (60.0%) | GPT-5.4-mini |
| 57/120 (47.5%) | Qwen 3.6 Plus |
| 55/120 (45.8%) | Grok 4.20 |
| 55/120 (45.8%) | DeepSeek v4 Pro |
| 47/120 (39.2%) | Gemini 3.1 Flash Lite |
| 38/120 (31.7%) | GLM 5.1 |
| 34/120 (28.3%) | Kimi K2 Thinking |
| 32/120 (26.7%) | MiMo v2.5 Pro |
| 30/120 (25.0%) | Gemma 4 31B IT |
| 26/120 (21.7%) | Kimi K2.6 |
| 14/120 (11.7%) | GPT-5.3 Codex Spark |
| 0/120 (0.0%) | GPT-5.1, Opus 4.5, Sonnet 4.5 |

Model scores are the official LamBench rankings. The supsearch run is
certified locally by LamBench's harness (`bun src/check.ts`); it holds no
official leaderboard row. Wall-clock per task ranges from 16 ms to 826 s.

## How it works

1. **Decode.** Test inputs and expected outputs are λ normal forms whose
   shape is fixed by the task's encoding (Church/Scott naturals, lists,
   trees, ADTs, tuples, balanced ternary, the GN number tower). Recognizers
   turn them into native values.
2. **Search.** A small typed DSL (~70 operations) is enumerated bottom-up by
   expression size. Each candidate is evaluated on the real test inputs;
   candidates with identical output vectors are merged, so millions of
   syntactically distinct programs collapse into thousands of behaviorally
   distinct ones. The first expression matching all outputs wins.
3. **Compile.** The winning expression is emitted as a `.lam` program
   through a hand-written λ standard library, with encoding adapters at the
   boundaries.
4. **Verify.** The program is checked against every test by an internal
   normalizer, then graded by LamBench's own referee. Nothing unsound can
   escape.

A direct λ-term enumerator (normal-form-keyed behavioral dedup over pure
λ-terms) exists as a fallback track; it solves 9/120 on its own. Searching
in the semantic space instead of the term space is what moves the score.

## Attribution

Search finds different amounts per task, and the split is stated plainly:

- ~50 tasks: raw compositional search over arithmetic/list operations.
- ~58 tasks: search over generic per-encoding machinery (descriptor-driven
  ADT fold/unfold, serializers, constructor builders, tree operations).
- 1 task (`algo_tsp`): a full algorithm discovered as a composition, with
  no TSP-specific primitive in the library.
- 11 tasks: one hand-written algorithm-library routine each (BFS, hull,
  Bresenham, SAT, Sudoku, STLC checking, β-normalization, brainfuck, GN
  DFT); search contributes decoding, selection, wiring and verification.

The library is human capital, as in any compiler; the engine decides what
to call, how to wire it, and whether the result is right. An earlier run in
which an LLM wrote six passing solutions directly was rejected and deleted.
The reference solutions shipped in `lambench/lam/` are never read.

## Usage

Requires Rust and [Bun](https://bun.sh) (for the referee), plus a checkout
of [LamBench](https://github.com/VictorTaelin/LamBench) in `lambench/`.

```sh
cargo build --release

# solve everything into outsem/
./target/release/supsearch lambench/tsk --out outsem

# certify with LamBench's own harness
cd lambench && bun src/check.ts ../outsem
```

Useful environment variables: `SUP_BUDGET` (search deadline in seconds,
default 90), `SUP_DEBUG` (decode dumps), `SUP_PROBE=OpName` (evaluate one
candidate against all tests), `SUP_NOOPS=Op1,Op2` (disable operations).

## Technical notes

- **The referee's cost model is part of the spec.** LamBench's `lam`
  evaluator is call-by-name with no sharing: any computed value consumed
  twice is recomputed, and iteration state chained through recursion goes
  exponential. The λ standard library is therefore written in affine style
  with explicit CPS duplication (`@sdup`) — hand-rolled interaction-net dup
  nodes, i.e. exactly the bookkeeping HVM automates, done in ~40 lines.
- **The tests are the real spec.** The task prose does not tell you that
  Bresenham rounds half-steps down, that both FFTs take input in
  bit-reversal order while only `stre_fft` also emits it, or what the
  Church-tree normal forms look like (plain constructor spines at value
  roots, self-passing spines `n(a,b,n,l)` / `l(x,n,l)` below — a shape that
  lets converters recurse by self-application, no Y combinator needed).
  All of it is recovered from the tests.
- **The FFT number system.** GN(m) = GN(m−1)[w] with w² = the previous
  root and w₀ = −1, over balanced-ternary integers. Multiplication by a
  root of unity is structural: `mulw(B(a,b)) = B(mulw(b), a)`, negation at
  scalars. An O(N²) DFT over this tower passes with seconds to spare.
- **Enumeration fuzzes your own primitives.** Bottom-up search feeds every
  operation every value it can build; it found a latent nontermination in
  the ADT deserializer (a 1-constructor descriptor consumes no tag bits but
  recurses into fields) that the ADT tasks themselves cannot trigger.
- **The finite oracle is memorizable.** ~70 tasks have fully concrete test
  inputs and would fall to a lookup table; we don't do this. Tasks whose
  tests pass free variables are immune — universally quantified tests are
  the better oracle design.

Full timings, per-task notes and the complete honesty audit are in
[RESULTS.md](RESULTS.md).

## Background

The project tests a thesis about Taelin's optimal-computation program: the
durable ideas — share work across candidates, verify with an oracle that
cannot be gamed, grow a library of solved abstractions — fit in ordinary
code on ordinary hardware. The superposition becomes a hash table keyed on
behavior; the substrate tax was never owed. LamBench was chosen because it
is his benchmark, graded by his referee.

## License

MIT. LamBench is by Victor Taelin (MIT), vendored under `lambench/`.
