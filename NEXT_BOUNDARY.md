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

---

# Next Boundary — M15 Oscillatory Coordinates

**Pre-registration date:** 2026-08-12

**Integrity contract:** `SCIENTIFIC_INTEGRITY.md`

**Debt ledger:** `ONTOLOGICAL_DEBT.md`

**Status:** frozen before M15 implementation; executed without amendment

## Recorded outcome

M15 constructed exact oscillatory coordinate families from 32 recurrence atoms
without named frequencies or a supplied Fourier dictionary. All five frozen
transfers reconstruct exactly, repeated closed shift dynamics predict every
cyclic phase, cross-family composition is exact, and aggregate descriptions
fall from 72 stored samples to 36 integers. Impulse, ramp, and corrupted exact
controls are rejected; noisy data has nonzero squared error 2.

The L3/meta-transfer gate failed. M9-style closed-dynamics ordering improved two
tasks but harmed three, including cross-family composition. The identical
candidate set required 1,668,649 unguided checks versus 16,484,585 guided
checks. Therefore `l3_boundary_passed=false` and M15 is reported as
`L2_invented_feature_in_supplied_meta_ontology`. No threshold, task, ordering,
or candidate space was changed after observing this result. One implementation
defect in gcd normalization of zero-leading sequences was corrected; it restored
the pre-audited 32-atom space and did not change the frozen protocol.

## Intended claim

M15 asks whether generic sequence generators can produce coordinate families
in which periodic signals have sparse descriptions and cyclic shift has simple
dynamics. “Fourier,” “frequency,” “sine,” “cosine,” complex exponential,
orthogonality, and projection are absent from proposal generation.

Maximum claim is `L3_transferred_ontology_with_measured_utility`. This boundary
cannot reach L4 because second-order recurrence generation and the preference
for simple shift dynamics are supplied meta-ontology.

## Frozen substrate and enumeration

- Signals are exact integer sequences of length 12.
- A raw coordinate generator is a second-order recurrence
  `u[n+2] = p*u[n+1] + q*u[n]`, with `p,q` enumerated in `[-2,2]` and initial
  values `(u[0],u[1])` in `[-2,2]^2 \ {(0,0)}`.
- Generated sequences must satisfy the recurrence around the cyclic boundary,
  be nonzero, and be normalized by their integer gcd and first nonzero sign.
- Extensionally equal sequences are deduplicated. No period or frequency label
  is available to search.
- Coordinate families are pairs of generated sequences with the same recurrence
  coefficients and independent initial states. Search enumerates recurrence
  coefficients, seed pairs, and integer reconstruction weights in the frozen
  order `0,1,-1,2,-2,3,-3`.
- Generic raw fitting enumerates all deduplicated coordinate atoms and weights;
  it does not receive recurrence-family grouping.
- Ordinary candidates contain up to two distinct atoms. The declared
  cross-family composition task contains exactly two such supports (up to four
  distinct atoms); both conditions enumerate the same support and weight space.
- The M9-retained idea may reorder complete enumeration by first considering
  coordinate pairs whose one-step cyclic shift is represented by a small exact
  2×2 integer matrix. It may not add or remove candidates. Applicability is
  endogenous: if no exact closed shift matrix exists, the ordering is unchanged.

## Frozen discovery signals

The following length-12 signals are generated before search and used only as
integer arrays:

1. `[2,1,-1,-2,-1,1]` repeated twice;
2. `[0,1,1,0,-1,-1]` repeated twice;
3. their coefficient mixture `2*s1 - s2`;
4. `[1,0,-1,0]` repeated three times;
5. `[0,1,0,-1]` repeated three times;
6. their coefficient mixture `s4 + 2*s5`.

The recurrence coefficients, coordinate families, periods, and generating
interpretation are not passed to proposal search.

## Exact validation

- Reconstruction must equal every signal sample exactly.
- A family certificate must independently recompute a 2×2 integer matrix that
  maps both coordinate sequences to their one-step cyclic shifts.
- Prediction holds only if repeated matrix action reconstructs every held-out
  cyclic shift exactly.
- Composition is checked by adding two signals in coordinate weights and
  reconstructing their pointwise sum exactly.
- Corrupted recurrences, dependent coordinate pairs, incomplete
  reconstructions, and finite-prefix-only fits must be rejected.

## Frozen transfer and controls

Transfer signals, unseen during family retention:

