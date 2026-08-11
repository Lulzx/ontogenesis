//! Learn deterministic ontology-search allocation from prior counterfactual utility.
//!
//! This module only learns *priority*. Completeness remains the responsibility
//! of the independently specified universal lane, which experiments interleave
//! through [`crate::universal::InterleavedDovetail`].

use crate::{
    recursion_search::{SearchMetrics, SearchOutcome},
    term::Term,
};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct ConceptCandidate {
    pub id: String,
    pub body: Rc<Term>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkSample {
    pub proposals: u64,
    pub evaluated_candidates: u64,
    pub max_syntax_size: u32,
    pub evaluation_fuel: u64,
    pub solved: bool,
    pub first_solution_rank: Option<u64>,
}

impl WorkSample {
    pub fn from_outcome(outcome: &SearchOutcome) -> Self {
        Self {
            proposals: outcome.metrics.proposals,
            evaluated_candidates: outcome.metrics.evaluated_candidates,
            max_syntax_size: outcome.metrics.max_syntax_size,
            evaluation_fuel: outcome.metrics.evaluation_fuel,
            solved: outcome.candidate.is_some(),
            first_solution_rank: outcome
                .candidate
                .as_ref()
                .map(|_| outcome.metrics.proposals),
        }
    }

    fn total_work(&self) -> i128 {
        i128::from(self.proposals) + i128::from(self.evaluated_candidates)
    }
}

#[derive(Clone, Debug)]
pub struct UtilityEvidence {
    pub training_task_id: String,
    pub concept_id: String,
    pub without: WorkSample,
    pub with: WorkSample,
    /// Extra proposals caused by widening the grammar with this concept.
    pub widening_penalty: u64,
    /// Number of learning updates since this evidence was observed.
    pub age: u32,
    /// Evidence constructed from the target solution is excluded to prevent a
    /// self-fulfilling prior.
    pub target_derived: bool,
}

impl UtilityEvidence {
    fn score(&self, decay_per_mille: u16) -> i128 {
        let before = self.without.total_work();
        let after = self.with.total_work();
        let mut score = match (self.without.solved, self.with.solved) {
            (false, true) => before.max(1),
            (true, true) => before - after,
            (true, false) => -(before + after).max(1),
            (false, false) => -after.max(1),
        };
        score -= i128::from(self.widening_penalty);

        // Reward reaching a smaller syntax frontier, but do not let the unit
        // choice for fuel dominate deterministic proposal/evaluation evidence.
        score +=
            (i128::from(self.without.max_syntax_size) - i128::from(self.with.max_syntax_size)) * 32;
        score += (i128::from(self.without.evaluation_fuel) - i128::from(self.with.evaluation_fuel))
            / 10_000;
        // If both conditions solve, prefer the concept that moves the first
        // admitted solution earlier in deterministic enumeration order.
        if let (Some(without_rank), Some(with_rank)) = (
            self.without.first_solution_rank,
            self.with.first_solution_rank,
        ) {
            score += i128::from(without_rank) - i128::from(with_rank);
        }

        for _ in 0..self.age {
            score = score * i128::from(decay_per_mille) / 1_000;
        }
        score
    }
}

#[derive(Clone, Debug, Default)]
pub struct UtilityLedger {
    evidence: Vec<UtilityEvidence>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LearnedWeight {
    pub concept_id: String,
    pub score: i64,
    pub evidence_count: usize,
}

#[derive(Clone, Debug)]
pub struct LearnedWeights {
    pub ranked: Vec<LearnedWeight>,
    pub skipped_target_leakage: usize,
    pub skipped_target_derived: usize,
}

impl UtilityLedger {
    pub fn record(&mut self, evidence: UtilityEvidence) {
        self.evidence.push(evidence);
    }

    pub fn len(&self) -> usize {
        self.evidence.len()
    }

    pub fn is_empty(&self) -> bool {
        self.evidence.is_empty()
    }

    /// Learn weights without looking at evidence from the held-out task or at
    /// evidence derived from a target solution.
    pub fn learn(
        &self,
        candidates: &[ConceptCandidate],
        held_out_task_id: &str,
        decay_per_mille: u16,
    ) -> LearnedWeights {
        assert!(decay_per_mille <= 1_000);
        let mut scores: HashMap<&str, (i128, usize)> = candidates
            .iter()
            .map(|candidate| (candidate.id.as_str(), (0, 0)))
            .collect();
        let mut skipped_target_leakage = 0;
        let mut skipped_target_derived = 0;
        for evidence in &self.evidence {
            if evidence.training_task_id == held_out_task_id {
                skipped_target_leakage += 1;
                continue;
            }
            if evidence.target_derived {
                skipped_target_derived += 1;
                continue;
            }
            if let Some((score, count)) = scores.get_mut(evidence.concept_id.as_str()) {
                *score += evidence.score(decay_per_mille);
                *count += 1;
            }
        }
        let mut ranked = scores
            .into_iter()
            .map(|(concept_id, (score, evidence_count))| LearnedWeight {
                concept_id: concept_id.to_string(),
                score: score.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64,
                evidence_count,
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| a.concept_id.cmp(&b.concept_id))
        });
        LearnedWeights {
            ranked,
            skipped_target_leakage,
            skipped_target_derived,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllocationDecision {
    pub concept_id: String,
    pub learned_score: i64,
    pub max_syntax_size: u32,
    pub evaluation_fuel: u64,
}

/// Convert learned weights into deterministic resource decisions. The leading
/// positive band receives the full lane; weaker positive, unknown, and harmful
/// concepts receive progressively smaller lanes but are not erased from the
/// ontology. The universal interleave is separate and unaffected.
pub fn allocate(
    weights: &LearnedWeights,
    full_max_syntax_size: u32,
    full_fuel: u64,
) -> Vec<AllocationDecision> {
    let best_positive = weights
        .ranked
        .iter()
        .map(|weight| weight.score)
        .max()
        .unwrap_or(0)
        .max(0);
    weights
        .ranked
        .iter()
        .map(|weight| {
            let (size_discount, fuel_divisor) = if weight.score > 0
                && (best_positive == 0 || weight.score.saturating_mul(4) >= best_positive)
            {
                (0, 1)
            } else if weight.score > 0 {
                // Positive but stale or weakly calibrated evidence receives a
                // real lane, just not the current leader's full budget.
                (1, 2)
            } else if weight.score == 0 {
                (2, 4)
            } else {
                (3, 10)
            };
            AllocationDecision {
                concept_id: weight.concept_id.clone(),
                learned_score: weight.score,
                max_syntax_size: full_max_syntax_size.saturating_sub(size_discount).max(1),
                evaluation_fuel: (full_fuel / fuel_divisor).max(1),
            }
        })
        .collect()
}

pub fn widening_penalty(without: &SearchMetrics, with: &SearchMetrics) -> u64 {
    with.proposals.saturating_sub(without.proposals)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term;

    fn sample(proposals: u64, evaluated: u64, solved: bool) -> WorkSample {
        WorkSample {
            proposals,
            evaluated_candidates: evaluated,
            max_syntax_size: 7,
            evaluation_fuel: 100_000,
            solved,
            first_solution_rank: solved.then_some(proposals),
        }
    }

    fn concept(id: &str) -> ConceptCandidate {
        ConceptCandidate {
            id: id.to_string(),
            body: term::lam(term::var(0)),
        }
    }

    #[test]
    fn counterfactual_utility_ranks_helpful_above_harmful_and_decays_stale_evidence() {
        let mut ledger = UtilityLedger::default();
        ledger.record(UtilityEvidence {
            training_task_id: "train-a".into(),
            concept_id: "helpful".into(),
            without: sample(10_000, 5_000, false),
            with: sample(100, 50, true),
            widening_penalty: 0,
            age: 0,
            target_derived: false,
        });
        ledger.record(UtilityEvidence {
            training_task_id: "train-a".into(),
            concept_id: "harmful".into(),
            without: sample(500, 200, false),
            with: sample(900, 350, false),
            widening_penalty: 400,
            age: 0,
            target_derived: false,
        });
        ledger.record(UtilityEvidence {
            training_task_id: "old".into(),
            concept_id: "stale".into(),
            without: sample(10_000, 5_000, false),
            with: sample(100, 50, true),
            widening_penalty: 0,
            age: 20,
            target_derived: false,
        });
        let learned = ledger.learn(
            &[concept("helpful"), concept("harmful"), concept("stale")],
            "held-out",
            800,
        );
        assert_eq!(learned.ranked[0].concept_id, "helpful");
        assert!(learned.ranked[0].score > learned.ranked[1].score);
        assert!(learned.ranked.last().unwrap().score < 0);
    }

    #[test]
    fn heldout_and_target_derived_evidence_cannot_make_a_self_fulfilling_prior() {
        let mut ledger = UtilityLedger::default();
        for (task, derived) in [("held-out", false), ("train", true)] {
            ledger.record(UtilityEvidence {
                training_task_id: task.into(),
                concept_id: "leaky".into(),
                without: sample(10_000, 5_000, false),
                with: sample(1, 1, true),
                widening_penalty: 0,
                age: 0,
                target_derived: derived,
            });
        }
        let learned = ledger.learn(&[concept("leaky")], "held-out", 900);
        assert_eq!(learned.ranked[0].score, 0);
        assert_eq!(learned.skipped_target_leakage, 1);
        assert_eq!(learned.skipped_target_derived, 1);
    }

    #[test]
    fn allocation_spends_less_on_unknown_and_harmful_concepts() {
        let weights = LearnedWeights {
            ranked: vec![
                LearnedWeight {
                    concept_id: "useful".into(),
                    score: 10,
                    evidence_count: 1,
                },
                LearnedWeight {
                    concept_id: "unknown".into(),
                    score: 0,
                    evidence_count: 0,
                },
                LearnedWeight {
                    concept_id: "harmful".into(),
                    score: -10,
                    evidence_count: 1,
                },
            ],
            skipped_target_leakage: 0,
            skipped_target_derived: 0,
        };
        let decisions = allocate(&weights, 7, 100_000);
        assert_eq!(decisions[0].max_syntax_size, 7);
        assert!(decisions[0].evaluation_fuel > decisions[1].evaluation_fuel);
        assert!(decisions[1].evaluation_fuel > decisions[2].evaluation_fuel);
    }

    #[test]
    fn stale_positive_evidence_receives_less_budget_than_current_utility() {
        let weights = LearnedWeights {
            ranked: vec![
                LearnedWeight {
                    concept_id: "current".into(),
                    score: 10_000,
                    evidence_count: 1,
                },
                LearnedWeight {
                    concept_id: "stale".into(),
                    score: 1_000,
                    evidence_count: 1,
                },
            ],
            skipped_target_leakage: 0,
            skipped_target_derived: 0,
        };
        let decisions = allocate(&weights, 7, 100_000);
        assert_eq!(decisions[0].evaluation_fuel, 100_000);
        assert_eq!(decisions[1].evaluation_fuel, 50_000);
        assert!(decisions[1].max_syntax_size < decisions[0].max_syntax_size);
    }
}
