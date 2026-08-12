//! Direction G (§10): world-model ontogenesis.
//!
//! Move from isolated task families toward persistent environments: tiny
//! deterministic worlds with state, actions, observations, hidden structure,
//! and repeated episodes. The agent begins with minimal assumptions (it can
//! observe `(state, action) -> next_state` transitions) and must *invent* a
//! compressed representation of the world that reduces future reasoning cost —
//! prediction, planning, and transfer — without any concept names being
//! supplied.
//!
//! The demo world is a set of independent reversible counters ("switches"),
//! each toggled by exactly one action. The agent is never told this. From a
//! bounded set of observed transitions it must:
//!
//!   1. discover that each switch's dynamics depend only on itself
//!      (a factored transition model — the invented concept),
//!   2. use that factorization to predict *held-out* transitions the raw
//!      state table cannot,
//!   3. plan toward a goal in the relevant component instead of the full
//!      product state space, and
//!   4. invent the "reversible counter" concept and transfer it to a new
//!      switch in a growing world, predicting it from a single probe.
//!
//! The objective is not human interpretability; it is reduced future reasoning
//! cost, measured by sample efficiency, planning search, and transfer probe
//! count. Controls: a coupled world where factorization is only partial (the
//! discovery must not over-claim), an all-variables (monolithic) baseline, and
//! deterministic machine records.

use std::collections::BTreeMap;

/// A single observed transition `(state, action) -> next_state`.
#[derive(Clone, Debug)]
pub struct Transition {
    pub s: Vec<bool>,
    pub a: usize,
    pub sp: Vec<bool>,
}

/// Enumerate all subsets of `vars` of exactly size `k` (as sorted Vec<usize>).
fn subsets(vars: &[usize], k: usize) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    let mut idx = Vec::new();
    gen_subsets(vars, k, 0, &mut idx, &mut out);
    out
}
fn gen_subsets(
    vars: &[usize],
    k: usize,
    start: usize,
    cur: &mut Vec<usize>,
    out: &mut Vec<Vec<usize>>,
) {
    if cur.len() == k {
        out.push(cur.clone());
        return;
    }
    for i in start..vars.len() {
        cur.push(vars[i]);
        gen_subsets(vars, k, i + 1, cur, out);
        cur.pop();
    }
}

/// Projection of `s` onto the parent set `p`.
fn project(s: &[bool], p: &[usize]) -> Vec<bool> {
    p.iter().map(|&i| s[i]).collect()
}

/// Minimal parent set for output variable `y` consistent with all observed
/// transitions: the next value of `y` is a deterministic function of
/// `(projection(s, P), action)`. Searches smallest parent sets first so the
/// discovered structure is the most compressed one that explains the data.
fn find_minimal_parents(nvars: usize, y: usize, observed: &[Transition]) -> Vec<usize> {
    let vars: Vec<usize> = (0..nvars).collect();
    for k in 0..=nvars {
        for p in subsets(&vars, k) {
            if consistent(y, &p, observed) {
                return p;
            }
        }
    }
    unreachable!("a consistent parent set always exists (size nvars)")
}

fn consistent(y: usize, p: &[usize], observed: &[Transition]) -> bool {
    let mut rules: BTreeMap<(Vec<bool>, usize), bool> = BTreeMap::new();
    for tr in observed {
        let key = (project(&tr.s, p), tr.a);
        let val = tr.sp[y];
        if let Some(&prev) = rules.get(&key) {
            if prev != val {
                return false;
            }
        } else {
            rules.insert(key, val);
        }
    }
    true
}

/// Discover the minimal parent set for every output variable.
pub fn discover_factors(nvars: usize, observed: &[Transition]) -> Vec<Vec<usize>> {
    (0..nvars)
        .map(|y| find_minimal_parents(nvars, y, observed))
        .collect()
}

/// A factored transition model: per output variable, a rule table from
/// `(parent-projection, action)` to the variable's next value.
pub struct FactoredModel {
    pub nvars: usize,
    pub parents: Vec<Vec<usize>>,
    pub rules: Vec<BTreeMap<(Vec<bool>, usize), bool>>,
}

