# U2: bounded coproduct-property ontogenesis

U2 asks whether relational patterns among computations can cause the system to
invent a shared anonymous carrier with two embeddings and a reusable mediator
generator. The qualified result is:

> In the declared bounded observational category, independent pure-lambda
> enumeration discovered and acquired a coproduct-like cocone. Its frozen
> mediator commutes on unseen heterogeneous evidence, and every mediator in the
> declared exhaustive typed boundary belongs to one observational class.

This is not an unrestricted coproduct proof or a claim that the latent encoding
is unique.

## Frozen world and split

Objects are colored left-domain, right-domain, result, and anonymous-carrier
interfaces. Morphisms are closed untyped lambda terms; identity is `λx.x`,
composition is lambda application/composition, and equality is full beta-normal
form equality on frozen probes with 100,000 evaluator fuel.

Training mixes numeral-to-Boolean and Boolean/numeral-to-numeral arrows.
Calibration uses unrelated numeral-to-Church-list arrows. Protected replay
contains newly composed Boolean arrows and a different Boolean/Church-list
domain with numeral results. The carrier and all search/accounting state freeze
at epoch 1 before protected evaluation.

Evidence records carry duplicate groups and derivation metadata. Target-,
output-, trace-, ancestry-, protected-ID-, near-duplicate-, and post-freeze
records are removed before candidate enumeration. Mutating protected annotations
leaves the discovered programs, class, ranking, budget, ordering, and counters
unchanged.

## Discovery language and law

The only candidate grammar is the empty-alphabet closed lambda language:

```text
t ::= bound-variable | λt | t(t)
```

It contains no supplied sum/either/variant/tag/injection/branch/case/cocone/
copair production, atom, lookup table, or universal-law schema. U2 neither calls
nor imports the U1 module. The external verifier receives candidates, not a
candidate structure.

Pure search enumerates 21 closed terms with at least three leading lambdas
through size 6 as possible embeddings and 239 through size 8 as possible
generators. It does not know an expected encoding. Two embeddings must be
closed, distinct, injective on four independent payloads, mutually disjoint,
total within fuel, and reusable across every visible result representation.

For visible `f:A->X` and `g:B->X`, the proposed higher-order term produces
`h:S->X`. The verifier tests:

```text
h(embed_left(a))  ~= f(a)
h(embed_right(b)) ~= g(b)
```

The discovered programs are:

```text
embed_left  = λa.λb.λc.c(a)
embed_right = λa.λb.λc.b(a)
generator   = λf.λg.λs.s(g,f)
```

The reversed generator matches the independently discovered handler order; no
conventional spelling was seeded. The carrier is operationally the shared
representation jointly determined by the two embeddings. Heterogeneous result
encodings block the `S=X` shortcut, and the grammar has no IDs, raw records,
traces, or whole-input storage. Normalized evidence arrows have no common useful
closed subtree (minimum size 4), so syntax mining proposes nothing.

## Existence and bounded uniqueness

After discovery, a separate generic typed normal-form enumerator receives only
the inferred colored interface and the discovered generator. For each protected
arrow pair it enumerates every `S->X` inhabitant through size 8 with a 50,000
per-cell cap. No cell truncates. Each protected item has four syntactic valid
mediators but exactly one full-normal-form observational class.

The multiple spellings matter: uniqueness is semantic rather than syntax
equality. A cap-1 regression makes the exhaustive flag false and forbids a
uniqueness claim.

A non-epic control adds a third independently observable image. Two arrows agree
on both supplied embeddings but disagree on that third image, so existence holds
and uniqueness fails. Swapped embeddings, a missing branch, collapsed/collision
encodings, arbitrary matched terms, open terms, undersized candidates, divergence,
and malformed evidence are rejected. Nondeterminism and primitive-seeded lookup
programs are unrepresentable in the production grammar.

## Downstream reuse and costs

The protected downstream task is not construction or elimination. It maps both
payload roles with successor and returns a new anonymous carrier (`S->S`). Exact
results are:

```text
condition                  solved  proposals  generated  observations
learned U2                    yes         12        914            22
uniform acquired grammar      yes         39        914            50
external one-shot oracle      yes          1          1             8
raw typed abstract carrier     no          1          2             1
irrelevant ontology            no          1          2             1
pure lambda through size 8     no        707        707           707
```

Learned U2 has proposal regret 11 against the oracle and is 3.25x earlier than
uniform (the machine record conservatively emits integer ratio 3). It is at
least 58x earlier than the unsolved pure-lambda boundary by evaluated proposals.
Raw typed is reported honestly: at the abstract carrier interface it has only a
failing identity inhabitant, not a fabricated infinite cost.

Discovery, representation, equation validation, and installation charge 8,588
lambda-observation units. Relative to uniform allocation, U2 saves 28 checks per
reuse. At the declared 10,000-use horizon:

```text
uniform allocation  500,000 checks
learned allocation  220,000 checks
discovery charge       8,588 checks
net gain             271,412 checks
```

The identity `S->S` control costs one proposal/eight checks with or without U2,
so it supplies no acquisition evidence. A valid but useless structure fails the
charged gate. A beta-expanded equivalent embedding loses to the smaller witness.

Lambda observations, typed proposals, universal resource points, and behavior
executions are labeled work domains. Mixed aggregation is rejected; no conversion
factor is invented.

## Cost geometry and universal fallback

Sampled Boolean morphism distances are measured observational checks spent by
pure-lambda enumeration. The triples `(2,384,384)`, `(384,2,384)`, and
`(384,384,2)` satisfy `d(A,C) <= d(A,B) + d(B,C) + 1`. This supports only sampled
finite cost geometry, not formal enrichment.

Learned resource points alternate with the unchanged universal dovetail.
Projecting out learned points reproduces the original schedule exactly for 256
sampled points, and representable extreme syntax/fuel pairs retain finite
indices. Learned bias changes latency, not universal coverage.

## Acceptance-test map

- `discovers_independent_coproduct_like_structure_and_generalizes`: empty-language
  membership, heterogeneous representations, protected equations, exhaustive
  semantic uniqueness, and syntax baseline.
- `non_epic_carrier_has_existence_but_not_uniqueness`: load-bearing third-image
  counterexample.
- `malformed_swapped_collapsed_open_partial_and_divergent_controls_fail`: wrong,
  missing, collision-heavy, open, undersized, partial, and divergent controls.
- `leakage_controls_do_not_change_discovery_or_counters`: protected/output/trace/
  ancestry/duplicate/post-freeze exclusion and exact accounting invariance.
- `protected_mutation_and_universal_lane_are_invariant`: mutation firewall,
  exact fallback projection, and finite resource indices.
- `downstream_gain_negative_transfer_accounting_and_geometry`: compositional
  `S->S` reuse, raw/uniform/irrelevant/oracle/universal baselines, regret, charged
  acquisition, negative transfer, unlike-unit rejection, and sampled triangle.
- `truncation_equivalent_cost_and_random_carrier_controls_are_explicit`: cap
  disclosure, cheaper-equivalent preference, and arbitrary-carrier rejection.

## Reproduction and limits

```sh
cargo run --release --example coproduct_property
cargo test --release -p supsearch coproduct_property --lib
cargo test --workspace
```

The example emits one machine-readable `record,...` row.

Object colors, lambda substrate, probe construction, observational equality,
size/fuel/cell bounds, cost prices, reuse horizon, split, and scheduler remain
human-supplied. U2 does not infer raw data types, prove unrestricted uniqueness,
discover arbitrary categories, establish practical efficiency, report real-task
transfer, or demonstrate U3 initial algebras/catamorphisms.
