//! Direction M13b: invent permutation invariance before root/coefficient laws.
//!
//! Roots initially occupy addressable slots. The feature proposer receives
//! only slot projection, constants, and arithmetic; it has no unordered-bag,
//! orbit-sum, or symmetric-polynomial primitive. Permutation interventions and
//! exact symbolic comparison select invariant programs. A second, generic
//! expression search composes the invented features with coefficients. Its
//! candidates are checked by exact reduction modulo the independently expanded
//! factorization constraints.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rational {
    numerator: i128,
    denominator: i128,
}

impl Rational {
    pub fn new(numerator: i128, denominator: i128) -> Self {
        assert_ne!(denominator, 0);
        let sign = if denominator < 0 { -1 } else { 1 };
        let (mut a, mut b) = (numerator.unsigned_abs(), denominator.unsigned_abs());
        while b != 0 {
            (a, b) = (b, a % b);
        }
        let divisor = a.max(1) as i128;
        Self {
            numerator: sign * numerator / divisor,
            denominator: denominator.abs() / divisor,
        }
    }

    fn integer(value: i128) -> Self {
        Self::new(value, 1)
    }

    fn is_zero(self) -> bool {
        self.numerator == 0
    }
}

impl std::ops::Add for Rational {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(
            self.numerator * rhs.denominator + rhs.numerator * self.denominator,
            self.denominator * rhs.denominator,
        )
    }
}

impl std::ops::Sub for Rational {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        self + Self::new(-rhs.numerator, rhs.denominator)
    }
}

impl std::ops::Mul for Rational {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self::new(
            self.numerator * rhs.numerator,
            self.denominator * rhs.denominator,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComplexRational {
    real: Rational,
    imaginary: Rational,
}

impl ComplexRational {
    fn real(value: i128) -> Self {
        Self {
            real: Rational::integer(value),
            imaginary: Rational::integer(0),
        }
    }

    fn new(real: Rational, imaginary: Rational) -> Self {
        Self { real, imaginary }
    }

    fn is_zero(self) -> bool {
        self.real.is_zero() && self.imaginary.is_zero()
    }
}

impl std::ops::Add for ComplexRational {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.real + rhs.real, self.imaginary + rhs.imaginary)
    }
}

impl std::ops::Sub for ComplexRational {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.real - rhs.real, self.imaginary - rhs.imaginary)
    }
}

impl std::ops::Mul for ComplexRational {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self::new(
            self.real * rhs.real - self.imaginary * rhs.imaginary,
            self.real * rhs.imaginary + self.imaginary * rhs.real,
        )
    }
}

/// At feature-discovery time roots are ordered observations. Swapping them is
/// an intervention, not a no-op hidden by the representation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootTuple([ComplexRational; 2]);

