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

### B2-contextual — task-conditioned attention and bounded interaction

The global ledger is generalized to frozen contextual utility over canonical concept
sets. Observable task features determine which historical evidence transfers; age
decay handles shift, and width-2 residual credit separates synergy from redundant or
antagonistic pairs. Held-out task/duplicate IDs, protected outputs, target programs,
candidate ancestry, post-freeze evidence, mixed engine units, and oversized sets are
rejected with audited counters.

On two disjoint reversed-protocol recursive holdouts, contextual utility selects
`not` for a single-chain representation and recursive `parity` for a nested-chain
representation. It solves 2/2 in 670 proposals, matching the oracle with zero regret;
global utility solves 1/2, uniform costs 1,318, and shuffled context labels solve 0/2.
A separate two-concept interaction unlocks a held-out recursive law while either
singleton and the interaction-disabled policy fail.

Shared accounting retains separate `UniversalLambda` and `BehaviorBank` variants and
rejects mixed aggregation. Labeled property tests prove that arbitrary learned
schedules project to the exact original universal dovetail. On a preregistered real
ARC-1 split, contextual `{mirror,vflip}` solves and independently test-verifies the
final rotation in 5 bounded-bank constructions versus 11 for uniform and 5 for the
oracle; global, shuffled, interaction-disabled, irrelevant, misleading, and raw-bank
controls fail. This is a one-task existence result, not an ARC population estimate,
and no ARC `built` count is called universal. Full protocol and limits are in
`CONTEXTUAL_ALLOCATION.md`.

### B2-context-learned — task representation earns its place by regret

The hand-authored context key is no longer the primary condition. A finite meta-search
enumerates safe projections of raw measurements, freezes each candidate, and retains
the smallest representation minimizing downstream allocation regret on disjoint
calibration groups. Protected outputs, identity, solution provenance, ancestry,
post-freeze fields, duplicate groups, and injected target-derived evidence are
excluded or rejected.

On the recursive holdouts, learned `z` chooses the oracle's `not` and `parity`: 2/2
solved in 670 proposals with zero regret, versus uniform 1,318, global 1/2, and
collapsed-encoder regret 1,237. Interaction, decay/shift adaptation, and unchanged
universal interleaving remain load-bearing and tested.

On the frozen ARC slice, an encoder selected only from six disjoint generated grid
tasks chooses an unnamed numeric relation coordinate before seeing real ARC. After rotation calibration
evidence is admitted, it selects `{mirror,vflip}` for protected `6150a2bd`, matching
oracle and the old hand-feature ablation at rank 5; uniform takes rank 11 and negative
controls fail. Encoder selection (16 candidates), evidence acquisition (22 bank
constructions), and final search are separately reported. This remains a one-task
existence result. Full protocol and boundaries are in `LEARNED_CONTEXT.md`.

### B2-feature-invented — executable situational predicates

The engineered raw-field vocabulary is removed from the primary condition. Lambda
terms and grids share a lower-level numeric ordered-tree substrate, and a deterministic
exact-size grammar synthesizes total, fuel-bounded feature programs. Nested calibration
selects programs by downstream allocation regret plus syntax/execution cost.

On controlled raw trees, the system invents `Mod2(InputNodeCount)` with zero regret;
all primitive projections and collapse have regret 180, and a size-1 ablation fails.
On nine disjoint generated grid tasks it invents `Relation(ReverseChildren)` and
`Relation(MapChildren(ReverseChildren))` without a precomputed relation code.

Frozen transfer solves four protected real ARC tasks at aggregate bank rank 12,
matching the oracle and old engineered-projection ablation, versus global 13 and
uniform 27. Disabling interaction solves 2/4. Program enumeration, feature execution,
bank evidence (43 constructions), and universal work are separately reported. This is
bounded multi-task evidence, not an ARC population claim. Full protocol and limits are
in `EXECUTABLE_FEATURES.md`.

### U1 — universal-property ontogenesis

The system now searches the empty-alphabet pure lambda language for an anonymous
carrier and two observers because heterogeneous computations repeatedly factor
through them. An operation-blind typed search then discovers a reusable
`(X->A)->(X->B)->X->P` generator. No pair/product/tuple/projection/mediator atom
or schema occurs in the discovery substrate.

