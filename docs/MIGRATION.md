# U7: knowledge migration across ontology changes

U6 revises an ontology non-monotonically; U7 asks what happens to the *knowledge*
encoded in the old concept set. When the ontology changes, every old concept is
classified into one of five migration kinds and two strategies are compared on a
single comparable cost ledger.

## Substrate

Same behavior-bank domain as `ontology_repair`: concepts are finite predicates
over probe tokens `0..15` with an executable Church-witness meaning. Old and new
ontologies are built deterministically by the same greedy MDL `Runner`.

## Migration kinds

Each old concept is classified against the new ontology by extension overlap and
label agreement:

| kind | condition | transfer work |
|---|---|---|
| **preserved** | identical extension and label survive | `0` |
| **refined** | every token keeps the old label but the structural expression changed (specialize / generalize / split into same-labeled pieces) | `REFINE_PRICE * tokens` (only recompute the adjusted extent) |
| **re-expressible** | all tokens now share one *different* new label (a consistent symbol rename of this concept) | `RENAME_PRICE` (small map) |
| **ambiguous** | the tokens now map to ≥2 distinct new labels (the old class is split) | `AMBIGUOUS_PRICE * tokens` (full re-derivation) |
| **invalidated** | tokens are no longer observed, or the concept dissolved | `INVALIDATE_PRICE` |

## Revision + migration vs cold restart

Both strategies end at the *identical* new ontology, so they share the same
descriptive and reasoning cost:

```text
new_structural   = HEADER*#new_concepts + TOKEN*#covered_tokens
reasoning        = LOOKUP*#new_task_queries
migration_total  = new_structural + reasoning + migration_transfer
cold_restart_total = new_structural + reasoning + cold_restart_transfer
migration_transfer = Σ transfer_cost(old concept)
cold_restart_transfer = Σ RELEARN_PRICE * tokens  (rediscover every new concept)
saving = cold_restart_total - migration_total
```

Migration is cheaper exactly when the world changed less than the full ontology:
preserved concepts cost nothing, refined concepts cost only the delta, and
re-expressible concepts cost a constant. Ambiguous and invalidated concepts cost
as much as (or more than) relearning, so those categories provide no saving —
which is reported honestly rather than forced.

## Behavioral verification

- `replay_old_task`: the new ontology answers every query from the old task.
  This is false precisely when the change made an old requirement unanswerable
  (e.g. tokens that left the world or moved class), and is reported as such.
- `held_out_answered`: the new ontology answers held-out queries it was not
  built against, showing the migrated ontology generalizes beyond its training
  task.

## Example

`examples/concept_migration.rs` runs a Runner trajectory that produces all four
Runner-derived kinds — preserved, refined, re-expressible, ambiguous — plus a
novel `added` concept, and reports the two totals and saving. A second,
manually constructed ontology pair exercises `invalidated`, because the
observation runner accumulates tokens and never drops them.

```text
--- Runner trajectory ---
old#0 -> preserved (transfer=0)
old#1 -> refined   (transfer=2)
old#2 -> re-expressible (transfer=2)
old#3 -> ambiguous (transfer=6)
migration_transfer=10  cold_restart_transfer=36
migration_total=82     cold_restart_total=108   saving=26
replay_old_task=false  held_out_answered=true
```

The `false` replay is honest: the old task asked about tokens that moved class.

## Controls

- Deterministic construction and classification; identical input yields the
  identical machine record (`deterministic=true`).
- Frozen evaluation and exact universal fallback are preserved.
- Cold restart and migration share the same target ontology, so the comparison
  isolates transfer cost and cannot be gamed by choosing a cheaper ontology.
- Ambiguous/invalidated concepts are charged full re-derivation, so a migration
  "saving" is only earned where knowledge genuinely carries over.

## Claim and limits

Supported claim:

> When an ontology changes, existing knowledge can be carried forward cheaply
> where the class identity is preserved (preserved/refined) or renamed
> (re-expressible), and must be re-derived where the change splits or retires a
> class (ambiguous/invalidated); migration is cheaper than a cold restart by an
> amount equal to the genuinely reusable knowledge.

Limits: classification is structural (extension + label), not probabilistic;
the re-expression map is a single concept relabel, not a learned general
rewriting language; the observation runner cannot retire tokens, so
invalidation is exercised only on manually constructed ontologies; costs are
the declared comparable units.

## Reproduce

```sh
cargo test --release -p supsearch concept_migration --lib
cargo run --release --example concept_migration
cargo test --workspace
```

The example ends with `experiment=concept_migration,deterministic=true,...`
machine output.
