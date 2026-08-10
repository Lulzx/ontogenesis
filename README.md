# supsearch

A λ-calculus synthesizer that invents its own vocabulary. No decoder, no
typed DSL, no hand-picked operations. Raw terms, an oracle, a hash table.

The whole trick: the "superposition" is a hash table keyed on behavior. Two
terms that behave identically on the tests are the same term. That collapses
millions of syntactically distinct programs into thousands of distinct ones,
and it's how a machine grows abstractions instead of memorizing them.

No neural network. No new language. No special runtime. ~1,700 lines of Rust,
zero dependencies.

## The loop

1. **Search** (`bank.rs`) — enumerate raw λ-terms bottom-up, dedup by
   normal-form behavior, verify every winner against the oracle. Nothing
   unsound escapes.
2. **Mine** (`bootstrap.rs`) — from solved solutions, pull open subterms,
   abstract them into closed combinators, and merge the ones that behave
   alike. Rank by compression gain. A seed has to earn its place.
3. **Grow** (`bootstrap` subcommand) — solve → mine → inject seeds → repeat.
   The claim is a cost curve `C(L₀) > C(L₁) > …` on held-out tasks.

## Honest fine print

No reference ontology exists to grade seeds against. So "good seed" means
*general*, not *true*; every seed's note says so. A bad seed can slow search
down. It can never produce a wrong answer — the oracle re-verifies each winner.

## Run it

```sh
cargo build --release
./target/release/supsearch mkbench solutions/round0 bench
./target/release/supsearch bootstrap bench \
  --train    clst_fol,clst_hed,clst_map,cnat_mul \
  --holdout  cnat_add,cnat_exp,ctre_rev,ntup_hed,slst_hed \
  --rounds 3 --budget 20
```

`mkbench` rebuilds the benchmark from the verified round-0 solutions, because
this repo doesn't vendor the external task files. Each test is a one-probe
`λA₁…λAₖ. @main(A₁,…,Aₖ)` and the expected output is the solution itself.
They're *synthesized*, not the real benchmark — the curve is meaningful
relative to them, not to LamBench.

## Where it stands

Works. On the 9-solution corpus the miner extracts the head idioms and
validates them on a fresh probe draw; a unit test proves a mined seed actually
computes successor on Church numerals. The Milestone-0 4-task split is too
thin to mine anything (flat curve — set `BOOT_DEBUG=1` to see why). And naive
seed injection widens search instead of narrowing it: seeds are size-1 atoms,
so one that isn't in the target just adds branching (median cost went *up*,
0.016s → 0.265s, in one run). The real lever is more raw solves → bigger
recurring idioms that pre-build what search can't cheaply enumerate. That's
the wall — a scale problem, not a mechanism failure.

`cargo test`: 16 pass.

## Layout

```
src/          live track: bank, bootstrap, nbe, term, parse
src/legacy/   frozen 120/120 engine
bench/        synthesized Milestone-0 tasks
legacy/       frozen engine outputs + RESULTS
```

## Legacy

Before this, supsearch solved all 120 LamBench tasks — with a hand-built
vocabulary: a typed DSL over decoded values, ~70 operations, a λ stdlib. That
9→120 jump is the vocabulary's payoff, which is why it's frozen in
[`legacy/`](legacy/RESULTS.md). The raw bank, no vocabulary: 9/120. This
project asks whether the loop above turns those 9 into the vocabulary.

## License

MIT.