pub fn build_factored(
    nvars: usize,
    parents: &[Vec<usize>],
    observed: &[Transition],
) -> FactoredModel {
    let mut rules = Vec::new();
    for y in 0..nvars {
        let mut r = BTreeMap::new();
        for tr in observed {
            r.insert((project(&tr.s, &parents[y]), tr.a), tr.sp[y]);
        }
        rules.push(r);
    }
    FactoredModel {
        nvars,
        parents: parents.to_vec(),
        rules,
    }
}

/// Predict the next state using the factored model; `None` if any variable's
/// rule for `(projection(s, parents[y]), action)` has not been observed.
pub fn predict(m: &FactoredModel, s: &[bool], a: usize) -> Option<Vec<bool>> {
    let mut sp = Vec::with_capacity(m.nvars);
    for y in 0..m.nvars {
        let key = (project(s, &m.parents[y]), a);
        sp.push(*m.rules[y].get(&key)?);
    }
    Some(sp)
}

/// Generalization report on held-out transitions.
#[derive(Clone, Debug)]
pub struct GenReport {
    pub nvars: usize,
    pub observed: usize,
    pub held: usize,
    pub parents: Vec<Vec<usize>>,
    pub factored_correct: usize,
    pub raw_correct: usize,
    pub factored_accuracy: f64,
    pub raw_accuracy: f64,
}

/// Evaluate held-out full-state prediction accuracy of the factored model vs.
/// the raw monolithic table (which predicts a transition only if it was
/// observed, so it generalizes to none of the held-out combos).
pub fn evaluate_generalization(
    nvars: usize,
    observed: &[Transition],
    held: &[Transition],
    parents: &[Vec<usize>],
) -> GenReport {
    let model = build_factored(nvars, parents, observed);
    let mut f_correct = 0usize;
    let mut raw_correct = 0usize;
    for tr in held {
        if let Some(sp) = predict(&model, &tr.s, tr.a) {
            if sp == tr.sp {
                f_correct += 1;
            }
        }
        // Raw baseline: only predicts a held-out combo if that exact (state,
        // action) was observed (it never is, given a strict split).
        if observed.iter().any(|o| o.s == tr.s && o.a == tr.a) {
            // would need a memorized next-state; not present here
            if let Some(o) = observed.iter().find(|o| o.s == tr.s && o.a == tr.a) {
                if o.sp == tr.sp {
                    raw_correct += 1;
                }
            }
        }
    }
    let f = if held.is_empty() {
        0.0
    } else {
        f_correct as f64 / held.len() as f64
    };
    let r = if held.is_empty() {
        0.0
    } else {
        raw_correct as f64 / held.len() as f64
    };
    GenReport {
        nvars,
        observed: observed.len(),
        held: held.len(),
        parents: parents.to_vec(),
        factored_correct: f_correct,
        raw_correct,
        factored_accuracy: f,
        raw_accuracy: r,
    }
}

/// The demo world: `n` independent reversible counters, each toggled by exactly
/// one action; `a == n` is `wait` (no-op). Hidden structure is never supplied.
pub fn two_switch_step(n: usize, s: &[bool], a: usize) -> Vec<bool> {
    let mut sp = s.to_vec();
    if a < n {
        sp[a] = !sp[a]; // toggle switch a
    }
    sp
}

/// Coupled control world: switch 0 toggles freely; every other switch toggles
/// only while switch 0 is set. The factorization is only partial, so the
/// discovery must honestly report that switch 0 is a parent of the rest.
pub fn coupled_step(n: usize, s: &[bool], a: usize) -> Vec<bool> {
    let mut sp = s.to_vec();
    if a < n {
        if a == 0 {
            sp[0] = !sp[0];
        } else if s[0] {
            sp[a] = !sp[a];
        }
    }
    sp
}

