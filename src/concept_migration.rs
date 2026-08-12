//! Knowledge migration across ontology changes.
//!
//! When an ontology is revised (U6 non-monotonic repair), the *knowledge*
//! encoded in the old concept set can be carried forward, re-expressed,
//! refined, split ambiguously, or invalidated. This module classifies every
//! old concept into one of five migration kinds and compares two strategies:
//!
//!   * revision + migration  — carry preserved/refined concepts forward,
//!     apply a small re-expression map, and only re-derive what the change
//!     actually invalidated or made ambiguous;
//!   * cold restart          — discard the old ontology entirely and rediscover
//!     the new one from scratch.
//!
//! Both strategies end with the same new ontology and therefore the same
//! descriptive and reasoning cost. They differ only in the *transfer* work:
//! migration pays to carry knowledge forward; cold restart pays to relearn
//! every concept. Migration is cheaper exactly when the world changed less
//! than the full ontology, which is the honest empirical claim we measure.
//!
//! Substrate: the same behavior-bank domain as `ontology_repair`. Concepts are
//! finite predicates over probe tokens 0..15 with an executable meaning
//! (`witness`/`evaluate_witness`). Reuse `Runner` to build the old and new
//! ontologies deterministically under the same greedy MDL policy.

use std::collections::BTreeSet;

use crate::ontology_repair::{structural_cost, Concept, Ontology, Task};

// Transfer economics in comparable units (same ledger as ontology_repair).
pub const REFINE_PRICE: u64 = 1; // per-token cost to recompute an adjusted extent
pub const RENAME_PRICE: u64 = 2; // cost of a small deterministic re-expression map
pub const AMBIGUOUS_PRICE: u64 = 3; // per-token cost to re-derive an ambiguous concept
pub const INVALIDATE_PRICE: u64 = 1; // cost to discard an invalidated concept
pub const RELEARN_PRICE: u64 = 3; // per-token cost to rediscover a concept from scratch

/// The five migration kinds for a concept carried across an ontology change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MigrationKind {
    /// Identical extension and label survive; zero transfer work.
    Preserved,
    /// The class identity survives but its structural expression changed
    /// (specialized / generalized / split among same-labeled pieces).
    Refined,
    /// The class is gone as-is but all its tokens now share one *different*
    /// new label: a small symbol rename recovers the old predictions.
    ReExpressible,
    /// The old concept's tokens now map to several distinct new classes:
    /// its prior behavior is no longer reproducible without re-derivation.
    Ambiguous,
    /// The old concept's tokens no longer correspond to any coherent observed
    /// class in the new world: the concept is retired.
    Invalidated,
}

impl MigrationKind {
    pub fn name(&self) -> &'static str {
        match self {
            MigrationKind::Preserved => "preserved",
            MigrationKind::Refined => "refined",
            MigrationKind::ReExpressible => "re-expressible",
            MigrationKind::Ambiguous => "ambiguous",
            MigrationKind::Invalidated => "invalidated",
        }
    }
    /// Is this kind a net positive carry-over (cheaper than relearning)?
    pub fn carries_knowledge(&self) -> bool {
        matches!(self, MigrationKind::Preserved | MigrationKind::Refined)
    }
}

/// One classified old concept.
#[derive(Clone, Debug)]
pub struct MigrationEntry {
    pub old_id: u64,
    pub label: u8,
    pub tokens: usize,
    pub kind: MigrationKind,
    pub transfer_cost: u64,
}

/// The full migration report for one ontology change.
#[derive(Clone, Debug)]
pub struct MigrationReport {
    pub old_concepts: usize,
    pub new_concepts: usize,
    pub entries: Vec<MigrationEntry>,
    /// new concepts with no old counterpart (genuinely novel).
    pub added_concepts: u64,
    pub preserved: u64,
    pub refined: u64,
    pub re_expressible: u64,
    pub ambiguous: u64,
    pub invalidated: u64,
    pub migration_transfer: u64,
    pub cold_restart_transfer: u64,
    /// The shared description+reasoning cost of the new ontology.
    pub new_structural: u64,
    pub reasoning: u64,
    pub migration_total: u64,
    pub cold_restart_total: u64,
    pub saving: u64,
    /// Behavioral verification against the *old* task (replay) and against
    /// held-out task queries not used to build the new ontology.
    pub replay_old_task: bool,
    pub held_out_answered: bool,
}