Frozen Boolean, numeral, and Church-list cones commute. Exhaustive typed
mediator enumeration through size 10 (50,000 terms per cell, with no truncation)
finds one valid full-normal-form class on each protected cone. A three-field
hidden-tag control satisfies both observer equations but fails uniqueness.
Syntax mining finds no shared normalized subtree.

On downstream swap, learned U1 matches the oracle at 1 proposal versus raw typed
10, irrelevant 31, and uniform 32; pure universal lambda is unsolved through
size 10 after 10,180 proposals. Discovery costs 5,286 comparable checks, breaks
even at 378 uses, and yields net gain 8,714 over 1,000 uses. A `mapBoth` negative
transfer control is worse (442 versus 221), preserving the contextual value
boundary. Full protocol and limitations are in `UNIVERSAL_PROPERTY.md`.

### U2 — coproduct-property ontogenesis

An independent empty-alphabet lambda search, with no U1 dependency or sum-like
production, discovers two size-6 embeddings and a size-8 mediator generator.
Heterogeneous Boolean, numeral, and Church-list result families satisfy both
embedding equations on protected probes. Exhaustive typed mediator enumeration
through size 8 is untruncated and yields one semantic class; a carrier with a
third observable image preserves existence but falsifies uniqueness.

Frozen U2 is reused on a branchwise carrier-to-carrier transformation: learned
allocation solves in 12 proposals/22 checks versus uniform 39/50 and oracle 1/8.
Raw typed and irrelevant conditions fail; pure universal remains unsolved after
707 proposals. Discovery costs 8,588 comparable checks and yields net +271,412
at 10,000 uses. Identity remains zero-benefit evidence. Full protocol and
limitations are in `COPRODUCT_PROPERTY.md`.

### U3 — initial-algebra/catamorphism ontogenesis

For the supplied finite action `F(X)=1+X`, independent lambda and generic typed
search discovers an anonymous recursive carrier witness (size 3), carrier step
(10), expanded constructor (17), and mediator generator (8). It imports no U1,
U2, recurrence, recursive-search, representation, or fixed-point result.
Boolean, numeral, and Church-list algebras share the frozen structure; protected
odd-parity and double-count equations hold at depths 5, 7, and 9. Exhaustive
mediator enumeration through size 8 is untruncated and yields one semantic class.
A hidden disconnected chain preserves existence but falsifies uniqueness.

On downstream carrier-to-carrier doubling, learned allocation takes 12
proposals/29 checks versus uniform 15/32 and oracle 1/10; raw typed and irrelevant
conditions fail, while pure universal remains unsolved after 707. Discovery costs
41,628 comparable checks and nets +258,372 at 100,000 uses. Identity remains a
negative transfer. Exact protocol: `INITIAL_ALGEBRA.md`.

### U4 — recursive-signature ontogenesis

U4 no longer receives `F(X)=1+X`. Exact size enumeration considers 237 anonymous
polynomial syntax candidates through size 5 and uniformly derives their semantic
variant profiles and executable actions. Weak evidence leaves 13 signature classes
and reports ambiguity. Rich Boolean, numeral, and Church-list experience leaves one
bounded semantic class with 12 syntax aliases, rather than claiming a unique syntax.

Independent lambda/typed search discovers its two Church constructors, expanded
`F(M)->M`, and generic mediator generator. Protected depths 5/7/9 commute and
untruncated mediator enumeration yields one semantic class. Truncation, leakage,
wrong-arity/nonrecursive, aliasing, weak-identifiability, binary-signature, and exact
universal-fallback controls are asserted. Downstream learned/uniform costs are 12/15;
the 928-unit discovery charge nets +29,072 at 10,000 uses. Supplying F also takes 12
downstream proposals, isolating signature invention as an upfront cost. Exact protocol:
`RECURSIVE_SIGNATURE.md`.

### U5 — open-world recursive-signature revision

U5 removes the complete-variant bit. Exact bounded enumeration retains every
semantic signature class compatible with the observations and ranks them by a
declared MDL-like description cost. A nullary -> unary -> binary stream leaves
44 -> 15 -> 2 live classes while the provisional incumbent restructures from
`[(0,0)]` to `[(0,0),(0,1)]` to `[(0,0),(0,1),(0,2)]`. It never claims logical
identifiability, and replays all earlier evidence after both revisions.

