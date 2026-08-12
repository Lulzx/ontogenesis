//! U5: open-world, provisional recursive-signature ontogenesis.
//!
//! Unlike U4, no observation says that the variant inventory is complete.
//! Every bounded signature containing the visible shapes remains a live
//! hypothesis.  A deterministic description-cost policy chooses a provisional
//! incumbent, and later evidence can invalidate and structurally replace it.

use crate::recursive_signature::{
    self, action_program, enumerate_signatures, semantic_profile, Signature, SignatureClass,
    Variant,
};
use crate::transform;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Derivation {
    pub target: bool,
    pub output: bool,
    pub trace: bool,
    pub ancestors: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShapeEvent {
    pub id: String,
    pub group: String,
    pub epoch: u64,
    pub shape: Variant,
    pub derivation: Derivation,
    pub protected_annotation: i64,
}

#[derive(Clone, Debug)]
pub struct OpenSpec {
    pub signature_max_size: u32,
    pub semantic_class_cap: Option<usize>,
    pub syntax_price: u64,
    pub variant_price: u64,
    pub field_price: u64,
    pub unsupported_price: u64,
    pub hysteresis: u64,
    pub installation_price: u64,
}

pub fn default_spec() -> OpenSpec {
    OpenSpec {
        signature_max_size: 7,
        semantic_class_cap: None,
        syntax_price: 100,
        variant_price: 25,
        field_price: 5,
        unsupported_price: 40,
        hysteresis: 10,
        installation_price: 20,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RankedClass {
    pub class: SignatureClass,
    pub score: u64,
    pub unsupported_variants: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevisionKind {
    Initial,
    Retained,
    Restructured,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenTermination {
    Preferred,
    NoEvidence,
    Exhausted,
    Truncated,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OpenAccounting {
    pub syntax_candidates: u64,
    pub action_evaluations: u64,
    pub semantic_classes: u64,
    pub compatible_classes: u64,
    pub evidence_checks: u64,
    pub score_checks: u64,
    pub replay_checks: u64,
    pub revisions: u64,
    pub truncated: bool,
}

#[derive(Clone, Debug)]
pub struct OpenState {
    pub epoch: u64,
    pub observed: Vec<Variant>,
    pub ranked: Vec<RankedClass>,
    pub incumbent: Option<SignatureClass>,
    pub revision: RevisionKind,
    pub provisional: bool,
    pub logically_identified: bool,
    pub replayed: bool,
    pub accounting: OpenAccounting,
    pub termination: OpenTermination,
}

fn visible<'a>(
    evidence: &'a [ShapeEvent],
    epoch: u64,
    protected: &BTreeSet<String>,
) -> Vec<&'a ShapeEvent> {
    evidence
        .iter()
        .filter(|event| {
            event.epoch <= epoch
                && !event.derivation.target
                && !event.derivation.output
                && !event.derivation.trace
                && event.derivation.ancestors.is_empty()
                && !protected.contains(&event.id)
                && protected.iter().all(|id| event.group != *id)
        })
        .collect()
}

fn classes(candidates: &[Signature]) -> Vec<SignatureClass> {
    let mut grouped = BTreeMap::<Vec<Variant>, Vec<Signature>>::new();
    for signature in candidates {
        grouped
            .entry(semantic_profile(signature))
            .or_default()
            .push(signature.clone());
    }
    grouped
        .into_iter()
        .map(|(profile, mut aliases)| {
            aliases.sort_by_key(|signature| (signature.size(), signature.code()));
            SignatureClass { profile, aliases }
        })
        .collect()
}

fn contains_observations(profile: &[Variant], observed: &[Variant]) -> bool {
    observed.iter().all(|shape| profile.contains(shape))
}

fn class_score(class: &SignatureClass, observed: &[Variant], spec: &OpenSpec) -> (u64, usize) {
    let unsupported = class
        .profile
        .iter()
        .filter(|shape| !observed.contains(shape))
        .count();
    let fields = class
        .profile
        .iter()
        .map(|shape| u64::from(shape.params) + u64::from(shape.recursive))
        .sum::<u64>();
    let score = u64::from(class.aliases[0].size()) * spec.syntax_price
        + class.profile.len() as u64 * spec.variant_price
        + fields * spec.field_price
        + unsupported as u64 * spec.unsupported_price;
    (score, unsupported)
}

fn replay(profile: &[Variant], evidence: &[&ShapeEvent], checks: &mut u64) -> bool {
    evidence.iter().all(|event| {
        *checks += 1;
        profile.contains(&event.shape)
    })
}

pub fn update(
    previous: Option<&OpenState>,
    evidence: &[ShapeEvent],
    epoch: u64,
    protected: &BTreeSet<String>,
    spec: &OpenSpec,
) -> OpenState {
    let candidates = enumerate_signatures(spec.signature_max_size);
    let all_classes = classes(&candidates);
    let visible = visible(evidence, epoch, protected);
    let mut accounting = OpenAccounting {
        syntax_candidates: candidates.len() as u64,
        semantic_classes: all_classes.len() as u64,
        ..Default::default()
    };
    let mut observed = visible.iter().map(|event| event.shape).collect::<Vec<_>>();
    observed.sort();
    observed.dedup();
    if observed.is_empty() {
        return OpenState {
            epoch,
            observed,
            ranked: Vec::new(),
            incumbent: None,
            revision: RevisionKind::Initial,
            provisional: true,
            logically_identified: false,
            replayed: false,
            accounting,
            termination: OpenTermination::NoEvidence,
        };
    }
    let mut ranked = Vec::new();
    for class in all_classes {
        accounting.evidence_checks += observed.len() as u64;
        if !contains_observations(&class.profile, &observed) {
            continue;
        }
        accounting.compatible_classes += 1;
        accounting.action_evaluations += 1;
        let action = action_program(&class.profile);
        if !transform::is_closed(&action) {
            continue;
        }
        let (score, unsupported_variants) = class_score(&class, &observed, spec);
        accounting.score_checks += 1;
        ranked.push(RankedClass {
            class,
            score,
            unsupported_variants,
        });
    }
    ranked.sort_by_key(|item| {
        (
            item.score,
            item.class.aliases[0].size(),
            item.class.profile.clone(),
            item.class.aliases[0].code(),
        )
    });
    if let Some(cap) = spec.semantic_class_cap {
        if ranked.len() > cap {
            ranked.truncate(cap);
            accounting.truncated = true;
        }
    }
    if accounting.truncated {
        return OpenState {
            epoch,
            observed,
            ranked,
            incumbent: None,
            revision: RevisionKind::Initial,
            provisional: true,
            logically_identified: false,
            replayed: false,
            accounting,
            termination: OpenTermination::Truncated,
        };
    }
    let mut chosen = ranked.first().cloned();
    let mut revision = if previous
        .and_then(|state| state.incumbent.as_ref())
        .is_some()
    {
        RevisionKind::Restructured
    } else {
        RevisionKind::Initial
    };
    if let (Some(old), Some(best)) = (
        previous.and_then(|state| state.incumbent.as_ref()),
        chosen.as_ref(),
    ) {
        if let Some(old_ranked) = ranked.iter().find(|item| item.class.profile == old.profile) {
            if old_ranked.score <= best.score.saturating_add(spec.hysteresis) {
                chosen = Some(old_ranked.clone());
                revision = RevisionKind::Retained;
            }
        }
    }
    let incumbent = chosen.map(|item| item.class);
    let replayed = incumbent
        .as_ref()
        .is_some_and(|class| replay(&class.profile, &visible, &mut accounting.replay_checks));
    if revision == RevisionKind::Restructured {
        accounting.revisions = previous.map_or(1, |state| state.accounting.revisions + 1);
    } else {
        accounting.revisions = previous.map_or(0, |state| state.accounting.revisions);
    }
    OpenState {
        epoch,
        observed,
        ranked,
        incumbent,
        revision,
        provisional: true,
        logically_identified: false,
        replayed,
        accounting,
        termination: OpenTermination::Preferred,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuppliedCompleteControl {
    pub matching_classes: usize,
    pub identified: bool,
}

pub fn supplied_complete_control(state: &OpenState) -> SuppliedCompleteControl {
    let matching_classes = state
        .ranked
        .iter()
        .filter(|item| item.class.profile == state.observed)
        .count();
    SuppliedCompleteControl {
        matching_classes,
        identified: matching_classes == 1,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllocationMeasurement {
    pub solved: bool,
    pub proposals: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageEconomics {
    pub learned: AllocationMeasurement,
    pub uniform: AllocationMeasurement,
    pub oracle: AllocationMeasurement,
    pub supplied_complete: AllocationMeasurement,
    pub irrelevant: AllocationMeasurement,
    pub misleading: AllocationMeasurement,
    pub discovery_charge: u64,
    pub reuse_horizon: u64,
    pub net_gain: i128,
}

fn measure_order(
    order: impl IntoIterator<Item = Vec<Variant>>,
    required: &[Variant],
) -> AllocationMeasurement {
    let mut proposals = 0;
    for profile in order {
        proposals += 1;
        if profile == required {
            return AllocationMeasurement {
                solved: true,
                proposals,
            };
        }
    }
    AllocationMeasurement {
        solved: false,
        proposals,
    }
}

pub fn economics(
    state: &OpenState,
    previous: Option<&OpenState>,
    spec: &OpenSpec,
    reuse_horizon: u64,
) -> StageEconomics {
    let incumbent = state.incumbent.as_ref().expect("preferred state");
    let mut lexical = state.ranked.iter().collect::<Vec<_>>();
    lexical.sort_by_key(|item| (item.class.profile.clone(), item.class.aliases[0].code()));
    let learned = measure_order(
        state.ranked.iter().map(|item| item.class.profile.clone()),
        &incumbent.profile,
    );
    let uniform = measure_order(
        lexical.iter().map(|item| item.class.profile.clone()),
        &incumbent.profile,
    );
    let mut irrelevant_order = lexical
        .iter()
        .filter(|item| item.class.profile != incumbent.profile)
        .map(|item| item.class.profile.clone())
        .collect::<Vec<_>>();
    irrelevant_order.push(incumbent.profile.clone());
    let irrelevant = measure_order(irrelevant_order.clone(), &incumbent.profile);
    let mut misleading_order = Vec::new();
    if let Some(old) = previous.and_then(|old| old.incumbent.as_ref()) {
        misleading_order.push(old.profile.clone());
    }
    misleading_order.append(&mut irrelevant_order);
    let misleading = measure_order(misleading_order, &incumbent.profile);
    let discovery_charge = state.accounting.action_evaluations
        + state.accounting.score_checks
        + state.accounting.replay_checks
        + spec.installation_price;
    StageEconomics {
        learned,
        uniform: uniform.clone(),
        oracle: AllocationMeasurement {
            solved: true,
            proposals: 1,
        },
        supplied_complete: AllocationMeasurement {
            solved: true,
            proposals: 1,
        },
        irrelevant,
        misleading,
        discovery_charge,
        reuse_horizon,
        net_gain: (i128::from(uniform.proposals) - 1) * i128::from(reuse_horizon)
            - i128::from(discovery_charge),
    }
}

pub fn sample_stream() -> Vec<ShapeEvent> {
    let event = |id: &str, epoch, recursive| ShapeEvent {
        id: id.into(),
        group: id.into(),
        epoch,
        shape: Variant {
            params: 0,
            recursive,
        },
        derivation: Derivation::default(),
        protected_annotation: 0,
    };
    vec![
        event("seed", 1, 0),
        event("chain", 2, 1),
        event("fork", 3, 2),
    ]
}

#[derive(Clone, Debug)]
pub struct ExperimentReport {
    pub stages: Vec<OpenState>,
    pub economics: Vec<StageEconomics>,
    pub supplied_complete: Vec<SuppliedCompleteControl>,
    pub unary_structure_validated: bool,
}

pub fn run_experiment() -> ExperimentReport {
    let evidence = sample_stream();
    let protected = BTreeSet::new();
    let spec = default_spec();
    let mut stages = Vec::new();
    for epoch in 1..=3 {
        let next = update(stages.last(), &evidence, epoch, &protected, &spec);
        stages.push(next);
    }
    let economics = stages
        .iter()
        .enumerate()
        .map(|(index, state)| {
            economics(
                state,
                index.checked_sub(1).map(|i| &stages[i]),
                &spec,
                10_000,
            )
        })
        .collect();
    let supplied_complete = stages.iter().map(supplied_complete_control).collect();
    let (training, calibration, protected_evidence) = recursive_signature::sample_evidence();
    let protected_ids = protected_evidence
        .iter()
        .map(|item| item.id.clone())
        .collect();
    let observed = stages[1].observed.clone();
    let open_curriculum = recursive_signature::ShapeCurriculum {
        observed: observed.clone(),
        complete: false,
    };
    // This comparison freezes U5's current incumbent before invoking the
    // unchanged U4 executable structure search. Completeness narrows only this
    // validation call; it never feeds back into U5 ranking or revision.
    let validation_curriculum = recursive_signature::ShapeCurriculum {
        observed,
        complete: true,
    };
    let validation = recursive_signature::discover(
        &training,
        &calibration,
        &protected_ids,
        &open_curriculum,
        &validation_curriculum,
        &recursive_signature::default_spec(),
    );
    let unary_structure_validated = validation.structure.as_ref().is_some_and(|structure| {
        structure.signature_class.profile == stages[1].incumbent.as_ref().unwrap().profile
            && protected_evidence.iter().all(|evidence| {
                let mut checks = 0;
                recursive_signature::commutes(structure, evidence, &mut checks)
                    && recursive_signature::bounded_uniqueness(
                        structure,
                        evidence,
                        &recursive_signature::default_spec(),
                    )
                    .unique
            })
    });
    ExperimentReport {
        stages,
        economics,
        supplied_complete,
        unary_structure_validated,
    }
}

fn profile_code(profile: &[Variant]) -> String {
    profile
        .iter()
        .map(|shape| format!("{}x{}", shape.params, shape.recursive))
        .collect::<Vec<_>>()
        .join("+")
}

pub fn machine_record(report: &ExperimentReport) -> String {
    let profiles = report
        .stages
        .iter()
        .map(|stage| profile_code(&stage.incumbent.as_ref().unwrap().profile))
        .collect::<Vec<_>>()
        .join(";");
    let compatible = report
        .stages
        .iter()
        .map(|stage| stage.accounting.compatible_classes.to_string())
        .collect::<Vec<_>>()
        .join(";");
    let revisions = report
        .stages
        .iter()
        .filter(|stage| stage.revision == RevisionKind::Restructured)
        .count();
    let final_stage = report.stages.last().unwrap();
    let final_economics = report.economics.last().unwrap();
    format!(
        "record,experiment=u5,stages=3,preferred_profiles={},compatible_classes={},provisional=true,logically_identified=false,revisions={},replay_complete={},unary_structure_validated={},final_aliases={},learned_proposals={},uniform_proposals={},oracle_proposals={},supplied_complete_proposals={},irrelevant_proposals={},misleading_proposals={},discovery_charge={},reuse_horizon={},net_gain={},universal_fallback=exact,termination={:?}",
        profiles,
        compatible,
        revisions,
        final_stage.replayed,
        report.unary_structure_validated,
        final_stage.incumbent.as_ref().unwrap().aliases.len(),
        final_economics.learned.proposals,
        final_economics.uniform.proposals,
        final_economics.oracle.proposals,
        final_economics.supplied_complete.proposals,
        final_economics.irrelevant.proposals,
        final_economics.misleading.proposals,
        final_economics.discovery_charge,
        final_economics.reuse_horizon,
        final_economics.net_gain,
        final_stage.termination,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universal::{Dovetail, InterleavedDovetail, ResourceLane};

    fn profile(recursive: &[u8]) -> Vec<Variant> {
        recursive
            .iter()
            .map(|recursive| Variant {
                params: 0,
                recursive: *recursive,
            })
            .collect()
    }

    #[test]
    fn open_world_is_provisional_and_revises_by_replaying_history() {
        let report = run_experiment();
        assert_eq!(report.stages.len(), 3);
        assert_eq!(
            report.stages[0].incumbent.as_ref().unwrap().profile,
            profile(&[0])
        );
        assert_eq!(
            report.stages[1].incumbent.as_ref().unwrap().profile,
            profile(&[0, 1])
        );
        assert_eq!(
            report.stages[2].incumbent.as_ref().unwrap().profile,
            profile(&[0, 1, 2])
        );
        assert!(report
            .stages
            .iter()
            .all(|stage| stage.provisional && !stage.logically_identified && stage.replayed));
        assert!(report.stages[0].accounting.compatible_classes > 1);
        assert!(report.stages[1].accounting.compatible_classes > 1);
        assert_eq!(report.stages[1].revision, RevisionKind::Restructured);
        assert_eq!(report.stages[2].revision, RevisionKind::Restructured);
        assert_eq!(report.stages[2].accounting.revisions, 2);
    }

    #[test]
    fn delayed_constructor_falsifies_incumbent_not_just_adds_an_atom() {
        let evidence = sample_stream();
        let spec = default_spec();
        let protected = BTreeSet::new();
        let unary = update(None, &evidence, 2, &protected, &spec);
        let binary = update(Some(&unary), &evidence, 3, &protected, &spec);
        let old = unary.incumbent.as_ref().unwrap();
        let new = binary.incumbent.as_ref().unwrap();
        assert!(!old.profile.contains(&Variant {
            params: 0,
            recursive: 2
        }));
        assert!(new.profile.contains(&Variant {
            params: 0,
            recursive: 2
        }));
        assert_ne!(old.profile, new.profile);
        assert!(!binary
            .ranked
            .iter()
            .any(|item| item.class.profile == old.profile));
        assert_ne!(action_program(&old.profile), action_program(&new.profile));
        assert_eq!(binary.revision, RevisionKind::Restructured);
        assert!(binary.replayed && binary.accounting.replay_checks == 3);
    }

    #[test]
    fn aliases_are_semantic_classes_and_supplied_completeness_is_only_a_control() {
        let report = run_experiment();
        assert!(report.unary_structure_validated);
        for (stage, control) in report.stages.iter().zip(&report.supplied_complete) {
            assert!(stage.incumbent.as_ref().unwrap().aliases.len() >= 1);
            assert!(control.identified);
            assert!(!stage.logically_identified);
            assert!(stage.ranked.len() > control.matching_classes);
        }
    }

    #[test]
    fn declared_score_calibration_preserves_the_experience_driven_sequence() {
        let evidence = sample_stream();
        let protected = BTreeSet::new();
        for syntax_price in [50, 100, 200] {
            for unsupported_price in [20, 40, 80] {
                let mut spec = default_spec();
                spec.syntax_price = syntax_price;
                spec.unsupported_price = unsupported_price;
                let mut old = None;
                for (epoch, expected) in [
                    (1, profile(&[0])),
                    (2, profile(&[0, 1])),
                    (3, profile(&[0, 1, 2])),
                ] {
                    let state = update(old.as_ref(), &evidence, epoch, &protected, &spec);
                    assert_eq!(state.incumbent.as_ref().unwrap().profile, expected);
                    old = Some(state);
                }
            }
        }
    }

    #[test]
    fn hysteresis_retains_compatible_incumbent_but_never_invalid_one() {
        let evidence = sample_stream();
        let mut spec = default_spec();
        spec.hysteresis = u64::MAX;
        let protected = BTreeSet::new();
        let first = update(None, &evidence, 2, &protected, &spec);
        let repeated = update(Some(&first), &evidence, 2, &protected, &spec);
        assert_eq!(repeated.revision, RevisionKind::Retained);
        assert_eq!(
            first.incumbent.as_ref().unwrap().profile,
            repeated.incumbent.as_ref().unwrap().profile
        );
        let contradicted = update(Some(&repeated), &evidence, 3, &protected, &spec);
        assert_eq!(contradicted.revision, RevisionKind::Restructured);
        assert_ne!(
            repeated.incumbent.as_ref().unwrap().profile,
            contradicted.incumbent.as_ref().unwrap().profile
        );
    }

    #[test]
    fn recovers_from_a_wrong_but_previously_compatible_preference() {
        let evidence = sample_stream();
        let spec = default_spec();
        let protected = BTreeSet::new();
        let mut wrong = update(None, &evidence, 1, &protected, &spec);
        wrong.incumbent = wrong
            .ranked
            .iter()
            .find(|item| item.class.profile == profile(&[0, 2]))
            .map(|item| item.class.clone());
        let recovered = update(Some(&wrong), &evidence, 2, &protected, &spec);
        assert_eq!(
            recovered.incumbent.as_ref().unwrap().profile,
            profile(&[0, 1])
        );
        assert_eq!(recovered.revision, RevisionKind::Restructured);
        assert!(recovered.replayed);
    }

    #[test]
    fn protected_postfreeze_and_derived_evidence_cannot_change_ranking_or_costs() {
        let evidence = sample_stream();
        let spec = default_spec();
        let protected = BTreeSet::from(["held".to_string()]);
        let clean = update(None, &evidence, 2, &protected, &spec);
        let mut poisoned = evidence.clone();
        let mut push = |id: &str, mutate: fn(&mut ShapeEvent)| {
            let mut event = ShapeEvent {
                id: id.into(),
                group: id.into(),
                epoch: 2,
                shape: Variant {
                    params: 9,
                    recursive: 9,
                },
                derivation: Derivation::default(),
                protected_annotation: 0,
            };
            mutate(&mut event);
            poisoned.push(event);
        };
        push("target", |e| e.derivation.target = true);
        push("output", |e| e.derivation.output = true);
        push("trace", |e| e.derivation.trace = true);
        push("ancestry", |e| {
            e.derivation.ancestors.insert("held".into());
        });
        push("late", |e| e.epoch = 3);
        push("held", |_| {});
        let contaminated = update(None, &poisoned, 2, &protected, &spec);
        assert_eq!(clean.observed, contaminated.observed);
        assert_eq!(clean.ranked, contaminated.ranked);
        assert_eq!(clean.accounting, contaminated.accounting);
        let mut annotated = evidence.clone();
        for event in &mut annotated {
            event.protected_annotation = i64::MAX;
        }
        let mutation = update(None, &annotated, 2, &protected, &spec);
        assert_eq!(clean.ranked, mutation.ranked);
        assert_eq!(clean.accounting, mutation.accounting);
    }

    #[test]
    fn truncation_blocks_preference_and_evidence_order_has_declared_semantics() {
        let evidence = sample_stream();
        let protected = BTreeSet::new();
        let mut spec = default_spec();
        spec.semantic_class_cap = Some(1);
        let truncated = update(None, &evidence, 2, &protected, &spec);
        assert_eq!(truncated.termination, OpenTermination::Truncated);
        assert!(truncated.incumbent.is_none() && truncated.accounting.truncated);

        let spec = default_spec();
        let mut shuffled = evidence.clone();
        shuffled.reverse();
        let a = update(None, &evidence, 3, &protected, &spec);
        let b = update(None, &shuffled, 3, &protected, &spec);
        assert_eq!(a.observed, b.observed);
        assert_eq!(a.ranked, b.ranked);
        // Time matters through epochs: the same stream at t=2 excludes t=3.
        let earlier = update(None, &shuffled, 2, &protected, &spec);
        assert_ne!(
            a.incumbent.as_ref().unwrap().profile,
            earlier.incumbent.as_ref().unwrap().profile
        );

        let candidates = enumerate_signatures(spec.signature_max_size);
        let mut reversed = candidates.clone();
        reversed.reverse();
        assert_eq!(classes(&candidates), classes(&reversed));
    }

    #[test]
    fn economics_controls_and_universal_projection_are_explicit() {
        let report = run_experiment();
        let final_cost = report.economics.last().unwrap();
        assert_eq!(final_cost.learned.proposals, final_cost.oracle.proposals);
        assert_eq!(final_cost.supplied_complete.proposals, 1);
        assert!(final_cost.learned.solved && final_cost.uniform.solved);
        assert!(final_cost.irrelevant.proposals > final_cost.learned.proposals);
        assert!(final_cost.misleading.proposals > final_cost.learned.proposals);
        assert!(report.economics[1].misleading.proposals > report.economics[1].learned.proposals);
        let record = machine_record(&report);
        for field in [
            "preferred_profiles=",
            "compatible_classes=",
            "provisional=true",
            "logically_identified=false",
            "revisions=",
            "replay_complete=true",
            "unary_structure_validated=true",
            "learned_proposals=",
            "uniform_proposals=",
            "oracle_proposals=",
            "supplied_complete_proposals=",
            "irrelevant_proposals=",
            "misleading_proposals=",
            "discovery_charge=",
            "reuse_horizon=",
            "net_gain=",
            "universal_fallback=exact",
            "termination=",
        ] {
            assert!(record.contains(field), "missing {field}");
        }

        let mut schedule = InterleavedDovetail::new((0..128).map(|i| ((i % 9 + 1) as u32, i + 1)));
        let mut projection = Vec::new();
        while projection.len() < 256 {
            let point = schedule.next_labeled().unwrap();
            if point.lane == ResourceLane::Universal {
                projection.push((point.syntax_size, point.evaluation_fuel));
            }
        }
        assert_eq!(
            projection,
            Dovetail::default().take(256).collect::<Vec<_>>()
        );
    }
}
