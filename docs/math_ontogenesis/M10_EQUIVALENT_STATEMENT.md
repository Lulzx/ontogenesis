# M10 — Invent an Equivalent Statement

## Required experiment

Starting from a proposition such as “`n` is even iff `n²` is even,” M10 must
generate alternative predicates `Q`, prove both `P->Q` and `Q->P`, and compare
description length and proof-search cost. Finite agreement is insufficient
because the proposition quantifies over all integers.

## Status: boundary, not reached

M1–M9 provide expression evaluation, bounded behavioral synthesis, recurrence
inference, and a verified successor-closure schema. They do not provide a typed
proposition AST, quantified predicate semantics, proof terms, or an independent
trusted proof checker. Consequently the system cannot yet distinguish a real
equivalence proof from agreement on sampled integers.

## Why no nominal solution was encoded

Supplying parity lemmas or the known equivalence would leak the object the
milestone is meant to invent. Labeling a truth-table search as proof would also
overstate the evidence. The standing directive therefore makes this a genuine
architecture stop.

## Evidence required to complete M10

A valid continuation needs domain-general proposition enumeration, separately
checked proof terms for both directions, answer-blind proposal generation, a
finite-sampling rejection control, and a measured downstream proof-cost gain.
The proposed substrate is specified in `NEXT_BOUNDARY.md`.
