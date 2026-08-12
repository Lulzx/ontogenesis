# M12 — Invent Contradiction for sqrt(2)

## Required discovery

M12 asks whether `sqrt(2)` is rational and requires a formal proof. The parity
and lowest-terms representation must be invented; the classical proof may not
be provided.

## Status: next boundary, not reached

M10 can check universal modular propositions and M11 can validate a finite-list
existential witness schema. The system still cannot express normalized rational
witnesses, derive lowest terms, carry divisibility through `p²=2q²`, or check a
contradiction that invalidates an assumed rational representation.

## Completion criteria

A valid run would search intermediate predicates, discover that rewriting
`sqrt(2)=p/q` as `p²=2q²` exposes a parity invariant, formally derive that both
`p` and `q` are even, contradict a checker-verified lowest-terms premise, and
measure transfer to other irrationality proofs. Numerical irrationality tests
or an encoded classical script would not satisfy the milestone.
