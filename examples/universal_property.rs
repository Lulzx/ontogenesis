use std::collections::BTreeSet;
use supsearch::{
    term,
    universal_property::{
        bounded_uniqueness, commutes, cost_geometry, decide_acquisition, default_spec, discover,
        measure_downstream, measure_irrelevant_downstream, measure_oracle_downstream,
        measure_uniform_downstream, measure_universal_downstream, measure_universal_with_structure,
        sample_cones, sampled_composition_costs, DownstreamTask,
    },
};

fn main() {
    let (training, calibration, protected) = sample_cones();
    let protected_ids = protected
        .iter()
        .map(|cone| cone.id.clone())
        .collect::<BTreeSet<_>>();
    let spec = default_spec();
    let report = discover(&training, &calibration, &protected_ids, &spec);
    let structure = report
        .structure
        .as_ref()
        .expect("bounded relational search should discover U1");
    let mut checks = 0;
    let held_commutes = protected
        .iter()
        .all(|cone| commutes(structure, cone, &mut checks));
    let held_uniqueness = protected
        .iter()
        .map(|cone| bounded_uniqueness(structure, cone, &spec))
        .collect::<Vec<_>>();
    let held_unique = held_uniqueness.iter().all(|report| report.unique);
    let base = [
        measure_downstream(DownstreamTask::Swap, structure, false, 14, 50_000),
        measure_downstream(DownstreamTask::MapBoth, structure, false, 20, 50_000),
    ];
    let acquired = [
        measure_downstream(DownstreamTask::Swap, structure, true, 14, 50_000),
        measure_downstream(DownstreamTask::MapBoth, structure, true, 20, 50_000),
    ];
    let uniform = measure_uniform_downstream(DownstreamTask::Swap, structure, 14, 50_000);
    let irrelevant = measure_irrelevant_downstream(DownstreamTask::Swap, structure, 14, 50_000);
    let oracle = measure_oracle_downstream(DownstreamTask::Swap, structure);
    let universal = measure_universal_downstream(DownstreamTask::Swap, structure, 10);
    let universal_learned = measure_universal_with_structure(DownstreamTask::Swap, structure, 10);
    // The gate is evaluated on the protected reuse family where U1 helps.
    // MapBoth remains reported as a negative-transfer control.
    let geometry = cost_geometry(
        report.charged_discovery_cost,
        &base[..1],
        &acquired[..1],
        1_000,
    );
    let triangle = sampled_composition_costs();
    let decision = decide_acquisition(&geometry);
    let saved_per_use = base[0]
        .observation_checks
        .saturating_sub(acquired[0].observation_checks);
    let break_even_uses = report
        .charged_discovery_cost
        .saturating_add(saved_per_use.saturating_sub(1))
        / saved_per_use.max(1);
    let universal_acquisition_proposals = report
        .accounting
        .carrier_terms
        .saturating_add(report.accounting.observer_terms);
    let universal_proposal_net_lower_bound = i128::from(universal.proposals)
        - i128::from(universal_learned.proposals)
        - i128::from(universal_acquisition_proposals);
    let typed_acquisition_proposals = report
        .accounting
        .generator_terms
        .saturating_add(report.accounting.mediator_terms);
    let typed_proposal_net = i128::from(
        base[0]
            .proposals
            .saturating_sub(acquired[0].proposals)
            .saturating_mul(geometry.protected_uses),
    ) - i128::from(typed_acquisition_proposals);
    let proposal_regret = acquired[0].proposals.saturating_sub(oracle.proposals);

    println!("U1 bounded universal-property ontogenesis");
    println!(
        "carrier={} observe_a={} observe_b={} generator={}",
        term::show(&structure.carrier),
        term::show(&structure.observe_a),
        term::show(&structure.observe_b),
        term::show(&structure.generator),
    );
    println!(
        "calibration_commutes={} calibration_unique={} held_commutes={} held_unique={} syntax_baseline={}",
        report.calibration_commutes,
        report.calibration_unique,
        held_commutes,
        held_unique,
        report.syntax_baseline_found,
    );
    println!(
        "held_mediators={:?} triangle_samples={triangle:?} break_even_uses={break_even_uses}",
        held_uniqueness
            .iter()
            .map(|report| (
                report.valid_mediators,
                report.equivalence_classes,
                report.exhaustive_within_size
            ))
            .collect::<Vec<_>>()
    );
    println!(
        "accounting_vector: observation_net={} typed_proposal_net={} universal_proposal_net_lower_bound={}",
        geometry.net_gain, typed_proposal_net, universal_proposal_net_lower_bound,
    );
    println!(
        "base={base:?} acquired={acquired:?} uniform={uniform:?} irrelevant={irrelevant:?} oracle={oracle:?} universal_size10={universal:?} universal_learned={universal_learned:?} discovery_charge={} net_gain={} retained={} triangle={}",
        report.charged_discovery_cost,
        geometry.net_gain,
        decision.retained,
        geometry.triangle_holds,
    );
    println!(
        "record,experiment=u1,carrier_size={},observer_a_size={},observer_b_size={},generator_size={},carrier_terms={},observer_terms={},factorizations={},generator_terms={},mediator_terms={},normalization_checks={},equivalence_checks={},held_commutes={},held_unique={},held_valid_mediators={},held_max_equivalence_classes={},uniqueness_exhaustive={},syntax_baseline={},universal_size10_solved={},universal_size10_proposals={},universal_learned_solved={},universal_learned_proposals={},universal_learned_generated={},base_swap_proposals={},base_swap_generated={},learned_swap_proposals={},learned_swap_generated={},uniform_swap_proposals={},irrelevant_swap_proposals={},oracle_swap_proposals={},proposal_regret={},raw_typed_ratio={},universal_ratio_lower_bound={},solved_conditions={},total_conditions={},base_map_proposals={},base_map_generated={},learned_map_proposals={},learned_map_generated={},discovery_charge={},protected_uses={},break_even_uses={},observation_net={},typed_proposal_net={},universal_proposal_net_lower_bound={},retained={},learned_budget={},triangle_holds={},discovery_termination={:?},universal_termination={:?},learned_termination={:?}",
        structure.carrier.size(),
        structure.observe_a.size(),
        structure.observe_b.size(),
        structure.generator.size(),
        report.accounting.carrier_terms,
        report.accounting.observer_terms,
        report.accounting.factorization_candidates,
        report.accounting.generator_terms,
        report.accounting.mediator_terms,
        report.accounting.normalization_checks,
        report.accounting.equivalence_checks,
        held_commutes,
        held_unique,
        held_uniqueness
            .iter()
            .map(|report| report.valid_mediators)
            .sum::<usize>(),
        held_uniqueness
            .iter()
            .map(|report| report.equivalence_classes)
            .max()
            .unwrap_or(0),
        held_uniqueness
            .iter()
            .all(|report| report.exhaustive_within_size),
        report.syntax_baseline_found,
        universal.solved,
        universal.proposals,
        universal_learned.solved,
        universal_learned.proposals,
        universal_learned.generated_candidates,
        base[0].proposals,
        base[0].generated_candidates,
        acquired[0].proposals,
        acquired[0].generated_candidates,
        uniform.proposals,
        irrelevant.proposals,
        oracle.proposals,
        proposal_regret,
        base[0].proposals / acquired[0].proposals.max(1),
        universal.proposals / universal_learned.proposals.max(1),
        [
            &base[0],
            &acquired[0],
            &uniform,
            &irrelevant,
            &oracle
        ]
        .iter()
        .filter(|run| run.solved)
        .count(),
        5,
        base[1].proposals,
        base[1].generated_candidates,
        acquired[1].proposals,
        acquired[1].generated_candidates,
        report.charged_discovery_cost,
        geometry.protected_uses,
        break_even_uses,
        geometry.net_gain,
        typed_proposal_net,
        universal_proposal_net_lower_bound,
        decision.retained,
        decision.learned_budget_units,
        geometry.triangle_holds,
        report.termination,
        universal.termination,
        acquired[0].termination,
    );
}
