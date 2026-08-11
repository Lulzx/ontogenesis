# U1: bounded universal-property ontogenesis

U1 asks whether relational patterns among computations can cause the system to
invent an anonymous shared representation, rather than merely factor repeated
program syntax.

The demonstrated claim is deliberately finite:

> In the declared bounded observational category, pure-lambda search discovers
> and acquires a product-like factorization whose mediators commute and are
> unique up to the frozen observational equivalence on every enumerated
> mediator within the stated boundary.

This is not an unrestricted proof of a categorical product.

## Declared observational category

Objects are colored interfaces for Church Booleans, Church numerals, Church
lists, an anonymous carrier, and several source roles. Morphisms are closed
untyped lambda terms. Composition is ordinary lambda application/composition;
identity is `λx.x`. Equality is normalization equality on a frozen finite probe
family with 100,000 units of evaluator fuel.

Training uses unrelated Boolean and ternary-numeral sources. Calibration swaps
the observed roles and uses composed numerical predicates. Protected replay
uses both a Boolean source and a genuinely different Church-list
representation. Exact and near-duplicate groups cannot cross the freeze.

Discovery never receives task IDs, protected annotations, target outputs,
solution traces, target ancestry, or post-freeze evidence as lambda inputs.

## Discovery language

The carrier and two observers are enumerated from the empty-alphabet universal
language:

```text
t ::= bound-variable | λt | t(t)
```

There are no productions or atoms for pair, tuple, product, field, projection,
cone, mediator, constructor, or the product equations. Candidates are ordered
by exact lambda syntax size. The experiment checks 450 closed candidates with
at least two leading lambdas through size 8 and 57 observer candidates through
size 6.

The experience supplies executable arrows `f:X->A` and `g:X->B`. Their observed
values `(f(x),g(x))` form the relational factorization evidence. A carrier must
separate those observed combinations and independently separate nine numeral
combinations. Two independently enumerated observers must recover the two
roles on both sets.

After a carrier is found, the operation-blind simply typed normal-form engine
receives only its inferred interface `A->B->P`. It enumerates a closed program
of type:

```text
(X->A) -> (X->B) -> X -> P
```

and the external verifier retains it only when both equations commute. The
typed grammar contains only variables, lambdas, application, and acquired
closed atoms; it has no mediator production.

## Discovered structure

The frozen result is:

```text
carrier  = λa.λb.λc.c(b,a)
observeA = λp.p(λa.λb.b)
observeB = λp.p(λa.λb.a)
generator= λf.λg.λx.carrier(f(x),g(x))
```

The carrier happens to store its arguments in the opposite internal order;
the independently discovered observers compensate. This is external evidence
that search was not seeded with the conventional Church-pair spelling.

The carrier has size 8, each observer size 6, and the generator size 12. The
normalized training morphisms have no common closed subtree of size at least 4,
so the existing syntax-factorization baseline proposes nothing.

## Existence and bounded uniqueness

For each frozen cone the verifier checks:

```text
observeA(generator(f,g)(x)) ~= f(x)
observeB(generator(f,g)(x)) ~= g(x)
```

Uniqueness is not inferred from those two equations. For every protected cone,
the typed engine independently enumerates every `X->P` inhabitant through size
10 using only the carrier, `f`, and `g`, with a per-cell cap of 50,000. The
enumerator reports whether that cap excluded a term. It did not. Each protected
cone has one valid mediator, one full-normal-form equivalence class, and an
exhaustive-within-size flag of `true`.

A three-field control stores an additional hidden tag. Two mediators with
different tags satisfy both observer equations but normalize to different
carrier values. It therefore passes existence and fails uniqueness. This is the
load-bearing distinction between a container and the tested universal object.

## Acquisition and downstream cost

The learned lane prioritizes programs by task-independent acquired-atom density
and is a finite prefix to the unchanged universal resource lane. Projecting
away learned points reproduces the original dovetail exactly.

On a protected downstream field-swap transformation—not merely construction of
the carrier—the conditions are:

```text
condition                         proposals   observation checks
learned U1                               1                    9
external oracle                          1                    9
raw typed search                        10                   23
fixed irrelevant ontology              31                   51
uniform acquired-atom grammar          32                   50
pure universal lambda through size 10  10,180           10,852  (unsolved)
learned prefix + same lambda observer       1                 9
```

Proposal counts are compared only within the typed conditions. Pure universal
work is reported separately; its learned-prefix condition uses the same lambda
observer and resumes the exact empty-alphabet stream after the finite prefix.
The learned condition has zero proposal regret against the external oracle.
It is exactly 10x earlier than raw typed search and at least 10,180x earlier
than the still-unsolved pure-universal boundary in evaluated-proposal units.

