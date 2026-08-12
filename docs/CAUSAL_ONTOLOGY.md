# Direction F: causal ontology from intervention responses

Current structures are largely observational and compositional. This milestone
moves to **causal** structure: the learner must separate *correlation*,
*mechanism*, and *intervention response* in tiny finite deterministic systems,
and use **interventions** — forcing a variable to a value and reading the
downstream response — to infer executable causal structure, without the graph
names being supplied.

The acquisition criterion stays practical:

```text
causal abstraction retained  iff  it improves prediction/planning under interventions
```

## Domain

Three binary variables `A, B, C`. The world is a hidden directed acyclic graph
with a deterministic boolean function per node (root, copy, not, and, or, xor).
A *causal model* assigns each variable a parent set and a function; the
enumerated model pool is every acyclic 3-variable model over this bounded
function set (94 models).

- **Passive observation** yields the joint set of natural outcomes. Passive
  data alone leaves a whole **Markov-equivalence class** of causal models
  indistinguishable — e.g. the chain `A → B → C` and the fork `A ← B → C` are
  observationally identical (both yield exactly `{000, 111}`).
- **Interventions** — `do(x = v)` — force a variable and read the downstream
  response. They separate structures that passive data cannot.

## Algorithm

The learner maintains every candidate model consistent with the observations
so far.

1. **Passive phase**: keep candidates consistent with the passive joint set.
   Report `passive_candidates` and whether passive data already distinguishes
   the world (`passive_distinguished`).
2. **Intervention phase**: while more than one candidate survives,
   - pick the intervention `(variable, value)` on which the surviving
     candidates *disagree the most* — the largest number of distinct predicted
     intervention-response sets. This is **answer-blind**: it uses only the
     candidate models, never the hidden truth.
   - if every survivor predicts the same response under every available
     intervention, they are observationally indistinguishable; stop.
   - otherwise perform the intervention on the true world, prune candidates
     whose predicted response differs from the observed one, and record the
     step.
3. Report the final candidate count, whether the true model was recovered, and
   whether the survivors collapse to a single structure
   (`causal_structure_identified`).

## Observed result

World truth = the chain `A → B → C` (`B=A`, `C=B`):

```text
model pool (acyclic, bounded functions) = 94
chain passive={ {000, 111} }   fork passive={ {000, 111} }
passively distinguishable? false (Markov-equivalent)

passive_candidates=34  passive_distinguished=false
intervene A=false (gain=27) -> candidates 34 -> 7
intervene B=false (gain=5)  -> candidates  7 -> 2
intervene B=true  (gain=1)  -> candidates  2 -> 1
final_candidates=1  true_recovered=true  causal_structure_identified=true

experiment=causal_ontology,model_pool=94,passive_candidates=34,passive_distinguished=false,
final_candidates=1,true_recovered=true,causal_structure_identified=true,
interventions=[A=false:g27:c34-7,B=false:g5:c7-2,B=true:g1:c2-1],deterministic=true,fallback=exact
```

Passive data leaves 34 candidate causal models; the first intervention
(`A=false`) is the decisive step that halves the ambiguity to 7, and two
further checks pin the pool down to exactly the chain. The fork is not
mistaken for the chain: forcing `A` in the fork leaves `B, C` free, whereas in
the chain they track `A`.

## Controls and honest fine print

- **Passive baseline**: the passive candidate count is computed in the same
  module and reported (`passive_distinguished=false` for the chain); success
  is only meaningful because passive data provably cannot resolve the world.
- **Answer-blind selection**: the intervention chosen depends only on the
  candidate models' predicted response sets, never on the hidden truth. A test
  exercises every model consistent with the passive class and asserts each
  intervention strictly reduces the candidate pool.
- **Honest termination**: when survivors agree on every available intervention,
  the learner stops (it does not fabricate a distinction that no intervention
  can make).
- Deterministic construction and machine records (`deterministic=true`); exact
  universal fallback preserved.
- The intervention set (force one of three binary variables to `0` or `1`) and
  the model pool are supplied; greedy one-step-ahead selection is not claimed
  globally optimal.

## Claim and limits

Supported claim:

> An agent with a bounded set of candidate causal models can use *interventions*
> to separate correlation, mechanism, and intervention response, recovering the
> exact executable causal structure of a tiny deterministic system that passive
> data leaves in a whole Markov-equivalence class — succeeding where a purely
> passive learner provably cannot.

Limits: the graphs and functions come from a small fixed grammar; no claim of
general causal discovery, confounder handling, non-deterministic worlds,
continuous/latent variables, or temporally extended (multi-step) experimental
planning.

## Reproduce

```sh
cargo test -p supsearch --lib causal_ontology
cargo run --release --example causal_ontology
cargo test --workspace
```

The example ends with
`experiment=causal_ontology,...deterministic=true,fallback=exact` machine output.
