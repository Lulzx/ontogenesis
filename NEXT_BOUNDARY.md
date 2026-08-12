# Next Boundary — M14 Transformation Symmetry

**Pre-registration date:** 2026-08-12

**Integrity contract:** `SCIENTIFIC_INTEGRITY.md`

**Status:** frozen before M14 implementation; executed without amendment

## Recorded outcome

The frozen experiment constructed `input * -1`, its two-sided inverse, and
independent identity/sign-reversing output responses. Exact checks passed and
aggregate downstream proposal checks fell 123 to 22. However, two frozen
nonconstant controls showed negative transfer: the linear form required 8
baseline checks versus 9 with the retained action, and the cyclic coordinate
required 7 versus 9. The pre-registered L3 condition required improvement on
every family, so `l3_boundary_passed=false` and the honest claim is L2. No
grammar, enumeration order, dataset, or threshold was changed after observing
this outcome.

## Intended claim

M14 asks whether a reusable transformation can be constructed from generic
arithmetic programs, retained because multiple observables respond simply to
it, and transferred across frozen domains. “Even,” “odd,” “symmetry,”
“reflection,” and “negation transformation” are not primitives or candidate
labels.

Maximum claim: `L3_transferred_ontology_with_measured_utility`, conditional on
exact validation and lower actual proposal counts on every frozen downstream
family. Otherwise report L2 or lower.

## Frozen substrate

- Inputs are finite integer coordinate vectors. Coordinates remain addressable;
  no quotient or invariant representation is supplied.
- A scalar program has atoms `coordinate_value`, `-1`, `0`, `1` and operations
  `+`, `-`, `*`.
- A transformation maps the same scalar program pointwise over every input
  coordinate. Programs are enumerated by AST size through size 3, normalized
  as exact univariate integer polynomials, and deduplicated semantically.
- An output-response program uses the same scalar grammar on `observed_output`
  through size 3. It is not restricted to identity or negation.
- Transformation/response pairs are enumerated deterministically by total AST
  cost, then transformation normal form, then response normal form.
- Identity and constant transformations are controls and cannot be retained as
  an invented action. A retained transformation must move a declared probe and
  be bijective with a compositionally discovered inverse in the same grammar.

## Frozen discovery tasks

Two one-variable integer-polynomial observables are used jointly:

- `x*x`;
- `x*x*x`.

The retained transformation must admit a checked output-response program for
each observable. The responses are not required to be the same. Candidate
ranking minimizes total checked description cost across both tasks. Training
samples are `[-3,-2,-1,0,1,2,3]`; they filter candidates but do not prove laws.

## Independent checking

- Polynomial tasks are normalized exactly after symbolic composition. A
  certificate is accepted only when `f(T(x)) = R(f(x))` as integer
  multivariable polynomials.
- Transformation inverse certificates are checked by exact composition in both
  orders.
- Fixed cyclic-group tasks are checked exhaustively on every group element;
  their status is explicitly bounded, not universal over all group orders.
- The checker receives submitted programs only. It does not expose normal
  forms, counterexamples, gradients, or error distinctions to proposal search.

## Frozen downstream tasks

These tasks are not used to rank or retain the transformation:

1. polynomial: `x^4 + x^2`;
2. geometry: squared norm `x^2+y^2`;
3. matrix: quadratic form `2x^2+2xy+3y^2`;
4. matrix/control: linear form `2x-3y`;
5. finite group: cyclic inverse-distance observable on `Z/7Z`;
6. finite group/control: the identity coordinate on `Z/7Z`.

For each task compare actual deterministic proposal checks until the first
valid response is found:

- baseline: enumerate every transformation/response pair;
- retained: freeze the discovered transformation and enumerate only responses.

Both conditions use identical response grammar, ordering, checker, and stopping
rule. Report per-task counts, negative transfer, and aggregate counts. No
candidate-count multiplication formula may be called measured gain.

## Controls and ablations

- constants: constant observables must not justify transformation retention;
- identity: the identity transformation must not count as invention;
- ordered/asymmetric polynomial `x^2+x` must reject the retained action;
- non-bijective transformations must fail inverse checking;
- irrelevant arithmetic primitives of matched size remain in enumeration;
- without the cubic discovery task, ambiguity among transformations must be
  reported rather than silently resolved;
- shuffled output pairs may fit samples but must fail symbolic validation;
- cyclic results are bounded controls and may not upgrade the universal claim.

## Supplied-ontology ledger

- **Representation supplied:** addressable integer coordinates and scalar
  observables.
- **Transformation meta-ontology supplied:** pointwise lifting of one generic
  scalar program; arbitrary coordinate permutations and nonlinear vector mixing
  are outside this boundary.
- **Primitives supplied:** constants and ring arithmetic.
- **Grammar supplied:** bounded arithmetic composition, but no named action,
  invariant, equivariant, parity, or sign-response constructor.
- **Objective supplied:** prefer a nontrivial invertible transformation under
  which several observables have short response programs. This explicitly
  supplies the scientific taste that transformation stability is valuable.
- **Checker supplied:** exact polynomial composition and bounded finite-group
  exhaustiveness.
- **Accounting:** actual checker invocations; wall time is not combined with
  proposal counts.

