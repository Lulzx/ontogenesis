# Current State — Mathematical Ontogenesis

**Date:** 2026-08-12

Directions G and Mathematical Ontogenesis M1–M12 are implemented. M10 added
checked modular propositions, M11 finite-list auxiliary-object proofs, and M12
integer-ratio assumptions plus checked contradiction.

## M12 result

From `p²=dq²`, a fixed factored search over integer measures and finite
quotients discovers prime-exponent count modulo 2 at candidate 14. For `d=2`,
the checker proves the prime-2 valuation must be both even and odd, discharges
the rational-witness assumption, and proves `sqrt(2)` irrational.

The schema transfers to six nonsquare radicands and refuses six perfect-square
controls. Direct reasoning falls 21→7; seven proof descriptions compress from
49 tokens to one schema plus witnesses (14 tokens), gain 35. Status is
`formally_checked_valuation_contradiction`.

## New boundary at M13

Polynomial root relations require unordered root-set semantics, multivariable
symbolic polynomial normalization, coefficient/root expression enumeration,
and controls for non-monic, repeated, negative, and complex roots. These are
not expressible in the current arithmetic and valuation fragments.

## Reproduce

```sh
cargo test -p supsearch --lib irrational_world
cargo run --release --example irrational_world
cargo test --workspace
```
