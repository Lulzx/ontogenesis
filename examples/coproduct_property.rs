use std::collections::BTreeSet;
use supsearch::{
    coproduct_property::{
        bounded_uniqueness, commutes, cost_geometry, decide_acquisition, default_spec, discover,
        measure_downstream, measure_irrelevant, measure_oracle, measure_pure_universal,
        measure_uniform, sample_evidence, DownstreamTask,
    },
    term,
};

fn main() {
    let (training, calibration, protected) = sample_evidence();
    let protected_ids = protected
        .iter()
        .map(|item| item.id.clone())
        .collect::<BTreeSet<_>>();
    let spec = default_spec();
    let report = discover(&training, &calibration, &protected_ids, &spec);
    let structure = report
        .structure
        .as_ref()
        .expect("U2 discovery should succeed");
    let mut protected_checks = 0;
    let protected_commutes = protected
        .iter()
        .all(|item| commutes(structure, item, &mut protected_checks));
    let uniqueness = protected
        .iter()
        .map(|item| bounded_uniqueness(structure, item, &spec))
        .collect::<Vec<_>>();
    let protected_unique = uniqueness.iter().all(|item| item.unique);

    let raw = measure_downstream(DownstreamTask::MapBranches, structure, false, 20, 50_000);
    let learned = measure_downstream(DownstreamTask::MapBranches, structure, true, 20, 50_000);
    let uniform = measure_uniform(DownstreamTask::MapBranches, structure, 20, 50_000);
    let irrelevant = measure_irrelevant(DownstreamTask::MapBranches, structure, 20, 50_000);
    let oracle = measure_oracle(DownstreamTask::MapBranches, structure);
    let universal = measure_pure_universal(DownstreamTask::MapBranches, structure, 8);
    let identity_raw =
        measure_downstream(DownstreamTask::IdentityControl, structure, false, 4, 50_000);
    let identity_learned =
        measure_downstream(DownstreamTask::IdentityControl, structure, true, 4, 50_000);
    let geometry = cost_geometry(
        report.charged_discovery_cost,
        std::slice::from_ref(&uniform),
        std::slice::from_ref(&learned),
        10_000,
    );
    let decision = decide_acquisition(&geometry);
    let regret = learned.proposals.saturating_sub(oracle.proposals);
    let ratio = uniform.proposals / learned.proposals.max(1);

    println!("U2 bounded coproduct-property ontogenesis");
    println!(
        "embed_left={} embed_right={} generator={}",
        term::show(&structure.embed_left),
        term::show(&structure.embed_right),
        term::show(&structure.generator)
    );
    println!("protected_commutes={protected_commutes} protected_unique={protected_unique} uniqueness={uniqueness:?}");
    println!("raw={raw:?} learned={learned:?} uniform={uniform:?} irrelevant={irrelevant:?} oracle={oracle:?} universal={universal:?}");
    println!(
        "negative_transfer_raw={identity_raw:?} negative_transfer_learned={identity_learned:?}"
    );
    println!(
        "record,experiment=u2,embed_left_size={},embed_right_size={},generator_size={},embedding_terms={},embedding_pairs={},generator_terms={},mediator_terms={},normalization_checks={},equation_checks={},equivalence_checks={},rejected_unsafe={},rejected_nonunique={},calibration_commutes={},calibration_unique={},protected_commutes={},protected_unique={},protected_valid_mediators={},protected_max_classes={},uniqueness_exhaustive={},syntax_baseline={},raw_typed_solved={},raw_typed_proposals={},learned_solved={},learned_proposals={},learned_generated={},uniform_solved={},uniform_proposals={},irrelevant_solved={},irrelevant_proposals={},oracle_solved={},oracle_proposals={},proposal_regret={},uniform_ratio={},universal_solved={},universal_proposals={},universal_ratio_lower_bound={},discovery_charge={},protected_uses={},net_gain={},retained={},learned_budget={},triangle_holds={},negative_transfer_raw={},negative_transfer_learned={},termination={:?}",
        structure.embed_left.size(), structure.embed_right.size(), structure.generator.size(),
        report.accounting.embedding_terms, report.accounting.embedding_pairs,
        report.accounting.generator_terms, report.accounting.mediator_terms,
        report.accounting.normalization_checks, report.accounting.equation_checks,
        report.accounting.equivalence_checks,
        report.accounting.rejected_unsafe, report.accounting.rejected_nonunique,
        report.calibration_commutes, report.calibration_unique, protected_commutes, protected_unique,
        uniqueness.iter().map(|x| x.valid_mediators).sum::<usize>(),
        uniqueness.iter().map(|x| x.equivalence_classes).max().unwrap_or(0),
        uniqueness.iter().all(|x| x.exhaustive_within_size), report.syntax_baseline_found,
        raw.solved, raw.proposals, learned.solved, learned.proposals, learned.generated_candidates,
        uniform.solved, uniform.proposals, irrelevant.solved, irrelevant.proposals,
        oracle.solved, oracle.proposals, regret, ratio, universal.solved, universal.proposals,
        universal.proposals / learned.proposals.max(1), report.charged_discovery_cost,
        geometry.protected_uses, geometry.net_gain, decision.retained, decision.learned_budget_units,
        geometry.triangle_holds, identity_raw.proposals, identity_learned.proposals, report.termination,
    );
}
