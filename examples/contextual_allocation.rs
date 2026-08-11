use supsearch::{
    contextual_guidance::{run_contextual_guidance, run_interaction_guidance, ContextCondition},
    search_accounting::EngineWork,
};

fn print_condition(condition: &ContextCondition) {
    let (proposals, evaluated, points) = match condition.accounting.work {
        EngineWork::UniversalLambda {
            proposals,
            evaluated_candidates,
            resource_points,
        } => (proposals, evaluated_candidates, resource_points),
        _ => unreachable!(),
    };
    println!(
        "{:<25} solved={}/{} proposals={:<7} evaluated={:<7} points={:<3} universal={}",
        condition.name,
        condition.solved_tasks(),
        condition.tasks.len(),
        proposals,
        evaluated,
        points,
        condition.coverage_preserved,
    );
    println!(
        "record,condition={},solved={},tasks={},proposals={},evaluated={},resource_points={},universal={}",
        condition.name,
        condition.solved_tasks(),
        condition.tasks.len(),
        proposals,
        evaluated,
        points,
        condition.coverage_preserved,
    );
}

fn main() {
    let report = run_contextual_guidance();
    println!("Contextual utility over heterogeneous recursive representations");
    println!(
        "learned encoder={:?} regret={} collapsed_regret={} encoder_candidates={}",
        report.learned_representation.encoder.kind,
        report.learned_representation.encoder.calibration_regret,
        report.learned_representation.encoder.collapsed_regret,
        report
            .learned_representation
            .accounting
            .candidates_evaluated,
    );
    println!(
        "learned single top={} | learned nested top={} | hand single={} | hand nested={} | global={}",
        report.learned_single_policy.ranked[0].concepts.0.join("+"),
        report.learned_nested_policy.ranked[0].concepts.0.join("+"),
        report.single_policy.ranked[0].concepts.0.join("+"),
        report.nested_policy.ranked[0].concepts.0.join("+"),
        report.global_policy.ranked[0].concepts.0.join("+"),
    );
    println!(
        "record,condition=encoder,kind={:?},regret={},collapsed_regret={},candidates={},predictions={},fields_inspected={}",
        report.learned_representation.encoder.kind,
        report.learned_representation.encoder.calibration_regret,
        report.learned_representation.encoder.collapsed_regret,
        report.learned_representation.accounting.candidates_evaluated,
        report.learned_representation.accounting.validation_predictions,
        report.learned_representation.accounting.raw_fields_inspected,
    );
    println!(
        "record,engine=universal-lambda,condition=encoder-evidence,primary_work={},universal=false",
        report
            .encoder_evidence_accounting
            .work
            .comparable_primary_work(),
    );
    for condition in [
        &report.learned,
        &report.contextual,
        &report.global,
        &report.uniform,
        &report.oracle,
        &report.shuffled_labels,
        &report.irrelevant,
        &report.misleading,
        &report.universal_only,
        &report.contextual_without_universal,
    ] {
        print_condition(condition);
    }
    println!(
        "contextual proposal regret versus oracle: {}",
        report.contextual_regret_vs_oracle
    );

    let interaction = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let report = run_interaction_guidance();
            (
                report.policy.ranked[0].concepts.0.join("+"),
                report.policy.ranked[0].interaction_residual,
                report.interaction.candidate.is_some(),
                report.first_only.candidate.is_some(),
                report.second_only.candidate.is_some(),
                report.interaction_disabled.candidate.is_some(),
            )
        })
        .unwrap()
        .join()
        .unwrap();
    println!("\nBounded concept-set interaction");
    println!(
        "top={} residual={} solved={} first_only={} second_only={} interaction_disabled={}",
        interaction.0, interaction.1, interaction.2, interaction.3, interaction.4, interaction.5,
    );
    println!(
        "record,condition=interaction,top={},residual={},solved={},first_only={},second_only={},interaction_disabled={}",
        interaction.0,
        interaction.1,
        interaction.2,
        interaction.3,
        interaction.4,
        interaction.5,
    );
}
