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

---

# Next Boundary — M17 Finite Euler Product

**Pre-registration date:** 2026-08-12

**Integrity contract:** `SCIENTIFIC_INTEGRITY.md`

**Debt ledger:** `ONTOLOGICAL_DEBT.md`

**Status:** frozen before M17 implementation; executed without amendment

## Recorded outcome

M17 passed the frozen L3 gate. From two extensional squarefree universes, the
learner inferred irreducible factors from raw multiplication behavior and
retained the local factor `1+r` at candidate index 8 of 2,934 semantically
unique grammar programs. The exact checker verifies that subset products of
`L(p)-1` reproduce the universe as a multiset and that `prod_p L(p)` equals the
global sum. All three frozen compatible universes are accepted and accelerated
(aggregate special-value operations 217→59, measured gain 158); all four
incompatible controls (missing composite, non-squarefree, duplicate, and
removed element) are declined with acquired operations equal to baseline.
False-positive acceptances and negative-transfer tasks are both zero. Claim
level is `L3_transferred_ontology_with_measured_utility`. No grammar, task,
threshold, ordering, or candidate space was changed after observing this
outcome.

## Motivation and intended claim

M17 asks whether, from an extensional finite arithmetic universe, the system
can infer primitive factors from multiplication behavior, construct local
factors in a generic arithmetic grammar, and retain the identity that the
product of those local factors expands to a globally defined special value,
accounting for every universe object exactly once. “Prime,” “irreducible,”
“local factor,” and “Euler product” are not primitives or candidate labels.

Maximum claim: `L3_transferred_ontology_with_measured_utility`, conditional on
exact validation on every frozen compatible universe, lower actual
special-value operations, exact decline of every incompatible control, and
zero negative transfer. This is not an L4 claim: the finite universe and the
exact checker are supplied meta-ontology.

## Frozen substrate

- A finite universe `U` is a sorted multiset of positive integers supplied
  extensionally. Generation metadata, primitive sets, and factorizations are
  not passed to proposal search.
- Compatible universes are exactly the squarefree products of a hidden set of
  primitive factors `P` (the factors themselves are not supplied). Training
  universes are `P=[2,3,5]` (8 elements) and `P=[2,3,5,7]` (16 elements).
- The global special value is the exact integer `S = sum_{u in U} u`.
- Primitive-factor inference: `n>1` is irreducible exactly when no pair
  `a,b` in `U` with `a>1`, `b>1`, and `a*b=n` exists. Inference performs one
  deterministic multiplication/equality check per pair examined and reports
  that count separately; it never receives the hidden factors.
- Local factors are scalar programs over one variable `r`, with constants
  `1,2,3`, atom `r`, operations `+,-,*`, and AST size through 5. Programs are
  enumerated by AST size, then deterministic construction order, and
  deduplicated by exact univariate integer polynomial normal form.
- The retained local factor is the first semantically unique program whose
  product identity validates on every training universe.

## Exact checker

For a candidate local factor `L` and a universe `U`, the checker:

1. recomputes the irreducible factors of `U` from raw multiplication
   behavior;
2. forms `a_p = L(p)-1` for every irreducible `p`;
3. expands subset products of the `a_p` values and requires the result to
   equal `U` as an exact multiset, so every global object is accounted for
   exactly once;
4. independently computes `prod_p L(p)` and requires it to equal `S`.

Pre-registration clarification made before the frozen run: the earlier wording
asked for subset products of `L(p)` directly, which no local factor can satisfy
while also matching `S`; the corrected checker is the Euler divisor-product
identity `prod_p L(p) = sum_{u in U} u` with terms `a_p = L(p)-1`. This
clarification is recorded here and no outcome was observed before it.

The checker returns boolean acceptance only. Normal forms, factors, partial
expansions, and counterexamples do not flow back into proposal ranking.

## Frozen downstream suite

None of these universes appear in training:

1. `P=[2,3,5,7,11]` (32 elements);
2. `P=[3,5,7,11,13]` (32 elements, no factor 2);
3. `P=[2,3,5,7,11,13,17]` (128 elements).

Controls, all incompatible:

1. `missing_composite`: `U={1,2,3,5}`;
2. `non_squarefree`: `U={1,...,12}`;
3. `duplicate`: the `P=[2,3,5]` universe with one copy of `6` duplicated;
4. `one_removed`: the `P=[2,3,5]` universe with `6` removed.

## Compared conditions and accounting

- **Baseline:** compute `S` by summing all universe elements; cost is
  `|U|-1` addition operations.
- **Acquired:** infer the irreducibles (reported separately), evaluate the
  retained local factor on each irreducible (one addition per factor), and
  multiply the local-factor values (`k-1` multiplications), for
  `2k-1` special-value operations. The exact checker then accepts or declines
  the identity; declined tasks fall back to the identical baseline sum and
  their failed product evaluation is reported as separate checker/probe work.

Report per task: universe cardinality, irreducible count, inference checks,
checker calls, baseline operations, acquired operations, accepted identity,
false-positive acceptance, and negative transfer. Probe/checker units are
never combined numerically with special-value operations.

Success requires:

- a retained local factor found from training alone;
- exact product identity on all three compatible downstream universes;
- strictly fewer acquired than baseline special-value operations on every
  compatible universe;
- every control declined with acquired operations equal to baseline;
- zero false-positive acceptances and zero negative-transfer tasks;
- lower aggregate special-value operations.

## Supplied-ontology ledger

- **Supplied:** extensional integer universe, integer multiplication and
  addition semantics, the bounded local-factor grammar, the exact expansion
  checker, and the objective that a product identity accounting for every
  object is valuable.
- **Acquired:** the irreducible classification, the local-factor program, and
  the product/special-value schema.
- **Not supplied:** prime lists, primality predicates, irreducible labels,
  Euler-factor templates, exponent bounds, or checker information beyond
  boolean acceptance.
- **Accounting:** actual inference checks, checker calls, and special-value
  operations are distinct labeled units.

## Controls and ablations

- `primes_supplied_ablation`: a stronger substrate that receives the hidden
  factors directly must pass, but this does not establish invention.
- `single_atom_grammar`: a weaker substrate restricted to single atoms must
  fail to find an exact local factor on the frozen training universes.
