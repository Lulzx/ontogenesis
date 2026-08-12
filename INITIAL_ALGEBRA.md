# U3: bounded initial-algebra/catamorphism ontogenesis

U3 connects the project’s recursion and universal-property tracks. Its qualified
claim is:

> In the declared bounded observational category for `F(X)=1+X`, independent
> lambda search discovers and acquires an initial-algebra-like recursive
> structure. Every unseen algebra has one bounded semantic mediator class
> satisfying the frozen recursion equation, and reuse reduces later search cost.

This is not unrestricted initiality, a theorem prover, or evidence that one
latent representation is uniquely correct.

## Declared world

Objects have colors `M` (anonymous recursive carrier) and `A` (an arbitrary
target algebra carrier). Morphisms are closed untyped lambda terms. Identity is
`λx.x`, composition is lambda application/composition, and equality is equality
of full beta-normal forms under 2,000,000 evaluator fuel.

The sole supplied structural assumption is the finite action `F(X)=1+X`:

```text
F(h)(base-layer) = base-layer
F(h)(step(m))    = step(h(m))
```

Its closed implementation is frozen before protected replay. There is no
candidate production for an algebra, initial object, catamorphism, fold, reduce,
recursion, fixed point, eliminator, or the universal equation. U3 imports only
the evaluator, term representation, generic transformations, typed enumerator,
and universal lambda enumerator. It does not import or call U1, U2, B2 recurrence,
recursive search, representation invention, or fixed-point synthesis.

## Evidence and split

An algebra is supplied extensionally as a base value and a unary step. Training
contains Boolean even parity and Church-numeral counting at depths 0–3.
Calibration reconstructs a Church list at depths 0–4. Protected replay contains
composed odd parity and double counting at depths 5, 7, and 9.

Thus one frozen `M` must mediate Boolean, numeral, and list targets. This blocks
`M=A` and per-algebra carriers. Evidence records include epoch, duplicate group,
and derivation metadata. Target/output/trace-derived, ancestor-derived,
near-duplicate, protected-ID, and post-freeze records are filtered before
enumeration. IDs, outputs, traces, and protected annotations are not lambda
inputs. Protected annotations may be mutated without changing structure,
classes, ordering, ranking, budget, or counters.

## Independent discovery

Carrier base and step candidates come from the empty-alphabet language:

```text
t ::= bound-variable | λt | t(t)
```

Search checks two base candidates through size 3 and 3,822 step candidates
through size 10. A carrier pair must produce ten distinct terminating normal
forms on depths 0–9. The generic typed grammar then searches for a constructor
`F(M)->M` using only the proposed base/step as opaque atoms. The winning term is
expanded back into a closed pure-lambda implementation; no atom remains.

Separately, the typed grammar enumerates 17 candidate programs through size 8
for the polymorphic interface:

```text
A -> (A -> A) -> M -> A
```

The external verifier—not the grammar—tests every visible algebra and the law:

```text
h . constructor ~= algebra . F(h)
```

The discovered structure is:

```text
carrier witness = λa.λb.b
carrier step    = λa.λb.λc.b(a(b,c))
constructor     = λlayer.layer(carrier-witness, carrier-step)
generator       = λbase.λstep.λm.m(step,base)
```

The printed constructor is expanded and has syntax size 17; its typed search
spelling has size at most 6 because the candidate base/step are opaque during
that independent interface search. The generator has size 8. Externally these
are recognizable as Church numerals and their iterator, but no such candidate
was seeded.

Discovery evaluates 4,255 generator candidates across 2,565 safe/unsafe carrier
pairs. Exact accounting is emitted by the executable experiment.

## Equation, uniqueness, and extrapolation

For every evidence algebra `(base,step)`, the generated `h` is tested on each
layer:

```text
h(constructor(base-layer)) ~= base
h(constructor(step-layer(m))) ~= step(h(m))
```

Protected odd-parity and double-count mediators terminate and satisfy the law at
depths 5, 7, and 9. These depths exceed the training/calibration range.

Uniqueness uses an independent typed enumeration of every `M->A` inhabitant
through size 8 with a 50,000 per-cell cap. It is untruncated. The two protected
algebras have two and one valid syntactic mediators respectively; each has one
full-normal-form observational class. A cap-1 control reports truncation and
forbids uniqueness.

