# ontogenesis

A λ-calculus synthesizer that invents its own vocabulary.

No decoder. No typed DSL. No hand-picked operations. Raw terms, an oracle, a hash table.

The whole trick, one line: **two terms that behave identically on the tests are the same term.** That behavior-keyed hash table collapses millions of syntactically distinct programs into thousands of distinct ones. That's how a machine grows abstractions instead of memorizing them.

~1,700 lines of Rust. Zero dependencies. No neural network, no new language, no special runtime.

## The loop

1. **Search** — enumerate raw λ-terms bottom-up, dedup by normal-form behavior, verify every winner against the oracle. Nothing unsound escapes.
2. **Mine** — from solved solutions, pull open subterms, abstract them into closed combinators, merge the ones that behave alike. Rank by compression gain. A seed earns its place.
3. **Grow** — solve → mine → inject → repeat. The claim is a cost curve `C(L₀) > C(L₁) > …` on held-out tasks.

## The trick, measured

Give the bank raw λ + one operation (`add`). It invents the rest.

```
ladder →  mul = λa.λb.λc.b(a(c))   square   power   parity
```

None of these were given. These are the textbook Church combinators, found, not handed over.

The collapse is real — but it lives in the **search**, not the seed:

| problem | raw search | after it invents `mul` |
|---|---|---|
| `a×b×c` | 17,270 states | **17** |
| `a×b×c×d` | unsolvable | **99** |

A machine has acquired a concept only when reasoning *through* it is cheaper than re-deriving it. That's the thesis, made concrete.

## Honest fine print

No reference ontology exists to grade seeds against. So a "good seed" means *general*, not *true* — every seed's note says so.

A bad seed can slow search down. It can never produce a wrong answer: the oracle re-verifies every winner.

The walls are honest too:

- Naive seed injection *widens* search (one run: median cost 0.016s → 0.265s). A size-1 atom seed branches against everything.
- The 9-fold product is a wall no product sub-concept breaks. `ablation` proves it's the composition search, not the value representation — compact semantic keys at full eval budget move nothing. Same columns, identical numbers.

The real lever is more raw solves → bigger recurring idioms. That's the wall — a scale problem, not a mechanism failure.

## Run it

```sh
cargo build --release
./target/release/supsearch mkbench solutions/round0 bench
./target/release/supsearch bootstrap bench --train ... --holdout ... --rounds 3 --budget 20
./target/release/supsearch ladder    # it invents mul/square/power from raw λ + add
./target/release/supsearch promote   # it picks mul itself, infers its arity, promotes it
./target/release/supsearch ablation  # why the fold-9 wall isn't the value representation
```

`cargo test`: 17 pass.

## Layout

```
src/        live track: bank, bootstrap, nbe, term, parse
src/legacy/ frozen 120/120 engine
bench/      synthesized tasks
legacy/     frozen engine outputs + RESULTS
```

## Legacy

Before this, the engine solved all 120 LamBench tasks — with a hand-built vocabulary: a typed DSL over decoded values, ~70 operations, a λ stdlib. That 9→120 jump is the vocabulary's payoff. This project asks whether the loop above turns the 9 into the vocabulary.

## License

MIT.