- Irrelevant constants and power atoms of matched syntactic cost remain in the
  enumeration and must not be retained.
- Shuffled universe order, duplicate elements, missing products, and
  non-squarefree objects must fail exact expansion.

M17 fails L3 if the retained local factor is not found, any compatible
universe is not exact or not accelerated, any control is accepted as exact,
aggregate special-value operations do not fall, or candidate accounting mixes
units. Any post-output change creates M17b and preserves this run.

---

# Next Boundary — M18 Toy Zeta Object

**Pre-registration date:** 2026-08-12

**Integrity contract:** `SCIENTIFIC_INTEGRITY.md`

**Debt ledger:** `ONTOLOGICAL_DEBT.md`

**Status:** frozen before M18 implementation; executed without amendment

## Recorded outcome

M18 passed the frozen L3 gate. From two extensional semigroup universes with
exponent cap 2, the learner inferred irreducibles and retained the local
factor `q^2+q+1` (candidate index 0, one of 16,806 coefficient vectors, the
only candidate valid on training). The exact checker verifies that
`prod_{p in P} (p^{2s}+p^s+1) = sum_{u in U} u^s` as exact integers, that the
completed factor `1-p^{-s}` has formal pole order `|P|` at `s=0`, and that the
reflection identity for `C(p,s)=p^{1-s}-p^s` holds at the rational center
`1/2`. All three frozen four-prime downstream universes are exact and
accelerated (aggregate operations 584→422, measured gain 162); all four
controls are declined with baseline counts preserved. False-positive
acceptances and negative-transfer tasks are both zero, and description cost
falls 249→21 stored integers. Claim level is
`L3_transferred_ontology_with_measured_utility`. No grammar, task, threshold,
ordering, or candidate space was changed after observing this outcome.

## Motivation and intended claim

M17 retained a squarefree local factor `1+r` from a generic arithmetic grammar.
M18 asks whether the same extensional universe substrate can produce a compact
exponent-parameterized object connecting a Dirichlet-like global sum,
multiplicative factorization, exact integer special values, and exact formal
poles/zeros. “Zeta,” “Euler product,” “pole,” and “zero” are not primitives or
candidate labels.

Maximum claim: `L3_transferred_ontology_with_measured_utility`, conditional on
exact validation and lower actual special-value operations on every frozen
compatible universe, exact decline of every control, and zero negative
transfer. This is a bounded toy result, not an analytic-continuation claim.

## Frozen substrate

- Hidden objects are finite sets `P` of distinct primes from `{2,3,5,7,11}`,
  with `2 <= |P| <= 4`. Only the extensional universe is observed.
- A universe is the sorted multiset
  `U(P,E) = {prod_{p in P} p^{e_p} : 0 <= e_p <= E}` with frozen `E = 2`.
  Generation metadata and factorizations are never passed to search.
- Special values are exact integers
  `S_s = sum_{u in U} u^s` for integer `s`; training exponents are
  `s = 1,2,3,4`. This integer form was fixed by pre-execution amendment so the
  exact toy remains closed under finite arithmetic; the object is a
  positive-power toy analog of the Dirichlet-like sum.
- Irreducible inference and exponent-cap inference use only multiplication
  and membership tests on the observed universe.
- A local factor is an integer-coefficient univariate polynomial in
  `q = p^s`, degree through 4, coefficients in `[-3,3]`, with the power
  evaluation `q = p^s` as a supplied generic primitive. Candidates are
  enumerated deterministically by coefficient vector (degree 0 through 4) and
  deduplicated by exact integer behavior on the training grid
  `(p in training irreducibles, s in training exponents)`. This coefficient
  grammar was fixed by pre-execution amendment as the tractable executable
  form of the frozen bounded local-factor grammar; it contains the retained
  object `q^2+q+1`.

## Exact checker

For a submitted local factor `L(q)`, the checker:

1. recomputes the irreducible factors and verifies `E = 2` from the raw
   universe (`p^2` present and `p^3` absent for every irreducible `p`);
2. recomputes every special value by direct exact integer summation;
3. evaluates `L(p,s)` exactly and requires
   `prod_{p in P} L(p,s) = S_s` for every frozen exponent;
4. verifies the formal completed factor `C(p,s)=p^{1-s}-p^s` satisfies
   `C(p,1/2)=0` and `C(p,1-s)=-C(p,s)` exactly for every irreducible, with
   `1/2` treated as the rational center of the toy reflection;
5. verifies the formal pole certificate: for the completed factor
   `1-p^{-s}`, the identity `1-p^0=0` holds for every irreducible, so the
   completed infinite Euler object has pole order `|P|` at `s=0`. This
   certificate is a fixed algebraic check on the retained completion, not a
   condition on the submitted finite local factor's denominator.

The checker returns boolean acceptance only; no normal form, denominator,
counterexample, or error distinction flows back into proposal ranking. The
pole/zero certificates are formal exact algebraic facts about the retained
object, not empirical evaluations at undefined points.

## Frozen downstream suite

Training universes are `P=[2,3]` and `P=[2,3,5]` with exponents `s=1,2,3,4`.
None of the downstream tasks appear in training:

1. `P=[2,3,5,7]`, exponents `s=5,6`;
2. `P=[3,5,7,11]`, exponents `s=5,6`;
3. `P=[2,5,7,11]`, exponents `s=5,6`.

All downstream universes have `k=4` irreducibles and `|U|=81`, so direct
summation is expensive relative to the retained product schema. This was fixed
before any implementation or output was observed.

Controls, all incompatible:

1. corrupted special value: one `S_s` numerator incremented by one;
2. missing element: `U(P,E)` with `4` removed;
3. extra element: `U(P,E)` with `8` inserted (not in the semigroup);
4. `s=0`: the raw sum is defined (`|U|`), but the retained schema must decline
   because its formal pole certificate makes evaluation at `s=0` undefined;
5. single-factor candidates `1+q` and `1-q` must fail exact
   validation on training.

## Compared conditions and accounting

- **Baseline:** store and evaluate the raw universe and special values. Cost
  is `|U|-1` exact integer additions per exponent plus `|U|` stored integers
  and one integer per special value.
