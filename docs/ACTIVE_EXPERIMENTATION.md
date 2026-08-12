# Direction E: active experimentation and the crucial experiment

Probe invention (Direction D) lets the agent choose which *input* to measure.
Active experimentation goes one step further: the agent chooses which *action*
to perform — an intervention, not just a passive reading. The decisive case is
the **crucial experiment** (§8.1):

> Two environments are observationally identical under all passive data, yet
> differ under one intervention. A passive learner cannot distinguish them. An
> active learner chooses the intervention, observes the result, and revises its
> ontology.

## Domain

A small, deterministic, domain-neutral machine with a 2-bit input `x in 0..3`
and a boolean output. The world is a hidden function `f: input -> output`. A
hypothesis is a candidate function; the pool is all 16 boolean functions of two
bits.

- **Passive input**: `x=0` is the default, observed for free.
- **Interventions**: setting `x` to 1, 2, or 3, each at unit cost.
- The agent maintains every hypothesis consistent with the observations so far
  and chooses the intervention that most reduces uncertainty, without ever
  seeing the answer in advance.

## Selection rule (§8.1 analogue)

```text
value(x) = expected_hypothesis_reduction(x) - ACTION_COST
         = min(#candidates_true(x), #candidates_false(x)) - 1
```

The agent picks the intervention with the highest value, runs it, prunes
inconsistent hypotheses, and repeats until one hypothesis remains or no
intervention separates the survivors.

## Observed result

World truth `y = (x == 1)`, passive observation `x=0 -> false`:

```text
hypothesis pool=16  consistent with passive data=8
passive_final_candidates=8  passive_distinguished=false
intervene x=1 (gain=4, value=3) -> observed=true   8 -> 4
intervene x=2 (gain=2, value=1) -> observed=false  4 -> 2
intervene x=3 (gain=1, value=0) -> observed=false  2 -> 1
active_final_candidates=1  actions_taken=3  total_action_cost=3  total_information=7
active_true_recovered=true  crucial_action=1
```

The passive learner ends with 8 candidates and cannot tell them apart; the
active learner's first intervention (`x=1`) is the crucial experiment that
reveals the world, and two further checks confirm it — 3 actions, cost 3,
information 7, exact recovery.

## Controls and honest fine print

- **Crucial-experiment baseline**: the passive baseline is computed in the same
  module and reported (`passive_distinguished=false`); the active learner's
  success is only meaningful because passive data provably cannot resolve the
  world.
- **Answer-blind selection**: intervention choice depends only on the candidate
  functions' predicted outputs, never on the hidden truth. A dedicated test
  asserts the value of every single-bit probe is exactly the balanced-split
  gain minus cost.
- Deterministic construction and machine records (`deterministic=true`).
- The exact universal fallback and frozen evaluation are preserved.
- Action cost is a declared comparable unit; no claim is made that the greedy
  one-step-ahead choice is globally optimal.

## Claim and limits

Supported claim:

> An agent with a bounded set of candidate world-models can choose an
> intervention that distinguishes environments which are observationally
> identical under all passive data, observe the result, and recover the true
> model — succeeding where a purely passive learner provably cannot.

Limits: the action set (setting a 2-bit input) and the hypothesis pool are
supplied; the world is a fixed function rather than a causal system with
feedback or hidden state; the choice is greedy (one action at a time) rather
than a full multi-step experiment plan; there is no notion of risk or a
partially observed state.

## Reproduce

```sh
cargo test --release -p supsearch active_experimentation --lib
cargo run --release --example active_experimentation
cargo test --workspace
```

The example ends with `experiment=active_experimentation,deterministic=true,...`
machine output.
