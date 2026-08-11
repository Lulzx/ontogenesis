use std::collections::BTreeMap;

use supsearch::{
    contextual_allocation::{ConceptSet, EvidenceDerivation},
    feature_invention::{
        freeze_feature_policy, invent_features, FeatureSelectionSpec, FeatureUtilityEvidence,
        RawExample, RawNode, RawTask,
    },
    search_accounting::{
        EngineWork, EvaluatorBudget, EvidencePhase, RunAccounting, RunProvenance, SearchEngine,
        TerminationStatus,
    },
};

fn chain(nodes: usize, label: i64) -> RawNode {
    (1..nodes).fold(RawNode::leaf(label), |tail, _| {
        RawNode::branch(label, vec![tail])
    })
}

fn task(id: &str, nodes: usize, surface_label: i64) -> RawTask {
    RawTask {
        task_id: id.into(),
        duplicate_group_id: id.into(),
        examples: vec![RawExample {
            inputs: vec![chain(nodes, surface_label)],
            published_output: None,
        }],
    }
}

fn accounting(task: &RawTask, work: u64) -> RunAccounting {
    RunAccounting {
        work: EngineWork::UniversalLambda {
            proposals: work,
            evaluated_candidates: 0,
            resource_points: 1,
        },
        max_structural_size: 7,
        evaluator_budget: EvaluatorBudget::LambdaFuel(100),
        solution_rank: Some(work),
        termination: TerminationStatus::Solved,
        provenance: RunProvenance {
            task_id: task.task_id.clone(),
            family_id: "raw-tree".into(),
            duplicate_group_id: task.duplicate_group_id.clone(),
            context_features: BTreeMap::new(),
            concept_ids: Vec::new(),
            phase: EvidencePhase::Training,
            observed_epoch: 1,
        },
    }
}

fn evidence(task: RawTask, concept: &str, useful: bool) -> FeatureUtilityEvidence {
    FeatureUtilityEvidence {
        without: accounting(&task, 100),
        with: accounting(&task, if useful { 10 } else { 100 }),
        task,
        concept_ids: vec![concept.into()],
        age: 0,
        recorded_epoch: 1,
        derivation: EvidenceDerivation::default(),
    }
}

fn add(records: &mut Vec<FeatureUtilityEvidence>, raw: RawTask, useful: &str) {
    records.push(evidence(raw.clone(), "A", useful == "A"));
    records.push(evidence(raw, "B", useful == "B"));
}

fn main() {
    let mut training = Vec::new();
    add(&mut training, task("train-even-2", 2, 10), "A");
    add(&mut training, task("train-even-4", 4, 20), "A");
    add(&mut training, task("train-odd-3", 3, 30), "B");
    add(&mut training, task("train-odd-5", 5, 40), "B");
    let mut calibration = Vec::new();
    add(&mut calibration, task("cal-even-6", 6, 50), "A");
    add(&mut calibration, task("cal-odd-7", 7, 60), "B");
    let concepts = [ConceptSet::singleton("A"), ConceptSet::singleton("B")];
    let spec = FeatureSelectionSpec {
        engine: SearchEngine::UniversalLambda,
        freeze_epoch: 1,
        decay_per_mille: 850,
        interactions: true,
        max_interaction_width: 2,
        max_program_size: 2,
        max_programs: 64,
        max_feature_width: 1,
        feature_pool_limit: 12,
        execution_fuel: 1_000,
        complexity_cost: 10,
        execution_cost: 1,
    };
    let report = invent_features(&training, &calibration, &concepts, &spec);
    let even = freeze_feature_policy(
        &report.encoder,
        &training,
        &task("held-even-100", 100, 999),
        &concepts,
        &spec,
    )
    .unwrap();
    let odd = freeze_feature_policy(
        &report.encoder,
        &training,
        &task("held-odd-101", 101, -999),
        &concepts,
        &spec,
    )
    .unwrap();
    println!("Executable context-feature invention from raw trees");
    println!(
        "phi={:?} retained={} regret={} primitive_regret={} collapsed_regret={}",
        report.encoder.programs,
        report.encoder.retained,
        report.encoder.calibration_regret,
        report.primitive_projection_regret,
        report.encoder.collapsed_regret,
    );
    println!(
        "held-even top={} held-odd top={} programs={} sets={} executions={} steps={}",
        even.ranked[0].concepts.0.join("+"),
        odd.ranked[0].concepts.0.join("+"),
        report.accounting.programs_enumerated,
        report.accounting.feature_sets_evaluated,
        report.accounting.task_executions,
        report.accounting.execution_steps,
    );
    println!(
        "record,engine=feature-programs,condition=controlled,retained={},regret={},primitive_regret={},collapsed_regret={},programs={},sets={},executions={},steps={},even_top={},odd_top={}",
        report.encoder.retained,
        report.encoder.calibration_regret,
        report.primitive_projection_regret,
        report.encoder.collapsed_regret,
        report.accounting.programs_enumerated,
        report.accounting.feature_sets_evaluated,
        report.accounting.task_executions,
        report.accounting.execution_steps,
        even.ranked[0].concepts.0.join("+"),
        odd.ranked[0].concepts.0.join("+"),
    );
}
