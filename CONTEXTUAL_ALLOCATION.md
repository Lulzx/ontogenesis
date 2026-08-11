# Contextual and interaction-aware search allocation

## Result

The allocator now estimates utility conditional on an observable task context and
supports explicitly bounded concept sets:

```text
U(concept set | task features, ontology, developmental history)
```

It no longer assumes that one global concept ordering is appropriate for every task.
Policies are immutable snapshots learned from earlier evidence and frozen before a
held-out search. The universal lambda lane is unchanged and remains the completeness
floor.

## Shared accounting without false equivalence

`search_accounting` gives both engines one reporting envelope while retaining two
different work variants:

- `UniversalLambda`: proposals, evaluated candidates, and exact resource points;
- `BehaviorBank`: candidate constructions (`built`), retained candidates, and
  aborted candidates.

Both also report structural frontier, labeled evaluator budget, solution rank,
termination, task/family/duplicate IDs, observable features, concept IDs, phase, and
epoch. Same-engine runs aggregate exactly. Mixed-engine aggregation returns
`MixedEngines`; no conversion factor or combined scalar exists. Wall time is excluded
from equality. The ARC raw diagnostic exhausts a fixed syntax boundary rather than a
wall-clock boundary, so replayed non-time counters are deterministic.

## Frozen contextual policy

Each evidence record contains a paired without/with search measurement, task context,
canonical concept set, age, epoch, and derivation provenance. Learning rejects:

- the held-out task itself;
- an exact/near-duplicate split group;
- target-program-derived or protected-output-derived evidence;
- evidence whose ancestry includes the held-out task;
- evidence recorded after the freeze epoch;
- mixed engine units;
- concept sets beyond the declared maximum width.

A regression starts from a valid frozen policy, injects every rejected evidence kind,
and proves the ranking and engine remain identical while the rejection counters
increase.

Context similarity is computed from predeclared observable features. Partial feature
overlap transfers less credit than an exact match. Evidence decays by age. Confidence
is currently the deterministic evidence-count proxy `n/(n+1)`, not a calibrated
probability.

Set utility is residualized against positive singleton credit:

```text
interaction(A,B) = utility({A,B}) - positive(A) - positive(B)
```

This admits synergy, assigns no new credit to a redundant pair, and makes an
antagonistic pair negative. Candidate sets are supplied by a bounded width-2 proposal
pool; arbitrary subset enumeration is not performed.

## Synthetic causal experiments

Earlier executable searches provide utility evidence for two representation
contexts. The final tasks use disjoint observations and reverse the earlier calling
protocols:

- reversed single-chain recursion requires acquired `not`;
- reversed nested-chain recursion requires acquired recursive `parity`.

Every winner passes the unchanged closedness, fixed-point, live-recursion,
distinct-output, discovery, and extrapolation checks.

```text
condition                  solved    proposals   evaluated   points   universal
contextual                   2/2          670         278       26       yes
global utility               1/2          983         405       27       yes
uniform ontology             2/2        1,318         544       40       yes
hand oracle                  2/2          670         278       26       yes
shuffled context labels      0/2        1,296         532       28       yes
irrelevant                   0/2        1,296         532       28       yes
misleading                   0/2        1,296         532       28       yes
universal-only through 9     0/2        5,244       3,408       18       yes
contextual, no universal     2/2          670         278       14        no
```

Contextual allocation independently swaps `not` and `parity`, solves both tasks, and
has zero proposal regret against the oracle. Global utility selects `parity` for both
and solves only one. Shuffling the context labels removes the gain.

A separate two-step anonymous recursive representation requires two opaque Boolean
steps. On disjoint training and held-out probes, neither singleton nor universal-only
finds the law through syntax size 9. The learned set `{first,second}` has interaction
residual 4,326 and solves; disabling interaction modeling fails.

Unit tests also pin:

- a context where A and B reverse usefulness;
- a useful pair, a redundant pair, and an antagonistic pair;
- decay adapting after distribution shift with lower cumulative regret than a stale
  policy, while replay of the old context still selects its old concept;
