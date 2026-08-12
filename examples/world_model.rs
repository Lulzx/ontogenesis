// Ontogenesis experiment: world-model ontogenesis (Direction G, §10).
//
// A persistent deterministic world of independent reversible counters
// ("switches"), each toggled by exactly one action. The agent is never told
// this. From a bounded set of observed (state, action) -> next_state
// transitions it must invent a compressed representation of the world that
// reduces future reasoning cost: predict held-out transitions the raw table
// cannot, plan in the relevant component instead of the full product space,
// and transfer an invented "reversible counter" concept to a new switch in a
// growing world.
use supsearch::world_model::{
    all_transitions, coupled_step, discover_factors, evaluate_generalization,
    invent_switch_concept, machine_record, parity_split, plan_factored, plan_raw,
    predict_with_concept, transfer_to_new_switch, two_switch_step,
};

fn main() {
    let n = 3usize;

    // Build the world and split into observed / held-out transitions.
    let trans = all_transitions(n, &two_switch_step);
    let (obs, held) = parity_split(&trans);

    // 1. Discover the factored transition model (the invented concept).
    let parents = discover_factors(n, &obs);
    println!("ontogenesis: world-model ontogenesis (G)");
    println!(
        "world: {} independent reversible counters, {} actions (toggle_i, wait)",
        n,
        n + 1
    );
    println!(
        "observed transitions={} held-out={}",
        obs.len(),
        held.len()
    );
    println!("discovered parent sets (per switch): {:?}", parents);

    // 2. Held-out generalization: factored vs raw monolithic table.
    let rep = evaluate_generalization(n, &obs, &held, &parents);
    println!(
        "held-out full-state prediction: factored {}/{} ({:.3}), raw {}/{} ({:.3})",
        rep.factored_correct,
        rep.held,
        rep.factored_accuracy,
        rep.raw_correct,
        rep.held,
        rep.raw_accuracy
    );

    // 3. Invented "reversible counter" concept and transfer to a new switch.
    let ref_state = vec![false, false, false];
    let concept = invent_switch_concept(n, 2, &ref_state, &two_switch_step);
    println!(
        "invented switch concept: switch {} toggles on action {} (probe cost {})",
        concept.switch, concept.toggle_action, concept.probe_cost
    );
    // Verify the concept predicts switch-2 behavior exactly.
    let mut concept_correct = 0usize;
    let mut concept_total = 0usize;
    for code in 0..(1usize << n) {
        let s: Vec<bool> = (0..n).map(|i| (code >> i) & 1 == 1).collect();
        for a in 0..=n {
            let true_sp = two_switch_step(n, &s, a)[2];
            if predict_with_concept(&concept, &s, a) == Some(true_sp) {
                concept_correct += 1;
            }
            concept_total += 1;
        }
    }
    println!(
        "concept predicts switch-2 behavior: {}/{}",
        concept_correct, concept_total
    );
    let tr = transfer_to_new_switch(n, 2, 0, &two_switch_step);
    println!(
        "transfer to new switch: probe cost {} vs cold-start {} (saved {})",
        tr.probe_cost, tr.cold_start_observations, tr.transfer_saved
    );

    // 4. Planning: raw full-space BFS vs factored component planning. Use a
    // larger world (6 switches) so the product-space search is non-trivial.
    let pn = 6usize;
    let pstart = [false; 6];
    let ptarget = [true; 6];
    let raw = plan_raw(pn, &pstart, &ptarget, &two_switch_step).unwrap_or(99);
    let fac = plan_factored(&pstart, &ptarget);
    println!(
        "planning to set all 6 switches: raw expansions={} factored expansions={}",
        raw, fac
    );

    // 5. Coupled control: a genuinely coupled world must not over-claim.
    let ctrans = all_transitions(n, &coupled_step);
    let (cobs, cheld) = parity_split(&ctrans);
    let cparents = discover_factors(n, &cobs);
    let crep = evaluate_generalization(n, &cobs, &cheld, &cparents);
    println!(
        "coupled control: parents={:?} held-out factored accuracy={:.3} (must be low)",
        cparents, crep.factored_accuracy
    );

    println!("{}", machine_record(&rep, &tr, raw, fac, &cparents));
}
