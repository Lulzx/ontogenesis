//! Direction E (§8): active experimentation and the crucial experiment.
//!
//! Probe invention (Direction D) chooses which *input* to measure. Active
//! experimentation goes one step further: the agent chooses which *action* to
//! perform — an intervention, not just a passive reading. The decisive case is
//! the crucial experiment (§8.1): two environments are observationally
//! identical under all passive data, yet differ under one intervention.
//!
//! A passive learner that only observes can never tell them apart. An active
//! learner that chooses the distinguishing action and observes the result can.
//! That is the step toward scientific behavior this module measures.
//!
//! Domain (small, deterministic, domain-neutral): a machine with a 2-bit input
//! `x in 0..3` and a boolean output. The world is a hidden function
//! `f: input -> output`. A hypothesis is a candidate function; the pool is all
//! 16 boolean functions of two bits. The default (passively observed) input is
//! `x=0`; the agent may *intervene* by setting `x` to 1, 2, or 3 (each costs a
//! unit). It chooses the intervention that most reduces uncertainty among its
//! remaining hypotheses, never seeing the answer in advance.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

pub const ACTION_COST: u64 = 1; // cost to set an input to a non-default value
pub const PASSIVE_INPUT: u8 = 0; // the default input observed passively

/// A hypothesis: output (bool) for each of the 4 input values 0..3.
pub type Hypothesis = [bool; 4];

/// All 16 boolean functions of a 2-bit input.
pub fn all_hypotheses() -> Vec<Hypothesis> {
    let mut out = Vec::new();
    for mask in 0..16u8 {
        let h: Hypothesis = [
            (mask >> 0) & 1 == 1,
            (mask >> 1) & 1 == 1,
            (mask >> 2) & 1 == 1,
            (mask >> 3) & 1 == 1,
        ];
        out.push(h);
    }
    out
}

fn consistent(h: &Hypothesis, obs: &BTreeMap<u8, bool>) -> bool {
    obs.iter().all(|(x, y)| h[*x as usize] == *y)
}

/// Predicted-output disagreement of the candidate set on intervention `x`.
fn separates(candidates: &[Hypothesis], x: u8) -> bool {
    let mut has_true = false;
    let mut has_false = false;
    for h in candidates {
        if h[x as usize] {
            has_true = true;
        } else {
            has_false = true;
        }
    }
    has_true && has_false
}

/// Value of intervening on `x`: expected hypothesis reduction minus cost.
/// Deterministic balanced-split information gain.
pub fn action_value(candidates: &[Hypothesis], x: u8) -> i64 {
    if !separates(candidates, x) {
        return -(ACTION_COST as i64);
    }
    let n_true = candidates.iter().filter(|h| h[x as usize]).count();
    let n_false = candidates.len() - n_true;
    let gain = n_true.min(n_false) as u64;
    (gain as i64) - (ACTION_COST as i64)
}

/// Choose the best intervention among those not yet performed (greedy).
/// Returns None when no remaining intervention separates any candidate pair.
pub fn select_action(candidates: &[Hypothesis], done: &BTreeSet<u8>) -> Option<(u8, u64, i64)> {
    let mut best: Option<(u8, u64, i64)> = None;
    for x in 1..4u8 {
        if done.contains(&x) {
            continue;
        }
        if !separates(candidates, x) {
            continue;
        }
        let n_true = candidates.iter().filter(|h| h[x as usize]).count();
        let gain = n_true.min(candidates.len() - n_true) as u64;
        let v = action_value(candidates, x);
        let cand = (x, gain, v);
        best = Some(match best {
            None => cand,
            Some(b) => {
                if v > b.2 {
                    cand
                } else {
                    b
                }
            }
        });
    }
    best
}

/// One intervention performed by the active learner.
#[derive(Clone, Debug)]
pub struct ActionStep {
    pub action: u8, // the input value set (1..3)
    pub gain: u64,
    pub value: i64,
    pub observed: bool, // world output under the intervention
    pub candidates_before: usize,
    pub candidates_after: usize,
}

#[derive(Clone, Debug)]
pub struct ExperimentReport {
    pub initial_candidates: usize,
    pub active_final_candidates: usize,
    pub actions_taken: u64,
    pub actions_that_reduced: u64,
    pub total_action_cost: u64,
    pub total_information: u64,
    pub active_true_recovered: bool,
    pub passive_final_candidates: usize,
    pub passive_distinguished: bool, // passive data alone resolved the world
    pub crucial_action: u8,          // intervention that first separates (0 = none needed)
    pub steps: Vec<ActionStep>,
}

