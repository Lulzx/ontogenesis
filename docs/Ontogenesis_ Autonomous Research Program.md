# Ontogenesis: Autonomous Research Program

## Mission

Continue developing Ontogenesis from its current state into a system that can progressively invent, revise, test, and reuse its own ontology and eventually parts of its own language of thought.

Do not treat the work as a sequence of arbitrary numbered milestones.

Treat this document as a standing research directive.

Continue advancing the system while each next experiment is scientifically justified, computationally bounded, falsifiable, reproducible, and compatible with the project's existing principles.

The north-star question is:

> Can a computational agent begin with a minimal substrate, discover useful structures in experience, acquire them because they reduce future reasoning cost, revise them when contradicted, and eventually invent the representations and hypothesis languages through which it reasons?

The desired developmental loop is:

```text
experience
    ↓
observational distinctions
    ↓
structural hypotheses
    ↓
provisional ontology
    ↓
executable concepts
    ↓
search / prediction / action
    ↓
new evidence
    ↓
counterfactual evaluation
    ↓
retain / refine / split / merge / replace
    ↓
new ontology
    ↓
new hypothesis language
    ↺
```

Do not ask for confirmation after every completed stage.

Advance automatically to the next scientifically meaningful experiment whenever:

1. the previous claim is fully implemented and tested;
2. the next limitation is clearly identifiable;
3. a bounded experiment can address that limitation;
4. the experiment can include adversarial controls;
5. no result must be forced to succeed.

If a proposed next step cannot be tested honestly at the current computational scale, document the boundary and pursue the strongest smaller experiment that attacks the same question.

---

# 1. Preserve the existing scientific contract

Existing Ontogenesis principles are load-bearing.

Preserve them unless a new experiment explicitly improves them.

## 1.1 Behavioral equivalence is observational

Never claim full semantic equivalence from finite probes.

Use language such as:

```text
observationally equivalent
bounded semantic class
equivalent within the frozen probe family
```

The system should eventually be able to revise these equivalence classes when new observations separate previously merged candidates.

---

## 1.2 Concepts are acquired counterfactually

A candidate is not a concept because:

```text
it appears often
it is mathematically elegant
it resembles a textbook abstraction
it compresses syntax
a human recognizes it
```

A candidate earns ontology status only if its installation measurably improves future reasoning under the declared cost model.

General form:

```text
Gain(c | O, D)
    = Cost(D | O)
    - Cost(D | O ∪ {c})
```

Use structural before/after costs rather than unsafe sentinel arithmetic.

Concept usefulness is conditional on:

```text
candidate
current ontology
task distribution
available generators
context
history
```

---

## 1.3 Universal fallback must remain exact

Learned allocation may change latency.

It must not silently remove universal coverage.

Where a fair universal search exists:

```text
filter(interleaved_schedule, universal_lane)
==
original_universal_schedule
```

This should remain asserted.

---

## 1.4 Separate incomparable work domains

Do not invent conversion ratios between unlike work units.

Examples:

```text
lambda evaluations
typed proposals
behavior-bank constructions
feature executions
signature candidates
normalization checks
environment interactions
```

Report them separately unless they are genuinely comparable under the same accounting rule.

---

## 1.5 Frozen evaluation

Protected information must not influence discovery.

Continue filtering:

```text
target-derived evidence
output-derived evidence
solution traces
target ancestry
protected IDs
post-freeze observations
near duplicates
duplicate groups
human labels encoding the answer
```

Protected annotation mutation should leave discovery invariant.

---

# 2. Current developmental state

Assume the repository already contains the following capabilities.

## Concept acquisition

Programs can become unit-cost concepts when doing so lowers future quotient-search cost.

## Generator acquisition

The system can acquire ways of proposing concepts, not only object-level concepts.

## Recursive-law invention

Finite program families can induce reusable recursive laws.

## Context and feature invention

Search allocation can depend on learned task representations and synthesized executable features.

## Relational universal structures

The system has bounded experiments for product-like and coproduct-like roles.

## Recursive universal structure

The system can discover an initial-algebra-like carrier and generic mediator under a supplied recursive signature.

## Recursive-signature discovery

The system can infer an anonymous bounded polynomial signature from structural evidence.