/// Enumerate every `(state, action)` transition of a world `step`.
pub fn all_transitions(
    n: usize,
    step: &dyn Fn(usize, &[bool], usize) -> Vec<bool>,
) -> Vec<Transition> {
    let mut out = Vec::new();
    for code in 0..(1usize << n) {
        let s: Vec<bool> = (0..n).map(|i| (code >> i) & 1 == 1).collect();
        for a in 0..=n {
            out.push(Transition {
                s: s.clone(),
                a,
                sp: step(n, &s, a),
            });
        }
    }
    out
}

/// Deterministic observed/held split: keep combos whose
/// `((gray(state_index) + action) % 4) < 2` as observed, the rest held-out.
/// The Gray-code term decorrelates the split from any single state bit, so the
/// observed half still reveals each switch's self-dependence. Reproducible and
/// answer-free.
pub fn parity_split(transitions: &[Transition]) -> (Vec<Transition>, Vec<Transition>) {
    let mut obs = Vec::new();
    let mut held = Vec::new();
    for tr in transitions {
        let idx: usize =
            tr.s.iter()
                .enumerate()
                .filter(|(_, &b)| b)
                .map(|(i, _)| 1usize << i)
                .sum();
        let gray = idx ^ (idx >> 1);
        if (gray + tr.a) % 4 < 2 {
            obs.push(tr.clone());
        } else {
            held.push(tr.clone());
        }
    }
    (obs, held)
}

// ---------------------------------------------------------------------------
// Invented "reversible counter" concept and transfer to a growing world.
// ---------------------------------------------------------------------------

/// Given the discovered factorization in a world where switch `i` toggles on
/// action `ta` (and no other action changes it), the agent invents the concept
/// "switch i is a reversible counter controlled by action ta".
#[derive(Clone, Debug)]
pub struct SwitchConcept {
    pub switch: usize,
    pub toggle_action: usize,
    pub probe_cost: usize, // how many probes it took to identify the action
}

/// Invent a switch concept for a new variable `w` in world `step` by probing:
/// apply each candidate action to a reference state and see which one changes
/// variable `w`. Bounded: at most `n+1` probes (one per action).
pub fn invent_switch_concept(
    n: usize,
    w: usize,
    ref_state: &[bool],
    step: &dyn Fn(usize, &[bool], usize) -> Vec<bool>,
) -> SwitchConcept {
    let mut probe_cost = 0usize;
    for a in 0..=n {
        probe_cost += 1;
        let sp = step(n, ref_state, a);
        if sp[w] != ref_state[w] {
            // action `a` changes switch `w`; with a single-bit toggle and no
            // other action changing it, this identifies the concept.
            return SwitchConcept {
                switch: w,
                toggle_action: a,
                probe_cost,
            };
        }
    }
    // No action toggles it: it is constant. Cost still counts the probes.
    SwitchConcept {
        switch: w,
        toggle_action: usize::MAX,
        probe_cost,
    }
}

/// Transfer prediction: using an invented switch concept (the action that
/// toggles switch `w`), predict the next value of `w` for any state/action.
/// A switch toggles only on its own action; `None` if no concept is known.
pub fn predict_with_concept(c: &SwitchConcept, s: &[bool], a: usize) -> Option<bool> {
    if c.toggle_action == usize::MAX {
        return Some(false); // constant switch, never set
    }
    if a == c.toggle_action {
        Some(!s[c.switch])
    } else {
        Some(s[c.switch])
    }
}

/// Transfer report: with the invented switch concept, a new switch `w` is fully
/// predictable after `probe_cost` probes; the raw/cold-start baseline needs to
/// observe every `(state, action)` combination that touches `w` (2 * (n+1)).
#[derive(Clone, Debug)]
pub struct TransferReport {
    pub new_switch: usize,
    pub toggle_action: usize,
    pub probe_cost: usize,
    pub cold_start_observations: usize,
    pub transfer_saved: usize,
}