- **Acquired:** infer `P` and `E` (reported separately), evaluate `k` local
  factors by Horner form (`degree` multiplications and `degree` additions),
  multiply them (`k-1` integer multiplications), and store `k` irreducibles
  plus the coefficient vector. `pow(p,s)` costs `s-1` multiplications and is
  reported separately.
- Description cost is the number of stored integers: raw stores `|U|` values
  plus one integer per special value; acquired stores `k` irreducibles, the
  exponent cap, and the coefficient vector length.

Report per task: universe cardinality, irreducible count, inference checks,
checker calls, baseline operations, acquired operations, exact equality,
formal pole order, false-positive acceptance, and negative transfer. Success
requires exact recovery of every compatible task, strictly fewer acquired
operations than baseline on every compatible task, exact decline of every
control with unchanged baseline counts, zero false-positive acceptances, zero
negative-transfer tasks, lower aggregate operations, and lower aggregate
description cost.

## Supplied-ontology ledger

- **Supplied:** extensional universe, exact integer arithmetic, the `pow`
  evaluation primitive, the bounded local-factor grammar, and the exact
  checker including the frozen rational center `1/2`.
- **Acquired:** irreducible/exponent inference and the retained local-factor
  program composing global sum, factorization, special values, and formal
  pole/zero certificates.
- **Not supplied:** prime lists, factorization metadata, zeta names, Euler
  templates, pole/zero labels, or checker information beyond boolean
  acceptance.
- **Accounting:** operations, stored integers, inference checks, and checker
  calls are distinct labeled units.

## Controls and ablations

- `single_atom_grammar`: a weaker substrate restricted to degree-0 and
  degree-1 coefficient vectors cannot produce a valid local factor.
- `primes_supplied_ablation`: a stronger substrate that receives `P` directly
  passes but does not establish inference.
- Corrupted, missing, extra, and `s=0` observations must be declined.
- The formal pole order must equal `|P|` and the reflection center must be
  exactly `1/2`.

M18 fails L3 if no local factor is retained from training, any compatible
universe is not exact or not accelerated, any control is accepted, aggregate
operations or description cost do not fall, or accounting mixes units. Any
post-output change creates M18b and preserves this run.

---

# Next Boundary — M19 Toy Functional Equation

**Pre-registration date:** 2026-08-12

**Integrity contract:** `SCIENTIFIC_INTEGRITY.md`

**Debt ledger:** `ONTOLOGICAL_DEBT.md`

**Status:** frozen before M19 implementation; executed without amendment

## Recorded outcome

M19 passed the frozen L3 gate. From exact values of the M18 completed object
in two regions, the learner retained the non-identity involution
`T(s)=1-s` (factor index 6, one of 2,972 semantically unique factor programs)
and the factor program `(-1)^k`. The exact checker verifies the universe is
the inferred semigroup, `T` is an involutive reflection with rational center
`1/2`, and `Xi(1-s)=(-1)^k Xi(s)` on every integer in `[-8,8]`. All three
frozen downstream objects are exact and accelerated (aggregate operations
212→148, measured gain 64); the asymmetric-universe and corrupted-`Xi`
controls are declined with baseline counts preserved. Description cost falls
72→14 stored integers. False-positive acceptances and negative-transfer tasks
are both zero. Claim level is
`L3_transferred_ontology_with_measured_utility`. No grammar, task, threshold,
ordering, or candidate space was changed after observing this outcome.

## Motivation and intended claim

M18 retained the toy completed factor `C(p,s)=p^{1-s}-p^s`. M19 asks whether,
from exact values of the completed object `Xi(s)=prod_p C(p,s)` in two regions,
the system can discover both the hidden reflection center and an auxiliary
factor program relating `Xi(c-s)` to `Xi(s)`. “Functional equation,”
“reflection,” “center,” and “symmetry” are not primitives or candidate labels.

Maximum claim: `L3_transferred_ontology_with_measured_utility`, conditional on
exact validation on every frozen compatible object, lower actual evaluation
work, exact decline of every control, and zero negative transfer. This is a
bounded exact toy claim, not an analytic-continuation theorem.

## Frozen substrate

- Hidden objects are finite irreducible sets `P` from `{2,3,5,7,11}` and the
  fixed center `c=1`; `k=|P|` is inferred from the extensional universe as in
  M18.
- The completed object is
  `Xi(s)=prod_{p in P} (p^{1-s}-p^s)`, evaluated with exact rational power
  arithmetic for integer `s`.
- Training observations are the exact rational values of `Xi` at
  `s in {-3,-2,-1,4,5,6}` for `P=[2,3,5]` and `P=[2,3,5,7]`. Only these
  values are stored; the formula and `P` are not passed to search.
- Transformation grammar: affine maps `T(s)=a-b*s` with `a in [-3,3]`,
  `b in [-2,2]\{0}`, enumerated lexicographically. A valid reflection must be
  involutive (`T(T(s))=s` for every integer `s`) and must move a probe
  (`T(0)!=0` or `T(1)!=1`); the identity transformation is a control and
  cannot be retained.
- Factor grammar: integer programs over one variable `k` with constants
  `-2,-1,0,1,2`, operations `+,-,*`, and the supplied primitive
  `pow(-1,k)`, AST size through 5, deduplicated by exact rational behavior on
  `k in {3,4}`.

## Exact checker

For a submitted pair `(T,F)`, the checker:

1. infers `P` and `k` from the extensional universe;
2. recomputes `Xi` values directly from `C(p,s)=p^{1-s}-p^s` with exact
   rational powers;
3. verifies the observed universe equals the inferred semigroup
   `U(P,2)` exactly;
4. verifies `T` is involutive and non-identity;
5. verifies `Xi(T(s)) = F(k)*Xi(s)` for every integer
   `s in [-8,8]` on every training object;
6. verifies the frozen center certificate: `T(1/2)=1/2` in rational
   arithmetic.

The checker returns boolean acceptance only; no normal form, residue,
counterexample, or center hint flows back to proposal ranking.

## Frozen downstream suite

None of these objects appear in training:

1. `P=[2,3,5,7,11]` with the four reflection pairs
   `(s,1-s)` for `s in {-5,-4,7,8}` (eight held-out points);
