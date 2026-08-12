//! U4: discovery of a bounded recursive polynomial signature.
//!
//! Signature syntax is anonymous and exhaustively enumerated.  One uniform
//! interpreter derives observable variants, the action on morphisms, carrier
//! constructor interfaces, algebra interfaces, and the universal equation.
//! No branch recognizes a target signature.

use crate::{
    initial_algebra, nbe,
    term::{self, Term},
    transform,
    typed::{self, Atom, Type},
    universal,
};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::rc::Rc;

const M: u32 = 40;
const A: u32 = 41;
const P: u32 = 42;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Signature {
    Unit,
    Rec,
    Param(u8),
    Sum(Box<Signature>, Box<Signature>),
    Prod(Box<Signature>, Box<Signature>),
}

impl Signature {
    pub fn size(&self) -> u32 {
        match self {
            Self::Unit | Self::Rec | Self::Param(_) => 1,
            Self::Sum(a, b) | Self::Prod(a, b) => 1 + a.size() + b.size(),
        }
    }
    pub fn code(&self) -> String {
        match self {
            Self::Unit => "U".into(),
            Self::Rec => "R".into(),
            Self::Param(i) => format!("P{i}"),
            Self::Sum(a, b) => format!("S({},{})", a.code(), b.code()),
            Self::Prod(a, b) => format!("T({},{})", a.code(), b.code()),
        }
    }
}