Weak evidence can remain ambiguous.

Rich evidence can collapse multiple syntactic aliases into one bounded semantic signature class.

Different worlds can select different recursive profiles using the same machinery.

## Open-world ontology revision

The current open-signature system does not require a supplied completeness flag.

It maintains provisional structural hypotheses and revises them as evidence changes.

The current demonstrated trajectory includes:

```text
[(0,0)]
→
[(0,0),(0,1)]
→
[(0,0),(0,1),(0,2)]
```

Compatible hypothesis classes contract as evidence accumulates.

Logical identifiability may remain false while a useful provisional ontology is still selected.

Hysteresis can discourage gratuitous revision but must never protect a falsified ontology.

Prior evidence must be replayed after structural revision.

---

# 3. Stop implementing arbitrary numbered milestones

Future work should be organized around unresolved capabilities, not U6/U7/U8 numbering.

Use descriptive names.

Examples:

```text
open_meta_language
ontology_repair
active_experimentation
probe_invention
representation_revision
structural_language_induction
concept_migration
self_generated_curriculum
causal_intervention
world_model_ontogenesis
```

A descriptive name should communicate the scientific question.

---

# 4. Major research direction A: ontology revision beyond simple extension

The current open-signature system mainly demonstrates ontology growth:

```text
O_t → richer O_{t+1}
```

Extend this to genuinely non-monotonic revision.

The system should support at least:

```text
retain
add
remove
split
merge
specialize
generalize
replace
```

## 4.1 Concept invalidation

Construct a world where an acquired concept initially predicts all observed evidence but later fails.

The learner must be able to demote or retire it.

Acceptance criterion:

```text
useful early concept
→ acquired
→ contradicted later
→ removed or revised
```

without corrupting unaffected concepts.

---

## 4.2 Concept splitting

Start with two behaviorally indistinguishable classes under limited probes.

Acquire one abstraction over the merged class.

Reveal new evidence separating them.

The learner should transform:

```text
C
```

into something like:

```text
C₁
C₂
```

and replay old knowledge under the refined ontology.

Measure:

```text
old equivalence class
new equivalence classes
knowledge preserved
knowledge invalidated
search cost before/after split
```

Do not simply rebuild the entire system from scratch and call that revision.

---

## 4.3 Concept merging

Construct the dual case.

Two early concepts appear distinct because of superficial observations.

Later experience shows no downstream value in maintaining the distinction.

Allow:

```text
C₁ + C₂ → C
```

when merging lowers total reasoning cost without losing protected predictive distinctions.

---

## 4.4 Structural replacement

Create a case where:

```text
old ontology + patches
```

is more expensive than:

```text
new ontology
```

The learner should be able to replace its structural model rather than endlessly append exceptions.

Use a cost such as:

```text
TotalCost(O_t)
=
description_cost
+ predictive_error
+ reasoning_cost
+ migration_cost
+ revision_penalty
```

The exact formula may differ, but every term must correspond to measurable work or explicit complexity.

---

# 5. Major research direction B: knowledge migration across ontology changes

Ontology revision becomes much more interesting when previously acquired concepts depend on the old ontology.

Implement explicit migration.

Given:

```text
O_old → O_new
```

classify each acquired concept as:

```text
preserved
re-expressible
refined
ambiguous
invalidated
```

A migration should not be accepted merely because a syntactic translation exists.

Verify transferred concepts behaviorally on replay and held-out evidence.

Measure:

```text
concepts preserved
concepts recomputed
concepts invalidated
migration search cost
cold-restart cost
post-migration reasoning cost
```

The key comparison is:

```text
ontology revision + migration
vs
discard everything + relearn
```

A successful developmental architecture should often preserve useful cognition through representational change.

---

# 6. Major research direction C: remove the fixed signature meta-language

The current recursive-signature learner still receives:

```text
Unit
Rec
Param
Sum
Prod
```

as the permanent language of structural hypotheses.

This is now a major human-supplied ontology.

The system should begin discovering useful structural languages themselves.

Do not immediately attempt an unrestricted meta-language.

Build a bounded meta-search.

## 6.1 Competing structural languages

Define several small anonymous structural grammars that overlap but differ in expressive bias.

For example:

```text
Language A:
  atom
  sum
  product

Language B:
  atom
  sequence
  branch

Language C:
  generic composition operators

Language D:
  small lambda-encoded structural operators
```

Avoid giving them semantic names inside the learner.

Give the system evidence from several worlds.

Measure which language:

```text
explains evidence
generates useful ontologies
reduces downstream search
generalizes to protected worlds
```

Charge the cost of choosing and using the meta-language.

---

## 6.2 Meta-language acquisition

The learner should retain structural constructors when they provide repeated cross-world value.

For instance, if an anonymous binary combination operator repeatedly produces useful ontologies, it may become a meta-level primitive.

This is the same acquisition philosophy one level higher:

```text
meta-constructor m is acquired
iff
future ontology search becomes cheaper with m
```

---

## 6.3 Meta-language revision

Do not assume meta-languages only grow.

If a structural operator becomes redundant or misleading under expanded experience, allow its removal or demotion.

Eventually support:

```text
M_t → M_{t+1}
```

where `M` is the language in which ontology hypotheses themselves are expressed.

---

# 7. Major research direction D: invent observational probes

A major remaining source of human ontology is the probe set.

Currently humans often choose the observations that make structural distinctions visible.

The agent should increasingly discover:

> What should I measure to distinguish my competing world models?

## 7.1 Passive probe invention

Give the learner a generic probe language.

Possible primitives:

```text
apply candidate
compose
compare
count
project
iterate
construct small contexts
observe normalized behavior
```

The exact substrate should remain domain-neutral where possible.

Enumerate candidate probes.

Score them by reduction in uncertainty or future reasoning cost.

Example objective:

```text
ProbeValue(p)
=
ExpectedHypothesisReduction(p)
- ProbeExecutionCost(p)
```

or deterministic bounded equivalent.

---

## 7.2 Distinguishing previously equivalent hypotheses

Create two ontology hypotheses that agree on all existing probes.

There must exist a small executable probe that separates them.

The system should discover that probe.

This experiment is important because it closes a loop:

```text
ontology
→ predicts what evidence would distinguish competitors
→ generates probe
→ receives result
→ revises ontology
```

---

## 7.3 Avoid answer-coded probes

Probe generation must not receive:

```text
target ontology ID
protected result
hidden correct label
post-hoc distinguishing example
```

The probe should arise from differences between candidate models.

---

# 8. Major research direction E: active experimentation

Once probes can be invented, allow the system to choose actions.

The learner should not merely wait for evidence.

It should ask:

> Which interaction would most reduce uncertainty between my current hypotheses?

Within a small simulated world, implement:

```text
hypotheses H
available actions A
predicted observations under each H
```

Choose:

```text
a* = argmax_a information_or_regret_gain(a)
```

A non-probabilistic finite version is fine.

For example:

```text
score(action)
=
number of currently merged hypothesis classes separated by its possible outcomes
- execution cost
```

---

## 8.1 Crucial experiment

Construct two environments that are observationally identical under passive data but differ under one intervention.

Passive learner:

```text
cannot distinguish
```

active learner:

```text
chooses intervention
observes result
revises ontology
```

This is a very important step toward scientific behavior.

---

# 9. Major research direction F: causal ontology

Current structures are largely observational and compositional.

Introduce tiny causal worlds.

The learner should distinguish:

```text
correlation
mechanism
intervention response
```

Start with finite deterministic systems.

Example hidden structures:

```text
A → B → C
A → C ← B
A ← B → C
```

Do not supply these graph names to the learner.

The system should use interventions to infer executable causal structure.

Acquisition criterion should remain practical:

```text
causal abstraction retained
iff
it improves prediction/planning under interventions
```

Do not claim general causal discovery.

---

# 10. Major research direction G: world-model ontogenesis

Move from isolated task families toward persistent environments.

Create tiny deterministic worlds with:

```text
state
actions
observations
hidden structure
repeated episodes
```

The agent begins with minimal assumptions.

It should invent concepts that improve:

```text
prediction
planning
state compression
transfer
experiment selection
```

Candidate invented structures might include anonymous equivalents of:

```text
object
container
location
inventory
door
key
counter
sequence
branch
reversible action
irreversible action
```

Do not seed those concepts.

The objective is not human interpretability.