2. `P=[3,5,7]` with the same four reflection pairs;
3. `P=[2,5,11]` with the same four reflection pairs.

Controls, all incompatible:

1. asymmetric universe: one element removed from `U(P,E)`;
2. corrupted `Xi`: one stored value incremented by one;
3. identity transformation plus `F(k)=1` must fail the checker;
4. non-involutive transformation `T(s)=2-s` must fail the checker;
5. constant factor `F(k)=1` with the correct non-identity reflection must
   fail the checker.

## Compared conditions and accounting

- **Baseline:** store and evaluate `Xi` at all six observed values per
  training object and all eight held-out values per downstream object. Each
  exact rational `C(p,s)` evaluation is one operation and the product over
  `P` is `k-1` multiplications, so `Xi(s)` costs `k + (k-1)` operations.
- **Acquired:** store the transformation and factor programs, infer `P` and
  `k`, evaluate `Xi` on one member of each reflection pair, and apply the
  factor to obtain the other member. Cost per pair is one `Xi` evaluation plus
  one `F(k)` evaluation.
- Description cost is stored integers: raw stores `6` rationals (two integers
  each) per training object and `8` rationals per downstream object; acquired
  stores the transformation parameters, factor program nodes, and `k`.

Report per task: irreducible count, inference checks, checker calls, baseline
operations, acquired operations, exact identity, false-positive acceptance,
and negative transfer. Success requires exact recovery of every compatible
task, strictly fewer acquired operations, exact decline of every control with
unchanged baseline counts, zero false-positive acceptances, zero
negative-transfer tasks, and lower aggregate operations and description cost.

## Supplied-ontology ledger

- **Supplied:** M18's completed factor and `pow` primitive, the affine and
  factor grammars, the frozen center `c=1`, and the exact rational checker.
- **Acquired:** the reflection transformation `T(s)=1-s`, the factor program
  `(-1)^k`, and the functional-equation schema.
- **Not supplied:** functional-equation labels, center values, reflection
  constructors, parity predicates, or checker information beyond boolean
  acceptance.
- **Accounting:** operations, stored integers, inference checks, and checker
  calls are distinct labeled units.

## Controls and ablations

- A stronger substrate that receives `c` and `F` directly passes but does not
  establish discovery.
- A weaker grammar without `pow(-1,k)` must fail to express the odd/even
  factor for both `k=3` and `k=4`.
- Wrong centers, wrong factors, corrupted values, and asymmetric universes
  must be declined.

M19 fails L3 if no functional-equation schema is retained, any compatible task
is not exact or not accelerated, any control is accepted, aggregate
operations or description cost do not fall, or accounting mixes units. Any
post-output change creates M19b and preserves this run.

---

# Next Boundary — M20 Toy Completed Object

**Pre-registration date:** 2026-08-12

**Integrity contract:** `SCIENTIFIC_INTEGRITY.md`

**Debt ledger:** `ONTOLOGICAL_DEBT.md`

**Status:** frozen before M20 implementation; executed without amendment

## Recorded outcome

M20 passed the frozen L3 gate. From exact values of the raw object
`R(s)=prod_p C(p,s)B(p,s)`, the learner retained the completion
`G(s)=prod_p C(p,s)^{-1}B'(p,s)` (score `(2,2)`, first of 455 exact
completions). The checker verifies that `Ξ(s)=G(s)R(s)` satisfies the maximally
simple symmetry `Ξ(1-s)=Ξ(s)`, is nonconstant on the training grid, and
rejects monomial/constant rescalings. All three frozen downstream objects are
exact and accelerated (aggregate operations 672→432, measured gain 240);
asymmetric-universe and corrupted-raw controls are declined with baseline
counts preserved. Description cost falls 72→15 stored integers.
False-positive acceptances and negative-transfer tasks are both zero. Claim
level is `L3_transferred_ontology_with_measured_utility`. No grammar, task,
threshold, ordering, or candidate space was changed after observing this
outcome.

## Motivation and intended claim

M19 retained `Xi(1-s)=(-1)^k Xi(s)` for the toy factor `C(p,s)=p^{1-s}-p^s`.
M20 asks whether, given a raw toy object with an awkward signed reflection,
the system can construct auxiliary completion factors so the completed object
has the maximally simple symmetry `Ξ(1-s)=Ξ(s)`. “Completion,” “factor,”
“symmetric,” and “normalization” are not supplied as constructors.

Maximum claim: `L3_transferred_ontology_with_measured_utility`, conditional on
exact validation on every frozen compatible object, lower actual evaluation
work, exact decline of every control, and zero negative transfer. This is a
bounded exact toy completion, not an analytic-completion theorem.

## Frozen substrate

- The raw toy object is `R(s)=prod_{p in P} C(p,s)*B(p,s)` with
  `C(p,s)=p^{1-s}-p^s` and `B(p,s)=p^s+1`, evaluated with exact rational
  powers for integer `s`. `P` and `k` are inferred from the extensional
  universe as in M18.
- Training objects are `P=[2,3,5]` and `P=[2,3,5,7]`; raw values are observed
  at `s in {-4,-3,-2,2,3,4}`.
- Completion-factor grammar per prime:
  - atom `C`: `p^{1-s}-p^s`;
  - atom `B`: `p^s+1`;
  - atom `B'`: `p^{1-s}+1`;
  - monomial atoms `p^{a*s+b}` with `a,b in [-2,2]`;
  - constants `-1,1`.
  A candidate completion is `G(s)=prod_p prod_atom atom^{e_atom}` with
  exponents `e_atom in [-2,2]`, at most four atoms per prime, enumerated
  deterministically and deduplicated by exact behavior on the training grid.

## Exact checker

For a submitted completion `G`, the checker:

1. infers `P` and verifies the universe equals `U(P,2)`;
2. recomputes `R` and `G` directly with exact rational powers;
3. verifies `Ξ(s)=G(s)*R(s)` satisfies `Ξ(1-s)=Ξ(s)` for every integer
   `s in [-6,6]` on every training object;
4. rejects completions that are monomial rescalings or constant rescalings,
   including completions whose completed object is constant over the training
   grid (they cannot be measured as a genuine completion);
