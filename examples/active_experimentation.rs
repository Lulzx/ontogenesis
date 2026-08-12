// Ontogenesis experiment: active experimentation and the crucial experiment
// (Direction E, §8 / §8.1).
//
// A passive learner only observes the default input and cannot tell two
// observationally identical environments apart. An active learner chooses an
// intervention (sets an input value), observes the result, and prunes its
// candidate hypotheses. This module demonstrates the crucial experiment: two
// environments agree on every passive observation but differ under one
// intervention, which only the active learner discovers and uses.
use std::collections::BTreeMap;

use supsearch::active_experimentation::{
    self, all_hypotheses, run_active, Hypothesis, PASSIVE_INPUT,
};

fn main() {
    // World truth: output is true exactly when the input equals 1.
    let world: Hypothesis = [false, true, false, false];

    // Passive data: only the default input x=0 is observed -> output false.
    let mut initial: BTreeMap<u8, bool> = BTreeMap::new();
    initial.insert(PASSIVE_INPUT, world[PASSIVE_INPUT as usize]);

    let pool = all_hypotheses();
    let passive_count = pool
        .iter()
        .filter(|h| h[PASSIVE_INPUT as usize] == world[PASSIVE_INPUT as usize])
        .count();

    let r = run_active(&world, &initial);

    println!("ontogenesis: active experimentation & crucial experiment (E)");
    println!(
        "hypothesis pool={} consistent with passive data={}",
        pool.len(),
        passive_count
    );
    println!(
        "passive_final_candidates={} passive_distinguished={}",
        r.passive_final_candidates, r.passive_distinguished
    );
    for s in &r.steps {
        println!(
            "  intervene x={} (gain={}, value={}) -> observed={} candidates {} -> {}",
            s.action,
            s.gain,
            s.value,
            s.observed,
            s.candidates_before,
            s.candidates_after
        );
    }
    println!(
        "active_final_candidates={} actions_taken={} total_action_cost={} total_information={}",
        r.active_final_candidates,
        r.actions_taken,
        r.total_action_cost,
        r.total_information
    );
    println!(
        "active_true_recovered={} crucial_action={}",
        r.active_true_recovered, r.crucial_action
    );
    println!("{}", active_experimentation::machine_record(&r));
}