Discovery and validation cost 5,286 comparable observational checks. Swap reuse
saves 14 checks per use, so the charged break-even point is 378 uses. Over the
preregistered 1,000-use reuse horizon:

```text
without U1       23,000 checks
with U1           9,000 checks
discovery charge  5,286 checks
net gain           8,714 checks
```

No unlike units are collapsed into that scalar. The separately comparable
evaluated-proposal domains also clear their own gates over the same horizon:

```text
observation-check net                         +8,714
typed-proposal net: 9,000 saved - 71 learned  +8,929
universal-proposal lower bound:
  10,180 raw - 1 learned - 507 discovery      +9,672
```

The universal number is a lower bound because raw search remains unsolved at
the frozen size-10 boundary.

Memoized syntax materialization is not hidden inside those ranks. Raw typed
swap materializes 256 terms, learned typed swap 2,991, and the learned universal
prefix 174. On `mapBoth`, raw and learned typed conditions materialize 28,791
and 384,876 terms respectively. These are deliberately reported as separate
generation counters; they are not converted into proposal or observation units.
They reinforce that U1 is a bounded latency/result, not a practical-efficiency
claim.

The value gate is contextual. A second full-carrier `mapBoth` task is a negative
transfer control: raw typed search takes 221 proposals while the learned lane
takes 442. Its evidence therefore cannot justify acquisition by itself. A valid
but useless structure fails the charged gate, and an observationally equivalent
beta-expanded carrier loses on syntax cost.

## Sampled cost enrichment

Distances are actual observational-equation checks spent by pure-lambda search,
not invented constants or proposal ranks. On the frozen Boolean morphism
sample, identity costs 2 and negation costs 384. The
measured `(d(A,B),d(B,C),d(A,C))` triples are:

```text
(2,384,384)
(384,2,384)
(384,384,2)
```

All satisfy `d(A,C) <= d(A,B)+d(B,C)+1`. Formal cost-enrichment language in U1
is restricted to this sampled finite world.

## Controls and leakage boundary

Tests reject or exclude:

- constant and collision-heavy carriers;
- wrong, swapped, or missing observers;
- a wrong mediator generator;
- the non-unique hidden-tag carrier;
- divergent and out-of-fuel terms;
- undersized and oversized candidates relative to the frozen boundary;
- open, malformed, primitive-seeded, and nondeterministic terms, which are
  unrepresentable in the production grammar;
- empty or single-source curricula;
- exact/near duplicates, target-derived and output-derived evidence, ancestry,
  post-freeze records, IDs, lookup-table access, and whole-input storage;
- richer equivalent structures without downstream gain;
- uniform, irrelevant, syntax-mining, and bounded universal baselines.

Mutating every protected annotation leaves the visible cone, discovered terms,
mediator classes, ranking, budget, candidate order, and all replay counters
unchanged.

Accounting exposes carrier candidates, observer candidates, factorization
triples, generated higher-order terms, independently enumerated mediators,
normalization checks, equivalence checks, syntax/fuel boundaries, and
termination. Lambda observations, typed proposals, universal resource points,
and BehaviorBank constructions are distinct work domains; mixed aggregation is
rejected.

## Reproduction

```sh
cargo run --release --example universal_property
cargo test --release -p supsearch universal_property --lib
cargo test --workspace
```

The executable emits a machine-readable `record,...` row.

## Acceptance-test map

`discovers_and_reuses_a_bounded_universal_factorization` covers pure-language
membership, heterogeneous cones, frozen commutativity, exhaustive bounded
uniqueness, protected mutation, downstream/oracle/uniform/irrelevant/universal
conditions, all three accounting domains, the value gate, sampled triangle
law, richer-equivalent cost, and exact universal projection.

`existence_without_uniqueness_and_unsafe_controls_are_rejected` covers the
hidden-tag counterexample, swapped observers, wrong generators, collapsed and
divergent carriers, size bounds, and insufficient curricula.

`leakage_is_removed_before_candidate_generation_and_accounting` injects
target-derived, output-derived, protected, and post-freeze records before
discovery and proves the frozen terms, accounting, and charge remain exact.

`bounded_enumeration_returns_all_sizes_deterministically` verifies deterministic
typed enumeration and explicit cap-truncation reporting used by the uniqueness
claim. Existing universal-schedule regressions prove every sampled finite
syntax/fuel pair retains a finite interleaved index.

## Remaining boundary

The object colors, lambda substrate, typed interfaces, observational probes,
normalizer, size/fuel/cell bounds, cost prices, equality rule, freeze protocol,
priority score, reuse horizon, and experimental curriculum remain supplied. U1
discovers one product-like structure inside that world. It does not discover
arbitrary categories, infer a uniquely correct latent representation, prove an
unbounded universal property, establish general theorem proving, or show
practical/statistical superiority outside the declared experiments.
