# Verification: code vs. the project narrative

**Date:** 2026-08-11 · **Audited baseline:** `0046816` plus the B2/B3 working tree

This file records an audit of the `supsearch` codebase against the written project
summary. It answers, claim by claim, whether the described behavior actually lives in
the code, and where the summary is now out of date.

**Method.** Three read-only exploration passes (repo architecture; the `gen`/`meta`/
`disc` layers; the acquisition ladder and ontology machinery) followed by manual
re-reading of the load-bearing code — the `propose_value`/`Gain::earns` gate, the
`ontogen` matrix, and the `disc` pre-filter. The ontology-dependence matrix was then
reproduced by running the command and locked in with a new regression test (see below).

---

## Verdict table

Legend: **asserted** = locked by a unit test; **printed** = produced at runtime by a
command but not guarded by any assertion.

| Claim in the summary | Status | Where |
|---|---|---|
| `Term::Prim` / quotient map / `concept_solve` substrate | confirmed, asserted | `src/term.rs`, `src/bank.rs:114` (`canonicalize`), `src/bank.rs:533` (`concept_solve`) |
| a×b×c: 17270→17; a×b×c×d: unsolved→99 | confirmed, asserted | `quotient_collapses_search_cost`, `src/main.rs:4487` |
| Acquisition ladder: raw → candidates → counterfactual install → quotient gain → acquire/reject | confirmed | `ladder` `src/main.rs:733`; gate `propose_value` `src/main.rs:3292`, `Gain::earns` `src/main.rs:3354` |
| mul → acquire | confirmed, asserted | `promote_loop`, `src/main.rs:4576` |
| square → reject (on the product family) | confirmed, asserted | `promote_loop`, `src/main.rs:4581` |
| power → acquire | confirmed, **printed** (not asserted) | `ladder` fine print; `ontogen` path |
| mined idiom C1 → reject | confirmed, **printed** (not asserted) | `ladder` candidate loop |
| parity → not found | confirmed (with a caveat, see below) | structural: `src/main.rs:932-938` |
| Ontology-dependence matrix `Gain(c|D,O)` | confirmed, **printed** → **now asserted** | `ontogen` `src/main.rs:1132`; new test `ontology_dependence_matrix` `src/main.rs:4628` |
| Value-representation is not the fold9 wall | confirmed, asserted | `src/canon.rs:200`; `ablation` `src/main.rs:3692` |
| Bottleneck is composition width, not representation; semantic dedup already at admission | confirmed, asserted | `diag` `src/main.rs:3910`; tests `semantic_pruning_*` |
| `gen` (C6): fixed `G(O)=iterate(C,seed)`, ontology fills the concept hole | confirmed | `gen` `src/main.rs:1579` (schema closure 1692-1697) |
| `transfer`: byte-identical G, depth value-space-bound | confirmed | `transfer` `src/main.rs:1868` |
| `meta` (C7): M={iterate,reduce,junk}, retain/drop | confirmed | `meta` `src/main.rs:2124` |
| Cross-schema bootstrap reduce→concat→iterate(concat,nil)→concat_n | confirmed, incl. the negative check | `meta` `src/main.rs:2413-2452` |
| `meta --ablate` Reach synergy matrix | confirmed | `meta --ablate` `src/main.rs:2491` |
| `disc` (C8): discover templates from a lower-level meta-language | confirmed | `MTm` `src/main.rs:2600`; `enumerate_templates` `2687`; prefilter `2951`; parallel `2912` |
| B1: raw program → generic context factorization → unseen-width acquisition | confirmed, asserted | `src/transform.rs`; `b1` and `b1_discovers_factors_and_acquires_row_duplication` in `demo/arc-1/src/main.rs` |
| B2: independently discovered depths 1–3 → exact recurrence → executable extrapolation 5/7/9 | confirmed, asserted | `src/recurrence.rs`; `b2_induces_extrapolates_and_earns_recursive_law` |
| B2 adversarial rejection: constant/headless/tailless/depth-specific/accidental equivalence | confirmed, asserted | `recurrence::tests::rejects_constant_headless_tailless_and_depth_specific_cheats` |
| B2-general: fair closed-functional/fuel enumeration and exhaustive finite-stage recursive discovery with depth-5/7/9 extrapolation | confirmed, asserted | `src/universal.rs`; `recursion_search::tests::finite_fair_stage_actually_discovers_a_recursive_functional` |
| B2-general: pure-lambda single/mutual fixed points, parity, and nested Ackermann | confirmed, asserted | `src/fixpoint.rs` |
| B2-general: semantic recurrence embedding beyond exact normalized subtrees | confirmed, asserted | `recurrence::tests::semantic_induction_crosses_nonidentical_normalized_embedding` |
| B2-general: anonymous representation invention, law probes, recursive tree traversal, and malformed controls | confirmed, asserted | `src/representation.rs` |
| B2-general: Ackermann counterfactual frontier gain and divergent-candidate rejection | confirmed, asserted | `src/recursion_search.rs`; `src/nbe.rs` |
| B2-guided: relevant ontology yields 122× then 230× recursive-discovery proposal gains while matched irrelevant/misleading controls fail | confirmed, asserted | `src/ontology_guidance.rs`; `ONTOLOGY_GUIDANCE.md` |
| B2-context-learned: raw observations → regret-selected `z` → oracle-matching held-out allocation, with leakage and universal-lane controls | confirmed, asserted, executable | `src/learned_context.rs`; `src/contextual_guidance.rs`; `LEARNED_CONTEXT.md` |
| B2-context-learned ARC existence result: learned `z` rank 5, uniform rank 11, protected test verification only | confirmed, asserted, executable | `demo/arc-1/src/main.rs`; `LEARNED_CONTEXT.md` |
| B2-feature-invented: compositional executable predicates beat every raw primitive projection and replay exactly | confirmed, asserted, executable | `src/feature_invention.rs`; `EXECUTABLE_FEATURES.md` |
| B2-feature-invented ARC: 4/4 at aggregate rank 12 from frozen generated experience, interaction ablation 2/4 | confirmed, asserted, executable | `demo/arc-1/src/main.rs`; `EXECUTABLE_FEATURES.md` |
| U1: pure-lambda relational search invents an anonymous carrier, observers, and mediator generator; protected cones commute and have one bounded mediator class | confirmed, asserted, executable | `src/universal_property.rs`; `UNIVERSAL_PROPERTY.md` |
| U1 cost contraction: downstream swap 1 proposal vs raw typed 10/uniform 32/irrelevant 31, charged break-even 378 uses; negative-transfer control preserved | confirmed, asserted, executable | `examples/universal_property.rs`; `UNIVERSAL_PROPERTY.md` |
| U2: independent pure-lambda search invents anonymous embeddings and mediator generator; heterogeneous protected evidence commutes with one untruncated bounded semantic mediator class | confirmed, asserted, executable | `src/coproduct_property.rs`; `COPRODUCT_PROPERTY.md` |
| U2 cost contraction: downstream branchwise S->S takes learned 12 proposals vs uniform 39; raw/irrelevant fail and universal is unsolved after 707; charged net +271412 at 10000 uses | confirmed, asserted, executable | `examples/coproduct_property.rs`; `COPRODUCT_PROPERTY.md` |
| U3: independent discovery for F(X)=1+X yields an anonymous carrier/constructor/mediator generator; protected depth-5/7/9 algebras commute with one untruncated semantic class | confirmed, asserted, executable | `src/initial_algebra.rs`; `INITIAL_ALGEBRA.md` |
| U3 cost contraction: carrier doubling takes learned 12 proposals vs uniform 15; raw/irrelevant fail, universal is unsolved after 707, and charged net is +258372 at 100000 uses | confirmed, asserted, executable | `examples/initial_algebra.rs`; `INITIAL_ALGEBRA.md` |
| U4: exact anonymous polynomial enumeration plus rich heterogeneous evidence narrows 237 syntax candidates to one bounded semantic recursive-signature class (12 aliases); weak evidence reports ambiguity | confirmed, asserted, executable | `src/recursive_signature.rs`; `RECURSIVE_SIGNATURE.md` |
| U4: independently discovered constructors/alpha/generator commute at protected depths 5/7/9 with one untruncated mediator class; leakage, truncation, wrong-arity, aliasing, binary-signature, and fallback controls pass | confirmed, asserted, executable | `examples/recursive_signature.rs`; `RECURSIVE_SIGNATURE.md` |
| U5: without a complete-inventory bit, a nullary/unary/binary stream retains 44/15/2 compatible semantic classes, makes two provisional structural revisions, and replays all prior evidence without claiming logical uniqueness | confirmed, asserted, executable | `src/open_signature.rs`; `OPEN_SIGNATURE.md` |
| U5: wrong-incumbent recovery, score calibration, hysteresis, aliases, leakage/post-freeze, truncation, temporal order, allocation economics, supplied-completeness, misleading/irrelevant, U4 executable bridge, and exact universal fallback controls | confirmed, asserted, executable | `examples/open_signature.rs`; `OPEN_SIGNATURE.md` |
| B3: invented recursion expands proposals to map/append/reverse | confirmed, asserted | `src/typed.rs`; `b3_invented_recursion_expands_proposals_and_recovers_vocabulary` |
| B3 transfer: invented map(reverse) mirrors unseen 5×4 grid | confirmed, asserted | same B3 regression test |

