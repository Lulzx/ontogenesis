//! Counterfactual concept acquisition — the "is this worth a concept?" gate.
//!
//! A concept `c` is worth installing for distribution `D` ⟺ reasoning on `D`
//! through a quotient-aware search over the ontology ∪ {c} is strictly cheaper
//! than over the ontology alone (Δ > 0). This module holds that measured gate
//! and the cost primitives it is built on:
//!
//! - [`concept_cost`] / [`raw_cost`] — the honest cost of solving a task through
//!   a concept set (the quotient reasoner's `built` count) vs. raw enumeration.
//! - [`propose_value`] — given a candidate closed body with *no known interface*,
//!   find the composition arity that makes the held-out family cheapest and
//!   report the effect as a [`Gain`] (before/after over `Cost ∈ N ∪ {∞}`).
//! - [`Gain::earns`] — the acquisition verdict: promote iff `after < before`.
//!
//! The arity is *inferred by measurement, never supplied*: with the wrong arity a
//! concept applied to a tuple produces non-domain values or oversized normal forms
//! pruned by hash fuel, so the correct arity is the one the cost structure picks.
//! No sentinel arithmetic is ever exposed as a delta ([`UNREACHABLE`] = ∞).

use crate::{bank, parse, term};
use std::rc::Rc;

/// ∞ — a task the reasoner cannot solve within budget. Real but deliberately huge,
/// and never used as an arithmetic delta (see `Gain`).
pub const UNREACHABLE: u64 = u64::MAX / 4;

/// Cost of a task through a quotient-aware search over the given concept set:
/// `built` if solvable, [`UNREACHABLE`] if not.
pub fn concept_cost(t: &parse::Task, set: &[bank::Concept], opts: &bank::Options) -> u64 {
    let o = bank::concept_solve(t, set, opts);
    match o.solution {
        Some(_) => o.stats.built,
        None => UNREACHABLE,
    }
}

/// [`concept_cost`] through the canonical-keying ablation path
/// ([`bank::concept_solve_abl`] with `use_canon`). Same cost semantics; the
/// value identity is the compact canonical key (numeral/grid) instead of the
/// structural hash, so ARC-sized grids stay hashable within budget. This is the
/// general mechanism the A1 slice uses to reason about grids past the 2048
/// structural cap — canonical keying is representation-only, not a new concept.
pub fn concept_cost_abl(
    t: &parse::Task,
    set: &[bank::Concept],
    opts: &bank::Options,
    use_canon: bool,
) -> u64 {
    let (o, _m) = bank::concept_solve_abl(t, set, opts, use_canon);
    match o.solution {
        Some(_) => o.stats.built,
        None => UNREACHABLE,
    }
}

/// Cost of a task through the raw bottom-up bank (the machine before it has
/// any concept to reason through).
pub fn raw_cost(t: &parse::Task, opts: &bank::Options) -> u64 {
    let o = bank::solve(t, opts);
    match o.solution {
        Some(_) => o.stats.built,
        None => UNREACHABLE,
    }
}

/// The C2+C3 meta-experiment in one call: given an invented closed computation
/// `body` (a candidate concept with NO known interface) and the currently-held
/// concept set, find the composition arity k that makes the held-out family
/// cheapest, and report the effect *structurally* — as a before/after cost pair
/// over `Cost ∈ N ∪ {∞}` ([`UNREACHABLE`] = ∞) — so promotion can distinguish a
/// frontier move (∞ → finite) from a search-cost reduction (finite → smaller)
/// without leaking a sentinel as an arithmetic delta.
///
/// Returns `Some(Gain)` for the best (cheapest-after) interface arity, or
/// `None` if no arity in 1..=5 is worth trying. The arity is *inferred* by
/// measurement, never supplied: with the wrong arity the concept applied to
/// a tuple produces values that do not reduce held-out cost, so the search
/// finds the true interface.
pub fn propose_value(
    body: &Rc<term::Term>,
    current: &[bank::Concept],
    holdout: &[parse::Task],
    opts: &bank::Options,
    baseline: u64,
) -> Option<Gain> {
    propose_value_with(body, current, holdout, opts, baseline, &concept_cost)
}

