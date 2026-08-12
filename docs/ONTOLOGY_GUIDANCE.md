# Ontology-guided universal recursive search

This experiment connects two previously separate results:

1. B2-general supplies a fair universal hypothesis generator.
2. Ontogenesis supplies acquired concepts that can make useful regions of that
   universal space cheap to reach.

The mechanism does not replace or cap universal enumeration. A finite priority prefix
tests ontology-compressed terms at useful fuel, then `PrioritizedDovetail` resumes the
ordinary `Dovetail` stream exactly. Adding closed ontology atoms also preserves every
pure lambda term in the grammar. The bias therefore changes time-to-first-test without
removing the completeness floor.

## Protocol

All conditions use the same deterministic exact-size grammar, observer, discovery
examples, independent extrapolation examples, fixed-point equation gate, load-bearing
recursion ablation, constant-output control, and evaluation fuel of 100,000.

The reported counters are:

- **proposals**: closed functionals visited in deterministic grammar order;
- **evaluated**: proposals that survive the required recursive-reference prefilter;
- **first-solution rank**: the proposal count when the first admitted law is found;
- **syntax/fuel resources**: the largest priority size and fixed diagnostic fuel;
- **wall time**: an informative, machine-dependent measurement, never a regression
  assertion.

### Stage 1: `O0 -> c1 -> O1 -> c2`

`c1` is Church Boolean negation, independently validated on both Boolean inputs. The
held-out target is a recursive parity interpreter over anonymous chain values. The
values accept a step algebra and recursive result; they do not contain negation or the
target interpreter. Discovery uses depths 0–3 and admission requires unseen depths 5,
7, and 9.

The empty ontology is exhaustively falsified through syntax size 11. The relevant,
irrelevant, and misleading conditions each add exactly one opaque size-1 atom and
search through the compressed target size 7. This separates semantic guidance from
mere alphabet enlargement.

### Stage 2: `O1 -> c2 -> O2 -> c3`

The recursive parity executable actually discovered in Stage 1 becomes `c2`. A
distinct outer recursive law must interpret payloads that are themselves anonymous
parity chains and xor their results. The prior ontology `{not}` is compared with
`{not, discovered-parity}`, `{not, irrelevant}`, and `{not, misleading}`.

An early version of the protocol admitted a size-9 aggregate surrogate that returned
negation rather than interpreting inner chains. Single-payload even/odd holdouts now
falsify that overfit; a regression test preserves this failure mode and its rejection.

## Measured result

One release run on 2026-08-11 produced the following deterministic work counts (wall
times are illustrative):

| Condition | Proposals | Evaluated | Result | Wall time |
|---|---:|---:|---|---:|
| Stage 1: empty `O0`, through size 11 | 41,272 | 28,258 | no solution | 11.380 s |
| Stage 1: `{not}`, through size 7 | 337 | 140 | solution at size 7 | 0.060 s |
| Stage 1: one irrelevant atom | 648 | 266 | no solution | 0.102 s |
| Stage 1: one misleading atom | 648 | 266 | no solution | 0.091 s |
| Stage 2: `{not}`, through size 11 | 162,550 | 84,946 | no solution | 29.196 s |
| Stage 2: `{not, parity}`, through size 7 | 705 | 216 | solution at size 7 | 0.100 s |
| Stage 2: `{not, irrelevant}` | 1,711 | 482 | no solution | 0.159 s |
| Stage 2: `{not, misleading}` | 1,711 | 482 | no solution | 0.141 s |

The counterfactual acquisition gates therefore observe lower bounds of **122×** fewer
proposals for Stage 1 and **230×** fewer proposals for Stage 2 while the baselines
remain unsolved. On evaluated recursive candidates the separations are over 201× and
393× respectively. Both relevant concepts earn installation; equally sized irrelevant
and misleading additions do not.

The supported developmental claim is:

```text
O0 = {}
  -> independently validate and counterfactually acquire c1 = not
O1 = {not}
  -> discover and acquire c2 = recursive parity
O2 = {not, parity}
  -> discover c3 = nested recursive parity aggregation
```

## Controls and limitations

- Nonrecursive shortcuts, dead recursive references, constant families, open terms,
  incomplete probes, and divergence are rejected by the B2-general validation suite.
- Matched irrelevant and misleading alphabets control for additional branching and
  opaque-atom compression alone.
- The explicit weak-probe surrogate controls aggregate overfit.
- Independent extrapolation is finite falsification, not proof of semantic uniqueness.
- The empty/prior baselines are bounded exhaustive absences through size 11, not claims
  that universal search can never find a solution. The expanded displayed targets have
  sizes 16 and 31 and remain in the unchanged universal fallback.
- Proposal counts depend on the declared grammar order; wall times depend on hardware.
- The experiment demonstrates learned allocation through acquired atoms, not yet a
  learned probabilistic scheduler, active observation selection, or raw-signature
  inference.
- B2-general's qualified completeness theorem and representation-invention boundary
  remain unchanged.

## Reproduce

```sh
cargo run --release --example ontology_guided
cargo test -p supsearch ontology_guidance --lib
cargo test --workspace --no-fail-fast
```
