//! Heterogeneous recursive-search experiment for contextual utility.
//!
//! Two input-only representation contexts require different acquired concepts.
//! Policies are learned from earlier tasks and frozen before disjoint reversed-
//! protocol holdouts are searched.

use crate::{
    contextual_allocation::{
        ConceptSet, ContextualEvidence, ContextualLedger, EvidenceDerivation, FreezeSpec,
        FrozenPolicy, TaskContext,
    },
    learned_context::{
        freeze_policy as freeze_learned_policy, learn_representation, LearnedRepresentation,
        RawField, RawTaskObservation, RawUtilityEvidence, RepresentationSpec,
    },
    ontology_guidance::{
        self, boolean_identity, boolean_not, heldout_problem, irrelevant_pair_constructor,
        nested_problem, parity_problem, run_developmental_guidance, EXPERIMENT_FUEL,
    },
    recursion_search::{self, Example, SearchMetrics, SearchOutcome, SearchProblem},
    search_accounting::{self, AccountingSummary, EvidencePhase, RunAccounting, RunProvenance},
    term::{self, Term},
    universal::{InterleavedDovetail, ResourceLane},
};
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct ContextTaskResult {
    pub task_id: &'static str,
    pub expected_concept: &'static str,
    pub lane_order: Vec<String>,
    pub outcome: SearchOutcome,
}

#[derive(Clone, Debug)]
pub struct ContextCondition {
    pub name: &'static str,
    pub tasks: Vec<ContextTaskResult>,
    pub accounting: AccountingSummary,
    pub coverage_preserved: bool,
}

impl ContextCondition {
    pub fn solved_tasks(&self) -> usize {
        self.tasks
            .iter()
            .filter(|task| task.outcome.candidate.is_some())
            .count()
    }

