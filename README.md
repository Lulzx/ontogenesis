# supsearch — ontology bootstrap

**supsearch** is an experiment in a machine that starts with almost no
human-designed concepts and invents its own executable language of thought.

The thesis is Taelin's: a superposition — share work across candidates,
verify against an oracle that cannot be gamed, grow a library of solved
abstractions — should fit in ordinary code on ordinary hardware. The
superposition becomes a hash table keyed on *behavior*; the library is a
list of λ-terms the machine abstracted from its own solved problems.

This repository is that growth track made self-contained. It has **no
semantic vocabulary**: no Church/Scott decoder, no typed DSL, no hand-chosen
operations. The only signal is the oracle — the test inputs of the tasks
it solves. It is ~1,700 lines of Rust, no dependencies.

## The frontier

The falsifiable claim is a cost curve `C(L₀) > C(L₁) > C(L₂)…` on
**held-out** tasks: as the machine abstracts and re-injects its own
abstractions, solving new tasks should get cheaper. The honest framing is
stated plainly, not elided:

- **The input-domain commitment is unavoidable.** The oracle passes raw
  λ-terms, so *something* is the input universe. supsearch commits only to
  "the corpus test inputs plus Church/Scott data and functions as *input
  generators*" — zero operations, zero decode, zero output typing. That is
  "start with almost no human concepts," not "from literally nothing."
- **There is no reference ontology to validate against.** Validation
  therefore measures *generality* — defined-rate and self-consistency across
  a broad held-out probe draw — and **generality ≠ truth**. Each promoted
  seed carries that caveat in its note.
- **A bad seed can never corrupt a solution.** `bank::solve` re-verifies
  every winning candidate's normal form against the oracle, so an overfit
  seed is a *performance* hazard, never a *correctness* one.

## How it works

1. **Raw-λ search** (`bank.rs`). Candidate programs are enumerated bottom-up
   and deduplicated by their behavior on the test inputs (a hash table keyed
   on normal-form vectors — the superposition). No decode, no typed ops.
2. **The miner** (`bootstrap.rs`). From the solutions the bank finds, it
   extracts *open* subterms, abstracts them into closed combinators, and
   **groups behaviorally**: syntactically distinct combinators that behave
   identically merge into one class (semantic abstraction over raw λ). It
   ranks classes by compression gain and validates each for generality on a
   fresh holdout probe universe before promoting it as a seed.
3. **The grow driver** (`bootstrap` subcommand). Solve train → measure
   held-out cost → mine → validate → inject seeds → repeat to a fixed point,
   recording `C(Lₜ)` at each generation.
4. **The benchmark** (`mkbench` subcommand). The repo vendors the verified
   round-0 *solutions* but not the external benchmark's task files, so a
   valid corpus is synthesized from the solutions themselves (see below).

```
cargo build --release

# 1. (re)generate the benchmark corpus from the verified round-0 solutions
./target/release/supsearch mkbench solutions/round0 bench

# 2. run the Milestone-0 split: mine on 4, measure the curve on 5
./target/release/supsearch bootstrap bench \
  --train    clst_fol,clst_hed,clst_map,cnat_mul \
  --holdout  cnat_add,cnat_exp,ctre_rev,ntup_hed,slst_hed \
  --rounds 3 --budget 20 --lib lib/bootstrap.lib
```

### The synthesized benchmark (honesty note)

`mkbench` writes one `.tsk` per solution. Every task test has the shape
`λA₁…λAₖ. @main(A₁,…,Aₖ)` — apply the program to `k` fresh binders — so a
valid task reconstructs from the verified solution alone: the test is the
solution's binder-head, the expected output is the solution itself. **These
are synthesized single-probe tasks, NOT the real external benchmark with its
rich concrete input suites.** They exercise the full driver loop faithfully
to the Milestone-0 task *set*, and each file's header says so. The cost curve
is meaningful only relative to this corpus, not as a claim about the external
benchmark.

## Where the frontier stands (verified, honest)

- The **mechanism works**: on the 9-solution corpus the miner fires — it
  mines the head idioms `λa.a(λb.λc.b)` (gain 4) and `λa.a(λb.λc.b, a)`
  (gain 3), validates them on the holdout probe draw (G 100% / H 100%), and
  promotes them. A unit test (`mined_seed_is_executable_succ`) proves a
  mined abstraction actually computes successor on Church numerals.
- The **4-task Milestone-0 split is too thin**: with only four solutions the
  best recurring behavioral class (composition `λa.λb.λc.a(b(c))`) recurs
  twice but has gain 0, below the bar, so no seeds and a flat curve. Set
  `BOOT_DEBUG=1` to see exactly why — this is the plan's anticipated
  thin-corpus fallback, diagnosable instead of silent.
- **Naive seed injection widens search.** Seeds are injected as size-1
  atoms, so a promoted seed that isn't a subterm of the target adds junk
  branching — median cost *rose* 0.016s → 0.265s in one run. This is the
  plan's Risk #3 (seed branching explosion), the same lesson the frozen
  engine recorded in `lib/small.lib`. The lever that changes the curve is
  more raw solves → larger recurring idioms that pre-build structure the
  bank can't cheaply enumerate.

`cargo test` passes (16 tests, incl. the safety property that bad seeds
cannot corrupt answers and the executable-succ proof).

## Layout

```
src/             the ontology-bootstrap track (live)
  bank.rs        raw-λ search: behavior-keyed superposition, oracle verify
  bootstrap.rs   miner: open-subterm abstraction, behavioral grouping,
                 reference-free generality validation, probe universes
  nbe.rs         normalizer / evaluator
  term.rs        λ-terms, de Bruijn, printing
  parse.rs       .tsk task parser
  legacy/        the frozen 120/120 semantic engine (sem, decode, dsl, compile)
bench/           synthesized Milestone-0 corpus (.tsk)
lib/             mined seed libraries
legacy/          frozen engine outputs: outsem/, final/, out2/, solutions/
                 semantic, RESULTS.md, certify.sh
```

## Legacy: the 120/120 semantic engine

Before the ontology-bootstrap pivot, supsearch solved **all 120 LamBench
tasks** with a hand-built semantic vocabulary — a typed DSL (~70 operations)
over decoded Church/Scott values, compiled through a hand-written λ standard
library. That work is frozen (not developed further) and lives in
[`legacy/`](legacy/RESULTS.md): `src/legacy/` holds the code (still
compilable; the `sem`/`grow`/`mine`/`validate` subcommands still run), and
`legacy/outsem/`, `legacy/final/`, `legacy/solutions_semantic/` hold the
outputs and the certified run.

Its 9→120 score jump is the payoff of that human vocabulary, not an invention
of it — which is exactly why the frontier exists. The raw-λ bank, with no
vocabulary, solved 9/120 on its own; the bootstrap track asks whether
behavioral quotienting + compression mining can re-derive the useful
abstractions from those 9 using only the oracle.

## License

MIT.
