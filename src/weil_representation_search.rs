//! SH19: dimension-uniform representation search for Weil positivity.

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
enum Kind {
    Recurrence,
    TriangularFactor,
    PositiveKernel,
    SumOfSquares,
    IntegralSplit,
    BoundedRemainder,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
enum Body {
    Zero,
    One,
    Two,
    Dim,
    PrevPivot,
    OneOverDim,
    TwoPrevPlusOne,
    ThreeTermAssembly,
    ObservedTable,
    DiagonalShift,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
enum Remainder {
    Zero,
    InvDim,
    InvPow2,
    Constant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
enum Topology {
    FiniteDiscrete,
    L2,
    TestFunction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
enum TermCover {
    AllThree,
    MissingPole,
    MissingArchimedean,
    MissingPrime,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Program {
    kind: Kind,
    body: Body,
    remainder: Remainder,
    topology: Topology,
    term_cover: TermCover,
}

fn generate_programs() -> Vec<Program> {
    let mut programs = Vec::new();
    for kind in [
        Kind::Recurrence,
        Kind::TriangularFactor,
        Kind::PositiveKernel,
        Kind::SumOfSquares,
        Kind::IntegralSplit,
        Kind::BoundedRemainder,
    ] {
        for body in [
            Body::Zero,
            Body::One,
            Body::Two,
            Body::Dim,
            Body::PrevPivot,
            Body::OneOverDim,
            Body::TwoPrevPlusOne,
            Body::ThreeTermAssembly,
            Body::ObservedTable,
            Body::DiagonalShift,
        ] {
            for remainder in [
                Remainder::Zero,
                Remainder::InvDim,
                Remainder::InvPow2,
                Remainder::Constant,
            ] {
                for topology in [
                    Topology::FiniteDiscrete,
                    Topology::L2,
                    Topology::TestFunction,
                ] {
                    for term_cover in [
                        TermCover::AllThree,
                        TermCover::MissingPole,
                        TermCover::MissingArchimedean,
                        TermCover::MissingPrime,
                    ] {
                        programs.push(Program {
                            kind,
                            body,
                            remainder,
                            topology,
                            term_cover,
                        });
                    }
                }
            }
        }
    }
    programs
}

fn control_body(program: Program) -> bool {
    matches!(program.body, Body::ObservedTable | Body::DiagonalShift)
}

fn identity_ok(program: Program) -> bool {
    program.body == Body::ThreeTermAssembly
        && program.term_cover == TermCover::AllThree
        && program.remainder == Remainder::Zero
        && !control_body(program)
}

fn has_certified_positive_witness(_: Program) -> bool {
    // Finite SH19a LDL margins are observations, not an SOS/kernel witness.
    false
}

fn nonnegativity_ok(program: Program) -> bool {
    identity_ok(program)
        && matches!(program.kind, Kind::SumOfSquares | Kind::PositiveKernel)
        && has_certified_positive_witness(program)
}

fn uniform_bound_ok(program: Program) -> bool {
    nonnegativity_ok(program) && program.remainder != Remainder::InvDim
}

fn density_continuity_ok(program: Program) -> bool {
    uniform_bound_ok(program)
        && program.topology == Topology::TestFunction
        && !matches!(program.kind, Kind::Recurrence | Kind::TriangularFactor)
}

fn retained(program: Program) -> bool {
    identity_ok(program)
        && nonnegativity_ok(program)
        && uniform_bound_ok(program)
        && density_continuity_ok(program)
}

fn interpolation_control() -> Program {
    Program {
        kind: Kind::Recurrence,
        body: Body::ObservedTable,
        remainder: Remainder::Zero,
        topology: Topology::FiniteDiscrete,
        term_cover: TermCover::AllThree,
    }
}

fn dimension_shift_control() -> Program {
    Program {
        kind: Kind::Recurrence,
        body: Body::DiagonalShift,
        remainder: Remainder::Zero,
        topology: Topology::TestFunction,
        term_cover: TermCover::AllThree,
    }
}

fn ldl_table_control() -> Program {
    Program {
        kind: Kind::TriangularFactor,
        body: Body::ObservedTable,
        remainder: Remainder::Zero,
        topology: Topology::FiniteDiscrete,
        term_cover: TermCover::AllThree,
    }
}

fn wrong_topology_control() -> Program {
    Program {
        kind: Kind::IntegralSplit,
        body: Body::ThreeTermAssembly,
        remainder: Remainder::Zero,
        topology: Topology::L2,
        term_cover: TermCover::AllThree,
    }
}

fn nonuniform_remainder_control() -> Program {
    Program {
        kind: Kind::BoundedRemainder,
        body: Body::ThreeTermAssembly,
        remainder: Remainder::InvDim,
        topology: Topology::TestFunction,
        term_cover: TermCover::AllThree,
    }
}

fn omitted_term_control() -> Program {
    Program {
        kind: Kind::IntegralSplit,
        body: Body::ThreeTermAssembly,
        remainder: Remainder::Zero,
        topology: Topology::TestFunction,
        term_cover: TermCover::MissingPrime,
    }
}

#[derive(Clone, Debug)]
pub struct Sh19Experiment {
    pub programs_checked: usize,
    pub identity_candidates: usize,
    pub retained_representation: bool,
    pub controls: [bool; 6],
    pub controls_declined: usize,
    pub uniform_identity: bool,
    pub manifest_nonnegativity: bool,
    pub uniform_remainder: bool,
    pub density_continuity_bridge: bool,
    pub stage_two_gate_held: bool,
    pub m29_reached: bool,
}

pub fn sh19_experiment() -> Sh19Experiment {
    let programs = generate_programs();
    let programs_checked = programs.len();
    let identity_candidates = programs.iter().copied().filter(|p| identity_ok(*p)).count();
    let retained_representation = programs.iter().copied().any(retained);
    let uniform_identity = retained_representation;
    let manifest_nonnegativity = programs.iter().copied().any(nonnegativity_ok);
    let uniform_remainder = programs.iter().copied().any(uniform_bound_ok);
    let density_continuity_bridge = programs.iter().copied().any(density_continuity_ok);
    let controls = [
        !identity_ok(interpolation_control()) && !retained(interpolation_control()),
        !identity_ok(dimension_shift_control()) && !retained(dimension_shift_control()),
        !identity_ok(ldl_table_control()) && !retained(ldl_table_control()),
        identity_ok(wrong_topology_control()) && !density_continuity_ok(wrong_topology_control()),
        !identity_ok(nonuniform_remainder_control()) && !retained(nonuniform_remainder_control()),
        !identity_ok(omitted_term_control()) && !retained(omitted_term_control()),
    ];
    Sh19Experiment {
        programs_checked,
        identity_candidates,
        retained_representation,
        controls_declined: controls.iter().filter(|value| **value).count(),
        controls,
        uniform_identity,
        manifest_nonnegativity,
        uniform_remainder,
        density_continuity_bridge,
        stage_two_gate_held: true,
        m29_reached: false,
    }
}

pub fn machine_record(report: &Sh19Experiment) -> String {
    format!(
        "SH19|programs_checked={}|identity_candidates={}|retained_representation={}|controls={:?}|controls_declined={}/6|uniform_identity={}|manifest_nonnegativity={}|uniform_remainder={}|density_continuity_bridge={}|stage_two_gate_held={}|m29_reached=false|claim=bounded_representation_search_only",
        report.programs_checked,
        report.identity_candidates,
        report.retained_representation,
        report.controls,
        report.controls_declined,
        report.uniform_identity,
        report.manifest_nonnegativity,
        report.uniform_remainder,
        report.density_continuity_bridge,
        report.stage_two_gate_held,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grammar_is_the_frozen_five_tuple_product() {
        let programs = generate_programs();
        assert_eq!(programs.len(), 6 * 10 * 4 * 3 * 4);
        assert!(programs.windows(2).all(|pair| pair[0].kind < pair[1].kind
            || pair[0].kind == pair[1].kind
                && (pair[0].body < pair[1].body
                    || pair[0].body == pair[1].body
                        && (pair[0].remainder < pair[1].remainder
                            || pair[0].remainder == pair[1].remainder
                                && (pair[0].topology < pair[1].topology
                                    || pair[0].topology == pair[1].topology
                                        && pair[0].term_cover <= pair[1].term_cover)))));
        assert!(programs
            .iter()
            .any(|program| program.body == Body::ThreeTermAssembly));
        assert!(programs
            .iter()
            .any(|program| program.body == Body::ObservedTable));
        assert!(!programs
            .iter()
            .any(|program| identity_ok(*program) && control_body(*program)));
    }

    #[test]
    fn three_term_assembly_is_identity_not_positivity() {
        let identity = Program {
            kind: Kind::IntegralSplit,
            body: Body::ThreeTermAssembly,
            remainder: Remainder::Zero,
            topology: Topology::TestFunction,
            term_cover: TermCover::AllThree,
        };
        assert!(identity_ok(identity));
        assert!(!nonnegativity_ok(identity));
        assert!(!retained(identity));
    }

    #[test]
    fn all_leakage_and_overfit_controls_decline() {
        let report = sh19_experiment();
        assert_eq!(report.programs_checked, 2880);
        assert!(report.identity_candidates > 0);
        assert_eq!(report.controls, [true; 6]);
        assert!(!report.retained_representation);
        assert!(!report.uniform_identity);
        assert!(!report.manifest_nonnegativity);
        assert!(!report.uniform_remainder);
        assert!(!report.density_continuity_bridge);
        assert!(report.stage_two_gate_held);
        assert!(!report.m29_reached);
    }

    #[test]
    fn finite_sh19a_margin_is_not_a_witness() {
        let sos = Program {
            kind: Kind::SumOfSquares,
            body: Body::ThreeTermAssembly,
            remainder: Remainder::Zero,
            topology: Topology::TestFunction,
            term_cover: TermCover::AllThree,
        };
        assert!(identity_ok(sos));
        assert!(!has_certified_positive_witness(sos));
        assert!(!nonnegativity_ok(sos));
    }
}
