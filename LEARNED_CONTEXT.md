# Learned developmental context representations

This milestone replaces the primary hand-authored context key with a frozen,
regret-selected representation:

```text
raw pre-search observations -> learned z -> U(concept set | z,O,H) -> allocation
```

The claim is operational: within a declared finite projection meta-space, the system
retains observable distinctions because they lower calibration allocation regret. It
does not infer a unique latent ontology or learn unrestricted features from raw bytes.

## Mechanism and leakage boundary

`src/learned_context.rs` defines numeric raw fields with an explicit origin and epoch.
Only `PreSearchObservable` fields recorded at or before the freeze may cross the
encoder boundary. Protected outputs, held-out identity, solution-derived fields,
target ancestry, target-derived evidence, post-freeze evidence, and duplicate groups
are rejected or excluded.

The encoder search enumerates `Collapsed` and all safe projections up to a fixed
width. Every candidate is frozen and evaluated on disjoint calibration task groups.
Loss is downstream concept-allocation regret, not reconstruction error. Equal-regret
candidates prefer lower width, so extra surface detail without allocation gain is not
retained. Identity memorization is a control, never a production proposal.

After selection, raw field names are erased: the contextual ledger receives only
`z0`, `z1`, ... and their values. Existing interaction residuals, confidence, decay,
finite-budget competition, and leakage gates then operate unchanged. The encoder is
immutable during protected evaluation.

## Accounting

Three domains stay separate: encoder candidates/predictions/fields inspected;
`UniversalLambda` proposals/evaluations used for synthetic evidence; and ARC
`BehaviorBank` constructions. The API rejects mixed-domain aggregation. Evidence
acquisition is printed separately and is never added to held-out solution rank.

## Controlled recursive result

The raw interface measures generic syntax properties of input lambda terms and never
exposes expected values, representation names, useful concepts, or target laws. It
searches 22 encoders and selects `Projection([raw-0])`:

```text
encoder regret             0
collapsed regret        1237
encoder evidence work  10218  (UniversalLambda primary work; separate)

condition          solved   proposals   evaluated   universal
learned z             2/2         670         278       yes
hand features         2/2         670         278       yes
global                1/2         983         405       yes
uniform               2/2        1318         544       yes
oracle                2/2         670         278       yes
shuffled              0/2        1296         532       yes
universal only        0/2        5244        3408       yes
```

Learned `z` separates the single-chain and nested-chain contexts while merging
surface variants within each. It selects `not` and `parity` and has zero proposal
regret versus oracle. A separate width-2 test selects `{first,second}` with residual
utility 4326; neither singleton nor interaction-disabled search solves.

## Frozen ARC existence demonstration

The final task remains `6150a2bd`; its test output is verification-only. Encoder
selection uses six disjoint generated grid tasks with varied sizes and values, before
any real ARC task is used by the encoder. Raw ARC fields are numeric: an unnamed
bitset over four rectangular coordinate
involutions plus shape, pair-count, cell-count, and color-count measurements. No field
names a transformation or concept.

The search evaluates 16 encoders, chooses `Projection([raw-0])`, and gets zero regret
versus 4 for collapse. After freezing, published pairs from rotation `3c9b0459`
provide interaction-utility evidence. Protected evaluation then gives:

```text
condition                    built/rank   protected test
learned z {mirror,vflip}          5/5          pass
hand-feature ablation             5/5          pass
oracle                            5/5          pass
uniform                          11/11         pass
global mirror                     3/-          fail
shuffled                          3/-          fail
interaction disabled              3/-          fail
irrelevant identity               1/-          fail
misleading projection          2016/-          fail
raw bank through size 7         3405/-          fail
```

Encoder evidence costs 22 separately reported bank constructions. Every ARC row is
`universal=false`. This is a deterministic one-task existence result, not an ARC
population estimate.

## Falsification suite

Tests cover separation and surface merging; global/collapsed, irrelevant,
noise-like unstable, identity-oversplit, higher-complexity, shuffled-label,
hand-feature, and oracle controls; regret rather than reconstruction selection;
width-2 interaction; decay under shift with old-context replay; every declared field
origin; injected target-derived evidence; protected-output mutation after freeze;
exact replay accounting; unlike-unit rejection; and universal-lane projection.

For arbitrary finite learned schedules, filtering the interleaving to its universal
lane reproduces the original `Dovetail` exactly. Learned representation changes
latency, not computability.

```sh
cargo test -p supsearch learned_context --lib
cargo test -p supsearch contextual_guidance --lib
cargo test -p supsearch universal --lib
cargo test -p arc1 contextual_arc_transfer_is_frozen_verified_and_deterministic
cargo run --release --example contextual_allocation
cargo run --release -p arc1 -- contextual
cargo test --workspace
```

Both executables emit machine-readable `record,...` rows.

## Remaining scaffolding

`EXECUTABLE_FEATURES.md` advances the primary condition beyond fixed projection by
synthesizing executable predicates from a lower-level tree substrate. This section
remains the boundary of the earlier projection milestone.

The raw measurement vocabulary, projection grammar/width, similarity rule,
decay/confidence formulas, candidate concept sets, interaction width, curriculum, and
ARC split remain fixed. ARC's unnamed coordinate bitset is more primitive than the
old named transform label, but is still engineered. The system does not yet invent
arbitrary feature programs, jointly learn continuous embeddings and allocation,
infer representation signatures from bytes, or establish statistical ARC superiority.
Universal search retains its documented combinatorial, resource, and undecidability
limits.
