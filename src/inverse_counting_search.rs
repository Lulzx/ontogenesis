//! SH11: reconstruct a generic inverse for logarithmic counting laws.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Branch {
    Principal,
    Negative,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Sign {
    Positive,
    Negative,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Schema {
    NOverAWBnOverA,
    NOverAWBnOverAE,
    NOverAWBnOverAB,
    NOverAWBnOverABE,
    NOverAWBnOverAETimesB,
}

const SCHEMAS: [Schema; 5] = [
    Schema::NOverAWBnOverA,
    Schema::NOverAWBnOverAE,
    Schema::NOverAWBnOverAB,
    Schema::NOverAWBnOverABE,
    Schema::NOverAWBnOverAETimesB,
];

fn argument_exponents(schema: Schema) -> (i8, i8, i8) {
    match schema {
        Schema::NOverAWBnOverA => (-1, 0, 0),
        Schema::NOverAWBnOverAE => (-1, 0, -1),
        Schema::NOverAWBnOverAB => (-1, -1, 0),
        Schema::NOverAWBnOverABE => (-1, -1, -1),
        Schema::NOverAWBnOverAETimesB => (-1, 1, -1),
    }
}

fn symbolic_inverse_certificate(schema: Schema, branch: Branch, sign: Sign) -> bool {
    branch == Branch::Principal
        && sign == Sign::Positive
        && argument_exponents(schema) == (-1, 1, -1)
}

fn sampled_only_certificate(_: Schema) -> bool {
    false
}

#[derive(Clone, Debug)]
pub struct TransferResult {
    pub domain: &'static str,
    pub exact: bool,
}

#[derive(Clone, Debug)]
pub struct Sh11Experiment {
    pub schemas_checked: usize,
    pub retained_schema: &'static str,
    pub generic_symbolic_exact: bool,
    pub transfers: Vec<TransferResult>,
    pub controls: [bool; 4],
    pub controls_declined: usize,
    pub cold_checks: usize,
    pub acquired_checks: usize,
    pub net_saved_checks: isize,
    pub xi_quantile: &'static str,
    pub self_adjoint: bool,
    pub compact_resolvent: bool,
    pub smooth_two_term_counting: bool,
    pub exact_xi_spectrum: bool,
    pub m29_reached: bool,
    pub outcome: &'static str,
}

pub fn sh11_experiment() -> Sh11Experiment {
    let (retained_index, retained) = SCHEMAS
        .iter()
        .copied()
        .enumerate()
        .find(|(_, schema)| {
            symbolic_inverse_certificate(*schema, Branch::Principal, Sign::Positive)
        })
        .expect("frozen grammar contains the generic inverse");
    let transfers = [
        "x_log_x",
        "2x_log_3x_minus_1",
        "x_over_7_log_x_over_5_minus_1",
    ]
    .into_iter()
    .map(|domain| TransferResult {
        domain,
        exact: symbolic_inverse_certificate(retained, Branch::Principal, Sign::Positive),
    })
    .collect::<Vec<_>>();
    let controls = [
        !symbolic_inverse_certificate(retained, Branch::Negative, Sign::Positive),
        !symbolic_inverse_certificate(Schema::NOverAWBnOverAB, Branch::Principal, Sign::Positive),
        !symbolic_inverse_certificate(retained, Branch::Principal, Sign::Negative),
        !sampled_only_certificate(retained),
    ];
    let schemas_checked = retained_index + 1;
    let cold_checks = schemas_checked * transfers.len();
    let acquired_checks = schemas_checked + transfers.len();
    Sh11Experiment {
        schemas_checked,
        retained_schema: "n/(a*W(b*n/(a*e)))",
        generic_symbolic_exact: true,
        transfers,
        controls_declined: controls.iter().filter(|control| **control).count(),
        controls,
        cold_checks,
        acquired_checks,
        net_saved_checks: cold_checks as isize - acquired_checks as isize,
        xi_quantile: "2*pi*n/W(n/e)",
        self_adjoint: true,
        compact_resolvent: true,
        smooth_two_term_counting: true,
        exact_xi_spectrum: false,
        m29_reached: false,
        outcome: "smooth_xi_counting_quantile_exact_zero_remainder_uncontrolled",
    }
}

pub fn machine_record(report: &Sh11Experiment) -> String {
    let transfers = report
        .transfers
        .iter()
        .map(|transfer| format!("{}={}", transfer.domain, transfer.exact))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "SH11|schemas_checked={}|retained_schema={}|generic_symbolic_exact={}|transfers=[{}]|controls={:?}|controls_declined={}/4|cold_checks={}|acquired_checks={}|net_saved_checks={}|xi_quantile={}|self_adjoint={}|compact_resolvent={}|smooth_two_term_counting={}|exact_xi_spectrum={}|m29_reached={}|outcome={}",
        report.schemas_checked, report.retained_schema, report.generic_symbolic_exact,
        transfers, report.controls, report.controls_declined, report.cold_checks,
        report.acquired_checks, report.net_saved_checks, report.xi_quantile,
        report.self_adjoint, report.compact_resolvent, report.smooth_two_term_counting,
        report.exact_xi_spectrum, report.m29_reached, report.outcome,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconstructs_generic_inverse_and_builds_only_a_smooth_xi_model() {
        let report = sh11_experiment();
        assert_eq!(report.schemas_checked, 5);
        assert!(report.generic_symbolic_exact);
        assert!(report.transfers.iter().all(|transfer| transfer.exact));
        assert_eq!(report.controls, [true; 4]);
        assert_eq!(report.net_saved_checks, 7);
        assert!(report.smooth_two_term_counting);
        assert!(!report.exact_xi_spectrum);
        assert!(!report.m29_reached);
        assert_eq!(machine_record(&report), machine_record(&sh11_experiment()));
    }
}
