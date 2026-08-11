use supsearch::ontology_guidance::{run_learned_allocation, AllocationConditionReport};

fn print_condition(condition: &AllocationConditionReport) {
    let metrics = &condition.outcome.metrics;
    println!(
        "{:<23} {:>9} proposals  {:>8} evaluated  {:>3} points  {:>7.3}s  solved={:<5}  universal={}  lanes={}",
        condition.name,
        metrics.proposals,
        metrics.evaluated_candidates,
        metrics.resource_points,
        metrics.wall_time.as_secs_f64(),
        condition.outcome.candidate.is_some(),
        condition.coverage_preserved,
        condition.lane_order.join(" -> "),
    );
}

fn main() {
    let report = run_learned_allocation(11, 11, 11);

    println!("Learned ontology allocation from prior counterfactual utility");
    println!("weights (held-out task excluded):");
    for weight in &report.weights.ranked {
        let decision = report
            .decisions
            .iter()
            .find(|decision| decision.concept_id == weight.concept_id)
            .unwrap();
        println!(
            "  {:<20} score={:>9} evidence={}  size<= {}  fuel={}",
            weight.concept_id,
            weight.score,
            weight.evidence_count,
            decision.max_syntax_size,
            decision.evaluation_fuel,
        );
    }
    println!(
        "held-out target: {} expanded -> {} with acquired parity",
        report.heldout_expanded_target_size, report.heldout_compressed_target_size
    );
    print_condition(&report.universal_only);
    print_condition(&report.uniform);
    print_condition(&report.hand_designed);
    print_condition(&report.learned);
    print_condition(&report.irrelevant);
    print_condition(&report.misleading);
    print_condition(&report.learned_without_universal);

    let lower_bound = report.universal_only.outcome.metrics.proposals
        / report.learned.outcome.metrics.proposals.max(1);
    println!(
        "calibration margin={}  proposal regret vs hand={}  widening saved vs uniform={}",
        report.calibration_margin, report.proposal_regret_vs_hand, report.widening_saved_vs_uniform,
    );
    println!(
        "universal-only lower-bound separation: >= {}x (baseline solved={})",
        lower_bound,
        report.universal_only.outcome.candidate.is_some()
    );
    println!(
        "held-out leakage attempt left ranking unchanged: {} (skipped held-out evidence={})",
        report.leakage_ranking_unchanged, report.leakage_evidence_skipped,
    );
    println!(
        "the learned policy alternates with the unchanged universal dovetail; the no-universal ablation does not preserve coverage"
    );
}
