# M4 — Invent the Odd-Sum Law

## Input and objective

The observations were the prefix totals `1,4,9,16,25`, shown as growing sums.
The general formula was not supplied. The objective was to generate a theorem
before being asked for a proof.

## What the system inferred

`discover_odd_sum_law` first differences adjacent totals to expose term
observations `1,3,5,7,9`. It searches the unary grammar for both a term
generator and a total expression, now permitting the M3-acquired square token.
At combined discovery cost 79 it produces
`sum(k=1..n, k+(k-1)) = square(n)`, equivalent to the desired odd-sum law.

## Evidence and savings

The conjecture is evaluated independently for every `n` from 6 through 20.
It replaces 15 explicit-term reasoning units with five concept evaluations and
compresses the representation by 11 tokens. A corrupted final prefix (`26`
instead of `25`) either yields no bounded law or one that fails held-out
generalization.

## Claim boundary

Machine status is `conjectured`, not proved. M4 demonstrates autonomous theorem
generation and transfer. The proof obligation is deliberately passed to M5.