impl RootTuple {
    fn swapped(&self) -> Self {
        Self([self.0[1], self.0[0]])
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RootProgram {
    Slot(usize),
    Constant(i128),
    Add(Box<RootProgram>, Box<RootProgram>),
    Sub(Box<RootProgram>, Box<RootProgram>),
    Mul(Box<RootProgram>, Box<RootProgram>),
}

impl RootProgram {
    fn eval(&self, roots: &RootTuple) -> ComplexRational {
        match self {
            Self::Slot(index) => roots.0[*index],
            Self::Constant(value) => ComplexRational::real(*value),
            Self::Add(a, b) => a.eval(roots) + b.eval(roots),
            Self::Sub(a, b) => a.eval(roots) - b.eval(roots),
            Self::Mul(a, b) => a.eval(roots) * b.eval(roots),
        }
    }

    pub fn render(&self) -> String {
        match self {
            Self::Slot(index) => format!("root[{index}]"),
            Self::Constant(value) => value.to_string(),
            Self::Add(a, b) => format!("({}+{})", a.render(), b.render()),
            Self::Sub(a, b) => format!("({}-{})", a.render(), b.render()),
            Self::Mul(a, b) => format!("({}*{})", a.render(), b.render()),
        }
    }
}

type RootMonomial = [u8; 2];
type RootPolynomial = BTreeMap<RootMonomial, Rational>;
type Monomial = [u8; 5]; // a, b, c, root[0], root[1]
type Polynomial = BTreeMap<Monomial, Rational>;

fn poly_add<const N: usize>(
    left: &BTreeMap<[u8; N], Rational>,
    right: &BTreeMap<[u8; N], Rational>,
    sign: i128,
) -> BTreeMap<[u8; N], Rational> {
    let mut result = left.clone();
    for (monomial, coefficient) in right {
        let old = result
            .get(monomial)
            .copied()
            .unwrap_or(Rational::integer(0));
        result.insert(*monomial, old + *coefficient * Rational::integer(sign));
    }
    result.retain(|_, coefficient| !coefficient.is_zero());
    result
}

fn poly_mul<const N: usize>(
    left: &BTreeMap<[u8; N], Rational>,
    right: &BTreeMap<[u8; N], Rational>,
) -> BTreeMap<[u8; N], Rational> {
    let mut result = BTreeMap::new();
    for (lm, lc) in left {
        for (rm, rc) in right {
            let mut monomial = [0; N];
            for index in 0..N {
                monomial[index] = lm[index] + rm[index];
            }
            let old = result
                .get(&monomial)
                .copied()
                .unwrap_or(Rational::integer(0));
            result.insert(monomial, old + *lc * *rc);
        }
    }
    result.retain(|_, coefficient| !coefficient.is_zero());
    result
}

fn normalize_root(program: &RootProgram) -> RootPolynomial {
    match program {
        RootProgram::Slot(index) => {
            let mut monomial = [0; 2];
            monomial[*index] = 1;
            RootPolynomial::from([(monomial, Rational::integer(1))])
        }
        RootProgram::Constant(0) => RootPolynomial::new(),
        RootProgram::Constant(value) => RootPolynomial::from([([0; 2], Rational::integer(*value))]),
        RootProgram::Add(a, b) => poly_add(&normalize_root(a), &normalize_root(b), 1),
        RootProgram::Sub(a, b) => poly_add(&normalize_root(a), &normalize_root(b), -1),
        RootProgram::Mul(a, b) => poly_mul(&normalize_root(a), &normalize_root(b)),
    }
}

fn swap_root_polynomial(polynomial: &RootPolynomial) -> RootPolynomial {
    polynomial
        .iter()
        .map(|(monomial, coefficient)| ([monomial[1], monomial[0]], *coefficient))
        .collect()
}

fn genuinely_binary(polynomial: &RootPolynomial) -> bool {
    let uses_left = polynomial.keys().any(|monomial| monomial[0] > 0);
    let uses_right = polynomial.keys().any(|monomial| monomial[1] > 0);
    uses_left && uses_right
}

fn root_program_layers(max_size: usize) -> Vec<Vec<RootProgram>> {
    let mut layers = vec![Vec::new(); max_size + 1];
    layers[1] = vec![
        RootProgram::Slot(0),
        RootProgram::Slot(1),
        RootProgram::Constant(-1),
        RootProgram::Constant(0),
        RootProgram::Constant(1),
    ];
    for size in 2..=max_size {
        let mut seen = BTreeSet::new();
        for left_size in 1..size {
            let right_size = size - 1 - left_size;
            if right_size == 0 {
                continue;
            }
            for left in layers[left_size].clone() {
                for right in layers[right_size].clone() {
                    for candidate in [
                        RootProgram::Add(Box::new(left.clone()), Box::new(right.clone())),
                        RootProgram::Sub(Box::new(left.clone()), Box::new(right.clone())),
                        RootProgram::Mul(Box::new(left.clone()), Box::new(right.clone())),
                    ] {
                        let normal = normalize_root(&candidate);
                        if seen.insert(normal) {
                            layers[size].push(candidate);
                        }
                    }
                }
            }
        }
    }
    layers
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InventedInvariant {
    pub program: RootProgram,
    pub discovery_size: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InvarianceError {
    ConstantOrUnary,
    ChangesUnderPermutation,
}

/// Independent exact checker for the representation-discovery stage. It
/// compares normalized polynomials before and after the transposition action;
/// finite intervention agreement alone can never certify invariance.
pub fn check_invariant_program(program: &RootProgram) -> Result<(), InvarianceError> {
    let normal = normalize_root(program);
    if normal.is_empty() || !genuinely_binary(&normal) {
        return Err(InvarianceError::ConstantOrUnary);
    }
    if swap_root_polynomial(&normal) != normal {
        return Err(InvarianceError::ChangesUnderPermutation);
    }
    Ok(())
}

fn discover_invariants(examples: &[QuadraticExample]) -> (Vec<InventedInvariant>, usize, usize) {
    let layers = root_program_layers(3);
    let mut tested = 0;
    let mut interventions = 0;
    let mut invariants = Vec::new();
    for (size, layer) in layers.into_iter().enumerate().skip(1) {
        for program in layer {
            tested += 1;
            let normal = normalize_root(&program);
            if check_invariant_program(&program).is_err() {
                continue;
            }
            let stable = examples.iter().all(|example| {
                interventions += 1;
                program.eval(&example.roots) == program.eval(&example.roots.swapped())
            });
            if stable
                && !invariants
                    .iter()
                    .any(|known: &InventedInvariant| normalize_root(&known.program) == normal)
            {
                invariants.push(InventedInvariant {
                    program,
                    discovery_size: size,
                });
            }
        }
    }
    (invariants, tested, interventions)
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RelationExpr {
    Coefficient(usize),
    Invariant(usize),
    Constant(i128),
    Add(Box<RelationExpr>, Box<RelationExpr>),
    Sub(Box<RelationExpr>, Box<RelationExpr>),
    Mul(Box<RelationExpr>, Box<RelationExpr>),
}

impl RelationExpr {
    pub fn render(&self, invariants: &[InventedInvariant]) -> String {
        match self {
            Self::Coefficient(index) => ["a", "b", "c"][*index].into(),
            Self::Invariant(index) => format!("inv{{{}}}", invariants[*index].program.render()),
            Self::Constant(value) => value.to_string(),
            Self::Add(a, b) => format!("({}+{})", a.render(invariants), b.render(invariants)),
            Self::Sub(a, b) => format!("({}-{})", a.render(invariants), b.render(invariants)),
            Self::Mul(a, b) => format!("({}*{})", a.render(invariants), b.render(invariants)),
        }
    }

    fn uses_coefficient(&self) -> bool {
        match self {
            Self::Coefficient(_) => true,
            Self::Invariant(_) | Self::Constant(_) => false,
            Self::Add(a, b) | Self::Sub(a, b) | Self::Mul(a, b) => {
                a.uses_coefficient() || b.uses_coefficient()
            }
        }
    }

    fn uses_invariant(&self) -> bool {
        match self {
            Self::Invariant(_) => true,
            Self::Coefficient(_) | Self::Constant(_) => false,
            Self::Add(a, b) | Self::Sub(a, b) | Self::Mul(a, b) => {
                a.uses_invariant() || b.uses_invariant()
            }
        }
    }
}

fn lift_root_polynomial(root: &RootPolynomial) -> Polynomial {
    root.iter()
        .map(|(monomial, coefficient)| ([0, 0, 0, monomial[0], monomial[1]], *coefficient))
        .collect()
}

fn normalize_relation(expr: &RelationExpr, invariants: &[InventedInvariant]) -> Polynomial {
    match expr {
        RelationExpr::Coefficient(index) => {
            let mut monomial = [0; 5];
            monomial[*index] = 1;
            Polynomial::from([(monomial, Rational::integer(1))])
        }
        RelationExpr::Invariant(index) => {
            lift_root_polynomial(&normalize_root(&invariants[*index].program))
        }
        RelationExpr::Constant(0) => Polynomial::new(),
        RelationExpr::Constant(value) => Polynomial::from([([0; 5], Rational::integer(*value))]),
        RelationExpr::Add(a, b) => poly_add(
            &normalize_relation(a, invariants),
            &normalize_relation(b, invariants),
            1,
        ),
        RelationExpr::Sub(a, b) => poly_add(
            &normalize_relation(a, invariants),
            &normalize_relation(b, invariants),
            -1,
        ),
        RelationExpr::Mul(a, b) => poly_mul(
            &normalize_relation(a, invariants),
            &normalize_relation(b, invariants),
        ),
    }
}

fn relation_layers(max_size: usize, invariant_count: usize) -> Vec<Vec<RelationExpr>> {
    let mut layers = vec![Vec::new(); max_size + 1];
    layers[1].extend((0..3).map(RelationExpr::Coefficient));
    layers[1].extend((0..invariant_count).map(RelationExpr::Invariant));
    layers[1].extend([-1, 0, 1].map(RelationExpr::Constant));
    for size in 2..=max_size {
        let mut seen = BTreeSet::new();
        for left_size in 1..size {
            let right_size = size - 1 - left_size;
            if right_size == 0 {
                continue;
            }
            for left in layers[left_size].clone() {
                for right in layers[right_size].clone() {
                    for candidate in [
                        RelationExpr::Add(Box::new(left.clone()), Box::new(right.clone())),
                        RelationExpr::Sub(Box::new(left.clone()), Box::new(right.clone())),
                        RelationExpr::Mul(Box::new(left.clone()), Box::new(right.clone())),
                    ] {
                        let structural_key = format!("{candidate:?}");
                        if seen.insert(structural_key) {
                            layers[size].push(candidate);
                        }
                    }
                }
            }
        }
    }
    layers
}

fn poly_pow(polynomial: &Polynomial, exponent: u8) -> Polynomial {
    (0..exponent).fold(
        Polynomial::from([([0; 5], Rational::integer(1))]),
        |acc, _| poly_mul(&acc, polynomial),
    )
}

/// Reduce a polynomial after independently expanding
/// `a(x-r0)(x-r1)=a*x^2+b*x+c`: b=-a(r0+r1), c=a*r0*r1.
fn reduce_by_factorization(polynomial: &Polynomial) -> Polynomial {
    let a = Polynomial::from([([1, 0, 0, 0, 0], Rational::integer(1))]);
    let root0 = Polynomial::from([([0, 0, 0, 1, 0], Rational::integer(1))]);
    let root1 = Polynomial::from([([0, 0, 0, 0, 1], Rational::integer(1))]);
    let b_image = poly_mul(&a, &poly_add(&root0, &root1, 1))
        .into_iter()
        .map(|(monomial, coefficient)| (monomial, coefficient * Rational::integer(-1)))
        .collect::<Polynomial>();
    let c_image = poly_mul(&a, &poly_mul(&root0, &root1));
    let mut result = Polynomial::new();
    for (monomial, coefficient) in polynomial {
        let base =
            Polynomial::from([([monomial[0], 0, 0, monomial[3], monomial[4]], *coefficient)]);
        let substituted = poly_mul(
            &poly_mul(&base, &poly_pow(&b_image, monomial[1])),
            &poly_pow(&c_image, monomial[2]),
        );
        result = poly_add(&result, &substituted, 1);
    }
    result
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FactorIdealCertificate {
    pub proposed_zero: RelationExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Theorem {
    UniversalQuadraticRootRelation(RelationExpr),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckError {
    Tautology,
    NotFactorizationConsequence,
}

pub fn check_factor_certificate(
    certificate: &FactorIdealCertificate,
    invariants: &[InventedInvariant],
) -> Result<Theorem, CheckError> {
    let polynomial = normalize_relation(&certificate.proposed_zero, invariants);
    if polynomial.is_empty() {
        return Err(CheckError::Tautology);
    }
    if !reduce_by_factorization(&polynomial).is_empty() {
        return Err(CheckError::NotFactorizationConsequence);
    }
    Ok(Theorem::UniversalQuadraticRootRelation(
        certificate.proposed_zero.clone(),
    ))
}

#[derive(Clone, Debug)]
pub struct QuadraticExample {
    coefficients: [Rational; 3],
    roots: RootTuple,
}

fn evaluate_relation(
    expr: &RelationExpr,
    invariants: &[InventedInvariant],
    example: &QuadraticExample,
) -> ComplexRational {
    match expr {
        RelationExpr::Coefficient(index) => {
            ComplexRational::new(example.coefficients[*index], Rational::integer(0))
        }
        RelationExpr::Invariant(index) => invariants[*index].program.eval(&example.roots),
        RelationExpr::Constant(value) => ComplexRational::real(*value),
        RelationExpr::Add(a, b) => {
            evaluate_relation(a, invariants, example) + evaluate_relation(b, invariants, example)
        }
        RelationExpr::Sub(a, b) => {
            evaluate_relation(a, invariants, example) - evaluate_relation(b, invariants, example)
        }
        RelationExpr::Mul(a, b) => {
            evaluate_relation(a, invariants, example) * evaluate_relation(b, invariants, example)
        }
    }
}

fn q(numerator: i128, denominator: i128) -> Rational {
    Rational::new(numerator, denominator)
}

fn real(numerator: i128, denominator: i128) -> ComplexRational {
    ComplexRational::new(q(numerator, denominator), q(0, 1))
}

fn gaussian(real_part: i128, imaginary_part: i128) -> ComplexRational {
    ComplexRational::new(q(real_part, 1), q(imaginary_part, 1))
}

fn example(
    a: i128,
    b: i128,
    c: i128,
    left: ComplexRational,
    right: ComplexRational,
) -> QuadraticExample {
    QuadraticExample {
        coefficients: [q(a, 1), q(b, 1), q(c, 1)],
        roots: RootTuple([left, right]),
    }
}

fn training_examples() -> Vec<QuadraticExample> {
    vec![
        example(1, -3, 2, real(1, 1), real(2, 1)),
        example(2, 5, -3, real(1, 2), real(-3, 1)),
        example(3, 12, 12, real(-2, 1), real(-2, 1)),
        example(2, 10, 8, real(-1, 1), real(-4, 1)),
        example(1, 0, 1, gaussian(0, 1), gaussian(0, -1)),
        example(5, -25, 30, real(2, 1), real(3, 1)),
    ]
}

fn held_out_examples() -> Vec<QuadraticExample> {
    vec![
        example(4, 2, -12, real(3, 2), real(-2, 1)),
        example(9, -6, 1, real(1, 3), real(1, 3)),
        example(2, -4, 4, gaussian(1, 1), gaussian(1, -1)),
        example(3, 3, -6, real(1, 1), real(-2, 1)),
        example(1, 7, 12, real(-3, 1), real(-4, 1)),
        example(
            8,
            0,
            2,
            ComplexRational::new(q(0, 1), q(1, 2)),
            ComplexRational::new(q(0, 1), q(-1, 2)),
        ),
    ]
}

#[derive(Clone, Debug)]
pub struct PolynomialDiscovery {
    pub invented_invariants: Vec<InventedInvariant>,
    pub retained_laws: Vec<FactorIdealCertificate>,
    pub root_programs_tested: usize,
    pub permutation_interventions: usize,
    pub relation_candidates_tested: usize,
    pub training_examples: usize,
    pub held_out_examples: usize,
    pub ordered_shortcuts_rejected: usize,
    pub modeled_baseline_future_cost: usize,
    pub modeled_retained_future_cost: usize,
    pub modeled_gain: usize,
}

pub fn m13_experiment() -> PolynomialDiscovery {
    let training = training_examples();
    let held_out = held_out_examples();
    let (invented_invariants, root_programs_tested, permutation_interventions) =
        discover_invariants(&training);

    let mut relation_candidates_tested = 0;
    let mut retained_laws = Vec::new();
    let mut retained_normals = BTreeSet::new();
    for layer in relation_layers(5, invented_invariants.len())
        .into_iter()
        .skip(1)
    {
        for candidate in layer {
            relation_candidates_tested += 1;
            if !candidate.uses_coefficient() || !candidate.uses_invariant() {
                continue;
            }
            if !training
                .iter()
                .all(|sample| evaluate_relation(&candidate, &invented_invariants, sample).is_zero())
            {
                continue;
            }
            let certificate = FactorIdealCertificate {
                proposed_zero: candidate,
            };
            if check_factor_certificate(&certificate, &invented_invariants).is_err()
                || !held_out.iter().all(|sample| {
                    evaluate_relation(&certificate.proposed_zero, &invented_invariants, sample)
                        .is_zero()
                })
            {
                continue;
            }
            let normal = normalize_relation(&certificate.proposed_zero, &invented_invariants);
            let coefficient_signature =
                normal.keys().fold([false; 3], |mut signature, monomial| {
                    for index in 0..3 {
                        signature[index] |= monomial[index] > 0;
                    }
                    signature
                });
            if retained_normals.insert(coefficient_signature) {
                retained_laws.push(certificate);
            }
        }
    }

    let ordered_shortcuts = [RootProgram::Slot(0), RootProgram::Slot(1)];
    let ordered_shortcuts_rejected = ordered_shortcuts
        .iter()
        .filter(|program| {
            let normal = normalize_root(program);
            swap_root_polynomial(&normal) != normal
                && training.iter().any(|sample| {
                    program.eval(&sample.roots) != program.eval(&sample.roots.swapped())
                })
        })
        .count();
    let modeled_baseline_future_cost = held_out.len() * relation_candidates_tested;
    let modeled_retained_future_cost = held_out.len() * retained_laws.len();
    PolynomialDiscovery {
        invented_invariants,
        retained_laws,
        root_programs_tested,
        permutation_interventions,
        relation_candidates_tested,
        training_examples: training.len(),
        held_out_examples: held_out.len(),
        ordered_shortcuts_rejected,
        modeled_baseline_future_cost,
        modeled_retained_future_cost,
        modeled_gain: modeled_baseline_future_cost.saturating_sub(modeled_retained_future_cost),
    }
}

pub fn machine_record(report: &PolynomialDiscovery) -> String {
    let invariants = report
        .invented_invariants
        .iter()
        .map(|invariant| invariant.program.render())
        .collect::<Vec<_>>()
        .join(";");
    let laws = report
        .retained_laws
        .iter()
        .map(|law| law.proposed_zero.render(&report.invented_invariants))
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "experiment=math_world_m13b,invented_invariants={},discovered_laws={},root_programs_tested={},permutation_interventions={},relation_candidates_tested={},training_examples={},held_out_examples={},orbit_primitives_supplied=false,symmetry_type_supplied=false,swap_action_supplied=true,invariance_objective_supplied=true,ordered_shortcuts_rejected={},exact_factor_ideal_check=true,modeled_baseline_future_cost={},modeled_retained_future_cost={},modeled_gain={},measured_downstream_gain=none,claim_level=L2_invented_feature_in_supplied_meta_ontology,proof_status=formally_checked_invented_permutation_invariants,deterministic=true,fallback=exact",
        invariants,
        laws,
        report.root_programs_tested,
        report.permutation_interventions,
        report.relation_candidates_tested,
        report.training_examples,
        report.held_out_examples,
        report.ordered_shortcuts_rejected,
        report.modeled_baseline_future_cost,
        report.modeled_retained_future_cost,
        report.modeled_gain,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invents_invariance_and_then_checked_coefficient_laws() {
        let report = m13_experiment();
        let invariant_polynomials = report
            .invented_invariants
            .iter()
            .map(|invariant| normalize_root(&invariant.program))
            .collect::<Vec<_>>();
        assert!(invariant_polynomials.contains(&RootPolynomial::from([
            ([1, 0], q(1, 1)),
            ([0, 1], q(1, 1)),
        ])));
        assert!(invariant_polynomials.contains(&RootPolynomial::from([([1, 1], q(1, 1))])));
        assert_eq!(report.retained_laws.len(), 2);
        assert!(report.retained_laws.iter().all(|certificate| {
            check_factor_certificate(certificate, &report.invented_invariants).is_ok()
        }));
        assert!(report.modeled_gain > 0);
    }

    #[test]
    fn raw_slots_are_observable_and_permutation_interventions_reject_them() {
        let sample = &training_examples()[0];
        assert_ne!(
            RootProgram::Slot(0).eval(&sample.roots),
            RootProgram::Slot(0).eval(&sample.roots.swapped())
        );
        let report = m13_experiment();
        assert_eq!(report.ordered_shortcuts_rejected, 2);
        assert!(report.permutation_interventions > 0);
        assert_eq!(
            check_invariant_program(&RootProgram::Slot(0)),
            Err(InvarianceError::ConstantOrUnary)
        );
        assert_eq!(
            check_invariant_program(&RootProgram::Add(
                Box::new(RootProgram::Slot(0)),
                Box::new(RootProgram::Constant(1)),
            )),
            Err(InvarianceError::ConstantOrUnary)
        );
    }

    #[test]
    fn checker_rejects_finite_fits_and_tautologies() {
        let report = m13_experiment();
        let fake = FactorIdealCertificate {
            proposed_zero: RelationExpr::Sub(
                Box::new(RelationExpr::Invariant(0)),
                Box::new(RelationExpr::Constant(3)),
            ),
        };
        assert_eq!(
            check_factor_certificate(&fake, &report.invented_invariants),
            Err(CheckError::NotFactorizationConsequence)
        );
        let tautology = FactorIdealCertificate {
            proposed_zero: RelationExpr::Sub(
                Box::new(RelationExpr::Coefficient(0)),
                Box::new(RelationExpr::Coefficient(0)),
            ),
        };
        assert_eq!(
            check_factor_certificate(&tautology, &report.invented_invariants),
            Err(CheckError::Tautology)
        );
    }

    #[test]
    fn diverse_exact_data_and_search_are_deterministic() {
        let all = training_examples()
            .into_iter()
            .chain(held_out_examples())
            .collect::<Vec<_>>();
        assert!(all.iter().any(|sample| sample.coefficients[0] != q(1, 1)));
        assert!(all
            .iter()
            .any(|sample| sample.roots.0[0] == sample.roots.0[1]));
        assert!(all
            .iter()
            .any(|sample| sample.roots.0.iter().any(|root| root.real.numerator < 0)));
        assert!(all.iter().any(|sample| sample
            .roots
            .0
            .iter()
            .any(|root| !root.imaginary.is_zero())));
        assert_eq!(
            machine_record(&m13_experiment()),
            machine_record(&m13_experiment())
        );
    }
}