1. phase shift by one of discovery signal 3;
2. amplitude scaling `-2` of discovery signal 6;
3. cross-family composition `signal3 + signal6` using two retained families;
4. new coefficient mixture `-signal1 + 3*signal2`;
5. new coefficient mixture `3*signal4 - signal5`.

Controls:

- constant signal `[3;12]` must not justify an oscillatory family;
- impulse `[1,0,...,0]` must not have an exact sparse two-coordinate fit;
- ramp `[0,1,...,11]` must not pass cyclic recurrence closure;
- one-sample corruption of a discovery mixture must fail exact reconstruction;
- additive-noise versions are evaluated with explicit squared error and cannot
  be reported as exact laws.

## Compared conditions and measured units

For every frozen transfer task compare actual candidate reconstruction checks:

- **unguided:** complete raw atom/weight enumeration in frozen order;
- **M9-guided:** the identical candidate set, with exact closed-shift coordinate
  families tried first; if the retained idea is inapplicable, fall back to the
  unchanged unguided stream.

Report proposal checks per task, exact reconstruction error, prediction checks,
composition checks, false-positive controls, negative transfer, and aggregate
counts. Success requires exact recovery of all five transfer tasks, strictly
fewer M9-guided proposal checks on at least four, no task with more checks, and
zero exact false positives on impulse, ramp, and corrupted controls.

## Baselines

- Time-domain storage cost is 12 integers per signal.
- Generic fitting cost is the actual unguided proposal count and the number of
  stored sample integers.
- Coordinate description cost counts recurrence parameters, two seeds, and
  signal weights; it must be lower in aggregate than time-domain storage.
- Noise error is reported as exact integer squared error; no floating tolerance
  or post-hoc threshold is allowed.

## Supplied-ontology ledger

- **Supplied:** length-12 cyclic indexing, integer arithmetic, bounded
  second-order recurrence execution, sparse-description and simple-dynamics
  objectives, exact equality checker.
- **Acquired and optionally transferred:** M9 relation that useful coordinates
  may have simple closed dynamics.
- **Not supplied:** oscillatory atoms, named frequencies, trigonometry, Fourier
  coefficients, basis orthogonality, projection formulas, phase/amplitude
  operators, or the mapping from signals to recurrence parameters.
- **Checker information flow:** boolean acceptance only; normal forms, matrix
  entries, residuals, and counterexamples do not return to proposal ranking.
- **Claim ceiling:** L3; recurrence generation is D4 debt.

## Falsification

M15 fails L3 if candidate sets differ between guided and unguided conditions,
if a frozen transfer is not exact, if any control is called exact, if aggregate
coordinate descriptions do not beat storage, if guided search causes negative
transfer, or if measured candidate checks do not meet the frozen improvement
criterion. Any post-output change creates M15b and preserves this run.

---

# Next Boundary — M15b Conditional Coordinate Routing

**Pre-registration date:** 2026-08-12

**Integrity contract:** `SCIENTIFIC_INTEGRITY.md`

**Debt ledger:** `ONTOLOGICAL_DEBT.md`

**Status:** frozen before M15b implementation; executed without amendment

## Recorded outcome

M15b passed the frozen L3 gate. The raw six-sample probes routed the
closed-shift priority to nine compatible tasks and declined the other seven.
All eleven compatible tasks reconstruct exactly and every routed winner passes
the closed-shift prediction check; the nine routed tasks fall from 134,049 to
12,423 actual proposal checks, while the two declined compatible tasks and all
five controls keep their exact baseline counts. Aggregate actual proposal
checks fall 223,737→102,111 (measured gain 121,626). The 96 probe evaluations
per condition are separately labeled. False-positive routes and negative-
transfer tasks are both zero; impulse, ramp, corruption, and noisy controls
are declined and rejected as exact laws (noisy squared error 2), and the
constant control keeps its one-atom explanation without routing. Claim level is
`L3_transferred_ontology_with_measured_utility`. No probe, threshold, task,
ordering, or candidate space was changed after observing this outcome.

## Motivation and intended claim

M15 forced the M9-style closed-shift priority on every task and exposed
negative transfer: it improved two tasks, harmed three, and raised actual
proposal checks from 1,668,649 to 16,484,585. M15b does not alter that result.
It tests a new hypothesis: the retained object should be the closed-shift
coordinate-pair schema plus a generic raw-sample applicability test, rather
than an unconditional ordering priority.

