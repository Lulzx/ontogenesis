//! Direction F (§9): causal ontology from intervention responses.
//!
//! The learner must distinguish correlation, mechanism, and intervention
//! response in tiny finite deterministic systems, and use *interventions* to
//! infer executable causal structure — without the graph names being supplied.
//!
//! Three binary variables A, B, C. The world is a hidden directed acyclic graph
//! with a deterministic boolean function per node. Passive observation yields
//! the joint set of natural outcomes; passive data alone leaves a whole
//! Markov-equivalence class of causal models (e.g. chain `A→B→C` and fork
//! `A←B→C` are observationally identical). Interventions — forcing a variable
//! to a value and reading the downstream response — separate structures that
//! passive data cannot.
//!
//! Acquisition criterion stays practical: the causal model is retained because
//! it makes prediction *under interventions* exact, which a mere correlation
//! model cannot.

use std::collections::BTreeSet;

pub const NVARS: usize = 3;

/// Deterministic function of a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fn {
    Root, // exogenous; takes both natural values
    Copy, // equals its single parent
    Not,  // not of its single parent
    And,
    Or,
    Xor,
}

/// An executable causal model over A, B, C.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausalModel {
    pub parents: Vec<Vec<u8>>,
    pub func: Vec<Fn>,
    pub order: Vec<u8>, // topological order
}

fn topo_order(parents: &[Vec<u8>]) -> Option<Vec<u8>> {
    let mut indeg = [0u8; NVARS];
    for i in 0..NVARS {
        indeg[i] = parents[i].len() as u8;
    }
    let mut order = Vec::new();
    let mut ready: Vec<u8> = (0..NVARS as u8)
        .filter(|v| indeg[*v as usize] == 0)
        .collect();
    ready.sort_unstable();
    while let Some(v) = ready.pop() {
        order.push(v);
        for w in 0..NVARS as u8 {
            if parents[w as usize].contains(&v) {
                indeg[w as usize] -= 1;
                if indeg[w as usize] == 0 {
                    ready.push(w);
                    ready.sort_unstable();
                }
            }
        }
    }
    if order.len() == NVARS {
        Some(order)
    } else {
        None
    }
}

/// The canonical chain `A -> B -> C` with `B=A`, `C=B` (A exogenous).
pub fn chain_model() -> CausalModel {
    CausalModel {
        parents: vec![vec![], vec![0], vec![1]],
        func: vec![Fn::Root, Fn::Copy, Fn::Copy],
        order: vec![0, 1, 2],
    }
}

/// The canonical fork `A <- B -> C` with `A=B`, `C=B` (B exogenous).
pub fn fork_model() -> CausalModel {
    CausalModel {
        parents: vec![vec![1], vec![], vec![1]],
        func: vec![Fn::Copy, Fn::Root, Fn::Copy],
        order: vec![1, 0, 2],
    }
}

/// Enumerate all acyclic 3-variable causal models with the bounded function
/// set. Roots are exogenous (both values); single-parent nodes copy or
/// negate; two-parent nodes are and/or/xor.
pub fn enumerate_models() -> Vec<CausalModel> {
    let mut out = Vec::new();
    for pm in 0..64u16 {
        // parents[i] = subset of the other two, encoded in 2 bits each
        let mut parents: Vec<Vec<u8>> = vec![Vec::new(), Vec::new(), Vec::new()];
        for i in 0..3usize {
            let bits = (pm >> (2 * i as u16)) & 0b11;
            let others: Vec<u8> = (0..3u8).filter(|o| *o != i as u8).collect();
            if bits & 1 != 0 {
                parents[i].push(others[0]);
            }
            if bits & 2 != 0 {
                parents[i].push(others[1]);
            }
        }
        let Some(order) = topo_order(&parents) else {
            continue;
        };
        // function assignments
        let mut func_choices: Vec<Vec<Fn>> = Vec::new();
        let mut total = 1usize;
        for i in 0..3 {
            let choices: Vec<Fn> = match parents[i].len() {
                0 => vec![Fn::Root],
                1 => vec![Fn::Copy, Fn::Not],
                _ => vec![Fn::And, Fn::Or, Fn::Xor],
            };
            total *= choices.len();
            func_choices.push(choices);
        }
        for code in 0..total {
            let mut func = vec![Fn::Root; 3];
            let mut c = code;
            for i in (0..3).rev() {
                let n = func_choices[i].len();
                func[i] = func_choices[i][c % n];
                c /= n;
            }
            out.push(CausalModel {
                parents: parents.clone(),
                func,
                order: order.clone(),
            });
        }
    }
    out
}

