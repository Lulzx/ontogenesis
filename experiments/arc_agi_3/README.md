# ARC-AGI-3 ontogenesis experiment

This is a bounded first attempt at applying Ontogenesis to the official
interactive ARC-AGI-3 interface. It is deliberately a frame-only agent: it
does not inspect environment source, game IDs, titles, tags, human baselines,
or solution recordings.

The acquired ontology is intentionally small:

1. intervene once with each opaque simple action;
2. infer translation laws from changed pixels;
3. quotient the 64x64 frame into a movable object and stride-sized cells;
4. propose rare visual components as causal modifiers or goal analogues;
5. plan shortest paths through the observed floor geometry;
6. retain deterministic fallback actions when the learned quotient is
   insufficient.

The second rung additionally transfers frozen action laws between levels,
extracts a visual latent `(color, 3x3 pattern)` state, discovers rotation and
color/shape modifiers from their effects, recognizes repeated objects as
resource equivalence classes, and reads the visible action-budget bar.

The current rung adds episode-level negative credit and product-state planning
over `(position, latent signature, resource classes, consumed resource
instances, visible budget)`. Status operators retain observed transition
functions instead of overwriting one `before -> after` pair, so cyclic effects
remain executable. A verified nonlocal edge is represented concretely and,
when supported by repeated evidence, by an entry-cell, visual-tile, or local
transition-basin schema. Basin predictions interpolate observed gaps and
extend only one lattice step beyond adjacent equal outcomes. Information
frontiers test unobserved basin-adjacent entries only on an incentive-bearing
goal route and only after episode-level negative evidence.

Run against the official toolkit (Python 3.12):

```bash
uv run --with arc-agi --python 3.12 \
  python experiments/arc_agi_3/run.py --game ls20 --max-actions 80
```

`ontogenesis.py` also defines the official starter's required `MyAgent`
subclass. It is self-contained because the Kaggle builder copies exactly one
`agent/my_agent.py` into its offline submission notebook. The adapter consumes
`latest_frame.available_actions`, returns `RESET` for initial and game-over
states, preserves acquired laws across level resets, and emits exactly one
valid action per turn. Coordinate-only environments receive deterministic
visual-object interventions through `ACTION6`.

For harness acceptance, copy that file into an unmodified checkout of the
[official Kaggle starter](https://github.com/arcprize/ARC-AGI-3-Kaggle-Starter)
as `agent/my_agent.py`, then run `make setup` and `make play-local`. The custom
runner below is useful for inspecting the low-level toolkit protocol, but is
not competition acceptance:

```bash
uv run --with arc-agi --python 3.12 \
  python experiments/arc_agi_3/run.py --competition --game ls20 \
  --max-actions 1000
```

That path uses API-only interaction, creates one environment instance, permits
only level resets, never reads an in-flight scorecard, and closes the single
scorecard once at the end. Local normal-mode runs are development diagnostics
only. Direct environment internals, source inspection, level selection, and
isolated-level results are excluded from solver inputs and acceptance claims.

Run the dependency-light inference tests:

```bash
uv run --with numpy --python 3.12 \
  python -m unittest discover -s experiments/arc_agi_3 -p 'test_*.py'
```

The developmental admission gate is a deterministic generated family with
shuffled action and visual bindings. Curriculum and frozen held-out manifests
cover direct goals, resources, recoloring, rotation, key/gate dependencies,
transports, toggles, hazards, and conveyors:

```bash
uv run --with numpy --python 3.12 \
  python experiments/arc_agi_3/evaluate_synthetic.py --split all
```

The frozen result at this checkpoint is `16/16` (curriculum `7/7`, held-out
`9/9`). The transition-basin curriculum case improved from 24 to 21 actions
after admitting local-continuity inference. This is an acquisition gate, not
an ARC-AGI-3 score or a claim that the
generated distribution matches private environments.

Claim boundary: passing a public environment would demonstrate acquisition of
a useful spatial/action ontology on that environment. It would not demonstrate
general ARC-AGI-3 ability. The present rung does not yet quotient animation
sequences, model stochastic transitions, learn robust goal ranking across
levels, or establish transfer to private environments.

The [ARC-AGI-3 technical report](https://arcprize.org/media/ARC_AGI_3_Technical_Report.pdf)
also draws a claim boundary that is stricter than Kaggle packaging: public-set
specialization and ARC-specific human handholding do not count as scientific
official-leaderboard evidence. This experiment therefore contains no game-ID
branches, replay traces, fixed solutions, source inspection, or validation
labels. Kaggle competition results, public diagnostics, and official-model
results are three distinct claims.

## Observed public run

On 2026-08-20, the official Kaggle starter at commit `eeb1535`, toolkit
`arc-agi` 0.9.9, and environment `ls20-9607627b` produced this result for the
current rung:

```text
level 1 completed at cumulative action 17
level 2 completed at cumulative action 64
levels completed: 2 / 7
framework actions: 1001 in 2.79 seconds
local diagnostic scorecard score: 10.714285714285714
acquired laws: ACTION1=(0,-5), ACTION2=(0,5),
               ACTION3=(-5,0), ACTION4=(5,0)
```

This was a normal-mode local diagnostic through the official starter, not a
competition rerun or leaderboard result. The controller solved the first two
levels while paying for four initial interventions and transferred its learned
action laws and reward-linked goal prototype between levels.

It still stalls on level 3. The frame-only trace discovers two refill
instances, a cyclic recoloring operator, rotation, transport hubs, and dozens
of nonlocal transitions; it invokes both product planning and
information-frontier probes. The score did not improve, which bounds the
result: representation reachability is no longer the only wall. The remaining
failure is efficient reward-conditioned arbitration under partially mapped
transition topology. Further mechanisms must improve the frozen generated
gate or a new pre-frozen held-out family before another public-only ranking
change is admitted.
