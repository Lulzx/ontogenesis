# Current State — Mathematical Ontogenesis

**Date:** 2026-08-12

Directions G and Mathematical Ontogenesis M1–M9 are implemented. M3–M9 were
advanced in order from the prior handoff and verified together with M1–M2.

## Newly completed milestones

- M3 invents `n*n` without a square primitive and registers it as a cheaper
  reusable concept.
- M4 generates the odd-sum theorem from prefix observations before proof.
- M5 searches proof schemas and retains base plus successor closure because it
  proves the M4 identity and transfers to two recursive identities.
- M6 invents a reciprocal-difference representation whose intermediate terms
  cancel, with two held-out offset families.
- M7 invents maximum common divisor as an invariant of remainder transitions.
- M8 invents a rational formal-power-series object from a learned recurrence.
- M9 infers a hidden linear transform and scaled latent directions for cheap
  long-horizon prediction, with an honest rotation control.

All claims are bounded to the explicit grammars, examples, and controls in
`src/math_world.rs`. Machine records distinguish empirical, conjectured,
identity-verified, proof-schema-verified, formal-series-verified, and
bounded-verified results. No general theorem prover, uniqueness result, or
M10–M30 result is claimed.

## Genuine stop condition

M10 is the first task whose success criterion fundamentally exceeds the
current thesis implementation. It requires generating alternative predicates,
constructing trusted proofs in both directions, and measuring proof search.
There is no proposition AST, quantifier/predicate semantics, proof-term
language, or trusted proof checker in this repository. The M5 schema experiment
cannot certify arbitrary implication proofs.

Implementing a proof assistant layer would be an architecture change. Supplying
the parity equivalence and its lemmas directly would be domain-specific and
would not demonstrate ontogenesis. The standing directive therefore requires a
stop here.

## Reproduce

```sh
cargo test -p supsearch --lib math_world
cargo run --release --example math_world
cargo test --workspace
```