The objective is reduced future reasoning cost.

---

# 11. Major research direction H: self-generated curriculum

So far humans design developmental curricula.

Begin reducing that dependency.

Given several reachable tasks or environments, let the system choose what to learn next.

A useful curriculum item is one that enables future cost collapse.

Possible objective:

```text
CurriculumValue(task)
=
expected ontology gain
+ expected downstream search reduction
- learning cost
```

Test whether the learner selects intermediate tasks that make a harder target solvable.

This should resemble:

```text
easy concept
→ acquired
→ enables harder concept
→ enables previously unreachable task
```

without the curriculum order being manually provided.

---

# 12. Major research direction I: abstraction over histories, not only static programs

Many useful concepts describe temporal patterns.

Introduce event histories.

Examples:

```text
A then B
repeat until C
toggle
periodic
once-only
reversible
state-resetting
```

Do not provide these names.

Use a small sequence-processing substrate.

Ask whether abstractions over history reduce prediction and planning cost.

This is necessary for agents operating in environments rather than static program synthesis tasks.

---

# 13. Major research direction J: hierarchical ontology

A flat concept bank is eventually insufficient.

Allow concepts to depend on other concepts.

Represent an ontology as a dependency structure:

```text
C₁
C₂(C₁)
C₃(C₁,C₂)
...
```

Measure:

```text
dependency depth
concept reuse
search savings
revision propagation
```

If a foundational concept is revised, downstream concepts should be revalidated selectively.

Avoid recomputing everything when dependency information can localize the affected region.

---

# 14. Major research direction K: ontology-local reasoning languages

Different domains may benefit from different internal languages.

Allow the system to acquire a local reasoning grammar conditioned on ontology/context.

For example:

```text
O_arithmetic → G_arithmetic
O_recursive → G_recursive
O_spatial → G_spatial
```

The grammars should be learned because they reduce search cost.

Eventually allow composition between them.

The goal is not one universal DSL manually designed by us.

The goal is:

```text
experience
→ ontology
→ locally useful executable language
```

---

# 15. Major research direction L: abstraction compilation and human inspection

Machine-discovered concepts may become difficult for humans to interpret.

Add a diagnostic layer that attempts to summarize learned concepts without affecting discovery.

Possible outputs:

```text
minimal lambda term
behavior table
dependency graph
distinguishing probes
nearest known mathematical analogue
counterexamples
domains where useful
domains where harmful
```

Interpretability tooling must be observational only.

Do not feed human-readable labels back into search unless explicitly part of an experiment.

---

# 16. Major research direction M: external transfer

Do not remain indefinitely inside synthetic lambda worlds.

Once mechanisms stabilize, test them on small external domains.

Possible domains:

## ARC

Use Ontogenesis to invent:

```text
structural features
object representations
transformation concepts
search allocation
```

Avoid claiming ARC superiority from hand-selected tasks.

Gradually move toward a frozen population-level evaluation.

---

## Code reasoning

Use repositories or small program families where hidden semantic abstractions recur.

Test whether the system can discover reusable concepts that reduce later bug-finding or synthesis cost.

---

## Protocol inference

Create tiny unknown protocols/state machines and test whether ontology formation reduces interaction complexity.

---

## Symbolic mathematics

Use theorem/problem families where invented definitions shorten subsequent proof search.

---

# 17. Required adversarial controls for every major experiment

Every experiment should ask:

```text
What is the easiest skeptical explanation of this result?
```

Then implement a control against it.

Common controls include:

```text
candidate-order control
smaller-syntax prior control
lookup/memorization control
label leakage control
target-derived evidence control
post-freeze mutation
irrelevant ontology
misleading ontology
uniform allocation
external oracle
raw baseline
universal fallback
truncation disclosure
weak-evidence ambiguity
wrong-arity structure
hidden-state structure
noise/anomaly control
cold restart
random curriculum
shuffled context
```

Do not weaken an experiment after seeing a negative result.

A clean falsification is valuable.

---

# 18. Research result classification

Each experiment should end in one of four states.

## Confirmed

The intended bounded claim passes all preregistered controls.

## Partial

Some mechanism works, but a stronger intended claim fails.

Record both.

## Negative

The proposed mechanism does not produce the desired effect.

