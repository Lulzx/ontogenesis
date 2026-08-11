# Ontogenesis B1–B3 milestone contract

This file freezes what each milestone proves, the evidence required to reproduce it,
and what remains outside the claim.

## B1 — nonrecursive abstraction invention

Question: can generic syntax machinery turn reusable structure inside one raw-discovered
program into a primitive that improves future reasoning?

Protocol:

1. Start with the computational substrate `{cons,nil}` and an empty learned ontology.
2. Raw-solve duplication of singleton rows.
3. Enumerate repeated subterms and factor each into a closed context abstraction.
4. Require the original and rewritten programs to solve a disjoint semantic suite at
   widths 2 and 3.
5. Measure acquisition only on held-out widths 4 and 7.

Evidence: `arc1 b1` reports one valid abstraction and a frontier gain `✗ → 2`.
The regression requires the raw winner to use the actual `cons` primitive, rejecting
the earlier degenerate-lambda failure mode.

## B2 — executable recurrence induction

Question: can the system infer an executable recursive law from finite programs it
discovered itself?

Protocol:

1. Independently raw-solve closed instances at depths 1, 2, and 3. Each instance
   supplies fresh endofunction atoms and a tail as its data; these are symbolized after
   discovery and are not retained ontology concepts.
2. Replace only the instance atoms with stable symbolic inputs and normalize.
3. Infer an arbitrary two-hole syntax context `C` satisfying
   `q[n+1] = C(head[n+1], shift(q[n]))` for every adjacent pair.
4. Recover the base by matching the same context against `q1`.
5. Require exact reconstruction of all observed unrollings.
6. Compile the equation for Church-encoded input lists.
7. Test depths 5, 7, and 9 using new function atoms and different tail widths.
8. Run the counterfactual acquisition gate on a separate future task.

Observed evidence:

```text
q1 = #0(#z)
q2 = #0(#1(#z))
q3 = #0(#1(#2(#z)))
law validation                         ✓
semantic extrapolation at 5,7,9       ✓
future reasoning                      ✗ → 7  ACQUIRE
```

The inducer rejects laws that are constant, ignore the head, ignore the recursive
result, change context by depth, use a lookup-like exceptional unrolling, or merely
insert a beta-redex that happens to be observationally equivalent.

This finite-unrolling route remains the efficient B2 path for structural laws. B2 now
also has a universal path for laws outside that fragment.

### B2-general — arbitrary recursion invention

Question: can the system propose and execute recursive laws without assuming a fold,
a particular data encoding, a single recursive function, or structural descent?

Mechanism and evidence:

1. `universal::terms_exact` enumerates the full well-scoped untyped de Bruijn lambda
   grammar, with an optional finite ontology alphabet, in finite exact-size classes.
   `Dovetail` schedules every positive `(syntax size, evaluation fuel)` pair. There is
   no candidate, size, depth, or fuel cap in the proposal stream.
2. `recursion_search` treats each generated closed term as an unknown functional `F`,
   constructs `fix F` using only lambda and application, and independently checks
   discovery behavior, the extensional equation `fix F = F(fix F)`, and unseen
   extrapolation behavior. Recursive-parameter and neutral-ablation controls
   reject both nonrecursive shortcuts and syntactically present but dead recursion.
   A complete finite size class actually rediscovers `λr.λvalue.value r` from
   depth-0..3 anonymous chain behaviors and extrapolates at depths 5, 7, and 9;
   this is not only a membership argument for a hand-supplied target.
3. `fixpoint::synthesize` constructs the fixed point directly; `Y`, `fold`, and a
   runtime recursion primitive are not ontology atoms. The same construction executes
   a nested Ackermann recurrence, demonstrating non-structural recursion.
4. `fixpoint::synthesize_mutual` takes a functional over an anonymous Church tuple and
   ties every component simultaneously. Independently projected component equations
   and even/odd behavior through depth 9 validate mutual recursion.
5. `recurrence::infer_semantic` discovers one invariant law even when an eta-expanded
   previous computation is not an exact normalized subtree. Discovery probes and
   independent holdout probes are checked separately.
6. `representation::invent` receives only an anonymous vector of constructor field
   counts and constructs new closed constructors plus their eliminator. It is validated
   against independent handler/field probes, then a synthesized recursive program
   traverses an invented binary-tree representation at unseen depths.
7. The invented Ackermann executable produces a measured counterfactual frontier gain:
   it is unreachable in the bounded future reasoner without the concept and reachable
   after installation as a two-argument primitive.

Controls reject open functionals, zero-component mutual recursion, nonrecursive
identity shortcuts, dead recursive references, constant-output families when that
control is enabled, incomplete semantic probe assignments, empty signatures, wrong
constructor arities, wrong branches/fields under representation-law probes, and
divergent candidates through finite fuel plus a process-safe evaluator stack guard.

The proposal-completeness claim is precise and limited:

> Every representable finite closed functional in the declared universal lambda
> language is eventually proposed, and every terminating observation within
> representable finite fuel is eventually tested.

