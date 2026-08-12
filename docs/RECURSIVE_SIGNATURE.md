# U4: recursive-signature ontogenesis

U4 removes U3's supplied `F(X)=1+X`. It exhaustively enumerates an anonymous
polynomial AST (`U`, `R`, `P0`, `S`, `T`) through syntax size 5, derives each
candidate's variant/field profile and executable action uniformly, and searches
for Church constructors, `F(M)->M`, and a generic mediator generator. No search
branch recognizes the intended profile or uses datatype names.

## Frozen protocol

The primary world contains Boolean parity, Church-numeral counting, and
Church-list reconstruction. Training and calibration use depths 0--4. Protected
odd-parity and double-count algebras use depths 5, 7, and 9 and are excluded by
identity, duplicate group, ancestry, target/output/trace flags, epoch, and freeze.

The weak curriculum observes only a nullary variant and is explicitly incomplete.
It leaves 13 semantic signature classes and U4 returns `Ambiguous`. The rich
curriculum additionally observes one recursively unary variant and declares the
observed variant inventory complete. Of 237 syntax candidates, it leaves one
bounded semantic profile:

```text
[(parameters=0, recursive=0), (parameters=0, recursive=1)]
```

That profile has 12 syntax aliases at the bound. U4 reports all aliases as one
class; it does not pretend syntax uniquely identifies a functor.

For this class the independent searches produce:

```text
constructors       λa.λb.a
                   λa.λb.λc.c(a(b,c))
alpha              λa.a(λb.λc.b, λb.λc.λd.d(b(c,d)))
mediator generator λa.λb.λc.c(a,b)
action             λa.λb.λc.λd.b(c, λe.d(a(e)))
```

Recursive constructor children are stored extensionally through the same
anonymous handler interface. This is important: one-layer tag/field probes admit
false carriers that fail beyond depth one. The constructor law and algebra tests
therefore exercise nested children.

Protected equations commute. Independent typed mediator enumeration is
untruncated and all valid mediators collapse to one semantic class after checking
every protected subtree, not merely the protected roots.

## Controls and boundaries

- `1`, `X`, and the binary-recursive profile do not match complete unary evidence.
  Binary probes expose the second child rather than accepting an unused branch.
- Weak evidence returns ambiguity instead of selecting the first/smallest syntax.
- Syntax aliases are grouped by an observational polynomial profile.
- A one-cell typed cap surfaces constructor, generator, or mediator truncation and
  blocks discovery. Signature enumeration is the exact finite `size <= 5` set and
  therefore has no proposal cap.
- Protected annotation mutation and injected target/output/trace/ancestry,
  post-freeze, or duplicate evidence leave candidate order, classing, programs,
  accounting, and ranking unchanged.
- Projecting learned work from the interleaved resource schedule reproduces the
  original universal dovetail exactly.
- The U3 hidden disconnected-chain result remains the non-initial-carrier control:
  existence holds while semantic uniqueness fails. U4 uses the same bounded
  uniqueness criterion as its discovery gate.
- The same signature enumerator selects the binary profile under a complete binary
  curriculum. Joint exhaustive binary carrier/mediator discovery is not claimed;
  it exceeds this experiment's small practical boundary.

The complete variant inventory is observational input. U4 does not infer a raw
datatype from bytes, prove a unique latent encoding, or search arbitrary
polynomial functors.

## Economics

Comparable U4 discovery charges include signature/action enumeration, constructor,
`alpha`, generator, equation, mediator, equivalence, and installed-program
complexity work. Downstream carrier doubling takes 12 learned proposals, 15 under
uniform installed allocation, 1 for the external oracle, and 707 for the bounded
pure-universal condition. An irrelevant atom does not solve. Supplying `F` as in
U3 also takes 12 downstream proposals: inventing the signature is honestly an
upfront U4 cost, not a manufactured downstream advantage.

The charged discovery cost is 928 comparable units. At the declared 10,000-use
horizon, `(15-12)*10000-928 = 29072` net units. Work from unlike domains is not
converted or aggregated.

## Reproduce

```sh
cargo test --release -p supsearch recursive_signature --lib
cargo run --release --example recursive_signature
cargo test --workspace
```

The example ends with the deterministic `record,experiment=u4,...` line.
