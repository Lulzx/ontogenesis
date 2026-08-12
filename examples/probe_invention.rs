// Ontogenesis experiment: invent observational probes (Direction D, §7).
//
// The agent holds a bounded set of executable candidate hypotheses (predicates
// over 4-bit inputs). They are observationally equivalent on every already
// measured probe, yet differ on some unmeasured input. The agent must invent
// which probe (which input to measure) separates them, scored by expected
// hypothesis reduction minus execution cost, and never seeing the hidden
// answer. Each measured result prunes the candidate set until either one
// hypothesis remains or the survivors are observationally equivalent under the
// probe language.
use std::collections::BTreeMap;

use supsearch::probe_invention::{self, generate_language, run, TOKENS};

fn main() {
    // World truth: inputs with bit 1 set (i.e., tokens 2,3,6,7,10,11,14,15).
    let truth: std::collections::BTreeSet<u8> =
        (0..TOKENS).filter(|x| (x >> 1) & 1 == 1).collect();

    // Already measured probes: a single token 2 (in the set).
    let mut measured: BTreeMap<u8, bool> = BTreeMap::new();
    measured.insert(2, true);

    let lang = generate_language();
    // consistent = agrees with the measured probe: token 2 is in the set.
    let consistent_count = lang.iter().filter(|h| h.contains(&2)).count();

    let report = run(&lang, &truth, &measured);

    println!("ontogenesis: invent observational probes (D)");
    println!(
        "predicate language size={} consistent with measured probes={}",
        lang.len(),
        consistent_count
    );
    println!(
        "initial_candidates={} final_candidates={}",
        report.initial_candidates, report.final_candidates
    );
    for s in &report.steps {
        println!(
            "  probe token {} (gain={}, value={}) -> label={} candidates {} -> {}",
            s.probe,
            s.gain,
            s.value,
            s.measured_label,
            s.candidates_before,
            s.candidates_after
        );
    }
    println!(
        "probes_run={} probes_that_reduced={} total_probe_cost={} total_information={}",
        report.probes_run,
        report.probes_that_reduced,
        report.total_probe_cost,
        report.total_information
    );
    println!(
        "true_recovered={} observationally_equivalent_stuck={}",
        report.true_recovered, report.observationally_equivalent_stuck
    );
    println!("{}", probe_invention::machine_record(&report));
}