---

## The one claim that was unguarded — now fixed

The **ontology-dependence matrix** is the load-bearing claim the summary presents as
established, but until this audit it was only *printed* at runtime by `ontogen`
(`src/main.rs:1252-1304`). The mirror-image narrative — square valuable under ∅,
redundant under {mul}; power worthless under ∅, valuable under {mul} — lived only in
the printed prose and depended on the raw solves and the search budget.

- **Reproduced:** `cargo run -- ontogen` at default settings produces exactly the
  claimed cells:
  ```
  Gain(square | ∅      ) = ✗  → 1     frontier ACQUIRE
  Gain(square | {mul}  ) = 1  → 1     no gain    reject
  Gain(power | ∅       ) = ✗  → ✗     no gain    reject
  Gain(power | {mul}   ) = ✗  → 16    frontier ACQUIRE
  ```
- **Now asserted:** new regression test `ontology_dependence_matrix`
  (`src/main.rs:4628`) hard-codes `ontogen`'s raw-discovered bodies (confirmed
  empirically: mul=λa.λb.λc.b(a(c)), square=λa.λb.a(a(b)),
  power=λa.λb.λc.λd.b(a,c,d)) and asserts the four mirror-image cells at the known
  interface arities. It runs in <1s in the debug test profile.

> **Note on reproducibility:** `ontogen` only completes in a reasonable time in
> **release** (`cargo build --release && ./target/release/supsearch ontogen`). In the
> debug profile, raw discovery of `power` is impractically slow and the unsolvable
> baselines grind to the time budget. This is why the test hard-codes the discovered
> bodies rather than re-running discovery.

