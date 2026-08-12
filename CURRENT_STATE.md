# Current State — Mathematical Ontogenesis

**Date:** 2026-08-12

Directions G and Mathematical Ontogenesis M1–M10 are implemented. M3–M9 grow
the arithmetic ontology from unary concepts through latent linear directions.
M10 crosses the earlier proof boundary with a small independently checked
proposition world.

## M10 result

From `forall n, (2|n iff 2|n²)`, answer-blind reformulation search discovers
`forall n, 2|(n²+n)`. Both implications and the alternative theorem are
accepted by an independent modular checker that exhausts the canonical residue
period. Direct proof cost falls from 37 to 14, syntax shrinks by two nodes, and
the same search transfers to `3|(n³-n)`. Forged-certificate and finite-sample
controls are rejected.

The status is `formally_checked_modular`, not general formal proof. The checker
is sound for the implemented integer-polynomial/divisibility fragment and its
implicitly universally closed integer variable.

## New boundary at M11

Euclid's proof needs first-class finite collections of primes, products over a
collection, existential witnesses for prime divisors, factorization lemmas,
contradiction, and auxiliary-object search. These cannot be represented in the
M10 modular fragment. Supplying `product(primes)+1` would leak M11's required
discovery, so the strict ladder stops before that experiment until a general,
answer-blind witness/object proposal layer is justified.

## Reproduce

```sh
cargo test -p supsearch --lib math_world
cargo test -p supsearch --lib proposition_world
cargo run --release --example math_world
cargo run --release --example proposition_world
cargo test --workspace
```
