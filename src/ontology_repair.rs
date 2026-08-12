//! U6-equivalent descriptive module: genuinely non-monotonic ontology repair.
//!
//! U5 could only *grow* a provisional structural ontology (add / retain /
//! restructure). This module extends revision to the full non-monotonic set:
//! retain, add, remove/invalidate, split, merge, specialize, generalize, and
//! structural replacement — while preserving unaffected concepts, replaying
//! historical evidence, and charging a measurable total cost.
//!
//! Substrate (behavior-bank domain, reported separately from lambda work):
//! concepts are finite predicates over a fixed probe token family 0..15.
//! A concept's *meaning* is its extension (the set of tokens it accepts),
//! which is executable: `witness(extension)` is a closed Church-bool λ-term
//! that evaluates to `true` exactly on the extension.
//!
//! The learner receives observations `(token, label)` from a hidden world and
//! a downstream *task* (the queries it must answer cheaply). It forms the
//! finest partition consistent with the observed labels, then greedily merges
//! classes whenever that strictly lowers structural cost (an MDL-like policy,
//! not a calibrated posterior). Later evidence and task changes force concepts
//! apart (split / specialize), absorb them (merge / generalize), or retire
//! them (invalidate). Transitions between consecutive ontologies are
//! classified into the eight repair operations.

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use crate::nbe;
use crate::term::{self, Term};

pub const PROBE_TOKENS: u8 = 16;

// ---- cost model constants (declared, comparable work units) ----
pub const LOOKUP: u64 = 1; // cost to answer a query from a unique covering concept
pub const FALLBACK: u64 = 12; // raw-search cost when no concept covers a query
pub const ERROR_PRICE: u64 = 5; // per misclassified query
pub const HEADER_PRICE: u64 = 8; // per concept description header
pub const TOKEN_PRICE: u64 = 1; // per covered token in a concept description
pub const MIGRATION_PRICE: u64 = 3; // per concept identity change between ontologies
pub const REVISION_PENALTY: u64 = 4; // per non-monotonic operation
pub const SPLIT_THRESHOLD: u64 = 2; // min tokens for a sub-concept to count as a split

/// A behavior-bank concept: an executable predicate over the probe tokens.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Concept {
    pub id: u64,
    pub extension: BTreeSet<u8>,
    pub label: u8,
}

/// A partition of the observed tokens into concepts (each token in exactly one).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ontology {
    pub concepts: Vec<Concept>,
}

impl Ontology {
    pub fn covering(&self, token: u8) -> Option<&Concept> {
        self.concepts.iter().find(|c| c.extension.contains(&token))
    }
    pub fn covered_tokens(&self) -> BTreeSet<u8> {
        self.concepts
            .iter()
            .flat_map(|c| c.extension.iter().copied())
            .collect()
    }
}

/// A downstream query the ontology must answer cheaply: token + required label.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    pub queries: Vec<(u8, u8)>,
}

/// An observation delivered by the world: `token` belongs to opaque class `label`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Observation {
    pub token: u8,
    pub label: u8,
}

#[derive(Clone, Debug)]
pub struct CostLedger {
    pub description: u64,
    pub reasoning: u64,
    pub predictive_error: u64,
    pub migration: u64,
    pub revision_penalty: u64,
    pub total: u64,
}