---

## Caveats the audit surfaced

1. **"parity → not discovered under budget" is actually a structural exclusion, not a
   resource failure.** `src/main.rs:932-938` assigns parity arity 0 and drops it from
   the discovered set *by construction*, so it never reaches the acquisition gate. The
   summary reads as a budget bound; the code makes it a deliberate exclusion.
2. **`square` reject / `power` acquire are ontology-relative.** square is rejected on
   the *product family* but earns under ∅ — which is exactly the thesis (a concept's
   value is relative to the ontology), not an inconsistency.
3. **`power` acquire and the C1 reject are printed, not asserted.** They are produced
   by `ladder`/`ontogen` at runtime but have no unit test. Only `mul` and `square`
   outcomes are asserted. (Candidate for future test hardening if the C1 mined-idiom
   machinery is expected to stay stable.)
4. **C8 already moved the "human-given layer" boundary.** The summary's closing
   section says the remaining human-supplied object is the meta-space M. That was true
   at C7, but C8 `disc` now discovers *which fixed-shape templates* pay off inside a
   bounded grammar (`MTm`: two leading binders, one concept hole `C`, ≤1 seed hole `S`,
   no inner λ, ≤4 leaves — 455 deduped templates, of which 6 survive the counterfactual
   prefilter). So the human-supplied layer is now the **template-grammar shape itself**,
   not the enumerated meta-space.
