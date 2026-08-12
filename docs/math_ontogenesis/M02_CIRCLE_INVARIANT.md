# M2 — Invent the Circle Invariant

## Question and supplied world

The system received positive and negative point examples. It was allowed the
arithmetic substrate acquired in M1 and equality, but not circle, radius, or
origin. The task was classification by an invented invariant rather than
another pointwise predictor.

## What it figured out

The search enumerated expressions `f(x,y)` and tested whether all members had
one value `c` while every non-member differed. The first generalizing result
was `(x*x)+(y*y)=25`, size 7, after 57,883 distinct behaviors. Four held-out
members and four held-out non-members were classified correctly.

## Reuse and compression

Once retained, eight classifications cost eight evaluations. Rediscovery plus
classification cost 57,891, so reuse saved 57,883 search steps. The invariant
and constant used eight tokens versus 16 tokens for the eight member points.

## Controls and limits

Constant expressions are rejected because they cannot distinguish negative
examples. A non-circular positive class produces no generalizing invariant.
The result is empirical and grammar-bounded; it does not assert that the found
encoding is unique. Implementation: `discover_invariant` and the `m2_tests`.
