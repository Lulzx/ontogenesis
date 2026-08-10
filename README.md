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
./target/release/supsearch ladder    # Concept Ladder demo (see below)
./target/release/supsearch promote   # autonomous concept promotion (see below)
./target/release/supsearch ablation  # value-representation probe (see below)
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

`cargo test`: 17 pass.

## Concept Ladder

The absurdly-small demo: give the bank raw λ + one operation (`add`), and it
invents the rest. `ladder` solves four rungs — multiplication, square, power,
parity — none of which it was given.

What's real and measured:
- The bank **discovers the canonical Church combinators it was never given**:
  `mul = λa.λb.λc.b(a(c))`, `power = λa.λb.λc.λd.b(a,c,d)` (the textbook
  encodings), plus square. Parity sits on the scale wall.
- The **miner extracts a recurring abstraction** from a recurring family of
  solved tasks and the language grows (`add → add C1`) — the compression-
  mining invention step works.
- The honest wall: **naive seed injection does not collapse search cost.** It
  usually widens it (a size-1 atom seed branches against everything; the
  emitted solution still carries the concept's full λ-body), and a mined
  abstraction is often not the clean textbook concept anyway.
- **The collapse is real, but it lives in the search procedure, not the seed.**
  Once the machine has invented `mul`, a *quotient-aware search* (condition C,
  `bank::concept_solve`) composes the concept over its inputs instead of
  re-deriving it: `a×b×c` 17,270 → **17 states**, and `a×b×c×d` — unsolvable
  raw — is **99 states**. That is the thesis made concrete: a machine has
  acquired a concept only when reasoning *through* it is cheaper than
  re-deriving it. Honest limits: condition C needs the concept's *composition*
  arity (2 for mul, not its λ-arity 3), and it composes given concepts over
  inputs — it does not itself invent new concepts or discover new structure.

## Autonomous promotion (`promote`)

The next step: nobody tells the machine which thing is a concept. Starting from
raw λ + `add` it discovers `mul`, **infers its interface** (composition arity 2,
by the cost structure — wrong arities cost more or fail, not by a label), and
**promotes it iff measured held-out reasoning gain Δ > 0**. Then it uses the
promoted concept to reach what it couldn't before.

What's real and measured:
- `mul` is promoted (Δ > 0: it turns a×b×c×d from raw-✗ into 65 states), and its
  arity is inferred, not given. The frontier moves: a×b×c×d through the 8-fold
  product all become reachable where raw search could not reach them.
- **Two negative controls pass:** the wrong-family `square` and the redundant
  4-fold (Δ = 0, since `mul` alone already reaches the 5-fold) are both declined.
  The machine promotes exactly one concept, and it is the right one.
- **The recursion is honestly bounded by the search:** the 9-fold is a wall no
  product sub-concept breaks (a chunked 8-fold Prim at its correct arity 8 still
  fails). The `ablation` below isolates *why* — it is the composition-search space,
  not the value representation. Genuine multi-generation recursion needs a family
  whose *computation* grows while the *composition* stays cheap — the open frontier.

## Value-representation ablation (`ablation`)

The cleanest probe of one hypothesis from `promote`: is the fold≥9 wall caused by
*value representation* — the pool dropping large Church-numeral results at the
2048-fuel structural-hash cap? To test it in isolation, I added a separate
canonical observation layer with the representation constraint taken literally:

- **`Val` (how computation executes) is untouched.** `apply`, β-reduction, and the
  searchable λ-language are byte-for-byte the same. No `mul(a,b)=>a*b` shortcut, no
  arithmetic primitive, no `Val::Num` in the evaluator.
- **`CanonicalValue` (how results are identified) is a new, separate key.** A value
  is quoted to its normal form, and if it is exactly `λf.λx.f^n(x)` it collapses to
  `ChurchNumeral(n)` — a compact O(1) key — with the *full* evaluation budget, not
  the 2048 hash cap. Otherwise it falls back to a structural hash. `mul(3)(4)`
  canonicalizes to the *same* key as the closed numeral 12 (unit-tested).

The result (`supsearch ablation --budget 20`):

```
fold       struct   canon    ...   pool_s pool_c
 2-fold         2       2                 2     2
 8-fold      1272    1272                39    39
 9-fold         ✗       ✗               104   104
10-fold         ✗       ✗               105   105
11-fold         ✗       ✗               106   106
```

**The two columns are identical.** Every fold structural solves, canonical solves
to the same count; every fold that fails does so at the same pool size. The
canonical path demonstrably keeps large numerals (the meter shows a 6561-node
expansion observed where the 2048 cap would have dropped it), yet reachability is
unchanged. **This is a falsifying negative for the representation-only
hypothesis:** compact semantic storage, at full eval budget, with the evaluator
untouched, does not move the wall on this family.

Why: the family's *answers* are small. The n-fold tests cycle arguments over 1,2,3,
so the largest target value is bounded (8-fold and 9-fold both peak at 216) — no
large value is ever on the critical path, so whether the pool keeps it is
irrelevant. The fold≥9 failure is the **composition search**: with the pool cap
frozen at 64, the binary-mul tree that assembles a 9-ary product from 9 leaf args
is never reached in budget. Raising the cap to 512 unlocks fold9 in **both** modes
(2040 vs 2056 built — structural even slightly cheaper). The earlier "2048 hash
cap drops the value" reading was real but misattributed: the dropped values are
not the ones that gate fold9.

The value-representation separation itself is real and verified (`mul(3)(4)` ≡
`numeral(12)` under the canonical key, no arithmetic involved) — it is just not the
limiting factor for this product family. The honest next lever is composition
search (pool capacity / how concepts assemble), or a family whose computation grows
faster than its composition cost.

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
