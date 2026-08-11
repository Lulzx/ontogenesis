# Learned allocation over universal recursive search

## Claim

The ontology no longer receives a hand-written priority order. A deterministic
utility ledger learns that order and its finite syntax/fuel allocation from prior
counterfactual search measurements. Learned work is alternated 1:1 with the original
universal dovetail, so any learned score can change time-to-first-test but cannot
remove a pure-lambda hypothesis or starve a finite resource point.

This is a search-allocation result, not a new expressiveness result. B2-general's
qualified completeness theorem is unchanged.

## Protocol

The developmental history is:

```text
O0 = {} -> discover parity through acquired not
          -> measure each concept's counterfactual utility
O1 = {not, parity, irrelevant pair, misleading identity}
          -> learn weights without seeing the target task
          -> search a held-out recursive protocol
```

The ledger records, for each `(training task, concept)` comparison:

- proposed and actually evaluated candidates;
- whether either condition solved and the first-solution rank;
- maximum syntax size and evaluation fuel reached;
- the proposal cost of widening the grammar;
- evidence age, with deterministic exponential decay;
- provenance flags that exclude target-derived evidence.

Evidence whose task ID equals the held-out ID is discarded. Concepts are ranked by
the resulting score. The current leader receives the full size/fuel lane, weaker or
stale positive evidence receives half fuel and one fewer syntax class, unknown
concepts receive a diagnostic quarter-fuel lane, and harmful concepts receive a
tenth-fuel lane and three fewer syntax classes. These thresholds are fixed before the
held-out run. They affect only the learned lane; they do not affect the universal
lane.

The target is not one of the training laws. Training's nested value protocol calls
`value parity recursive`; the held-out anonymous representation reverses those roles
and requires `value recursive parity`. Discovery and extrapolation examples are
disjoint. The extrapolation set includes single-payload even and odd cases plus
longer aggregates, preventing the previously observed aggregate-output surrogate.

## Fairness invariant

`InterleavedDovetail` emits:

```text
learned_0, universal_0, learned_1, universal_1, ...
```

and, after the finite learned allocation is exhausted, emits the ordinary `Dovetail`
forever. The universal subsequence is byte-for-byte the original diagonal stream.
Therefore every representable positive `(u32 syntax size, u64 fuel)` point still has
a finite position independent of learned weights. Evaluation remains bounded by the
existing `i64` conversion and stack guard. This preserves the exact B2-general claim:

> Every representable finite closed functional in the declared universal lambda
> language is eventually proposed, and every terminating observation within
> representable finite fuel is eventually tested.

The learned-without-universal ablation can find this particular target, but it is a
finite schedule and does not preserve that theorem.

## Measured result

Command:

```sh
cargo run --release --example learned_allocation
```

Deterministic proposal/evaluation counts from the release run:

| Held-out condition | Proposals | Evaluated | Resource points | Solved | Universal coverage |
|---|---:|---:|---:|:---:|:---:|
| universal-only through size 11 | 41,272 | 28,258 | 11 | no | yes |
| uniform ontology allocation | 2,310 | 956 | 55 | yes | yes |
| hand-designed parity-first oracle | 335 | 139 | 13 | yes | yes |
| learned allocation | **335** | **139** | **13** | yes | yes |
| irrelevant pair only | 648 | 266 | 14 | no | yes |
| misleading identity only | 648 | 266 | 14 | no | yes |
| learned, universal lane ablated | 335 | 139 | 7 | yes | **no** |

The expanded held-out functional has syntax size 31; using the previously discovered
parity executable as an opaque acquired concept makes it size 7. The learned scores
were `parity = 247,624`, `not = 59,209`, and `irrelevant = misleading = -4,412`.
Consequently parity is scheduled first; `not` receives a weaker positive lane; both
harmful controls receive smaller diagnostic lanes.

The learned policy has zero proposal regret against the hand-designed oracle, saves
1,975 proposals against uniform ordering, and has a calibration margin of 252,036
over the best matched control. The universal-only condition remains unsolved after
41,272 exhaustive proposals, while learned allocation solves at 335, so the measured
proposal separation is a lower bound of **at least 123x**. Proposal and evaluation
counts are the scientific metrics; wall time is reported by the executable but is not
used for the ratio.

Every admitted winner passes both discovery and extrapolation observations, the fixed
point equation, closedness, live recursive-reference ablation, and distinct-output
requirements. The existing evaluator fuel and scoped stack guard bound divergent
candidates.

## Controls

- Equally sized irrelevant and misleading atoms isolate useful semantic bias from
  grammar enlargement or syntax compression alone.
- Nonrecursive shortcuts, dead recursive references, constant-output families, open
  terms, and divergence are rejected by the unchanged recursive-search validator.
- Single-payload even/odd holdouts reject the known aggregate surrogate.
- Held-out task IDs and target-derived evidence are excluded. A regression injects a
  deliberately leaky held-out record and proves the ranking is unchanged and the
  record is counted as skipped.
- Utility is estimated from paired with/without measurements, not from how often the
  learned policy later selects a concept. This blocks self-fulfilling credit.
- Decay and allocation tests prove that stale positive evidence receives less budget,
  while measured harmful evidence receives less again.
- The universal-lane ablation distinguishes solving this sample from preserving the
  universal coverage guarantee.

## ARC-1 transfer boundary

The local ARC-1 crate was inspected rather than used as an uncontrolled add-on. It
has real ARC JSON loading and counterfactual `built` costs, but its search interface is
a separately bounded, behavior-deduplicated grid bank (`parse::Task`, wall-clock and
per-level caps, canonical grid keys). It does not expose the exact syntax-size/fuel
resource points or the unbounded pure-lambda fallback consumed by this learned
allocator. Treating ARC `built` counts as if they were universal resource lanes would
change both the observer and the coverage theorem.

Accordingly no ARC number is claimed for this milestone. A controlled ARC follow-up
must first define an adapter with stable per-concept work accounting, task-family
train/holdout IDs, and a separately scheduled universal lane. The existing real-ARC
geometry tests remain evidence about ontology transfer, not evidence for learned
universal resource allocation.

## Limits

This experiment learns among four already acquired concepts from two synthetic
training stages and evaluates one structurally changed held-out recursive family. It
does not learn the utility formula, decay rate, lane thresholds, task distribution,
or representations. Scores are not probabilities, and one successful developmental
sequence does not establish broad-domain calibration. Universal fallback supplies a
completeness floor, not practical tractability; combinatorial growth, memory, finite
machine integers, evaluator safeguards, and undecidability remain.

Focused verification:

```sh
cargo test -p supsearch learned_allocation --lib
cargo test -p supsearch ontology_guidance --lib
cargo test -p supsearch universal --lib
cargo test --workspace
```
