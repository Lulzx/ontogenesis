# supsearch

*Search, ratcheted by verification, compounding through abstraction — in a repo you can read in an afternoon.*

## What this is

supsearch is a program synthesis engine built on a simple discipline: enumerate candidate programs bottom-up, merge every candidate that behaves identically on the given examples into a single equivalence class, verify survivors against an oracle that cannot be gamed, and feed every solved problem back into the system — as training data for a learned proposal prior, and as new named primitives in a growing library. It is roughly 1,300 lines of Rust. It has no runtime of its own, no new language, and no exotic theory. That is the point.

The project exists to demonstrate a thesis: the durable core of "symbolic AI via optimal computation" — the vision Victor Taelin has pursued for a decade through interaction nets, the HVM, and superposed program search — does not require the substrate he is building. The value lives in three ideas: share work across candidates, verify with a binary oracle, and grow a library of solved abstractions. All three fit in ordinary code on ordinary hardware. The fourth idea, the one his learning-free architecture omits, is what makes the system improve over time: a learned prior over the search space, trained on the system's own victories. supsearch keeps his critic and adds the learner.

## Where it came from

This design fell out of a longer first-principles exercise: if you rebuilt the AI stack from scratch — no tokenizers, no train/deploy split, a memory hierarchy with a nightly consolidation loop, compute allocated by surprise, learning driven by verified outcomes rather than human labels — what would intelligence actually consist of? The answer we converged on: intelligence is a process, not a substance. Compression gives you a world model; search lets you exceed your training; verification is the ratchet that makes search compound; abstraction mining turns wins into reusable pieces; and amortizing search results back into the model turns today's laborious reasoning into tomorrow's intuition.

That loop needs a domain to ignite in — one with dense free verification, cheap problem generation, and tolerable failure cost. Program synthesis from examples is the purest such domain that exists: a candidate either matches the spec or it dies, and reality grades the homework for free.

Taelin arrived at the same summit from the opposite face. His bet is that if the computational substrate is made mathematically perfect — Lévy-optimal reduction, inherent parallelism, superpositions that evaluate exponentially many programs while paying for shared substructure once — then intelligence becomes a search query, no learning required. The bet is beautiful, and its cost has been a decade of substrate-building before value extraction. Our claim, made respectfully and testably: for every concrete search demo he has published, the superposition's benefit is captured by behavioral hashing — deduplicating candidates by their outputs on the test inputs — and the substrate tax was never owed.

## How it works

**The DSL.** A small typed language: integers, booleans, lists, lambdas, ~25 primitives (arithmetic, comparison, map/filter/fold, conditionals). For the λ-calculus track, the term language is pure lambda calculus with de Bruijn indices and a normalizer.

**The bank.** The heart of the engine. Programs are enumerated by size: everything of size 1, then combinations of smaller entries into size 2, 3, and so on, via type-correct constructors. Every candidate is evaluated on the actual test inputs as it is built, and its vector of outputs is hashed. Two programs with identical output vectors are behaviorally indistinguishable for this problem — so they merge into one equivalence class, and only one representative is ever extended further. Millions of syntactically distinct programs collapse into thousands of behaviorally distinct ones. This is the superposition, implemented as a hash table.

**The oracle.** A candidate passes only if it matches every example exactly (plus optional property-based fuzzing against a reference). There is no reward model, no learned judge in the loop's critical path, and therefore nothing to reward-hack. This is the property worth taking from the symbolic tradition without compromise.

**The library.** Every solved problem is a new fact about the problem distribution. Solutions are anti-unified against previous solutions; recurring fragments are factored out, named, and added to the DSL as primitives (the DreamCoder move). The DSL grows toward the problems, average solution size shrinks, and shrinking size is exponential search relief. This is abstraction mining, done exactly rather than statistically.

**The prior.** At every enumeration choice point — which constructor to expand, which hole to fill — a small learned model ranks the options. It starts as nothing (uniform), becomes a production-bigram model, and graduates to a small transformer mapping a spec embedding to production logits. Its training data is the system's own solved corpus: every win sharpens the proposal distribution, so effective search depth grows week over week with no new code. This is the ratchet, and it is the one component the learning-free vision has no seat for — despite its author's own daily experience that neural agents optimize his evaluator better than anything else.

## The benchmark plan

The project proves itself on targets its intellectual counterparty chose:

**Phase 1 — the gists.** Taelin's published demos, reproduced with his exact inputs: the ADD-CARRY hunt (16 missing truth-table bits from two I/O pairs — his flagship 7,277x-speedup demo, which behavioral dedup plus per-bit constraint propagation solves near-instantly), the Fast Discrete Program Search series (bitstring functions from examples), SAT via superposition (reproduced with SIMD lanes — the CPU has been a superposition machine since MMX), and rule induction on his A::B rewrite system, recovering the rules of his own puzzle from its example traces. Each lands in `taelin_bench/` with a link to the source, his inputs, our matching outputs, and lines-of-code side by side.