5. accepts only the lexicographically first candidate by (atom count, total
   exponent magnitude) among exact completions.

The checker returns boolean acceptance only; no residual, normal form, or
counterexample flows back into ranking.

## Frozen downstream suite

None of these objects appear in training:

1. `P=[2,3,5,7,11]`;
2. `P=[3,5,7]`;
3. `P=[2,5,11]`.

Each downstream object is evaluated on the reflection pairs
`(s,1-s)` for `s in {-4,-3,6,7}` (eight held-out points).

Controls, all incompatible:

1. monomial completion `G(s)=prod_p p^s` must fail;
2. corrupted raw value (one `R` value incremented by one) must fail;
3. asymmetric universe (one element removed from `U(P,E)`) must fail;
4. incomplete completion (only the `C` atom) must fail.

## Compared conditions and accounting

- **Baseline:** evaluate `R` and `G` at all eight held-out points.
- **Acquired:** evaluate `R` and `G` on one member of each reflection pair and
  reuse the completed value on the other member.
- Cost per `R(s)` evaluation is `2k` rational power evaluations plus `2k-1`
  multiplications; per `G(s)` evaluation it is the product of its atom
  evaluations; atom evaluation costs are counted as their arithmetic node
  counts.
- Description cost is stored integers: raw stores eight rationals per
  downstream object plus the raw-object program; acquired stores the
  completion atoms, exponents, and inferred `P`.

Report per task: irreducible count, inference checks, checker calls, baseline
operations, acquired operations, exact symmetry, false-positive acceptance,
and negative transfer. Success requires exact symmetry on every compatible
task, strictly fewer acquired operations, exact decline of every control,
zero false-positive acceptances, zero negative-transfer tasks, and lower
aggregate operations and description cost.

## Supplied-ontology ledger

- **Supplied:** the raw object definition, M19's factor atoms, exact rational
  powers, the bounded exponent grammar, and the exact checker.
- **Acquired:** the completion factor and the simple-symmetry schema.
- **Not supplied:** completion labels, normalization constructors, symmetry
  templates, or checker information beyond boolean acceptance.
- **Accounting:** operations, stored integers, inference checks, and checker
  calls are distinct labeled units.

## Controls and ablations

- Trivial monomial/constant rescalings must fail.
- A weaker grammar without the `B'` atom must fail to complete the raw object.
- Corrupted values and asymmetric universes must be declined.

M20 fails L3 if no completion is retained, any compatible task is not exact or
not accelerated, any control is accepted, aggregate operations or description
cost do not fall, or accounting mixes units. Any post-output change creates
M20b and preserves this run.

---

# Next Boundary — M21 Toy Critical Locus

**Pre-registration date:** 2026-08-12

**Integrity contract:** `SCIENTIFIC_INTEGRITY.md`

**Debt ledger:** `ONTOLOGICAL_DEBT.md`

**Status:** frozen before M21 implementation; executed without amendment

## Recorded outcome

M21 passed the frozen L3 gate. From exact zero positions on the toy lattice,
the learner retained the diagonal locus `u+v=1` (locus index 203 of 287 in
the fixed complexity order). The checker verifies that the locus equals the
zero set of `Ξ(u,v)=prod_p(p^{2-u-v}-p^{u+v})` exactly and is invariant under
both conjugation and reflection. All three frozen downstream objects are exact
and accelerated (aggregate lattice evaluations 654→183, measured gain 471);
asymmetric-universe, missing-zero, and extra-zero controls are declined with
baseline counts preserved. Description cost falls 96→13 stored integers.
False-positive acceptances and negative-transfer tasks are both zero. Claim
level is `L3_transferred_ontology_with_measured_utility`. No grammar, task,
threshold, ordering, or candidate space was changed after observing this
outcome.

## Motivation and intended claim

M20 retained a completed toy object with the simple symmetry
`Ξ(1-s)=Ξ(s)`. M21 asks whether, from exact zero positions on a toy lattice,
the system can infer the simplest geometric locus jointly explaining zero
positions, functional reflection, and conjugation. “Line,” “axis,” “critical
line,” and “symmetry locus” are not supplied.

Maximum claim: `L3_transferred_ontology_with_measured_utility`, conditional on
exact zero-set recovery, lower actual lattice evaluation work, exact decline
of every control, and zero negative transfer. This is a bounded integer-lattice
toy, not a complex-analytic critical-line theorem.

## Frozen substrate

- The toy domain is the integer lattice `(u,v)` with `u,v in [-6,6]`.
- Hidden objects are irreducible sets `P` from `{2,3,5,7,11}` inferred from
  the extensional universe as before.
- The completed lattice object is
  `Ξ(u,v)=prod_{p in P} (p^{2-u-v}-p^{u+v})`, evaluated with exact rational
  powers for integer exponents. Its zeros are exactly the lattice points with
  `u+v=1`.
- Training observations are the zero positions on `[-3,3]^2` for
  `P=[2,3,5]` and `P=[2,3,5,7]`, plus the boolean facts that the object is
  symmetric under conjugation `(u,v)->(v,u)` and that reflection
  `(u,v)->(1-v,1-u)` maps zero positions to zero positions.

## Frozen locus grammar

Candidates are geometric predicates over the lattice, enumerated in this
fixed complexity order:

1. `all` (accepts every lattice point);
2. `point(a,b)`;
3. `vertical(a)` (`u=a`);
4. `horizontal(b)` (`v=b`);
5. `diagonal(c)` (`u+v=c`);
6. `pair_diagonals(c1,c2)` (union of two diagonals).

Parameters range over `[-6,6]`. The retained locus is the first candidate
that exactly matches the training zero set and is invariant under both the
conjugation and reflection maps.

## Exact checker

For a submitted locus, the checker:

1. infers `P` and verifies the universe equals `U(P,2)`;
2. recomputes `Ξ(u,v)` directly for every lattice point in the training range
   and compares its zero set to the locus;
3. verifies the locus is invariant under conjugation and reflection;
4. rejects loci that are not minimal: if a simpler candidate in the fixed
   order also matches, the submitted locus is not retained.