5. **B2-general moves the boundary beyond structural recurrence but does not erase
   it.** The exact first-order Church-list inducer remains the efficient B2 path.
   `universal`, `fixpoint`, and `recursion_search` add relative-semidecision search for
   arbitrary representable closed functionals, including mutual and non-structural
   recursion. `representation` invents anonymous finite sum-of-products encodings when
   constructor arities are supplied. It does not infer those arities from raw bytes,
   establish a unique latent encoding, decide program equivalence, or remove finite
   machine and combinatorial limits. The World A/B/C demo also remains open.

---

## Files of note (all `src/`)

- `bank.rs` — search engine (`solve`, `concept_solve`, `concept_solve_abl`, `concept_solve_diag`), the quotient map, `Concept`.
- `canon.rs` — the value-representation layer (`canonicalize`, `CanonicalValue`); used to falsify the representation-only fold9 wall.
- `term.rs` / `nbe.rs` — λ-term representation and normalization-by-evaluation.
- `transform.rs` — B1 generic context abstraction.
- `recurrence.rs` — B2 cross-depth recurrence induction and equation compilation.
- `typed.rs` — B3 operation-blind simply-typed beta-normal proposal enumeration.
- `universal.rs` / `recursion_search.rs` — fair closed-functional and fuel dovetailing,
  fixed-point validation, finite discovery, and extrapolation controls.
- `fixpoint.rs` / `representation.rs` — pure-lambda single/mutual knot tying and
  anonymous sum-of-products encoding invention.
- `main.rs` — all experiment commands (`ladder`, `ontogen`, `dep`, `gen`, `transfer`, `meta`, `disc`, `ablation`, `diag`, `prune`) and the tests.

---

## Addendum (2026-08-12): U6 / U7 non-monotonic repair and migration

| Claim | Status | Where |
|---|---|---|
| Non-monotonic repair ops (retain/add/remove/split/merge/specialize/generalize/structural-replace) classified between consecutive ontologies | confirmed, asserted | `src/ontology_repair.rs` (`op_between`), 10 tests |
| Comparable cost ledger (description/reasoning/predictive-error/migration/revision) | confirmed, asserted | `ontology_repair::structural_cost` + `Runner::add_stage` ledger |
| Preserved/affected concept accounting and predictive replay (current + accumulated) | confirmed, asserted | `Runner::add_stage` |
| Executable concept meaning via Church-witness term accepted exactly on its extension | confirmed, asserted | `witness`/`evaluate_witness` |
| Structural replacement when patch cost exceeds rebuild | confirmed, asserted | `structural_decision` |
| Deterministic machine records (`deterministic=true`) | confirmed, asserted | `machine_record` (both modules) |
| Five-way migration classification (preserved/refined/re-expressible/ambiguous/invalidated) | confirmed, asserted | `src/concept_migration.rs`, 6 tests |
| Migration cheaper than cold restart by genuinely reusable knowledge only | confirmed, asserted | `migrate` saving, tests assert `saving > 0` only where carry-over exists |
| Replay of old task + held-out verification | confirmed, asserted | `migrate` fields; example output |

New modules: `src/ontology_repair.rs`, `src/concept_migration.rs`. Examples:
`examples/ontology_repair.rs`, `examples/concept_migration.rs`. Docs:
`docs/ONTOLOGY_REPAIR.md`, `docs/MIGRATION.md`.

| Invent observational probes: distinguish previously equivalent hypotheses from model disagreement only | confirmed, asserted | `src/probe_invention.rs`, 5 tests |
| Probe scoring = expected hypothesis reduction - execution cost; correct stop when observationally equivalent | confirmed, asserted | `probe_value`, `select_probe`, `run` |
| Probe selection never sees the hidden answer (§7.3) | confirmed, asserted | `probe_selection_does_not_see_the_answer` |

New module: `src/probe_invention.rs`. Example: `examples/probe_invention.rs`.
Docs: `docs/PROBE_INVENTION.md`.

| Active experimentation: choose intervention that resolves observationally identical environments | confirmed, asserted | `src/active_experimentation.rs`, 4 tests |
| Crucial-experiment baseline: passive learner provably cannot distinguish | confirmed, asserted | `passive_distinguished` field, example |
| Answer-blind intervention selection | confirmed, asserted | `action_selection_never_sees_answer` |