- fixed learned-budget competition that excludes uncertain/nonpositive candidates;
- constants, dead recursion, shortcuts, aggregate overfit, divergence, and
  compression-only controls inherited from the unchanged recursive validator.

## Universal invariance

`InterleavedDovetail::next_labeled` makes lane provenance observable. Deterministic
generated policies include empty, invalid, duplicate, extreme, and ordinary learned
points. For every case, projecting the labeled stream onto `Universal` equals the
first N points of the original `Dovetail` exactly. A second test locates sampled finite
universal pairs at finite interleaved indices behind 100 adversarial learned points.

Thus a wrong high-confidence contextual policy may delay a hypothesis but cannot
delete it. The no-universal ablation can solve these samples and still correctly loses
the theorem.

## Preregistered real ARC-1 slice

The local corpus contains exactly one pure horizontal mirror task, one pure vertical
flip task, and two exact 180-degree rotation tasks. The split is fixed in code:

```text
earlier training: 67a3c6ac (mirror), 68b16354 (vertical flip)
calibration:       3c9b0459 (180-degree rotation)
final holdout:     6150a2bd (180-degree rotation)
```

The context extractor sees only each task's published training pairs. It records a
generic D4 relation and whether shape is preserved. Mutating the final task's protected
test output leaves its context byte-identical. The policy, feature extractor, width-2
interaction model, decay, and split are frozen before final search. The discovered
program is then independently checked on the protected test pair; that output never
participates in allocation or search.

All conditions use the same canonical behavior-bank concept lane. The raw diagnostic
uses a separately labeled, deterministic exact-size-7 raw-bank boundary.

```text
condition                bank built   train solved   protected test verified
contextual {mirror,vflip}       5          yes                  yes
global mirror                   3           no                   no
uniform mirror→vflip→pair      11          yes                  yes
oracle pair                     5          yes                  yes
shuffled context                3           no                   no
interaction disabled            3           no                   no
irrelevant identity             1           no                   no
misleading projection       2,016           no                   no
raw bank through size 7      3,405           no                   no
```

Contextual allocation matches the oracle with zero `built` regret and improves on
uniform allocation from 11 to 5 constructions. Its score margin is 1 and its
evidence-count confidence proxy is 500/1000. Paired held-out solve rate is 1/1. This
single final task is a deterministic existence demonstration, not an estimate of ARC
population performance and not a statistical confidence interval.

Because every condition stops at its first independently verified solution, `built`
is also the reported allocation-level solution rank for the successful rows:
contextual/oracle rank 5 and uniform rank 11; failed rows report no rank.

The ARC result is deliberately **not called universal**. Every ARC row has
`universal=false`; `built` is never added to lambda proposals. The universal theorem is
preserved and tested in the synthetic recursive engine, while ARC demonstrates that
the same frozen contextual/interaction learner transfers over a correctly labeled
bounded-bank adapter.

## Reproduction

```sh
cargo run --release --example contextual_allocation
cargo run --release -p arc1 -- contextual
cargo test -p supsearch contextual_allocation --lib
cargo test -p supsearch contextual_guidance --lib
cargo test -p supsearch universal --lib
cargo test -p arc1 contextual_arc_transfer_is_frozen_verified_and_deterministic
cargo test --workspace
```

Both release commands emit human-readable rows and `record,...` machine-readable
rows.

## Boundaries

The feature vocabulary, similarity function, decay rate, confidence proxy,
interaction width, candidate-set pool, and ARC split are fixed by the experiment; the
system does not learn them. The ARC slice contains only one final task and one
geometric context, so it does not establish broad heterogeneous ARC performance.
Concept-set enumeration is width-bounded. Distribution-shift evidence is controlled
and synthetic. Universal search remains combinatorial and semidecidable, with the same
finite machine, evaluator, memory, stack-guard, and undecidability limits documented
for B2-general. Representation invention still assumes supplied anonymous constructor
arities and neither infers raw-byte signatures nor proves a unique latent encoding.
