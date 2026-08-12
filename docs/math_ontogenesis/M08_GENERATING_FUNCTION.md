# M8 — Invent a Generating Function

## Input and representation opportunity

The observed sequence is `1,1,2,3,5,8,13`. Formal power series operations are
allowed, but “generating function” and the desired rational object are not
supplied. The system must invent one object that encodes more than pointwise
coefficient prediction.

## Derivation

`discover_sequence_object` enumerates linear recurrences of orders 1–3 with
small integer coefficients. At cost 24 it finds `[1,1]`, meaning
`a_n=a_(n-1)+a_(n-2)`. It then derives—not looks up—the denominator by moving
recurrence terms into `Q(x)F(x)`: `Q=[1,-1,-1]`. Multiplying the observed
prefix by `Q` yields numerator `[1,0]`, hence the formal-series object
`F(x)=(1+0x)/(1-x-x²)` under this indexing convention.

## Transfer and compression

The object/recurrence predicts held-out coefficients `21,34,55` exactly.
Ten raw observed-plus-held coefficients are represented with five numerator
and denominator coefficients, gain 5.

## Status and limits

`formal_series_verified` records an exact finite recurrence derivation and
held-out coefficient test. Closed-form and asymptotic extraction are not yet
implemented, so M8 does not claim all four possible benefits listed by the
benchmark.
