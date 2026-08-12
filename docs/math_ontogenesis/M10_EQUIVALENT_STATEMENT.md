# M10 — Invent an Equivalent Statement

## Result

M10 is reached. Starting from the universally interpreted predicate

```text
2|n iff 2|(n*n)
```

the system discovers the smaller theorem

```text
2|((n*n)+n)
```

or: the product of two consecutive integers is even. The alternative is not
listed in the experiment driver and is selected by a fixed answer-blind
proposal order plus independently checked proof and cost criteria.

## New proposition world

`src/proposition_world.rs` introduces a typed integer-polynomial expression AST
and a proposition AST containing equality, divisibility, conjunction,
implication, biconditional, and negation. A free integer variable is interpreted
universally at theorem-checking boundaries. Proof terms include assumptions,
implication introduction/elimination, conjunction and biconditional rules,
exact arithmetic normalization, and modular certificates.

The checker is separate from proposal generation. For a proposition built from
integer polynomials and divisibility modulo `m`, truth depends only on the
input residue modulo the least common multiple of its moduli. A modular
certificate is accepted only if the checker independently recomputes that
canonical period and verifies every residue. Thus the proof covers all
integers; it is not extrapolation from examples.

## Search

The input equivalence supplies two arithmetic expressions, `n` and `n*n`, but
not a replacement theorem. The proposer applies the same fixed grammar to any
such pair: sum, either difference, then product. Each resulting divisibility
predicate must satisfy all of the following:

1. the checker accepts the implication from the original;
2. the checker accepts the implication back to the original;
3. the checker accepts the alternative theorem itself;
4. its direct proof cost is below the original proof cost;
5. its syntax is smaller.

The first candidate, `2|((n*n)+n)`, passes. Discovery cost is one proposal.

## Costs and transfer

The original biconditional has checked modular proof cost 37. The discovered
single divisibility predicate costs 14, lowering downstream theorem reasoning
by 23 units. Its proposition syntax is two nodes smaller. Forward and backward
certificates are separately checked; together they cost 53 units and are
reported as the one-time transfer/equivalence cost rather than hidden.

On a new modulus and degree, the exact same proposal search starts from
`3|n iff 3|n³`. The addition proposal fails, after which it discovers and
checks `3|(n³-n)`. This transfer is not used to choose the M10 result.

## Controls and precise limits

- A forged certificate for `2|n` is rejected.
- A polynomial that vanishes only at sampled integers `0..4` passes those five
  observations but receives no universal proof.
- Generic polynomial normalization is tested independently of parity.
- Repeated runs produce identical candidates, proofs, costs, and records.

The proof status is `formally_checked_modular`: formal relative to this small,
auditable decision procedure. It is not a general first-order proof kernel.
The current AST has implicit universal closure of its one integer variable; it
does not yet express existential witnesses, finite collections, factorization
lemmas, or arbitrary quantified domains. Those missing objects define M11's
next boundary.

## Reproduce

```sh
cargo test -p supsearch --lib proposition_world
cargo run --release --example proposition_world
cargo test --workspace
```
