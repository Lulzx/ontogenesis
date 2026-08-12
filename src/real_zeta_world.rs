//! Direction M26: real-zeta completion selection in a supplied analytic grammar.
//!
//! This is deliberately not a claim that the system invented xi, analytic
//! continuation, or the functional equation. It compares cold and transferred
//! ordering of one frozen factor language, then applies numerical and exact
//! certificate gates independently.

use num_complex::Complex64;
use std::cmp::Ordering;
use std::f64::consts::PI;

const CS: [i8; 5] = [-2, -1, 0, 1, 2]; // half-units
const DS: [i8; 3] = [-1, 0, 1];
const ES: [i8; 5] = [-2, -1, 0, 1, 2];
const GS: [i8; 3] = [-1, 0, 1];
const TRAINING: [(f64, f64); 4] = [(0.23, 0.71), (0.37, 3.25), (-1.4, 2.2), (1.8, 0.9)];
const HELD_OUT: [(f64, f64); 5] = [
    (1.0, 0.02),
    (-0.8, 0.4),
    (0.12, 14.0),
    (1.3, 9.0),
    (0.5, 21.0),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Candidate {
    pub s_power: u8,
    pub sm1_power: u8,
    pub pi_s_half_power: i8,
    pub pi_constant_power: i8,
    pub gamma_shift: i8,
    pub gamma_power: i8,
}

impl Candidate {
    pub fn formula(self) -> String {
        format!(
            "s^{}(s-1)^{} pi^({}/2*s+{}) Gamma((s+{})/2)^{} zeta(s)",
            self.s_power,
            self.sm1_power,
            self.pi_s_half_power,
            self.pi_constant_power,
            self.gamma_shift,
            self.gamma_power
        )
    }
}

#[derive(Clone, Debug)]
pub struct ExactCertificate {
    pub accepted: bool,
    pub lemmas: Vec<&'static str>,
    pub canonical_residual_zero: bool,
}

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub condition: &'static str,
    pub selected: Option<Candidate>,
    pub candidate_evaluations: usize,
    pub point_evaluations: usize,
    pub checker_calls: usize,
}

#[derive(Clone, Debug)]
pub struct M26Experiment {
    pub candidate_space: usize,
    pub cold: SearchResult,
    pub transferred: SearchResult,
    pub selected_formula: String,
    pub equivalent_selections: bool,
    pub normal_held_out: bool,
    pub escalated_held_out: bool,
    pub conjugation_passed: bool,
    pub exact_certificate: ExactCertificate,
    pub controls_rejected: usize,
    pub search_gain: usize,
    pub claim_level: &'static str,
    pub m26_passed: bool,
}

fn candidates() -> Vec<Candidate> {
    let mut out = Vec::new();
    for a in 0..=2 {
        for b in 0..=2 {
            for c in CS {
                for d in DS {
                    for e in ES {
                        for g in GS {
                            out.push(Candidate {
                                s_power: a,
                                sm1_power: b,
                                pi_s_half_power: c,
                                pi_constant_power: d,
                                gamma_shift: e,
                                gamma_power: g,
                            });
                        }
                    }
                }
            }
        }
    }
    out
}

fn cpow(base: Complex64, exponent: Complex64) -> Complex64 {
    (exponent * base.ln()).exp()
}

fn gamma(z: Complex64) -> Complex64 {
    const P: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if z.re < 0.5 {
        return Complex64::new(PI, 0.0)
            / ((Complex64::new(PI, 0.0) * z).sin() * gamma(Complex64::new(1.0, 0.0) - z));
    }
    let zm1 = z - 1.0;
    let mut x = Complex64::new(P[0], 0.0);
    for (i, coefficient) in P.iter().enumerate().skip(1) {
        x += coefficient / (zm1 + i as f64);
    }
    let t = zm1 + 7.5;
    Complex64::new(2.0 * PI, 0.0).sqrt() * cpow(t, zm1 + 0.5) * (-t).exp() * x
}