/// [`propose_value`] through the canonical-keying ablation path: the held-out
/// cost is measured with [`concept_cost_abl`] instead of [`concept_cost`], so
/// the gate can reason about ARC-sized grids past the structural 2048 cap. The
/// verdict semantics are identical — promotion iff `after < before`.
pub fn propose_value_abl(
    body: &Rc<term::Term>,
    current: &[bank::Concept],
    holdout: &[parse::Task],
    opts: &bank::Options,
    baseline: u64,
    use_canon: bool,
) -> Option<Gain> {
    propose_value_with(body, current, holdout, opts, baseline, &|t, set, o| {
        concept_cost_abl(t, set, o, use_canon)
    })
}

/// The shared acquisition loop, parameterized by the held-out cost function so
/// the structural and canonical paths share one gate. See [`propose_value`].
fn propose_value_with(
    body: &Rc<term::Term>,
    current: &[bank::Concept],
    holdout: &[parse::Task],
    opts: &bank::Options,
    baseline: u64,
    cost: &dyn Fn(&parse::Task, &[bank::Concept], &bank::Options) -> u64,
) -> Option<Gain> {
    // Early-exit on the first arity that earns: in these tasks exactly one arity
    // is the correct interface (others produce non-domain values and cost more),
    // so the first win is the inferred interface, and the wrong-arity grind is
    // skipped. For a candidate nothing earns, evaluate all arities to report the
    // cheapest after (so a rejection still shows its measured before → after).
    let mut best: Option<Gain> = None;
    for k in 1..=5u32 {
        let mut set = current.to_vec();
        set.push(bank::Concept {
            body: body.clone(),
            name: "cand".into(),
            arity: k,
        });
        let after: u64 = holdout.iter().map(|t| cost(t, &set, opts)).sum();
        let g = Gain {
            arity: k,
            before: baseline,
            after,
        };
        if best.as_ref().map_or(true, |b: &Gain| after < b.after) {
            best = Some(g.clone());
        }
        if g.earns() {
            return Some(g);
        }
    }
    best
}

/// The measured effect of installing a candidate as a Prim, as a cost pair
/// over `Cost ∈ N ∪ {∞}`. No sentinel arithmetic is ever exposed as a delta.
#[derive(Clone, Copy)]
pub struct Gain {
    /// The inferred composition arity.
    pub arity: u32,
    /// Held-out cost under the current ontology ([`UNREACHABLE`] = unsolved).
    pub before: u64,
    /// Held-out cost with the candidate installed at `arity`.
    pub after: u64,
}

impl Gain {
    /// Frontier move: baseline was unsolved (∞), the candidate makes it solvable.
    pub fn frontier(&self) -> bool {
        self.before >= UNREACHABLE && self.after < UNREACHABLE
    }
    /// Solved → solved cost reduction (0 if not applicable).
    pub fn search_gain(&self) -> u64 {
        if self.before < UNREACHABLE && self.after < self.before {
            self.before - self.after
        } else {
            0
        }
    }
    /// The acquisition verdict: strictly cheaper under `Cost ∈ N ∪ {∞}`.
    pub fn earns(&self) -> bool {
        self.after < self.before
    }
    /// Human label of the kind of change this candidate causes.
    pub fn kind(&self) -> &'static str {
        if self.frontier() {
            "frontier gain"
        } else if self.search_gain() > 0 {
            "search gain"
        } else if self.after == self.before {
            "no gain"
        } else {
            "regression"
        }
    }
}

/// Rank two candidates for promotion: frontier gain ≻ search-cost reduction.
/// Returns `Greater` if `a` outranks `b`.
pub fn gain_rank(a: &Gain, b: &Gain) -> std::cmp::Ordering {
    match (a.frontier(), b.frontier()) {
        (true, false) => std::cmp::Ordering::Greater,
        (false, true) => std::cmp::Ordering::Less,
        _ => a.search_gain().cmp(&b.search_gain()),
    }
}

/// Display a cost, collapsing [`UNREACHABLE`] to the "✗" (unsolved) marker.
pub fn disp_cost(x: u64) -> String {
    if x >= UNREACHABLE {
        "✗".into()
    } else {
        format!("{x}")
    }
}