New module: `src/active_experimentation.rs`. Example: `examples/active_experimentation.rs`.
Docs: `docs/ACTIVE_EXPERIMENTATION.md`.

| Causal ontology: interventions separate correlation, mechanism, and intervention response in a Markov-equivalence class | confirmed, asserted | `src/causal_ontology.rs`, 4 tests |
| Passive baseline: passive data leaves multiple candidates (`passive_distinguished=false` for the chain) | confirmed, asserted | `passive_candidates`/`passive_distinguished`, example |
| Answer-blind intervention selection (uses only candidate disagreement) | confirmed, asserted | `selection_is_answer_blind_and_terminates_on_indistinguishable_survivors`, example |
| Honest termination when survivors are indistinguishable under all available interventions | confirmed, asserted | `ndistinct < 2` break, test over all consistent models |
| Deterministic machine record with exact fallback | confirmed, asserted | `machine_record_is_deterministic_and_complete` |

New module: `src/causal_ontology.rs`. Example: `examples/causal_ontology.rs`.
Docs: `docs/CAUSAL_ONTOLOGY.md`.

## Addendum (2026-08-12): Direction G world-model ontogenesis

| Claim | Status | Where |
|---|---|---|
| Factored transition-model discovery (minimal parent set per output variable) | confirmed, asserted | `src/world_model.rs` (`discover_factors`), 6 tests |
| Held-out generalization: factored predicts all held-out where raw table predicts none | confirmed, asserted | `evaluate_generalization`, `two_switch_world_is_fully_factorable` |
| Coupled control: partial factorization honestly reported, no over-claim | confirmed, asserted | `coupled_world_is_only_partially_factorable` |
| Invented reversible-counter concept predicts switch behavior exactly and transfers | confirmed, asserted | `invent_switch_concept`/`predict_with_concept`/`transfer_to_new_switch`, `switch_concept_transfers_to_a_new_switch` |
| Component planning cheaper than raw BFS | confirmed, asserted | `plan_factored` vs `plan_raw`, `planning_is_cheaper_with_the_component_abstraction` |
| Deterministic machine record (`deterministic=true`) | confirmed, asserted | `machine_record_is_deterministic` |

New module: `src/world_model.rs`. Example: `examples/world_model.rs`.
Docs: `docs/WORLD_MODEL.md`.

## Addendum (2026-08-12): Direction M1 mathematical ontogenesis

| Claim | Status | Where |
|---|---|---|
| Arithmetic expression grammar over x,y with + - * / sqrt and composition | confirmed, asserted | `src/math_world.rs` (`Expr`) |
| Bottom-up behavior-deduped enumeration | confirmed, asserted | `build_table` |
| Discovers `sqrt(x*x+y*y)` from Pythagorean triples, generalizes to held-out | confirmed, asserted | `discover_concept`, `discovers_distance_expression` |
| Concept cheaper than re-synthesis (transfer saving) | confirmed, asserted | `transfer_report`, `concept_is_cheaper_than_resynthesis` |
| Concept compresses observations | confirmed, asserted | `compression_report`, `concept_compresses_observations` |
| Non-generalizing control honestly reports no fit | confirmed, asserted | `non_generalizing_fit_is_detected` |
| Deterministic machine record (`deterministic=true`) | confirmed, asserted | `machine_record_is_deterministic` |

New module: `src/math_world.rs`. Example: `examples/math_world.rs`.
Docs: `docs/MATH_WORLD.md`.

## Addendum (2026-08-12): Direction M2 circle invariant

| Claim | Status | Where |
|---|---|---|
| Discovers the invariant `x²+y²=25` from member/non-member points, generalizes to held-out | confirmed, asserted | `discover_invariant`, `m2_tests::discovers_circle_invariant` |
| Invariant cheaper than re-discovery (transfer saving) | confirmed, asserted | `invariant_transfer`, `m2_tests::invariant_is_cheaper_than_rediscovery` |
| Invariant compresses the class | confirmed, asserted | `invariant_compression`, `m2_tests::invariant_compresses_the_class` |
| Non-circular control honestly reports no generalizing invariant | confirmed, asserted | `m2_tests::non_circular_class_has_no_invariant` |
| Deterministic machine record (`deterministic=true`) | confirmed, asserted | `machine_record_m2_is_deterministic` |

