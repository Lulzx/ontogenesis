# Scientific Integrity Contract — Ontogenesis Experiments

This document is a binding design and reporting standard for every milestone
after M13. A mathematically correct result is not sufficient evidence of
ontology genesis when the winning representation is substantially encoded in
the supplied grammar.

## Core distinction

Every experiment must distinguish three achievements:

1. **Search:** selecting a useful expression inside a supplied representation;
2. **law invention:** proposing and independently proving a previously unnamed
   relation inside that representation;
3. **ontology genesis:** constructing a useful representation from a more
   primitive substrate, then retaining it because it improves frozen future
   reasoning.

Claims must use the weakest description that fully matches the evidence. A
system must not be said to have invented an object, invariance, coordinate
system, proof strategy, or ontology if an extensionally equivalent constructor
was supplied as a primitive or enforced by its types, grammar, candidate
template, canonicalization, scoring rule, or checker interface.

## Pre-registration boundary

Before running a milestone, its boundary document must freeze all of the
following:

- primitive data types and observations;
- primitive operations and constants;
- type restrictions and canonicalization;
- candidate constructors and maximum search depth;
- enumeration order and pruning/equivalence rules;
- interventions available to the learner;
- training, validation, control, and frozen downstream task generators;
- scoring, retention, and stopping rules;
- checker axioms, trusted lemmas, and certificate format;
- accounting units and the exact formulas used for costs;
- the intended claim and the conditions that would weaken or falsify it.

Changing any frozen item after observing a result creates a new experiment
version and must be recorded. Failed grammars and negative runs must not be
silently replaced by a successful hand-tuned grammar.

## Supplied-ontology ledger

Every machine record and human report must identify what was supplied at each
of these channels:

| Channel | Questions that must be answered |
| --- | --- |
| Representation | Were invariance, ordering, quotienting, latent objects, or coordinates encoded in the data type? |
| Primitives | Was the winner, an equivalent constructor, or a domain-specific measurement available atomically? |
| Grammar | Did a target-shaped template sharply constrain how primitives could combine? |
| Search | Did enumeration order, pruning, or a privileged depth favor the winner? |
| Data | Were examples selected after seeing candidate failures? Could finite coincidences identify the target? |
| Objective | Did the score directly reward the desired named structure rather than task utility? |
| Checker | Does the checker only verify a proposal, or does its interface effectively construct or identify it? |
| Accounting | Is reported gain measured execution work or a modeled formula? |

The ledger must name the closest supplied object to every winning concept and
explain the remaining construction distance. “Not supplied by name” is not
enough; extensional and type-level equivalents count as supplied structure.

## Required evidence

No milestone counts as ontology genesis unless all applicable requirements
below pass.

### 1. Generative distance

The retained concept must require a nontrivial construction from the frozen
substrate. Report at least:

- minimal discovered program size;
- number of candidates tested before retention;
- search depth and branching;
- intermediate retained concepts used by the winner;
- the closest supplied primitive or template;
- whether a shorter extensionally equivalent expression was available.

These are descriptive measures, not a universal scalar metric. Do not collapse
them into a single “ontology distance” number without independently justifying
that metric.

### 2. Scaffolding ablations

Run frozen ablations that remove or replace each plausible source of target
knowledge. At minimum compare:

- the actual substrate;
- a stronger, target-privileged substrate;
- a weaker substrate expected to make discovery harder;
- irrelevant primitives with comparable syntactic cost;
- shuffled or adversarial observations/interventions where meaningful.

Report failures as evidence about dependency on scaffolding. An ablation is not
valid if the search budget, task distribution, or checker is quietly changed to
favor the desired outcome.

### 3. Independent validation

Finite regression never establishes a mathematical law. A checker must:

- be separate from candidate generation and scoring;
- recompute semantic obligations from the submitted certificate;
- reject malformed, corrupted, finite-fit, and target-shaped false controls;
- validate a general schema, not merely replay training examples;
- expose all trusted axioms and domain lemmas.

A checker may contain the theorem needed to verify a result, but its API must
not reveal which candidate to propose. If it returns gradients, counterexamples,
normal forms, or error distinctions to search, those information channels must
be declared as part of the supplied ontology.

### 4. Frozen downstream utility

Acquisition requires measured benefit on tasks frozen before concept discovery.
Report both conditions under identical budgets:

- reasoning from the original substrate;
- reasoning with the retained concept added.

Use actual counted work whenever possible. A formula such as
`task_count * candidate_count` is a modeled estimate and must be labeled
`modeled_gain`, never simply `reasoning_gain`. Report negative transfer and
tasks on which the concept provides no benefit.

### 5. Transfer

The retained representation must transfer beyond surface variants of its
discovery data. Prefer changes in at least two independent dimensions, such as:

- object cardinality or polynomial degree;
- numeric domain;
- task family;
- presentation or encoding;
- proof goal;
- intervention group.

If types, grammar productions, or feature indices are rebuilt separately for
each transfer domain, the result is degree- or task-specific search rather than
a transferred ontology.

## Claim levels

Use these labels in milestone reports and machine records:

- `L0_finite_fit`: matches observations only;
- `L1_checked_law_in_supplied_representation`: independently proved, but the
  useful representation was supplied;
- `L2_invented_feature_in_supplied_meta_ontology`: the feature was composed
  from generic primitives, while the relevant transformation class, objective,
  or task shape was supplied;
- `L3_transferred_ontology_with_measured_utility`: an invented representation
  transfers and reduces actual work on frozen downstream tasks;
- `L4_multistage_ontology_genesis`: multiple independently retained ontology
  transitions enable a result not tractable in the original substrate under
  matched resources.

Reports may state higher mathematical proof status separately. Formal validity
does not raise the ontology-genesis level by itself.

## M13 and M13b audit

### Original M13

- Supplied: unordered-root canonicalization, permutation orbit sums, degree
  bound two, and the template `coefficient * symmetric_feature ± coefficient`.
- Discovered: which two supplied features pair with which coefficients and
  signs.
- Checked: universal quadratic factorization consequences.
- Honest level: `L1_checked_law_in_supplied_representation`.
- The reported `324 -> 12` was modeled accounting, not frozen downstream task
  execution.

### M13b

- Removed: unordered-root type, orbit-sum constructor, exponent partitions,
  and the target-shaped relation template.
- Supplied: two addressable root slots, the swap intervention, arithmetic,
  exact polynomial normalization, quadratic factorization semantics, bounded
  program sizes, and an objective that retains exact swap invariants involving
  both slots.
- Invented: `root[0] + root[1]` and `root[0] * root[1]` as short programs stable
  under the supplied swap action; subsequent generic arithmetic search finds
  their coefficient laws.
- Checked: exact invariance under the transposition and universal membership in
  the quadratic factorization ideal.
- Honest level: `L2_invented_feature_in_supplied_meta_ontology`.
- Current future-cost reduction remains modeled and must not be called measured
  downstream acquisition.

M13b therefore improves the ontology distance substantially, but it does not
invent the permutation action, the idea that stability under that action is
valuable, arbitrary-cardinality collection symmetry, or a transferable family
of elementary symmetric coordinates.

## Gate for the next claim

Before claiming `L3` or above for symmetric coordinates, the system must:

1. operate over varying root cardinalities without degree-specific feature
   indices or rebuilt grammars;
2. construct invariant aggregations from generic collection programs;
3. transfer the same retained generator to unseen polynomial degrees;
4. beat matched baselines on frozen coefficient/root reasoning tasks using
   actual search work;
5. pass ablations for permutation interventions, arithmetic primitives,
   invariant-retention objectives, and task distributions;
6. report all unsuccessful pre-registered variants and negative transfer.

## Review checklist

Before marking any boundary complete, answer each question with evidence:

- [ ] Could the winning concept be recovered by renaming a supplied primitive?
- [ ] Does a type or canonicalizer already enforce the property claimed as invented?
- [ ] Does the grammar encode the winning equation or proof shape?
- [ ] Were the boundary and datasets frozen before observing results?
- [ ] Are proposal and checking information channels separated and documented?
- [ ] Do symbolic/adversarial controls reject finite coincidences and shortcuts?
- [ ] Were scaffolding ablations run under matched resources?
- [ ] Is transfer structural rather than a surface-value change?
- [ ] Is utility measured on frozen downstream tasks rather than estimated?
- [ ] Are failures, negative transfer, trusted lemmas, and caveats reported?
- [ ] Does the claim label match the weakest demonstrated level?

If any answer is “no,” the milestone may still be a valid search or theorem
result, but it must not be reported as stronger ontology genesis.
