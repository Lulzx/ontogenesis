//! Labeled deterministic search accounting shared by experiment drivers.
//!
//! Universal enumeration and the quotient-aware behavior bank do not perform
//! the same unit of work. This module gives them one reporting envelope while
//! deliberately retaining distinct work variants. Aggregation across variants
//! is rejected instead of manufacturing an apples-to-oranges scalar.

use crate::{bank, recursion_search};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SearchEngine {
    UniversalLambda,
    BehaviorBank,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineWork {
    UniversalLambda {
        proposals: u64,
        evaluated_candidates: u64,
        resource_points: u64,
    },
    BehaviorBank {
        candidate_constructions: u64,
        retained_candidates: u64,
        aborted_candidates: u64,
    },
}

impl EngineWork {
    pub fn engine(&self) -> SearchEngine {
        match self {
            Self::UniversalLambda { .. } => SearchEngine::UniversalLambda,
            Self::BehaviorBank { .. } => SearchEngine::BehaviorBank,
        }
    }

    /// Primary work is meaningful only after callers prove both samples use
    /// the same engine variant. It is never a cross-engine conversion.
    pub fn comparable_primary_work(&self) -> u64 {
        match self {
            Self::UniversalLambda {
                proposals,
                evaluated_candidates,
                ..
            } => proposals.saturating_add(*evaluated_candidates),
            Self::BehaviorBank {
                candidate_constructions,
                ..
            } => *candidate_constructions,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvaluatorBudget {
    LambdaFuel(u64),
    BankFuel(i64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminationStatus {
    Solved,
    ExhaustedFiniteBoundary,
    BudgetLimited,
    GuardRejected,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidencePhase {
    Training,
    Calibration,
    HeldOut,
    Diagnostic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunProvenance {
    pub task_id: String,
    pub family_id: String,
    /// Identifies exact or generated near-duplicate tasks that must not cross
    /// a train/holdout boundary.
    pub duplicate_group_id: String,
    /// Predeclared observable task features. Protected holdout outputs and
    /// target-program provenance are separately gated by contextual learning.
    pub context_features: BTreeMap<String, String>,
    pub concept_ids: Vec<String>,
    pub phase: EvidencePhase,
    pub observed_epoch: u64,
}

impl RunProvenance {
    pub fn canonicalize(&mut self) {
        self.concept_ids.sort();
        self.concept_ids.dedup();
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunAccounting {
    pub work: EngineWork,
    pub max_structural_size: u32,
    pub evaluator_budget: EvaluatorBudget,
    pub solution_rank: Option<u64>,
    pub termination: TerminationStatus,
    pub provenance: RunProvenance,
}

impl RunAccounting {
    pub fn from_universal(
        outcome: &recursion_search::SearchOutcome,
        mut provenance: RunProvenance,
    ) -> Self {
        provenance.canonicalize();
        Self {
            work: EngineWork::UniversalLambda {
                proposals: outcome.metrics.proposals,
                evaluated_candidates: outcome.metrics.evaluated_candidates,
                resource_points: outcome.metrics.resource_points,
            },
            max_structural_size: outcome.metrics.max_syntax_size,
            evaluator_budget: EvaluatorBudget::LambdaFuel(outcome.metrics.evaluation_fuel),
            solution_rank: outcome
                .candidate
                .as_ref()
                .map(|_| outcome.metrics.proposals),
            termination: if outcome.candidate.is_some() {
                TerminationStatus::Solved
            } else {
                TerminationStatus::ExhaustedFiniteBoundary
            },
            provenance,
        }
    }

    pub fn from_bank(
        outcome: &bank::Outcome,
        opts: &bank::Options,
        mut provenance: RunProvenance,
    ) -> Self {
        provenance.canonicalize();
        Self {
            work: EngineWork::BehaviorBank {
                candidate_constructions: outcome.stats.built,
                retained_candidates: outcome.stats.kept,
                aborted_candidates: outcome.stats.aborted,
            },
            max_structural_size: outcome.stats.reached_size,
            evaluator_budget: EvaluatorBudget::BankFuel(opts.fuel),
            solution_rank: outcome.solution.as_ref().map(|_| outcome.stats.built),
            termination: if outcome.solution.is_some() {
                TerminationStatus::Solved
            } else if outcome.stats.elapsed_secs >= opts.time_budget_secs {
                TerminationStatus::BudgetLimited
            } else {
                TerminationStatus::ExhaustedFiniteBoundary
            },
            provenance,
        }
    }

    pub fn engine(&self) -> SearchEngine {
        self.work.engine()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountingSummary {
    pub engine: SearchEngine,
    pub work: EngineWork,
    pub max_structural_size: u32,
    pub evaluator_budget: EvaluatorBudget,
    pub solved_runs: usize,
    pub run_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AccountingError {
    Empty,
    MixedEngines,
    MixedBudgetUnits,
}

pub fn aggregate(runs: &[RunAccounting]) -> Result<AccountingSummary, AccountingError> {
    let first = runs.first().ok_or(AccountingError::Empty)?;
    let engine = first.engine();
    let mut work = match engine {
        SearchEngine::UniversalLambda => EngineWork::UniversalLambda {
            proposals: 0,
            evaluated_candidates: 0,
            resource_points: 0,
        },
        SearchEngine::BehaviorBank => EngineWork::BehaviorBank {
            candidate_constructions: 0,
            retained_candidates: 0,
            aborted_candidates: 0,
        },
    };
    let mut max_structural_size = 0;
    let mut solved_runs = 0;
    let mut evaluator_budget = first.evaluator_budget.clone();
    for run in runs {
        if run.engine() != engine {
            return Err(AccountingError::MixedEngines);
        }
        match (&mut evaluator_budget, &run.evaluator_budget) {
            (EvaluatorBudget::LambdaFuel(total), EvaluatorBudget::LambdaFuel(next)) => {
                *total = (*total).max(*next)
            }
            (EvaluatorBudget::BankFuel(total), EvaluatorBudget::BankFuel(next)) => {
                *total = (*total).max(*next)
            }
            _ => return Err(AccountingError::MixedBudgetUnits),
        }
        match (&mut work, &run.work) {
            (
                EngineWork::UniversalLambda {
                    proposals,
                    evaluated_candidates,
                    resource_points,
                },
                EngineWork::UniversalLambda {
                    proposals: p,
                    evaluated_candidates: e,
                    resource_points: r,
                },
            ) => {
                *proposals = proposals.saturating_add(*p);
                *evaluated_candidates = evaluated_candidates.saturating_add(*e);
                *resource_points = resource_points.saturating_add(*r);
            }
            (
                EngineWork::BehaviorBank {
                    candidate_constructions,
                    retained_candidates,
                    aborted_candidates,
                },
                EngineWork::BehaviorBank {
                    candidate_constructions: c,
                    retained_candidates: k,
                    aborted_candidates: a,
                },
            ) => {
                *candidate_constructions = candidate_constructions.saturating_add(*c);
                *retained_candidates = retained_candidates.saturating_add(*k);
                *aborted_candidates = aborted_candidates.saturating_add(*a);
            }
            _ => return Err(AccountingError::MixedEngines),
        }
        max_structural_size = max_structural_size.max(run.max_structural_size);
        solved_runs += usize::from(run.termination == TerminationStatus::Solved);
    }
    Ok(AccountingSummary {
        engine,
        work,
        max_structural_size,
        evaluator_budget,
        solved_runs,
        run_count: runs.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse, term};

    fn provenance(task: &str) -> RunProvenance {
        RunProvenance {
            task_id: task.into(),
            family_id: "family".into(),
            duplicate_group_id: task.into(),
            context_features: BTreeMap::new(),
            concept_ids: Vec::new(),
            phase: EvidencePhase::Diagnostic,
            observed_epoch: 0,
        }
    }

    fn universal(task: &str, p: u64, e: u64) -> RunAccounting {
        RunAccounting {
            work: EngineWork::UniversalLambda {
                proposals: p,
                evaluated_candidates: e,
                resource_points: 1,
            },
            max_structural_size: 7,
            evaluator_budget: EvaluatorBudget::LambdaFuel(100),
            solution_rank: None,
            termination: TerminationStatus::ExhaustedFiniteBoundary,
            provenance: provenance(task),
        }
    }

    #[test]
    fn same_engine_totals_are_exact_and_deterministic() {
        let runs = [universal("a", 10, 3), universal("b", 20, 7)];
        let first = aggregate(&runs).unwrap();
        let replay = aggregate(&runs).unwrap();
        assert_eq!(first, replay);
        assert_eq!(
            first.work,
            EngineWork::UniversalLambda {
                proposals: 30,
                evaluated_candidates: 10,
                resource_points: 2
            }
        );
    }

    #[test]
    fn unlike_engine_units_cannot_be_aggregated() {
        let mut bank = universal("bank", 0, 0);
        bank.work = EngineWork::BehaviorBank {
            candidate_constructions: 4,
            retained_candidates: 2,
            aborted_candidates: 1,
        };
        bank.evaluator_budget = EvaluatorBudget::BankFuel(100);
        assert_eq!(
            aggregate(&[universal("lambda", 1, 1), bank]),
            Err(AccountingError::MixedEngines)
        );
    }

    #[test]
    fn behavior_bank_replay_has_stable_labeled_non_time_accounting() {
        let task = parse::Task {
            arity: 1,
            tests: vec![parse::Test {
                args: vec![term::lam(term::var(0))],
                want: term::lam(term::var(0)),
                outer: 0,
            }],
        };
        let opts = bank::Options {
            max_size: 3,
            time_budget_secs: 5.0,
            ..bank::Options::default()
        };
        let first = bank::solve(&task, &opts);
        let replay = bank::solve(&task, &opts);
        let accounted_first = RunAccounting::from_bank(&first, &opts, provenance("bank-task"));
        let accounted_replay = RunAccounting::from_bank(&replay, &opts, provenance("bank-task"));
        assert_eq!(accounted_first, accounted_replay);
        assert_eq!(accounted_first.engine(), SearchEngine::BehaviorBank);
        assert!(matches!(
            accounted_first.work,
            EngineWork::BehaviorBank { .. }
        ));
        assert_eq!(accounted_first.termination, TerminationStatus::Solved);
    }
}