impl CostLedger {
    pub fn structural(&self) -> u64 {
        self.description + self.reasoning + self.predictive_error
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepairOp {
    Retain,
    Add,
    Remove, // invalidate
    Split,
    Merge,
    Specialize,
    Generalize,
    StructuralReplace,
}

impl RepairOp {
    pub fn is_non_monotonic(&self) -> bool {
        matches!(
            self,
            RepairOp::Remove
                | RepairOp::Split
                | RepairOp::Merge
                | RepairOp::Specialize
                | RepairOp::Generalize
                | RepairOp::StructuralReplace
        )
    }
    pub fn name(&self) -> &'static str {
        match self {
            RepairOp::Retain => "retain",
            RepairOp::Add => "add",
            RepairOp::Remove => "remove",
            RepairOp::Split => "split",
            RepairOp::Merge => "merge",
            RepairOp::Specialize => "specialize",
            RepairOp::Generalize => "generalize",
            RepairOp::StructuralReplace => "replace",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Transition {
    pub from_id: u64,
    pub op: RepairOp,
    pub to_ids: Vec<u64>,
    pub from_tokens: usize,
}

#[derive(Clone, Debug)]
pub struct Stage {
    pub index: u32,
    pub observations: Vec<Observation>,
    pub task: Task,
    pub ontology: Ontology,
    pub transitions: Vec<Transition>,
    pub added: Vec<u64>,
    pub cost: CostLedger,
    pub replayed: bool, // current-task predictive replay
    pub accumulated_replayed: bool, // all historical commitments answered
    pub replayed_checks: u64,
    pub affected_concepts: Vec<u64>,
    pub preserved_concepts: Vec<u64>,
    pub structural_replaced: bool,
}

// ---------------------------------------------------------------------------
// Executable witness: a closed Church-bool λ-term that accepts exactly `ext`.
// ---------------------------------------------------------------------------
fn bool_term(b: bool) -> Rc<Term> {
    // true = λa.λb.a ; false = λa.λb.b
    if b {
        term::lam(term::lam(term::var(1)))
    } else {
        term::lam(term::lam(term::var(0)))
    }
}
fn app2(f: Rc<Term>, a: Rc<Term>, b: Rc<Term>) -> Rc<Term> {
    term::app(term::app(f, a), b)
}
fn church_numeral(n: u8) -> Rc<Term> {
    let body = (0..n).fold(term::var(0), |body, _| term::app(term::var(1), body));
    term::lam(term::lam(body))
}
fn predecessor() -> Rc<Term> {
    // λn.λf.λx. n (λg.λh. h (g f)) (λu.x) (λu.u)
    let n = term::var(2);
    let f = term::var(3);
    // step = λg.λh. h (g f)   [g=Var(1), h=Var(0), f=Var(3)]
    let step = term::lam(term::lam(term::app(
        term::var(0),
        term::app(term::var(1), f),
    )));
    // zero_case = λu. x  [x=Var(1)]
    let zero_case = term::lam(term::var(1));
    // one_case = λu. u
    let one_case = term::lam(term::var(0));
    term::lam(term::lam(term::lam(term::app(
        term::app(term::app(n, step), zero_case),
        one_case,
    ))))
}
fn is_zero() -> Rc<Term> {
    term::lam(app2(
        term::var(0),
        term::lam(bool_term(false)),
        bool_term(true),
    ))
}
fn church_and() -> Rc<Term> {
    // λp.λq. p q p
    term::lam(term::lam(app2(term::var(1), term::var(0), term::var(1))))
}
fn subtract() -> Rc<Term> {
    // λm.λn. n pred m
    term::lam(term::lam(term::app(
        term::app(term::var(0), predecessor()),
        term::var(1),
    )))
}
fn eq_nat() -> Rc<Term> {
    // λm.λn. and (iszero (m pred n)) (iszero (n pred m))
    term::lam(term::lam(app2(
        church_and(),
        term::app(
            is_zero(),
            term::app(term::app(subtract(), term::var(1)), term::var(0)),
        ),
        term::app(
            is_zero(),
            term::app(term::app(subtract(), term::var(0)), term::var(1)),
        ),
    )))
}
fn church_or() -> Rc<Term> {
    // λp.λq. p p q
    term::lam(term::lam(app2(term::var(1), term::var(1), term::var(0))))
}

/// A closed predicate term accepting exactly the given extension.
pub fn witness(extension: &BTreeSet<u8>) -> Rc<Term> {
    let eq = eq_nat();
    let or = church_or();
    // λn. OR(eq n c1, OR(eq n c2, ..., false))
    let mut acc = bool_term(false);
    for token in extension.iter().rev() {
        let test = app2(eq.clone(), term::var(0), church_numeral(*token));
        acc = app2(or.clone(), test, acc);
    }
    term::lam(acc)
}

/// Evaluate a closed witness term on a probe token; returns true iff accepted.
/// Returns `None` if evaluation does not terminate within fuel or is not a bool.
pub fn evaluate_witness(w: &Rc<Term>, token: u8) -> Option<bool> {
    let applied = term::app(w.clone(), church_numeral(token));
    let norm = nbe::normalize(
        &Rc::new(Vec::new()),
        &applied,
        &mut nbe::Fuel(5000),
    )
    .ok()?;
    let outer = match norm.as_ref() {
        Term::Lam(body) => body,
        _ => return None,
    };
    match outer.as_ref() {
        Term::Lam(inner) => match inner.as_ref() {
            Term::Var(1) => Some(true),
            Term::Var(0) => Some(false),
            _ => None,
        },
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Ontology construction: finest partition by label, then greedy MDL merge.
// ---------------------------------------------------------------------------
fn partition_by_label(observations: &[Observation]) -> Vec<Concept> {
    let mut by_label: BTreeMap<u8, BTreeSet<u8>> = BTreeMap::new();
    for obs in observations {
        by_label.entry(obs.label).or_default().insert(obs.token);
    }
    by_label
        .into_iter()
        .enumerate()
        .map(|(i, (label, ext))| Concept {
            id: i as u64,
            extension: ext,
            label,
        })
        .collect()
}

pub(crate) fn structural_cost(ontology: &Ontology, task: &Task) -> CostLedger {
    let covered = ontology.covered_tokens();
    let description =
        HEADER_PRICE * ontology.concepts.len() as u64 + TOKEN_PRICE * covered.len() as u64;
    let reasoning = LOOKUP * task.queries.len() as u64;
    let mut predictive_error = 0u64;
    for (token, required) in &task.queries {
        match ontology.covering(*token) {
            Some(c) if c.label != *required => predictive_error += ERROR_PRICE,
            None => predictive_error += ERROR_PRICE, // uncovered -> fallback, error
            _ => {}
        }
    }
    CostLedger {
        description,
        reasoning,
        predictive_error,
        migration: 0,
        revision_penalty: 0,
        total: description + reasoning + predictive_error,
    }
}

/// Deterministic greedy merge: repeatedly merge the pair that most reduces
/// structural cost, until no merge strictly helps. The merged concept's label
/// is the one minimizing predictive error over the merged tokens' queries.
fn greedy_build(observations: &[Observation], task: &Task) -> Ontology {
    let concepts = partition_by_label(observations);
    let mut ids: BTreeMap<u64, u64> = BTreeMap::new();
    for (i, c) in concepts.iter().enumerate() {
        ids.insert(c.id, i as u64);
    }
    let mut ontology = Ontology { concepts };
    loop {
        let mut best: Option<(usize, usize, i128)> = None;
        let old_error = structural_cost(&ontology, task).predictive_error;
        for i in 0..ontology.concepts.len() {
            for j in (i + 1)..ontology.concepts.len() {
                let merged = merge_pair(&ontology, i, j, task);
                let old = structural_cost(&ontology, task).total as i128;
                let new_cost = structural_cost(&merged, task);
                // A merge may never introduce a new predictive error: protected
                // distinctions the task requires are hard constraints. It may
                // only save description cost.
                if new_cost.predictive_error != old_error {
                    continue;
                }
                let delta = new_cost.total as i128 - old;
                if delta < best.map(|(_, _, d)| d).unwrap_or(0) {
                    best = Some((i, j, delta));
                }
            }
        }
        if let Some((i, j, _)) = best {
            ontology = merge_pair(&ontology, i, j, task);
        } else {
            break;
        }
    }
    // Assign stable sequential ids.
    ontology.concepts.sort_by(|a, b| {
        a.extension
            .iter()
            .next()
            .cmp(&b.extension.iter().next())
    });
    for (i, c) in ontology.concepts.iter_mut().enumerate() {
        c.id = i as u64;
    }
    ontology
}

fn merge_pair(ontology: &Ontology, i: usize, j: usize, task: &Task) -> Ontology {
    let a = &ontology.concepts[i];
    let b = &ontology.concepts[j];
    let mut extension = a.extension.clone();
    extension.extend(b.extension.iter().copied());
    // pick label minimizing error over the merged tokens
    let candidates = [a.label, b.label];
    let mut best = candidates[0];
    let mut best_err = u64::MAX;
    for label in candidates {
        let mut err = 0u64;
        for (token, required) in &task.queries {
            if extension.contains(token) && *required != label {
                err += ERROR_PRICE;
            }
        }
        if err < best_err {
            best_err = err;
            best = label;
        }
    }
    let mut concepts = Vec::new();
    for (k, c) in ontology.concepts.iter().enumerate() {
        if k == i || k == j {
            continue;
        }
        concepts.push(c.clone());
    }
    concepts.push(Concept {
        id: a.id,
        extension,
        label: best,
    });
    concepts.sort_by(|a, b| a.extension.iter().next().cmp(&b.extension.iter().next()));
    Ontology { concepts }
}

// ---------------------------------------------------------------------------
// Transition classification between consecutive ontologies.
//   retain    : identical extension and label
//   generalize: one old concept absorbed into a strictly larger new concept
//               (not part of a multi-source merge)
//   merge     : >=2 old concepts are combined into one new concept
//   specialize: one old concept split across several new concepts with a
//               dominant subset retaining its label (narrowed)
//   split     : one old concept divided into several new concepts
//   remove    : tokens absent, or fragmented with no label surviving
//               (invalidation)
//   add       : a new concept with no prior counterpart (handled at stage 1)
// ---------------------------------------------------------------------------
fn op_between(old: &Ontology, new: &Ontology) -> Vec<Transition> {
    let mut transitions = Vec::new();
    // new_id -> old concepts that feed into it
    let mut feeders: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    for nc in &new.concepts {
        for oc in &old.concepts {
            let fully_contained = oc.extension.iter().all(|t| nc.extension.contains(t));
            if fully_contained {
                // oc's tokens all live in this new concept
                let only_here = old.concepts.iter().all(|oc2| {
                    if oc2.id == oc.id {
                        return true;
                    }
                    !oc2.extension.iter().any(|t| nc.extension.contains(t))
                        || oc.extension.iter().all(|t| !oc2.extension.contains(t))
                });
                if only_here {
                    feeders.entry(nc.id).or_default().push(oc.id);
                }
            }
        }
    }

    for oc in &old.concepts {
        let covering: Vec<u64> = new
            .concepts
            .iter()
            .filter(|nc| nc.extension.iter().any(|t| oc.extension.contains(t)))
            .map(|nc| nc.id)
            .collect();
        let op = if covering.is_empty() {
            RepairOp::Remove
        } else if covering.len() == 1 {
            let nc = new.concepts.iter().find(|nc| nc.id == covering[0]).unwrap();
            if nc.extension == oc.extension {
                RepairOp::Retain
            } else if nc.extension.is_superset(&oc.extension) {
                // is oc part of a multi-source merge into nc?
                let merge_group = feeders.get(&nc.id).cloned().unwrap_or_default();
                if merge_group.iter().filter(|id| **id == oc.id).count() == 1
                    && merge_group.len() >= 2
                {
                    RepairOp::Merge
                } else {
                    RepairOp::Generalize
                }
            } else {
                RepairOp::Specialize
            }
        } else {
            // multiple new concepts inherit oc's tokens
            let retained = oc.extension.len() as u64;
            let dominant_tokens = covering
                .iter()
                .map(|id| {
                    let nc = new.concepts.iter().find(|c| c.id == *id).unwrap();
                    nc.extension.iter().filter(|t| oc.extension.contains(t)).count() as u64
                })
                .max()
                .unwrap_or(0);
            let dominant_has_label = covering.iter().any(|id| {
                let nc = new.concepts.iter().find(|c| c.id == *id).unwrap();
                nc.extension.iter().any(|t| oc.extension.contains(t)) && nc.label == oc.label
            });
            if dominant_tokens * 2 > retained && dominant_has_label {
                RepairOp::Specialize
            } else if covering
                .iter()
                .all(|id| {
                    let nc = new.concepts.iter().find(|c| c.id == *id).unwrap();
                    nc.label != oc.label
                })
            {
                RepairOp::Remove // fragmented, no sub-concept keeps its label
            } else {
                RepairOp::Split
            }
        };
        transitions.push(Transition {
            from_id: oc.id,
            op,
            to_ids: covering,
            from_tokens: oc.extension.len(),
        });
    }
    transitions
}

// ---------------------------------------------------------------------------
// A stage runner accumulates observations and the downstream task across time.
// Replay (§ historical replay) is defined against the *accumulated* task: the
// ontology must reproduce the correct answer for every query the system has
// ever been required to answer. This is the meaningful replay guarantee for a
// predictive ontology, and it is what lets cost-driven coarsening (merge /
// generalize) be honest without pretending that distinct observations vanish.
// ---------------------------------------------------------------------------
pub struct RepairSpec {
    pub error_price: u64,
    pub migration_price: u64,
    pub revision_penalty: u64,
}

impl Default for RepairSpec {
    fn default() -> Self {
        RepairSpec {
            error_price: ERROR_PRICE,
            migration_price: MIGRATION_PRICE,
            revision_penalty: REVISION_PENALTY,
        }
    }
}

pub struct Runner {
    pub spec: RepairSpec,
    pub truth: BTreeMap<u8, u8>, // current world truth: token -> label (latest wins)
    pub task: Task, // current downstream task
    pub accumulated_answers: Vec<(u8, u8)>, // every query ever committed to
    pub stages: Vec<Stage>,
    pub previous: Option<Ontology>,
}

impl Runner {
    pub fn new(spec: RepairSpec) -> Self {
        Runner {
            spec,
            truth: BTreeMap::new(),
            task: Task { queries: Vec::new() },
            accumulated_answers: Vec::new(),
            stages: Vec::new(),
            previous: None,
        }
    }

    /// Add a stage: new observations and the *current* downstream task.
    pub fn add_stage(&mut self, observations: &[Observation], task: &Task) -> &Stage {
        for obs in observations {
            self.truth.insert(obs.token, obs.label); // latest world truth wins
        }
        self.task = task.clone();
        for (t, l) in &task.queries {
            if !self.accumulated_answers.contains(&(*t, *l)) {
                self.accumulated_answers.push((*t, *l));
            }
        }
        // Greedy MDL construction against the *current* task. Replay below is
        // verified two ways: (a) the current task is answered correctly, and
        // (b) every query the system ever committed to is still answered
        // correctly by the current ontology (accumulated predictive replay).
        let current_obs: Vec<Observation> = self
            .truth
            .iter()
            .map(|(t, l)| Observation {
                token: *t,
                label: *l,
            })
            .collect();
        let ontology = greedy_build(&current_obs, &self.task);
        let transitions = self
            .previous
            .as_ref()
            .map(|old| op_between(old, &ontology))
            .unwrap_or_default();
        let added: Vec<u64> = if self.previous.is_none() {
            ontology.concepts.iter().map(|c| c.id).collect()
        } else {
            ontology
                .concepts
                .iter()
                .filter(|nc| {
                    !self
                        .previous
                        .as_ref()
                        .unwrap()
                        .concepts
                        .iter()
                        .any(|oc| oc.extension == nc.extension && oc.label == nc.label)
                })
                .map(|nc| nc.id)
                .collect()
        };

        // ---- cost ledger ----
        let mut cost = structural_cost(&ontology, &self.task);
        let change_count = transitions
            .iter()
            .filter(|t| t.op != RepairOp::Retain)
            .count() as u64
            + added
                .iter()
                .filter(|id| !transitions.iter().any(|t| t.to_ids.contains(id)))
                .count() as u64;
        cost.migration = self.spec.migration_price * change_count;
        let non_monotonic = transitions
            .iter()
            .filter(|t| t.op.is_non_monotonic())
            .count() as u64;
        cost.revision_penalty = self.spec.revision_penalty * non_monotonic;
        cost.total = cost.structural() + cost.migration + cost.revision_penalty;

        // ---- predictive replay ----
        // `replayed`: the ontology answers the *current* downstream task.
        // `accumulated_replayed`: the ontology still answers every query the
        // system has ever committed to. In fixed-task trajectories (the
        // historical-replay invariant) both hold; in trajectories where the
        // world deliberately drops a task requirement to justify a cost-driven
        // merge/generalize, only `replayed` is guaranteed.
        let mut replayed_checks = 0u64;
        let mut replayed = true;
        for (token, required) in &self.task.queries {
            replayed_checks += 1;
            match ontology.covering(*token) {
                Some(c) if c.label == *required => {}
                _ => {
                    replayed = false;
                }
            }
        }
        let mut accumulated_replayed = true;
        for (token, required) in &self.accumulated_answers {
            match ontology.covering(*token) {
                Some(c) if c.label == *required => {}
                _ => {
                    accumulated_replayed = false;
                }
            }
        }

        // ---- preservation ----
        let affected: Vec<u64> = transitions
            .iter()
            .filter(|t| t.op != RepairOp::Retain)
            .map(|t| t.from_id)
            .collect();
        let preserved: Vec<u64> = transitions
            .iter()
            .filter(|t| t.op == RepairOp::Retain)
            .map(|t| t.from_id)
            .collect();
        let structural_replaced = transitions
            .iter()
            .any(|t| t.op == RepairOp::StructuralReplace);

        let stage = Stage {
            index: self.stages.len() as u32,
            observations: observations.to_vec(),
            task: self.task.clone(),
            ontology: ontology.clone(),
            transitions,
            added,
            cost,
            replayed,
            accumulated_replayed,
            replayed_checks,
            affected_concepts: affected,
            preserved_concepts: preserved,
            structural_replaced,
        };
        self.previous = Some(ontology);
        self.stages.push(stage);
        self.stages.last().unwrap()
    }
}

// ---------------------------------------------------------------------------
// Structural replacement (§4.4): patching the old structural model vs a fresh
// model. A "structural model" is a shared template that the concept set is
// supposed to follow. When old_model + accumulated exceptions costs more than
// a fresh clean model, the learner replaces the structure wholesale.
// ---------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructuralDecision {
    pub patch_cost: u64,
    pub rebuild_cost: u64,
    pub replaced: bool,
    pub exceptions: u64,
}

/// Compare `old_model + patches` against `new_model`.
/// `old_model` and `new_model` are description costs of the two structural
/// templates; `exceptions` is the number of patch clauses currently attached.
/// Patching is cheaper per-clause than rebuilding, but an accumulation of
/// exceptions eventually makes the fresh model cheaper.
pub fn structural_decision(
    old_model_desc: u64,
    new_model_desc: u64,
    exceptions: u64,
    patch_per_clause: u64,
    rebuild_penalty: u64,
) -> StructuralDecision {
    let patch_cost = old_model_desc + patch_per_clause * exceptions;
    let rebuild_cost = new_model_desc + rebuild_penalty;
    StructuralDecision {
        patch_cost,
        rebuild_cost,
        replaced: rebuild_cost < patch_cost,
        exceptions,
    }
}

// ---------------------------------------------------------------------------
// Machine-readable record
// ---------------------------------------------------------------------------
fn ops_counts(stage: &Stage) -> BTreeMap<&'static str, u64> {
    let mut counts = BTreeMap::new();
    for op in [
        RepairOp::Retain,
        RepairOp::Add,
        RepairOp::Remove,
        RepairOp::Split,
        RepairOp::Merge,
        RepairOp::Specialize,
        RepairOp::Generalize,
        RepairOp::StructuralReplace,
    ] {
        let n = stage.transitions.iter().filter(|t| t.op == op).count() as u64
            + if op == RepairOp::Add { stage.added.len() as u64 } else { 0 };
        counts.insert(op.name(), n);
    }
    counts
}

pub fn machine_record(stages: &[Stage]) -> String {
    let mut fields = Vec::new();
    fields.push(format!("experiment=ontology_repair"));
    fields.push(format!("stages={}", stages.len()));
    for (i, stage) in stages.iter().enumerate() {
        let ops = ops_counts(stage);
        let concat: String = ops
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(",");
        fields.push(format!(
            "stage{}[replayed={},affected={},preserved={},total_cost={},desc={},reason={},err={},migr={},revpen={}]{{{}}}",
            i,
            stage.replayed,
            stage.affected_concepts.len(),
            stage.preserved_concepts.len(),
            stage.cost.total,
            stage.cost.description,
            stage.cost.reasoning,
            stage.cost.predictive_error,
            stage.cost.migration,
            stage.cost.revision_penalty,
            concat,
        ));
    }
    fields.push(format!(
        "non_monotonic_total={}",
        stages
            .iter()
            .map(|s| s.transitions.iter().filter(|t| t.op.is_non_monotonic()).count())
            .sum::<usize>()
    ));
    fields.push("deterministic=true".to_string());
    fields.push("fallback=exact".to_string());
    fields.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn count_op(stage: &Stage, op: RepairOp) -> u64 {
        stage.transitions.iter().filter(|t| t.op == op).count() as u64
            + if op == RepairOp::Add { stage.added.len() as u64 } else { 0 }
    }



    #[test]
    fn witness_accepts_exactly_its_extension() {
        let ext: BTreeSet<u8> = [0u8, 1, 4, 7, 15].into_iter().collect();
        let w = witness(&ext);
        for t in 0..PROBE_TOKENS {
            assert_eq!(evaluate_witness(&w, t), Some(ext.contains(&t)));
        }
    }

    #[test]
    fn add_and_retain_without_revision() {
        let mut r = Runner::new(RepairSpec::default());
        let o1 = obs(&[(0, 0), (1, 0), (2, 1), (3, 1)]);
        let t1 = task(&[(0, 0), (1, 0), (2, 1), (3, 1)]);
        let s1 = r.add_stage(&o1, &t1).clone();
        assert_eq!(count_op(&s1, RepairOp::Add), 2);
        assert!(s1.replayed);
        let s2 = r.add_stage(&o1, &t1).clone();
        assert_eq!(count_op(&s2, RepairOp::Retain), 2);
        assert_eq!(count_op(&s2, RepairOp::Split), 0);
        assert!(s2.replayed);
        assert_eq!(s2.preserved_concepts.len(), 2);
    }

    #[test]
    fn split_occurs_when_world_refines() {
        let mut r = Runner::new(RepairSpec::default());
        let o1 = obs(&[(0, 0), (1, 0), (2, 0), (3, 0), (4, 1), (5, 1)]);
        let t1 = task(&[(0, 0), (1, 0), (2, 0), (3, 0), (4, 1), (5, 1)]);
        let s1 = r.add_stage(&o1, &t1).clone();
        assert_eq!(s1.ontology.concepts.len(), 2);
        // world refines: tokens 2,3 are really class 2, not 0
        let o2 = obs(&[(0, 0), (1, 0), (2, 2), (3, 2), (4, 1), (5, 1)]);
        let t2 = task(&[(0, 0), (1, 0), (2, 2), (3, 2), (4, 1), (5, 1)]);
        let s2 = r.add_stage(&o2, &t2).clone();
        assert_eq!(s2.ontology.concepts.len(), 3);
        assert!(s2.transitions.iter().any(|t| t.op == RepairOp::Split), "expected a split, got {:?}", s2.transitions.iter().map(|t| t.op).collect::<Vec<_>>());
        assert!(s2.replayed);
        // {4,5} class1 untouched -> preserved
        assert!(s2.preserved_concepts.iter().any(|id| {
            r.stages[0]
                .ontology
                .concepts
                .iter()
                .any(|c| c.id == *id && c.label == 1)
        }));
    }

    #[test]
    fn merge_occurs_when_world_coarsens_and_task_drops_distinction() {
        let mut r = Runner::new(RepairSpec::default());
        let o1 = obs(&[(0, 0), (1, 0), (2, 1), (3, 1)]);
        let t1 = task(&[(0, 0), (1, 0), (2, 1), (3, 1)]);
        let s1 = r.add_stage(&o1, &t1).clone();
        assert_eq!(s1.ontology.concepts.len(), 2);
        let o2 = obs(&[(0, 0), (1, 0), (2, 0), (3, 0)]);
        let t2 = task(&[(0, 0), (1, 0), (2, 0), (3, 0)]);
        let s2 = r.add_stage(&o2, &t2).clone();
        assert_eq!(s2.ontology.concepts.len(), 1);
        assert!(s2.transitions.iter().filter(|t| t.op == RepairOp::Merge).count() >= 1, "expected a merge, got {:?}", s2.transitions.iter().map(|t| t.op).collect::<Vec<_>>());
        assert!(s2.replayed); // current-task replay holds
        // accumulated replay is not required to hold after a deliberate world coarsening
    }

    #[test]
    fn specialize_narrows_a_concept() {
        let mut r = Runner::new(RepairSpec::default());
        let o1 = obs(&[(0, 0), (1, 0), (2, 0), (3, 0), (4, 1), (5, 1)]);
        let t1 = task(&[(0, 0), (1, 0), (2, 0), (3, 0), (4, 1), (5, 1)]);
        let _s1 = r.add_stage(&o1, &t1).clone();
        // token 3 reclassified to a distinct class
        let o2 = obs(&[(0, 0), (1, 0), (2, 0), (3, 5), (4, 1), (5, 1)]);
        let t2 = task(&[(0, 0), (1, 0), (2, 0), (3, 5), (4, 1), (5, 1)]);
        let s2 = r.add_stage(&o2, &t2).clone();
        assert!(s2.transitions.iter().any(|t| t.op == RepairOp::Specialize), "expected specialize: {:?}", s2.transitions.iter().map(|t| t.op).collect::<Vec<_>>());
    }

    #[test]
    fn generalize_widens_a_concept() {
        let mut r = Runner::new(RepairSpec::default());
        let o1 = obs(&[(0, 0), (1, 0), (2, 1), (3, 1)]);
        let t1 = task(&[(0, 0), (1, 0), (2, 1), (3, 1)]);
        let _s1 = r.add_stage(&o1, &t1).clone();
        // tokens 4,5 revealed to be class 1
        let o2 = obs(&[(0, 0), (1, 0), (2, 1), (3, 1), (4, 1), (5, 1)]);
        let t2 = task(&[(0, 0), (1, 0), (2, 1), (3, 1), (4, 1), (5, 1)]);
        let s2 = r.add_stage(&o2, &t2).clone();
        assert!(s2.transitions.iter().any(|t| t.op == RepairOp::Generalize), "expected generalize: {:?}", s2.transitions.iter().map(|t| t.op).collect::<Vec<_>>());
    }

    #[test]
    fn invalidates_a_fragmented_concept() {
        let mut r = Runner::new(RepairSpec::default());
        let o1 = obs(&[(0, 0), (1, 0), (2, 1), (3, 1), (4, 2), (5, 2)]);
        let t1 = task(&[(0, 0), (1, 0), (2, 1), (3, 1), (4, 2), (5, 2)]);
        let s1 = r.add_stage(&o1, &t1).clone();
        assert_eq!(s1.ontology.concepts.len(), 3);
        // world reclassifies both members of {0,1} to brand-new distinct classes
        let o2 = obs(&[(0, 3), (1, 4), (2, 1), (3, 1), (4, 2), (5, 2)]);
        let t2 = task(&[(0, 3), (1, 4), (2, 1), (3, 1), (4, 2), (5, 2)]);
        let s2 = r.add_stage(&o2, &t2).clone();
        assert!(s2.transitions.iter().any(|t| t.op == RepairOp::Remove), "expected remove/invalidate: {:?}", s2.transitions.iter().map(|t| t.op).collect::<Vec<_>>());
        // unaffected concepts {2,3} and {4,5} preserved
        assert_eq!(count_op(&s2, RepairOp::Retain), 2);
    }

    #[test]
    fn structural_replacement_happens_when_patches_exceed_rebuild() {
        let small = structural_decision(100, 140, 0, 20, 30);
        assert!(!small.replaced);
        let many = structural_decision(100, 140, 10, 20, 30);
        assert!(many.replaced);
        assert!(many.patch_cost > many.rebuild_cost);
    }

    #[test]
    fn fixed_task_history_is_fully_replayed_after_revision() {
        // Task only grows; the ontology must keep answering every commitment.
        let mut r = Runner::new(RepairSpec::default());
        let s1 = r
            .add_stage(
                &obs(&[(0, 0), (1, 0), (2, 1), (3, 1)]),
                &task(&[(0, 0), (1, 0), (2, 1), (3, 1)]),
            )
            .clone();
        assert!(s1.replayed && s1.accumulated_replayed);
        let s2 = r
            .add_stage(
                &obs(&[(0, 0), (1, 0), (2, 1), (3, 1), (4, 2), (5, 2), (6, 3), (7, 3)]),
                &task(&[(0, 0), (1, 0), (2, 1), (3, 1), (4, 2), (5, 2), (6, 3), (7, 3)]),
            )
            .clone();
        assert!(s2.replayed && s2.accumulated_replayed);
        assert_eq!(s2.ontology.concepts.len(), 4);
        let s3 = r
            .add_stage(
                &obs(&[(0, 0), (1, 0), (2, 1), (3, 1), (4, 2), (5, 2), (6, 3), (7, 3), (8, 4), (9, 4)]),
                &task(&[(0, 0), (1, 0), (2, 1), (3, 1), (4, 2), (5, 2), (6, 3), (7, 3), (8, 4), (9, 4)]),
            )
            .clone();
        assert!(s3.replayed && s3.accumulated_replayed);
        assert_eq!(s3.ontology.concepts.len(), 5);
        assert_eq!(count_op(&s3, RepairOp::Retain), 4);
        assert_eq!(count_op(&s3, RepairOp::Add), 1);
    }

    #[test]
    fn machine_record_is_deterministic_and_complete() {
        let mut r = Runner::new(RepairSpec::default());
        let s1 = r
            .add_stage(
                &obs(&[(0, 0), (1, 0), (2, 0), (3, 0), (4, 1), (5, 1)]),
                &task(&[(0, 0), (1, 0), (2, 0), (3, 0), (4, 1), (5, 1)]),
            )
            .clone();
        let s2 = r
            .add_stage(
                &obs(&[(0, 0), (1, 0), (2, 2), (3, 2), (4, 1), (5, 1)]),
                &task(&[(0, 0), (1, 0), (2, 2), (3, 2), (4, 1), (5, 1)]),
            )
            .clone();
        let _ = s1;
        let _ = s2;
        let rec = machine_record(&r.stages);
        for field in [
            "experiment=ontology_repair",
            "stages=2",
            "replayed=",
            "total_cost=",
            "deterministic=true",
            "fallback=exact",
            "retain=",
            "split=",
            "add=",
        ] {
            assert!(rec.contains(field), "missing {field}: {rec}");
        }
        assert_eq!(machine_record(&r.stages), rec, "deterministic record");
    }
}
