//! Direction M14: construct and retain a reusable transformation action.
//!
//! Neither symmetry nor even/odd response labels occur in the proposal
//! language. Search combines generic scalar programs as input transformations
//! and output responses. Separate exact checkers validate composition laws and
//! inverses; frozen downstream tasks measure actual proposal checks.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ScalarProgram {
    Variable,
    Constant(i128),
    Add(Box<ScalarProgram>, Box<ScalarProgram>),
    Sub(Box<ScalarProgram>, Box<ScalarProgram>),
    Mul(Box<ScalarProgram>, Box<ScalarProgram>),
}

impl ScalarProgram {
    pub fn render(&self, variable: &str) -> String {
        match self {
            Self::Variable => variable.into(),
            Self::Constant(value) => value.to_string(),
            Self::Add(a, b) => format!("({}+{})", a.render(variable), b.render(variable)),
            Self::Sub(a, b) => format!("({}-{})", a.render(variable), b.render(variable)),
            Self::Mul(a, b) => format!("({}*{})", a.render(variable), b.render(variable)),
        }
    }

    fn size(&self) -> usize {
        match self {
            Self::Variable | Self::Constant(_) => 1,
            Self::Add(a, b) | Self::Sub(a, b) | Self::Mul(a, b) => 1 + a.size() + b.size(),
        }
    }

    fn eval(&self, value: i128) -> i128 {
        match self {
            Self::Variable => value,
            Self::Constant(constant) => *constant,
            Self::Add(a, b) => a.eval(value) + b.eval(value),
            Self::Sub(a, b) => a.eval(value) - b.eval(value),
            Self::Mul(a, b) => a.eval(value) * b.eval(value),
        }
    }
}

type ScalarPolynomial = BTreeMap<u8, i128>;

fn scalar_add(left: &ScalarPolynomial, right: &ScalarPolynomial, sign: i128) -> ScalarPolynomial {
    let mut result = left.clone();
    for (degree, coefficient) in right {
        *result.entry(*degree).or_default() += sign * coefficient;
    }
    result.retain(|_, coefficient| *coefficient != 0);
    result
}

fn scalar_mul(left: &ScalarPolynomial, right: &ScalarPolynomial) -> ScalarPolynomial {
    let mut result = ScalarPolynomial::new();
    for (left_degree, left_coefficient) in left {
        for (right_degree, right_coefficient) in right {
            *result.entry(left_degree + right_degree).or_default() +=
                left_coefficient * right_coefficient;
        }
    }
    result.retain(|_, coefficient| *coefficient != 0);
    result
}

fn normalize(program: &ScalarProgram) -> ScalarPolynomial {
    match program {
        ScalarProgram::Variable => ScalarPolynomial::from([(1, 1)]),
        ScalarProgram::Constant(0) => ScalarPolynomial::new(),
        ScalarProgram::Constant(value) => ScalarPolynomial::from([(0, *value)]),
        ScalarProgram::Add(a, b) => scalar_add(&normalize(a), &normalize(b), 1),
        ScalarProgram::Sub(a, b) => scalar_add(&normalize(a), &normalize(b), -1),
        ScalarProgram::Mul(a, b) => scalar_mul(&normalize(a), &normalize(b)),
    }
}

fn compose_scalar(outer: &ScalarPolynomial, inner: &ScalarPolynomial) -> ScalarPolynomial {
    let mut result = ScalarPolynomial::new();
    for (degree, coefficient) in outer {
        let mut power = ScalarPolynomial::from([(0, 1)]);
        for _ in 0..*degree {
            power = scalar_mul(&power, inner);
        }
        for value in power.values_mut() {
            *value *= coefficient;
        }
        result = scalar_add(&result, &power, 1);
    }
    result
}

fn enumerate_scalar_programs(max_size: usize) -> Vec<ScalarProgram> {
    let mut layers = vec![Vec::new(); max_size + 1];
    layers[1] = vec![
        ScalarProgram::Variable,
        ScalarProgram::Constant(-1),
        ScalarProgram::Constant(0),
        ScalarProgram::Constant(1),
    ];
    let mut globally_seen = layers[1].iter().map(normalize).collect::<BTreeSet<_>>();
    for size in 2..=max_size {
        for left_size in 1..size {
            let right_size = size - 1 - left_size;
            if right_size == 0 {
                continue;
            }
            for left in layers[left_size].clone() {
                for right in layers[right_size].clone() {
                    for candidate in [
                        ScalarProgram::Add(Box::new(left.clone()), Box::new(right.clone())),
                        ScalarProgram::Sub(Box::new(left.clone()), Box::new(right.clone())),
                        ScalarProgram::Mul(Box::new(left.clone()), Box::new(right.clone())),
                    ] {
                        if globally_seen.insert(normalize(&candidate)) {
                            layers[size].push(candidate);
                        }
                    }
                }
            }
        }
    }
    layers.into_iter().flatten().collect()
}