The checker returns boolean acceptance only; no zero coordinates, residuals,
or counterexamples flow into ranking.

## Frozen downstream suite

None of these objects appear in training:

1. `P=[2,3,5,7,11]` on `[-6,6]^2`;
2. `P=[3,5,7]` on `[-6,6]^2`;
3. `P=[2,5,11]` on `[-6,6]^2`.

Controls, all incompatible:

1. a single off-line zero added to the observed set;
2. one on-line zero removed from the observed set;
3. an asymmetric universe (one element removed from `U(P,E)`);
4. wrong loci: `point`, `vertical`, `horizontal`, `pair_diagonals`, and
   `all` must fail the checker.

## Compared conditions and accounting

- **Baseline:** evaluate `Ξ` at every point of the downstream lattice
  (`169` points) to find zeros.
- **Acquired:** evaluate the retained diagonal locus only (`13` points) and
  verify it matches the observed zeros.
- Description cost: raw stores the zero coordinates; acquired stores the locus
  parameters and inferred `P`.

Report per task: irreducible count, inference checks, baseline evaluations,
acquired evaluations, exact zero-set recovery, false-positive acceptance, and
negative transfer. Success requires exact recovery on every compatible task,
strictly fewer acquired evaluations, exact decline of every control, zero
false-positive acceptances, zero negative-transfer tasks, and lower aggregate
evaluations and description cost.

## Supplied-ontology ledger

- **Supplied:** the integer lattice, the completed-object definition, exact
  rational powers, the fixed locus grammar, and the reflection/conjugation
  constraints.
- **Acquired:** the diagonal locus `u+v=1` and its zero-set explanation.
- **Not supplied:** line/axis labels, critical-line templates, symmetry
  constructors, or checker information beyond boolean acceptance.
- **Accounting:** lattice evaluations, stored coordinates, inference checks,
  and checker calls are distinct labeled units.

## Controls and ablations

- A stronger substrate that receives the hidden diagonal directly passes but
  does not establish discovery.
- A weaker grammar without the diagonal candidate cannot explain the zero set.
- Corrupted zero sets and asymmetric universes must be declined.

M21 fails L3 if no locus is retained, any compatible task is not exact or not
accelerated, any control is accepted, aggregate evaluations or description
cost do not fall, or accounting mixes units. Any post-output change creates
M21b and preserves this run.

---

# Next Boundary — M22 Hidden Toy Zeros

**Pre-registration date:** 2026-08-12

**Integrity contract:** `SCIENTIFIC_INTEGRITY.md`

**Debt ledger:** `ONTOLOGICAL_DEBT.md`

**Status:** frozen before M22 implementation; executed without amendment

## Recorded outcome

M22 passed the frozen L3 gate. From exact oscillation signals with zero
positions withheld, the learner retained the oscillator model
`u in {-2,0,3}` with weights `{1,2,1}` from 31,713 dictionary models, and the
checker mapped every recovered frequency through the retained diagonal locus
to `(u,1-u)`. All three frozen downstream hidden sets are recovered exactly
and accelerated (aggregate lattice evaluations 6,897→4,779, measured gain
2,118); corrupted-signal, off-locus, and asymmetric-universe controls are
declined with baseline counts preserved. Description cost falls 51→12 stored
integers. False-positive acceptances and negative-transfer tasks are both
zero. Claim level is `L3_transferred_ontology_with_measured_utility`. No
grammar, task, threshold, ordering, or candidate space was changed after
observing this outcome.

## Motivation and intended claim

M21 retained the diagonal zero locus `u+v=1`. M22 withholds the zero positions
and asks whether, from exact arithmetic oscillation signals over the toy
lattice, the system can invent latent oscillators and recover the hidden
spectral locations, completing each recovered frequency with the retained
diagonal locus. “Zero,” “frequency,” “oscillator,” and “spectrum” are not
supplied as candidate labels.

Maximum claim: `L3_transferred_ontology_with_measured_utility`, conditional on
exact signal recovery and hidden-location recovery on every frozen compatible
task, lower actual lattice work, exact decline of every control, and zero
negative transfer. This is a bounded integer toy, not complex-frequency
estimation.

## Frozen substrate

- The toy lattice is `(u,v)` with `u,v in [-5,5]`; hidden zeros lie on the
  M21 diagonal `v=1-u`.
- A hidden oscillator for coordinate `u` has exact integer base
  `q(u)=prime(u+4)` where `prime` is the frozen list
  `[2,3,5,7,11,13,17,19,23,29,31]` for `u=-5..5`, and an integer weight
  `w in {1,2,3}`.
- The arithmetic observable is `a[t]=sum_{u in hidden} w_u * q(u)^t` for
  integer `t`, evaluated with exact integers.
- Training hidden set is `u in {-2,0,3}` with weights `{1,2,1}`; observable
  values are given for `t=0..12`.
- Candidate models enumerate subsets of `u in [-5,5]` of size at most 4 with
  weights in `{1,2,3}`, lexicographically, and are deduplicated by exact
  signal behavior.

## Exact checker

For a submitted oscillator model, the checker:

1. infers `P` from the extensional universe and verifies `U(P,2)`;
2. recomputes `a[t]` for every frozen `t` directly from the dictionary and
   submitted weights;
3. requires exact equality with the observed signal on all training and
   held-out exponents;
4. maps each recovered `u` to the hidden location `(u,1-u)` and verifies it
   lies on the retained diagonal locus;
5. rejects models with fewer distinct frequencies when an equally fitting
   smaller support exists (lexicographic minimality).

The checker returns boolean acceptance only; no residuals, weights, or
frequency hints flow into ranking.

## Frozen downstream suite

None of these hidden sets appear in training:

1. `u in {-4,-1,1,4}`, weights `{1,2,1,3}`;
2. `u in {-3,2,5}`, weights `{1,3,2}`;
3. `u in {-5,0,5}`, weights `{2,1,2}`.

Each downstream object supplies a universe from the frozen prime pool and
observable values at held-out `t=13..18`.

Controls, all incompatible:

1. corrupted signal: one held-out value incremented by one;
2. off-dictionary frequency: a base not in the frozen prime list;
3. off-locus location: a recovered `u` mapped to `v != 1-u`;
4. asymmetric universe: one element removed from `U(P,E)`.