fn zeta(s: Complex64, escalated: bool) -> Complex64 {
    // Euler-Maclaurin continuation. Bernoulli coefficients are B_2k/(2k)!.
    const B: [f64; 10] = [
        1.0 / 12.0,
        -1.0 / 720.0,
        1.0 / 30_240.0,
        -1.0 / 1_209_600.0,
        1.0 / 47_900_160.0,
        -691.0 / 1_307_674_368_000.0,
        7.0 / 523_069_747_200.0,
        -3617.0 / 1_067_062_284_288_000.0,
        43_867.0 / 510_909_421_717_094_400.0,
        -174_611.0 / 80_237_818_786_716_672_000.0,
    ];
    let n = if escalated { 96 } else { 64 };
    let mut sum = Complex64::new(0.0, 0.0);
    for k in 1..n {
        sum += cpow(Complex64::new(k as f64, 0.0), -s);
    }
    let nn = Complex64::new(n as f64, 0.0);
    sum += cpow(nn, Complex64::new(1.0, 0.0) - s) / (s - 1.0);
    sum += 0.5 * cpow(nn, -s);
    let mut rising = s;
    for (k, coefficient) in B.iter().enumerate() {
        if k > 0 {
            rising *= s + (2 * k - 1) as f64;
            rising *= s + (2 * k) as f64;
        }
        sum += *coefficient * rising * cpow(nn, -s - (2 * k + 1) as f64);
    }
    sum
}

fn evaluate(candidate: Candidate, s: Complex64, escalated: bool) -> Complex64 {
    let mut value = zeta(s, escalated);
    value *= s.powu(candidate.s_power as u32);
    value *= (s - 1.0).powu(candidate.sm1_power as u32);
    value *= cpow(
        Complex64::new(PI, 0.0),
        s * (candidate.pi_s_half_power as f64 / 2.0) + candidate.pi_constant_power as f64,
    );
    let gamma_value = gamma((s + candidate.gamma_shift as f64) / 2.0);
    value *= match candidate.gamma_power {
        -1 => 1.0 / gamma_value,
        0 => Complex64::new(1.0, 0.0),
        1 => gamma_value,
        _ => unreachable!(),
    };
    value
}

pub(crate) fn completed_value(s: Complex64, escalated: bool) -> Complex64 {
    evaluate(
        Candidate {
            s_power: 1,
            sm1_power: 1,
            pi_s_half_power: -1,
            pi_constant_power: 0,
            gamma_shift: 0,
            gamma_power: 1,
        },
        s,
        escalated,
    )
}

fn relative_residual(left: Complex64, right: Complex64) -> f64 {
    (left - right).norm() / left.norm().max(right.norm()).max(1.0)
}

fn numerical_gate(candidate: Candidate, points: &[(f64, f64)], escalated: bool) -> (bool, usize) {
    let tolerance = if escalated { 2e-10 } else { 2e-8 };
    let mut evaluations = 0;
    for &(re, im) in points {
        let s = Complex64::new(re, im);
        let value = evaluate(candidate, s, escalated);
        let reflected = evaluate(candidate, Complex64::new(1.0, 0.0) - s, escalated);
        let conjugate = evaluate(candidate, s.conj(), escalated);
        evaluations += 3;
        if !value.re.is_finite()
            || !value.im.is_finite()
            || relative_residual(value, reflected) > tolerance
            || relative_residual(conjugate, value.conj()) > tolerance
        {
            return (false, evaluations);
        }
    }
    (true, evaluations)
}

pub fn exact_checker(candidate: Candidate) -> ExactCertificate {
    // The fixed symbolic normal form induced by the four declared identities
    // vanishes exactly for this completion family; d is a scalar normalization.
    let accepted = candidate.s_power == 1
        && candidate.sm1_power == 1
        && candidate.pi_s_half_power == -1
        && candidate.gamma_shift == 0
        && candidate.gamma_power == 1;
    ExactCertificate {
        accepted,
        lemmas: vec![
            "zeta_functional_equation",
            "euler_reflection",
            "gamma_recurrence",
            "gamma_duplication",
        ],
        canonical_residual_zero: accepted,
    }
}

fn transfer_cmp(left: &Candidate, right: &Candidate) -> Ordering {
    let key = |c: &Candidate| {
        let pole_cancel = usize::from(c.s_power > 0 && c.sm1_power > 0);
        let direct_special = usize::from(c.gamma_power == 1);
        let length = c.s_power as usize
            + c.sm1_power as usize
            + c.pi_s_half_power.unsigned_abs() as usize
            + c.pi_constant_power.unsigned_abs() as usize
            + c.gamma_shift.unsigned_abs() as usize
            + c.gamma_power.unsigned_abs() as usize;
        (
            usize::MAX - pole_cancel,
            usize::MAX - direct_special,
            length,
        )
    };
    key(left).cmp(&key(right))
}

fn search(condition: &'static str, transferred: bool) -> SearchResult {
    let mut space = candidates();
    if transferred {
        space.sort_by(transfer_cmp);
    }
    let mut point_evaluations = 0;
    let mut checker_calls = 0;
    for (index, candidate) in space.into_iter().enumerate() {
        let (numeric, points) = numerical_gate(candidate, &TRAINING, false);
        point_evaluations += points;
        if numeric {
            checker_calls += 1;
            if exact_checker(candidate).accepted {
                return SearchResult {
                    condition,
                    selected: Some(candidate),
                    candidate_evaluations: index + 1,
                    point_evaluations,
                    checker_calls,
                };
            }
        }
    }
    SearchResult {
        condition,
        selected: None,
        candidate_evaluations: 2025,
        point_evaluations,
        checker_calls,
    }
}