type VectorMonomial = [u8; 2];
type VectorPolynomial = BTreeMap<VectorMonomial, i128>;

fn vector_add(left: &VectorPolynomial, right: &VectorPolynomial, sign: i128) -> VectorPolynomial {
    let mut result = left.clone();
    for (monomial, coefficient) in right {
        *result.entry(*monomial).or_default() += sign * coefficient;
    }
    result.retain(|_, coefficient| *coefficient != 0);
    result
}

fn vector_mul(left: &VectorPolynomial, right: &VectorPolynomial) -> VectorPolynomial {
    let mut result = VectorPolynomial::new();
    for (lm, lc) in left {
        for (rm, rc) in right {
            *result.entry([lm[0] + rm[0], lm[1] + rm[1]]).or_default() += lc * rc;
        }
    }
    result.retain(|_, coefficient| *coefficient != 0);
    result
}

fn vector_pow(polynomial: &VectorPolynomial, exponent: u8) -> VectorPolynomial {
    (0..exponent).fold(VectorPolynomial::from([([0, 0], 1)]), |acc, _| {
        vector_mul(&acc, polynomial)
    })
}

fn lift_scalar(polynomial: &ScalarPolynomial, coordinate: usize) -> VectorPolynomial {
    polynomial
        .iter()
        .map(|(degree, coefficient)| {
            let mut monomial = [0, 0];
            monomial[coordinate] = *degree;
            (monomial, *coefficient)
        })
        .collect()
}

fn compose_observable_with_transformation(
    observable: &VectorPolynomial,
    transformation: &ScalarProgram,
) -> VectorPolynomial {
    let scalar = normalize(transformation);
    let images = [lift_scalar(&scalar, 0), lift_scalar(&scalar, 1)];
    let mut result = VectorPolynomial::new();
    for (monomial, coefficient) in observable {
        let mut term = vector_mul(
            &vector_pow(&images[0], monomial[0]),
            &vector_pow(&images[1], monomial[1]),
        );
        for value in term.values_mut() {
            *value *= coefficient;
        }
        result = vector_add(&result, &term, 1);
    }
    result
}