Maximum claim: `L3_transferred_ontology_with_measured_utility` if the schema
reduces actual proposal checks on every routed compatible family, never
increases checks on any task, declines every incompatible control, and lowers
the aggregate. This is not an L4 claim: recurrence generation, the
closed-shift objective, and the probe mechanism remain supplied meta-ontology.

## Frozen acquired concepts

M15's outputs are frozen without modification:

- the 32 deduplicated recurrence atoms;
- the 18,048 candidate reconstructions in the frozen unguided order;
- the identical candidate set in the closed-shift-first guided order;
- the 52 deduplicated coordinate pairs whose one-step cyclic shift has an
  exact checked 2×2 integer matrix.

No new coordinate atom, recurrence coefficient, weight, response program, or
family is learned from M15b tasks.

## Frozen applicability policy

Every condition observes the same raw probes before proposal search: samples
`0..=5` of the task signal (six scalar values). Probe evaluations are reported
separately and are identical in both conditions.

The policy compares the six samples against the frozen candidate substrate in
this order:

1. **single-atom consistency:** if some atom and weight in
   `[1,-1,2,-2,3,-3]` equals all six samples, decline the schema and use the
   unguided order immediately;
2. **closed-pair consistency:** otherwise, if some closed-shift pair and two
   weights in the same frozen weight set equals all six samples, route to the
   guided order;
3. otherwise decline the schema and use the unguided order immediately.

Probe agreement alone cannot establish a law. The exact checker remains the
only acceptance criterion and is identical to M15. Routing changes only the
enumeration order; both orders contain exactly the same candidate set, so a
routed task that fails exact checking still performs the exhaustive guided
scan with the same candidate content as the baseline.

## Frozen new downstream suite

None of these tasks appeared in M15 discovery or evaluation. Discovery signals
`s1`, `s2`, `s3`, `s4`, `s5`, `s6` below are generated only as integer arrays,
exactly as in M15:

1. `phase_shift_two`: two-step cyclic shift of `s3`;
2. `phase_shift_three`: three-step cyclic shift of `s3`;
3. `phase_shift_four`: four-step cyclic shift of `s3`;
4. `phase_shift_five`: five-step cyclic shift of `s3`;
5. `period4_pair_2_3`: `2*s4 + 3*s5`;
6. `period4_pair_minus2_3`: `-2*s4 + 3*s5`;
7. `period4_pair_2_minus3`: `2*s4 - 3*s5`;
8. `period6_pair_2_3`: `2*s1 + 3*s2`;
9. `period6_pair_minus2_3`: `-2*s1 + 3*s2`;
10. `period6_nonclosed_3_minus2`: `3*s1 - 2*s2`;
11. `period6_single_atom_minus3_s6`: `-3*s6`;
12. `constant`: `[3;12]` control;
13. `impulse`: `[1,0,...,0]` control;
14. `ramp`: `[0,1,...,11]` control;
15. `corrupt_s3_sample3`: `s3` with sample 3 incremented by one;
16. `noisy_s3_samples2_5`: `s3` with sample 2 incremented by one and sample 5
    decremented by one.

Tasks 1–11 are compatible (they have exact sparse reconstructions in the frozen
candidate space). Tasks 12–16 are incompatible controls. The policy is not
given task names, domain labels, period labels, or any per-task rule.

## Compared conditions and accounting

- **Baseline:** after receiving the common probes, enumerate every candidate
  in the frozen unguided order.
- **Acquired:** after receiving the same probes, apply the frozen policy;
  declined tasks run the identical unguided stream, routed tasks run the
  identical guided stream.

Report per task: common probe evaluations, unguided checks, acquired checks,
route decision, exact winner, exact reconstruction error, and negative
transfer. Controls additionally report exact rejection and integer squared
error where noisy.

Success requires:

- exact recovery of every compatible task (error 0, and closed-shift
  prediction checked for every routed winner);
- strictly fewer acquired checks than baseline on every routed compatible
  task;
- acquired checks equal to baseline on every declined task;
- every incompatible control declined with no exact false-positive acceptance
  (noise is evaluated by explicit squared error, never as a law);
- zero false-positive routes and zero negative-transfer tasks;
- lower aggregate actual proposal checks;
- identical candidate sets in both conditions.

Probe evaluations cannot be combined numerically with checker calls and cannot
be omitted from reporting.

## Supplied-ontology ledger

- **Supplied:** the M15 substrate (length-12 cyclic indexing, integer
  arithmetic, bounded second-order recurrence execution, atom deduplication,
  frozen weights, exact equality checker) and six fixed raw sample positions
  per task.
