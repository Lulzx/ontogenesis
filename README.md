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
ladder →  mul = λa.λb.λc.b(a(c))   square   power   parity   (raw λ + add)
```

None of these were given. These are the textbook Church combinators, found, not handed over.
But *discovering* a behavior and *acquiring* it are different acts. Acquisition is a
measured decision: promote a candidate only if, installed as a Prim, it drops the held-out
quotient-search cost (Δ > 0) — never because its subterm recurs often.

```
candidate       held-out quotient          verdict
mul             a×b×c ✗→17                 ACQUIRE   (frontier gain)
square          x³ already 3 (Δ ≤ 0)       REJECT    (mul covers it)
power           x^(n+1) ✗→16               ACQUIRE   (frontier gain)
mined idiom     a×b×c ✗→✗ (Δ ≤ 0)          REJECT    (recurs, buys nothing)
```

So: **c is a concept for a distribution ⟺ installing c makes reasoning on it cheaper.**
square is discovered but not acquired; the recurring mined idiom is not acquired. mul and
power earn their slots because each unlocks a frontier mul-alone cannot reach.

The collapse is real — but it lives in the **search**, not the seed:

| problem | raw search | after it invents `mul` |
|---|---|---|
| `a×b×c` | 17,270 states | **17** |
| `a×b×c×d` | unsolvable | **99** |

A machine has acquired a concept only when reasoning *through* it is cheaper than re-deriving it. That's the thesis, made concrete.

## Honest fine print

No reference ontology exists to grade seeds against. So a "good seed" means *general*, not *true* — every seed's note says so.

A bad seed can slow search down. It can never produce a wrong answer: the oracle re-verifies every winner.

Acquisition is counterfactual, not syntactic. A candidate becomes a concept only if, installed as a Prim, it reduces held-out quotient-search cost (Δ > 0). "Occurs often" and "is a textbook combinator" do not earn a slot on their own — `ladder` shows square (discovered, but mul already covers its held-out) and a recurring mined idiom both rejected, while mul and power are acquired for the frontier each unlocks.

Concepthood is *relative to the current ontology*, not intrinsic. `ontogen` evaluates the same raw-discovered candidates (mul/square/power) under several ontologies and measures `Gain(c | D, O)`. square is ACQUIRED under ∅ (it makes x² solvable) but rejected under {mul} — `mul(x,x)` already reaches x². power is the mirror image: worthless under ∅ (`x^(n+1)` stays ✗) and valuable only under {mul}, because `x^(n+1) = mul(x, power(x,n))` needs mul as a substrate. Each is valuable exactly where the other is not — a property of the candidate × ontology pair, not of the candidate alone.

**Conditional usefulness does not imply conditional discoverability.** `dep` records the negative: in the Grzegorczyk arithmetic tower, no dependency chain `C₁ ⇒ C₂ ⇒ C₃` (each C_{k+1} both *depends* on C_k to be found and *extends* what O_k can express) is constructible with the current searches. pow is base-findable (Church pow = λm.λn.n m, ~6 nodes), so mul→pow is not a discovery dependency; tet is not findable even through {mul,pow} at any depth, and a×b×c×d is raw-✗ and via-{mul}-✗ through bottom-up too (it needs depth 4 > max_depth 3; only pool-composition reaches it, and that is the usefulness mechanism). The structural reason is a closure argument: anything discovered by *composing* O lies in the composition closure of O, so it cannot be the very thing that extends that closure; and bottom-up finds compact combinators (mul, pow) directly but cannot synthesize deeper recursion (tet). Both searches sit on the same side of the wall. Concept-aware *reasoning* exists; concept-aware *generation of closure-extending hypotheses* does not — the search generator itself has to change.

C6 answers the `dep` negative by moving concept-awareness into the **generator** (`gen`). The ontology must change the *grammar* of what can be proposed, not just provide atoms to compose. `G` is a single fixed production — the bounded self-iteration schema `iterate(C, seed) = λa.λn.((n (C a)) seed)`, realized in pure λ via the numeral iterator (no new runtime primitive) — applied once per proposal; the acquisition loop (O grows) builds depth. G stays fixed; only the ontology changes:

```
Gen  candidate raw(base)?  via G(O_{k-1})? via G(O_k)?  useful(H)          verdict
0    mul       ✓           —               ✓            a×b×c×d ✗→65      frontier ACQUIRE
1    pow       ✓           ✗               ✓            x^(n+1)  ✗→16      frontier ACQUIRE
2    tet       ✗           ✗               ✓            tower    121→11    search  ACQUIRE
acquired trajectory: O0=∅ → O1={mul} → O2={mul,pow} → O3={mul,pow,tet}
```

The sharp claim is G-conditional, not raw: raw finds pow (Church compression), but `pow ∉ G(∅)` yet `pow ∈ G({mul})`, and `tet ∉ G({mul})` yet `tet ∈ G({mul,pow})` — a dependency chain `C₁ ⇒ C₂ ⇒ C₃` that composition could never build, exactly the chain `dep` showed is absent from the current searches. tet earns a SEARCH gain (121→11), not a frontier, because composition-{mul,pow} overfits the tower holdout to a^(a^n) for the representable bases; a genuine tet frontier needs tower(2,4)=65536 or 3^27, both beyond the 2048 fuel — an honest range limit, not a forced result. G also proposes non-targets (1+na, constant 0); they fail the target-task verification and are not acquired — the gate is selective.

**G is domain-independent; its depth is value-space-bound.** `transfer` runs the byte-identical `iterate(C, seed)` schema in a non-arithmetic domain — strings as Church lists, base `{cons}` (prepend). `replicate(c,n) = iterate(cons,nil) = (cons c)^n nil ∈ G(∅)`, and it earns a genuine frontier (✗→3): composition-{cons} cannot build a count-dependent list length, because that needs the iterator. The junk proposal `iterate(cons,[1])` (a leading-element list) fails the target-check and is rejected — the gate transfers. But there is **no second-order concept** in this value space: `iterate`'s second argument is always an iteration count, so re-iterating replicate would require replicate's output (a list) to feed back as a count — a type the flat-list space does not carry. So the multi-level chain (mul→pow→tet) is the signature of a *self-iterable* value space (the numerals), not of G. This refines the C6 claim to: *a domain-independent higher-order proposal schema lets an acquired ontology restructure hypothesis generation* — at depth 1 in every domain, at depth >1 only where the value space itself iterates. Going deeper in non-arithmetic domains is exactly the C7 problem (acquiring the proposal schemas), not a tweak to `iterate`.

**C7 (first cut) — acquire the proposal schemas themselves.** `meta` makes the meta-operators candidates: the machine is given a fixed meta-space M = {`iterate`, `reduce` (= fold with a free seed, `λxs.λys. xs C ys`), `junk`} and keeps the generators whose proposals measurably earn acquisition (`Gain(G_i|O,D)`), dropping the rest — so `(O_t, G_t)` evolves jointly. In the string domain: `iterate` → replicate and `reduce` → concat both earn frontier gains (✗→3); junk is dropped (solves no target). The payoff is the transfer-negative made concrete — `iterate` alone stalls after replicate, but the acquired `reduce` supplies `concat = reduce(cons) = λxs.λys. xs cons ys`, which is **not** in `Closure_compose({cons})`; and with concat as substrate `iterate(concat, nil) = concat_n = (concat xs)^n nil` (`xs` concatenated n times) is the depth-2 list concept that iterate-from-{cons,replicate} alone cannot reach (✗→11). **C7 shows that acquiring an additional proposal schema restores structural leverage where the previously retained schema alone stalls:** `reduce` generates `concat`, which lies outside `Closure_compose({cons})`; that new concept becomes a substrate on which the already-retained `iterate` schema can generate the depth-2 concept `concat_n`. `meta --ablate` pins this as **measured cross-schema synergy** (identical budgets): `Reach({iterate})` = replicate only, `Reach({reduce})` = concat only, `Reach({iterate, reduce})` = all three, and junk is inert — so `concat_n ∈ Reach({iterate, reduce})` while `∉ Reach({iterate}) ∪ Reach({reduce})`. One cognitive operator (reduce) produces the substrate that makes another (iterate) productive again. This is the Level-2→Level-3 transition: program search → concept acquisition → **acquisition of the ways of generating concepts**. The remaining human-given object is the meta-space M itself — C8's target.

**C8 — discover the proposal schemas from a lower-level meta-language.** `disc` makes M itself a search space: λ-templates over two binders with a concept hole `C` (≤1 seed hole `S`), no inner abstraction. The machine enumerates the ≤4-leaf skeleton space — 455 deduplicated templates; `iterate` and `reduce` are just two programs in it — instantiates each over the ontology, and retains only the behaviorally-useful ones by the same counterfactual gate (6 of 455; 449 inert). The retained set reproduces C7's bootstrap: the joint rollout reaches replicate, concat, and concat_n, growing concat_n itself as a concept (`composition-{replicate, concat} ✗ → 11`). The new content is **meta-level credit assignment**: an operator's measured value is horizon-dependent. In the operator-pair control, at H=1 `iterate` unlocks replicate and `reduce` unlocks concat (tied, one frontier each); at H=2 both are credited with concat_n — a depth-2 concept produced jointly through BOTH operators across two generations, invisible to H=1 scoring. Honest control: the enumerated space also contains direct one-step concat_n templates, so the full retained set reaches concat_n at H=1 and no single schema is load-bearing there — the horizon-dependence is a property of the operator pair, not of every retained template.

The walls are honest too:

- Naive seed injection *widens* search (one run: median cost 0.016s → 0.265s). A size-1 atom seed branches against everything.
- The 9-fold product is a wall no product sub-concept breaks. `ablation` proves it's the composition search, not the value representation — compact semantic keys at full eval budget move nothing. Same columns, identical numbers.
- `diag` measures the composition wall exactly, and the verdict is sharper: the pool is already behaviorally deduped and contains **zero dominated** candidates, so semantic pruning is a no-op here (folds 2–11 give byte-identical baseline vs pruned). fold9 fails at cap 64 because it needs a single distinct intermediate at admission #133, and cap 64 saturates at 104 distinct semantics before it — a genuine width/ordering wall, not redundant representatives.

The real lever is more raw solves → bigger recurring idioms. That's the wall — a scale problem, not a mechanism failure.

## Run it

```sh
cargo build --release
./target/release/supsearch mkbench solutions/round0 bench
./target/release/supsearch bootstrap bench --train ... --holdout ... --rounds 3 --budget 20
./target/release/supsearch ladder    # raw discovers mul/square/power; only mul+power earn acquisition (counterfactual Δ>0), square and the mined idiom are rejected
./target/release/supsearch ontogen   # the same candidates under several ontologies — Gain(c|O) is relative: square ACQUIRED under ∅, rejected under {mul}; power the mirror image (needs mul as substrate)
./target/release/supsearch dep       # recorded negative: conditional DISCOVERABILITY does not hold in the arithmetic tower (conditional usefulness does) — see fine print
./target/release/supsearch gen       # C6: fixed iterate-schema generator G(O) — mul∈G(∅), pow∉G(∅)∧pow∈G({mul}), tet∉G({mul})∧tet∈G({mul,pow}); G fixed, only O changes
./target/release/supsearch transfer  # the SAME G into a non-arithmetic domain (strings): replicate=iterate(cons,nil)∈G(∅), genuine frontier ✗→3, junk rejected, NO 2nd-order concept (depth is value-space-bound)
./target/release/supsearch meta      # C7: acquire PROPOSAL SCHEMAS — iterate+reduce retained (frontier ✗→3 each), junk dropped; reduce's concat restores iterate's leverage to reach concat_n (depth-2, ✗→11), the step iterate-from-{cons,replicate} alone can't
./target/release/supsearch meta --ablate   # ablation matrix Reach(subset): {iterate}→{✓,✗,✗}, {reduce}→{✗,✓,✗}, {iterate,reduce}→{✓,✓,✓} — concat_n ∉ Reach({iterate})∪Reach({reduce}): measured cross-schema synergy
./target/release/supsearch disc      # C8: schemas are DISCOVERED from ≤4-leaf λ-templates (455 enumerated, 6 retained by the gain gate) — joint rollout reaches replicate/concat/concat_n; operator-pair H=1 vs H=2 control shows concat_n deferred (meta-level credit assignment); full-set H=1 is one-step (direct templates mask the pair effect)
./target/release/supsearch promote   # it picks mul itself, infers its arity, promotes it
./target/release/supsearch ablation  # why the fold-9 wall isn't the value representation
./target/release/supsearch diag      # provenance: winner ancestry, semantic redundancy, cap64 vs cap512
./target/release/supsearch prune     # decisive: semantic pruning @ cap64 — Outcome B (no-op on this family)
```

`cargo test`: 29 pass.

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
