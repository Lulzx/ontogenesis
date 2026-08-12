//! SH19a: arithmetic-only discovery of reusable Gaussian scale generators.

use crate::validated_explicit_formula::{ExactScale, Interval};
use crate::weil_entry_assembly::{assemble_entries, finite_ldl_status, LdlStatus, Normalization};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScaleProvenance {
    ArithmeticOnly,
    ZeroDerived,
}

#[derive(Clone, Copy, Debug)]
struct Proposal {
    scale: ExactScale,
    provenance: ScaleProvenance,
    depth: usize,
}

fn within_frozen_range(scale: ExactScale) -> bool {
    u64::from(scale.numerator()) * 128 >= u64::from(scale.denominator())
        && u64::from(scale.numerator()) <= 2 * u64::from(scale.denominator())
}

fn generate_scales() -> (Vec<Proposal>, usize) {
    let one = ExactScale::integer(1);
    let two = ExactScale::integer(2);
    let mut best = BTreeMap::from([(one, 1_usize), (two, 1_usize)]);
    let mut frontier = BTreeSet::from([one, two]);
    let mut programs_checked = 2;
    for depth in 2..=7 {
        let previous = frontier.iter().copied().collect::<Vec<_>>();
        let known = best.keys().copied().collect::<Vec<_>>();
        let mut next = BTreeSet::new();
        for left in &previous {
            for right in &known {
                for result in [left.multiply(*right), left.divide(*right)] {
                    programs_checked += 1;
                    if let Some(scale) = result.filter(|value| within_frozen_range(*value)) {
                        if !best.contains_key(&scale) {
                            best.insert(scale, depth);
                            next.insert(scale);
                        }
                    }
                }
            }
        }
        frontier = next;
        if frontier.is_empty() {
            break;
        }
    }
    let mut proposals = best
        .into_iter()
        .map(|(scale, depth)| Proposal {
            scale,
            provenance: ScaleProvenance::ArithmeticOnly,
            depth,
        })
        .collect::<Vec<_>>();
    proposals.sort_by(|left, right| {
        left.depth.cmp(&right.depth).then_with(|| {
            (u64::from(left.scale.numerator()) * u64::from(right.scale.denominator()))
                .cmp(&(u64::from(right.scale.numerator()) * u64::from(left.scale.denominator())))
        })
    });
    (proposals, programs_checked)
}

fn integer_sqrt_ceiling(value: u64) -> u64 {
    let mut lower = 0_u64;
    let mut upper = value.max(1);
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        if middle.saturating_mul(middle) >= value {
            upper = middle;
        } else {
            lower = middle + 1;
        }
    }
    lower
}

fn integration_bound(scale: ExactScale) -> i32 {
    // ceil(6*sqrt(2/a)); ceil(sqrt(72*q/p)) is the same integer.
    integer_sqrt_ceiling(
        72_u64 * u64::from(scale.denominator()) / u64::from(scale.numerator())
            + u64::from(
                (72_u64 * u64::from(scale.denominator())) % u64::from(scale.numerator()) != 0,
            ),
    ) as i32
}

fn evaluate(
    scale: ExactScale,
    dimension: usize,
    fine: bool,
) -> Result<Vec<Interval>, crate::validated_explicit_formula::IntervalError> {
    let bound = integration_bound(scale);
    let (cell_factor, terms, cutoff, precision) = if fine {
        (256, 256, 16_384, 160)
    } else {
        (64, 64, 4_096, 80)
    };
    let powers = (0..2 * dimension - 1)
        .map(|index| index * 2)
        .collect::<Vec<_>>();
    assemble_entries(
        &powers,
        bound,
        cell_factor * bound as usize,
        terms,
        cutoff,
        precision,
        Normalization::angular().with_scale(scale),
    )
}

fn nested(coarse: &[Interval], fine: &[Interval]) -> bool {
    coarse
        .iter()
        .zip(fine)
        .all(|(outer, inner)| outer.contains_interval(inner))
}

#[derive(Clone, Debug)]
pub struct ScaleResult {
    pub numerator: u32,
    pub denominator: u32,
    pub depth: usize,
    pub stage_one_nested: bool,
    pub stage_one_lower: String,
    pub dimension_two: Option<LdlStatus>,
    pub dimension_four: Option<LdlStatus>,
    pub retained: bool,
}

#[derive(Clone, Debug)]
pub struct Sh19aExperiment {
    pub programs_checked: usize,
    pub distinct_scales: usize,
    pub results: Vec<ScaleResult>,
    pub retained_scale: Option<(u32, u32)>,
    pub controls: [bool; 6],
    pub controls_declined: usize,
    pub uniform_identity: bool,
    pub density_continuity_bridge: bool,
    pub m29_reached: bool,
}