pub fn signatures_exact(size: u32) -> Vec<Signature> {
    if size == 1 {
        return vec![Signature::Unit, Signature::Rec, Signature::Param(0)];
    }
    if size < 3 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for left_size in 1..=(size - 2) {
        let right_size = size - 1 - left_size;
        for left in signatures_exact(left_size) {
            for right in signatures_exact(right_size) {
                out.push(Signature::Sum(
                    Box::new(left.clone()),
                    Box::new(right.clone()),
                ));
                out.push(Signature::Prod(Box::new(left.clone()), Box::new(right)));
            }
        }
    }
    out
}
pub fn enumerate_signatures(max_size: u32) -> Vec<Signature> {
    (1..=max_size).flat_map(signatures_exact).collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Variant {
    pub params: u8,
    pub recursive: u8,
}
pub type SemanticProfile = Vec<Variant>;

pub fn semantic_profile(signature: &Signature) -> SemanticProfile {
    fn raw(s: &Signature) -> Vec<Variant> {
        match s {
            Signature::Unit => vec![Variant {
                params: 0,
                recursive: 0,
            }],
            Signature::Rec => vec![Variant {
                params: 0,
                recursive: 1,
            }],
            Signature::Param(_) => vec![Variant {
                params: 1,
                recursive: 0,
            }],
            Signature::Sum(a, b) => {
                let mut x = raw(a);
                x.extend(raw(b));
                x
            }
            Signature::Prod(a, b) => {
                let l = raw(a);
                let r = raw(b);
                l.iter()
                    .flat_map(|x| {
                        r.iter().map(move |y| Variant {
                            params: x.params + y.params,
                            recursive: x.recursive + y.recursive,
                        })
                    })
                    .collect()
            }
        }
    }
    let mut profile = raw(signature);
    profile.sort();
    profile
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShapeCurriculum {
    pub observed: Vec<Variant>,
    pub complete: bool,
}
pub fn signature_fits(signature: &Signature, curriculum: &ShapeCurriculum) -> bool {
    let profile = semantic_profile(signature);
    let observed = curriculum.observed.iter().copied().collect::<BTreeSet<_>>();
    let available = profile.iter().copied().collect::<BTreeSet<_>>();
    let mut required = curriculum.observed.clone();
    required.sort();
    observed.is_subset(&available) && (!curriculum.complete || required == profile)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignatureClass {
    pub profile: SemanticProfile,
    pub aliases: Vec<Signature>,
}
pub fn surviving_classes(
    candidates: &[Signature],
    curriculum: &ShapeCurriculum,
) -> Vec<SignatureClass> {
    let mut groups: BTreeMap<SemanticProfile, Vec<Signature>> = BTreeMap::new();
    for s in candidates.iter().filter(|s| signature_fits(s, curriculum)) {
        groups
            .entry(semantic_profile(s))
            .or_default()
            .push(s.clone());
    }
    groups
        .into_iter()
        .map(|(profile, mut aliases)| {
            aliases.sort_by_key(|s| (s.size(), s.code()));
            SignatureClass { profile, aliases }
        })
        .collect()
}

fn arrow(args: &[Type], result: Type) -> Type {
    args.iter()
        .rev()
        .fold(result, |out, arg| Type::arrow(arg.clone(), out))
}
fn handler_type(v: Variant, recursive: Type, result: Type) -> Type {
    let mut args = vec![Type::Atom(P); v.params as usize];
    args.extend(vec![recursive; v.recursive as usize]);
    arrow(&args, result)
}
fn layer_type(profile: &[Variant], recursive: Type, result: Type) -> Type {
    let handlers = profile
        .iter()
        .map(|v| handler_type(*v, recursive.clone(), result.clone()))
        .collect::<Vec<_>>();
    arrow(&handlers, result)
}
fn constructor_type(profile: &[Variant]) -> Type {
    let m = Type::Atom(M);
    Type::arrow(layer_type(profile, m.clone(), m.clone()), m)
}
fn generator_type(profile: &[Variant]) -> Type {
    let a = Type::Atom(A);
    let handlers = profile
        .iter()
        .map(|v| handler_type(*v, a.clone(), a.clone()))
        .collect::<Vec<_>>();
    let carrier = layer_type(profile, a.clone(), a.clone());
    arrow(&handlers, Type::arrow(carrier, a))
}

fn app(f: Rc<Term>, args: impl IntoIterator<Item = Rc<Term>>) -> Rc<Term> {
    args.into_iter().fold(f, term::app)
}
fn normalize(t: &Rc<Term>, fuel: i64) -> Option<Rc<Term>> {
    nbe::normalize(&Rc::new(Vec::new()), t, &mut nbe::Fuel(fuel)).ok()
}
fn equivalent(a: &Rc<Term>, b: &Rc<Term>, fuel: i64) -> bool {
    normalize(a, fuel)
        .zip(normalize(b, fuel))
        .is_some_and(|(a, b)| a == b)
}
fn leading_lambdas(t: &Rc<Term>) -> usize {
    let (mut c, mut n) = (t.as_ref(), 0);
    while let Term::Lam(b) = c {
        n += 1;
        c = b;
    }
    n
}
fn closed_terms(max: u32, lambdas: usize) -> Vec<Rc<Term>> {
    (1..=max)
        .flat_map(|n| universal::terms_exact(n, 0, &[]))
        .filter(|t| leading_lambdas(t) >= lambdas)
        .collect()
}
fn expand_prims(t: &Rc<Term>) -> Rc<Term> {
    match t.as_ref() {
        Term::Prim(b) => expand_prims(b),
        Term::Lam(b) => term::lam(expand_prims(b)),
        Term::App(f, a) => term::app(expand_prims(f), expand_prims(a)),
        Term::Var(_) | Term::Free(_) => t.clone(),
    }
}

fn canonical_constructor(profile: &[Variant], variant: usize) -> Rc<Term> {
    let params = profile[variant].params as usize;
    let recursive = profile[variant].recursive as usize;
    let fields = params + recursive;
    let mut body = term::var((profile.len() - 1 - variant) as u32);
    for index in 0..params {
        body = term::app(body, term::var((profile.len() + fields - 1 - index) as u32));
    }
    for index in 0..recursive {
        let child = term::var((profile.len() + recursive - 1 - index) as u32);
        let handlers = (0..profile.len())
            .map(|h| term::var((profile.len() - 1 - h) as u32))
            .collect::<Vec<_>>();
        body = term::app(body, app(child, handlers));
    }
    (0..profile.len() + fields).fold(body, |b, _| term::lam(b))
}

fn constructor_law(candidate: &Rc<Term>, profile: &[Variant], variant: usize, fuel: i64) -> bool {
    equivalent(candidate, &canonical_constructor(profile, variant), fuel)
}

fn constructor_candidates(
    profile: &[Variant],
    variant: usize,
    max_size: u32,
    fuel: i64,
) -> Vec<Rc<Term>> {
    let minimum =
        profile.len() + profile[variant].params as usize + profile[variant].recursive as usize;
    for candidate in closed_terms(max_size, minimum) {
        if constructor_law(&candidate, profile, variant, fuel) {
            return vec![candidate];
        }
    }
    Vec::new()
}

fn cartesian<T: Clone>(choices: &[Vec<T>]) -> Vec<Vec<T>> {
    let mut out = vec![Vec::new()];
    for choice in choices {
        let mut next = Vec::new();
        for prefix in &out {
            for item in choice {
                let mut p = prefix.clone();
                p.push(item.clone());
                next.push(p);
            }
        }
        out = next;
    }
    out
}

fn alpha_for(
    profile: &[Variant],
    constructors: &[Rc<Term>],
    max_size: u32,
    cap: usize,
    fuel: i64,
) -> (Option<Rc<Term>>, u64, bool) {
    let m = Type::Atom(M);
    let atoms = profile
        .iter()
        .zip(constructors)
        .map(|(v, c)| Atom {
            body: c.clone(),
            ty: handler_type(*v, m.clone(), m.clone()),
        })
        .collect::<Vec<_>>();
    let enumeration = typed::enumerate_closed(&constructor_type(profile), &atoms, max_size, cap);
    for candidate in &enumeration.terms {
        let expanded = expand_prims(candidate);
        let all = (0..profile.len()).all(|variant| {
            let fields = (0..profile[variant].recursive as u32)
                .map(|n| church_numeral(n + 2))
                .collect::<Vec<_>>();
            let layer = encode_layer(profile, variant, &[], &fields);
            let expected = app(constructors[variant].clone(), fields);
            equivalent(&term::app(expanded.clone(), layer), &expected, fuel)
        });
        if all {
            return (Some(expanded), enumeration.generated, enumeration.truncated);
        }
    }
    (None, enumeration.generated, enumeration.truncated)
}

fn encode_layer(
    profile: &[Variant],
    variant: usize,
    params: &[Rc<Term>],
    children: &[Rc<Term>],
) -> Rc<Term> {
    assert_eq!(params.len(), profile[variant].params as usize);
    assert_eq!(children.len(), profile[variant].recursive as usize);
    let selected = (profile.len() - 1 - variant) as u32;
    let mut body = term::var(selected);
    for field in params.iter().chain(children) {
        body = term::app(body, transform::shift(field, profile.len() as i32, 0));
    }
    (0..profile.len()).fold(body, |b, _| term::lam(b))
}

pub fn action_program(profile: &[Variant]) -> Rc<Term> {
    // λh.λlayer.λout0...outK. layer mapped0 ... mappedK
    let k = profile.len();
    let mut body = term::var(k as u32);
    for (i, v) in profile.iter().enumerate() {
        let fields = v.params as usize + v.recursive as usize;
        let mut mapped = term::var((fields + k - 1 - i) as u32);
        for p in 0..v.params as usize {
            mapped = term::app(mapped, term::var((fields - 1 - p) as u32));
        }
        for r in 0..v.recursive as usize {
            let child = term::var((v.recursive as usize - 1 - r) as u32);
            let h = term::var((fields + k + 1) as u32);
            mapped = term::app(mapped, term::app(h, child));
        }
        let handler = (0..fields).fold(mapped, |b, _| term::lam(b));
        body = term::app(body, handler);
    }
    (0..k + 2).fold(body, |b, _| term::lam(b))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tree {
    Node {
        variant: usize,
        params: Vec<Rc<Term>>,
        children: Vec<Tree>,
    },
}
fn carrier_value(constructors: &[Rc<Term>], tree: &Tree) -> Rc<Term> {
    match tree {
        Tree::Node {
            variant,
            params,
            children,
        } => app(
            constructors[*variant].clone(),
            params
                .iter()
                .cloned()
                .chain(children.iter().map(|c| carrier_value(constructors, c))),
        ),
    }
}
fn layer_for_tree(profile: &[Variant], constructors: &[Rc<Term>], tree: &Tree) -> Rc<Term> {
    match tree {
        Tree::Node {
            variant,
            params,
            children,
        } => encode_layer(
            profile,
            *variant,
            params,
            &children
                .iter()
                .map(|c| carrier_value(constructors, c))
                .collect::<Vec<_>>(),
        ),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResultEncoding {
    Boolean,
    Numeral,
    List,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Derivation {
    pub target: bool,
    pub output: bool,
    pub trace: bool,
    pub ancestors: BTreeSet<String>,
}
#[derive(Clone, Debug)]
pub struct AlgebraEvidence {
    pub id: String,
    pub group: String,
    pub result: ResultEncoding,
    pub handlers: Vec<Rc<Term>>,
    pub probes: Vec<(Tree, Rc<Term>)>,
    pub epoch: u64,
    pub derivation: Derivation,
    pub protected_annotation: i64,
}

fn algebra_term(e: &AlgebraEvidence) -> Rc<Term> {
    term::lam(app(term::var(0), e.handlers.clone()))
}
fn mediator(generator: &Rc<Term>, e: &AlgebraEvidence) -> Rc<Term> {
    app(generator.clone(), e.handlers.clone())
}

#[derive(Clone, Debug)]
pub struct Structure {
    pub signature_class: SignatureClass,
    pub constructors: Vec<Rc<Term>>,
    pub alpha: Rc<Term>,
    pub generator: Rc<Term>,
    pub action: Rc<Term>,
    pub fuel: i64,
    pub freeze: u64,
}

pub fn commutes(s: &Structure, e: &AlgebraEvidence, checks: &mut u64) -> bool {
    if e.probes.is_empty()
        || e.handlers.len() != s.signature_class.profile.len()
        || e.handlers.iter().any(|h| !transform::is_closed(h))
    {
        return false;
    }
    let h = mediator(&s.generator, e);
    let algebra = algebra_term(e);
    e.probes.iter().all(|(tree, expected)| {
        let layer = layer_for_tree(&s.signature_class.profile, &s.constructors, tree);
        let lhs = term::app(h.clone(), term::app(s.alpha.clone(), layer.clone()));
        let rhs = term::app(algebra.clone(), app(s.action.clone(), [h.clone(), layer]));
        *checks += 2;
        equivalent(&lhs, &rhs, s.fuel) && equivalent(&lhs, expected, s.fuel)
    })
}

#[derive(Clone, Debug)]
pub struct Spec {
    pub freeze: u64,
    pub signature_max_size: u32,
    pub constructor_max_size: u32,
    pub alpha_max_size: u32,
    pub generator_max_size: u32,
    pub mediator_max_size: u32,
    pub cap: usize,
    pub fuel: i64,
    pub complexity_price: u64,
    pub execution_price: u64,
}
pub fn default_spec() -> Spec {
    Spec {
        freeze: 1,
        signature_max_size: 5,
        constructor_max_size: 10,
        alpha_max_size: 8,
        generator_max_size: 10,
        mediator_max_size: 10,
        cap: 50_000,
        fuel: 2_000_000,
        complexity_price: 10,
        execution_price: 1,
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Accounting {
    pub signature_candidates: u64,
    pub action_evaluations: u64,
    pub signature_semantic_classes: u64,
    pub surviving_signatures: u64,
    pub carrier_candidates: u64,
    pub constructor_candidates: u64,
    pub generator_candidates: u64,
    pub mediator_candidates: u64,
    pub equation_checks: u64,
    pub equivalence_checks: u64,
    pub signature_truncated: bool,
    pub constructor_truncated: bool,
    pub generator_truncated: bool,
    pub mediator_truncated: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Termination {
    Discovered,
    Ambiguous,
    Exhausted,
    Truncated,
    InvalidEvidence,
}
#[derive(Clone, Debug)]
pub struct DiscoveryReport {
    pub structure: Option<Structure>,
    pub weak_classes: Vec<SignatureClass>,
    pub rich_classes: Vec<SignatureClass>,
    pub accounting: Accounting,
    pub weak_identifiable: bool,
    pub rich_identifiable: bool,
    pub charged_discovery_cost: u64,
    pub termination: Termination,
}

pub fn candidate_order(report: &DiscoveryReport) -> Vec<String> {
    report
        .rich_classes
        .iter()
        .flat_map(|class| class.aliases.iter().map(Signature::code))
        .collect()
}

fn visible<'a>(
    records: &'a [AlgebraEvidence],
    freeze: u64,
    protected: &BTreeSet<String>,
) -> Vec<&'a AlgebraEvidence> {
    records
        .iter()
        .filter(|e| {
            e.epoch <= freeze
                && !e.derivation.target
                && !e.derivation.output
                && !e.derivation.trace
                && e.derivation.ancestors.is_empty()
                && !protected.contains(&e.id)
                && protected.iter().all(|id| e.group != *id)
        })
        .collect()
}

fn discover_structure(
    class: &SignatureClass,
    evidence: &[&AlgebraEvidence],
    spec: &Spec,
    accounting: &mut Accounting,
) -> Option<Structure> {
    let profile = &class.profile;
    let choices = profile
        .iter()
        .enumerate()
        .map(|(i, _)| constructor_candidates(profile, i, spec.constructor_max_size, spec.fuel))
        .collect::<Vec<_>>();
    accounting.carrier_candidates += choices.iter().map(|x| x.len() as u64).sum::<u64>();
    if choices.iter().any(Vec::is_empty) {
        return None;
    }
    let generator_enum = typed::enumerate_closed(
        &generator_type(profile),
        &[],
        spec.generator_max_size,
        spec.cap,
    );
    accounting.generator_candidates += generator_enum.generated;
    accounting.generator_truncated |= generator_enum.truncated;
    for constructors in cartesian(&choices) {
        accounting.constructor_candidates += 1;
        let (alpha, generated, truncated) = alpha_for(
            profile,
            &constructors,
            spec.alpha_max_size,
            spec.cap,
            spec.fuel,
        );
        accounting.constructor_candidates += generated;
        accounting.constructor_truncated |= truncated;
        let Some(alpha) = alpha else { continue };
        for generator in &generator_enum.terms {
            let candidate = Structure {
                signature_class: class.clone(),
                constructors: constructors.clone(),
                alpha: alpha.clone(),
                generator: generator.clone(),
                action: action_program(profile),
                fuel: spec.fuel,
                freeze: spec.freeze,
            };
            if evidence
                .iter()
                .all(|e| commutes(&candidate, e, &mut accounting.equation_checks))
            {
                let unique = evidence.iter().all(|e| {
                    let u = bounded_uniqueness(&candidate, e, spec);
                    accounting.mediator_candidates += u.generated;
                    accounting.equivalence_checks += u.checks;
                    accounting.mediator_truncated |= !u.exhaustive;
                    u.unique
                });
                if unique {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

pub fn discover(
    training: &[AlgebraEvidence],
    calibration: &[AlgebraEvidence],
    protected: &BTreeSet<String>,
    weak: &ShapeCurriculum,
    rich: &ShapeCurriculum,
    spec: &Spec,
) -> DiscoveryReport {
    let candidates = enumerate_signatures(spec.signature_max_size);
    let mut accounting = Accounting {
        signature_candidates: candidates.len() as u64,
        ..Default::default()
    };
    for signature in &candidates {
        let action = action_program(&semantic_profile(signature));
        accounting.action_evaluations += 1;
        if !transform::is_closed(&action) {
            accounting.signature_truncated = true;
        }
    }
    let weak_classes = surviving_classes(&candidates, weak);
    let rich_classes = surviving_classes(&candidates, rich);
    accounting.signature_semantic_classes = rich_classes.len() as u64;
    accounting.surviving_signatures = rich_classes.iter().map(|c| c.aliases.len() as u64).sum();
    let visible = visible(training, spec.freeze, protected)
        .into_iter()
        .chain(visible(calibration, spec.freeze, protected))
        .collect::<Vec<_>>();
    if visible.is_empty() {
        return DiscoveryReport {
            structure: None,
            weak_identifiable: weak_classes.len() == 1,
            rich_identifiable: rich_classes.len() == 1,
            weak_classes,
            rich_classes,
            accounting,
            charged_discovery_cost: u64::MAX,
            termination: Termination::InvalidEvidence,
        };
    }
    let encodings = visible.iter().map(|e| e.result).collect::<BTreeSet<_>>();
    if encodings.len() < 3 || visible.iter().any(|e| e.probes.is_empty()) {
        return DiscoveryReport {
            structure: None,
            weak_identifiable: weak_classes.len() == 1,
            rich_identifiable: rich_classes.len() == 1,
            weak_classes,
            rich_classes,
            accounting,
            charged_discovery_cost: u64::MAX,
            termination: Termination::InvalidEvidence,
        };
    }
    if rich_classes.len() != 1 {
        return DiscoveryReport {
            structure: None,
            weak_identifiable: weak_classes.len() == 1,
            rich_identifiable: false,
            weak_classes,
            rich_classes,
            accounting,
            charged_discovery_cost: u64::MAX,
            termination: Termination::Ambiguous,
        };
    }
    let mut survivors = Vec::new();
    for class in &rich_classes {
        if let Some(s) = discover_structure(class, &visible, spec, &mut accounting) {
            survivors.push(s);
        }
    }
    let mut structure = (survivors.len() == 1).then(|| survivors.remove(0));
    let truncated = accounting.signature_truncated
        || accounting.constructor_truncated
        || accounting.generator_truncated
        || accounting.mediator_truncated;
    let termination = if truncated {
        structure = None;
        Termination::Truncated
    } else if structure.is_some() {
        Termination::Discovered
    } else if survivors.len() > 1 || rich_classes.len() > 1 {
        Termination::Ambiguous
    } else {
        Termination::Exhausted
    };
    let charged_discovery_cost = structure.as_ref().map_or(u64::MAX, |s| {
        let complexity = s.signature_class.aliases[0].size()
            + s.constructors.iter().map(|x| x.size()).sum::<u32>()
            + s.alpha.size()
            + s.generator.size()
            + s.action.size();
        u64::from(complexity) * spec.complexity_price
            + (accounting.action_evaluations
                + accounting.equation_checks
                + accounting.equivalence_checks)
                * spec.execution_price
    });
    DiscoveryReport {
        structure,
        weak_identifiable: weak_classes.len() == 1,
        rich_identifiable: rich_classes.len() == 1,
        weak_classes,
        rich_classes,
        accounting,
        charged_discovery_cost,
        termination,
    }
}

fn as_supplied_structure(s: &Structure) -> Option<initial_algebra::InitialStructure> {
    (s.signature_class.profile
        == vec![
            Variant {
                params: 0,
                recursive: 0,
            },
            Variant {
                params: 0,
                recursive: 1,
            },
        ])
    .then(|| initial_algebra::InitialStructure {
        carrier_witness: s.constructors[0].clone(),
        carrier_step: s.constructors[1].clone(),
        constructor: s.alpha.clone(),
        generator: s.generator.clone(),
        f_action: s.action.clone(),
        freeze_epoch: s.freeze,
        observational_fuel: s.fuel,
        mediator_boundary: 10,
    })
}

#[derive(Clone, Debug)]
pub struct Economics {
    pub learned: initial_algebra::SearchMeasurement,
    pub uniform: initial_algebra::SearchMeasurement,
    pub oracle: initial_algebra::SearchMeasurement,
    pub irrelevant: initial_algebra::SearchMeasurement,
    pub supplied_f: initial_algebra::SearchMeasurement,
    pub universal: initial_algebra::SearchMeasurement,
    pub reuse_horizon: u64,
    pub net_gain: i128,
}

pub fn measure_economics(
    structure: &Structure,
    discovery_charge: u64,
    reuse_horizon: u64,
) -> Option<Economics> {
    let supplied = as_supplied_structure(structure)?;
    let task = initial_algebra::DownstreamTask::DoubleCarrier;
    let learned = initial_algebra::measure_downstream(task, &supplied, true, 16, 50_000);
    let uniform = initial_algebra::measure_uniform(task, &supplied, 16, 50_000);
    let oracle = initial_algebra::measure_oracle(task, &supplied);
    let irrelevant = initial_algebra::measure_irrelevant(task, &supplied, 16, 50_000);
    let universal = initial_algebra::measure_pure_universal(task, &supplied, 8);
    // Once F is externally supplied, the installed downstream proposal language
    // is identical. The extra U4 price is therefore carried only by discovery_charge.
    let supplied_f = learned.clone();
    let per_use = i128::from(uniform.proposals) - i128::from(learned.proposals);
    Some(Economics {
        learned,
        uniform,
        oracle,
        irrelevant,
        supplied_f,
        universal,
        reuse_horizon,
        net_gain: per_use * i128::from(reuse_horizon) - i128::from(discovery_charge),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UniquenessReport {
    pub valid: usize,
    pub classes: usize,
    pub generated: u64,
    pub checks: u64,
    pub exhaustive: bool,
    pub unique: bool,
}
pub fn bounded_uniqueness(s: &Structure, e: &AlgebraEvidence, spec: &Spec) -> UniquenessReport {
    let a = Type::Atom(A);
    let atoms = e
        .handlers
        .iter()
        .zip(&s.signature_class.profile)
        .map(|(h, v)| Atom {
            body: h.clone(),
            ty: handler_type(*v, a.clone(), a.clone()),
        })
        .collect::<Vec<_>>();
    let carrier = layer_type(&s.signature_class.profile, a.clone(), a.clone());
    let enumeration = typed::enumerate_closed(
        &Type::arrow(carrier, a),
        &atoms,
        spec.mediator_max_size,
        spec.cap,
    );
    let mut valid = 0;
    let mut classes = HashSet::new();
    let mut checks = 0;
    fn collect<'a>(tree: &'a Tree, out: &mut Vec<&'a Tree>) {
        if out.iter().any(|seen| *seen == tree) {
            return;
        }
        out.push(tree);
        let Tree::Node { children, .. } = tree;
        for child in children {
            collect(child, out);
        }
    }
    let mut equation_trees = Vec::new();
    for (tree, _) in &e.probes {
        collect(tree, &mut equation_trees);
    }
    for candidate in &enumeration.terms {
        let holds = equation_trees.iter().all(|tree| {
            let layer = layer_for_tree(&s.signature_class.profile, &s.constructors, tree);
            let lhs = term::app(candidate.clone(), term::app(s.alpha.clone(), layer.clone()));
            let rhs = term::app(
                algebra_term(e),
                app(s.action.clone(), [candidate.clone(), layer]),
            );
            checks += 1;
            equivalent(&lhs, &rhs, spec.fuel)
        });
        if holds {
            valid += 1;
            let obs = e
                .probes
                .iter()
                .map(|(tree, _)| {
                    normalize(
                        &term::app(candidate.clone(), carrier_value(&s.constructors, tree)),
                        spec.fuel,
                    )
                })
                .collect::<Option<Vec<_>>>();
            if let Some(v) = obs {
                classes.insert(
                    v.into_iter()
                        .map(|x| x.as_ref().clone())
                        .collect::<Vec<_>>(),
                );
            }
        }
    }
    UniquenessReport {
        valid,
        classes: classes.len(),
        generated: enumeration.generated,
        checks,
        exhaustive: !enumeration.truncated,
        unique: valid > 0 && classes.len() == 1 && !enumeration.truncated,
    }
}

fn bool_term(v: bool) -> Rc<Term> {
    if v {
        term::lam(term::lam(term::var(1)))
    } else {
        term::lam(term::lam(term::var(0)))
    }
}
fn boolean_not() -> Rc<Term> {
    term::lam(app(term::var(0), [bool_term(false), bool_term(true)]))
}
fn church_numeral(n: u32) -> Rc<Term> {
    let body = (0..n).fold(term::var(0), |b, _| term::app(term::var(1), b));
    term::lam(term::lam(body))
}
fn successor() -> Rc<Term> {
    term::lam(term::lam(term::lam(term::app(
        term::var(1),
        app(term::var(2), [term::var(1), term::var(0)]),
    ))))
}
fn church_list(values: &[u32]) -> Rc<Term> {
    let body = values.iter().rev().fold(term::var(0), |tail, v| {
        app(term::var(1), [church_numeral(*v), tail])
    });
    term::lam(term::lam(body))
}
fn prepend_one() -> Rc<Term> {
    term::lam(term::lam(term::lam(app(
        term::var(1),
        [
            church_numeral(1),
            app(term::var(2), [term::var(1), term::var(0)]),
        ],
    ))))
}
fn numeral_add() -> Rc<Term> {
    // λm.λn.λf.λx.m f (n f x)
    (0..4).fold(
        app(
            term::var(3),
            [
                term::var(1),
                app(term::var(2), [term::var(1), term::var(0)]),
            ],
        ),
        |b, _| term::lam(b),
    )
}
fn boolean_xor() -> Rc<Term> {
    // λa.λb.a (not b) b
    term::lam(term::lam(app(
        term::var(1),
        [term::app(boolean_not(), term::var(0)), term::var(0)],
    )))
}
fn list_append() -> Rc<Term> {
    // λxs.λys.λc.λn.xs c (ys c n)
    (0..4).fold(
        app(
            term::var(3),
            [
                term::var(1),
                app(term::var(2), [term::var(1), term::var(0)]),
            ],
        ),
        |b, _| term::lam(b),
    )
}
fn chain(depth: u32) -> Tree {
    (0..depth).fold(
        Tree::Node {
            variant: 0,
            params: vec![],
            children: vec![],
        },
        |child, _| Tree::Node {
            variant: 1,
            params: vec![],
            children: vec![child],
        },
    )
}

fn binary_comb(depth: u32) -> Tree {
    let leaf = || Tree::Node {
        variant: 0,
        params: vec![],
        children: vec![],
    };
    (0..depth).fold(leaf(), |left, _| Tree::Node {
        variant: 1,
        params: vec![],
        children: vec![left, leaf()],
    })
}

pub fn curricula() -> (ShapeCurriculum, ShapeCurriculum) {
    (
        ShapeCurriculum {
            observed: vec![Variant {
                params: 0,
                recursive: 0,
            }],
            complete: false,
        },
        ShapeCurriculum {
            observed: vec![
                Variant {
                    params: 0,
                    recursive: 0,
                },
                Variant {
                    params: 0,
                    recursive: 1,
                },
            ],
            complete: true,
        },
    )
}
pub fn sample_evidence() -> (
    Vec<AlgebraEvidence>,
    Vec<AlgebraEvidence>,
    Vec<AlgebraEvidence>,
) {
    let make = |id: &str,
                result,
                handlers: Vec<Rc<Term>>,
                depths: Vec<u32>,
                expected: fn(u32) -> Rc<Term>| AlgebraEvidence {
        id: id.into(),
        group: id.into(),
        result,
        handlers,
        probes: depths
            .into_iter()
            .map(|d| (chain(d), expected(d)))
            .collect(),
        epoch: 1,
        derivation: Derivation::default(),
        protected_annotation: 0,
    };
    let even = |d| bool_term(d % 2 == 0);
    let count = church_numeral;
    let rebuild = |d| church_list(&vec![1; d as usize]);
    let odd = |d| bool_term(d % 2 == 1);
    let double = |d| church_numeral(d * 2);
    let training = vec![
        make(
            "train-even",
            ResultEncoding::Boolean,
            vec![bool_term(true), boolean_not()],
            vec![0, 1, 2, 3],
            even,
        ),
        make(
            "train-count",
            ResultEncoding::Numeral,
            vec![church_numeral(0), successor()],
            vec![0, 1, 2, 3],
            count,
        ),
    ];
    let calibration = vec![make(
        "cal-list",
        ResultEncoding::List,
        vec![church_list(&[]), prepend_one()],
        vec![0, 1, 2, 3, 4],
        rebuild,
    )];
    let protected = vec![
        make(
            "held-odd",
            ResultEncoding::Boolean,
            vec![bool_term(false), boolean_not()],
            vec![5, 7, 9],
            odd,
        ),
        make(
            "held-double",
            ResultEncoding::Numeral,
            vec![
                church_numeral(0),
                term::lam(term::app(successor(), term::app(successor(), term::var(0)))),
            ],
            vec![5, 7, 9],
            double,
        ),
    ];
    (training, calibration, protected)
}

pub fn binary_curricula() -> (ShapeCurriculum, ShapeCurriculum) {
    (
        ShapeCurriculum {
            observed: vec![Variant {
                params: 0,
                recursive: 0,
            }],
            complete: false,
        },
        ShapeCurriculum {
            observed: vec![
                Variant {
                    params: 0,
                    recursive: 0,
                },
                Variant {
                    params: 0,
                    recursive: 2,
                },
            ],
            complete: true,
        },
    )
}

pub fn binary_evidence() -> (
    Vec<AlgebraEvidence>,
    Vec<AlgebraEvidence>,
    Vec<AlgebraEvidence>,
) {
    let make = |id: &str,
                result,
                handlers: Vec<Rc<Term>>,
                depths: Vec<u32>,
                expected: fn(u32) -> Rc<Term>| AlgebraEvidence {
        id: id.into(),
        group: id.into(),
        result,
        handlers,
        probes: depths
            .into_iter()
            .map(|d| (binary_comb(d), expected(d)))
            .collect(),
        epoch: 1,
        derivation: Derivation::default(),
        protected_annotation: 0,
    };
    let leaves = |d| church_numeral(d + 1);
    let parity = |d| bool_term((d + 1) % 2 == 1);
    let rebuild = |d| church_list(&vec![1; (d + 1) as usize]);
    let training = vec![
        make(
            "binary-count",
            ResultEncoding::Numeral,
            vec![church_numeral(1), numeral_add()],
            vec![0, 1, 2, 3],
            leaves,
        ),
        make(
            "binary-parity",
            ResultEncoding::Boolean,
            vec![bool_term(true), boolean_xor()],
            vec![0, 1, 2, 3],
            parity,
        ),
    ];
    let calibration = vec![make(
        "binary-list",
        ResultEncoding::List,
        vec![church_list(&[1]), list_append()],
        vec![0, 1, 2, 3, 4],
        rebuild,
    )];
    let protected = vec![make(
        "binary-held-count",
        ResultEncoding::Numeral,
        vec![church_numeral(1), numeral_add()],
        vec![5, 7],
        leaves,
    )];
    (training, calibration, protected)
}

#[derive(Clone, Debug)]
pub struct ExperimentReport {
    pub discovery: DiscoveryReport,
    pub protected_commutes: bool,
    pub protected_unique: bool,
    pub uniqueness_exhaustive: bool,
    pub protected_mediator_candidates: u64,
    pub protected_equivalence_checks: u64,
    pub economics: Economics,
}

pub fn run_experiment() -> ExperimentReport {
    let (training, calibration, protected) = sample_evidence();
    let protected_ids = protected.iter().map(|e| e.id.clone()).collect();
    let (weak, rich) = curricula();
    let spec = default_spec();
    let discovery = discover(&training, &calibration, &protected_ids, &weak, &rich, &spec);
    let structure = discovery
        .structure
        .as_ref()
        .expect("declared U4 experiment");
    let mut equation_checks = 0;
    let protected_commutes = protected
        .iter()
        .all(|e| commutes(structure, e, &mut equation_checks));
    let uniqueness = protected
        .iter()
        .map(|e| bounded_uniqueness(structure, e, &spec))
        .collect::<Vec<_>>();
    let protected_unique = uniqueness.iter().all(|u| u.unique);
    let uniqueness_exhaustive = uniqueness.iter().all(|u| u.exhaustive);
    let protected_mediator_candidates = uniqueness.iter().map(|u| u.generated).sum();
    let protected_equivalence_checks = uniqueness.iter().map(|u| u.checks).sum();
    let economics = measure_economics(structure, discovery.charged_discovery_cost, 10_000)
        .expect("unary experiment economics");
    ExperimentReport {
        discovery,
        protected_commutes,
        protected_unique,
        uniqueness_exhaustive,
        protected_mediator_candidates,
        protected_equivalence_checks,
        economics,
    }
}

fn yes(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

pub fn machine_record(report: &ExperimentReport) -> String {
    let d = &report.discovery;
    let a = &d.accounting;
    let selected_size = d
        .structure
        .as_ref()
        .map(|s| s.signature_class.aliases[0].size())
        .unwrap_or(0);
    format!(
        "record,experiment=u4,signature_candidates={},signature_semantic_classes={},surviving_signatures={},selected_signature_size={},carrier_candidates={},constructor_candidates={},generator_candidates={},mediator_candidates={},equation_checks={},equivalence_checks={},uniqueness_exhaustive={},weak_identifiable={},rich_identifiable={},protected_commutes={},protected_unique={},learned_proposals={},uniform_proposals={},oracle_proposals={},supplied_f_proposals={},irrelevant_proposals={},universal_proposals={},discovery_charge={},reuse_horizon={},net_gain={},termination={:?}",
        a.signature_candidates,
        a.signature_semantic_classes,
        a.surviving_signatures,
        selected_size,
        a.carrier_candidates,
        a.constructor_candidates,
        a.generator_candidates,
        a.mediator_candidates + report.protected_mediator_candidates,
        a.equation_checks,
        a.equivalence_checks + report.protected_equivalence_checks,
        yes(report.uniqueness_exhaustive),
        yes(d.weak_identifiable),
        yes(d.rich_identifiable),
        yes(report.protected_commutes),
        yes(report.protected_unique),
        report.economics.learned.proposals,
        report.economics.uniform.proposals,
        report.economics.oracle.proposals,
        report.economics.supplied_f.proposals,
        report.economics.irrelevant.proposals,
        report.economics.universal.proposals,
        d.charged_discovery_cost,
        report.economics.reuse_horizon,
        report.economics.net_gain,
        d.termination,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universal::{Dovetail, InterleavedDovetail, ResourceLane};
    #[test]
    fn exact_enumeration_and_identifiability() {
        let candidates = enumerate_signatures(5);
        assert_eq!(
            signatures_exact(1),
            vec![Signature::Unit, Signature::Rec, Signature::Param(0)]
        );
        assert!(signatures_exact(2).is_empty());
        assert!(candidates.windows(2).all(|w| w[0].size() <= w[1].size()));
        let (weak, rich) = curricula();
        let wc = surviving_classes(&candidates, &weak);
        let rc = surviving_classes(&candidates, &rich);
        assert!(wc.len() > 1);
        assert_eq!(rc.len(), 1);
        assert_eq!(
            rc[0].profile,
            vec![
                Variant {
                    params: 0,
                    recursive: 0
                },
                Variant {
                    params: 0,
                    recursive: 1
                }
            ]
        );
        assert!(rc[0].aliases.len() > 1);
        for required in [
            Signature::Unit,
            Signature::Rec,
            Signature::Sum(Box::new(Signature::Unit), Box::new(Signature::Rec)),
            Signature::Sum(
                Box::new(Signature::Unit),
                Box::new(Signature::Prod(
                    Box::new(Signature::Rec),
                    Box::new(Signature::Rec),
                )),
            ),
            Signature::Sum(
                Box::new(Signature::Unit),
                Box::new(Signature::Prod(
                    Box::new(Signature::Param(0)),
                    Box::new(Signature::Rec),
                )),
            ),
        ] {
            assert!(
                candidates.contains(&required),
                "missing {}",
                required.code()
            );
        }
    }
    #[test]
    fn discovers_signature_and_structure() {
        let (t, c, p) = sample_evidence();
        let ids = p.iter().map(|e| e.id.clone()).collect();
        let (w, r) = curricula();
        let spec = default_spec();
        let report = discover(&t, &c, &ids, &w, &r, &spec);
        let s = report.structure.as_ref().expect("U4 discovery");
        assert!(!report.weak_identifiable && report.rich_identifiable);
        for e in &p {
            let mut checks = 0;
            assert!(commutes(s, e, &mut checks));
            let u = bounded_uniqueness(s, e, &spec);
            assert!(u.unique && u.exhaustive);
        }
    }

    #[test]
    fn weak_evidence_reports_ambiguity_and_wrong_arities_fail() {
        let candidates = enumerate_signatures(5);
        let (weak, rich) = curricula();
        assert!(surviving_classes(&candidates, &weak).len() > 1);
        let binary = vec![
            Variant {
                params: 0,
                recursive: 0,
            },
            Variant {
                params: 0,
                recursive: 2,
            },
        ];
        let no_recursive = vec![Variant {
            params: 0,
            recursive: 0,
        }];
        assert!(!surviving_classes(&candidates, &rich)
            .iter()
            .any(|class| class.profile == binary || class.profile == no_recursive));
        let (t, c, p) = sample_evidence();
        let ids = p.iter().map(|e| e.id.clone()).collect();
        let report = discover(&t, &c, &ids, &weak, &weak, &default_spec());
        assert_eq!(report.termination, Termination::Ambiguous);
        assert!(report.structure.is_none() && report.rich_classes.len() > 1);

        // The generic action observes both recursive fields; the binary profile
        // cannot pass by silently dropping one branch.
        let binary_layer = encode_layer(&binary, 1, &[], &[church_numeral(1), church_numeral(2)]);
        let mapped = app(action_program(&binary), [successor(), binary_layer]);
        let observed = app(mapped, [church_numeral(0), numeral_add()]);
        assert!(equivalent(&observed, &church_numeral(5), 100_000));
    }

    #[test]
    fn disconnected_hidden_state_preserves_existence_but_breaks_uniqueness() {
        let pair = |n: Rc<Term>, tag: Rc<Term>| {
            term::lam(term::app(
                term::app(term::var(0), transform::shift(&n, 1, 0)),
                transform::shift(&tag, 1, 0),
            ))
        };
        let first = term::lam(term::app(term::var(0), term::lam(term::lam(term::var(1)))));
        let second = term::lam(term::app(term::var(0), term::lam(term::lam(term::var(0)))));
        let zero = pair(church_numeral(0), bool_term(false));
        let disconnected = pair(church_numeral(0), bool_term(true));
        let step = term::lam(pair(
            term::app(successor(), term::app(first.clone(), term::var(0))),
            term::app(second.clone(), term::var(0)),
        ));
        let h0 = first.clone();
        let h1 = term::lam(app(
            term::app(second.clone(), term::var(0)),
            [
                term::app(successor(), term::app(first.clone(), term::var(0))),
                term::app(first.clone(), term::var(0)),
            ],
        ));
        for h in [&h0, &h1] {
            let mut value = zero.clone();
            for depth in 0..=5 {
                assert!(equivalent(
                    &term::app(h.clone(), value.clone()),
                    &church_numeral(depth),
                    500_000,
                ));
                value = term::app(step.clone(), value);
            }
            assert!(equivalent(
                &term::app(h.clone(), term::app(step.clone(), disconnected.clone())),
                &term::app(successor(), term::app(h.clone(), disconnected.clone())),
                500_000,
            ));
        }
        assert!(!equivalent(
            &term::app(h0, disconnected.clone()),
            &term::app(h1, disconnected),
            100_000,
        ));
    }

    #[test]
    fn leakage_truncation_and_fallback_are_invariant() {
        let (training, calibration, protected) = sample_evidence();
        let ids = protected.iter().map(|e| e.id.clone()).collect();
        let (weak, rich) = curricula();
        let spec = default_spec();
        let clean = discover(&training, &calibration, &ids, &weak, &rich, &spec);
        let mut poisoned = training.clone();
        for (index, mode) in (0..5).enumerate() {
            let mut e = protected[0].clone();
            e.id = format!("poison-{index}");
            match mode {
                0 => e.derivation.target = true,
                1 => e.derivation.output = true,
                2 => e.derivation.trace = true,
                3 => {
                    e.derivation.ancestors.insert("held".into());
                }
                _ => e.epoch = spec.freeze + 1,
            }
            poisoned.push(e);
        }
        let contaminated = discover(&poisoned, &calibration, &ids, &weak, &rich, &spec);
        assert_eq!(candidate_order(&clean), candidate_order(&contaminated));
        assert_eq!(clean.accounting, contaminated.accounting);
        let a = clean.structure.as_ref().unwrap();
        let b = contaminated.structure.as_ref().unwrap();
        assert_eq!(
            (&a.constructors, &a.alpha, &a.generator),
            (&b.constructors, &b.alpha, &b.generator)
        );
        let mut mutated = protected.clone();
        for e in &mut mutated {
            e.protected_annotation = i64::MAX;
        }
        let mutated_ids = mutated.iter().map(|e| e.id.clone()).collect();
        let unchanged = discover(&training, &calibration, &mutated_ids, &weak, &rich, &spec);
        assert_eq!(clean.accounting, unchanged.accounting);

        let mut tiny = spec.clone();
        tiny.cap = 1;
        let truncated = discover(&training, &calibration, &ids, &weak, &rich, &tiny);
        assert_eq!(truncated.termination, Termination::Truncated);
        assert!(truncated.structure.is_none());

        let mut schedule = InterleavedDovetail::new((0..64).map(|i| ((i % 7 + 1) as u32, i + 1)));
        let mut projection = Vec::new();
        while projection.len() < 128 {
            let point = schedule.next_labeled().unwrap();
            if point.lane == ResourceLane::Universal {
                projection.push((point.syntax_size, point.evaluation_fuel));
            }
        }
        assert_eq!(
            projection,
            Dovetail::default().take(128).collect::<Vec<_>>()
        );
    }

    #[test]
    fn machine_report_and_economics_are_complete() {
        let report = run_experiment();
        let record = machine_record(&report);
        for field in [
            "signature_candidates=",
            "signature_semantic_classes=",
            "surviving_signatures=",
            "selected_signature_size=",
            "carrier_candidates=",
            "constructor_candidates=",
            "generator_candidates=",
            "mediator_candidates=",
            "equation_checks=",
            "equivalence_checks=",
            "uniqueness_exhaustive=",
            "weak_identifiable=",
            "rich_identifiable=",
            "protected_commutes=",
            "protected_unique=",
            "learned_proposals=",
            "uniform_proposals=",
            "oracle_proposals=",
            "supplied_f_proposals=",
            "universal_proposals=",
            "discovery_charge=",
            "reuse_horizon=",
            "net_gain=",
            "termination=",
        ] {
            assert!(record.contains(field), "missing {field}");
        }
        assert!(report.economics.learned.solved && report.economics.uniform.solved);
        assert!(report.economics.oracle.solved);
    }

    #[test]
    fn identical_signature_machinery_switches_to_binary_shape() {
        let (_, rich) = binary_curricula();
        let classes = surviving_classes(&enumerate_signatures(5), &rich);
        assert_eq!(classes.len(), 1);
        let structure = &classes[0];
        assert_eq!(
            structure.profile,
            vec![
                Variant {
                    params: 0,
                    recursive: 0
                },
                Variant {
                    params: 0,
                    recursive: 2
                },
            ]
        );
        assert_ne!(
            structure.profile,
            surviving_classes(&enumerate_signatures(5), &curricula().1)[0].profile
        );
        assert!(transform::is_closed(&action_program(&structure.profile)));
    }
}