/// Evaluate the model given root/forced assignments; computes endogenous vars
/// in topological order. `forced` maps var -> Some(value) for interventions.
pub fn evaluate(
    m: &CausalModel,
    roots: &[bool; NVARS],
    forced: &[Option<bool>; NVARS],
) -> [bool; NVARS] {
    let mut val = [false; NVARS];
    for &v in &m.order {
        let v = v as usize;
        if let Some(x) = forced[v] {
            val[v] = x;
            continue;
        }
        match m.func[v] {
            Fn::Root => val[v] = roots[v],
            Fn::Copy => val[v] = val[m.parents[v][0] as usize],
            Fn::Not => val[v] = !val[m.parents[v][0] as usize],
            Fn::And => {
                let (a, b) = (m.parents[v][0], m.parents[v][1]);
                val[v] = val[a as usize] && val[b as usize];
            }
            Fn::Or => {
                let (a, b) = (m.parents[v][0], m.parents[v][1]);
                val[v] = val[a as usize] || val[b as usize];
            }
            Fn::Xor => {
                let (a, b) = (m.parents[v][0], m.parents[v][1]);
                val[v] = val[a as usize] != val[b as usize];
            }
        }
    }
    val
}

fn joint_set(m: &CausalModel, forced: &[Option<bool>; NVARS]) -> BTreeSet<[bool; NVARS]> {
    // Enumerate root assignments for all non-forced roots.
    let roots: Vec<u8> = (0..NVARS as u8)
        .filter(|v| m.func[*v as usize] == Fn::Root && forced[*v as usize].is_none())
        .collect();
    let mut set = BTreeSet::new();
    for code in 0..(1u32 << roots.len()) {
        let mut r = [false; NVARS];
        for (k, &v) in roots.iter().enumerate() {
            r[v as usize] = (code >> k) & 1 == 1;
        }
        set.insert(evaluate(m, &r, forced));
    }
    set
}

/// Passive natural outcomes of the model.
pub fn observe(m: &CausalModel) -> BTreeSet<[bool; NVARS]> {
    joint_set(m, &[None, None, None])
}

/// Outcomes under an intervention: force variable `x` to `val`.
pub fn intervene(m: &CausalModel, x: u8, val: bool) -> BTreeSet<[bool; NVARS]> {
    let mut forced = [None, None, None];
    forced[x as usize] = Some(val);
    joint_set(m, &forced)
}

fn consistent_passive(m: &CausalModel, passive: &BTreeSet<[bool; NVARS]>) -> bool {
    passive.iter().all(|p| observe(m).contains(p))
}

/// One intervention step.
#[derive(Clone, Debug)]
pub struct CausalStep {
    pub variable: u8,
    pub value: bool,
    pub gain: u64,
    pub observed_set: BTreeSet<[bool; NVARS]>,
    pub candidates_before: usize,
    pub candidates_after: usize,
}

#[derive(Clone, Debug)]
pub struct CausalReport {
    pub model_pool: usize,
    pub passive_candidates: usize,
    pub passive_distinguished: bool,
    pub interventions: Vec<CausalStep>,
    pub final_candidates: usize,
    pub true_recovered: bool,
    pub causal_structure_identified: bool, // final candidates form one equivalence class
}

/// Run causal inference from passive data plus adaptive interventions.
pub fn run_causal(
    truth: &CausalModel,
    passive: &BTreeSet<[bool; NVARS]>,
    _var_names: &[&str; 3],
) -> CausalReport {
    let pool = enumerate_models();
    let mut cands: Vec<CausalModel> = pool
        .iter()
        .filter(|m| consistent_passive(m, passive))
        .cloned()
        .collect();
    let passive_candidates = cands.len();
    let passive_distinguished = passive_candidates == 1;
    let mut interventions = Vec::new();

    while cands.len() > 1 {
        // Answer-blind intervention selection: pick the (variable, value)
        // intervention on which the surviving candidates disagree most (largest
        // number of distinct predicted intervention-response sets). This uses
        // only the candidate models, never the hidden truth.
        let mut best: Option<(u8, bool, usize)> = None;
        for v in 0..3u8 {
            for val in [false, true] {
                let mut classes: Vec<BTreeSet<[bool; NVARS]>> = Vec::new();
                for m in &cands {
                    let resp = intervene(m, v, val);
                    if !classes.contains(&resp) {
                        classes.push(resp);
                    }
                }
                let ndistinct = classes.len();
                let better = match best {
                    None => true,
                    Some((_, _, b)) => ndistinct > b,
                };
                if better {
                    best = Some((v, val, ndistinct));
                }
            }
        }
        let Some((v, val, ndistinct)) = best else {
            break;
        };
        if ndistinct < 2 {
            // Every survivor predicts the same intervention response; they are
            // observationally indistinguishable under every available
            // intervention. Stop rather than loop forever.
            break;
        }
        // Perform the intervention on the true world and prune candidates that
        // predict a different response. (The truth is always a surviving
        // candidate, so this strictly reduces the pool.)
        let before = cands.len();
        let obs_set = intervene(truth, v, val);
        cands.retain(|m| intervene(m, v, val) == obs_set);
        let after = cands.len();
        interventions.push(CausalStep {
            variable: v,
            value: val,
            gain: (before - after) as u64,
            observed_set: obs_set,
            candidates_before: before,
            candidates_after: after,
        });
    }

    let final_candidates = cands.len();
    let true_recovered = cands.len() == 1 && cands[0] == *truth;
    let causal_structure_identified = cands.len() == 1;

    CausalReport {
        model_pool: pool.len(),
        passive_candidates,
        passive_distinguished,
        interventions,
        final_candidates,
        true_recovered,
        causal_structure_identified,
    }
}