Hysteresis prevents compatible-score thrashing but cannot preserve a falsified
incumbent. A deliberately wrong early preference recovers; aliases, calibration,
leakage, post-freeze, truncation, temporal-order, supplied-completeness, irrelevant,
misleading, and exact universal-fallback controls are asserted. The frozen unary
incumbent passes unchanged U4 executable carrier/constructor/mediator discovery and
protected uniqueness. Signature-allocation costs change from learned/uniform 1/1
before useful revision to 1/5 and 1/2 afterward. Protocol: `OPEN_SIGNATURE.md`.


### U6 — non-monotonic ontology repair

U6 extends U5's growth-only revision to the full non-monotonic repair set:
retain, add, remove/invalidate, split, merge, specialize, generalize, and
structural replacement, over a behavior-bank substrate (concepts are finite
predicates on probe tokens 0..15 whose meaning is an executable Church-witness
term). Transitions between consecutive ontologies are classified by extension
overlap; a comparable cost ledger covers description, reasoning, predictive
error, migration, and revision penalty; unaffected concepts are counted as
preserved and historical commitments are replayed (current-task and
accumulated, reported honestly when a dropped task distinction makes the
latter impossible). A merge is only admitted when predictive error stays
equal, and structural replacement fires when patch clauses cost more than a
fresh model. Witness acceptance is verified to be exact on every probe token.

Controls: deterministic construction, causal epoch ordering, exact fallback,
frozen evaluation, no leakage, adversarial wrong-incumbent recovery, and
explicit `machine_record` output with `deterministic=true`. Protocol:
`ONTOLOGY_REPAIR.md`.

### U7 — knowledge migration across ontology changes

U7 asks what happens to existing knowledge when an ontology is revised. Every
old concept is classified as preserved, refined, re-expressible, ambiguous, or
invalidated against the new ontology, and two strategies are compared on one
comparable ledger: revision + migration (carry preserved/refined knowledge
forward, apply a small re-expression map, re-derive only ambiguous/invalidated
concepts) versus cold restart (discard everything, rediscover the new
ontology). Both end at the identical new ontology, so they share descriptive
and reasoning cost and differ only in transfer work; migration saves exactly
when the world changed less than the full ontology. Replay of the old task and
held-out coverage are verified. The example demonstrates all five kinds;
invalidated is exercised with a manually constructed ontology pair because the
observation runner never drops accumulated tokens. Protocol: `MIGRATION.md`.



### Direction D — invent observational probes

A major source of human ontology is the probe set: humans choose which
observations make structural distinctions visible. The agent holds a bounded
set of executable candidate hypotheses (predicates over 4-bit inputs, accepted
extensions with executable Church-witness meaning) that are observationally
equivalent on every measured probe yet differ on some unmeasured input. It
must invent the probe — which input to measure — scoring probes by expected
hypothesis reduction minus execution cost, and never seeing the hidden answer
(§7.3). With world truth "bit 1 set" and one measured probe, 29 initially
equivalent candidates are narrowed to the truth by five invented probes
(5 cost, 25 information); when the survivors agree on every unmeasured input
the agent correctly stops rather than fabricating a distinction. Probe
selection depends only on candidate disagreement. Protocol: `PROBE_INVENTION.md`.



### Direction E — active experimentation and the crucial experiment

Passive observation alone can leave environments indistinguishable. An active
learner chooses an *intervention* (sets an input value), observes the result,
and prunes its candidate world-models. In the crucial-experiment setup — two
environments observationally identical under all passive data but differing
under one intervention — a 2-bit-input machine with a hidden boolean function
leaves 8 candidate hypotheses after passive data; the active learner's first
intervention (`x=1`) is the decisive one, and two confirmatory checks recover
the world truth (3 actions, cost 3, information 7) where the passive learner
provably cannot (`passive_distinguished=false`). Intervention choice depends
only on candidate predicted outputs, never on the hidden answer. Protocol:
`ACTIVE_EXPERIMENTATION.md`.


### Direction F — causal ontology from interventions