The non-initial control contains two disconnected successor chains. Its
constructor starts in one chain but preserves either hidden tag on successor.
Two mediators satisfy the same counting equation on both chains yet choose
different values on the unreachable second base. Existence therefore holds and
uniqueness fails.

## Distinction from B2 and controls

Identity exactly fits the carrier’s finite count observations, so a recurrence-
only learner can appear successful on that one representation. Installing it as
the purported generator fails the protected odd-parity algebra equation. U3’s
claim therefore depends on one mediator generator working across heterogeneous
algebras and on bounded uniqueness, not merely a recursive term fitting finite
examples.

Normalized base/step programs across evidence share no closed subtree of size
four or larger. Normalized carrier values also do not contain the previous value
as an exact subtree, so matched syntax and recurrence-subtree baselines fail.

Tests additionally reject wrong and missing constructor behavior, constant
generators, identity/collision carriers, open terms, divergence, incomplete
probe suites, undersized step search, single-representation curricula, and cap
truncation. Nondeterminism, lookup primitives, task IDs, and whole-record storage
are unrepresentable in the empty-alphabet production grammar.

## Downstream economics

The downstream positive task is carrier-to-carrier doubling: map a depth-`n`
carrier value to depth `2n`. It is neither construction nor simple elimination.

```text
condition                 solved  proposals  generated  observations
learned U3                   yes         12      3,311            29
uniform acquired grammar     yes         15      3,311            32
external oracle              yes          1          1            10
raw typed abstract M          no          1          2             2
irrelevant ontology           no          9         17            18
pure lambda through size 8    no        707        707           866
```

Learned U3 has oracle regret 11. It is 1.25x earlier than uniform and at least
58x earlier than the unsolved pure-lambda prefix by evaluated proposals. The
claim is deliberately modest: this demonstrates allocation gain, not practical
efficiency.

Discovery, representation, equation checks, uniqueness validation, and
installation cost 41,628 lambda-observation units. Learned allocation saves
three checks per protected reuse. At the declared 100,000-use horizon:

```text
uniform allocation  3,200,000 checks
learned allocation  2,900,000 checks
discovery charge        41,628 checks
net gain               258,372 checks
```

Identity is a negative-transfer control: raw search needs one proposal/ten
checks, while the learned ordering needs three/thirteen. A valid but useless
structure fails the charged acquisition gate. A beta-expanded equivalent step
is extensionally checked through depth 9 and loses on syntax cost.

Lambda observations, typed proposals, universal resource points, and behavior
executions are separate work domains; mixed aggregation is rejected.

## Cost geometry and fallback

Measured Boolean morphism distances yield `(2,384,384)`, `(384,2,384)`, and
`(384,384,2)`. Each satisfies `d(A,C) <= d(A,B)+d(B,C)+1`. This supports only a
sampled finite cost geometry.

Learned resource points alternate with the unchanged universal dovetail.
Projecting out learned work reproduces the first 256 universal points exactly,
and extreme representable finite syntax/fuel pairs retain finite indices.

## Acceptance map

- `discovers_initial_like_structure_and_extrapolates`: independent language
  membership, heterogeneous algebras, protected depths, equation, uniqueness,
  syntax and recurrence baselines.
- `hidden_state_has_existence_without_uniqueness`: non-initial disconnected-chain
  falsification.
- `recurrence_only_fit_fails_the_relational_law`: B2 distinction.
- `leakage_mutation_and_fallback_are_invariant`: freeze/leakage firewall,
  protected mutation, ranking/counter replay, exact universal projection.
- `truncation_economics_controls_and_units_are_explicit`: cap disclosure,
  baselines, regret/gain, acquisition, negative transfer, equivalent-cost
  preference, unlike-unit rejection, undersized boundary.
- `wrong_constant_open_divergent_and_incomplete_controls_fail`: unsafe,
  collision, incomplete, and target-collapse controls.

## Reproduction and limitations

```sh
cargo run --release --example initial_algebra
cargo test --release -p supsearch initial_algebra --lib
cargo test --workspace
```

The experiment emits one machine-readable `record,...` row. No real-task
transfer is claimed because this finite algebra interface has not been unified
with ARC accounting.

The supplied pieces remain substantial: `F(X)=1+X`, object colors, lambda
substrate, typed interfaces, probes, equality, evaluator, bounds, cost prices,
freeze policy, learned score, reuse horizon, and split. U3 does not discover an
arbitrary functor, prove unbounded initiality, establish general induction,
recover a unique mathematical ontology, or show external superiority.
