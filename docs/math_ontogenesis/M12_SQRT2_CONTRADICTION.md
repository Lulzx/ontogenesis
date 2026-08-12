# M12 — Invent Contradiction for sqrt(2)

## Required discovery

M12 asks whether `sqrt(2)` is rational and requires a formal proof. The parity
and lowest-terms representation must be invented; the classical proof may not
be provided.

## Status: not reached

The current system can evaluate squares and discover finite invariants, but it
cannot express coprimality assumptions, quantify over integer ratios, derive
parity from divisibility, or check a contradiction proof. Those are precisely
the proposition/proof capabilities missing at M10.

## Completion criteria

A valid run would search intermediate predicates, discover that rewriting
`sqrt(2)=p/q` as `p²=2q²` exposes a parity invariant, formally derive that both
`p` and `q` are even, contradict a checker-verified lowest-terms premise, and
measure transfer to other irrationality proofs. Numerical irrationality tests
or an encoded classical script would not satisfy the milestone.
