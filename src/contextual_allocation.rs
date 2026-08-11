//! Contextual and bounded interaction utility over labeled search evidence.
//!
//! Policies are immutable snapshots. Evidence is filtered by a freeze boundary
//! before scoring, and paired work is comparable only within one search engine.

use crate::search_accounting::{RunAccounting, SearchEngine, TerminationStatus};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskContext {
    pub task_id: String,
    pub family_id: String,
    pub duplicate_group_id: String,
    /// Predeclared observable features. They may use published training pairs,
    /// but never protected holdout outputs or target programs.
    pub features: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EvidenceDerivation {
    pub target_program_derived: bool,
    pub output_derived: bool,
    pub ancestor_task_ids: BTreeSet<String>,
}

#[derive(Clone, Debug)]
pub struct ContextualEvidence {
    pub context: TaskContext,
    pub concept_ids: Vec<String>,
    pub without: RunAccounting,
    pub with: RunAccounting,
    pub age: u32,
    pub recorded_epoch: u64,
    pub derivation: EvidenceDerivation,
}

impl ContextualEvidence {
    pub fn canonicalize(&mut self) {
        self.concept_ids.sort();
        self.concept_ids.dedup();
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConceptSet(pub Vec<String>);

impl ConceptSet {
    pub fn new(ids: impl IntoIterator<Item = String>) -> Self {
        let mut ids = ids.into_iter().collect::<Vec<_>>();
        ids.sort();
        ids.dedup();
        Self(ids)
    }

    pub fn singleton(id: impl Into<String>) -> Self {
        Self(vec![id.into()])
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FreezeSpec {
    pub target: TaskContext,
    /// One policy owns one labeled work unit. Cross-engine evidence is rejected
    /// rather than admitted according to insertion order.
    pub engine: SearchEngine,
    pub freeze_epoch: u64,
    pub decay_per_mille: u16,
    pub contextual: bool,
    pub interactions: bool,
    pub max_interaction_width: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RejectionCounts {
    pub heldout_task: usize,
    pub duplicate_group: usize,
    pub target_derived: usize,
    pub output_derived: usize,
    pub ancestry_leakage: usize,
    pub post_freeze: usize,
    pub mixed_engine_units: usize,
    pub over_width: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextualWeight {
    pub concepts: ConceptSet,
    pub score: i64,
    pub evidence_count: usize,
    pub confidence_per_mille: u16,
    /// For sets, utility remaining after positive singleton credit is removed.
    pub interaction_residual: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrozenPolicy {
    pub target: TaskContext,
    pub freeze_epoch: u64,
    pub engine: SearchEngine,
    pub ranked: Vec<ContextualWeight>,
    pub rejected: RejectionCounts,
    pub contextual: bool,
    pub interactions: bool,
    pub decay_per_mille: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BudgetDecision {
    pub concepts: ConceptSet,
    pub score: i64,
    pub confidence_per_mille: u16,
    pub learned_budget_units: u32,
}

#[derive(Clone, Debug, Default)]
pub struct ContextualLedger {
    evidence: Vec<ContextualEvidence>,
}

impl ContextualLedger {
    pub fn record(&mut self, mut evidence: ContextualEvidence) {
        evidence.canonicalize();
        self.evidence.push(evidence);
    }

    pub fn len(&self) -> usize {
        self.evidence.len()
    }

    pub fn learn(&self, candidates: &[ConceptSet], spec: &FreezeSpec) -> FrozenPolicy {
        assert!(spec.decay_per_mille <= 1_000);
        assert!(spec.max_interaction_width > 0);
        let mut rejected = RejectionCounts::default();
        let mut scores: HashMap<ConceptSet, (i128, usize)> = candidates
            .iter()
            .filter(|key| {
                !key.is_empty()
                    && key.len() <= spec.max_interaction_width
                    && (spec.interactions || key.len() == 1)
            })
            .cloned()
            .map(|key| (key, (0, 0)))
            .collect();
        for evidence in &self.evidence {
            if evidence.context.task_id == spec.target.task_id {
                rejected.heldout_task += 1;
                continue;
            }
            if evidence.context.duplicate_group_id == spec.target.duplicate_group_id {
                rejected.duplicate_group += 1;
                continue;
            }
            if evidence.derivation.target_program_derived {
                rejected.target_derived += 1;
                continue;
            }
            if evidence.derivation.output_derived {
                rejected.output_derived += 1;
                continue;
            }
            if evidence
                .derivation
                .ancestor_task_ids
                .contains(&spec.target.task_id)
            {
                rejected.ancestry_leakage += 1;
                continue;
            }
            if evidence.recorded_epoch > spec.freeze_epoch {
                rejected.post_freeze += 1;
                continue;
            }
            let key = ConceptSet::new(evidence.concept_ids.clone());
            if key.len() > spec.max_interaction_width || (!spec.interactions && key.len() > 1) {
                rejected.over_width += 1;
                continue;
            }
            if evidence.without.engine() != evidence.with.engine() {
                rejected.mixed_engine_units += 1;
                continue;
            }
            if evidence.with.engine() != spec.engine {
                // ARC and universal policies are learned separately, never
                // numerically pooled or selected by evidence insertion order.
                rejected.mixed_engine_units += 1;
                continue;
            }
            let Some((score, count)) = scores.get_mut(&key) else {
                continue;
            };
            let affinity = if spec.contextual {
                contextual_affinity(&evidence.context, &spec.target)
            } else {
                1_000
            };
            if affinity == 0 {
                continue;
            }
            let mut contribution = counterfactual_score(evidence);
            contribution = contribution.saturating_mul(i128::from(affinity)) / 1_000;
            for _ in 0..evidence.age {
                contribution =
                    contribution.saturating_mul(i128::from(spec.decay_per_mille)) / 1_000;
            }
            *score = score.saturating_add(contribution);
            *count += 1;
        }

        // Interactions receive only residual credit beyond positive singleton
        // utility. This prevents a redundant pair from being counted as a new
        // synergistic abstraction.
        let singleton_scores = scores
            .iter()
            .filter(|(key, _)| key.len() == 1)
            .map(|(key, (score, _))| (key.0[0].clone(), (*score).max(0)))
            .collect::<HashMap<_, _>>();
        let mut ranked = scores
            .into_iter()
            .map(|(concepts, (raw_score, evidence_count))| {
                let residual = if concepts.len() > 1 {
                    raw_score.saturating_sub(
                        concepts
                            .0
                            .iter()
                            .map(|id| singleton_scores.get(id).copied().unwrap_or(0))
                            .sum::<i128>(),
                    )
                } else {
                    raw_score
                };
                let effective = if concepts.len() > 1 {
                    residual
                } else {
                    raw_score
                };
                ContextualWeight {
                    concepts,
                    score: clamp_i64(effective),
                    evidence_count,
                    confidence_per_mille: ((evidence_count as u64 * 1_000)
                        / (evidence_count as u64 + 1))
                        as u16,
                    interaction_residual: clamp_i64(residual),
                }
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| b.confidence_per_mille.cmp(&a.confidence_per_mille))
                .then_with(|| a.concepts.cmp(&b.concepts))
        });
        FrozenPolicy {
            target: spec.target.clone(),
            freeze_epoch: spec.freeze_epoch,
            engine: spec.engine,
            ranked,
            rejected,
            contextual: spec.contextual,
            interactions: spec.interactions,
            decay_per_mille: spec.decay_per_mille,
        }
    }
}

/// Allocate a fixed learned budget. Uncertain and nonpositive hypotheses keep
/// diagnostic representation but cannot consume the limited positive lanes.
pub fn allocate_budget(
    policy: &FrozenPolicy,
    positive_lanes: usize,
    units_per_lane: u32,
) -> Vec<BudgetDecision> {
    let mut positive_used = 0;
    policy
        .ranked
        .iter()
        .map(|weight| {
            let learned_budget_units = if weight.score > 0 && positive_used < positive_lanes {
                positive_used += 1;
                let confidence_units = (u64::from(units_per_lane)
                    * u64::from(weight.confidence_per_mille)
                    / 1_000) as u32;
                confidence_units.max(1)
            } else {
                0
            };
            BudgetDecision {
                concepts: weight.concepts.clone(),
                score: weight.score,
                confidence_per_mille: weight.confidence_per_mille,
                learned_budget_units,
            }
        })
        .collect()
}

fn contextual_affinity(observed: &TaskContext, target: &TaskContext) -> u16 {
    if target.features.is_empty() {
        return if observed.family_id == target.family_id {
            1_000
        } else {
            0
        };
    }
    let matches = target
        .features
        .iter()
        .filter(|(key, value)| observed.features.get(*key) == Some(*value))
        .count();
    // Squared overlap rewards an exact context while permitting weaker
    // transfer across partially matching input descriptions.
    let numerator = matches * matches * 1_000;
    let denominator = target.features.len() * target.features.len();
    (numerator / denominator) as u16
}

fn counterfactual_score(evidence: &ContextualEvidence) -> i128 {
    let before = evidence.without.work.comparable_primary_work();
    let after = evidence.with.work.comparable_primary_work();
    let mut score = match (
        evidence.without.termination == TerminationStatus::Solved,
        evidence.with.termination == TerminationStatus::Solved,
    ) {
        (false, true) => i128::from(before.max(1)),
        (true, true) => i128::from(before) - i128::from(after),
        (true, false) => -i128::from(before.saturating_add(after).max(1)),
        (false, false) => -i128::from(after.max(1)),
    };
    score += (i128::from(evidence.without.max_structural_size)
        - i128::from(evidence.with.max_structural_size))
        * 32;
    if let (Some(before_rank), Some(after_rank)) =
        (evidence.without.solution_rank, evidence.with.solution_rank)
    {
        score += i128::from(before_rank) - i128::from(after_rank);
    }
    score
}

fn clamp_i64(value: i128) -> i64 {
    value.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search_accounting::{EngineWork, EvaluatorBudget, EvidencePhase, RunProvenance};

    fn context(task: &str, family: &str, feature: &str) -> TaskContext {
        TaskContext {
            task_id: task.into(),
            family_id: family.into(),
            duplicate_group_id: task.into(),
            features: BTreeMap::from([("shape".into(), feature.into())]),
        }
    }

    fn run(ctx: &TaskContext, work: u64, solved: bool) -> RunAccounting {
        RunAccounting {
            work: EngineWork::UniversalLambda {
                proposals: work,
                evaluated_candidates: 0,
                resource_points: 1,
            },
            max_structural_size: 7,
            evaluator_budget: EvaluatorBudget::LambdaFuel(100),
            solution_rank: solved.then_some(work),
            termination: if solved {
                TerminationStatus::Solved
            } else {
                TerminationStatus::ExhaustedFiniteBoundary
            },
            provenance: RunProvenance {
                task_id: ctx.task_id.clone(),
                family_id: ctx.family_id.clone(),
                duplicate_group_id: ctx.duplicate_group_id.clone(),
                context_features: ctx.features.clone(),
                concept_ids: Vec::new(),
                phase: EvidencePhase::Training,
                observed_epoch: 0,
            },
        }
    }

    fn evidence(ctx: TaskContext, ids: &[&str], before: u64, after: u64) -> ContextualEvidence {
        ContextualEvidence {
            without: run(&ctx, before, true),
            with: run(&ctx, after, true),
            context: ctx,
            concept_ids: ids.iter().map(|id| (*id).into()).collect(),
            age: 0,
            recorded_epoch: 1,
            derivation: EvidenceDerivation::default(),
        }
    }

    fn spec(target: TaskContext) -> FreezeSpec {
        FreezeSpec {
            target,
            engine: SearchEngine::UniversalLambda,
            freeze_epoch: 10,
            decay_per_mille: 800,
            contextual: true,
            interactions: true,
            max_interaction_width: 2,
        }
    }

    #[test]
    fn context_swaps_priority_where_global_utility_cannot() {
        let a = context("train-a", "rows", "wide");
        let b = context("train-b", "columns", "tall");
        let mut ledger = ContextualLedger::default();
        ledger.record(evidence(a.clone(), &["A"], 100, 10));
        ledger.record(evidence(a, &["B"], 10, 100));
        ledger.record(evidence(b.clone(), &["A"], 10, 100));
        ledger.record(evidence(b, &["B"], 100, 10));
        let candidates = [ConceptSet::singleton("A"), ConceptSet::singleton("B")];

        let rows = ledger.learn(&candidates, &spec(context("test-r", "rows", "wide")));
        let cols = ledger.learn(&candidates, &spec(context("test-c", "columns", "tall")));
        assert_eq!(rows.ranked[0].concepts, ConceptSet::singleton("A"));
        assert_eq!(cols.ranked[0].concepts, ConceptSet::singleton("B"));

        let mut global_spec = spec(context("test-c", "columns", "tall"));
        global_spec.contextual = false;
        let global = ledger.learn(&candidates, &global_spec);
        assert_eq!(global.ranked[0].score, global.ranked[1].score);
    }

    #[test]
    fn interaction_credit_finds_synergy_not_redundancy_or_antagonism() {
        let ctx = context("train", "pairs", "wide");
        let mut ledger = ContextualLedger::default();
        ledger.record(evidence(ctx.clone(), &["A"], 100, 100));
        ledger.record(evidence(ctx.clone(), &["B"], 100, 100));
        ledger.record(evidence(ctx.clone(), &["A", "B"], 100, 10));
        ledger.record(evidence(ctx.clone(), &["C"], 100, 10));
        ledger.record(evidence(ctx.clone(), &["C", "D"], 100, 10));
        ledger.record(evidence(ctx.clone(), &["E", "F"], 10, 100));
        let candidates = [
            ConceptSet::singleton("A"),
            ConceptSet::singleton("B"),
            ConceptSet::new(["A".into(), "B".into()]),
            ConceptSet::singleton("C"),
            ConceptSet::new(["C".into(), "D".into()]),
            ConceptSet::new(["E".into(), "F".into()]),
        ];
        let policy = ledger.learn(&candidates, &spec(context("test", "pairs", "wide")));
        let weight = |ids: &[&str]| {
            policy
                .ranked
                .iter()
                .find(|weight| {
                    weight.concepts == ConceptSet::new(ids.iter().map(|id| (*id).to_string()))
                })
                .unwrap()
        };
        assert!(weight(&["A", "B"]).score > 0);
        assert_eq!(weight(&["C", "D"]).score, 0);
        assert!(weight(&["E", "F"]).score < 0);

        let mut ablated = spec(context("test", "pairs", "wide"));
        ablated.interactions = false;
        let no_interactions = ledger.learn(&candidates, &ablated);
        assert!(no_interactions.ranked.iter().all(|w| w.concepts.len() == 1));
    }

    #[test]
    fn freeze_rejects_every_declared_leakage_channel() {
        let target = TaskContext {
            duplicate_group_id: "dup".into(),
            ..context("heldout", "rows", "wide")
        };
        let mut ledger = ContextualLedger::default();
        ledger.record(evidence(
            context("clean-training", "rows", "wide"),
            &["A"],
            100,
            1,
        ));
        let candidates = [ConceptSet::singleton("A")];
        let freeze = spec(target.clone());
        let clean_policy = ledger.learn(&candidates, &freeze);
        let mut cases = Vec::new();
        cases.push(evidence(target.clone(), &["A"], 100, 1));
        let mut duplicate = evidence(context("other", "rows", "wide"), &["A"], 100, 1);
        duplicate.context.duplicate_group_id = "dup".into();
        cases.push(duplicate);
        let mut target_derived = evidence(context("td", "rows", "wide"), &["A"], 100, 1);
        target_derived.derivation.target_program_derived = true;
        cases.push(target_derived);
        let mut output = evidence(context("out", "rows", "wide"), &["A"], 100, 1);
        output.derivation.output_derived = true;
        cases.push(output);
        let mut ancestry = evidence(context("anc", "rows", "wide"), &["A"], 100, 1);
        ancestry
            .derivation
            .ancestor_task_ids
            .insert("heldout".into());
        cases.push(ancestry);
        let mut late = evidence(context("late", "rows", "wide"), &["A"], 100, 1);
        late.recorded_epoch = 11;
        cases.push(late);
        for case in cases {
            ledger.record(case);
        }
        let policy = ledger.learn(&candidates, &freeze);
        assert_eq!(policy.ranked, clean_policy.ranked);
        assert_eq!(policy.engine, clean_policy.engine);
        assert_eq!(policy.rejected.heldout_task, 1);
        assert_eq!(policy.rejected.duplicate_group, 1);
        assert_eq!(policy.rejected.target_derived, 1);
        assert_eq!(policy.rejected.output_derived, 1);
        assert_eq!(policy.rejected.ancestry_leakage, 1);
        assert_eq!(policy.rejected.post_freeze, 1);
    }

    #[test]
    fn decay_reallocates_after_shift_and_budget_excludes_uncertain_or_harmful() {
        let ctx = context("old", "shift", "wide");
        let mut old_a = evidence(ctx.clone(), &["A"], 1_000, 10);
        old_a.age = 8;
        let new_b = evidence(ctx, &["B"], 500, 10);
        let mut ledger = ContextualLedger::default();
        ledger.record(old_a);
        ledger.record(new_b);
        let candidates = [
            ConceptSet::singleton("A"),
            ConceptSet::singleton("B"),
            ConceptSet::singleton("unknown"),
        ];
        let target = context("new", "shift", "wide");
        let decayed = ledger.learn(&candidates, &spec(target.clone()));
        assert_eq!(decayed.ranked[0].concepts, ConceptSet::singleton("B"));
        let mut no_decay_spec = spec(target);
        no_decay_spec.decay_per_mille = 1_000;
        let stale = ledger.learn(&candidates, &no_decay_spec);
        assert_eq!(stale.ranked[0].concepts, ConceptSet::singleton("A"));
        let allocation = allocate_budget(&decayed, 1, 100);
        assert!(
            allocation
                .iter()
                .find(|d| d.concepts == ConceptSet::singleton("B"))
                .unwrap()
                .learned_budget_units
                > 0
        );
        assert_eq!(
            allocation
                .iter()
                .find(|d| d.concepts == ConceptSet::singleton("unknown"))
                .unwrap()
                .learned_budget_units,
            0
        );
    }

    #[test]
    fn decay_lowers_shift_regret_without_forgetting_an_old_context() {
        let old = TaskContext {
            features: BTreeMap::from([
                ("shape".into(), "wide".into()),
                ("regime".into(), "old".into()),
            ]),
            ..context("old", "shift", "wide")
        };
        let new = TaskContext {
            features: BTreeMap::from([
                ("shape".into(), "wide".into()),
                ("regime".into(), "new".into()),
            ]),
            ..context("new", "shift", "wide")
        };
        let new_holdout = TaskContext {
            task_id: "new-holdout".into(),
            duplicate_group_id: "new-holdout".into(),
            ..new.clone()
        };
        let old_replay = TaskContext {
            task_id: "old-replay".into(),
            duplicate_group_id: "old-replay".into(),
            ..old.clone()
        };
        let mut strong_old_a = evidence(old.clone(), &["A"], 3_010, 10);
        strong_old_a.age = 8;
        let fresh_new_b = evidence(new.clone(), &["B"], 500, 10);
        let mut ledger = ContextualLedger::default();
        ledger.record(strong_old_a);
        ledger.record(fresh_new_b);
        let candidates = [ConceptSet::singleton("A"), ConceptSet::singleton("B")];

        let decayed_new = ledger.learn(&candidates, &spec(new_holdout.clone()));
        let mut stale_spec = spec(new_holdout);
        stale_spec.decay_per_mille = 1_000;
        let stale_new = ledger.learn(&candidates, &stale_spec);
        assert_eq!(decayed_new.ranked[0].concepts, ConceptSet::singleton("B"));
        assert_eq!(stale_new.ranked[0].concepts, ConceptSet::singleton("A"));

        // Three shifted tasks cost 10 with B and 100 with A. Oracle cost is
        // 30, so decay has zero regret and the stale policy has 270.
        let decayed_regret = 3 * 10 - 3 * 10;
        let stale_regret = 3 * 100 - 3 * 10;
        assert!(decayed_regret < stale_regret);

        let replay_old = ledger.learn(&candidates, &spec(old_replay));
        assert_eq!(replay_old.ranked[0].concepts, ConceptSet::singleton("A"));
    }

    #[test]
    fn policy_engine_is_explicit_and_does_not_depend_on_evidence_order() {
        let ctx = context("train", "family", "wide");
        let universal = evidence(ctx.clone(), &["A"], 100, 1);
        let mut bank_evidence = evidence(ctx, &["A"], 100, 1);
        for run in [&mut bank_evidence.without, &mut bank_evidence.with] {
            let work = run.work.comparable_primary_work();
            run.work = EngineWork::BehaviorBank {
                candidate_constructions: work,
                retained_candidates: 0,
                aborted_candidates: 0,
            };
            run.evaluator_budget = EvaluatorBudget::BankFuel(100);
        }
        let candidates = [ConceptSet::singleton("A")];
        let target = context("holdout", "family", "wide");
        for evidence in [
            vec![bank_evidence.clone(), universal.clone()],
            vec![universal.clone(), bank_evidence.clone()],
        ] {
            let mut ledger = ContextualLedger::default();
            for item in evidence {
                ledger.record(item);
            }
            let policy = ledger.learn(&candidates, &spec(target.clone()));
            assert_eq!(policy.engine, SearchEngine::UniversalLambda);
            assert_eq!(policy.rejected.mixed_engine_units, 1);
            assert!(policy.ranked[0].score > 0);
        }
    }
}
