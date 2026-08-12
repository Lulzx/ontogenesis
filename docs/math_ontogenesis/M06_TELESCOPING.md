# M6 — Invent Telescoping

## Representation search

The input family has terms `1/(k(k+1))`. The grammar permits rational
arithmetic and rewriting but supplies neither partial fractions nor a
telescoping concept. The system enumerates small integer numerators `a,b` in
`a/k + b/(k+c)` and checks equality by exact cross multiplication.

## Discovery

For `c=1`, candidate 49 is `1/k - 1/(k+1)`. Written across a finite sum, every
interior denominator occurs once positively and once negatively, leaving only
the two boundary terms. Thus a 12-term evaluation collapses from 12 reasoning
units to 2 and gains 10 description units.

## Transfer and controls

The representation transfers to the held-out families
`2/(k(k+2))` and `3/(k(k+3))`, discovering the same reciprocal-difference
shape. The `c=0` control has no member of this candidate family, and is reported
as no discovery rather than forced cancellation.

## Status

`identity_verified` means the local rational identity is exact within the
integer cross-multiplication model. It is not a general-purpose symbolic sum
prover or a claim that every telescoping series can be found.