Causal structure is not observable: passive data leaves a Markov-equivalence
class of causal models indistinguishable. In a tiny deterministic 3-variable
system (`A, B, C`, bounded boolean functions, 94 acyclic models), the agent
distinguishes *correlation*, *mechanism*, and *intervention response* by
choosing an intervention (`do(x=v)`), observing the downstream response, and
pruning candidates that predict a different response. With world truth the
chain `A → B → C`, passive data leaves 34 candidates
(`passive_distinguished=false`); the answer-blind choice `A=false` halves the
pool to 7, and two further interventions pin it to exactly the chain
(`final_candidates=1, true_recovered=true`). Selection uses only candidate
disagreement; when survivors agree on every available intervention the agent
stops rather than fabricating a distinction. Protocol: `CAUSAL_ONTOLOGY.md`.

### Direction G — world-model ontogenesis

A persistent deterministic world of independent reversible counters
("switches"), each toggled by exactly one action, is never described to the
agent. From a bounded set of observed `(state, action) -> next_state`
transitions it must invent a compressed representation that reduces future
reasoning cost. It discovers the factored transition model
(`parents=[[0],[1],[2]]`), predicts all 16 held-out full-state transitions
(1.000) where the raw monolithic table predicts none (0.000), invents a
"reversible counter" concept that predicts switch behavior exactly (32/32) and
transfers to a new switch at probe cost 3 vs. cold-start 8 (saved 5), and plans
to set all 6 switches in 6 factored expansions vs. 58 raw BFS expansions. A
coupled control world honestly reports partial factorization
(`parents=[[0],[0,1],[0,1]]`) with held-out factored accuracy 0.000, so the
discovery does not over-claim compression. Protocol: `WORLD_MODEL.md`.

### Direction M1 — invent distance as a reusable mathematical concept

The first problem of the Mathematical Ontogenesis track. Mathematics is
treated as a world `W = (S, A, T, O)`. From four Pythagorean-triple
observations `(x,y) -> d`, the agent invents `sqrt(x*x + y*y)` (the Euclidean
distance, size 8, discovery_cost 99,573) which generalizes to all four held-out
points. The concept is reusable: predicting a new point's distance costs 1
evaluation (`concept_reasoning_cost=4`) versus re-synthesizing from scratch
(`baseline_reasoning_cost=99,577`), a saving of 99,573 expressions. It also
compresses the observations (24 raw tokens -> 8-node expression, gain 16). A
non-Pythagorean control honestly reports no generalizing fit. Protocol:
`MATH_WORLD.md`.

### Direction M2 — invent the circle invariant

The second problem of the Mathematical Ontogenesis track. Given member points
on a hidden circle and non-member points, the agent invents the invariant
`x² + y² = 25` (the circle of radius 5, size 7, discovery_cost 57,883) which
generalizes to all held-out members and non-members. The invariant is
reusable: classifying a held-out point costs 1 evaluation
(`concept_reasoning_cost=8`) versus re-discovering the invariant from scratch
(`baseline_reasoning_cost=57,891`), a saving of 57,883 expressions. It also
compresses the class (16 raw tokens -> 8 tokens, gain 8). A non-circular
control honestly reports no generalizing invariant. Protocol: `MATH_WORLD.md`.


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

Contextual allocation and controlled ARC transfer:

```sh
cargo run --release --example contextual_allocation
cargo run --release -p arc1 -- contextual
cargo test -p supsearch contextual_allocation --lib
cargo test -p supsearch contextual_guidance --lib
cargo test -p arc1 contextual_arc_transfer_is_frozen_verified_and_deterministic
```

Learned context representation:

```sh
cargo test -p supsearch learned_context --lib
cargo run --release --example contextual_allocation
cargo run --release -p arc1 -- contextual
```

Executable feature invention:

```sh
cargo run --release --example feature_invention
cargo run --release -p arc1 -- features
cargo test -p supsearch feature_invention --lib
cargo test -p arc1 invented_features_generalize_to_multiple_frozen_arc_holdouts
```

Universal-property ontogenesis:

```sh
cargo run --release --example universal_property
cargo test --release -p supsearch universal_property --lib
```

Learned-allocation experiment:

```sh
cargo run --release --example learned_allocation
cargo test -p supsearch learned_allocation --lib
cargo test -p supsearch ontology_guidance --lib
```

Causal ontology (Direction F):

```sh
cargo run --release --example causal_ontology
cargo test -p supsearch causal_ontology --lib
```

World-model ontogenesis (Direction G):

```sh
cargo run --release --example world_model
cargo test -p supsearch world_model --lib
```

Mathematical ontogenesis (Direction M1):

```sh
cargo run --release --example math_world
cargo test -p supsearch math_world --lib
```