## Compared conditions and accounting

- **Baseline:** evaluate the observable over all 121 lattice points for each
  held-out exponent.
- **Acquired:** evaluate only the retained oscillators (one term per hidden
  zero) and map them through the retained diagonal locus.
- Description cost: raw stores the observable values and zero-counting
  positions; acquired stores the retained oscillator bases, weights, and the
  locus parameters.

Report per task: irreducible count, inference checks, baseline evaluations,
acquired evaluations, exact signal recovery, exact location recovery,
false-positive acceptance, and negative transfer. Success requires exact
recovery on every compatible task, strictly fewer acquired evaluations, exact
decline of every control, zero false-positive acceptances, zero
negative-transfer tasks, and lower aggregate evaluations and description cost.

## Supplied-ontology ledger

- **Supplied:** the oscillator dictionary, weight range, exact integer
  arithmetic, the retained M21 diagonal locus, and the exact checker.
- **Acquired:** the hidden oscillator set and weights, and the mapping from
  recovered frequencies to lattice locations.
- **Not supplied:** zero labels, frequency labels, spectral templates,
  projection formulas, or checker information beyond boolean acceptance.
- **Accounting:** lattice evaluations, stored integers, inference checks, and
  checker calls are distinct labeled units.

## Controls and ablations

- A stronger substrate that receives the hidden set directly passes but does
  not establish discovery.
- A weaker dictionary missing the true bases cannot recover the signal.
- Corrupted signals, off-dictionary bases, and off-locus locations must be
  declined.

M22 fails L3 if no oscillator model is retained, any compatible task is not
exact or not accelerated, any control is accepted, aggregate evaluations or
description cost do not fall, or accounting mixes units. Any post-output
change creates M22b and preserves this run.

---

# Next Boundary — M23 Toy RH-Like Conjecture

**Pre-registration date:** 2026-08-12

**Integrity contract:** `SCIENTIFIC_INTEGRITY.md`

**Debt ledger:** `ONTOLOGICAL_DEBT.md`

**Status:** frozen before M23 implementation; executed without amendment

## Recorded outcome

M23 passed the frozen L3 gate. From partial training zeros with hidden truth
withheld, the frozen scoring rule retained the conjecture
`all_zeros_have_u+v=1` (conjecture index 7 of 209). The checker validates the
conjecture on every held-out zero for all three frozen downstream objects,
records the primary falsifier `(-6,-6)`, and never labels the result proved.
Prediction evaluations fall 845→374 (measured gain 471); corrupted-training
and asymmetric-universe controls are declined. Description cost falls 48→13
stored integers. False-positive acceptances and negative-transfer tasks are
both zero. Claim level is
`L3_transferred_ontology_with_measured_utility` with
`status=conjectured,proof=false`. No grammar, score, task, threshold, or
candidate space was changed after observing this outcome.

## Motivation and intended claim

M21 retained the diagonal zero locus and M22 recovered hidden zeros through
it. M23 asks whether, from partial zero evidence on the toy lattice, the
system can generate the strongest simple conjecture supported by the
evidence, ideally that all hidden zeros lie on the discovered symmetry locus.
“Conjecture,” “critical line,” and “RH” are not supplied as candidate labels.

Maximum claim: `L3_transferred_ontology_with_measured_utility` if the
conjecture is generated by frozen scoring, validates on held-out zeros,
reduces actual prediction work, and falsification controls behave exactly.
The output must remain labeled `conjectured`, never proved.

## Frozen conjecture language and scoring

Conjectures are lattice predicates over `(u,v) in [-6,6]^2`:

1. `diagonal(c)`: all zeros satisfy `u+v=c`;
2. `vertical(a)`: all zeros satisfy `u=a`;
3. `horizontal(b)`: all zeros satisfy `v=b`;
4. `point(a,b)`: all zeros equal `(a,b)`;
5. `all`: no restriction.

The frozen scoring rule, fixed before hidden truth is inspected:

1. a conjecture must cover every observed training zero;
2. score = (predicate complexity in the fixed order above, number of covered
   training zeros, parameter magnitude);
3. the retained conjecture is the lexicographically smallest exact cover.

Training evidence is a frozen subset of the hidden zeros: for
`u in {-5,-3,-1,1,3,5}`, the points `(u,1-u)` are observed; the remaining
diagonal points and all off-diagonal points are withheld.

## Exact checker

For a submitted conjecture, the checker:

1. verifies it covers every training zero;
2. verifies the retained score is minimal in the frozen ordering;
3. validates the conjecture on all held-out lattice points: every held-out
   zero must lie on the conjecture, and the first off-conjecture lattice point
   is recorded as the primary falsifier;
4. returns `conjectured=true` and never returns `proved`.

The checker returns boolean acceptance only; no held-out coordinates or
falsifiers flow into scoring.

## Frozen downstream suite

None of these objects appear in scoring:

1. `P=[2,3,5,7,11]` on `[-6,6]^2`;
2. `P=[3,5,7]` on `[-6,6]^2`;
3. `P=[2,5,11]` on `[-6,6]^2`.

For each, the conjecture predicts the hidden zero positions.

Controls, all incompatible:

1. corrupted training zero: one observed point moved off the diagonal;
2. weaker conjecture that also covers training (e.g., `all`) must lose
   scoring or fail held-out validation;
3. asymmetric universe: one element removed from `U(P,E)`.

## Compared conditions and accounting

- **Baseline:** evaluate the toy object over all `169` lattice points per
  downstream object.
- **Conjectured:** evaluate only the predicted diagonal points (`13`) and
  compare against the observed zero positions.
- Description cost: raw stores all observed zero coordinates; conjectured
  stores the predicate parameters.

Report per task: irreducible count, inference checks, baseline evaluations,
conjectured evaluations, held-out zero validation, primary falsifier,
false-positive acceptance, and negative transfer. Success requires the
conjecture to be generated by frozen scoring, hold on every held-out zero,
use strictly fewer evaluations, decline every control, and lower aggregate
evaluations and description cost.

## Supplied-ontology ledger

- **Supplied:** the lattice, partial zero evidence, the frozen predicate
  language and score, M21's symmetry facts, and the exact checker.