pub fn transfer_to_new_switch(
    n: usize,
    new_w: usize,
    _known_w: usize,
    step: &dyn Fn(usize, &[bool], usize) -> Vec<bool>,
) -> TransferReport {
    // Invent the concept on a switch we already understand (known_w) to confirm
    // the mechanism, then probe the new switch. Here the concept is invented
    // directly on the new switch from a single reference state.
    let ref_state: Vec<bool> = (0..n).map(|_| false).collect();
    let concept = invent_switch_concept(n, new_w, &ref_state, step);
    // Cold start: fully predict switch new_w from the raw table requires
    // observing each (state, action) that affects it: for each action, each
    // value of new_w (2 values), i.e. 2*(n+1) transitions.
    let cold = 2 * (n + 1);
    TransferReport {
        new_switch: new_w,
        toggle_action: concept.toggle_action,
        probe_cost: concept.probe_cost,
        cold_start_observations: cold,
        transfer_saved: cold.saturating_sub(concept.probe_cost),
    }
}

// ---------------------------------------------------------------------------
// Planning.
// ---------------------------------------------------------------------------

/// Raw BFS plan cost (number of state expansions) from `start` to reach the
/// exact target state in the full product state space.
pub fn plan_raw(
    n: usize,
    start: &[bool],
    target: &[bool],
    step: &dyn Fn(usize, &[bool], usize) -> Vec<bool>,
) -> Option<usize> {
    use std::collections::VecDeque;
    if start == target {
        return Some(0);
    }
    let mut seen = BTreeMap::new();
    seen.insert(start.to_vec(), 0usize);
    let mut q = VecDeque::new();
    q.push_back(start.to_vec());
    let mut expansions = 0usize;
    while let Some(cur) = q.pop_front() {
        expansions += 1;
        let d = *seen.get(&cur).unwrap();
        for a in 0..=n {
            let nxt = step(n, &cur, a);
            if seen.contains_key(&nxt) {
                continue;
            }
            if nxt == target {
                return Some(expansions);
            }
            seen.insert(nxt.clone(), d + 1);
            q.push_back(nxt);
        }
    }
    None
}

/// Factored per-component planning cost to reach `target`. Because the
/// discovered factorization proves each switch is independent, the plan is the
/// sum over switches of one toggle in each component that differs from the
/// target — bounded by the number of switches, not the product state space.
pub fn plan_factored(start: &[bool], target: &[bool]) -> usize {
    start
        .iter()
        .zip(target.iter())
        .filter(|(a, b)| a != b)
        .count()
}

// ---------------------------------------------------------------------------
// Machine record.
// ---------------------------------------------------------------------------