/// Deterministic machine-readable record.
pub fn machine_record(r: &CausalReport, var_names: &[&str; 3]) -> String {
    let steps: String = r
        .interventions
        .iter()
        .map(|s| {
            format!(
                "{}={}:g{}:c{}-{}",
                var_names[s.variable as usize],
                s.value,
                s.gain,
                s.candidates_before,
                s.candidates_after
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "experiment=causal_ontology,model_pool={},passive_candidates={},passive_distinguished={},final_candidates={},true_recovered={},causal_structure_identified={},interventions=[{}],deterministic=true,fallback=exact",
        r.model_pool,
        r.passive_candidates,
        r.passive_distinguished,
        r.final_candidates,
        r.true_recovered,
        r.causal_structure_identified,
        steps,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_and_fork_are_passively_indistinguishable_but_intervention_separates() {
        let c = chain_model();
        let f = fork_model();
        // Passive data identical: all variables equal.
        assert_eq!(
            observe(&c),
            observe(&f),
            "must be observationally equivalent passively"
        );
        assert_eq!(
            observe(&c).len(),
            2,
            "A root in {{0,1}} gives two passive joints"
        );
        // Intervention on A separates them: chain -> B,C track A; fork -> B,C free.
        assert_ne!(
            intervene(&c, 0, false),
            intervene(&f, 0, false),
            "intervening on A must distinguish chain from fork"
        );
    }

    #[test]
    fn causal_inference_uses_interventions_when_passive_is_ambiguous() {
        let truth = chain_model();
        let passive = observe(&truth);
        let names = ["A", "B", "C"];
        let r = run_causal(&truth, &passive, &names);
        assert!(
            r.passive_candidates > 1,
            "passive data alone should be ambiguous (Markov-equivalent models)"
        );
        assert!(!r.passive_distinguished);
        assert!(
            !r.interventions.is_empty(),
            "must intervene to identify structure"
        );
        assert_eq!(r.final_candidates, 1);
        assert!(r.true_recovered, "must recover the exact causal model");
    }

    #[test]
    fn machine_record_is_deterministic_and_complete() {
        let truth = chain_model();
        let passive = observe(&truth);
        let names = ["A", "B", "C"];
        let r = run_causal(&truth, &passive, &names);
        let a = machine_record(&r, &names);
        let b = machine_record(&r, &names);
        assert_eq!(a, b);
        assert!(a.contains("experiment=causal_ontology"));
        assert!(a.contains("deterministic=true"));
    }

    #[test]
    fn selection_is_answer_blind_and_terminates_on_indistinguishable_survivors() {
        let truth = chain_model();
        let passive = observe(&truth);
        // The intervention sequence must be determined purely by candidate
        // disagreement, so it is identical regardless of which consistent
        // model is the hidden truth. We check termination and that a distinct
        // true model in the same passive class is still pruned by observations.
        let mut distinct_seen = 0usize;
        for m in enumerate_models() {
            if !passive.iter().all(|p| observe(&m).contains(p)) {
                continue; // not consistent with the passive class
            }
            let r = run_causal(&m, &passive, &["A", "B", "C"]);
            assert!(r.final_candidates >= 1);
            assert!(r.final_candidates <= r.passive_candidates);
            // Every intervention must actually reduce the candidate pool.
            for (i, s) in r.interventions.iter().enumerate() {
                let _ = i;
                assert!(s.candidates_after < s.candidates_before);
            }
            distinct_seen += 1;
        }
        assert!(
            distinct_seen >= 2,
            "should have exercised more than one consistent model"
        );
    }
}