- **Acquired:** the conjectured diagonal predicate and its falsifier.
- **Not supplied:** critical-line labels, RH statements, target-shaped
  templates, or hidden-truth information in scoring.
- **Accounting:** lattice evaluations, stored coordinates, inference checks,
  and checker calls are distinct labeled units.

## Controls and falsification

Any off-diagonal training zero must force a different conjecture or a failed
held-out validation. The output is conjectured; finite agreement is not a
proof. M23 fails L3 if the conjecture is not generated by the frozen score, any
held-out zero contradicts it, any control is accepted, aggregate evaluations or
description cost do not fall, or accounting mixes units. Any post-output
change creates M23b and preserves this run.

---

# Next Boundary — M24 Toy-RH Equivalence

**Pre-registration date:** 2026-08-12

**Integrity contract:** `SCIENTIFIC_INTEGRITY.md`

**Debt ledger:** `ONTOLOGICAL_DEBT.md`

**Status:** frozen before M24 implementation; executed without amendment

## Recorded outcome

M24 passed the frozen L3 gate. From the frozen candidate predicate grammar
and novelty rule, the learner retained `Q: Xi(1-v,1-u)=0` (index 0 of 5). The
independent checker proves `D -> Q` and `Q -> D` by exhaustive finite case
analysis on every frozen object; the `zero_at_point` paraphrase, `vertical`,
`all`, corrupted-`Xi`, and asymmetric-universe controls are all declined.
Including the separately reported proof comparisons, membership reasoning is
cheaper on all three downstream objects (baseline 2,378 ops vs 943 Q ops plus
1,014 proof comparisons). Description cost falls 507→13 stored integers.
False-positive acceptances and negative-transfer tasks are both zero. Claim
level is `L3_transferred_ontology_with_measured_utility`. No grammar, task,
threshold, novelty rule, or candidate space was changed after observing this
outcome.

## Motivation and intended claim

M23 generated the toy conjecture `D(u,v): u+v=1` for hidden zeros. M24 asks
whether the system can generate a non-obvious predicate `Q`, prove both
`D -> Q` and `Q -> D` with an independent checker, and show that reasoning
about `Q` is cheaper after accounting for the equivalence proof. “Equivalent,”
“reflection,” and “conjugation” are not supplied as candidate labels.

Maximum claim: `L3_transferred_ontology_with_measured_utility` if both
directions pass the checker on every frozen object, `Q` passes the novelty
rule, and downstream membership reasoning is cheaper including the amortized
equivalence proof.

## Frozen substrate

- The lattice is `(u,v) in [-6,6]^2` and the completed object is M20's
  `Ξ(u,v)=prod_p(p^{2-u-v}-p^{u+v})`.
- ToyRH `D` is the frozen conjecture: every lattice zero satisfies `u+v=1`.
- Candidate `Q` grammar over point predicates:
  1. `zero_after_reflection`: `Ξ(1-v,1-u)=0`;
  2. `zero_after_conjugation`: `Ξ(v,u)=0`;
  3. `zero_at_point`: `Ξ(u,v)=0` (paraphrase control);
  4. `vertical_zero`: `u=0 and Ξ(u,v)=0`;
  5. `all_points`: always true.
- Novelty rule: `Q` must be semantically different from `D` and from the
  `zero_at_point` paraphrase; `all_points` and `vertical_zero` are controls.

## Exact checker

For a submitted `Q`, the checker:

1. recomputes `Ξ` directly and verifies the M21 reflection and conjugation
   invariance axioms on the full lattice;
2. proves `D -> Q` and `Q -> D` by exhaustive finite case analysis over all
   lattice points, returning separate boolean certificates for each
   direction;
3. applies the novelty rule;
4. verifies the equivalence on all frozen downstream objects and held-out
   points.

The checker returns boolean certificates only; no residuals or
counterexamples flow into ranking.

## Frozen downstream suite

None of these objects appear in equivalence discovery:

1. `P=[2,3,5,7,11]`;
2. `P=[3,5,7]`;
3. `P=[2,5,11]`.

Each object supplies held-out lattice points.

Controls, all incompatible:

1. `zero_at_point` paraphrase;
2. `vertical_zero`;
3. `all_points`;
4. corrupted `Ξ` (one value changed);
5. asymmetric universe.

## Compared conditions and accounting

- **Baseline ToyRH membership:** evaluate `Ξ` over the product `P` at the
  candidate point; cost is `2k` power evaluations plus `2k-1` multiplications.
- **Q membership:** one reflection/conjugation table lookup against the
  retained equivalent predicate; cost is one operation.
- Equivalence-proof cost is counted once per downstream object and reported
  separately: each direction costs one exhaustive lattice check of
  `(2R+1)^2` point comparisons.
- Description cost: raw stores `Ξ` over the lattice; acquired stores `Q` plus
  the equivalence certificates.

Report per task: irreducible count, inference checks, direction-A certificate,
direction-B certificate, baseline membership ops, Q membership ops, exact
equivalence, false-positive acceptance, and negative transfer. Success
requires both certificates on every compatible task, novelty passes, Q
membership cheaper on every compatible task including amortized proof cost,
exact decline of every control, zero false-positive acceptances, zero
negative-transfer tasks, and lower aggregate cost.

## Supplied-ontology ledger

- **Supplied:** the lattice, completed object, reflection/conjugation
  invariance axioms, candidate predicate grammar, novelty rule, and exact
  finite checker.
- **Acquired:** the equivalent predicate `Q` and its bidirectional
  certificates.
- **Not supplied:** equivalence labels, reflection/conjugation templates,
  target leakage, or checker information beyond boolean certificates.
- **Accounting:** membership operations, proof comparisons, stored integers,
  inference checks, and checker calls are distinct labeled units.

## Controls and falsification

Paraphrases, vacuous predicates, corrupted objects, and asymmetric universes
must fail. Both directions are required; a one-way consequence is not an
equivalence. M24 fails L3 if no novel equivalent predicate is retained, either
direction fails on any compatible task, Q is not cheaper, any control is
accepted, or accounting mixes units. Any post-output change creates M24b and
preserves this run.