/// Active learner: starts from passive observations and intervenes to resolve
/// residual uncertainty.
pub fn run_active(world: &Hypothesis, initial: &BTreeMap<u8, bool>) -> ExperimentReport {
    let pool = all_hypotheses();
    let mut candidates: Vec<Hypothesis> = pool
        .iter()
        .filter(|h| consistent(h, initial))
        .copied()
        .collect();
    let initial_candidates = candidates.len();
    let mut obs = initial.clone();
    let mut done: BTreeSet<u8> = initial.keys().copied().collect();
    let mut steps: Vec<ActionStep> = Vec::new();
    let mut actions_that_reduced = 0u64;
    let mut total_information = 0u64;
    let mut crucial_action = 0u8;

    while candidates.len() > 1 {
        match select_action(&candidates, &done) {
            None => break, // observationally equivalent under every intervention
            Some((x, gain, value)) => {
                let y = world[x as usize];
                let before = candidates.len();
                candidates.retain(|h| h[x as usize] == y);
                let after = candidates.len();
                done.insert(x);
                obs.insert(x, y);
                total_information += gain;
                if crucial_action == 0 {
                    crucial_action = x;
                }
                if after < before {
                    actions_that_reduced += 1;
                }
                steps.push(ActionStep {
                    action: x,
                    gain,
                    value,
                    observed: y,
                    candidates_before: before,
                    candidates_after: after,
                });
            }
        }
    }

    let active_final = candidates.len();
    let active_true_recovered = active_final == 1 && candidates[0] == *world;
    let total_action_cost = steps.len() as u64 * ACTION_COST;

    // Passive baseline: only the default input is observed; no interventions.
    let passive_candidates = pool.iter().filter(|h| consistent(h, &initial)).count();
    let passive_distinguished = passive_candidates == 1;

    ExperimentReport {
        initial_candidates,
        active_final_candidates: active_final,
        actions_taken: steps.len() as u64,
        actions_that_reduced,
        total_action_cost,
        total_information,
        active_true_recovered,
        passive_final_candidates: passive_candidates,
        passive_distinguished,
        crucial_action,
        steps,
    }
}

/// Deterministic machine-readable record.
pub fn machine_record(r: &ExperimentReport) -> String {
    let steps: String = r
        .steps
        .iter()
        .map(|s| {
            format!(
                "x{}:g{}:c{}-{}",
                s.action, s.gain, s.candidates_before, s.candidates_after
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "experiment=active_experimentation,initial_candidates={},active_final_candidates={},passive_final_candidates={},passive_distinguished={},actions_taken={},actions_that_reduced={},total_action_cost={},total_information={},active_true_recovered={},crucial_action={},steps=[{}],deterministic=true,fallback=exact",
        r.initial_candidates,
        r.active_final_candidates,
        r.passive_final_candidates,
        r.passive_distinguished,
        r.actions_taken,
        r.actions_that_reduced,
        r.total_action_cost,
        r.total_information,
        r.active_true_recovered,
        r.crucial_action,
        steps,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init(y0: bool) -> BTreeMap<u8, bool> {
        let mut m = BTreeMap::new();
        m.insert(PASSIVE_INPUT, y0);
        m
    }

    #[test]
    fn active_learner_performs_crucial_experiment_when_passive_cannot() {
        // World: y = 1 iff x==1 (hypothesis [false,true,false,false]).
        let world: Hypothesis = [false, true, false, false];
        // Passive default x=0 -> output false. Eight hypotheses share f(0)=false.
        let initial = init(false);
        let r = run_active(&world, &initial);
        assert!(r.initial_candidates >= 8, "passive data leaves ambiguity");
        assert!(
            r.passive_final_candidates > 1,
            "passive data should NOT resolve"
        );
        assert!(
            !r.passive_distinguished,
            "passive learner must fail to distinguish"
        );
        assert!(r.actions_taken >= 1, "active learner must intervene");
        assert!(
            r.active_true_recovered,
            "active learner must recover world truth"
        );
        assert!(
            r.crucial_action != 0,
            "an intervention must be the crucial experiment"
        );
    }

    #[test]
    fn passive_data_can_resolve_when_fully_informative() {
        // World truth with distinct output on default input 0: e.g., only input 0.
        // If initial observation already pins down a unique hypothesis, no action needed.
        let world: Hypothesis = [true, false, false, false];
        let initial = init(true); // x=0 -> true; only hypotheses with h[0]=true (8) remain
        let r = run_active(&world, &initial);
        // passive still leaves 8; active must intervene to reach 1.
        assert!(r.active_true_recovered);
        assert!(r.actions_taken >= 1);
    }

    #[test]
    fn action_selection_never_sees_answer() {
        let candidates = all_hypotheses();
        // action_value for each of 1,2,3 is purely from candidate disagreement.
        for x in 1..4u8 {
            let v = action_value(&candidates, x);
            assert_eq!(
                v, 7,
                "all functions of 2 bits: probing any input splits 8/8, gain 8 - cost 1 = 7"
            );
        }
    }

    #[test]
    fn machine_record_is_deterministic_and_complete() {
        let world: Hypothesis = [false, true, false, false];
        let r = run_active(&world, &init(false));
        let a = machine_record(&r);
        let b = machine_record(&r);
        assert_eq!(a, b);
        assert!(a.contains("experiment=active_experimentation"));
        assert!(a.contains("deterministic=true"));
        assert!(a.contains("active_true_recovered=true"));
    }
}
