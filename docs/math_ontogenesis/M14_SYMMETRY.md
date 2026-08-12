# M14 — Invent Symmetry as a Representation

## Required discovery

M14 asks the system to invent input transformations that preserve or negate
observed outputs, then reuse that abstraction across functions, geometry,
polynomials, matrices, and finite groups. “Even,” “odd,” and “symmetry” are not
supplied.

## Status: not attempted

M2 and M7 discovered invariants, and M9 found special directions, but none
searches transformations as first-class objects across heterogeneous domains.
The ordered ladder is already stopped at M10, so no M14-specific architecture
has been added.

## Evidence required

Completion requires a transformation grammar, composition and inverse tests,
invariant/equivariant predicate discovery, held-out cross-domain transfer, and
cost reduction versus storing pairwise coincidences. Degenerate constant
functions must be a control. Merely recognizing `f(x)=f(-x)` from supplied
pairs would not establish the reusable representation demanded here.
