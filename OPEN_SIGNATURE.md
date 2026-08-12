# U5: open-world recursive-signature ontogenesis

U5 removes U4's `complete=true` input. Observing a constructor shape never means
that no other constructor can appear later. Within the same anonymous polynomial
signature language, every bounded semantic class containing the visible shapes
remains live. A deterministic description-cost policy chooses a **provisional
incumbent**, not a logically identified datatype.

## Sequential experiment

The stream reveals three anonymous shapes at epochs 1, 2, and 3:

```text
t1  (parameters=0, recursive=0)
t2  (parameters=0, recursive=1)
t3  (parameters=0, recursive=2)
```

No event carries a completeness annotation. Exact signature enumeration through
syntax size 7 produces the following open-world states:

| epoch | compatible semantic classes | provisional incumbent | transition |
|---|---:|---|---|
| t1 | 44 | `[(0,0)]` | initial |
| t2 | 15 | `[(0,0),(0,1)]` | restructured |
| t3 | 2 | `[(0,0),(0,1),(0,2)]` | restructured |

The final two classes are deliberately retained. One is the incumbent and the
other admits additional unseen structure. U5 therefore reports
`logically_identified=false` at every epoch. Supplying completeness externally
would leave one exact-profile class, but that result is recorded only as a control
and never feeds the open-world learner.

## Preference and revision

Each compatible semantic class is scored by declared comparable terms:

```text
minimum syntax size * syntax_price
+ variant count       * variant_price
+ field count         * field_price
+ unsupported variants* unsupported_price
```

The policy is MDL-like, not a posterior probability. It says which adequate
bounded ontology is currently cheapest; it does not assign calibrated belief.
The chosen sequence is stable over a declared 3x3 calibration grid varying syntax
and unsupported-variant prices by fourfold.

Hysteresis retains a still-compatible incumbent when its score is within the
declared margin, preventing cost ties from causing thrashing. Hysteresis cannot
protect an incompatible ontology. When the delayed binary event arrives, the
unary incumbent is absent from the compatible frontier, a replacement is selected,
and all nullary, unary, and binary history is replayed against the replacement.
This is structural revision, not merely appending an atom to an unchanged profile.

An adversarial test starts from a deliberately wrong but initially compatible
`[(0,0),(0,2)]` incumbent. Unary evidence eliminates it and the same update rule
recovers `[(0,0),(0,1)]`.

## Executable connection to U4

U5 reuses U4's exact anonymous AST enumeration, semantic profile interpreter, and
generic `F(h)` action generation. At t2, the U5 incumbent is frozen and handed to
the unchanged U4 structure verifier. Independent lambda/typed search rediscovers
the carrier constructors, `F(M)->M`, and mediator generator; protected depths
5/7/9 commute and bounded mediator uniqueness succeeds.

This validation call uses an exact-profile U4 curriculum only after U5 has chosen
and frozen its incumbent. It cannot influence U5 ranking or revision. U5 does not
claim joint carrier/mediator synthesis for the final mixed unary/binary profile;
that remains beyond the small practical search boundary.

## Controls

- Syntax aliases are grouped before ranking; AST spelling cannot receive multiple
  votes.
- Evidence permutation within the same epoch is invariant. Epoch order is
  intentionally causal: future evidence is invisible at earlier stages.
- Target/output/trace/ancestry-derived, protected-group, and post-freeze events are
  removed before classing and scoring. Protected annotations are semantically inert.
- A semantic-class cap surfaces truncation and blocks preference claims.
- Repeating unchanged evidence retains the incumbent; delayed contradictory shape
  evidence forces restructuring and exact historical replay.
- Supplied-completeness and oracle schedules are explicit controls. Irrelevant and
  stale/misleading schedules spend more proposals than learned allocation after
  revision.
- Projecting learned points from the interleaved scheduler exactly reproduces the
  original universal dovetail.

## Economics

Proposal counts are within the signature-allocation domain. No conversion to
lambda observations, behavior-bank builds, or wall time is made.

| epoch | learned | uniform | oracle | supplied complete | irrelevant | stale/misleading |
|---|---:|---:|---:|---:|---:|---:|
| t1 | 1 | 1 | 1 | 1 | 44 | 44 |
| t2 | 1 | 5 | 1 | 1 | 15 | 16 |
| t3 | 1 | 2 | 1 | 1 | 2 | 3 |

The t1 net is negative (-109 units): no allocation saving exists yet. At t2 the
net is +39,948 and at t3 +9,973 over the declared 10,000-use horizon after charging
action evaluation, scoring, replay, and installation. This demonstrates economics
before and after revision without claiming that U5 beats an oracle or a human-
supplied complete inventory.

## Claim and limits

The supported claim is:

> Within a bounded anonymous polynomial-signature language, an open-ended stream
> of constructor-shape observations maintains multiple compatible semantic
> ontologies, selects a cost-minimal provisional incumbent, and structurally
> revises that incumbent when delayed evidence makes it incompatible, replaying
> prior evidence while preserving the universal fallback.

U5 still receives constructor shapes as observations. It does not infer shapes
from raw bytes, learn the scoring language, produce calibrated probabilities,
prove that no unseen constructor exists, or synthesize the final mixed-arity
carrier. Its preference is relative to syntax size 7 and the declared cost model.

## Reproduce

```sh
cargo test --release -p supsearch open_signature --lib
cargo run --release --example open_signature
cargo test --workspace
```

The example ends with deterministic `record,experiment=u5,...` output.

