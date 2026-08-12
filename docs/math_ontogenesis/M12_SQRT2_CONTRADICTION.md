# M12 — Invent Contradiction for sqrt(2)

## Result

M12 is reached. From an assumed integer-ratio witness `sqrt(d)=p/q`, the
system rewrites to `p²=dq²` and searches for an intermediate representation.
At candidate 14 it invents prime-exponent count modulo 2. For `d=2`, the
exponent of prime 2 is even on the left and odd on the right. A separate checker
validates the contradiction and discharges the rational-witness assumption.

This is structurally equivalent to the classical parity argument but more
general: prime-valuation parity proves every nonsquare integer radicand
irrational without choosing a lowest-terms representative.

## Representation search

The search does not receive parity or 2-adic valuation as one candidate. It
combines two fixed generic axes:

- measures: magnitude, sign, divisibility, common-factor structure, and
  prime-exponent count;
- quotients: exact value, modulo 2, and modulo 3.

The winning composition is `prime_exponent_count ∘ modulo 2`. The checker
ignores this strategy label and accepts only an explicit derivation.

## Checked contradiction

For radicand `d` and obstruction prime `r`, the checker independently verifies
that `r` is prime and `v_r(d)` is odd. The proof then checks:

1. assume integers `p,q`, `q != 0`, witness `sqrt(d)=p/q`;
2. derive `p²=dq²`;
3. select the odd-valuation prime `r`;
4. use additivity of valuations on products;
5. use that every square has even valuation;
6. derive the impossible equality even = odd;
7. discharge the assumption by contradiction.

Equivalently, `2v_r(p)=v_r(d)+2v_r(q)` has even left side and odd right side.
This quantifies over arbitrary integer witnesses, not bounded denominators.

## Transfer, controls, and costs

- Transfers prove radicands `3,5,6,7,10,12` irrational.
- Perfect squares `1,4,9,16,25,36` yield no contradiction; extended tests check
  squares through 100.
- Composite and even-valuation obstruction witnesses are rejected.
- Removing a derivation step is rejected.
- Changing only the strategy label does not affect checking.
- Search and records are deterministic.

Discovery costs 14 candidates. Direct reasoning falls from 21 to 7. Seven
separate seven-step proofs require 49 tokens; one retained schema plus seven
witnesses requires 14, compression gain 35. Status is
`formally_checked_valuation_contradiction`, formal within this fragment rather
than a general number-theory proof assistant.

## Next boundary

M13 requires unordered root sets, multivariable symbolic polynomial
normalization, and discovery of coefficient/root invariants.

## Reproduce

```sh
cargo test -p supsearch --lib irrational_world
cargo run --release --example irrational_world
cargo test --workspace
```