M2 extends `src/math_world.rs` and `examples/math_world.rs`. Docs:
`docs/MATH_WORLD.md`.

## Addendum (2026-08-12): Directions M3–M12 and M13 boundary

| Claim | Status | Where |
|---|---|---|
| M3 synthesizes `n*n` without a square primitive and lowers transfer description cost | confirmed, asserted | `discover_square`, `m3_invents_multiplicative_square_without_square_primitive` |
| M4 generates a generalizing odd-sum theorem; malformed control is rejected | confirmed, asserted | `discover_odd_sum_law`, `m4_generates_and_generalizes_the_odd_sum_theorem`, `m4_rejects_non_law_control` |
| M5 retains base + successor closure and transfers to two recursive identities | confirmed, bounded proof-schema claim | `discover_induction`, `m5_invents_reusable_successor_closure_schema` |
| M6 invents reciprocal-difference cancellation and transfers to two offset families | confirmed, asserted | `discover_telescoping`, `m6_invents_cancellation_representation_and_transfers` |
| M7 invents maximum common divisor from divisibility as a transition invariant | confirmed, bounded | `discover_divisor_invariant`, `m7_invents_maximum_common_divisor_invariant` |
| M8 derives a rational formal-series object and predicts three held-out coefficients | confirmed, asserted | `discover_sequence_object`, `m8_invents_fibonacci_formal_series_object` |
| M9 infers hidden transform and scaled latent directions for exact 10-step prediction | confirmed, bounded | `discover_latent_directions`, `invents_scaled_latent_directions_for_hidden_transform` |
| M10 searches a fixed answer-blind reformulation grammar and discovers `2|(n²+n)` | confirmed, asserted | `src/proposition_world.rs`, `m10_discovers_checked_cheaper_equivalent` |
| Independent checker validates both implications for all integers by exhaustive canonical residues | confirmed within modular fragment | `check_proof`, `ModularCertificate` |
| Forged certificate and finite-sample overfit controls are rejected | confirmed, asserted | `checker_rejects_forged_modular_certificate`, `finite_sample_fit_is_not_an_unbounded_proof` |
| M10 alternative lowers checked proof cost (37 to 14), compresses syntax, transfers to `3|(n³-n)` | confirmed, asserted | `m10_experiment`, machine record |
| M11 fixed collection grammar discovers `(product(xs)+1)` at proposal 19 | confirmed, asserted | `src/euclid_world.rs`, `invents_product_plus_one_and_checks_arbitrary_prime_list_theorem` |
| Independent checker proves nonzero member remainder, size, and outside prime witness for arbitrary finite prime lists | confirmed within finite-list schema | `check_escape_certificate` |
| Singleton, non-prime, and corrupted-certificate controls are rejected | confirmed, asserted | `euclid_world::tests` |
| M11 transfers to three unseen prime lists and three composite-divisor lists; reuse cost 12 vs 31 | confirmed, asserted | `m11_experiment`, machine record |
| M12 factored search discovers prime-exponent count modulo 2 at candidate 14 | confirmed, asserted | `src/irrational_world.rs`, `invents_valuation_parity_contradiction_for_sqrt_two` |
| Independent checker proves the valuation even/odd contradiction for arbitrary ratio witnesses | confirmed within valuation fragment | `check_irrationality_certificate` |
| Six nonsquare transfers pass; perfect-square and corrupt-obstruction/proof controls reject | confirmed, asserted | `irrational_world::tests`, `m12_experiment` |
| M12 lowers reasoning 21→7 and compresses seven proofs 49→14 | confirmed, asserted | M12 machine record |
| M13 requires unordered-root/multivariable polynomial architecture | boundary demonstrated | `CURRENT_STATE.md`, `NEXT_BOUNDARY.md` |
| M26 real-zeta completion selection and transfer | confirmed, exact within supplied identity theory | `src/real_zeta_world.rs`, `NEXT_BOUNDARY.md` |
| M27b real-zeta zero-locus conjecture and corrected controls | confirmed, finite numerical conjecture | `src/critical_line_world.rs`, `NEXT_BOUNDARY.md` |
