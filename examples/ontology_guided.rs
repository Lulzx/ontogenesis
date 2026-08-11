use supsearch::{
    ontology_guidance::{run_developmental_guidance, ConditionReport},
    term,
};

fn print_condition(condition: &ConditionReport) {
    let found = condition
        .outcome
        .candidate
        .as_ref()
        .map(|candidate| {
            format!(
                "size {}: {}",
                candidate.syntax_size,
                term::show(&candidate.functional)
            )
        })
        .unwrap_or_else(|| "not found in priority prefix".to_string());
    let metrics = &condition.outcome.metrics;
    println!(
        "{:<22} {:>10} proposals  {:>9} evaluated  sizes 1..={:<2}  fuel {:>6}  {:>8.3}s  {}",
        condition.name,
        metrics.proposals,
        metrics.evaluated_candidates,
        metrics.max_syntax_size,
        metrics.evaluation_fuel,
        metrics.wall_time.as_secs_f64(),
        found,
    );
}

fn main() {
    let report = run_developmental_guidance(11, 11);
    println!("Ontology-guided universal recursive search");
    println!(
        "negation independently validated: {} | target size: {} expanded -> {} acquired",
        report.parity.negation_validated,
        report.parity.expanded_target_size,
        report.parity.compressed_target_size,
    );
    print_condition(&report.parity.empty);
    print_condition(&report.parity.relevant);
    print_condition(&report.parity.irrelevant);
    print_condition(&report.parity.misleading);
    println!(
        "counterfactual proposal gain: {}x ({})",
        report.parity.gain.ratio(),
        if report.parity.gain.earns(100) {
            "ACQUIRE"
        } else {
            "REJECT"
        }
    );
    println!("\nAcquired recursive parity guides a distinct nested recursive law");
    println!(
        "target size: {} expanded -> {} acquired",
        report.nested.expanded_target_size, report.nested.compressed_target_size,
    );
    print_condition(&report.nested.prior_ontology);
    print_condition(&report.nested.acquired_recursive);
    print_condition(&report.nested.irrelevant_recursive);
    print_condition(&report.nested.misleading_recursive);
    println!(
        "counterfactual proposal gain: {}x ({})",
        report.nested.gain.ratio(),
        if report.nested.gain.earns(100) {
            "ACQUIRE"
        } else {
            "REJECT"
        }
    );
    println!("after this finite prefix, the unchanged universal dovetail is the fallback");
}