**Phase 2 — LamBench.** His 120-problem pure-λ-calculus benchmark (MIT licensed) is, conveniently, an LLM leaderboard: frontier models score up to 108/120; several score zero. Its format — problem description, test cases as expressions with expected normal forms, pass only on exact match — is supsearch's native input. The plan: parse `tsk/`, clean-room a Lamb-compatible normalizer, enumerate λ-terms with normal-form-keyed behavioral dedup, mine the library across categories, emit `.lam` files, and grade them with *his own harness* so the scores are certified by his referee. The reference solutions in `lam/` are never read by the search — stated loudly, because the result's credibility rests on it.

Expected shape of the result: the encoding-arithmetic and list categories (roughly 40–60 problems) fall to raw enumeration; sorting, serialization, and tree operations fall after library learning kicks in; the `algo_` category — BF interpreter, Sudoku, FFT, TSP — stands as a wall, honestly marked. That wall is the point, not an embarrassment: it is the empirical boundary between search's kingdom and the prior's kingdom, and it motivates phase 3.

**Phase 3 — the hybrid.** An LLM proposes a sketch with typed holes; supsearch fills the holes exhaustively and returns something *proven* against the oracle. Neural reads intent; symbolic guarantees correctness. Entered on the LamBench leaderboard as a row that is not a model, scoring above pure LLMs at a fraction of the tokens.

## The deliverable chart

One graph carries the entire thesis: solve rate at a fixed search budget, measured weekly, with the prior and library learning enabled — climbing, with no new code, only accumulated solutions. Capability rising from the system's own experience is the whole argument for search-ratcheted-by-verification made visible on one axis.

## Roadmap

- **Week 1** — DSL, evaluator, bank, enumerator, verifier. Solve SyGuS bitvector/string benchmarks (the field's shared yardstick — instant credibility or instant refutation).
- **Week 2** — `taelin_bench/` phase 1 complete. FlashFill-class string transformations from 2–3 examples.
- **Week 3–4** — Lamb normalizer, LamBench harness integration, first leaderboard run. Library learning on.
- **Week 4–6** — Learned prior on. Publish the compounding chart. Open the PR adding the non-model leaderboard row.
- **Later** — the hybrid; then the engine's real destiny: serving as the formal back half of a personal automation system, where a byte-stream watcher mines repeated behavior from one person's computational life and supsearch turns each repetition into a proven script. Replay-against-your-own-history as the oracle; your past as the test suite.

## What we deliberately threw away

Lévy-optimal reduction (observational equivalence gets the sharing where it counts). A new language and runtime (Rust plus a 25-primitive DSL tests every claim). Full λ-generality outside the LamBench track (a closed DSL makes typing and enumeration trivial; grow it only when problems demand). GPU graph-rewriting (batch the evaluator onto GPU later if profiling says so — output-vector evaluation is dense and regular, which is to say, actually GPU-shaped). Each discard trades theoretical maximalism for a working system this quarter.

## The stance

This is not "HVM is pointless." Lévy-optimality is real theory, λ-term enumeration is more general than any closed DSL, and the incorruptible critic is the single best idea in the symbolic program — we build on it without modification. The claim is narrower and sharper: the demos never needed the cathedral, and the cathedral's architect left out the congregation. A perfect verifier without a learner is a ratchet that never turns; a learner without a perfect verifier is a ratchet that slips. supsearch bolts them together in the smallest possible machine and lets the chart do the talking — which is, in the end, the same test everything here has been held to: if the idea is real, it survives being made small.

---

## Status (LamBench track)

**108/120 certified by LamBench's own harness — tied with the leaderboard leaders
(GPT-5.3 Codex, Opus 4.6), with no model, no tokens, and ~3.5 minutes of wall-clock.**
See [RESULTS.md](RESULTS.md) for the full table, timing, attribution chain, findings,
and honesty notes. The 12 unsolved tasks (algo_ ×10, fft ×2) are the honest wall —
phase-3 hybrid territory.

**Clean-room guarantee:** the search never reads `lambench/lam/` (the reference
solutions). The only inputs are the task descriptions' test cases in `lambench/tsk/`.

### Usage

```
git clone https://github.com/VictorTaelin/lambench   # tasks + referee (MIT)
cargo build --release
./target/release/supsearch lambench/tsk --out out --timeout 5
cd lambench && bun src/check.ts ../out               # certify with Taelin's referee
```