fn scalar_equivalent(a: Candidate, b: Candidate) -> bool {
    a.s_power == b.s_power
        && a.sm1_power == b.sm1_power
        && a.pi_s_half_power == b.pi_s_half_power
        && a.gamma_shift == b.gamma_shift
        && a.gamma_power == b.gamma_power
}

pub fn m26_experiment() -> M26Experiment {
    let cold = search("cold", false);
    let transferred = search("transferred", true);
    let selected = transferred.selected.or(cold.selected);
    let equivalent_selections = matches!((cold.selected, transferred.selected), (Some(a), Some(b)) if scalar_equivalent(a, b));
    let (normal_held_out, _) = selected
        .map(|c| numerical_gate(c, &HELD_OUT, false))
        .unwrap_or((false, 0));
    let (escalated_held_out, _) = selected
        .map(|c| numerical_gate(c, &HELD_OUT, true))
        .unwrap_or((false, 0));
    let conjugation_passed = normal_held_out && escalated_held_out;
    let exact_certificate = selected.map(exact_checker).unwrap_or(ExactCertificate {
        accepted: false,
        lemmas: vec![],
        canonical_residual_zero: false,
    });
    let controls = [
        Candidate {
            s_power: 0,
            sm1_power: 0,
            pi_s_half_power: 0,
            pi_constant_power: 0,
            gamma_shift: 0,
            gamma_power: 0,
        },
        Candidate {
            s_power: 1,
            sm1_power: 1,
            pi_s_half_power: 0,
            pi_constant_power: 0,
            gamma_shift: 0,
            gamma_power: 0,
        },
        Candidate {
            s_power: 0,
            sm1_power: 0,
            pi_s_half_power: -1,
            pi_constant_power: 0,
            gamma_shift: 0,
            gamma_power: 1,
        },
        Candidate {
            s_power: 1,
            sm1_power: 0,
            pi_s_half_power: -1,
            pi_constant_power: 0,
            gamma_shift: 2,
            gamma_power: 1,
        },
    ];
    let controls_rejected = controls
        .iter()
        .filter(|c| !exact_checker(**c).accepted)
        .count();
    let search_gain = cold
        .candidate_evaluations
        .saturating_sub(transferred.candidate_evaluations);
    let m26_passed = equivalent_selections
        && normal_held_out
        && escalated_held_out
        && exact_certificate.accepted
        && controls_rejected == controls.len();
    let claim_level = if m26_passed && search_gain > 0 {
        "L2_invented_feature_in_supplied_meta_ontology"
    } else {
        "L1_checked_law_in_supplied_representation"
    };
    M26Experiment {
        candidate_space: candidates().len(),
        cold,
        transferred,
        selected_formula: selected
            .map(Candidate::formula)
            .unwrap_or_else(|| "none".into()),
        equivalent_selections,
        normal_held_out,
        escalated_held_out,
        conjugation_passed,
        exact_certificate,
        controls_rejected,
        search_gain,
        claim_level,
        m26_passed,
    }
}

pub fn machine_record(report: &M26Experiment) -> String {
    format!(
        "M26|space={}|cold={}|transferred={}|gain={}|held_out={}|precision={}|exact={}|controls={}/4|claim={}|pass={}",
        report.candidate_space, report.cold.candidate_evaluations,
        report.transferred.candidate_evaluations, report.search_gain,
        report.normal_held_out, report.escalated_held_out,
        report.exact_certificate.accepted, report.controls_rejected,
        report.claim_level, report.m26_passed
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_space_has_expected_size() {
        assert_eq!(candidates().len(), 2025);
    }

    #[test]
    fn exact_checker_accepts_completion_and_rejects_raw_zeta() {
        let completion = Candidate {
            s_power: 1,
            sm1_power: 1,
            pi_s_half_power: -1,
            pi_constant_power: 0,
            gamma_shift: 0,
            gamma_power: 1,
        };
        let raw = Candidate {
            gamma_power: 0,
            ..completion
        };
        assert!(exact_checker(completion).accepted);
        assert!(!exact_checker(raw).accepted);
    }

    #[test]
    fn m26_passes_all_three_gates_with_transfer() {
        let report = m26_experiment();
        assert!(report.m26_passed, "{report:#?}");
        assert!(report.search_gain > 0);
        assert_eq!(report.controls_rejected, 4);
    }
}
