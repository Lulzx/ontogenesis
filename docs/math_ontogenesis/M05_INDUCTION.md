# M5 — Invent Induction

## Failure that required a new abstraction

Finite evaluation could support the M4 conjecture but could not establish it
for arbitrary positive `n`. The supplied proof operations were assumptions,
substitution, equality rewriting, arithmetic normalization, and implication;
induction was not supplied as a named rule.

## What was searched and retained

The system compares three proof schemas: check samples, direct normalization,
and base plus successor closure. Sample checking is rejected as unbounded
evidence, while direct normalization cannot unfold an arbitrary finite sum.
The third candidate reduces the theorem to `P(1)` and `P(n)->P(n+1)`. For the
odd-sum law the base equality and successor polynomial identity normalize
successfully. This schema is sound relative to the representation of positive
naturals as the least successor-closed set containing 1.

## Transfer and cost

The acquired schema verifies corresponding base/step obligations for the
triangular-number identity and the sum-of-cubes identity. Rediscovering a
schema for three identities is accounted as 48 units; applying the acquired
two-obligation schema costs 6, gain 42.

## Precise status

The record says `proof_schema_verified`, not `formally_proved`. The arithmetic
identities and implementation guards are checked, but the repository lacks a
general proof-term kernel. M10 later adds a narrow, independently checked
modular proposition fragment without retroactively strengthening M5's claim.
