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
5. **B1–B3 move the boundary but do not erase it.** Inner lambdas and new binder
   structures are now generated by B3's generic typed normal-form enumerator, and
   map/reverse are recovered. B2 is nevertheless restricted to exact first-order
   structural right recurrences over a Church-list execution backend. Arbitrary
   fixed-point recursion, inferred data representations, mutual recursion, and the
   World A/B/C demo remain open.

---

## Files of note (all `src/`)

- `bank.rs` — search engine (`solve`, `concept_solve`, `concept_solve_abl`, `concept_solve_diag`), the quotient map, `Concept`.
- `canon.rs` — the value-representation layer (`canonicalize`, `CanonicalValue`); used to falsify the representation-only fold9 wall.
- `term.rs` / `nbe.rs` — λ-term representation and normalization-by-evaluation.
- `transform.rs` — B1 generic context abstraction.
- `recurrence.rs` — B2 cross-depth recurrence induction and equation compilation.
- `typed.rs` — B3 operation-blind simply-typed beta-normal proposal enumeration.
- `main.rs` — all experiment commands (`ladder`, `ontogen`, `dep`, `gen`, `transfer`, `meta`, `disc`, `ablation`, `diag`, `prune`) and the tests.