Closest supplied object to the hoped-for winner is the pointwise-program
lifting mechanism. Therefore even a successful result does not establish
invention of arbitrary group actions or symmetry as a fully domain-general
category.

## Stop/falsification conditions

M14 does not reach L3 if the winner is identity/constant, has no checked
inverse, depends on sample-only fit, fails any nonconstant frozen transfer
family, or does not reduce aggregate actual proposal checks. Any change to
this document after observing M14 output creates a separately versioned
experiment and must preserve this failed/partial run.

---

# Next Boundary — M14c Conditional Action–Response Acquisition

**Pre-registration date:** 2026-08-12

**Integrity contract:** `SCIENTIFIC_INTEGRITY.md`

**Status:** frozen before M14c implementation; executed without amendment

## Recorded outcome

M14c passed the frozen L3 gate. All six compatible unseen tasks were routed to
an acquired response and accelerated; all three incompatible controls were
declined and incurred exactly their baseline checker counts. There were zero
false-positive routes and zero negative-transfer tasks. Exact checker calls
fell 328→176, while the identical 54 probe evaluations in each condition remain
separately accounted. Two controls had no bounded baseline action-response
solution; exhaustive `no_solution` is preserved rather than replacing those
tasks. Claim level is `L3_transferred_ontology_with_measured_utility`.

## Motivation and intended claim

M14 forced one acquired action on every task and exposed negative transfer.
M14c does not alter that result. It tests a new hypothesis: the retained object
should be the entire discovered schema
`(input transformation, identity response, sign-reversing response)` plus a
generic applicability test, rather than an unconditional action.

Maximum claim: `L3_transferred_ontology_with_measured_utility` if the schema
reduces actual checker proposals on every compatible frozen family, never
increases them on incompatible controls, and lowers the aggregate. This is not
an L4 claim: pointwise actions, intervention probing, and stability as an
objective remain supplied meta-ontology.

## Frozen acquired concepts

M14's outputs are frozen without modification:

- transformation: `input * -1`;
- inverse: `input * -1`;
- response program A: `output`;
- response program B: `output * -1`.

Their order is frozen as A then B. No new response is learned from M14c tasks.

## Frozen applicability policy

Every condition observes the same intervention probes before proposal search.
For exact polynomial/vector tasks the points are `(1,2)`, `(-1,-2)`, and
`(2,-1)`. For cyclic tasks the elements are `1`, `2`, and `3` modulo the group
order. Probe evaluations are reported separately and are identical in both
conditions.

The policy compares the transformed output at all probes with response A and
then response B applied to the original output:

- if A matches every probe, try acquired response A then B;
- else if B matches every probe, try B then A;
- otherwise skip the acquired schema and use baseline search immediately.

An acquired response is accepted only by the same exact symbolic or exhaustive
checker as M14. Probe agreement alone cannot establish a law. If both acquired
responses fail exact checking, baseline search resumes and all failed checker
calls count.

## Frozen new downstream suite

None of these tasks appeared in M14 discovery or evaluation:

1. polynomial: `x^6 - 2x^4 + 3x^2`;
2. polynomial: `x^5 - 2x^3 + x`;
3. geometry: `4x^2 + 9y^2`;
4. matrix/tensor: `x^3 + 2xy^2 - y^3`;
5. finite group: inverse distance on `Z/11Z`;
6. finite group: coordinate on `Z/11Z`;
7. incompatible polynomial control: `x^2 + x + 1`;
8. incompatible geometry control: `x^2 + 2x + y^2`;
9. incompatible finite-group control: indicator of element `1` on `Z/11Z`.

## Compared conditions and accounting

- **Baseline:** after receiving the common probes, enumerate every admissible
  transformation/response pair in the frozen M14 order.
- **Acquired:** after receiving the same probes, apply the frozen policy above;
  if inapplicable or if acquired exact checks fail, run the identical baseline.

Report per task common probe evaluations, exact checker calls in each
condition, selected/declined schema, exact response or fallback winner,
false-positive routing, and negative transfer.

Success requires acquired checker calls `<` baseline on each of tasks 1–6,
`==` baseline on tasks 7–9, zero false-positive routes, and lower aggregate
checker calls. Probe evaluations cannot be combined numerically with checker
calls and cannot be omitted from reporting.

## Supplied-ontology ledger

- **Supplied representation:** explicit task observables and pointwise input
  transformation.
- **Supplied meta-ontology:** intervention-response stability and conditional
  allocation are valuable; three fixed probes are available.
- **Acquired from M14:** one action, inverse, and two response programs.
- **Not supplied:** even/odd labels, degree parity, monomial-degree features,
  domain labels in the policy, or task-specific routing rules.
- **Checker:** unchanged exact polynomial equality and bounded cyclic
  exhaustion; no checker errors flow into applicability.
- **Accounting:** exact checker invocations and common probe evaluations are
  distinct labeled units.

## Controls and falsification

M14c fails L3 if any compatible family is not accelerated, any incompatible
control is routed to the schema, any acquired response passes probes but fails
exact checking, any task has higher checker count, or aggregate checker count
does not fall. The already-observed M14 tasks may be reported only as historical
context and cannot be substituted into this suite.
