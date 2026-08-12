# Current State — Mathematical Ontogenesis

**Date:** 2026-08-12

Directions G and Mathematical Ontogenesis M1–M11 are implemented. M10 added a
checked modular proposition fragment. M11 adds generic finite collections,
auxiliary-object enumeration, and a checked existential prime-witness schema.

## M11 result

From finite prime-list behavior, a fixed grammar over `product`, `sum`,
`length`, constants, and arithmetic discovers `(product(xs)+1)` at proposal 19.
The independent checker does not recognize that syntax. It symbolically proves
that the construction has constant nonzero remainder modulo every arbitrary
listed member and is greater than one; the trusted generic prime-divisor lemma
then supplies a prime witness outside the list. Hence every finite prime list
is incomplete and there are infinitely many primes.

Three unseen prime lists and three composite-divisor lists pass. A singleton
control rejects `product-1`, non-prime evidence cannot claim the theorem, and a
corrupted derivation is rejected. Reuse costs 12 reasoning units versus 31 with
rediscovery; training compression gain is six tokens. The exact status is
`formally_checked_finite_list_schema`.

## New boundary at M12

The square-root-of-two problem needs rational witnesses in lowest terms,
coprimality, parity/divisibility inference through a squared equality, and
proof by contradiction. These are not expressible in the M10 modular checker
or M11 finite-list witness schema. Encoding the classical parity chain directly
would violate M12's discovery restriction.

## Reproduce

```sh
cargo test -p supsearch --lib math_world
cargo test -p supsearch --lib proposition_world
cargo test -p supsearch --lib euclid_world
cargo run --release --example euclid_world
cargo test --workspace
```
