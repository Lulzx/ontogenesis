# M11 — Rediscover Euclid's Proof

## Required discovery

The system must infer infinitude rather than finitude of primes and invent the
auxiliary construction `product(candidate primes)+1`. The construction must be
selected because it collapses the proof, not supplied as a template.

## Status: not reached

M11 depends on M10's missing proposition and proof-term world. It additionally
needs quantified finite sets, integer factorization/divisibility lemmas,
existential witnesses, contradiction, and search over auxiliary objects.
Current bounded arithmetic evaluators cannot certify “for every finite list of
primes there exists another prime.”

## What would count

A future experiment must generate the conjecture from factorization evidence,
enumerate candidate constructions without target leakage, produce a checker-
accepted proof that no listed prime divides the constructed integer, extract a
new prime divisor, and show that retaining the construction lowers proof search
on related “escape a finite list” arguments. No part of this is claimed today.
