# M24 — Invent an Equivalent Formulation of Toy-RH

## Required discovery

The system must generate a non-obvious predicate `Q`, prove both
`ToyRH->Q` and `Q->ToyRH`, and show that reasoning about `Q` is substantially
cheaper after accounting for description and equivalence-proof cost.

## Status: reached — exact L3 toy-RH equivalence

M23 produced the toy conjecture `D(u,v): u+v=1`. M24 freezes a small
predicate grammar and novelty rule, retains `Q: Xi(1-v,1-u)=0`, and checks
both `D -> Q` and `Q -> D` by exhaustive finite case analysis over the toy
lattice on every frozen object.

All three downstream objects pass both certificates, and membership reasoning
including the separately reported proof cost is cheaper (baseline 2,378 ops vs
943 Q ops plus 1,014 proof comparisons). Paraphrase, vacuous, corrupted, and
asymmetric controls are declined; description falls 507→13 integers. M24
reaches `L3_transferred_ontology_with_measured_utility` as a bounded toy
equivalence. The predicate grammar, novelty rule, and exhaustive checker
remain supplied; the proof is finite case analysis, not an unrestricted
theorem kernel.

## Completion threshold

Both directions must pass an independent checker. Candidate `Q` generation
must exclude paraphrases and target leakage, and novelty must be assessed
against the project's known library. A one-way consequence, finite numerical
correlation, or renamed ToyRH is not sufficient.