/// Classify every old concept against the new ontology.
/// A token still "present" iff it is covered by the new ontology (the greedy
/// build only covers observed tokens). Returns entries sorted by old concept id.
pub fn classify_concepts(old: &Ontology, new: &Ontology) -> Vec<MigrationEntry> {
    let mut entries = Vec::new();
    for oc in &old.concepts {
        let kind = classify_one(oc, new);
        let transfer = transfer_cost(oc, kind);
        entries.push(MigrationEntry {
            old_id: oc.id,
            label: oc.label,
            tokens: oc.extension.len(),
            kind,
            transfer_cost: transfer,
        });
    }
    entries.sort_by_key(|e| e.old_id);
    entries
}

fn classify_one(oc: &Concept, new: &Ontology) -> MigrationKind {
    // 1. Exact identity preserved.
    if new.concepts.iter().any(|nc| {
        nc.extension == oc.extension && nc.label == oc.label
    }) {
        return MigrationKind::Preserved;
    }
    // 2. Any old token no longer observed => concept dissolved.
    let mut token_labels: BTreeSet<u8> = BTreeSet::new();
    for t in &oc.extension {
        match new.covering(*t) {
            Some(nc) => {
                token_labels.insert(nc.label);
            }
            None => return MigrationKind::Invalidated,
        }
    }
    let distinct = token_labels.len();
    if distinct == 0 {
        return MigrationKind::Invalidated;
    }
    if distinct == 1 {
        let single = *token_labels.iter().next().unwrap();
        if single == oc.label {
            MigrationKind::Refined
        } else {
            MigrationKind::ReExpressible
        }
    } else {
        MigrationKind::Ambiguous
    }
}

fn transfer_cost(oc: &Concept, kind: MigrationKind) -> u64 {
    match kind {
        MigrationKind::Preserved => 0,
        MigrationKind::Refined => REFINE_PRICE * oc.extension.len() as u64,
        MigrationKind::ReExpressible => RENAME_PRICE,
        MigrationKind::Ambiguous => AMBIGUOUS_PRICE * oc.extension.len() as u64,
        MigrationKind::Invalidated => INVALIDATE_PRICE,
    }
}

/// Run the full migration comparison for one ontology change.
///
/// `old`/`new` are the two ontologies (built by `ontology_repair::Runner`);
/// `new_task` is the downstream task the new ontology must answer; `old_task`
/// is used for replay verification; `held_out` are additional queries the new
/// ontology should answer without having been used to build it.
pub fn migrate(
    old: &Ontology,
    new: &Ontology,
    old_task: &Task,
    new_task: &Task,
    held_out: &Task,
) -> MigrationReport {
    let entries = classify_concepts(old, new);

    // Counts.
    let mut preserved = 0u64;
    let mut refined = 0u64;
    let mut re_expressible = 0u64;
    let mut ambiguous = 0u64;
    let mut invalidated = 0u64;
    let mut migration_transfer = 0u64;
    for e in &entries {
        migration_transfer += e.transfer_cost;
        match e.kind {
            MigrationKind::Preserved => preserved += 1,
            MigrationKind::Refined => refined += 1,
            MigrationKind::ReExpressible => re_expressible += 1,
            MigrationKind::Ambiguous => ambiguous += 1,
            MigrationKind::Invalidated => invalidated += 1,
        }
    }

    // Cold restart: relearn every new concept from scratch.
    let cold_restart_transfer: u64 = new
        .concepts
        .iter()
        .map(|nc| RELEARN_PRICE * nc.extension.len() as u64)
        .sum();

    // Novel concepts (no old counterpart).
    let added_concepts = new
        .concepts
        .iter()
        .filter(|nc| {
            !old
                .concepts
                .iter()
                .any(|oc| oc.extension == nc.extension && oc.label == nc.label)
        })
        .count() as u64;

    // Shared structural + reasoning cost of the new ontology.
    let ledger = structural_cost(new, new_task);
    let new_structural = ledger.description;
    let reasoning = ledger.reasoning;

    let migration_total = new_structural + reasoning + migration_transfer;
    let cold_restart_total = new_structural + reasoning + cold_restart_transfer;
    let saving = cold_restart_total.saturating_sub(migration_total);

    // Behavioral verification.
    let replay_old_task = old_task
        .queries
        .iter()
        .all(|(t, l)| new.covering(*t).map(|c| c.label == *l).unwrap_or(false));
    let held_out_answered = held_out
        .queries
        .iter()
        .all(|(t, l)| new.covering(*t).map(|c| c.label == *l).unwrap_or(false));

    MigrationReport {
        old_concepts: old.concepts.len(),
        new_concepts: new.concepts.len(),
        entries,
        added_concepts,
        preserved,
        refined,
        re_expressible,
        ambiguous,
        invalidated,
        migration_transfer,
        cold_restart_transfer,
        new_structural,
        reasoning,
        migration_total,
        cold_restart_total,
        saving,
        replay_old_task,
        held_out_answered,
    }
}

