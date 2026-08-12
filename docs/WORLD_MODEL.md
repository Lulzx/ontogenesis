# Direction G: world-model ontogenesis

Directions D–F moved from isolated task families toward *interventions* and
*causal structure*, but each still treated a single task in isolation. This
milestone moves to **persistent environments**: tiny deterministic worlds with
state, actions, observations, hidden structure, and repeated episodes. The
learner begins with minimal assumptions — it can observe
`(state, action) -> next_state` transitions — and must **invent a compressed
representation of the world** that reduces future reasoning cost: prediction,
planning, and transfer. No concept names are supplied.

The acquisition criterion stays practical:

```text
world-model abstraction retained  iff  it reduces future reasoning cost
```

## Domain

The demo world is a set of `n` independent reversible counters ("switches"),
each toggled by exactly one action; `a == n` is `wait` (no-op). The agent is
never told this. From a bounded set of observed transitions it must:

1. **discover** that each switch's dynamics depend only on itself — a factored
   transition model (the invented concept),
2. use that factorization to predict **held-out** transitions the raw state
   table cannot,
3. **plan** toward a goal in the relevant component instead of the full
   product state space, and
4. **invent** the "reversible counter" concept and transfer it to a new switch
   in a growing world, predicting it from a single probe.

## Algorithm

1. **Factor discovery.** For each output variable `y`, search the smallest
   parent set `P` such that the next value of `y` is a deterministic function of
   `(projection(s, P), action)` over all observed transitions. Smallest parent
   sets first, so the discovered structure is the most compressed one that
   explains the data.
2. **Held-out generalization.** Split all transitions into observed / held-out
   by a deterministic **Gray-code split** (`((gray(state_index) + action) % 4)
   < 2`). The Gray-code term decorrelates the split from any single state bit,
   so the observed half still reveals each switch's self-dependence. Compare
   the factored model against the raw monolithic table (which predicts a
   transition only if that exact `(state, action)` was observed, so it
   generalizes to none of the held-out combos).
3. **Invented switch concept.** Probe each action from a reference state and
   identify which one toggles a given switch (bounded `n+1` probes). The
   resulting "reversible counter" concept predicts the switch's behavior for
   every state/action.
4. **Transfer.** A new switch in a growing world is fully predictable after
   `probe_cost` probes; the raw/cold-start baseline needs to observe every
   `(state, action)` combination that touches it (`2*(n+1)`).
5. **Planning.** Raw BFS over the full product state space vs. factored
   per-component planning (one toggle per differing switch).
6. **Coupled control.** A genuinely coupled world (switch 0 toggles freely;
   every other switch toggles only while switch 0 is set) must *not* over-claim
   compression.

## Observed result

World truth = 3 independent reversible counters:

```text
ontogenesis: world-model ontogenesis (G)
world: 3 independent reversible counters, 4 actions (toggle_i, wait)
observed transitions=16 held-out=16
discovered parent sets (per switch): [[0], [1], [2]]
held-out full-state prediction: factored 16/16 (1.000), raw 0/16 (0.000)
invented switch concept: switch 2 toggles on action 2 (probe cost 3)
concept predicts switch-2 behavior: 32/32
transfer to new switch: probe cost 3 vs cold-start 8 (saved 5)
planning to set all 6 switches: raw expansions=58 factored expansions=6
coupled control: parents=[[0], [0, 1], [0, 1]] held-out factored accuracy=0.000 (must be low)
experiment=world_model,nvars=3,observed=16,held=16,factored_parents=[[0], [1], [2]],factored_accuracy=1.0000,raw_accuracy=0.0000,factored_correct=16/16,transfer_probe=3,transfer_cold=8,transfer_saved=5,planning_raw=58,planning_factored=6,partial_parents=[[0], [0, 1], [0, 1]],deterministic=true,fallback=exact
```

The factored model predicts **all 16 held-out** full-state transitions (1.000)
where the raw table predicts **none** (0.000). The invented switch concept
predicts switch-2 behavior exactly (32/32) and transfers to a new switch at
probe cost 3 vs. cold-start 8 (saved 5). Planning to set all 6 switches costs
58 raw expansions vs. 6 factored expansions.

## Controls and honest fine print

- **Raw baseline**: the monolithic table generalizes to none of the held-out
  combos (`raw_accuracy=0.000`), so the factored gain is measured against a
  provably non-generalizing baseline.
- **Coupled control**: the coupled world's discovered parents are
  `[[0], [0, 1], [0, 1]]` — switch 0 is correctly detected as a parent of the
  rest — and its held-out factored accuracy is **0.000**, so the discovery does
  not over-claim compression where the world is genuinely coupled.
- **Answer-blind split**: the observed/held split is deterministic and
  answer-free (Gray-code parity), reproducible across runs.
- **Honest transfer accounting**: transfer saves exactly the difference between
  probe cost and cold-start observations; it never claims a saving where the
  concept is not load-bearing.
- Deterministic construction and machine records (`deterministic=true`); exact
  universal fallback preserved.

## Claim and limits

Supported claim:

> In a persistent deterministic environment, an agent that observes
> `(state, action) -> next_state` transitions can invent latent concepts
> (a factored transition model, a reversible-counter concept) that compress
> history and measurably reduce the cost of predicting held-out states, planning
> toward goals, and transferring to new parts of a growing world — succeeding
> where a raw monolithic state table provably cannot.

Limits: the worlds are tiny, finite, deterministic, and fully enumerable; the
factorization search is exact over a small variable set; no claim of
continuous/latent state, stochastic dynamics, partial observability, or
temporally extended (multi-step) experimental planning. The "reversible
counter" concept is invented within a bounded probe budget and is not claimed
to be the unique latent encoding.

## Reproduce

```sh
cargo test -p supsearch --lib world_model
cargo run --release --example world_model
cargo test --workspace
```

The example ends with
`experiment=world_model,...deterministic=true,fallback=exact` machine output.