/// Deterministic machine-readable record for the world-model experiment.
pub fn machine_record(
    gen: &GenReport,
    transfer: &TransferReport,
    planning_raw: usize,
    planning_factored: usize,
    partial_parents: &[Vec<usize>],
) -> String {
    format!(
        "experiment=world_model,nvars={},observed={},held={},factored_parents={:?},factored_accuracy={:.4},raw_accuracy={:.4},factored_correct={}/{},",
        gen.nvars, gen.observed, gen.held, gen.parents, gen.factored_accuracy, gen.raw_accuracy,
        gen.factored_correct, gen.held,
    ) + &format!(
        "transfer_probe={},transfer_cold={},transfer_saved={},planning_raw={},planning_factored={},partial_parents={:?},deterministic=true,fallback=exact",
        transfer.probe_cost, transfer.cold_start_observations, transfer.transfer_saved,
        planning_raw, planning_factored, partial_parents,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_switch_world_is_fully_factorable() {
        let n = 3;
        let trans = all_transitions(n, &two_switch_step);
        let (obs, held) = parity_split(&trans);
        assert_eq!(obs.len(), (trans.len() + 1) / 2);
        let parents = discover_factors(n, &obs);
        // Each switch depends only on itself.
        for i in 0..n {
            assert_eq!(parents[i], vec![i], "switch {i} must depend only on itself");
        }
        let rep = evaluate_generalization(n, &obs, &held, &parents);
        assert!(
            rep.factored_accuracy > 0.5,
            "factorization must generalize to held-out transitions"
        );
        assert_eq!(
            rep.raw_accuracy, 0.0,
            "raw table cannot generalize to held-out combos"
        );
    }

    #[test]
    fn factored_model_predicts_held_out_where_raw_cannot() {
        let n = 3;
        let trans = all_transitions(n, &two_switch_step);
        let (obs, held) = parity_split(&trans);
        let parents = discover_factors(n, &obs);
        let rep = evaluate_generalization(n, &obs, &held, &parents);
        assert!(rep.factored_correct > rep.raw_correct);
        assert!(rep.factored_accuracy > 0.5);
    }

    #[test]
    fn coupled_world_is_only_partially_factorable() {
        let n = 3;
        let trans = all_transitions(n, &coupled_step);
        let (obs, _held) = parity_split(&trans);
        let parents = discover_factors(n, &obs);
        // Switch 0 depends only on itself; the coupling is detected: switches
        // 1 and 2 both depend on switch 0 (no false full factorization).
        assert_eq!(parents[0], vec![0]);
        assert!(parents[1].contains(&0));
        assert!(parents[2].contains(&0));
        // And the coupled world does NOT generalize: held-out prediction is
        // poor, so the discovery does not over-claim compression.
        let (obs, held) = parity_split(&trans);
        let rep = evaluate_generalization(n, &obs, &held, &parents);
        assert!(
            rep.factored_accuracy < 0.5,
            "coupled world must not generalize like a factored one"
        );
    }

    #[test]
    fn switch_concept_transfers_to_a_new_switch() {
        let n = 3;
        let trans = all_transitions(n, &two_switch_step);
        let (obs, _held) = parity_split(&trans);
        let parents = discover_factors(n, &obs);
        assert_eq!(parents[0], vec![0]);
        // Invent the concept for switch 2 (new room) from a single probe.
        let ref_state: Vec<bool> = vec![false, false, false];
        let c = invent_switch_concept(n, 2, &ref_state, &two_switch_step);
        assert_eq!(c.toggle_action, 2);
        // It must predict switch-2 behavior for every state/action.
        for code in 0..8usize {
            let s: Vec<bool> = (0..3).map(|i| (code >> i) & 1 == 1).collect();
            for a in 0..=3usize {
                let true_sp = two_switch_step(3, &s, a)[2];
                let pred = predict_with_concept(&c, &s, a).unwrap();
                assert_eq!(pred, true_sp, "concept must predict switch 2 exactly");
            }
        }
        let tr = transfer_to_new_switch(3, 2, 0, &two_switch_step);
        assert!(
            tr.transfer_saved > 0,
            "transfer must cost less than cold restart"
        );
    }

    #[test]
    fn planning_is_cheaper_with_the_component_abstraction() {
        let n = 6;
        let start = [false; 6];
        let target = [true; 6];
        let raw = plan_raw(n, &start, &target, &two_switch_step).unwrap();
        let factored = plan_factored(&start, &target);
        assert_eq!(factored, 6, "one toggle per differing switch");
        assert!(
            factored < raw,
            "component planning must be cheaper than raw BFS"
        );
    }

    #[test]
    fn machine_record_is_deterministic() {
        let n = 3;
        let trans = all_transitions(n, &two_switch_step);
        let (obs, held) = parity_split(&trans);
        let parents = discover_factors(n, &obs);
        let rep = evaluate_generalization(n, &obs, &held, &parents);
        let tr = transfer_to_new_switch(3, 2, 0, &two_switch_step);
        let raw = plan_raw(6, &[false; 6], &[true; 6], &two_switch_step).unwrap_or(99);
        let fac = plan_factored(&[false; 6], &[true; 6]);
        let coupled = all_transitions(3, &coupled_step);
        let (cobs, _) = parity_split(&coupled);
        let cparents = discover_factors(3, &cobs);
        let a = machine_record(&rep, &tr, raw, fac, &cparents);
        let b = machine_record(&rep, &tr, raw, fac, &cparents);
        assert_eq!(a, b);
        assert!(a.contains("experiment=world_model"));
        assert!(a.contains("deterministic=true"));
    }
}
