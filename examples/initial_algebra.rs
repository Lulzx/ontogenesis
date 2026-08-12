use std::collections::BTreeSet;
use supsearch::{
    initial_algebra::{
        bounded_uniqueness, commutes, cost_geometry, decide_acquisition, default_spec, discover,
        measure_downstream, measure_irrelevant, measure_oracle, measure_pure_universal,
        measure_recurrence_only, measure_uniform, sample_algebras, DownstreamTask,
    },
    term,
};

fn main() {
    let (training, calibration, protected) = sample_algebras();
    let ids = protected
        .iter()
        .map(|e| e.id.clone())
        .collect::<BTreeSet<_>>();
    let spec = default_spec();
    let report = discover(&training, &calibration, &ids, &spec);
    let structure = report
        .structure
        .as_ref()
        .expect("U3 discovery should succeed");
    let mut protected_checks = 0;
    let protected_commutes = protected
        .iter()
        .all(|e| commutes(structure, e, &mut protected_checks));
    let uniqueness = protected
        .iter()
        .map(|e| bounded_uniqueness(structure, e, &spec))
        .collect::<Vec<_>>();
    let protected_unique = uniqueness.iter().all(|u| u.unique);

    let raw = measure_downstream(DownstreamTask::DoubleCarrier, structure, false, 16, 50_000);
    let learned = measure_downstream(DownstreamTask::DoubleCarrier, structure, true, 16, 50_000);
    let uniform = measure_uniform(DownstreamTask::DoubleCarrier, structure, 16, 50_000);
    let irrelevant = measure_irrelevant(DownstreamTask::DoubleCarrier, structure, 16, 50_000);
    let oracle = measure_oracle(DownstreamTask::DoubleCarrier, structure);
    let universal = measure_pure_universal(DownstreamTask::DoubleCarrier, structure, 8);
    let recurrence = measure_recurrence_only(structure, &protected);
    let identity_raw =
        measure_downstream(DownstreamTask::IdentityControl, structure, false, 4, 50_000);
    let identity_learned =
        measure_downstream(DownstreamTask::IdentityControl, structure, true, 4, 50_000);
    let geometry = cost_geometry(
        report.charged_discovery_cost,
        std::slice::from_ref(&uniform),
        std::slice::from_ref(&learned),
        100_000,
    );
    let decision = decide_acquisition(&geometry);

    println!("U3 bounded initial-algebra/catamorphism ontogenesis");
    println!(
        "carrier={} step={} constructor={} generator={} F={}",
        term::show(&structure.carrier_witness),
        term::show(&structure.carrier_step),
        term::show(&structure.constructor),
        term::show(&structure.generator),
        term::show(&structure.f_action)
    );
    println!("protected_commutes={protected_commutes} protected_unique={protected_unique} uniqueness={uniqueness:?}");
    println!("raw={raw:?} learned={learned:?} uniform={uniform:?} irrelevant={irrelevant:?} oracle={oracle:?} recurrence={recurrence:?} universal={universal:?}");
    println!("identity_raw={identity_raw:?} identity_learned={identity_learned:?}");
    println!(
        "record,experiment=u3,carrier_size={},step_size={},constructor_size={},generator_size={},f_action_size={},base_terms={},step_terms={},carrier_pairs={},constructor_terms={},generator_terms={},mediator_terms={},generated_candidates={},evaluated_candidates={},observation_checks={},equation_checks={},equivalence_checks={},rejected_unsafe={},rejected_nonunique={},calibration_commutes={},calibration_unique={},protected_commutes={},protected_unique={},protected_valid_mediators={},protected_max_classes={},uniqueness_exhaustive={},syntax_baseline={},recurrence_subtree_baseline={},raw_solved={},raw_proposals={},learned_solved={},learned_proposals={},learned_generated={},uniform_solved={},uniform_proposals={},irrelevant_solved={},irrelevant_proposals={},oracle_solved={},oracle_proposals={},oracle_regret={},recurrence_solved={},recurrence_proposals={},recurrence_checks={},universal_solved={},universal_proposals={},universal_ratio_lower_bound={},discovery_charge={},protected_uses={},net_gain={},retained={},learned_budget={},triangle_holds={},identity_raw_proposals={},identity_learned_proposals={},termination={:?}",
        structure.carrier_witness.size(), structure.carrier_step.size(), structure.constructor.size(),
        structure.generator.size(), structure.f_action.size(), report.accounting.base_terms,
        report.accounting.step_terms, report.accounting.carrier_pairs, report.accounting.constructor_terms,
        report.accounting.generator_terms, report.accounting.mediator_terms, report.accounting.generated_candidates,
        report.accounting.evaluated_candidates, report.accounting.observation_checks, report.accounting.equation_checks,
        report.accounting.equivalence_checks, report.accounting.rejected_unsafe, report.accounting.rejected_nonunique,
        report.calibration_commutes, report.calibration_unique, protected_commutes, protected_unique,
        uniqueness.iter().map(|u|u.valid_mediators).sum::<usize>(), uniqueness.iter().map(|u|u.equivalence_classes).max().unwrap_or(0),
        uniqueness.iter().all(|u|u.exhaustive_within_size), report.syntax_baseline_found, report.recurrence_subtree_found,
        raw.solved, raw.proposals, learned.solved, learned.proposals, learned.generated_candidates,
        uniform.solved, uniform.proposals, irrelevant.solved, irrelevant.proposals, oracle.solved, oracle.proposals,
        learned.proposals.saturating_sub(oracle.proposals),
        recurrence.solved, recurrence.proposals, recurrence.observation_checks,
        universal.solved, universal.proposals,
        universal.proposals/learned.proposals.max(1), report.charged_discovery_cost, geometry.protected_uses,
        geometry.net_gain, decision.retained, decision.learned_budget_units, geometry.triangle_holds,
        identity_raw.proposals, identity_learned.proposals, report.termination,
    );
}
