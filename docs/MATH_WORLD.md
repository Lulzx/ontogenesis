# Direction M1: invent distance as a reusable mathematical concept

This is the first problem of the **Mathematical Ontogenesis** track. The
transition from the Ontogenesis directions (A–G) is:

```text
Current Ontogenesis                    Mathematical Ontogenesis
observations                           mathematical observations
→ provisional ontology                → invented representation
→ revision                            → invariant
→ causal concepts                     → reusable concept
→ persistent world model              → conjecture
→ cheaper future reasoning            → proof abstraction
                                       → cheaper theorem reasoning
```

The standing directive for this track:

> Do not add capabilities merely because they seem useful. Advance through the
> Mathematical Ontogenesis benchmark in order. For each problem, determine
> whether the existing system can solve it without architectural changes. If it
> fails, identify the minimal missing ontogenetic capability, implement only
> that capability, verify it on the failing problem and all earlier problems,
> then continue. Stop when progress requires an unjustified domain-specific
> primitive or when the next problem cannot be reached without fundamentally
> changing the research thesis.

## The world abstraction

Mathematics is treated as a world `W = (S, A, T, O)`:

```text
S = known expressions / concepts (the ontology)
A = admissible operations (+, -, *, /, sqrt, composition)
T = the derivation relation (evaluation of an expression on a point)
O = observations ((x, y) -> d pairs)
```

This is the same shape as the persistent world of Direction G, but the "state"
is now a set of known expressions and the "transition" is a valid mathematical
operation. A mathematical domain fits naturally:

```text
state       = known expressions / propositions / concepts
action      = transform / evaluate / prove / construct
transition  = valid mathematical operation
observation = resulting equality / value / theorem / counterexample
```

## Problem 1: Invent Distance

**Given** Pythagorean-triple observations:

```text
(3, 4)  -> 5
(5, 12) -> 13
(8, 15) -> 17
(7, 24) -> 25
```

**Primitive language**: variables `x, y`; constants; `+ - * / sqrt`;
composition. The concept of Euclidean distance is **not** supplied.

**Task**: invent an expression that explains the observations and generalizes
to unseen examples.

**Desired discovery**: something equivalent to `sqrt(x*x + y*y)`.

## Algorithm

1. **Enumerate** arithmetic expressions bottom-up by size over the primitive
   language, deduplicating by behavior on the training points (two expressions
   that produce the same outputs on all training points are the same).
2. **Verify** each candidate against the training observations (within epsilon).
3. **Generalize**: the first (smallest) fitting expression is checked against
   held-out points.
4. **Reuse**: the discovered expression is registered as a reusable concept;
   predicting a new point's distance is then a single evaluation.

## Observed result

```text
ontogenesis: mathematical ontogenesis (M1)
world: arithmetic expressions over x,y with + - * / sqrt
training observations=4 held-out=4
discovered expression: sqrt(((x*x)+(y*y))) (size 8, discovery_cost 99573)
generalizes to held-out: true
transfer: concept_reasoning_cost=4 baseline_reasoning_cost=99577 saving=99573
compression: raw_observations=8 raw_tokens=24 concept_tokens=8 gain=16
experiment=math_world_m1,discovered=sqrt(((x*x)+(y*y))),size=8,discovery_cost=99573,generalizes=true,heldout=4,concept_reasoning_cost=4,baseline_reasoning_cost=99577,transfer_saving=99573,raw_observations=8,raw_tokens=24,concept_tokens=8,compression_gain=16,proof_status=empirical,deterministic=true,fallback=exact
```

The agent invents `sqrt(x*x + y*y)` — the Euclidean distance — from the four
Pythagorean-triple observations, and it generalizes to all four held-out points
(`(20,21)->29`, `(9,40)->41`, `(12,35)->37`, `(28,45)->53`). The concept is
**reusable**: predicting a new point's distance costs 1 evaluation
(`concept_reasoning_cost=4` for 4 points), versus re-synthesizing the
expression from scratch (`baseline_reasoning_cost=99577`), a saving of 99,573
expressions. The concept also compresses the observations: 24 raw tokens
(8 `(x,y,d)` triples) become an 8-node expression, a gain of 16 tokens.

## Controls and honest fine print

