# Developmental invention of executable context features

The context basis is now executable and synthesized:

```text
lower-level raw tree
  -> fair finite feature-program enumeration
  -> frozen regret selection
  -> invented z
  -> U(concept set | z,O,H)
  -> protected search allocation
```

This moves beyond `LEARNED_CONTEXT.md`, where the learner selected projections from
an engineered numeric field vocabulary. Here no useful context measurement is present
as a raw field.

## Raw substrate

`src/feature_invention.rs` exposes one domain-neutral ordered tree. Lambda terms are
lowered to numeric constructor trees; grids are lowered to a root containing row
nodes containing cell leaves. Feature programs cannot inspect task/family IDs,
duplicate metadata, ontology concepts, solution traces, target ancestry, search
outcomes, or protected test outputs. A task may contain published training outputs;
protected outputs are never placed in the substrate.

## Feature language

The exact-size grammar contains generic scalar observations, constants, unary and
binary arithmetic/predicates, and generic ordered-tree transformations:

```text
Primitive p
Constant n
Unary(op, phi)
Binary(op, phi1, phi2)
Relation(transform)

transform := Identity
           | ReverseChildren
           | MapChildren(transform)
           | Compose(transform, transform)
```

`Relation(t)` executes `t` on every published first input and returns whether it equals
the corresponding published output. It has no grid-, lambda-, mirror-, rotation-, or
concept-named production. Increasing syntax size and candidate cap widens the exact
grammar deterministically. Every execution has explicit fuel and step accounting.

Malformed terms are unrepresentable by the typed AST. Divergent, partial,
nondeterministic/noise, shuffled, unstable, identity-metadata, and protected-output
controls are rejected and never emitted by the production grammar.

## Regret selection

Programs are learned on nested training/calibration groups. Each candidate feature set
is materialized, passed to the frozen contextual utility learner, and scored by
downstream allocation regret. The charged objective is:

```text
regret * 1,000,000 + syntax complexity cost + execution-step cost
```

This makes regret decisive while preferring smaller/cheaper features among equal
allocators. Reconstruction quality and target accuracy are not objectives. Unsafe,
target/output-derived, ancestral, and post-freeze evidence is removed before program
enumeration, scoring, and accounting. Duplicate groups remain excluded by the
contextual freeze.

## Controlled compositional result

Raw training trees have node counts 2/4 versus 3/5; calibration uses unseen counts
6/7 and protected replay uses 100/101 with unrelated labels. No primitive raw
projection transfers because exact counts and surface labels change. The system
enumerates 30 programs and invents:

```text
Unary(Mod2, Primitive(InputNodeCount))
```

This two-operation feature is absent as a primitive. Results:

```text
invented feature regret       0
best primitive regret       180
collapsed regret            180
heldout count 100 top         A
heldout count 101 top         B
feature sets evaluated       31
task executions             360
execution steps            1692
```

Independent replay returns byte-identical `z`; structurally different inputs with the
same parity merge. A size-1/primitive-only ablation fails. A richer program with the
same behavior loses after complexity cost.

## Multi-holdout ARC result

Feature and utility learning uses nine disjoint generated tasks: two training tasks
per relation and one calibration task per relation, with varied sizes, colors, and
duplicate groups. No real ARC task participates in feature or utility learning.

From raw nested trees, the system enumerates 297 programs, evaluates 418 feature sets,
and retains:

```text
Relation(ReverseChildren)
Relation(MapChildren(ReverseChildren))
```

Both are compositional. Together they distinguish vertical reversal, per-row reversal,
and their interaction without a precomputed transformation code.

Frozen evaluation covers four real ARC tasks whose test outputs are verification-only:
mirror `67a3c6ac`, vertical flip `68b16354`, and rotations `3c9b0459` and `6150a2bd`.

```text
condition                 solved    aggregate built/rank
invented features           4/4              12
engineered raw projection   4/4              12
oracle                      4/4              12
global utility              4/4              13
uniform allocation          4/4              27
interaction disabled        2/4               8
```

Invented-feature regret is 0; best primitive projection and collapse each have regret
2. The engineered-projection ablation also reaches 12 because it receives the old
precomputed relation bit; it is reported to show exactly what was removed from the new
substrate. Feature evidence costs 43 separately reported `BehaviorBank` constructions.
Feature enumeration (297), feature-set evaluation (418), executions (11,277), steps
(352,349), bank constructions, and universal-lambda work remain incomparable units.

This is bounded evidence over four selected ARC tasks, not a population score.

## Falsification suite

Tests cover compositional invention, primitive/depth-1 failure, separation and surface
merging, independent replay, regret/complexity selection, interaction dependence,
decay and old-context replay, injected target/output/ancestry/post-freeze evidence,
duplicate groups, protected-output mutation, divergent/partial/nondeterministic/
metadata controls, constant/collision controls, deterministic exact enumeration,
exact accounting, mixed-unit rejection, and multi-holdout ARC verification.

Arbitrary feature schedules remain only the learned lane. Projecting them away yields
the original universal dovetail exactly, so feature invention changes latency, not
computability.

## Reproduction

```sh
cargo run --release --example feature_invention
cargo run --release -p arc1 -- features
cargo test -p supsearch feature_invention --lib
cargo test -p arc1 invented_features_generalize_to_multiple_frozen_arc_holdouts
cargo test --workspace
```

Both executables emit `record,...` machine-readable rows.

## Remaining boundary

The raw tree lowering, feature grammar, constants/operators, syntax/fuel/candidate
bounds, regret multiplier, complexity/execution prices, utility model, interaction
width, curriculum, and ARC holdouts remain supplied. The system synthesizes programs
inside that language; it does not invent the language itself, infer unrestricted
features from bytes, establish a unique ontology, or demonstrate practical/statistical
ARC superiority. Universal search keeps its existing combinatorial, machine, stack,
memory, and undecidability limits.
