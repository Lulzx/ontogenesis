# Handoff — Mathematical Ontogenesis track (M1–M11 complete; M12 boundary)

**Date:** 2026-08-12

> **Superseding update:** M3–M11 are now complete. M11 added auxiliary-object
> search and a checked finite-list prime witness; the ladder now stops at M12's
> rational lowest-terms contradiction boundary. Read `/CURRENT_STATE.md` and
> `/NEXT_BOUNDARY.md` before continuing. The earlier M3 suggestion below is
> retained only as historical context.

This handoff originally documented the state after M2. It is retained to show
the exact starting point and standing directive used by the M3–M9 continuation.

## Where the work stands

The Ontogenesis directions A–G are complete and pushed. The Mathematical
Ontogenesis track (per `/Users/lulzx/Downloads/Ontogenesis Mathematical
Discovery Ladder.md`) has completed **M1** and **M2**. All work is committed
and pushed to `origin/main`.

```text
git log --oneline -6
de3a8f9 math_world: invent the circle invariant (M2)
7df2cd3 math_world: invent distance as a reusable mathematical concept (M1)
0e3d7b8 world_model: invent factored state compression and reversible-counter transfer in a persistent deterministic world
6d17c93 causal_ontology: infer executable causal structure from interventions
71c418e active_experimentation: crucial experiment over passive observation
06e63be probe_invention: invent distinguishing observational probes
```

Working tree is clean.

## The standing directive for this track

> Do not add capabilities merely because they seem useful. Advance through the
> Mathematical Ontogenesis benchmark in order. For each problem, determine
> whether the existing system can solve it without architectural changes. If it
> fails, identify the minimal missing ontogenetic capability, implement only
> that capability, verify it on the failing problem and all earlier problems,
> then continue. Stop when progress requires an unjustified domain-specific
> primitive or when the next problem cannot be reached without fundamentally
> changing the research thesis.

## The world abstraction (architectural decision)

Mathematics is treated as a world `W = (S, A, T, O)`:

```text
S = known expressions / concepts (the ontology)
A = admissible operations (+, -, *, /, sqrt, composition)
T = the derivation relation (evaluation of an expression on a point)
O = observations ((x, y) -> d pairs)
```

This is the same shape as the Direction G persistent world, but the "state" is a
set of known expressions and the "transition" is a valid mathematical
operation. This design lets M3–M30 build on the same substrate without a
rewrite.

## What M1 and M2 proved

### M1 — invent distance

From four Pythagorean-triple observations `(x,y) -> d`, the agent invents
`sqrt(x*x + y*y)` (the Euclidean distance, size 8, discovery_cost 99,573),
which generalizes to all four held-out points. The concept is reusable:
predicting a new point's distance costs 1 evaluation
(`concept_reasoning_cost=4`) versus re-synthesizing from scratch
(`baseline_reasoning_cost=99,577`), a saving of 99,573 expressions. It also
compresses the observations (24 raw tokens -> 8 nodes, gain 16).

### M2 — invent the circle invariant

Given member points on a hidden circle and non-member points, the agent invents
the invariant `x² + y² = 25` (the circle of radius 5, size 7, discovery_cost
57,883), which generalizes to all held-out members and non-members. The
invariant is reusable: classifying a held-out point costs 1 evaluation
(`concept_reasoning_cost=8`) versus re-discovering the invariant from scratch
(`baseline_reasoning_cost=57,891`), a saving of 57,883 expressions. It also
compresses the class (16 raw tokens -> 8 tokens, gain 8).

## Code layout

- `src/math_world.rs` — the arithmetic expression grammar (`Expr`), bottom-up
  behavior-deduped enumeration (`build_table`), M1 (`discover_concept`,
  `transfer_report`, `compression_report`, `machine_record`) and M2
  (`discover_invariant`, `invariant_transfer`, `invariant_compression`,
  `machine_record_m2`).
- `examples/math_world.rs` — driver producing both M1 and M2 machine records.
- `docs/MATH_WORLD.md` — full protocol, observed results, controls, limits.
- `docs/MILESTONES.md`, `docs/README.md`, `docs/VERIFICATION.md` — updated.

## Reproduction

```sh
cargo test -p supsearch --lib math_world   # 10 tests (5 M1 + 5 M2)
cargo run --release --example math_world   # M1 + M2 machine records
cargo test --workspace                     # full suite (~163 lib + 37 arc1)
```

## Next step: M3 — invent square numbers

The next problem in the ladder is **M3: Invent Square Numbers**:

```text
1 -> 1
2 -> 4
3 -> 9
4 -> 16
5 -> 25
```

Primitive language: `+ - *`, constants, variables, composition. Do **not**
provide exponentiation or a `square` primitive. Task: infer the reusable
transformation governing the observations. Desired discovery: `square(x) =
x * x`. Transfer test: tasks involving `x² + y²`, `(n+1)² - n²`, sum of odd
integers. Success: the system uses the acquired square concept rather than
expanding multiplication from scratch each time.

**Suggested approach**: extend `src/math_world.rs` with an M3 section. The
existing `Expr` grammar already has `+ - *` and variables; add a unary variable
`n` (or reuse `x`). The search should discover `x * x` (size 3) as the simplest
expression fitting `1->1, 2->4, 3->9, 4->16, 5->25`. Then demonstrate reuse on
the transfer tasks (e.g., `x² + y²` using the acquired square concept, and the
odd-sum law `sum(2k-1) = n²`). Follow the M1/M2 pattern: module functions +
example + tests + docs + machine record + commit + push.

## Key constraints (from the standing directive)

- **Do not force positive outcomes** — negative controls are as valuable as
  positive results.
- Every experiment needs: deterministic, small, release-mode reproducible,
  machine-readable, well-tested, independently falsifiable.
- Each result must distinguish: discovered / supplied / inferred / verified /
  bounded / not claimed.
- Small coherent commits; never leave major working changes uncommitted.
- Stop conditions are strict — do not stop early. Only produce
  `CURRENT_STATE.md` / `NEXT_BOUNDARY.md` at a genuine proven stop condition
  (fundamental computational wall, missing theoretical substrate,
  agenda-changing ambiguity, external-data decision, or architecture rewrite).
- The directive explicitly says: "Do not ask for confirmation after every
  completed stage. Advance automatically to the next scientifically meaningful
  experiment."

## The full ladder (for orientation)

M1 distance → M2 circle invariant → M3 square numbers → M4 odd-sum law → M5
induction → M6 telescoping → M7 GCD invariant → M8 generating function → M9
eigenvectors → M10 equivalent statement → M11 Euclid's proof → M12 √2
contradiction → M13 Vieta's formulas → M14 symmetry → M15 Fourier → M16 toy
spectral theorem → M17 Euler product → M18 toy zeta → M19 functional equation →
M20 completed object → M21 critical symmetry locus → M22 hidden zeros → M23
RH-like conjecture → M24 equivalent formulation → M25 RH-making object → M26
real zeta completion → M27 critical line → M28 new RH equivalences → M29
RH-making object search → M30 Riemann Hypothesis.

The early problems (M1–M7) are about concept/invariant/law induction in
arithmetic. M5 (induction) and M6 (telescoping) introduce proof abstraction.
M8+ move toward generating functions, spectral objects, and eventually the
Riemann Hypothesis. The benchmark's purpose is to measure whether increasingly
difficult mathematical worlds cause the system to grow the ontology required to
understand them.