- **Generalization is checked, not assumed**: the discovered expression must
  fit all held-out points, not just the training points.
- **Non-generalizing control**: a world whose training points are *not*
  Pythagorean triples yields either no fit within the bound or a fit that does
  not generalize — the search honestly reports this rather than forcing a fit.
- **Behavior dedup**: the search collapses behaviorally equivalent expressions,
  so the discovery cost is in distinct behaviors, not raw syntax.
- **Deterministic**: the enumeration order and dedup are deterministic, and the
  machine record is reproducible (`deterministic=true`).
- **proof_status = empirical**: the expression fits the examples and generalizes
  to held-out points, but this is not a formal proof that it is the unique or
  intended concept.

## Claim and limits

Supported claim:

> In an arithmetic world `W = (S, A, T, O)`, an agent that observes
> `(x, y) -> d` pairs can invent a reusable expression (the Euclidean distance)
> that explains the observations, generalizes to unseen points, and makes later
> reasoning measurably cheaper than re-synthesizing it from scratch.

Limits: the primitive language (variables, constants, `+ - * / sqrt`,
composition) is supplied; the search is bounded by expression size 8; the
domain is two-variable real arithmetic; no claim of general symbolic
regression, higher-order concepts, or formal proof. The discovered concept is
not claimed to be the unique latent encoding.

## Reproduce

```sh
cargo test -p supsearch --lib math_world
cargo run --release --example math_world
cargo test --workspace
```

The example ends with
`experiment=math_world_m1,...deterministic=true,fallback=exact` machine output.

---

# Direction M2: invent the circle invariant

## Problem 2: Invent the Circle Invariant

**Given** sets of points that all belong to the same hidden class (a circle of
unknown radius), plus other points that do not:

```text
members:      (3, 4), (4, 3), (-3, 4), (0, 5)
non-members:  (1, 1), (2, 2), (5, 5), (1, 3)
```

**Primitive language**: the concepts acquired in Problem 1 (the base arithmetic
language) plus basic arithmetic and equality.

**Task**: discover the simplest property distinguishing members from
non-members.

**Desired discovery**: something equivalent to `x² + y² = constant` or
`distance(x, y) = constant`.

**Restriction**: the concepts of circle, radius, and origin are **not**
supplied.

## Algorithm

1. **Enumerate** candidate expressions `f` by size over the base arithmetic
   language, deduplicating by behavior on the member + non-member points.
2. **Invariant check**: for each `f`, if all members share a common value `c`
   (within epsilon) and all non-members differ from `c`, then `f(x,y) = c` is a
   candidate invariant.
3. **Generalize**: the simplest such invariant is checked against held-out
   members and non-members.
4. **Reuse**: the invariant compresses the entire class — classifying a new
   point is a single evaluation and comparison.

## Observed result

```text
ontogenesis: mathematical ontogenesis (M2)
world: points on a hidden circle; members=4 non-members=4 held-members=4 held-non-members=4
discovered invariant: ((x*x)+(y*y)) = 25 (size 7, discovery_cost 57883)
generalizes to held-out: true
transfer: concept_reasoning_cost=8 baseline_reasoning_cost=57891 saving=57883
compression: raw_points=8 raw_tokens=16 concept_tokens=8 gain=8
experiment=math_world_m2,invariant=((x*x)+(y*y)),constant=25,size=7,discovery_cost=57883,generalizes=true,members=4,non_members=4,held_members=4,held_non_members=4,concept_reasoning_cost=8,baseline_reasoning_cost=57891,transfer_saving=57883,raw_points=8,raw_tokens=16,concept_tokens=8,compression_gain=8,proof_status=empirical,deterministic=true,fallback=exact
```

The agent invents the invariant `x² + y² = 25` — the circle of radius 5 — from
the four member points, and it generalizes to all held-out members and
non-members. The invariant is **reusable**: classifying a held-out point costs
1 evaluation (`concept_reasoning_cost=8` for 8 points), versus re-discovering
the invariant from scratch (`baseline_reasoning_cost=57,891`), a saving of
57,883 expressions. The invariant also compresses the class: 16 raw tokens
(8 `(x,y)` points) become a 7-node expression plus a constant (8 tokens), a
gain of 8 tokens.

