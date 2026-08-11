# Ontogenesis B1–B3 milestone contract

This file freezes what each milestone proves, the evidence required to reproduce it,
and what remains outside the claim.

## B1 — nonrecursive abstraction invention

Question: can generic syntax machinery turn reusable structure inside one raw-discovered
program into a primitive that improves future reasoning?

Protocol:

1. Start with the computational substrate `{cons,nil}` and an empty learned ontology.
2. Raw-solve duplication of singleton rows.
3. Enumerate repeated subterms and factor each into a closed context abstraction.
4. Require the original and rewritten programs to solve a disjoint semantic suite at
   widths 2 and 3.
5. Measure acquisition only on held-out widths 4 and 7.

Evidence: `arc1 b1` reports one valid abstraction and a frontier gain `✗ → 2`.
The regression requires the raw winner to use the actual `cons` primitive, rejecting
the earlier degenerate-lambda failure mode.

## B2 — executable recurrence induction

Question: can the system infer an executable recursive law from finite programs it
discovered itself?

Protocol:

1. Independently raw-solve closed instances at depths 1, 2, and 3. Each instance
   supplies fresh endofunction atoms and a tail as its data; these are symbolized after
   discovery and are not retained ontology concepts.
2. Replace only the instance atoms with stable symbolic inputs and normalize.
3. Infer an arbitrary two-hole syntax context `C` satisfying
   `q[n+1] = C(head[n+1], shift(q[n]))` for every adjacent pair.
4. Recover the base by matching the same context against `q1`.
5. Require exact reconstruction of all observed unrollings.
6. Compile the equation for Church-encoded input lists.
7. Test depths 5, 7, and 9 using new function atoms and different tail widths.
8. Run the counterfactual acquisition gate on a separate future task.

Observed evidence:

```text
q1 = #0(#z)
q2 = #0(#1(#z))
q3 = #0(#1(#2(#z)))
law validation                         ✓
semantic extrapolation at 5,7,9       ✓
future reasoning                      ✗ → 7  ACQUIRE
```

The inducer rejects laws that are constant, ignore the head, ignore the recursive
result, change context by depth, use a lookup-like exceptional unrolling, or merely
insert a beta-redex that happens to be observationally equivalent.

Boundary: this is exact first-order structural right-recurrence induction. The
equation compiler targets the existing Church-list representation. It is not evidence
for arbitrary Y-combinator synthesis, unknown constructors, mutual recursion, or
recurrences whose previous computation is not an exact embedded subterm after
normalization.

## B3 — recursive law to reasoning vocabulary

Question: does reifying the B2 recursion scheme change which higher concepts are
generable and useful?

Protocol:

1. Promote the B2 structural recursion scheme to an ontology atom.
2. Run one generic simply-typed beta-normal enumerator. Its grammar has variables,
   lambda, application, and currently available typed atoms; it has no named
   operation productions.
3. Semantically gate candidates on multi-example map, append, and reverse tasks.
4. Compare the identical generator with and without the recursion atom.
5. Counterfactually acquire the resulting concepts.
6. Compose invented `map` and `reverse` and transfer to a held-out grid.

Observed evidence:

```text
                         {cons,nil}    + invented recursion
map (size 11)            absent        reachable
append (size 9)          absent        reachable
reverse (size 14)        absent        reachable

map acquisition                         ✗ → 15
append acquisition                      ✗ → 15
reverse acquisition                     ✗ → 3
map(reverse), unseen 5×4 grid mirror    ✓
```

Honest control: `append` is useful vocabulary but is not load-bearing for reverse in
this search space; recursion can inline an append-like helper. The supported claim is
`R → {map, append, reverse}`, not `R → append → reverse`.

All B3 absence claims are bounded-search claims: the enumerator uses the displayed
maximum term sizes and a 50,000-candidate cap per memoized type/context/size cell.

## Reproduction

```sh
cargo run -p arc1 -- b1
cargo run -p arc1 -- b2
cargo run -p arc1 -- b3
cargo test --workspace
```