Document why and use the failure to identify the next structural limitation.

## Inconclusive

Search or evaluator boundaries prevent a scientifically meaningful conclusion.

Do not report an exhausted computational budget as evidence of impossibility.

---

# 19. Autonomous progression rule

After completing an experiment:

1. identify the strongest remaining human-supplied assumption;
2. determine whether removing it is currently testable;
3. identify the strongest skeptic's explanation of the current result;
4. design the smallest experiment that distinguishes the current explanation from the stronger one;
5. implement it;
6. add controls;
7. run full verification;
8. document exact boundaries;
9. commit;
10. continue.

Do not create a new milestone merely because the previous one has a number.

Continue only when the next experiment changes the scientific question.

---

# 20. Stop conditions

Pause autonomous advancement only when at least one of the following is true.

## Fundamental computational wall

The next experiment requires search many orders of magnitude beyond the current implementation and no meaningful bounded proxy is available.

## Missing theoretical substrate

The next step requires a formalism whose semantics are not sufficiently defined to implement without arbitrary choices.

## Experimental ambiguity

Several substantially different experiment designs would imply different scientific claims, and choosing among them would amount to changing the research agenda rather than implementing it.

## External-data decision

A meaningful next result requires selecting a new external benchmark, dataset, or domain with substantial consequences.

## Architecture rewrite

Further progress requires replacing a major subsystem rather than extending the current research harness.

When stopping, produce:

```text
CURRENT_STATE.md
NEXT_BOUNDARY.md
```

with:

```text
what is proven
what failed
what remains supplied
what the next scientific question is
candidate experiments
estimated computational barriers
recommended direction
```

Do not simply say "ask the user what to do next."

---

# 21. Engineering standards

Keep experiments:

```text
deterministic
small
release-mode reproducible
machine-readable
well tested
independently falsifiable
```

Every major result should have:

```text
src/<experiment>.rs
examples/<experiment>.rs
<EXPERIMENT>.md
tests
machine-readable record
README/MILESTONES/VERIFICATION update when warranted
```

Avoid pushing everything into `main.rs`.

Prefer reusable primitives when multiple experiments duplicate the same machinery.

Do not refactor solely for aesthetics while scientific work is ongoing.

---

# 22. Git discipline

Work in small coherent commits.

Recommended pattern:

```text
<experiment>: implement mechanism
tests: add falsification controls
docs: record result and limitations
```

Do not leave major working changes uncommitted.

Before destructive cleanup:

```text
git status
git diff
git add/commit if valuable
```

Never delete uncommitted experimental work merely because it appears generated.

---

# 23. Documentation discipline

Every claimed result must distinguish:

```text
discovered
supplied
inferred
verified
bounded
exhaustive within bound
untruncated
not claimed
```

Prefer qualified statements such as:

> Within the declared bounded observational world...

over universal claims.

Report negative controls as prominently as positive results.

---

# 24. Long-term convergence target

The system should progressively reduce the amount of human-authored ontology in this stack:

```text
human-supplied tasks
human-supplied probes
human-supplied equivalence
human-supplied representations
human-supplied concepts
human-supplied generators
human-supplied structural signatures
human-supplied meta-language
human-supplied curriculum
human-supplied experiments
```

The goal is not necessarily to remove every prior.

The goal is to make increasingly high-level assumptions **learned, revisable, and justified by measured downstream utility**.

A mature form of Ontogenesis should approximate:

```text
minimal computational substrate
        +
experience stream
        +
resource bounds
        ↓
provisional world model
        ↓
invented ontology
        ↓
invented executable concepts
        ↓
invented probes
        ↓
chosen experiments
        ↓
ontology revision
        ↓
new internal language
        ↓
more efficient cognition
        ↺
```

---

# 25. Ultimate experimental question

The project should eventually be able to test:

> Can a machine discover abstractions that were not represented in its initial object-level vocabulary, use them to make previously expensive reasoning cheap, notice when those abstractions stop being adequate, invent observations that distinguish competing replacements, and preserve useful knowledge while changing the language in which it thinks?

Do not optimize for producing impressive named mathematical objects.

Do not optimize for incrementing milestone numbers.

Optimize for making that loop increasingly real.

Continue until you reach a genuine scientific boundary.