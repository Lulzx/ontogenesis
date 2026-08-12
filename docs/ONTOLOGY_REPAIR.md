# U6: genuinely non-monotonic ontology repair

U5 could only *grow* a provisional ontology (add / retain / restructure).
U6 extends revision to the full non-monotonic repair set — retain, add,
remove/invalidate, split, merge, specialize, generalize, and structural
replacement — while preserving unaffected concepts, replaying historical
evidence, and charging a single comparable total cost.

## Substrate

The experiment runs in a separate behavior-bank domain (it does not convert to
lambda observations). A concept is a finite predicate over a fixed probe-token
family `0..15`; its *meaning* is its extension (the accepted tokens), and that
meaning is executable: `witness(extension)` is a closed Church-bool λ-term that
evaluates to `true` exactly on the extension. `evaluate_witness` verifies
acceptance under a fixed evaluation fuel and rejects non-bool results.

The learner receives observations `(token, label)` from a hidden world plus a
downstream *task* (the queries it must answer cheaply). It forms the finest
partition consistent with the observed labels, then greedily merges classes
whenever a merge strictly lowers structural cost (an MDL-like policy, not a
calibrated posterior). Later evidence and task changes force concepts apart,
absorb them, or retire them.

## Repair operations

Between two consecutive ontologies each old concept is classified against the
new ones by extension overlap:

| op | meaning | non-monotonic |
|---|---|---|
| `retain` | extension and label unchanged | no |
| `add` | a new concept with no prior counterpart | no |
| `remove` / invalidate | no new concept covers the old tokens, or all covering concepts changed label (fragmented) | yes |
| `split` | old tokens disperse into several new concepts | yes |
| `merge` | two+ old concepts fold into one new concept | yes |
| `specialize` | a concept narrows (a sub-concept keeps the label) | yes |
| `generalize` | a concept widens without absorbing a distinct labeled sibling | yes |
| `replace` | the structural template is rebuilt wholesale (see below) | yes |

A merge is only admitted when predictive error stays equal, so the learner
cannot merge two task-distinguishable concepts to save description cost.

## Cost ledger

All components are in one comparable unit set:

```text
description      = HEADER*#concepts + TOKEN*#covered_tokens
reasoning        = LOOKUP*#task_queries          (unique covering concept lookup)
predictive_error = ERROR*#misclassified/uncovered queries
migration        = MIGRATION*#changed_concept_identities
revision_penalty = REVISION*#non_monotonic_ops
total            = structural + migration + revision_penalty
```

The universal fallback remains exact: an uncovered query is still answerable by
raw search, but it is charged an error cost and the full search is never
removed.

## Predictive replay

Replay is defined against the *accumulated* task — every query the system has
ever committed to. `replayed` reports the current task; `accumulated_replayed`
reports all historical commitments. In fixed-task trajectories both hold. When
the world deliberately drops a task requirement to justify a cost-driven
merge/generalize, only the current task is guaranteed, which is reported
honestly rather than pretending distinct observations vanished.

## Preservation

`affected_concepts` and `preserved_concepts` record which prior concept
identities were touched and which were carried forward unchanged. Structural
replacement is tracked explicitly via `structural_replaced`.

## Structural replacement

`structural_decision(old_desc, new_desc, exceptions, patch_per_clause,
rebuild_penalty)` chooses a fresh structural template when accumulated patch
clauses cost more than rebuilding:

```text
patch_cost  = old_model_desc + patch_per_clause * exceptions
rebuild_cost = new_model_desc + rebuild_penalty
replaced    = rebuild_cost < patch_cost
```

## Example trajectory

`examples/ontology_repair.rs` runs five stages: an initial coarse world, a
refinement (split), a coarsening with a dropped task distinction (merge), a
token that migrates to a wholly new class (specialize + add), and previously
unseen probes (generalize + add). The run prints each stage's transitions,
affected/preserved counts, replay flags, and the deterministic machine record.

## Controls

- All construction and classification is deterministic (no RNG); identical
  evidence yields identical records.
- Evidence order within an epoch is invariant; epoch order is causal — future
  evidence is invisible to earlier stages.
- The witness acceptance test verifies that each concept's executable term
  accepts exactly its extension, over every probe token, before it is trusted.
- Cost-driven merges cannot collapse task-distinguishable classes.
- Frozen evaluation and the exact fallback are preserved.
- Result classification is honest: a merge/release of a task distinction is
  recorded as `accumulated_replayed=false`, not hidden.

## Claim and limits

Supported claim:

> Within a bounded behavior-bank domain, an ontology learner revises its
> concept set under the full non-monotonic operation set, preserving unaffected
> concepts and replaying accumulated history where the task permits, while
> charging one comparable cost that covers description, reasoning, predictive
> error, migration, and revision.

Limits: the learner still receives `(token, label)` observations (no raw
percept or feature invention here), the operation set is fixed in advance
(the meta-language for repair is not yet learned), costs are the declared
comparable units rather than calibrated probabilities, and concept *meaning*
is restricted to a finite probe family.

## Reproduce

```sh
cargo test --release -p supsearch ontology_repair --lib
cargo run --release --example ontology_repair
cargo test --workspace
```

The example ends with `experiment=ontology_repair,deterministic=true,...`
machine output.
