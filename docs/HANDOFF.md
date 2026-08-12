# Handoff — B1–B3 ontogenesis milestones

**Date:** 2026-08-11

The active implementation now lives in:

- `src/transform.rs` — B1 within-program context abstraction.
- `src/recurrence.rs` — B2 cross-depth induction and executable equation compiler.
- `src/typed.rs` — B3 generic typed beta-normal proposal enumeration.
- `src/universal.rs` / `src/recursion_search.rs` — B2-general universal functional
  enumeration, fair fuel scheduling, behavior discovery, and extrapolation.
- `src/fixpoint.rs` — pure-lambda single and mutual fixed-point synthesis.
- `src/representation.rs` — anonymous sum-of-products representation invention from
  supplied constructor arities.
- `src/ontology_guidance.rs` / `examples/ontology_guided.rs` — measured developmental
  recursive-search bias with the universal dovetail preserved as fallback.
- `demo/arc-1/src/main.rs` — `b1`, `b2`, and `b3` experiments plus regression tests.
- `MILESTONES.md` — frozen claims, controls, evidence, and limitations.

Current measured path:

```text
B1  raw program → context abstraction → unseen widths       ✗ → 2
B2  raw p1,p2,p3 → recurrence → depths 5,7,9               ✗ → 7
B3  invented recursion → map/append/reverse
    map acquisition                                         ✗ → 15
    reverse acquisition                                     ✗ → 3
    map(reverse) → unseen 5×4 grid mirror                   ✓
B2-guided  {not}: recursive parity                          ≥122× proposals
           {not, parity}: nested recursive law              ≥230× proposals
```

The earlier substrate bug remains documented in git history and the B1 regression now
requires the raw winner to contain the real `cons` primitive.

Keep the two B2 routes distinct. The original B2 inducer handles exact first-order
structural right recurrences efficiently and compiles them for Church lists.
B2-general adds relative-semidecision enumeration of arbitrary representable closed
functionals, pure-lambda single and mutual fixed points, semantic recurrence matching,
and representations invented from supplied anonymous constructor arities. It does not
infer signatures from raw bytes, prove a unique latent representation, decide general
program equivalence, or eliminate finite machine and combinatorial limits.