    pub fn proposals(&self) -> u64 {
        match self.accounting.work {
            search_accounting::EngineWork::UniversalLambda { proposals, .. } => proposals,
            _ => unreachable!("synthetic guidance uses universal accounting"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ContextualGuidanceReport {
    pub learned_representation: LearnedRepresentation,
    pub encoder_evidence_accounting: AccountingSummary,
    pub learned_single_policy: FrozenPolicy,
    pub learned_nested_policy: FrozenPolicy,
    pub learned: ContextCondition,
    pub single_policy: FrozenPolicy,
    pub nested_policy: FrozenPolicy,
    pub global_policy: FrozenPolicy,
    pub contextual: ContextCondition,
    pub global: ContextCondition,
    pub uniform: ContextCondition,
    pub oracle: ContextCondition,
    pub shuffled_labels: ContextCondition,
    pub irrelevant: ContextCondition,
    pub misleading: ContextCondition,
    pub universal_only: ContextCondition,
    pub contextual_without_universal: ContextCondition,
    pub contextual_regret_vs_oracle: u64,
}

#[derive(Clone, Debug)]
pub struct InteractionGuidanceReport {
    pub policy: FrozenPolicy,
    pub interaction: SearchOutcome,
    pub interaction_disabled: SearchOutcome,
    pub first_only: SearchOutcome,
    pub second_only: SearchOutcome,
    pub universal_only: SearchOutcome,
}

type ProblemFactory = fn(Vec<Rc<Term>>) -> SearchProblem;

fn context(task: &str, family: &str, representation: &str) -> TaskContext {
    TaskContext {
        task_id: task.into(),
        family_id: family.into(),
        duplicate_group_id: task.into(),
        features: BTreeMap::from([("input-representation".into(), representation.into())]),
    }
}

#[derive(Default)]
struct StructuralStats {
    nodes: i64,
    lambdas: i64,
    applications: i64,
    max_app_spine: i64,
    max_lambda_prefix: i64,
}

fn collect_stats(term: &Term, stats: &mut StructuralStats) {
    stats.nodes += 1;
    let mut prefix = 0;
    let mut cursor = term;
    while let Term::Lam(body) = cursor {
        prefix += 1;
        cursor = body;
    }
    stats.max_lambda_prefix = stats.max_lambda_prefix.max(prefix);
    let mut spine = 0;
    let mut cursor = term;
    while let Term::App(function, _) = cursor {
        spine += 1;
        cursor = function;
    }
    stats.max_app_spine = stats.max_app_spine.max(spine);
    match term {
        Term::Lam(body) | Term::Prim(body) => {
            if matches!(term, Term::Lam(_)) {
                stats.lambdas += 1;
            }
            collect_stats(body, stats);
        }
        Term::App(function, argument) => {
            stats.applications += 1;
            collect_stats(function, stats);
            collect_stats(argument, stats);
        }
        Term::Var(_) | Term::Free(_) => {}
    }
}

/// Generic input-only measurements. No field names a task family, target law,
/// useful concept, expected output, or held-out identity.
fn raw_problem_observation(
    task_id: &str,
    duplicate_group_id: &str,
    problem: &SearchProblem,
) -> RawTaskObservation {
    let mut stats = StructuralStats::default();
    for example in problem.discovery.iter().chain(&problem.extrapolation) {
        for argument in &example.arguments {
            collect_stats(argument, &mut stats);
        }
    }
    RawTaskObservation {
        task_id: task_id.into(),
        duplicate_group_id: duplicate_group_id.into(),
        fields: BTreeMap::from([
            ("raw-0".into(), RawField::observable(stats.max_app_spine, 1)),
            (
                "raw-1".into(),
                RawField::observable(stats.max_lambda_prefix, 1),
            ),
            ("raw-2".into(), RawField::observable(stats.nodes, 1)),
            ("raw-3".into(), RawField::observable(stats.lambdas, 1)),
            ("raw-4".into(), RawField::observable(stats.applications, 1)),
            (
                "raw-5".into(),
                RawField::observable(problem.discovery.len() as i64, 1),
            ),
        ]),
    }
}

fn raw_evidence(
    observation: RawTaskObservation,
    concepts: &[&str],
    without: &SearchOutcome,
    with: &SearchOutcome,
) -> RawUtilityEvidence {
    let context = TaskContext {
        task_id: observation.task_id.clone(),
        family_id: "raw-observation".into(),
        duplicate_group_id: observation.duplicate_group_id.clone(),
        features: BTreeMap::new(),
    };
    let ids = concepts
        .iter()
        .map(|id| (*id).to_string())
        .collect::<Vec<_>>();
    RawUtilityEvidence {
        without: RunAccounting::from_universal(
            without,
            provenance(&context, &[], EvidencePhase::Training),
        ),
        with: RunAccounting::from_universal(
            with,
            provenance(&context, &ids, EvidencePhase::Training),
        ),
        observation,
        concept_ids: ids,
        age: 0,
        recorded_epoch: 1,
        derivation: Default::default(),
    }
}

fn provenance(ctx: &TaskContext, concepts: &[String], phase: EvidencePhase) -> RunProvenance {
    RunProvenance {
        task_id: ctx.task_id.clone(),
        family_id: ctx.family_id.clone(),
        duplicate_group_id: ctx.duplicate_group_id.clone(),
        context_features: ctx.features.clone(),
        concept_ids: concepts.to_vec(),
        phase,
        observed_epoch: 1,
    }
}

fn evidence(
    ctx: TaskContext,
    concepts: &[&str],
    without: &SearchOutcome,
    with: &SearchOutcome,
) -> ContextualEvidence {
    let ids = concepts
        .iter()
        .map(|id| (*id).to_string())
        .collect::<Vec<_>>();
    ContextualEvidence {
        without: RunAccounting::from_universal(
            without,
            provenance(&ctx, &[], EvidencePhase::Training),
        ),
        with: RunAccounting::from_universal(with, provenance(&ctx, &ids, EvidencePhase::Training)),
        context: ctx,
        concept_ids: ids,
        age: 0,
        recorded_epoch: 1,
        derivation: EvidenceDerivation::default(),
    }
}

fn reversed_parity_chain(depth: u32) -> Rc<Term> {
    (0..depth).fold(
        term::lam(term::lam(ontology_guidance::church_bool(false))),
        |tail, _| {
            // λrecursive.λstep. step (recursive tail)
            term::lam(term::lam(term::app(
                term::var(0),
                term::app(term::var(1), tail),
            )))
        },
    )
}

fn reversed_parity_problem(atoms: Vec<Rc<Term>>) -> SearchProblem {
    let example = |depth| Example {
        arguments: vec![reversed_parity_chain(depth)],
        expected: ontology_guidance::church_bool(depth % 2 == 1),
    };
    SearchProblem {
        atoms,
        discovery: [0, 2, 3, 4].into_iter().map(example).collect(),
        extrapolation: [5, 7, 9].into_iter().map(example).collect(),
        require_recursive_reference: true,
        require_load_bearing_recursion: true,
        require_distinct_outputs: true,
    }
}

fn constant_true_step() -> Rc<Term> {
    term::lam(ontology_guidance::church_bool(true))
}

fn two_step_chain(tags: &[bool]) -> Rc<Term> {
    tags.iter().rev().fold(
        term::lam(term::lam(term::lam(ontology_guidance::church_bool(false)))),
        |tail, first| {
            // λfirst.λsecond.λrecursive. selected_step (recursive tail)
            let step = if *first { term::var(2) } else { term::var(1) };
            term::lam(term::lam(term::lam(term::app(
                step,
                term::app(term::var(0), tail),
            ))))
        },
    )
}

fn two_step_expected(tags: &[bool]) -> Rc<Term> {
    let value = tags
        .iter()
        .rev()
        .fold(false, |tail, first| if *first { !tail } else { true });
    ontology_guidance::church_bool(value)
}

fn two_step_training_problem(atoms: Vec<Rc<Term>>) -> SearchProblem {
    let example = |tags: &[bool]| Example {
        arguments: vec![two_step_chain(tags)],
        expected: two_step_expected(tags),
    };
    SearchProblem {
        atoms,
        discovery: [vec![], vec![true], vec![false], vec![true, false]]
            .iter()
            .map(|tags| example(tags))
            .collect(),
        extrapolation: [
            vec![true, true],
            vec![false, false],
            vec![false, true, true],
            vec![true, false, true, false, true],
        ]
        .iter()
        .map(|tags| example(tags))
        .collect(),
        require_recursive_reference: true,
        require_load_bearing_recursion: true,
        require_distinct_outputs: true,
    }
}

fn two_step_holdout_problem(atoms: Vec<Rc<Term>>) -> SearchProblem {
    let example = |tags: &[bool]| Example {
        arguments: vec![two_step_chain(tags)],
        expected: two_step_expected(tags),
    };
    SearchProblem {
        atoms,
        discovery: [
            vec![true, true, false],
            vec![false, true],
            vec![true, false, false],
        ]
        .iter()
        .map(|tags| example(tags))
        .collect(),
        extrapolation: [
            vec![true, true, true],
            vec![false, false, true],
            vec![false, true, false],
        ]
        .iter()
        .map(|tags| example(tags))
        .collect(),
        require_recursive_reference: true,
        require_load_bearing_recursion: true,
        require_distinct_outputs: true,
    }
}

fn add_metrics(total: &mut SearchMetrics, part: &SearchMetrics) {
    total.resource_points += part.resource_points;
    total.proposals += part.proposals;
    total.evaluated_candidates += part.evaluated_candidates;
    total.max_syntax_size = total.max_syntax_size.max(part.max_syntax_size);
    total.evaluation_fuel = total.evaluation_fuel.max(part.evaluation_fuel);
    total.wall_time += part.wall_time;
}

fn run_lane(
    factory: ProblemFactory,
    atoms: Vec<Rc<Term>>,
    max_size: u32,
    preserve_universal: bool,
) -> SearchOutcome {
    let mut metrics = SearchMetrics::default();
    let mut candidate = None;
    if preserve_universal {
        let mut schedule =
            InterleavedDovetail::new((1..=max_size).map(|size| (size, EXPERIMENT_FUEL)));
        for _ in 0..max_size * 2 {
            let point = schedule.next_labeled().unwrap();
            let problem = if point.lane == ResourceLane::Learned {
                factory(atoms.clone())
            } else {
                factory(Vec::new())
            };
            let outcome = recursion_search::search_resource_point_first(
                &problem,
                point.syntax_size,
                point.evaluation_fuel,
            );
            add_metrics(&mut metrics, &outcome.metrics);
            if outcome.candidate.is_some() {
                candidate = outcome.candidate;
                break;
            }
        }
    } else {
        for size in 1..=max_size {
            let outcome = recursion_search::search_resource_point_first(
                &factory(atoms.clone()),
                size,
                EXPERIMENT_FUEL,
            );
            add_metrics(&mut metrics, &outcome.metrics);
            if outcome.candidate.is_some() {
                candidate = outcome.candidate;
                break;
            }
        }
    }
    SearchOutcome { candidate, metrics }
}

fn run_task(
    task_id: &'static str,
    expected_concept: &'static str,
    factory: ProblemFactory,
    order: &[ConceptSet],
    concepts: &HashMap<String, Rc<Term>>,
    preserve_universal: bool,
) -> ContextTaskResult {
    let mut metrics = SearchMetrics::default();
    let mut candidate = None;
    let mut lane_order = Vec::new();
    for set in order {
        lane_order.push(set.0.join("+"));
        let atoms = set
            .0
            .iter()
            .map(|id| concepts.get(id).expect("declared concept").clone())
            .collect();
        let outcome = run_lane(factory, atoms, 7, preserve_universal);
        add_metrics(&mut metrics, &outcome.metrics);
        if outcome.candidate.is_some() {
            candidate = outcome.candidate;
            break;
        }
    }
    ContextTaskResult {
        task_id,
        expected_concept,
        lane_order,
        outcome: SearchOutcome { candidate, metrics },
    }
}

fn condition(
    name: &'static str,
    orders: [&[ConceptSet]; 2],
    concepts: &HashMap<String, Rc<Term>>,
    preserve_universal: bool,
) -> ContextCondition {
    let tasks = vec![
        run_task(
            "heldout-reversed-single",
            "not",
            reversed_parity_problem,
            orders[0],
            concepts,
            preserve_universal,
        ),
        run_task(
            "heldout-reversed-nested",
            "parity",
            heldout_problem,
            orders[1],
            concepts,
            preserve_universal,
        ),
    ];
    let accounted = tasks
        .iter()
        .map(|task| {
            let ctx = if task.expected_concept == "not" {
                context(task.task_id, "single-chain", "single-chain")
            } else {
                context(task.task_id, "nested-chain", "nested-chain")
            };
            RunAccounting::from_universal(
                &task.outcome,
                provenance(&ctx, &task.lane_order, EvidencePhase::HeldOut),
            )
        })
        .collect::<Vec<_>>();
    ContextCondition {
        name,
        accounting: search_accounting::aggregate(&accounted).unwrap(),
        tasks,
        coverage_preserved: preserve_universal,
    }
}

fn universal_condition() -> ContextCondition {
    let tasks = [
        (
            "heldout-reversed-single",
            "not",
            reversed_parity_problem as ProblemFactory,
        ),
        (
            "heldout-reversed-nested",
            "parity",
            heldout_problem as ProblemFactory,
        ),
    ]
    .into_iter()
    .map(|(task_id, expected_concept, factory)| ContextTaskResult {
        task_id,
        expected_concept,
        lane_order: vec!["universal".into()],
        outcome: recursion_search::search_priority_prefix(&factory(Vec::new()), 9, EXPERIMENT_FUEL),
    })
    .collect::<Vec<_>>();
    let accounted = tasks
        .iter()
        .map(|task| {
            let ctx = context(task.task_id, "universal", "universal");
            RunAccounting::from_universal(
                &task.outcome,
                provenance(&ctx, &[], EvidencePhase::Diagnostic),
            )
        })
        .collect::<Vec<_>>();
    ContextCondition {
        name: "universal-only",
        accounting: search_accounting::aggregate(&accounted).unwrap(),
        tasks,
        coverage_preserved: true,
    }
}

fn top_singleton(policy: &FrozenPolicy) -> ConceptSet {
    policy
        .ranked
        .iter()
        .find(|weight| weight.concepts.len() == 1)
        .expect("candidate singleton")
        .concepts
        .clone()
}

pub fn run_contextual_guidance() -> ContextualGuidanceReport {
    let training = run_developmental_guidance(8, 9);
    let parity = training
        .parity
        .relevant
        .outcome
        .candidate
        .as_ref()
        .expect("parity training discovery")
        .executable
        .clone();
    let not = boolean_not();
    let concepts = HashMap::from([
        ("not".into(), not.clone()),
        ("parity".into(), parity.clone()),
        ("irrelevant".into(), irrelevant_pair_constructor()),
        ("misleading".into(), boolean_identity()),
    ]);

    // All marginal comparisons use the same size/fuel boundary within their
    // context. The nested parity evidence is conditional on the prior ontology
    // {not}; the candidate being credited is only the newly added parity atom.
    let single_ctx = context("train-single", "single-chain", "single-chain");
    let nested_ctx = context("train-nested", "nested-chain", "nested-chain");
    let single_base =
        recursion_search::search_priority_prefix(&parity_problem(Vec::new()), 7, EXPERIMENT_FUEL);
    let single_not = recursion_search::search_priority_prefix(
        &parity_problem(vec![not.clone()]),
        7,
        EXPERIMENT_FUEL,
    );
    let single_parity = recursion_search::search_priority_prefix(
        &parity_problem(vec![parity.clone()]),
        7,
        EXPERIMENT_FUEL,
    );
    let nested_base =
        recursion_search::search_priority_prefix(&nested_problem(Vec::new()), 7, EXPERIMENT_FUEL);
    let nested_not = recursion_search::search_priority_prefix(
        &nested_problem(vec![not.clone()]),
        7,
        EXPERIMENT_FUEL,
    );
    let nested_both = recursion_search::search_priority_prefix(
        &nested_problem(vec![not, parity]),
        7,
        EXPERIMENT_FUEL,
    );

    // Learn z from generic syntax measurements. Training and calibration IDs
    // are disjoint; the reversed-protocol heldouts are not observed until the
    // encoder and utility ledger have both frozen.
    let single_raw = raw_problem_observation(
        "raw-train-single",
        "raw-train-single",
        &parity_problem(Vec::new()),
    );
    let nested_raw = raw_problem_observation(
        "raw-train-nested",
        "raw-train-nested",
        &nested_problem(Vec::new()),
    );
    let mut raw_training = vec![
        raw_evidence(single_raw.clone(), &["not"], &single_base, &single_not),
        raw_evidence(
            single_raw.clone(),
            &["parity"],
            &single_base,
            &single_parity,
        ),
        raw_evidence(nested_raw.clone(), &["not"], &nested_base, &nested_not),
        raw_evidence(nested_raw.clone(), &["parity"], &nested_not, &nested_both),
    ];
    let mut single_cal = single_raw;
    single_cal.task_id = "raw-calibration-single".into();
    single_cal.duplicate_group_id = "raw-calibration-single".into();
    let mut nested_cal = nested_raw;
    nested_cal.task_id = "raw-calibration-nested".into();
    nested_cal.duplicate_group_id = "raw-calibration-nested".into();
    let raw_calibration = vec![
        raw_evidence(single_cal.clone(), &["not"], &single_base, &single_not),
        raw_evidence(single_cal, &["parity"], &single_base, &single_parity),
        raw_evidence(nested_cal.clone(), &["not"], &nested_base, &nested_not),
        raw_evidence(nested_cal.clone(), &["parity"], &nested_not, &nested_both),
    ];
    let learned_candidates = [
        ConceptSet::singleton("not"),
        ConceptSet::singleton("parity"),
        ConceptSet::singleton("irrelevant"),
        ConceptSet::singleton("misleading"),
    ];
    let representation_spec = RepresentationSpec {
        engine: search_accounting::SearchEngine::UniversalLambda,
        freeze_epoch: 1,
        decay_per_mille: 850,
        interactions: false,
        max_interaction_width: 1,
        max_projection_width: 2,
    };
    let learned_representation = learn_representation(
        &raw_training,
        &raw_calibration,
        &learned_candidates,
        &representation_spec,
    );
    let encoder_evidence_runs = raw_training
        .iter()
        .chain(&raw_calibration)
        .flat_map(|record| [record.without.clone(), record.with.clone()])
        .collect::<Vec<_>>();
    let encoder_evidence_accounting = search_accounting::aggregate(&encoder_evidence_runs)
        .expect("encoder evidence uses only universal-lambda units");
    // Calibration becomes legitimate historical utility evidence only after
    // encoder selection; protected heldouts remain absent from both stages.
    raw_training.extend(raw_calibration);
    let learned_single_target = raw_problem_observation(
        "heldout-reversed-single",
        "heldout-reversed-single",
        &reversed_parity_problem(Vec::new()),
    );
    let learned_nested_target = raw_problem_observation(
        "heldout-reversed-nested",
        "heldout-reversed-nested",
        &heldout_problem(Vec::new()),
    );
    let learned_single_policy = freeze_learned_policy(
        &learned_representation.encoder,
        &raw_training,
        &learned_single_target,
        &learned_candidates,
        &representation_spec,
    )
    .expect("safe frozen single context");
    let learned_nested_policy = freeze_learned_policy(
        &learned_representation.encoder,
        &raw_training,
        &learned_nested_target,
        &learned_candidates,
        &representation_spec,
    )
    .expect("safe frozen nested context");
    let mut ledger = ContextualLedger::default();
    ledger.record(evidence(
        single_ctx.clone(),
        &["not"],
        &single_base,
        &single_not,
    ));
    ledger.record(evidence(
        single_ctx,
        &["parity"],
        &single_base,
        &single_parity,
    ));
    ledger.record(evidence(
        nested_ctx.clone(),
        &["not"],
        &nested_base,
        &nested_not,
    ));
    ledger.record(evidence(nested_ctx, &["parity"], &nested_not, &nested_both));
    let candidates = [
        ConceptSet::singleton("not"),
        ConceptSet::singleton("parity"),
        ConceptSet::singleton("irrelevant"),
        ConceptSet::singleton("misleading"),
    ];
    let make_spec = |target: TaskContext, contextual: bool| FreezeSpec {
        target,
        engine: search_accounting::SearchEngine::UniversalLambda,
        freeze_epoch: 1,
        decay_per_mille: 850,
        contextual,
        interactions: false,
        max_interaction_width: 1,
    };
    let single_target = context("heldout-reversed-single", "single-chain", "single-chain");
    let nested_target = context("heldout-reversed-nested", "nested-chain", "nested-chain");
    let single_policy = ledger.learn(&candidates, &make_spec(single_target.clone(), true));
    let nested_policy = ledger.learn(&candidates, &make_spec(nested_target.clone(), true));
    let global_policy = ledger.learn(&candidates, &make_spec(single_target.clone(), false));
    let shuffled_single = ledger.learn(&candidates, &make_spec(nested_target, true));
    let shuffled_nested = ledger.learn(&candidates, &make_spec(single_target, true));

    let single_contextual = [top_singleton(&single_policy)];
    let nested_contextual = [top_singleton(&nested_policy)];
    let global_top = [top_singleton(&global_policy)];
    let uniform_order = [
        ConceptSet::singleton("parity"),
        ConceptSet::singleton("not"),
    ];
    let oracle_single = [ConceptSet::singleton("not")];
    let oracle_nested = [ConceptSet::singleton("parity")];
    let shuffled_single_order = [top_singleton(&shuffled_single)];
    let shuffled_nested_order = [top_singleton(&shuffled_nested)];
    let irrelevant_order = [ConceptSet::singleton("irrelevant")];
    let misleading_order = [ConceptSet::singleton("misleading")];

    let contextual = condition(
        "contextual",
        [&single_contextual, &nested_contextual],
        &concepts,
        true,
    );
    let learned_single_order = [top_singleton(&learned_single_policy)];
    let learned_nested_order = [top_singleton(&learned_nested_policy)];
    let learned = condition(
        "learned-context",
        [&learned_single_order, &learned_nested_order],
        &concepts,
        true,
    );
    let global = condition("global", [&global_top, &global_top], &concepts, true);
    let uniform = condition("uniform", [&uniform_order, &uniform_order], &concepts, true);
    let oracle = condition("oracle", [&oracle_single, &oracle_nested], &concepts, true);
    let shuffled_labels = condition(
        "shuffled-labels",
        [&shuffled_single_order, &shuffled_nested_order],
        &concepts,
        true,
    );
    let irrelevant = condition(
        "irrelevant",
        [&irrelevant_order, &irrelevant_order],
        &concepts,
        true,
    );
    let misleading = condition(
        "misleading",
        [&misleading_order, &misleading_order],
        &concepts,
        true,
    );
    let contextual_without_universal = condition(
        "contextual-no-universal",
        [&single_contextual, &nested_contextual],
        &concepts,
        false,
    );
    let universal_only = universal_condition();
    let contextual_regret_vs_oracle = contextual.proposals().saturating_sub(oracle.proposals());

    ContextualGuidanceReport {
        learned_representation,
        encoder_evidence_accounting,
        learned_single_policy,
        learned_nested_policy,
        learned,
        single_policy,
        nested_policy,
        global_policy,
        contextual,
        global,
        uniform,
        oracle,
        shuffled_labels,
        irrelevant,
        misleading,
        universal_only,
        contextual_without_universal,
        contextual_regret_vs_oracle,
    }
}

pub fn run_interaction_guidance() -> InteractionGuidanceReport {
    let first = boolean_not();
    let second = constant_true_step();
    let empty = recursion_search::search_priority_prefix(
        &two_step_training_problem(Vec::new()),
        9,
        EXPERIMENT_FUEL,
    );
    let first_only = recursion_search::search_priority_prefix(
        &two_step_training_problem(vec![first.clone()]),
        9,
        EXPERIMENT_FUEL,
    );
    let second_only = recursion_search::search_priority_prefix(
        &two_step_training_problem(vec![second.clone()]),
        9,
        EXPERIMENT_FUEL,
    );
    let both = recursion_search::search_priority_prefix(
        &two_step_training_problem(vec![first.clone(), second.clone()]),
        9,
        EXPERIMENT_FUEL,
    );
    let ctx = context("train-two-step", "two-step-chain", "two-step-chain");
    let mut ledger = ContextualLedger::default();
    ledger.record(evidence(ctx.clone(), &["first"], &empty, &first_only));
    ledger.record(evidence(ctx.clone(), &["second"], &empty, &second_only));
    ledger.record(evidence(ctx, &["first", "second"], &empty, &both));
    let candidates = [
        ConceptSet::singleton("first"),
        ConceptSet::singleton("second"),
        ConceptSet::new(["first".into(), "second".into()]),
    ];
    let target = context("heldout-two-step", "two-step-chain", "two-step-chain");
    let policy = ledger.learn(
        &candidates,
        &FreezeSpec {
            target,
            engine: search_accounting::SearchEngine::UniversalLambda,
            freeze_epoch: 1,
            decay_per_mille: 850,
            contextual: true,
            interactions: true,
            max_interaction_width: 2,
        },
    );
    let top = &policy.ranked[0].concepts;
    let heldout_empty = recursion_search::search_priority_prefix(
        &two_step_holdout_problem(Vec::new()),
        9,
        EXPERIMENT_FUEL,
    );
    let heldout_first = recursion_search::search_priority_prefix(
        &two_step_holdout_problem(vec![first.clone()]),
        9,
        EXPERIMENT_FUEL,
    );
    let heldout_second = recursion_search::search_priority_prefix(
        &two_step_holdout_problem(vec![second.clone()]),
        9,
        EXPERIMENT_FUEL,
    );
    let heldout_both = recursion_search::search_priority_prefix(
        &two_step_holdout_problem(vec![first, second]),
        9,
        EXPERIMENT_FUEL,
    );
    let interaction = if top.len() == 2 {
        heldout_both
    } else {
        heldout_empty.clone()
    };
    // With interactions disabled, only the two unsuccessful singleton lanes
    // are available under the same syntax boundary.
    let mut disabled_metrics = SearchMetrics::default();
    add_metrics(&mut disabled_metrics, &heldout_first.metrics);
    add_metrics(&mut disabled_metrics, &heldout_second.metrics);
    InteractionGuidanceReport {
        policy,
        interaction,
        interaction_disabled: SearchOutcome {
            candidate: None,
            metrics: disabled_metrics,
        },
        first_only: heldout_first,
        second_only: heldout_second,
        universal_only: heldout_empty,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contextual_utility_swaps_concepts_and_matches_oracle_on_disjoint_holdouts() {
        let report = run_contextual_guidance();
        assert!(report.learned_representation.encoder.retained);
        assert_eq!(report.learned_representation.encoder.calibration_regret, 0);
        assert_eq!(
            top_singleton(&report.learned_single_policy),
            ConceptSet::singleton("not")
        );
        assert_eq!(
            top_singleton(&report.learned_nested_policy),
            ConceptSet::singleton("parity")
        );
        assert_eq!(report.learned.solved_tasks(), 2);
        assert_eq!(report.learned.proposals(), report.oracle.proposals());
        assert_eq!(
            top_singleton(&report.single_policy),
            ConceptSet::singleton("not")
        );
        assert_eq!(
            top_singleton(&report.nested_policy),
            ConceptSet::singleton("parity")
        );
        assert_eq!(report.contextual.solved_tasks(), 2);
        assert_eq!(report.oracle.solved_tasks(), 2);
        assert_eq!(report.contextual_regret_vs_oracle, 0);
        assert_eq!(report.global.solved_tasks(), 1);
        assert_eq!(report.shuffled_labels.solved_tasks(), 0);
        assert!(report.contextual.proposals() < report.uniform.proposals());
        assert_eq!(report.contextual.proposals(), 670);
        assert_eq!(report.global.proposals(), 983);
        assert_eq!(report.uniform.proposals(), 1_318);
        assert_eq!(report.universal_only.proposals(), 5_244);
        assert_eq!(report.irrelevant.solved_tasks(), 0);
        assert_eq!(report.misleading.solved_tasks(), 0);
        assert_eq!(report.universal_only.solved_tasks(), 0);
        assert!(report.contextual.coverage_preserved);
        assert!(!report.contextual_without_universal.coverage_preserved);
        assert_eq!(report.contextual_without_universal.solved_tasks(), 2);
    }

    #[test]
    fn bounded_interaction_credit_unlocks_a_joint_recursive_law() {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| {
                let report = run_interaction_guidance();
                assert_eq!(
                    report.policy.ranked[0].concepts,
                    ConceptSet::new(["first".into(), "second".into()])
                );
                assert!(report.policy.ranked[0].interaction_residual > 0);
                assert!(report.interaction.candidate.is_some());
                assert!(report.first_only.candidate.is_none());
                assert!(report.second_only.candidate.is_none());
                assert!(report.interaction_disabled.candidate.is_none());
                assert!(report.universal_only.candidate.is_none());
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
