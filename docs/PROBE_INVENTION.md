# Direction D: invent observational probes

A major remaining source of human ontology is the probe set: humans choose which
observations make structural distinctions visible. This experiment closes the loop

```text
ontology -> predict what evidence would distinguish competitors
         -> generate probe -> receive result -> revise ontology
```

without ever handing the agent the hidden answer.

## The §7.2 experiment

The agent holds a bounded set of **candidate hypotheses** — executable
predicates over a finite 4-bit input family (`tokens 0..15`), each given by its
accepted extension with an executable Church-witness meaning. Two (or more)
candidates agree on every **already-measured** probe; they are observationally
equivalent on the current evidence. Yet they differ on some unmeasured input.

The agent must *invent the probe* — decide which input to measure — that
separates them, then run it, receive the true label, and prune the candidate
set. The loop repeats until one hypothesis survives or the survivors agree on
every unmeasured input.

## Probe language and scoring (§7.1)

The predicate language is a bounded boolean grammar over the four bit features
(leaves, their negations, and depth-2 `and`/`or`), deduplicated by extension —
58 distinct executable predicates.

Probe scoring is the deterministic bounded equivalent of
`ProbeValue = ExpectedHypothesisReduction − ProbeExecutionCost`:

```text
gain(t) = min(#candidates_true(t), #candidates_false(t))   // balanced split
value(t) = gain(t) − PROBE_EXECUTION_COST
```

A probe that does not separate any candidate pair (all candidates agree on `t`)
has negative value and is never selected.

## Controls (§7.3 and adversarial)

- Probe selection reads **only the candidate models' extensions**, never the
  world truth or any candidate id. A dedicated test asserts the chosen probe is
  a genuinely separating, unmeasured token with positive value.
- If the surviving candidates are observationally equivalent under the whole
  probe language (they agree on every unmeasured input), the agent correctly
  stops and reports `observationally_equivalent_stuck=true` rather than
  inventing a fabricated distinction.
- Construction is fully deterministic; the machine record is reproducible.
- The exact universal fallback and frozen evaluation are preserved.

## Observed result

With world truth "bit 1 set" and one measured probe (`token 2 ∈ set`):

```text
predicate language size=58, consistent candidates=29
probe token 5  -> label=false  29 -> 15
probe token 8  -> label=false  15 ->  6
probe token 10 -> label=true    6 ->  3
probe token 3  -> label=true    3 ->  2
probe token 6  -> label=true    2 ->  1
probes_run=5  probes_that_reduced=5  total_probe_cost=5  total_information=25
true_recovered=true  observationally_equivalent_stuck=false
```

The agent discovered five measurements that, taken together, single out the
world truth from 29 initially equivalent hypotheses, at a total cost of 5
measurement units and 25 units of gained information.

## Claim and limits

Supported claim:

> An agent holding hypotheses that are observationally equivalent on current
> evidence can invent a bounded probe that separates them, purely from the
> differences between the candidate models, and can prune to the world truth —
> while correctly declining to invent a probe when the survivors are truly
> indistinguishable under its probe language.

Limits: the probe language and input features are still supplied (the agent does
not yet choose its own measurement vocabulary); hypotheses are static predicates
rather than full ontologies revised by the U6 runner; scoring is greedy and
one-step-ahead rather than a full information-gain plan; and the world is a fixed
hidden predicate rather than a causal system with interventions.

## Reproduce

```sh
cargo test --release -p supsearch probe_invention --lib
cargo run --release --example probe_invention
cargo test --workspace
```

The example ends with `experiment=probe_invention,deterministic=true,...`
machine output.