- **Acquired from M15:** the closed-shift coordinate-pair schema and the
  guided ordering over the identical candidate set.
- **Supplied mechanism:** the probe/comparison route (D2) and the
  sparse-description/simple-dynamics objective (D3).
- **Not supplied:** named frequencies, trigonometry, Fourier coefficients,
  orthogonality, projection, period labels, target-shaped templates, per-task
  routing rules, or checker information flow beyond boolean acceptance.
- **Accounting:** actual checker invocations; common probe evaluations are a
  distinct labeled unit.

## Controls and falsification

M15b fails L3 if any compatible task is not exact, any routed task is not
accelerated, any declined task changes its checker count, any incompatible
control is routed or accepted as exact, aggregate checker calls do not fall,
or the candidate sets differ between conditions. The already-observed M15
tasks may be reported only as historical context and cannot be substituted
into this suite. Any post-output change creates M15c and preserves this run.

---

# Next Boundary — M16 Toy Spectral Regularity

**Pre-registration date:** 2026-08-12

**Integrity contract:** `SCIENTIFIC_INTEGRITY.md`

**Debt ledger:** `ONTOLOGICAL_DEBT.md`

**Status:** frozen before M16 implementation; executed without amendment

## Recorded outcome

M16 passed the frozen L3 gate. From unlabelled transition observations and a
generic scalar predicate grammar, the learner retained `a01-a10=0` at
predicate index 30 (size 3, one of 106,513 semantically unique programs). The
predicate separates all ten training matrices exactly; every admitted matrix
has a checked certificate with two orthogonal exact latent directions, and
every control has no certificate. On the nine frozen downstream tasks, all
five compatible matrices are admitted and accelerated (aggregate certificate
checks 109,039→83,182, measured gain 25,857), every rational spectral
decomposition checks exactly, and all four controls are declined with
identical exhaustive `no_solution` counts. Long-horizon operation cost falls
from 180 to 59 on every compatible task. False-positive routes and negative
transfer are both zero. Claim level is
`L3_transferred_ontology_with_measured_utility`. No predicate grammar, task,
threshold, ordering, or candidate space was changed after observing this
outcome.

## Motivation and intended claim

M9 infers small integer matrices from transitions and invents scaled latent
directions for individual transforms, but it does not discover why some
matrices admit two orthogonal latent directions and a checked spectral
decomposition. M16 asks the system to synthesize an unlabelled structural
predicate over matrix entries from transition observations, retain it only if
it separates matrices that admit two exact orthogonal latent directions from
matrices that do not, and use it to accelerate frozen long-horizon reasoning.
“Symmetric,” “orthogonal,” “eigenvalue,” “eigenvector,” and
“spectral decomposition” are not primitives or candidate labels.

Maximum claim: `L3_transferred_ontology_with_measured_utility`, conditional on
exact validation, a retained structural predicate, acceleration of every
frozen compatible family, and exact decline of every incompatible control.
This is not an L4 claim and is not a general real spectral theorem: the toy
domain is bounded integer matrices with exact integer/rational certificates.

## Frozen substrate

- Hidden objects are integer 2×2 matrices with entries in `[-3,3]`.
- Observations are one-step vector transitions on the fixed probe order
  `(1,0),(0,1),(1,1),(1,-1)`.
- Matrix inference enumerates entries `a00,a01,a10,a11` in `[-3,3]`
  lexicographically and accepts a task only when exactly one matrix matches
  every transition exactly.
- Direction universe: primitive vectors `(x,y)` with `x,y` in `[-3,3]`, not
  zero, `gcd(|x|,|y|)=1`, and first nonzero coordinate positive, ordered
  lexicographically.
- Scale universe: integers `-6..=6`, ordered ascending.
- A certificate is a pair of distinct directions plus two scales; it is valid
  exactly when both eigen equations hold, the direction dot product is zero,
  and every power `n=1..=8` satisfies `A^n d = s^n d` exactly for both
  directions.
- Long-horizon probes are horizon 10 on the two latent directions and the
  generic vector `(1,3)`.

## Frozen predicate language

Structural predicates are scalar programs over the four inferred matrix
entries, with constants `-2,-1,0,1,2` and operations `+,-,*`. A predicate is
true exactly when its integer value is zero. Programs are enumerated by AST
size through 5, then lexicographically, and deduplicated by exact behavior on
all `7^4=2401` matrices in `[-3,3]^4`. No symmetry, equality-of-entries,
orthogonality, or spectral constructor exists in the grammar.