fn apply_response(response: &ScalarProgram, observable: &VectorPolynomial) -> VectorPolynomial {
    let mut result = VectorPolynomial::new();
    for (degree, coefficient) in normalize(response) {
        let mut term = vector_pow(observable, degree);
        for value in term.values_mut() {
            *value *= coefficient;
        }
        result = vector_add(&result, &term, 1);
    }
    result
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InverseCertificate {
    pub transformation: ScalarProgram,
    pub inverse: ScalarProgram,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InverseError {
    IdentityOrConstant,
    NotTwoSidedInverse,
}

pub fn check_inverse(certificate: &InverseCertificate) -> Result<(), InverseError> {
    let transformation = normalize(&certificate.transformation);
    if transformation == ScalarPolynomial::from([(1, 1)])
        || !transformation.keys().any(|degree| *degree > 0)
    {
        return Err(InverseError::IdentityOrConstant);
    }
    let inverse = normalize(&certificate.inverse);
    let identity = ScalarPolynomial::from([(1, 1)]);
    if compose_scalar(&transformation, &inverse) != identity
        || compose_scalar(&inverse, &transformation) != identity
    {
        return Err(InverseError::NotTwoSidedInverse);
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolynomialActionCertificate {
    pub transformation: ScalarProgram,
    pub response: ScalarProgram,
    pub observable: VectorPolynomial,
}

pub fn check_polynomial_action(certificate: &PolynomialActionCertificate) -> bool {
    compose_observable_with_transformation(&certificate.observable, &certificate.transformation)
        == apply_response(&certificate.response, &certificate.observable)
}

fn univariate_observable(terms: &[(u8, i128)]) -> VectorPolynomial {
    terms
        .iter()
        .map(|(degree, coefficient)| ([*degree, 0], *coefficient))
        .collect()
}

fn find_inverse(
    transformation: &ScalarProgram,
    programs: &[ScalarProgram],
) -> Option<ScalarProgram> {
    programs.iter().find_map(|inverse| {
        let certificate = InverseCertificate {
            transformation: transformation.clone(),
            inverse: inverse.clone(),
        };
        check_inverse(&certificate).is_ok().then(|| inverse.clone())
    })
}

fn sample_agrees(
    observable: &dyn Fn(i128) -> i128,
    transformation: &ScalarProgram,
    response: &ScalarProgram,
) -> bool {
    [-3, -2, -1, 0, 1, 2, 3]
        .into_iter()
        .all(|x| observable(transformation.eval(x)) == response.eval(observable(x)))
}

#[derive(Clone, Debug)]
pub struct DiscoveredAction {
    pub transformation: ScalarProgram,
    pub inverse: ScalarProgram,
    pub square_response: ScalarProgram,
    pub cube_response: ScalarProgram,
    pub proposal_checks: usize,
}

fn discover_action(programs: &[ScalarProgram]) -> Option<DiscoveredAction> {
    let square = univariate_observable(&[(2, 1)]);
    let cube = univariate_observable(&[(3, 1)]);
    let mut proposal_checks = 0;
    let mut winners = Vec::new();
    for transformation in programs {
        let Some(inverse) = find_inverse(transformation, programs) else {
            continue;
        };
        for square_response in programs {
            proposal_checks += 1;
            if !sample_agrees(&|x| x * x, transformation, square_response)
                || !check_polynomial_action(&PolynomialActionCertificate {
                    transformation: transformation.clone(),
                    response: square_response.clone(),
                    observable: square.clone(),
                })
            {
                continue;
            }
            for cube_response in programs {
                proposal_checks += 1;
                if sample_agrees(&|x| x * x * x, transformation, cube_response)
                    && check_polynomial_action(&PolynomialActionCertificate {
                        transformation: transformation.clone(),
                        response: cube_response.clone(),
                        observable: cube.clone(),
                    })
                {
                    winners.push((
                        transformation.size() + square_response.size() + cube_response.size(),
                        transformation.clone(),
                        inverse.clone(),
                        square_response.clone(),
                        cube_response.clone(),
                    ));
                }
            }
        }
    }
    winners.sort_by_key(
        |(cost, transformation, _, square_response, cube_response)| {
            (
                *cost,
                normalize(transformation),
                normalize(square_response),
                normalize(cube_response),
            )
        },
    );
    winners.into_iter().next().map(
        |(_, transformation, inverse, square_response, cube_response)| DiscoveredAction {
            transformation,
            inverse,
            square_response,
            cube_response,
            proposal_checks,
        },
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain {
    Polynomial,
    Geometry,
    Matrix,
    Cyclic(u8),
}

#[derive(Clone, Debug)]
struct DownstreamTask {
    name: &'static str,
    domain: Domain,
    observable: VectorPolynomial,
}

fn downstream_tasks() -> Vec<DownstreamTask> {
    vec![
        DownstreamTask {
            name: "quartic_polynomial",
            domain: Domain::Polynomial,
            observable: univariate_observable(&[(4, 1), (2, 1)]),
        },
        DownstreamTask {
            name: "squared_norm",
            domain: Domain::Geometry,
            observable: VectorPolynomial::from([([2, 0], 1), ([0, 2], 1)]),
        },
        DownstreamTask {
            name: "quadratic_form",
            domain: Domain::Matrix,
            observable: VectorPolynomial::from([([2, 0], 2), ([1, 1], 2), ([0, 2], 3)]),
        },
        DownstreamTask {
            name: "linear_form_control",
            domain: Domain::Matrix,
            observable: VectorPolynomial::from([([1, 0], 2), ([0, 1], -3)]),
        },
        DownstreamTask {
            name: "cyclic_inverse_distance",
            domain: Domain::Cyclic(7),
            observable: VectorPolynomial::new(),
        },
        DownstreamTask {
            name: "cyclic_coordinate_control",
            domain: Domain::Cyclic(7),
            observable: VectorPolynomial::new(),
        },
    ]
}

fn check_cyclic_action(
    task_name: &str,
    modulus: i128,
    transformation: &ScalarProgram,
    response: &ScalarProgram,
) -> bool {
    (0..modulus).all(|x| {
        let tx = transformation.eval(x).rem_euclid(modulus);
        let observable = |value: i128| match task_name {
            "cyclic_inverse_distance" => value.min(modulus - value),
            "cyclic_coordinate_control" => value,
            _ => unreachable!(),
        };
        response.eval(observable(x)).rem_euclid(modulus) == observable(tx).rem_euclid(modulus)
    })
}

fn check_task(
    task: &DownstreamTask,
    transformation: &ScalarProgram,
    response: &ScalarProgram,
) -> bool {
    match task.domain {
        Domain::Polynomial | Domain::Geometry | Domain::Matrix => {
            check_polynomial_action(&PolynomialActionCertificate {
                transformation: transformation.clone(),
                response: response.clone(),
                observable: task.observable.clone(),
            })
        }
        Domain::Cyclic(modulus) => {
            check_cyclic_action(task.name, modulus as i128, transformation, response)
        }
    }
}

#[derive(Clone, Debug)]
pub struct TransferMeasurement {
    pub task: &'static str,
    pub domain: Domain,
    pub baseline_checks: usize,
    pub retained_checks: usize,
    pub response: ScalarProgram,
}

fn measure_transfer(
    action: &DiscoveredAction,
    programs: &[ScalarProgram],
) -> Vec<TransferMeasurement> {
    let admissible = programs
        .iter()
        .filter(|transformation| find_inverse(transformation, programs).is_some())
        .cloned()
        .collect::<Vec<_>>();
    downstream_tasks()
        .into_iter()
        .map(|task| {
            let mut baseline_checks = 0;
            let baseline = admissible.iter().find_map(|transformation| {
                programs.iter().find_map(|response| {
                    baseline_checks += 1;
                    check_task(&task, transformation, response)
                        .then(|| (transformation.clone(), response.clone()))
                })
            });
            let mut retained_checks = 0;
            let response = programs
                .iter()
                .find(|response| {
                    retained_checks += 1;
                    check_task(&task, &action.transformation, response)
                })
                .cloned()
                .expect("frozen transfer task must have a response");
            let _baseline_winner = baseline.expect("baseline must find an action");
            TransferMeasurement {
                task: task.name,
                domain: task.domain,
                baseline_checks,
                retained_checks,
                response,
            }
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct SymmetryDiscovery {
    pub action: DiscoveredAction,
    pub transfers: Vec<TransferMeasurement>,
    pub scalar_programs: usize,
    pub constant_control_rejected: bool,
    pub identity_control_rejected: bool,
    pub asymmetric_control_rejected: bool,
    pub nonbijective_control_rejected: bool,
    pub constant_observable_uninformative: bool,
    pub square_only_ambiguous: bool,
    pub negative_transfer_tasks: usize,
    pub l3_boundary_passed: bool,
    pub baseline_checks: usize,
    pub retained_checks: usize,
    pub measured_gain: usize,
}

pub fn m14_experiment() -> SymmetryDiscovery {
    let programs = enumerate_scalar_programs(3);
    let action = discover_action(&programs).expect("M14 must find a checked action");
    let transfers = measure_transfer(&action, &programs);
    let baseline_checks = transfers
        .iter()
        .map(|transfer| transfer.baseline_checks)
        .sum();
    let retained_checks = transfers
        .iter()
        .map(|transfer| transfer.retained_checks)
        .sum();
    let identity = ScalarProgram::Variable;
    let square = univariate_observable(&[(2, 1)]);
    let identity_response = ScalarProgram::Variable;
    let square_only_ambiguous = check_polynomial_action(&PolynomialActionCertificate {
        transformation: identity.clone(),
        response: identity_response.clone(),
        observable: square.clone(),
    }) && check_polynomial_action(&PolynomialActionCertificate {
        transformation: action.transformation.clone(),
        response: identity_response,
        observable: square,
    });
    let asymmetric = univariate_observable(&[(2, 1), (1, 1)]);
    let constant_observable = univariate_observable(&[(0, 1)]);
    let admissible_constant_actions = programs
        .iter()
        .filter(|transformation| {
            find_inverse(transformation, &programs).is_some()
                && check_polynomial_action(&PolynomialActionCertificate {
                    transformation: (*transformation).clone(),
                    response: ScalarProgram::Variable,
                    observable: constant_observable.clone(),
                })
        })
        .count();
    let negative_transfer_tasks = transfers
        .iter()
        .filter(|transfer| transfer.retained_checks >= transfer.baseline_checks)
        .count();
    SymmetryDiscovery {
        constant_control_rejected: check_inverse(&InverseCertificate {
            transformation: ScalarProgram::Constant(0),
            inverse: identity.clone(),
        })
        .is_err(),
        identity_control_rejected: check_inverse(&InverseCertificate {
            transformation: identity.clone(),
            inverse: identity.clone(),
        })
        .is_err(),
        asymmetric_control_rejected: !check_polynomial_action(&PolynomialActionCertificate {
            transformation: action.transformation.clone(),
            response: ScalarProgram::Variable,
            observable: asymmetric,
        }),
        nonbijective_control_rejected: check_inverse(&InverseCertificate {
            transformation: ScalarProgram::Mul(
                Box::new(ScalarProgram::Variable),
                Box::new(ScalarProgram::Variable),
            ),
            inverse: identity,
        })
        .is_err(),
        constant_observable_uninformative: admissible_constant_actions > 1,
        square_only_ambiguous,
        negative_transfer_tasks,
        l3_boundary_passed: negative_transfer_tasks == 0,
        scalar_programs: programs.len(),
        action,
        transfers,
        baseline_checks,
        retained_checks,
        measured_gain: baseline_checks.saturating_sub(retained_checks),
    }
}

pub fn machine_record(report: &SymmetryDiscovery) -> String {
    let transfers = report
        .transfers
        .iter()
        .map(|transfer| {
            format!(
                "{}:{:?}:{}>{}:{}",
                transfer.task,
                transfer.domain,
                transfer.baseline_checks,
                transfer.retained_checks,
                transfer.response.render("output")
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "experiment=math_world_m14,transformation={},inverse={},square_response={},cube_response={},scalar_programs={},discovery_checks={},transfers={},constant_control_rejected={},constant_observable_uninformative={},identity_control_rejected={},asymmetric_control_rejected={},nonbijective_control_rejected={},square_only_ambiguous={},negative_transfer_tasks={},l3_boundary_passed={},baseline_checks={},retained_checks={},measured_aggregate_gain={},modeled_gain=none,pointwise_lifting_supplied=true,stability_objective_supplied=true,named_symmetry_primitives=false,claim_level=L2_invented_feature_in_supplied_meta_ontology,proof_status=exact_polynomial_and_bounded_group_action_checks,deterministic=true,fallback=exact",
        report.action.transformation.render("input"),
        report.action.inverse.render("input"),
        report.action.square_response.render("output"),
        report.action.cube_response.render("output"),
        report.scalar_programs,
        report.action.proposal_checks,
        transfers,
        report.constant_control_rejected,
        report.constant_observable_uninformative,
        report.identity_control_rejected,
        report.asymmetric_control_rejected,
        report.nonbijective_control_rejected,
        report.square_only_ambiguous,
        report.negative_transfer_tasks,
        report.l3_boundary_passed,
        report.baseline_checks,
        report.retained_checks,
        report.measured_gain,
    )
}

#[derive(Clone, Debug)]
enum ConditionalObservable {
    Polynomial(VectorPolynomial),
    CyclicDistance(i128),
    CyclicCoordinate(i128),
    CyclicIndicatorOne(i128),
}

#[derive(Clone, Debug)]
struct ConditionalTask {
    name: &'static str,
    domain: Domain,
    observable: ConditionalObservable,
    compatible: bool,
}

fn m14c_tasks() -> Vec<ConditionalTask> {
    vec![
        ConditionalTask {
            name: "sextic_even_polynomial",
            domain: Domain::Polynomial,
            observable: ConditionalObservable::Polynomial(univariate_observable(&[
                (6, 1),
                (4, -2),
                (2, 3),
            ])),
            compatible: true,
        },
        ConditionalTask {
            name: "quintic_odd_polynomial",
            domain: Domain::Polynomial,
            observable: ConditionalObservable::Polynomial(univariate_observable(&[
                (5, 1),
                (3, -2),
                (1, 1),
            ])),
            compatible: true,
        },
        ConditionalTask {
            name: "weighted_squared_norm",
            domain: Domain::Geometry,
            observable: ConditionalObservable::Polynomial(VectorPolynomial::from([
                ([2, 0], 4),
                ([0, 2], 9),
            ])),
            compatible: true,
        },
        ConditionalTask {
            name: "cubic_tensor_form",
            domain: Domain::Matrix,
            observable: ConditionalObservable::Polynomial(VectorPolynomial::from([
                ([3, 0], 1),
                ([1, 2], 2),
                ([0, 3], -1),
            ])),
            compatible: true,
        },
        ConditionalTask {
            name: "cyclic_11_inverse_distance",
            domain: Domain::Cyclic(11),
            observable: ConditionalObservable::CyclicDistance(11),
            compatible: true,
        },
        ConditionalTask {
            name: "cyclic_11_coordinate",
            domain: Domain::Cyclic(11),
            observable: ConditionalObservable::CyclicCoordinate(11),
            compatible: true,
        },
        ConditionalTask {
            name: "mixed_polynomial_control",
            domain: Domain::Polynomial,
            observable: ConditionalObservable::Polynomial(univariate_observable(&[
                (2, 1),
                (1, 1),
                (0, 1),
            ])),
            compatible: false,
        },
        ConditionalTask {
            name: "shifted_geometry_control",
            domain: Domain::Geometry,
            observable: ConditionalObservable::Polynomial(VectorPolynomial::from([
                ([2, 0], 1),
                ([1, 0], 2),
                ([0, 2], 1),
            ])),
            compatible: false,
        },
        ConditionalTask {
            name: "cyclic_11_indicator_control",
            domain: Domain::Cyclic(11),
            observable: ConditionalObservable::CyclicIndicatorOne(11),
            compatible: false,
        },
    ]
}

fn eval_vector_polynomial(polynomial: &VectorPolynomial, point: [i128; 2]) -> i128 {
    polynomial
        .iter()
        .map(|(monomial, coefficient)| {
            coefficient * point[0].pow(monomial[0] as u32) * point[1].pow(monomial[1] as u32)
        })
        .sum()
}

fn cyclic_observable(observable: &ConditionalObservable, value: i128) -> i128 {
    match observable {
        ConditionalObservable::CyclicDistance(modulus) => {
            let residue = value.rem_euclid(*modulus);
            residue.min(modulus - residue)
        }
        ConditionalObservable::CyclicCoordinate(modulus) => value.rem_euclid(*modulus),
        ConditionalObservable::CyclicIndicatorOne(modulus) => {
            i128::from(value.rem_euclid(*modulus) == 1)
        }
        ConditionalObservable::Polynomial(_) => unreachable!(),
    }
}

fn check_conditional_task(
    task: &ConditionalTask,
    transformation: &ScalarProgram,
    response: &ScalarProgram,
) -> bool {
    match &task.observable {
        ConditionalObservable::Polynomial(observable) => {
            check_polynomial_action(&PolynomialActionCertificate {
                transformation: transformation.clone(),
                response: response.clone(),
                observable: observable.clone(),
            })
        }
        ConditionalObservable::CyclicDistance(modulus)
        | ConditionalObservable::CyclicCoordinate(modulus)
        | ConditionalObservable::CyclicIndicatorOne(modulus) => (0..*modulus).all(|value| {
            let transformed = transformation.eval(value).rem_euclid(*modulus);
            response
                .eval(cyclic_observable(&task.observable, value))
                .rem_euclid(*modulus)
                == cyclic_observable(&task.observable, transformed).rem_euclid(*modulus)
        }),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Applicability {
    FirstResponse,
    SecondResponse,
    Declined,
}

fn probe_applicability(
    task: &ConditionalTask,
    transformation: &ScalarProgram,
    responses: &[ScalarProgram; 2],
) -> Applicability {
    let matches = |response: &ScalarProgram| match &task.observable {
        ConditionalObservable::Polynomial(observable) => {
            [[1, 2], [-1, -2], [2, -1]].into_iter().all(|point| {
                let transformed = [transformation.eval(point[0]), transformation.eval(point[1])];
                eval_vector_polynomial(observable, transformed)
                    == response.eval(eval_vector_polynomial(observable, point))
            })
        }
        ConditionalObservable::CyclicDistance(modulus)
        | ConditionalObservable::CyclicCoordinate(modulus)
        | ConditionalObservable::CyclicIndicatorOne(modulus) => {
            [1, 2, 3].into_iter().all(|value| {
                let transformed = transformation.eval(value).rem_euclid(*modulus);
                response
                    .eval(cyclic_observable(&task.observable, value))
                    .rem_euclid(*modulus)
                    == cyclic_observable(&task.observable, transformed).rem_euclid(*modulus)
            })
        }
    };
    if matches(&responses[0]) {
        Applicability::FirstResponse
    } else if matches(&responses[1]) {
        Applicability::SecondResponse
    } else {
        Applicability::Declined
    }
}

fn baseline_conditional_search(
    task: &ConditionalTask,
    admissible: &[ScalarProgram],
    responses: &[ScalarProgram],
) -> (usize, Option<(ScalarProgram, ScalarProgram)>) {
    let mut checks = 0;
    for transformation in admissible {
        for response in responses {
            checks += 1;
            if check_conditional_task(task, transformation, response) {
                return (checks, Some((transformation.clone(), response.clone())));
            }
        }
    }
    (checks, None)
}

#[derive(Clone, Debug)]
pub struct ConditionalTransferMeasurement {
    pub task: &'static str,
    pub domain: Domain,
    pub compatible: bool,
    pub applicability: Applicability,
    pub probe_evaluations_per_condition: usize,
    pub baseline_checks: usize,
    pub acquired_checks: usize,
    pub false_positive_route: bool,
    pub winner_response: Option<ScalarProgram>,
}

#[derive(Clone, Debug)]
pub struct ConditionalSymmetryDiscovery {
    pub transformation: ScalarProgram,
    pub acquired_responses: [ScalarProgram; 2],
    pub transfers: Vec<ConditionalTransferMeasurement>,
    pub baseline_checks: usize,
    pub acquired_checks: usize,
    pub probe_evaluations_per_condition: usize,
    pub false_positive_routes: usize,
    pub negative_transfer_tasks: usize,
    pub compatible_accelerated: usize,
    pub controls_unchanged: usize,
    pub measured_gain: usize,
    pub l3_boundary_passed: bool,
}

/// Separately pre-registered M14c repair. It consumes M14's concepts but none
/// of M14c's frozen tasks participate in action or response discovery.
pub fn m14c_experiment() -> ConditionalSymmetryDiscovery {
    let programs = enumerate_scalar_programs(3);
    let action = discover_action(&programs).expect("frozen M14 action");
    let acquired_responses = [action.square_response, action.cube_response];
    let admissible = programs
        .iter()
        .filter(|transformation| find_inverse(transformation, &programs).is_some())
        .cloned()
        .collect::<Vec<_>>();
    let mut transfers = Vec::new();
    for task in m14c_tasks() {
        let (baseline_checks, baseline_winner) =
            baseline_conditional_search(&task, &admissible, &programs);
        let applicability = probe_applicability(&task, &action.transformation, &acquired_responses);
        let (acquired_checks, winner_response) = match applicability {
            Applicability::Declined => (
                baseline_checks,
                baseline_winner
                    .as_ref()
                    .map(|(_, response)| response.clone()),
            ),
            Applicability::FirstResponse | Applicability::SecondResponse => {
                let order = if applicability == Applicability::FirstResponse {
                    [0, 1]
                } else {
                    [1, 0]
                };
                let mut checks = 0;
                let found = order.into_iter().find_map(|index| {
                    checks += 1;
                    check_conditional_task(
                        &task,
                        &action.transformation,
                        &acquired_responses[index],
                    )
                    .then(|| acquired_responses[index].clone())
                });
                match found {
                    Some(response) => (checks, Some(response)),
                    None => {
                        let (_, winner) =
                            baseline_conditional_search(&task, &admissible, &programs);
                        (
                            checks + baseline_checks,
                            winner.map(|(_, response)| response),
                        )
                    }
                }
            }
        };
        transfers.push(ConditionalTransferMeasurement {
            task: task.name,
            domain: task.domain,
            compatible: task.compatible,
            applicability,
            probe_evaluations_per_condition: 6,
            baseline_checks,
            acquired_checks,
            false_positive_route: !task.compatible && applicability != Applicability::Declined,
            winner_response,
        });
    }
    let baseline_checks = transfers.iter().map(|task| task.baseline_checks).sum();
    let acquired_checks = transfers.iter().map(|task| task.acquired_checks).sum();
    let probe_evaluations_per_condition = transfers
        .iter()
        .map(|task| task.probe_evaluations_per_condition)
        .sum();
    let false_positive_routes = transfers
        .iter()
        .filter(|task| task.false_positive_route)
        .count();
    let negative_transfer_tasks = transfers
        .iter()
        .filter(|task| task.acquired_checks > task.baseline_checks)
        .count();
    let compatible_accelerated = transfers
        .iter()
        .filter(|task| task.compatible && task.acquired_checks < task.baseline_checks)
        .count();
    let controls_unchanged = transfers
        .iter()
        .filter(|task| !task.compatible && task.acquired_checks == task.baseline_checks)
        .count();
    let l3_boundary_passed = compatible_accelerated == 6
        && controls_unchanged == 3
        && false_positive_routes == 0
        && negative_transfer_tasks == 0
        && acquired_checks < baseline_checks;
    ConditionalSymmetryDiscovery {
        transformation: action.transformation,
        acquired_responses,
        transfers,
        baseline_checks,
        acquired_checks,
        probe_evaluations_per_condition,
        false_positive_routes,
        negative_transfer_tasks,
        compatible_accelerated,
        controls_unchanged,
        measured_gain: baseline_checks.saturating_sub(acquired_checks),
        l3_boundary_passed,
    }
}

pub fn m14c_machine_record(report: &ConditionalSymmetryDiscovery) -> String {
    let transfers = report
        .transfers
        .iter()
        .map(|task| {
            format!(
                "{}:{:?}:compatible={}:route={:?}:checks={}>{}:response={}",
                task.task,
                task.domain,
                task.compatible,
                task.applicability,
                task.baseline_checks,
                task.acquired_checks,
                task.winner_response
                    .as_ref()
                    .map(|response| response.render("output"))
                    .unwrap_or_else(|| "no_solution".into())
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "experiment=math_world_m14c,transformation={},responses={};{},transfers={},probe_evaluations_per_condition={},baseline_checks={},acquired_checks={},measured_gain={},compatible_accelerated={},controls_unchanged={},false_positive_routes={},negative_transfer_tasks={},l3_boundary_passed={},domain_labels_used_by_policy=false,degree_parity_supplied=false,pointwise_lifting_supplied=true,intervention_routing_supplied=true,claim_level={},proof_status=exact_conditional_action_response_transfer,deterministic=true,fallback=exact",
        report.transformation.render("input"),
        report.acquired_responses[0].render("output"),
        report.acquired_responses[1].render("output"),
        transfers,
        report.probe_evaluations_per_condition,
        report.baseline_checks,
        report.acquired_checks,
        report.measured_gain,
        report.compatible_accelerated,
        report.controls_unchanged,
        report.false_positive_routes,
        report.negative_transfer_tasks,
        report.l3_boundary_passed,
        if report.l3_boundary_passed {
            "L3_transferred_ontology_with_measured_utility"
        } else {
            "L2_invented_feature_in_supplied_meta_ontology"
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs_nontrivial_action_and_independent_responses() {
        let report = m14_experiment();
        assert_eq!(
            normalize(&report.action.transformation),
            ScalarPolynomial::from([(1, -1)])
        );
        assert_eq!(
            normalize(&report.action.inverse),
            ScalarPolynomial::from([(1, -1)])
        );
        assert_eq!(
            normalize(&report.action.square_response),
            ScalarPolynomial::from([(1, 1)])
        );
        assert_eq!(
            normalize(&report.action.cube_response),
            ScalarPolynomial::from([(1, -1)])
        );
        assert!(check_inverse(&InverseCertificate {
            transformation: report.action.transformation.clone(),
            inverse: report.action.inverse.clone(),
        })
        .is_ok());
    }

    #[test]
    fn transfers_with_actual_measured_search_reduction() {
        let report = m14_experiment();
        assert_eq!(report.transfers.len(), 6);
        assert_eq!(
            report
                .transfers
                .iter()
                .filter(|transfer| transfer.retained_checks < transfer.baseline_checks)
                .count(),
            4
        );
        assert_eq!(report.negative_transfer_tasks, 2);
        assert!(!report.l3_boundary_passed);
        assert!(report.retained_checks < report.baseline_checks);
        assert_eq!(
            report.measured_gain,
            report.baseline_checks - report.retained_checks
        );
    }

    #[test]
    fn controls_and_ablation_are_explicit() {
        let report = m14_experiment();
        assert!(report.constant_control_rejected);
        assert!(report.constant_observable_uninformative);
        assert!(report.identity_control_rejected);
        assert!(report.asymmetric_control_rejected);
        assert!(report.nonbijective_control_rejected);
        assert!(report.square_only_ambiguous);
    }

    #[test]
    fn corrupted_certificates_fail_and_record_is_deterministic() {
        let report = m14_experiment();
        let bad = PolynomialActionCertificate {
            transformation: report.action.transformation.clone(),
            response: ScalarProgram::Variable,
            observable: univariate_observable(&[(3, 1)]),
        };
        assert!(!check_polynomial_action(&bad));
        assert_eq!(
            machine_record(&m14_experiment()),
            machine_record(&m14_experiment())
        );
    }

    #[test]
    fn conditional_schema_accelerates_compatible_families_without_negative_transfer() {
        let report = m14c_experiment();
        assert_eq!(report.compatible_accelerated, 6);
        assert_eq!(report.controls_unchanged, 3);
        assert_eq!(report.false_positive_routes, 0);
        assert_eq!(report.negative_transfer_tasks, 0);
        assert!(report.acquired_checks < report.baseline_checks);
        assert!(report.l3_boundary_passed);
    }

    #[test]
    fn m14c_has_separate_accounting_and_is_deterministic() {
        let report = m14c_experiment();
        assert_eq!(report.probe_evaluations_per_condition, 54);
        assert!(report
            .transfers
            .iter()
            .filter(|task| !task.compatible)
            .all(|task| task.applicability == Applicability::Declined));
        assert_eq!(
            m14c_machine_record(&m14c_experiment()),
            m14c_machine_record(&m14c_experiment())
        );
    }
}