Concretely, every such functional over the declared finite atom alphabet occurs at a
finite syntax size; if its required observations terminate within representable finite
fuel, the diagonal schedule tests it with enough fuel after finitely many stages. There
is no configured experimental cap; the implementation resource model uses `u32`
syntax sizes, `i64` evaluator fuel, available memory, and the evaluator's explicit
stack guard. This is relative semidecision completeness for that universal language
and observer. It is not decidable program equivalence, finite-time proof that no
solution exists, literal infinite hardware, or a claim that practical enumeration
avoids combinatorial growth.

The invented representation result covers representable finite sum-of-products
signatures whose constructor arities are supplied anonymously. It does not yet infer
the signature itself from raw bytes or prove a unique latent encoding.

### B2-guided — learned ontology as universal-search bias

Question: can acquired concepts reduce the cost of discovering the next recursive law
by orders of magnitude without replacing the universal fallback?

`ontology_guidance` runs a two-step developmental sequence. An independently validated
Boolean-negation concept reduces recursive parity discovery by at least 122× proposals
(41,272 baseline proposals without a solution versus rank 337 with the concept). The
actually discovered parity executable is then acquired and reduces discovery of a
distinct nested recursive law by at least 230× (162,550 prior-ontology proposals
without a solution versus rank 705 after installation). Both results extrapolate on
independent holdouts and require load-bearing recursion.

Equally sized irrelevant and misleading ontologies fail at the guided bound, ruling
out mere alphabet enlargement. A recorded aggregate overfit motivated single-payload
discriminating holdouts and is locked in as a negative regression. The finite priority
prefix then returns to the unchanged universal dovetail, so these gains reshape
practical reachability without weakening B2-general's completeness floor. Full
protocol, measurements, limitations, and reproduction commands are in
`ONTOLOGY_GUIDANCE.md`.

### B2-learned — prior utility allocates future universal search

The hand-selected ontology prefix is now replaced by a deterministic utility ledger.
Paired training measurements record proposals, evaluated candidates, solution rank,
syntax/fuel frontiers, widening cost, age/decay, and provenance. Held-out and
target-derived evidence is excluded. Learned scores allocate proposal order, syntax
classes, and fuel; useful concepts receive more, stale concepts less, and harmful
matched controls less again.

On a held-out anonymous recursive protocol with reversed interpreter/payload order,
the learned policy discovers the law in 335 proposals, exactly matching the
hand-designed parity-first oracle. Uniform allocation takes 2,310 proposals. The
pure universal prefix remains unsolved after 41,272 proposals, giving a proposal-count
separation of at least 123x. Irrelevant and misleading one-atom controls remain
unsolved at their matched bound. A deliberately injected held-out utility record is
skipped and leaves the ranking unchanged.

Every finite learned point is alternated with a point from the unchanged universal
dovetail; after learned work ends, that dovetail continues forever. Thus learned
weights change practical allocation but not B2-general's exact qualified completeness
claim. The ablated learned-only schedule also solves this target but correctly loses
universal coverage. Protocol, exact counts, controls, ARC-interface assessment, and
limitations are in `LEARNED_ALLOCATION.md`.

## B3 — recursive law to reasoning vocabulary

Question: does reifying the B2 recursion scheme change which higher concepts are
generable and useful?

Protocol:

1. Promote the B2 structural recursion scheme to an ontology atom.
2. Run one generic simply-typed beta-normal enumerator. Its grammar has variables,
   lambda, application, and currently available typed atoms; it has no named
   operation productions.
3. Semantically gate candidates on multi-example map, append, and reverse tasks.
4. Compare the identical generator with and without the recursion atom.
5. Counterfactually acquire the resulting concepts.
6. Compose invented `map` and `reverse` and transfer to a held-out grid.

Observed evidence:

```text
                         {cons,nil}    + invented recursion
map (size 11)            absent        reachable
append (size 9)          absent        reachable
reverse (size 14)        absent        reachable

map acquisition                         ✗ → 15
append acquisition                      ✗ → 15
reverse acquisition                     ✗ → 3
map(reverse), unseen 5×4 grid mirror    ✓
```

Honest control: `append` is useful vocabulary but is not load-bearing for reverse in
this search space; recursion can inline an append-like helper. The supported claim is
`R → {map, append, reverse}`, not `R → append → reverse`.

All B3 absence claims are bounded-search claims: the enumerator uses the displayed
maximum term sizes and a 50,000-candidate cap per memoized type/context/size cell.

## Reproduction

```sh
cargo run -p arc1 -- b1
cargo run -p arc1 -- b2
cargo run -p arc1 -- b3
cargo test --workspace
```

Focused B2-general checks:

```sh
cargo test -p supsearch universal --lib
cargo test -p supsearch fixpoint --lib
cargo test -p supsearch recurrence --lib
cargo test -p supsearch representation --lib
cargo test -p supsearch recursion_search --lib
```

Ontology-guidance experiment:

```sh
cargo run --release --example ontology_guided
cargo test -p supsearch ontology_guidance --lib
```

Learned-allocation experiment:

```sh
cargo run --release --example learned_allocation
cargo test -p supsearch learned_allocation --lib
cargo test -p supsearch ontology_guidance --lib
```
