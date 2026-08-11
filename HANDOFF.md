# Handoff — B1–B3 ontogenesis milestones

**Date:** 2026-08-11

The active implementation now lives in:

- `src/transform.rs` — B1 within-program context abstraction.
- `src/recurrence.rs` — B2 cross-depth induction and executable equation compiler.
- `src/typed.rs` — B3 generic typed beta-normal proposal enumeration.
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
```

The earlier substrate bug remains documented in git history and the B1 regression now
requires the raw winner to contain the real `cons` primitive.

Do not broaden the current B2 claim: it induces exact first-order structural right
recurrences and compiles them for Church lists. Arbitrary fixed-point recursion,
unknown data representations, and mutual recursion remain future work.
