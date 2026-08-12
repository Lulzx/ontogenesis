# M11 — Rediscover Euclid's Proof

## Result

M11 is reached. From finite prime-list observations and a generic
finite-collection expression grammar, the system conjectures that there are
infinitely many primes and invents the auxiliary object

```text
product(xs) + 1
```

at proposal 19. A separate symbolic checker accepts a proof certificate that
this construction produces a prime outside every arbitrary nonempty finite list
of primes. The construction is not supplied as an atom, proposal template, or
checker pattern.

## Supplied world

The candidate language contains the generic collection folds `product`, `sum`,
and `length`; constants `0,1,2`; and arithmetic `+,-,*`. Candidates of size one
and size three are enumerated deterministically in one fixed order. The input
consists of four training prime lists; three different prime lists are held out.

The trusted proof substrate contains generic facts rather than Euclid's
construction:

1. each collection member divides the collection product;
2. a nonempty prime-list product is at least two;
3. every integer greater than one has a prime divisor;
4. a prime divisor that is not any listed member lies outside the list.

## How the construction was selected

Finite evidence first rejects candidates that fail to produce an integer
greater than one with no listed prime divisor. Passing finite examples is not
enough. The candidate must then receive an arbitrary-list certificate from the
independent checker.

The checker converts the collection expression into a polynomial over the
abstract quantities `product(xs)`, `sum(xs)`, and `length(xs)`. For an arbitrary
listed member `p`, it substitutes `product(xs)=0 mod p`. It accepts only when
the remaining expression is universally the constant `1` or `-1`, proving
that no listed `p>1` divides the construction. Separate interval reasoning must
prove the construction is greater than one for every nonempty prime list.

For `(product(xs)+1)`, both obligations hold. The generic prime-divisor lemma
then introduces a prime witness `q` dividing the constructed integer. Since no
listed prime divides it, `q` is outside the list. Therefore every finite prime
list omits a prime, which proves there are infinitely many primes.

Crucially, the checker never asks whether the syntax is `product+1`; it checks
only these generic symbolic consequences. This prevents the proof checker from
leaking the target auxiliary object into search.

## Evidence, transfer, and costs

- Discovery cost: 19 enumerated auxiliary expressions.
- Held-out: three unseen prime lists pass.
- Transfer: the same construction escapes three lists of composite divisors,
  showing the lower-level “escape listed divisors” idea is not prime-specific.
- Baseline reasoning cost: 31 units (rediscovery plus held-out obligations).
- Acquired-concept reasoning cost: 12 units.
- Compression gain: six tokens across the training observations.
- Proof status: `formally_checked_finite_list_schema`.

## Controls and limits

- Including the singleton list `[2]` rejects `product(xs)-1`, which can look
  successful on larger prime-prefix examples.
- Non-prime data cannot be used to claim the prime-list theorem.
- Removing a derivation step causes the checker to reject the certificate.
- Search and machine records are deterministic.

The proof is formal relative to the explicitly implemented finite-list schema
and the trusted elementary prime-divisor lemma. It is not unrestricted
first-order arithmetic. The checker has no rational-number normalization,
lowest-terms witnesses, or contradiction certificates; those define M12's
next boundary.

## Reproduce

```sh
cargo test -p supsearch --lib euclid_world
cargo run --release --example euclid_world
cargo test --workspace
```