## Frozen discovery task

The learner receives transitions for the following matrices in this order,
without family labels:

- compatible: `[[2,1],[1,2]]`, `[[1,2],[2,1]]`, `[[0,1],[1,0]]`,
  `[[3,2],[2,3]]`, `[[2,0],[0,3]]`, `[[1,0],[0,1]]`;
- incompatible: `[[1,1],[0,2]]`, `[[0,-1],[1,0]]`, `[[1,1],[0,1]]`,
  `[[2,1],[0,2]]`.

For each matrix the checker computes whether a valid certificate exists. The
retained predicate is the first semantically unique program whose true/false
split exactly matches the checker outcome on every training matrix. If no such
predicate exists, the boundary fails. The winner is expected to be equivalent
to `a01-a10=0`; the grammar may also expose many irrelevant and partially
separating predicates.

## Retained spectral object

For every admitted matrix, the checker finds a valid certificate and verifies
the exact rational decomposition
`A = (s1/(d1·d1)) d1 d1ᵀ + (s2/(d2·d2)) d2 d2ᵀ` entrywise. The predicate,
the certificate search order, and this decomposition checker are retained;
no new matrix or direction is learned from downstream tasks.

## Frozen downstream suite

None of these matrices appear in discovery:

1. `[[3,1],[1,3]]`;
2. `[[3,2],[2,0]]`;
3. `[[0,2],[2,3]]`;
4. `[[3,0],[0,1]]`;
5. `[[2,0],[0,2]]` (repeated eigenvalue, compatible);
6. `[[0,-1],[1,0]]` (complex-eigenvalue control);
7. `[[1,1],[0,1]]` (defective control);
8. `[[2,1],[0,2]]` (repeated defective control);
9. `[[1,1],[0,2]]` (two latent directions but non-orthogonal control).

## Compared conditions and accounting

Both conditions infer the same hidden matrix from the same transitions;
common inference checks are reported separately.

- **Baseline:** enumerate every direction-pair/scale certificate in canonical
  order until a valid certificate or exhaustive `no_solution`.
- **Acquired:** evaluate the retained predicate on the inferred matrix; if
  admitted, enumerate every orthogonal direction pair first (preserving
  baseline order inside each group), otherwise run the identical baseline.

Report per task: common inference checks, baseline certificate checks,
acquired certificate checks, route, exact winner or `no_solution`,
decomposition validity, and negative transfer. Long-horizon operation counts
use a deterministic interpreter in which multiplication and
addition/subtraction each cost one unit:

- baseline: `6` operations per matrix-vector step, so `60` per vector over
  horizon 10 and `180` for the three probe vectors;
- acquired directions: `H+2` operations per direction (power update plus
  scaling), so `2H+4` total;
- acquired generic vector: `9` operations for the fixed 2×2 coefficient solve,
  `2H` power updates, and `6` reconstruction operations, so `2H+15` total;
- acquired total over the three probes: `4H+19`, or `59` at horizon 10.

Success requires the retained predicate to separate training exactly, every
compatible downstream matrix to be admitted with strictly fewer acquired
certificate checks, exact decomposition and long-horizon predictions, every
control to be declined with identical `no_solution` counts, zero false-positive
routes, zero negative-transfer tasks, lower aggregate certificate checks, and
lower long-horizon operations on every compatible task.

## Supplied-ontology ledger

- **Supplied:** addressable matrix-entry observations after inference,
  transition probes, scalar arithmetic predicate grammar, bounded direction
  and scale universes, exact certificate checker, exact rational decomposition
  checker, and the objective that orthogonal-latent regularity is valuable.
- **Acquired:** the structural predicate and the orthogonal-first certificate
  ordering over the identical candidate set.
- **Not supplied:** symmetric/orthogonal/eigen labels, entry-equality
  constructors, spectral templates, target-shaped predicate shapes, or
  checker information flow beyond boolean acceptance.
- **Accounting:** actual checker invocations and deterministic interpreter
  operations are distinct labeled units.

## Controls and falsification

M16 fails L3 if no exact separating predicate is found, any compatible task is
not admitted or not accelerated, any control is admitted or accepted as a
valid certificate, aggregate checks do not fall, candidate sets differ between
conditions, or the retained decomposition fails on any compatible task. A
post-output change creates M16b and preserves this run.