/// Deterministic machine-readable record for one migration report.
pub fn machine_record(r: &MigrationReport) -> String {
    let entries: String = r
        .entries
        .iter()
        .map(|e| format!("{}:{}:{}", e.old_id, e.kind.name(), e.transfer_cost))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "experiment=concept_migration,old_concepts={},new_concepts={},preserved={},refined={},re_expressible={},ambiguous={},invalidated={},added={},migration_transfer={},cold_restart_transfer={},new_structural={},reasoning={},migration_total={},cold_restart_total={},saving={},replay_old_task={},held_out_answered={},entries=[{}],deterministic=true,fallback=exact",
        r.old_concepts,
        r.new_concepts,
        r.preserved,
        r.refined,
        r.re_expressible,
        r.ambiguous,
        r.invalidated,
        r.added_concepts,
        r.migration_transfer,
        r.cold_restart_transfer,
        r.new_structural,
        r.reasoning,
        r.migration_total,
        r.cold_restart_total,
        r.saving,
        r.replay_old_task,
        r.held_out_answered,
        entries,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology_repair::{Observation, RepairSpec, Runner};

    fn obs(tokens: &[(u8, u8)]) -> Vec<Observation> {
        tokens
            .iter()
            .map(|(t, l)| Observation {
                token: *t,
                label: *l,
            })
            .collect()
    }
    fn task(tokens: &[(u8, u8)]) -> Task {
        Task {
            queries: tokens.iter().map(|(t, l)| (*t, *l)).collect(),
        }
    }
    fn kind_of(r: &MigrationReport, id: u64) -> MigrationKind {
        r.entries
            .iter()
            .find(|e| e.old_id == id)
            .unwrap()
            .kind
    }

    #[test]
    fn preserved_costs_zero() {
        let mut run = Runner::new(RepairSpec::default());
        let o = obs(&[(0, 0), (1, 0), (2, 1), (3, 1), (4, 2), (5, 2)]);
        let t = task(&[(0, 0), (1, 0), (2, 1), (3, 1), (4, 2), (5, 2)]);
        run.add_stage(&o, &t); // stage 0
        run.add_stage(&o, &t); // unchanged world
        let r = migrate(&run.stages[0].ontology, &run.stages[1].ontology, &t, &t, &t);
        assert_eq!(r.preserved, 3);
        assert_eq!(r.invalidated, 0);
        assert_eq!(r.migration_transfer, 0);
        assert!(r.replay_old_task);
        assert!(r.held_out_answered);
        assert!(r.saving > 0, "cold restart must cost more than migration");
    }

    #[test]
    fn refinement_is_cheaper_than_relearn() {
        let mut run = Runner::new(RepairSpec::default());
        // old: two concepts {0,1}->0, {2,3}->1
        run.add_stage(
            &obs(&[(0, 0), (1, 0), (2, 1), (3, 1)]),
            &task(&[(0, 0), (1, 0), (2, 1), (3, 1)]),
        );
        // new: {2,3} refined/split? make it a pure refinement: token 4,5 join class 1
        run.add_stage(
            &obs(&[(0, 0), (1, 0), (2, 1), (3, 1), (4, 1), (5, 1)]),
            &task(&[(0, 0), (1, 0), (2, 1), (3, 1), (4, 1), (5, 1)]),
        );
        let old = run.stages[0].ontology.clone();
        let new = run.stages[1].ontology.clone();
        let t = task(&[(0, 0), (1, 0), (2, 1), (3, 1), (4, 1), (5, 1)]);
        let r = migrate(&old, &new, &t, &t, &t);
        // the class-1 concept generalized (widened): Refined; class-0 preserved
        assert_eq!(r.refined, 1, "generalization should classify as refined");
        assert_eq!(r.preserved, 1);
        assert!(r.migration_transfer < r.cold_restart_transfer);
        assert!(r.saving > 0);
    }

    #[test]
    fn relabel_is_re_expressible() {
        let mut run = Runner::new(RepairSpec::default());
        run.add_stage(
            &obs(&[(0, 0), (1, 0), (2, 1), (3, 1)]),
            &task(&[(0, 0), (1, 0), (2, 1), (3, 1)]),
        );
        // whole world labels shift by +10: every concept is re-expressible
        run.add_stage(
            &obs(&[(0, 10), (1, 10), (2, 11), (3, 11)]),
            &task(&[(0, 10), (1, 10), (2, 11), (3, 11)]),
        );
        let old = run.stages[0].ontology.clone();
        let new = run.stages[1].ontology.clone();
        let t = task(&[(0, 10), (1, 10), (2, 11), (3, 11)]);
        let r = migrate(&old, &new, &task(&[(0, 0), (2, 1)]), &t, &t);
        assert_eq!(r.re_expressible, 2);
        assert_eq!(r.preserved, 0);
        assert_eq!(r.ambiguous, 0);
    }

    #[test]
    fn split_into_different_classes_is_ambiguous() {
        let mut run = Runner::new(RepairSpec::default());
        run.add_stage(
            &obs(&[(0, 0), (1, 0), (2, 1), (3, 1)]),
            &task(&[(0, 0), (1, 0), (2, 1), (3, 1)]),
        );
        // the world now says token 2 is a different class from token 3
        run.add_stage(
            &obs(&[(0, 0), (1, 0), (2, 9), (3, 1)]),
            &task(&[(0, 0), (1, 0), (2, 9), (3, 1)]),
        );
        let old = run.stages[0].ontology.clone();
        let new = run.stages[1].ontology.clone();
        let t = task(&[(0, 0), (1, 0), (2, 9), (3, 1)]);
        let r = migrate(&old, &new, &task(&[(0, 0), (2, 1)]), &t, &t);
        // old concept {2,3}->1 is now split across labels 1 and 9 => ambiguous
        assert_eq!(kind_of(&r, 1), MigrationKind::Ambiguous);
        assert_eq!(r.ambiguous, 1);
        // old concept {0,1}->0 preserved
        assert_eq!(kind_of(&r, 0), MigrationKind::Preserved);
    }

    #[test]
    fn vanished_tokens_are_invalidated() {
        // The Runner accumulates observations (tokens never disappear), so
        // invalidation is exercised with manually constructed ontologies.
        fn set(items: &[u8]) -> std::collections::BTreeSet<u8> {
            items.iter().copied().collect()
        }
        fn con(id: u64, ext: &[u8], label: u8) -> Concept {
            Concept {
                id,
                extension: set(ext),
                label,
            }
        }
        let old = Ontology {
            concepts: vec![con(0, &[0, 1], 0), con(1, &[2, 3], 1)],
        };
        // new world no longer observes tokens 2,3 at all
        let new = Ontology {
            concepts: vec![con(0, &[0, 1], 0), con(1, &[4, 5], 1)],
        };
        let old_task = task(&[(0, 0), (1, 0), (2, 1), (3, 1)]);
        let new_task = task(&[(0, 0), (1, 0), (4, 1), (5, 1)]);
        let r = migrate(&old, &new, &old_task, &new_task, &new_task);
        assert_eq!(kind_of(&r, 1), MigrationKind::Invalidated);
        assert_eq!(r.invalidated, 1);
        assert_eq!(kind_of(&r, 0), MigrationKind::Preserved);
        // replay of the old task is impossible: tokens 2,3 are no longer covered
        assert!(!r.replay_old_task);
    }

    #[test]
    fn machine_record_is_deterministic_and_complete() {
        let mut run = Runner::new(RepairSpec::default());
        run.add_stage(
            &obs(&[(0, 0), (1, 0), (2, 1), (3, 1)]),
            &task(&[(0, 0), (1, 0), (2, 1), (3, 1)]),
        );
        run.add_stage(
            &obs(&[(0, 0), (1, 0), (2, 1), (3, 1), (4, 1), (5, 1)]),
            &task(&[(0, 0), (1, 0), (2, 1), (3, 1), (4, 1), (5, 1)]),
        );
        let old = run.stages[0].ontology.clone();
        let new = run.stages[1].ontology.clone();
        let t = task(&[(0, 0), (1, 0), (2, 1), (3, 1), (4, 1), (5, 1)]);
        let r = migrate(&old, &new, &t, &t, &t);
        let a = machine_record(&r);
        let b = machine_record(&r);
        assert_eq!(a, b);
        assert!(a.contains("experiment=concept_migration"));
        assert!(a.contains("deterministic=true"));
        assert!(a.contains("preserved=1"));
        assert!(a.contains("refined=1"));
    }
}