## Controls and honest fine print

- **Generalization is checked, not assumed**: the invariant must hold on all
  held-out members and fail on all held-out non-members.
- **Non-circular control**: a class of points that do *not* lie on a common
  circle yields either no invariant within the bound or one that does not
  generalize — the search honestly reports this rather than forcing a fit.
- **Trivial-constant rejection**: an expression constant on *all* points cannot
  distinguish members from non-members, so it is rejected by the non-member
  check.
- **Deterministic**: the enumeration order and dedup are deterministic, and
  the machine record is reproducible (`deterministic=true`).
- **proof_status = empirical**: the invariant fits the examples and generalizes
  to held-out points, but this is not a formal proof that it is the unique or
  intended invariant.

## Claim and limits

Supported claim:

> In an arithmetic world, an agent that observes member and non-member points
> can invent a persistent invariant (`x² + y² = c`) that compresses the entire
> class and makes classifying new points measurably cheaper than re-discovering
> the invariant from scratch.

Limits: the primitive language is supplied; the search is bounded by expression
size 7; the domain is two-variable real arithmetic; no claim of general
invariant discovery, higher-order concepts, or formal proof. The discovered
invariant is not claimed to be the unique latent encoding.

## Reproduce

```sh
cargo test -p supsearch --lib math_world
cargo run --release --example math_world
cargo test --workspace
```

The example ends with
`experiment=math_world_m2,...deterministic=true,fallback=exact` machine output.

---

# Directions M3–M9: arithmetic laws to latent linear coordinates

The ladder was continued strictly in order. Each stage searches a bounded,
explicit candidate space, checks held-out transfer, records reasoning and
description costs, and retains the result only when it reduces downstream
cost. The implementation and machine records live in `src/math_world.rs` and
`examples/math_world.rs`.

| Stage | Invented object | Evidence | Honest status |
|---|---|---|---|
| M3 | `square(n) = n*n` from five input/output pairs | exact on three held-out values; five square occurrences cost 10 ontology tokens vs 15 expanded tokens | empirical |
| M4 | `sum(k=1..n, k+(k-1)) = square(n)` | theorem generated before proof and checked for `n=6..20`; corrupted-prefix control has no generalizing law | conjectured |
| M5 | base case + successor closure proof schema | verifies the odd-sum base/step and transfers to triangular and cube-sum identities | proof schema verified; no general proof kernel claimed |
| M6 | `1/(k(k+1)) = 1/k - 1/(k+1)` and boundary cancellation | exact rational checks and transfer to offsets 2 and 3; 2 boundary terms vs 12 raw terms | identity verified |
| M7 | `max({d : d divides a and b})` invariant | unchanged over three Euclidean trajectories, equals their terminal result, and transfers to two held-out trajectories | bounded verified |
| M8 | rational formal-series object `F(x)=(1+0x)/(1-x-x²)` | recurrence and numerator derived from observations; predicts 21, 34, 55 | formal-series verified |
| M9 | latent directions satisfying `A(v)=scale*v` | hidden matrix inferred; two primitive directions give exact 10-step predictions and transfer to two unseen transforms | bounded verified |

These stages add only the minimal representation needed by the failing next
problem. In particular, `square` is unavailable during M3 search and becomes
an ontology token only afterward; M6 searches reciprocal decompositions
without a telescoping primitive; M7 constructs common-divisor sets from the
divisibility predicate without a GCD primitive; and M9 searches scaled images
without supplying eigenvalue/eigenvector terminology.

## Boundary after M9

M10 requires candidate propositions and machine-checked proofs of both
`P -> Q` and `Q -> P`, then proof-cost comparison. The repository currently has
evaluators, bounded expression synthesis, and a verified proof-schema
experiment, but it has no proposition language, proof terms, or trusted proof
kernel. Adding those is a fundamental architecture change, not the minimal
extension of the arithmetic world. Hard-coding parity lemmas would also make
M10 nominally pass while violating its discovery restriction.

Accordingly the ladder stops at the first proven thesis boundary. M10–M30 are
not claimed. See `CURRENT_STATE.md` and `NEXT_BOUNDARY.md` for the evidence and
the requirements for a scientifically valid continuation.