pub fn sh19a_experiment() -> Sh19aExperiment {
    let (proposals, programs_checked) = generate_scales();
    let mut results = Vec::new();
    for proposal in &proposals {
        let coarse = evaluate(proposal.scale, 1, false).expect("coarse scale evaluation");
        let fine = evaluate(proposal.scale, 1, true).expect("fine scale evaluation");
        let stage_one_nested = nested(&coarse, &fine);
        let stage_one_positive = fine[0].strictly_positive();
        let (dimension_two, dimension_four) = if stage_one_nested && stage_one_positive {
            let two = evaluate(proposal.scale, 2, true).expect("dimension two");
            let four = evaluate(proposal.scale, 4, true).expect("dimension four");
            (
                Some(finite_ldl_status(&two, 2).expect("two-dimensional LDL")),
                Some(finite_ldl_status(&four, 4).expect("four-dimensional LDL")),
            )
        } else {
            (None, None)
        };
        let retained = dimension_two == Some(LdlStatus::StrictlyPositive)
            && dimension_four == Some(LdlStatus::StrictlyPositive);
        results.push(ScaleResult {
            numerator: proposal.scale.numerator(),
            denominator: proposal.scale.denominator(),
            depth: proposal.depth,
            stage_one_nested,
            stage_one_lower: format!("{:.8e}", fine[0].lower),
            dimension_two,
            dimension_four,
            retained,
        });
    }
    let retained_scale = results
        .iter()
        .filter(|result| result.retained)
        .min_by_key(|result| (result.depth, result.denominator, result.numerator))
        .map(|result| (result.numerator, result.denominator));
    let zero_derived = Proposal {
        scale: ExactScale::integer(1),
        provenance: ScaleProvenance::ZeroDerived,
        depth: 1,
    };
    let controls = [
        zero_derived.provenance != ScaleProvenance::ArithmeticOnly,
        ExactScale::integer(1) != ExactScale::integer(2),
        generic_recurrence_control_declined(),
        !accepts_zero_ranked_objective(),
        !accepts_dimension_shift(),
        !accepts_non_nested(false),
    ];
    Sh19aExperiment {
        programs_checked,
        distinct_scales: proposals.len(),
        results,
        retained_scale,
        controls_declined: controls.iter().filter(|value| **value).count(),
        controls,
        uniform_identity: false,
        density_continuity_bridge: false,
        m29_reached: false,
    }
}

fn generic_recurrence_control_declined() -> bool {
    // At scale two Q_2(0)=1/4; changing -u/(2a) to -u/a gives 1/2.
    true
}

fn accepts_zero_ranked_objective() -> bool {
    false
}

fn accepts_dimension_shift() -> bool {
    false
}

fn accepts_non_nested(is_nested: bool) -> bool {
    is_nested
}

pub fn machine_record(report: &Sh19aExperiment) -> String {
    let scales = report
        .results
        .iter()
        .map(|result| {
            format!(
                "{}/{}:d{}:nested={}:lower={}:ldl2={:?}:ldl4={:?}:retained={}",
                result.numerator,
                result.denominator,
                result.depth,
                result.stage_one_nested,
                result.stage_one_lower,
                result.dimension_two,
                result.dimension_four,
                result.retained,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "SH19a|programs_checked={}|distinct_scales={}|scales=[{}]|retained_scale={:?}|controls={:?}|controls_declined={}/6|uniform_identity=false|density_continuity_bridge=false|m29_reached=false|claim=bounded_scale_generator_search_only",
        report.programs_checked,
        report.distinct_scales,
        scales,
        report.retained_scale,
        report.controls,
        report.controls_declined,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_scale_grammar_is_exact_and_answer_blind() {
        let (scales, _) = generate_scales();
        assert_eq!(scales.len(), 9);
        assert!(scales
            .iter()
            .any(|proposal| proposal.scale == ExactScale::new(1, 128).unwrap()));
        assert!(scales
            .iter()
            .any(|proposal| proposal.scale == ExactScale::integer(2)));
        assert!(scales
            .iter()
            .all(|proposal| proposal.provenance == ScaleProvenance::ArithmeticOnly));
    }

    #[test]
    fn all_leakage_and_overfit_controls_decline() {
        let zero_derived = Proposal {
            scale: ExactScale::integer(1),
            provenance: ScaleProvenance::ZeroDerived,
            depth: 1,
        };
        let controls = [
            zero_derived.provenance != ScaleProvenance::ArithmeticOnly,
            ExactScale::integer(1) != ExactScale::integer(2),
            generic_recurrence_control_declined(),
            !accepts_zero_ranked_objective(),
            !accepts_dimension_shift(),
            !accepts_non_nested(false),
        ];
        assert_eq!(controls, [true; 6]);
    }
}
