# M7 — Invent GCD as an Invariant

## Input without the target concept

The observations are trajectories under `(a,b)->(b,a mod b)`, such as
`(48,18)->(18,12)->(12,6)->(6,0)`. The system is given integer divisibility but
not greatest common divisor, divisibility invariant, or Euclidean algorithm.

## How the invariant emerged

For each state it constructs the set `{d : d divides a and d divides b}` and
searches simple scalar aggregations: cardinality, sum, minimum, and maximum.
Invariance alone is underdetermined because several summaries of an unchanged
set remain unchanged. The second criterion asks which scalar also equals the
procedure's terminal nonzero coordinate. Only the maximum satisfies both
requirements, yielding the executable description `max(common divisors)` and
the law `I(a,b)=I(b,a mod b)`.

## Evidence and reuse

The law is verified on three training trajectories and two held-out pairs,
including coprime input `(17,5)`. Reasoning across 14 raw states is compressed
to three concept applications, a measured gain of 11.

## Limits

Status is `bounded_verified`. The experiment explains the sampled procedure
with a discovered invariant, but does not include a kernel proof of the general
divisor-set equality. The search space of aggregations is deliberately small
and explicit.
