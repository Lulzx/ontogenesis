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
