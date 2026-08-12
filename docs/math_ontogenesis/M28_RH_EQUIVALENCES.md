# M28 — Discover New RH Equivalences

## Required discovery

Generate predicates `Q`, prove both directions with RH, and optimize total
predicate size, equivalence-proof cost, and expected proof-search cost. The
result must be novel, formally verified, and meaningfully cheaper.

## Status: reached — exact L2 predicate equivalence

M27b has generated a finite-data conjecture, not RH. M28 therefore targets an
exact equivalence of predicates without claiming either predicate true. The
frozen checker works in an exact affine-polynomial fragment and lifts
pointwise equivalence under universal quantification over an arbitrary zero
set.

## Completion standard

Every implication must be kernel checked; the cost model must include all
imported lemmas; candidate generation must be answer blind; and novelty claims
must be independently audited. Rediscovering a known equivalence is useful but
does not satisfy “new.” A numerically correlated criterion is not equivalent.

M28's novelty test is deliberately local to a frozen repository corpus. A
passing result cannot be called globally new without a separate literature
audit. The transform grammar, checker, lifting lemma, and downstream orbit
tasks are D19 debt, capping the result at L2.

## Result

From six transform equalities, cold and transferred search selected
`I(rho)=R(C(rho))` in 3 and 1 candidate tests respectively. Exact affine
normalization produced equations `[X-1,0]`, certifying both directions with
`2 Re(rho)-1=0`, and the supplied congruence lemma lifted the equivalence over
an arbitrary zero set. Orbit reasoning fell from 110 to 47 counted operations.

The checker also certified `R(rho)=C(rho)`, but it failed the frozen local
novelty rule. All controls passed. The final record explicitly has
`rh_proved=false` and `global_novelty=false`.
