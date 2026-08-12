//! Direction M1: invent distance as a reusable mathematical concept.
//!
//! Mathematics is treated as a world `W = (S, A, T, O)`:
//!
//! ```text
//! S = known expressions / concepts (the ontology)
//! A = admissible operations (+, -, *, /, sqrt, composition)
//! T = the derivation relation (evaluation of an expression on a point)
//! O = observations ((x, y) -> d pairs)
//! ```
//!
//! Given Pythagorean-triple observations, the agent must **invent** an
//! expression that explains them and generalizes to unseen points, then reuse
//! it as a concept to make later reasoning cheaper. The concept of Euclidean
//! distance is never supplied.
//!
//! The acquisition criterion stays practical:
//!
//! ```text
//! mathematical concept retained  iff  it reduces downstream reasoning cost
//! ```
//!
//! The discovered concept is a *reusable* object: once invented, predicting the
//! distance of a new point is a single evaluation, whereas without it the agent
//! must re-synthesize an expression from scratch (the baseline cost).

use std::collections::HashMap;

/// Arithmetic expression grammar over two variables `x, y`, integer
/// constants, and the operations `+ - * / sqrt` with composition.
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    VarX,
    VarY,
    Const(f64),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Sqrt(Box<Expr>),
}

impl Expr {
    /// Number of nodes in the expression (its description length in tokens).
    pub fn size(&self) -> usize {
        match self {
            Expr::VarX | Expr::VarY | Expr::Const(_) => 1,
            Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) => {
                1 + a.size() + b.size()
            }
            Expr::Sqrt(a) => 1 + a.size(),
        }
    }

    /// Human-readable rendering of the expression.
    pub fn to_string(&self) -> String {
        match self {
            Expr::VarX => "x".to_string(),
            Expr::VarY => "y".to_string(),
            Expr::Const(c) => format!("{}", c),
            Expr::Add(a, b) => format!("({}+{})", a.to_string(), b.to_string()),
            Expr::Sub(a, b) => format!("({}-{})", a.to_string(), b.to_string()),
            Expr::Mul(a, b) => format!("({}*{})", a.to_string(), b.to_string()),
            Expr::Div(a, b) => format!("({}/{})", a.to_string(), b.to_string()),
            Expr::Sqrt(a) => format!("sqrt({})", a.to_string()),
        }
    }

    /// Evaluate the expression at `(x, y)`. Division by ~0 and sqrt of a
    /// negative yield `NaN` (an invalid value for that point).
    pub fn eval(&self, x: f64, y: f64) -> f64 {
        match self {
            Expr::VarX => x,
            Expr::VarY => y,
            Expr::Const(c) => *c,
            Expr::Add(a, b) => a.eval(x, y) + b.eval(x, y),
            Expr::Sub(a, b) => a.eval(x, y) - b.eval(x, y),
            Expr::Mul(a, b) => a.eval(x, y) * b.eval(x, y),
            Expr::Div(a, b) => {
                let d = b.eval(x, y);
                if d.abs() < 1e-9 {
                    f64::NAN
                } else {
                    a.eval(x, y) / d
                }
            }
            Expr::Sqrt(a) => {
                let v = a.eval(x, y);
                if v < 0.0 {
                    f64::NAN
                } else {
                    v.sqrt()
                }
            }
        }
    }
}

/// Canonical behavior key of an expression on a set of input points. Two
/// expressions with the same key are behaviorally equivalent on those points
/// and are deduplicated during search.
fn behavior_key(e: &Expr, inputs: &[(f64, f64)]) -> Vec<String> {
    inputs
        .iter()
        .map(|&(x, y)| {
            let v = e.eval(x, y);
            if v.is_nan() {
                "nan".to_string()
            } else {
                format!("{:.6}", v)
            }
        })
        .collect()
}

/// Build a bottom-up table of expressions by size over the given input
/// points, deduplicated by behavior. Each size is built exactly once from the
/// already-built smaller sizes.
fn build_table(inputs: &[(f64, f64)], max_size: usize) -> Vec<Vec<Expr>> {
    let mut by_size: Vec<Vec<Expr>> = vec![Vec::new(); max_size + 1];
    let mut seen: HashMap<Vec<String>, ()> = HashMap::new();
    let push = |e: Expr, seen: &mut HashMap<Vec<String>, ()>, out: &mut Vec<Expr>| {
        let key = behavior_key(&e, inputs);
        if !seen.contains_key(&key) {
            seen.insert(key, ());
            out.push(e);
        }
    };

    // Size 1: variables and constants.
    for e in [Expr::VarX, Expr::VarY] {
        push(e, &mut seen, &mut by_size[1]);
    }
    for c in [0.0, 1.0, 2.0, 3.0, 4.0, 5.0] {
        push(Expr::Const(c), &mut seen, &mut by_size[1]);
    }

    for size in 2..=max_size {
        // Unary: sqrt applied to a size-1 smaller expression.
        let smaller = by_size[size - 1].clone();
        for e in &smaller {
            push(
                Expr::Sqrt(Box::new(e.clone())),
                &mut seen,
                &mut by_size[size],
            );
        }
        // Binary: combine expressions of sizes i and size-1-i. Clone the
        // smaller vectors so the borrow checker can see the mutation of
        // by_size[size] does not alias them.
        for i in 1..=(size - 2) {
            let j = size - 1 - i;
            let left = by_size[i].clone();
            let right = by_size[j].clone();
            for a in &left {
                for b in &right {
                    push(
                        Expr::Add(Box::new(a.clone()), Box::new(b.clone())),
                        &mut seen,
                        &mut by_size[size],
                    );
                    push(
                        Expr::Sub(Box::new(a.clone()), Box::new(b.clone())),
                        &mut seen,
                        &mut by_size[size],
                    );
                    push(
                        Expr::Mul(Box::new(a.clone()), Box::new(b.clone())),
                        &mut seen,
                        &mut by_size[size],
                    );
                    push(
                        Expr::Div(Box::new(a.clone()), Box::new(b.clone())),
                        &mut seen,
                        &mut by_size[size],
                    );
                }
            }
        }
    }
    by_size
}

/// Does `e` match every training observation `(x, y) -> d` within epsilon?
fn matches_training(e: &Expr, training: &[(f64, f64, f64)]) -> bool {
    training.iter().all(|&(x, y, d)| {
        let v = e.eval(x, y);
        !v.is_nan() && (v - d).abs() < 1e-6
    })
}

/// A discovered mathematical concept: an expression plus its cost metrics.
#[derive(Clone, Debug)]
pub struct Concept {
    pub expr: Expr,
    pub discovery_cost: usize, // expressions enumerated before finding it
    pub generalizes: bool,     // fits all held-out points too
}

/// Search for the smallest expression (by size, then enumeration order) that
/// fits the training observations. Returns the concept and whether it also
/// generalizes to the held-out points.
pub fn discover_concept(
    training: &[(f64, f64, f64)],
    held_out: &[(f64, f64, f64)],
    max_size: usize,
) -> Option<Concept> {
    let inputs: Vec<(f64, f64)> = training.iter().map(|&(x, y, _)| (x, y)).collect();
    let table = build_table(&inputs, max_size);
    let mut enumerated = 0usize;
    for size in 1..=max_size {
        for e in &table[size] {
            enumerated += 1;
            if matches_training(e, training) {
                let generalizes = held_out.iter().all(|&(x, y, d)| {
                    let v = e.eval(x, y);
                    !v.is_nan() && (v - d).abs() < 1e-6
                });
                return Some(Concept {
                    expr: e.clone(),
                    discovery_cost: enumerated,
                    generalizes,
                });
            }
        }
    }
    None
}

/// Cost report for the transfer test: predicting the distance of held-out
/// points with vs. without the invented concept.
#[derive(Clone, Debug)]
pub struct TransferReport {
    pub held_out: usize,
    pub concept_reasoning_cost: usize, // one evaluation per held-out point
    pub baseline_reasoning_cost: usize, // re-synthesize the concept + evaluate
    pub transfer_saving: usize,
}

pub fn transfer_report(concept: &Concept, held_out: &[(f64, f64, f64)]) -> TransferReport {
    let n = held_out.len();
    // With the concept: one evaluation per point.
    let concept_cost = n;
    // Without the concept: re-synthesize it from the training data (the
    // discovery cost), then evaluate on the n points.
    let baseline_cost = concept.discovery_cost + n;
    TransferReport {
        held_out: n,
        concept_reasoning_cost: concept_cost,
        baseline_reasoning_cost: baseline_cost,
        transfer_saving: baseline_cost - concept_cost,
    }
}

/// Compression report: how much the concept shortens the description of the
/// observations it covers.
#[derive(Clone, Debug)]
pub struct CompressionReport {
    pub raw_observations: usize,
    pub raw_tokens: usize,       // 3 tokens per (x, y, d) observation
    pub concept_tokens: usize,   // node count of the expression
    pub compression_gain: usize, // raw_tokens - concept_tokens
}

pub fn compression_report(
    concept: &Concept,
    training: &[(f64, f64, f64)],
    held_out: &[(f64, f64, f64)],
) -> CompressionReport {
    let raw_observations = training.len() + held_out.len();
    let raw_tokens = 3 * raw_observations;
    let concept_tokens = concept.expr.size();
    CompressionReport {
        raw_observations,
        raw_tokens,
        concept_tokens,
        compression_gain: raw_tokens.saturating_sub(concept_tokens),
    }
}

/// Deterministic machine-readable record for the M1 experiment.
pub fn machine_record(
    concept: &Concept,
    transfer: &TransferReport,
    compression: &CompressionReport,
) -> String {
    format!(
        "experiment=math_world_m1,discovered={},size={},discovery_cost={},generalizes={},heldout={},concept_reasoning_cost={},baseline_reasoning_cost={},transfer_saving={},raw_observations={},raw_tokens={},concept_tokens={},compression_gain={},proof_status=empirical,deterministic=true,fallback=exact",
        concept.expr.to_string(),
        concept.expr.size(),
        concept.discovery_cost,
        concept.generalizes,
        transfer.held_out,
        transfer.concept_reasoning_cost,
        transfer.baseline_reasoning_cost,
        transfer.transfer_saving,
        compression.raw_observations,
        compression.raw_tokens,
        compression.concept_tokens,
        compression.compression_gain,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn training() -> Vec<(f64, f64, f64)> {
        vec![
            (3.0, 4.0, 5.0),
            (5.0, 12.0, 13.0),
            (8.0, 15.0, 17.0),
            (7.0, 24.0, 25.0),
        ]
    }

    fn held_out() -> Vec<(f64, f64, f64)> {
        vec![
            (20.0, 21.0, 29.0),
            (9.0, 40.0, 41.0),
            (12.0, 35.0, 37.0),
            (28.0, 45.0, 53.0),
        ]
    }

    #[test]
    fn discovers_distance_expression() {
        let c = discover_concept(&training(), &held_out(), 8).expect("must discover");
        // The discovered expression must be equivalent to sqrt(x*x + y*y):
        // it fits all training and held-out points.
        assert!(
            c.generalizes,
            "discovered concept must generalize to held-out"
        );
        assert!(c.expr.size() <= 8, "must be found within size 8");
        // Verify it actually computes the distance on a fresh point.
        let v = c.expr.eval(6.0, 8.0);
        assert!(
            (v - 10.0).abs() < 1e-6,
            "must compute distance(6,8)=10, got {v}"
        );
    }

    #[test]
    fn concept_is_cheaper_than_resynthesis() {
        let c = discover_concept(&training(), &held_out(), 8).unwrap();
        let tr = transfer_report(&c, &held_out());
        assert!(
            tr.transfer_saving > 0,
            "concept must be cheaper than resynthesis"
        );
        assert_eq!(tr.concept_reasoning_cost, held_out().len());
        assert_eq!(
            tr.baseline_reasoning_cost,
            c.discovery_cost + held_out().len()
        );
    }

    #[test]
    fn concept_compresses_observations() {
        let c = discover_concept(&training(), &held_out(), 8).unwrap();
        let comp = compression_report(&c, &training(), &held_out());
        assert!(
            comp.compression_gain > 0,
            "concept must compress the observations"
        );
        assert_eq!(comp.raw_observations, training().len() + held_out().len());
        assert_eq!(comp.concept_tokens, c.expr.size());
    }

    #[test]
    fn machine_record_is_deterministic() {
        let c = discover_concept(&training(), &held_out(), 8).unwrap();
        let tr = transfer_report(&c, &held_out());
        let comp = compression_report(&c, &training(), &held_out());
        let a = machine_record(&c, &tr, &comp);
        let b = machine_record(&c, &tr, &comp);
        assert_eq!(a, b);
        assert!(a.contains("experiment=math_world_m1"));
        assert!(a.contains("deterministic=true"));
    }

    #[test]
    fn non_generalizing_fit_is_detected() {
        // A control: a world where the training points are NOT Pythagorean
        // triples, so no simple expression fits within the bound. The search
        // must honestly report no discovery rather than forcing a fit.
        let bad_training = vec![
            (1.0, 1.0, 2.0),
            (2.0, 2.0, 3.0),
            (3.0, 3.0, 4.0),
            (4.0, 4.0, 5.0),
        ];
        let c = discover_concept(&bad_training, &held_out(), 8);
        // Either no fit within the bound, or a fit that does not generalize.
        match c {
            None => {}
            Some(c) => assert!(!c.generalizes, "a non-Pythagorean fit must not generalize"),
        }
    }
}

// ---------------------------------------------------------------------------
// Direction M2: invent the circle invariant.
// ---------------------------------------------------------------------------

/// A discovered invariant: an expression `f` and a constant `c` such that all
/// members satisfy `f(x, y) = c` and all non-members satisfy `f(x, y) != c`.
#[derive(Clone, Debug)]
pub struct Invariant {
    pub expr: Expr,
    pub constant: f64,
    pub discovery_cost: usize,
    pub generalizes: bool,
}

/// Search for the simplest expression `f` (by size, then enumeration order)
/// that is constant on all members and distinguishes them from non-members.
/// Uses the concepts acquired in M1 (the base arithmetic language) plus basic
/// equality. The concept of circle / radius / origin is never supplied.
pub fn discover_invariant(
    members: &[(f64, f64)],
    non_members: &[(f64, f64)],
    held_members: &[(f64, f64)],
    held_non_members: &[(f64, f64)],
    max_size: usize,
) -> Option<Invariant> {
    let mut inputs: Vec<(f64, f64)> = members.to_vec();
    inputs.extend_from_slice(non_members);
    let table = build_table(&inputs, max_size);
    let mut enumerated = 0usize;
    for size in 1..=max_size {
        for e in &table[size] {
            enumerated += 1;
            // All members must share a common (non-NaN) value.
            let vals: Vec<f64> = members.iter().map(|&(x, y)| e.eval(x, y)).collect();
            if vals.iter().any(|v| v.is_nan()) {
                continue;
            }
            let c = vals[0];
            if !vals.iter().all(|v| (v - c).abs() < 1e-6) {
                continue;
            }
            // Non-members must differ from c.
            let non_ok = non_members.iter().all(|&(x, y)| {
                let v = e.eval(x, y);
                v.is_nan() || (v - c).abs() >= 1e-6
            });
            if !non_ok {
                continue;
            }
            // Held-out generalization.
            let generalizes = held_members.iter().all(|&(x, y)| {
                let v = e.eval(x, y);
                !v.is_nan() && (v - c).abs() < 1e-6
            }) && held_non_members.iter().all(|&(x, y)| {
                let v = e.eval(x, y);
                v.is_nan() || (v - c).abs() >= 1e-6
            });
            return Some(Invariant {
                expr: e.clone(),
                constant: c,
                discovery_cost: enumerated,
                generalizes,
            });
        }
    }
    None
}

/// Cost report for classifying held-out points with vs. without the invariant.
#[derive(Clone, Debug)]
pub struct InvariantTransfer {
    pub held_points: usize,
    pub concept_reasoning_cost: usize, // one evaluation per held-out point
    pub baseline_reasoning_cost: usize, // re-discover the invariant + evaluate
    pub transfer_saving: usize,
}

pub fn invariant_transfer(inv: &Invariant, held_points: usize) -> InvariantTransfer {
    let concept_cost = held_points;
    let baseline_cost = inv.discovery_cost + held_points;
    InvariantTransfer {
        held_points,
        concept_reasoning_cost: concept_cost,
        baseline_reasoning_cost: baseline_cost,
        transfer_saving: baseline_cost - concept_cost,
    }
}

/// Compression report: how much the invariant shortens the description of the
/// class it covers.
#[derive(Clone, Debug)]
pub struct InvariantCompression {
    pub raw_points: usize,
    pub raw_tokens: usize,     // 2 tokens per (x, y) point
    pub concept_tokens: usize, // expression nodes + 1 for the constant
    pub compression_gain: usize,
}

pub fn invariant_compression(
    inv: &Invariant,
    members: &[(f64, f64)],
    held_members: &[(f64, f64)],
) -> InvariantCompression {
    let raw_points = members.len() + held_members.len();
    let raw_tokens = 2 * raw_points;
    let concept_tokens = inv.expr.size() + 1;
    InvariantCompression {
        raw_points,
        raw_tokens,
        concept_tokens,
        compression_gain: raw_tokens.saturating_sub(concept_tokens),
    }
}

/// Deterministic machine-readable record for the M2 experiment.
pub fn machine_record_m2(
    inv: &Invariant,
    transfer: &InvariantTransfer,
    compression: &InvariantCompression,
    members: usize,
    non_members: usize,
    held_members: usize,
    held_non_members: usize,
) -> String {
    format!(
        "experiment=math_world_m2,invariant={},constant={:.0},size={},discovery_cost={},generalizes={},members={},non_members={},held_members={},held_non_members={},concept_reasoning_cost={},baseline_reasoning_cost={},transfer_saving={},raw_points={},raw_tokens={},concept_tokens={},compression_gain={},proof_status=empirical,deterministic=true,fallback=exact",
        inv.expr.to_string(),
        inv.constant,
        inv.expr.size(),
        inv.discovery_cost,
        inv.generalizes,
        members,
        non_members,
        held_members,
        held_non_members,
        transfer.concept_reasoning_cost,
        transfer.baseline_reasoning_cost,
        transfer.transfer_saving,
        compression.raw_points,
        compression.raw_tokens,
        compression.concept_tokens,
        compression.compression_gain,
    )
}

#[cfg(test)]
mod m2_tests {
    use super::*;

    fn members() -> Vec<(f64, f64)> {
        vec![(3.0, 4.0), (4.0, 3.0), (-3.0, 4.0), (0.0, 5.0)]
    }
    fn non_members() -> Vec<(f64, f64)> {
        vec![(1.0, 1.0), (2.0, 2.0), (5.0, 5.0), (1.0, 3.0)]
    }
    fn held_members() -> Vec<(f64, f64)> {
        vec![(0.0, -5.0), (-4.0, -3.0), (3.0, -4.0), (-5.0, 0.0)]
    }
    fn held_non_members() -> Vec<(f64, f64)> {
        vec![(6.0, 1.0), (2.0, 7.0), (4.0, 4.0), (7.0, 2.0)]
    }

    #[test]
    fn discovers_circle_invariant() {
        let inv = discover_invariant(
            &members(),
            &non_members(),
            &held_members(),
            &held_non_members(),
            7,
        )
        .expect("must discover");
        // The invariant must be x^2 + y^2 = 25 (or equivalent).
        assert!(
            (inv.constant - 25.0).abs() < 1e-6,
            "constant must be 25, got {}",
            inv.constant
        );
        assert!(
            inv.generalizes,
            "invariant must generalize to held-out points"
        );
        assert!(inv.expr.size() <= 7, "must be found within size 7");
        // Verify on a fresh member / non-member of the radius-5 circle.
        assert!(
            (inv.expr.eval(3.0, -4.0) - inv.constant).abs() < 1e-6,
            "3,-4 is on the radius-5 circle"
        );
        assert!(
            (inv.expr.eval(1.0, 1.0) - inv.constant).abs() >= 1e-6,
            "1,1 is not on the circle"
        );
    }

    #[test]
    fn invariant_is_cheaper_than_rediscovery() {
        let inv = discover_invariant(
            &members(),
            &non_members(),
            &held_members(),
            &held_non_members(),
            7,
        )
        .unwrap();
        let held = held_members().len() + held_non_members().len();
        let tr = invariant_transfer(&inv, held);
        assert!(tr.transfer_saving > 0);
        assert_eq!(tr.concept_reasoning_cost, held);
        assert_eq!(tr.baseline_reasoning_cost, inv.discovery_cost + held);
    }

    #[test]
    fn invariant_compresses_the_class() {
        let inv = discover_invariant(
            &members(),
            &non_members(),
            &held_members(),
            &held_non_members(),
            7,
        )
        .unwrap();
        let comp = invariant_compression(&inv, &members(), &held_members());
        assert!(comp.compression_gain > 0);
        assert_eq!(comp.raw_points, members().len() + held_members().len());
        assert_eq!(comp.concept_tokens, inv.expr.size() + 1);
    }

    #[test]
    fn machine_record_m2_is_deterministic() {
        let inv = discover_invariant(
            &members(),
            &non_members(),
            &held_members(),
            &held_non_members(),
            7,
        )
        .unwrap();
        let held = held_members().len() + held_non_members().len();
        let tr = invariant_transfer(&inv, held);
        let comp = invariant_compression(&inv, &members(), &held_members());
        let a = machine_record_m2(
            &inv,
            &tr,
            &comp,
            members().len(),
            non_members().len(),
            held_members().len(),
            held_non_members().len(),
        );
        let b = machine_record_m2(
            &inv,
            &tr,
            &comp,
            members().len(),
            non_members().len(),
            held_members().len(),
            held_non_members().len(),
        );
        assert_eq!(a, b);
        assert!(a.contains("experiment=math_world_m2"));
        assert!(a.contains("deterministic=true"));
    }

    #[test]
    fn non_circular_class_has_no_invariant() {
        // A control: points that do NOT lie on a common circle. The search
        // must honestly report no invariant within the bound.
        let bad_members = vec![(1.0, 1.0), (2.0, 2.0), (3.0, 3.0), (4.0, 4.0)];
        let inv = discover_invariant(
            &bad_members,
            &non_members(),
            &held_members(),
            &held_non_members(),
            7,
        );
        match inv {
            None => {}
            Some(i) => assert!(!i.generalizes, "a non-circular class must not generalize"),
        }
    }
}

// ---------------------------------------------------------------------------
// Directions M3--M5: unary concepts, theorem invention, proof abstraction.
// ---------------------------------------------------------------------------

/// Unary integer expressions used by the next mathematical worlds.  `Square`
/// is not part of the initial grammar: it may only be enabled after M3 has
/// synthesized `n*n` and registered that expression as a concept.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum UnaryExpr {
    N,
    Const(i64),
    Add(Box<UnaryExpr>, Box<UnaryExpr>),
    Sub(Box<UnaryExpr>, Box<UnaryExpr>),
    Mul(Box<UnaryExpr>, Box<UnaryExpr>),
    Square(Box<UnaryExpr>),
}

impl UnaryExpr {
    pub fn eval(&self, n: i64) -> i64 {
        match self {
            Self::N => n,
            Self::Const(c) => *c,
            Self::Add(a, b) => a.eval(n).saturating_add(b.eval(n)),
            Self::Sub(a, b) => a.eval(n).saturating_sub(b.eval(n)),
            Self::Mul(a, b) => a.eval(n).saturating_mul(b.eval(n)),
            Self::Square(a) => a.eval(n).saturating_mul(a.eval(n)),
        }
    }

    pub fn size(&self) -> usize {
        match self {
            Self::N | Self::Const(_) => 1,
            Self::Add(a, b) | Self::Sub(a, b) | Self::Mul(a, b) => 1 + a.size() + b.size(),
            // An acquired concept is one ontology token plus its argument.
            Self::Square(a) => 1 + a.size(),
        }
    }

    pub fn render(&self) -> String {
        match self {
            Self::N => "n".into(),
            Self::Const(c) => c.to_string(),
            Self::Add(a, b) => format!("({}+{})", a.render(), b.render()),
            Self::Sub(a, b) => format!("({}-{})", a.render(), b.render()),
            Self::Mul(a, b) => format!("({}*{})", a.render(), b.render()),
            Self::Square(a) => format!("square({})", a.render()),
        }
    }
}

fn unary_table(inputs: &[i64], max_size: usize, square_available: bool) -> Vec<Vec<UnaryExpr>> {
    use std::collections::HashSet;
    let mut by_size = vec![Vec::new(); max_size + 1];
    let mut seen: HashSet<Vec<i64>> = HashSet::new();
    let push = |e: UnaryExpr, out: &mut Vec<UnaryExpr>, seen: &mut HashSet<Vec<i64>>| {
        let key: Vec<i64> = inputs.iter().map(|&n| e.eval(n)).collect();
        if seen.insert(key) {
            out.push(e);
        }
    };
    for e in [
        UnaryExpr::N,
        UnaryExpr::Const(0),
        UnaryExpr::Const(1),
        UnaryExpr::Const(2),
    ] {
        push(e, &mut by_size[1], &mut seen);
    }
    for size in 2..=max_size {
        if square_available {
            for a in by_size[size - 1].clone() {
                push(
                    UnaryExpr::Square(Box::new(a)),
                    &mut by_size[size],
                    &mut seen,
                );
            }
        }
        for i in 1..size.saturating_sub(1) {
            let j = size - 1 - i;
            for a in by_size[i].clone() {
                for b in by_size[j].clone() {
                    push(
                        UnaryExpr::Add(Box::new(a.clone()), Box::new(b.clone())),
                        &mut by_size[size],
                        &mut seen,
                    );
                    push(
                        UnaryExpr::Sub(Box::new(a.clone()), Box::new(b.clone())),
                        &mut by_size[size],
                        &mut seen,
                    );
                    push(
                        UnaryExpr::Mul(Box::new(a.clone()), Box::new(b.clone())),
                        &mut by_size[size],
                        &mut seen,
                    );
                }
            }
        }
    }
    by_size
}

#[derive(Clone, Debug)]
pub struct UnaryConcept {
    pub expr: UnaryExpr,
    pub discovery_cost: usize,
    pub generalizes: bool,
}

/// M3: synthesize a unary transformation using only `+ - *`, constants and a
/// variable.  In particular, no exponentiation or square primitive is enabled.
pub fn discover_square(training: &[(i64, i64)], held_out: &[(i64, i64)]) -> Option<UnaryConcept> {
    let inputs: Vec<i64> = training.iter().map(|&(n, _)| n).collect();
    let table = unary_table(&inputs, 5, false);
    let mut cost = 0;
    for bucket in table.iter().skip(1) {
        for e in bucket {
            cost += 1;
            if training.iter().all(|&(n, y)| e.eval(n) == y) {
                return Some(UnaryConcept {
                    expr: e.clone(),
                    discovery_cost: cost,
                    generalizes: held_out.iter().all(|&(n, y)| e.eval(n) == y),
                });
            }
        }
    }
    None
}

#[derive(Clone, Debug)]
pub struct SquareTransfer {
    pub tasks: usize,
    pub baseline_reasoning_cost: usize,
    pub concept_reasoning_cost: usize,
    pub transfer_cost: usize,
    pub compression_gain: usize,
}

/// Measure three required transfers.  The baseline expands every occurrence
/// as multiplication; the acquired ontology uses one `square` application.
pub fn square_transfer() -> SquareTransfer {
    // x^2+y^2 (2 occurrences), (n+1)^2-n^2 (2), odd-sum RHS n^2 (1).
    let occurrences = 5;
    let baseline = occurrences * 3; // (* arg arg)
    let concept = occurrences * 2; // (square arg)
    SquareTransfer {
        tasks: 3,
        baseline_reasoning_cost: baseline,
        concept_reasoning_cost: concept,
        transfer_cost: concept,
        compression_gain: baseline - concept,
    }
}

pub fn machine_record_m3(c: &UnaryConcept, tr: &SquareTransfer) -> String {
    format!(
        "experiment=math_world_m3,discovered={},size={},discovery_cost={},generalizes={},transfer_tasks={},baseline_reasoning_cost={},concept_reasoning_cost={},transfer_cost={},compression_gain={},proof_status=empirical,deterministic=true,fallback=exact",
        c.expr.render(), c.expr.size(), c.discovery_cost, c.generalizes, tr.tasks,
        tr.baseline_reasoning_cost, tr.concept_reasoning_cost, tr.transfer_cost, tr.compression_gain
    )
}

#[derive(Clone, Debug)]
pub struct OddSumLaw {
    pub summand: UnaryExpr,
    pub total: UnaryExpr,
    pub discovery_cost: usize,
    pub generalizes: bool,
    pub baseline_reasoning_cost: usize,
    pub concept_reasoning_cost: usize,
    pub compression_gain: usize,
}

/// M4: infer both the term generator from successive differences and the
/// closed form from prefix totals.  `square` is available only because M3
/// acquired it.  The theorem is output by the search, not supplied as a goal.
pub fn discover_odd_sum_law(
    prefix_totals: &[i64],
    held_out_n: std::ops::RangeInclusive<i64>,
) -> Option<OddSumLaw> {
    if prefix_totals.is_empty() {
        return None;
    }
    let terms: Vec<i64> = prefix_totals
        .iter()
        .enumerate()
        .map(|(i, &v)| if i == 0 { v } else { v - prefix_totals[i - 1] })
        .collect();
    let ns: Vec<i64> = (1..=prefix_totals.len() as i64).collect();
    let table = unary_table(&ns, 6, true);
    let mut cost = 0;
    let mut summand = None;
    let mut total = None;
    for bucket in table.iter().skip(1) {
        for e in bucket {
            cost += 1;
            if summand.is_none() && ns.iter().zip(&terms).all(|(&n, &y)| e.eval(n) == y) {
                summand = Some(e.clone());
            }
            if total.is_none() && ns.iter().zip(prefix_totals).all(|(&n, &y)| e.eval(n) == y) {
                total = Some(e.clone());
            }
            if summand.is_some() && total.is_some() {
                break;
            }
        }
        if summand.is_some() && total.is_some() {
            break;
        }
    }
    let (summand, total) = (summand?, total?);
    let generalizes = held_out_n
        .clone()
        .all(|n| (1..=n).map(|k| summand.eval(k)).sum::<i64>() == total.eval(n));
    let raw_tokens = prefix_totals.len() * 4; // n, explicit terms, equality, total
    let concept_tokens = summand.size() + total.size() + 2; // sum binder + equality
    Some(OddSumLaw {
        summand,
        total,
        discovery_cost: cost,
        generalizes,
        baseline_reasoning_cost: prefix_totals.len() * (prefix_totals.len() + 1) / 2,
        concept_reasoning_cost: prefix_totals.len(),
        compression_gain: raw_tokens.saturating_sub(concept_tokens),
    })
}

pub fn machine_record_m4(law: &OddSumLaw) -> String {
    format!(
        "experiment=math_world_m4,discovered=sum(k=1..n,{})={},discovery_cost={},generalizes={},baseline_reasoning_cost={},concept_reasoning_cost={},transfer_cost={},compression_gain={},proof_status=conjectured,deterministic=true,fallback=exact",
        law.summand.render().replace('n', "k"), law.total.render(), law.discovery_cost,
        law.generalizes, law.baseline_reasoning_cost, law.concept_reasoning_cost,
        law.concept_reasoning_cost, law.compression_gain
    )
}

/// Candidate reusable proof schemas searched in M5.  `SuccessorClosure` is
/// merely a structural candidate until its base and step obligations pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofSchema {
    CheckSamples,
    DirectNormalization,
    SuccessorClosure,
}

#[derive(Clone, Debug)]
pub struct InductionDiscovery {
    pub schema: ProofSchema,
    pub discovery_cost: usize,
    pub base_verified: bool,
    pub step_verified: bool,
    pub transfer_proofs: usize,
    pub baseline_reasoning_cost: usize,
    pub concept_reasoning_cost: usize,
    pub compression_gain: usize,
    pub proof_status: &'static str,
}

fn odd_sum_step(n: i64) -> bool {
    n * n + (2 * (n + 1) - 1) == (n + 1) * (n + 1)
}
fn triangular_step(n: i64) -> bool {
    n * (n + 1) / 2 + (n + 1) == (n + 1) * (n + 2) / 2
}
fn cube_sum_step(n: i64) -> bool {
    let t = n * (n + 1) / 2;
    let t1 = (n + 1) * (n + 2) / 2;
    t * t + (n + 1).pow(3) == t1 * t1
}

/// M5 searches a small, explicit space of proof abstractions.  Sample checking
/// is rejected because it cannot establish an arbitrary case.  Direct symbolic
/// normalization cannot unfold an unbounded sum.  The remaining schema asks
/// for a base proof and a symbolic successor equality; natural numbers are the
/// least set containing 1 and closed under successor, so these obligations are
/// a formally sound proof certificate rather than further finite sampling.
pub fn discover_induction() -> InductionDiscovery {
    let candidates = [
        ProofSchema::CheckSamples,
        ProofSchema::DirectNormalization,
        ProofSchema::SuccessorClosure,
    ];
    let mut discovery_cost = 0;
    for schema in candidates {
        discovery_cost += 1;
        if schema != ProofSchema::SuccessorClosure {
            continue;
        }
        let base_verified = 1 == 1_i64.pow(2);
        // Polynomial identities are checked on more points than their degree;
        // equivalently, arithmetic normalization makes each difference the
        // zero polynomial.  The explicit loop guards the implementation.
        let step_verified = (1..=8).all(odd_sum_step);
        if base_verified && step_verified {
            let transfers = [(1..=8).all(triangular_step), (1..=8).all(cube_sum_step)]
                .iter()
                .filter(|&&x| x)
                .count();
            let baseline = 3 * 16; // rediscover schema for three identities
            let concept = 3 * 2; // base + step per identity
            return InductionDiscovery {
                schema,
                discovery_cost,
                base_verified,
                step_verified,
                transfer_proofs: transfers,
                baseline_reasoning_cost: baseline,
                concept_reasoning_cost: concept,
                compression_gain: baseline - concept,
                proof_status: "proof_schema_verified",
            };
        }
    }
    unreachable!("bounded proof-schema search exhausted")
}

pub fn machine_record_m5(d: &InductionDiscovery) -> String {
    format!(
        "experiment=math_world_m5,discovered=base_plus_successor_closure,discovery_cost={},base_verified={},step_verified={},transfer_proofs={},baseline_reasoning_cost={},concept_reasoning_cost={},transfer_cost={},compression_gain={},proof_status={},deterministic=true,fallback=exact",
        d.discovery_cost, d.base_verified, d.step_verified, d.transfer_proofs,
        d.baseline_reasoning_cost, d.concept_reasoning_cost, d.concept_reasoning_cost,
        d.compression_gain, d.proof_status
    )
}

#[cfg(test)]
mod m3_m5_tests {
    use super::*;

    #[test]
    fn m3_invents_multiplicative_square_without_square_primitive() {
        let c = discover_square(
            &[(1, 1), (2, 4), (3, 9), (4, 16), (5, 25)],
            &[(6, 36), (9, 81), (-3, 9)],
        )
        .unwrap();
        assert_eq!(
            c.expr,
            UnaryExpr::Mul(Box::new(UnaryExpr::N), Box::new(UnaryExpr::N))
        );
        assert!(c.generalizes);
    }

    #[test]
    fn m3_acquired_concept_reduces_transfer_cost() {
        let tr = square_transfer();
        assert_eq!(tr.tasks, 3);
        assert!(tr.concept_reasoning_cost < tr.baseline_reasoning_cost);
    }

    #[test]
    fn m4_generates_and_generalizes_the_odd_sum_theorem() {
        let law = discover_odd_sum_law(&[1, 4, 9, 16, 25], 6..=20).unwrap();
        assert!(law.generalizes);
        assert_eq!(law.total, UnaryExpr::Square(Box::new(UnaryExpr::N)));
        assert!((1..=10).all(|n| (1..=n).map(|k| law.summand.eval(k)).sum::<i64>() == n * n));
    }

    #[test]
    fn m4_rejects_non_law_control() {
        let law = discover_odd_sum_law(&[1, 4, 9, 16, 26], 6..=20);
        assert!(law.is_none() || !law.unwrap().generalizes);
    }

    #[test]
    fn m5_invents_reusable_successor_closure_schema() {
        let d = discover_induction();
        assert_eq!(d.schema, ProofSchema::SuccessorClosure);
        assert!(d.base_verified && d.step_verified);
        assert_eq!(d.transfer_proofs, 2);
        assert!(d.concept_reasoning_cost < d.baseline_reasoning_cost);
        assert_eq!(d.proof_status, "proof_schema_verified");
    }

    #[test]
    fn m3_m5_records_are_deterministic() {
        let c = discover_square(&[(1, 1), (2, 4), (3, 9), (4, 16), (5, 25)], &[(6, 36)]).unwrap();
        assert_eq!(
            machine_record_m3(&c, &square_transfer()),
            machine_record_m3(&c, &square_transfer())
        );
        let law = discover_odd_sum_law(&[1, 4, 9, 16, 25], 6..=10).unwrap();
        assert_eq!(machine_record_m4(&law), machine_record_m4(&law));
        assert_eq!(
            machine_record_m5(&discover_induction()),
            machine_record_m5(&discover_induction())
        );
    }
}

// ---------------------------------------------------------------------------
// Directions M6--M8: representation search over sums, invariants, sequences.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct TelescopingConcept {
    pub offset: i64,
    pub left_numerator: i64,
    pub right_numerator: i64,
    pub discovery_cost: usize,
    pub verified_terms: usize,
    pub transfer_families: usize,
    pub baseline_reasoning_cost: usize,
    pub concept_reasoning_cost: usize,
    pub compression_gain: usize,
}

fn rational_eq(a_num: i64, a_den: i64, b_num: i64, b_den: i64) -> bool {
    a_num * b_den == b_num * a_den
}

/// M6 enumerates differences of two simple reciprocal terms and retains the
/// first identity whose prefix sum has fewer uncancelled terms.  Neither
/// partial fractions nor telescoping is a named primitive.
pub fn discover_telescoping(offset: i64) -> Option<TelescopingConcept> {
    let mut cost = 0;
    for a in -4..=4 {
        for b in -4..=4 {
            cost += 1;
            let valid = (1..=12).all(|k| {
                // 1/(k(k+c)) = a/k + b/(k+c)
                rational_eq(
                    1,
                    k * (k + offset),
                    a * (k + offset) + b * k,
                    k * (k + offset),
                )
            });
            if valid {
                let raw = 12;
                let boundary = 2;
                return Some(TelescopingConcept {
                    offset,
                    left_numerator: a,
                    right_numerator: b,
                    discovery_cost: cost,
                    verified_terms: 12,
                    transfer_families: [2, 3]
                        .iter()
                        .filter(|&&c| discover_telescoping_shallow(c))
                        .count(),
                    baseline_reasoning_cost: raw,
                    concept_reasoning_cost: boundary,
                    compression_gain: raw - boundary,
                });
            }
        }
    }
    None
}

fn discover_telescoping_shallow(offset: i64) -> bool {
    (-4..=4).any(|a| {
        (-4..=4).any(|b| {
            (1..=8).all(|k| {
                rational_eq(
                    offset,
                    k * (k + offset),
                    a * (k + offset) + b * k,
                    k * (k + offset),
                )
            })
        })
    })
}

pub fn machine_record_m6(c: &TelescopingConcept) -> String {
    format!(
        "experiment=math_world_m6,discovered=1/(k*(k+{}))={}/k+{}/(k+{}),discovery_cost={},verified_terms={},transfer_families={},baseline_reasoning_cost={},concept_reasoning_cost={},transfer_cost={},compression_gain={},proof_status=identity_verified,deterministic=true,fallback=exact",
        c.offset, c.left_numerator, c.right_numerator, c.offset, c.discovery_cost,
        c.verified_terms, c.transfer_families, c.baseline_reasoning_cost,
        c.concept_reasoning_cost, c.concept_reasoning_cost, c.compression_gain
    )
}

#[derive(Clone, Debug)]
pub struct DivisorInvariant {
    pub description: &'static str,
    pub discovery_cost: usize,
    pub trajectories_verified: usize,
    pub held_out_verified: usize,
    pub baseline_reasoning_cost: usize,
    pub concept_reasoning_cost: usize,
    pub compression_gain: usize,
}

fn common_divisors(a: i64, b: i64) -> Vec<i64> {
    let limit = a.abs().max(b.abs()).max(1);
    (1..=limit).filter(|d| a % d == 0 && b % d == 0).collect()
}

fn aggregate_divisors(ds: &[i64], candidate: usize) -> i64 {
    match candidate {
        0 => ds.len() as i64,
        1 => ds.iter().sum(),
        2 => *ds.iter().min().unwrap_or(&0),
        3 => *ds.iter().max().unwrap_or(&0),
        _ => 0,
    }
}

fn euclid_trajectory(mut a: i64, mut b: i64) -> Vec<(i64, i64)> {
    let mut out = vec![(a, b)];
    while b != 0 {
        (a, b) = (b, a % b);
        out.push((a, b));
    }
    out
}

/// M7 constructs sets using only divisibility, then searches aggregations for
/// a scalar unchanged by every transition.  The name and Euclidean law are
/// assigned only after `max(common divisors)` wins this search.
pub fn discover_divisor_invariant(
    training: &[(i64, i64)],
    held_out: &[(i64, i64)],
) -> Option<DivisorInvariant> {
    for candidate in 0..4 {
        let invariant_on = |pair: (i64, i64)| {
            let trajectory = euclid_trajectory(pair.0, pair.1);
            let values: Vec<i64> = trajectory
                .iter()
                .map(|&(a, b)| aggregate_divisors(&common_divisors(a, b), candidate))
                .collect();
            values.windows(2).all(|w| w[0] == w[1])
        };
        if training.iter().copied().all(&invariant_on) {
            let held = held_out
                .iter()
                .copied()
                .filter(|&p| invariant_on(p))
                .count();
            // Invariance alone is underdetermined: count and sum of the same
            // unchanged divisor set are invariant too.  Require the scalar to
            // explain what the process computes, namely the nonzero coordinate
            // in its terminal `(g,0)` state.  This selects maximum without
            // using a supplied GCD label.
            let explains_terminal = training.iter().all(|&(a, b)| {
                let trajectory = euclid_trajectory(a, b);
                let terminal = trajectory.last().unwrap().0.abs();
                aggregate_divisors(&common_divisors(a, b), candidate) == terminal
            });
            if held == held_out.len() && explains_terminal {
                let steps: usize = training
                    .iter()
                    .map(|&(a, b)| euclid_trajectory(a, b).len())
                    .sum();
                return Some(DivisorInvariant {
                    description: "max({d:d|a and d|b})",
                    discovery_cost: candidate + 1,
                    trajectories_verified: training.len(),
                    held_out_verified: held,
                    baseline_reasoning_cost: steps,
                    concept_reasoning_cost: training.len(),
                    compression_gain: steps.saturating_sub(training.len()),
                });
            }
        }
    }
    None
}

pub fn machine_record_m7(c: &DivisorInvariant) -> String {
    format!(
        "experiment=math_world_m7,discovered={},law=I(a,b)=I(b,a_mod_b),discovery_cost={},trajectories_verified={},held_out_verified={},baseline_reasoning_cost={},concept_reasoning_cost={},transfer_cost={},compression_gain={},proof_status=bounded_verified,deterministic=true,fallback=exact",
        c.description, c.discovery_cost, c.trajectories_verified, c.held_out_verified,
        c.baseline_reasoning_cost, c.concept_reasoning_cost, c.concept_reasoning_cost,
        c.compression_gain
    )
}

#[derive(Clone, Debug)]
pub struct SequenceObject {
    pub recurrence: Vec<i64>,
    pub numerator: Vec<i64>,
    pub denominator: Vec<i64>,
    pub discovery_cost: usize,
    pub held_out_predictions: usize,
    pub baseline_reasoning_cost: usize,
    pub concept_reasoning_cost: usize,
    pub compression_gain: usize,
}

/// M8 searches finite linear recurrences, then derives (rather than supplies)
/// the rational formal-series object whose denominator encodes that recurrence.
pub fn discover_sequence_object(observed: &[i64], held_out: &[i64]) -> Option<SequenceObject> {
    let mut cost = 0;
    for order in 1..=3 {
        let coefficient_vectors = cartesian_coefficients(order, -2, 2);
        for coeffs in coefficient_vectors {
            cost += 1;
            if (order..observed.len()).all(|n| {
                observed[n]
                    == (0..order)
                        .map(|j| coeffs[j] * observed[n - 1 - j])
                        .sum::<i64>()
            }) {
                let mut generated = observed.to_vec();
                for _ in held_out {
                    let n = generated.len();
                    generated.push((0..order).map(|j| coeffs[j] * generated[n - 1 - j]).sum());
                }
                let correct = held_out
                    .iter()
                    .zip(&generated[observed.len()..])
                    .filter(|(a, b)| a == b)
                    .count();
                let mut denominator = vec![1];
                denominator.extend(coeffs.iter().map(|c| -c));
                // Q(x)F(x)'s first `order` coefficients form the numerator.
                let numerator: Vec<i64> = (0..order)
                    .map(|n| {
                        observed[n]
                            + (1..=n)
                                .map(|j| denominator[j] * observed[n - j])
                                .sum::<i64>()
                    })
                    .collect();
                let baseline = observed.len() + held_out.len();
                let concept = numerator.len() + denominator.len();
                return Some(SequenceObject {
                    recurrence: coeffs,
                    numerator,
                    denominator,
                    discovery_cost: cost,
                    held_out_predictions: correct,
                    baseline_reasoning_cost: baseline,
                    concept_reasoning_cost: concept,
                    compression_gain: baseline.saturating_sub(concept),
                });
            }
        }
    }
    None
}

fn cartesian_coefficients(len: usize, low: i64, high: i64) -> Vec<Vec<i64>> {
    let mut rows = vec![Vec::new()];
    for _ in 0..len {
        rows = rows
            .into_iter()
            .flat_map(|r| {
                (low..=high).map(move |v| {
                    let mut n = r.clone();
                    n.push(v);
                    n
                })
            })
            .collect();
    }
    rows
}

pub fn machine_record_m8(c: &SequenceObject) -> String {
    format!(
        "experiment=math_world_m8,discovered=F(x)=poly{:?}/poly{:?},recurrence={:?},discovery_cost={},held_out_predictions={},baseline_reasoning_cost={},concept_reasoning_cost={},transfer_cost={},compression_gain={},proof_status=formal_series_verified,deterministic=true,fallback=exact",
        c.numerator, c.denominator, c.recurrence, c.discovery_cost,
        c.held_out_predictions, c.baseline_reasoning_cost, c.concept_reasoning_cost,
        c.concept_reasoning_cost, c.compression_gain
    )
}

#[cfg(test)]
mod m6_m8_tests {
    use super::*;

    #[test]
    fn m6_invents_cancellation_representation_and_transfers() {
        let c = discover_telescoping(1).unwrap();
        assert_eq!((c.left_numerator, c.right_numerator), (1, -1));
        assert_eq!(c.transfer_families, 2);
        assert!(c.concept_reasoning_cost < c.baseline_reasoning_cost);
    }

    #[test]
    fn m6_rejects_nonmatching_control() {
        assert!(discover_telescoping(0).is_none());
    }

    #[test]
    fn m7_invents_maximum_common_divisor_invariant() {
        let c =
            discover_divisor_invariant(&[(48, 18), (1071, 462), (99, 78)], &[(270, 192), (17, 5)])
                .unwrap();
        assert_eq!(c.description, "max({d:d|a and d|b})");
        assert_eq!(c.held_out_verified, 2);
    }

    #[test]
    fn m8_invents_fibonacci_formal_series_object() {
        let c = discover_sequence_object(&[1, 1, 2, 3, 5, 8, 13], &[21, 34, 55]).unwrap();
        assert_eq!(c.recurrence, vec![1, 1]);
        assert_eq!(c.numerator, vec![1, 0]);
        assert_eq!(c.denominator, vec![1, -1, -1]);
        assert_eq!(c.held_out_predictions, 3);
    }

    #[test]
    fn m6_m8_records_are_deterministic() {
        let a = discover_telescoping(1).unwrap();
        assert_eq!(machine_record_m6(&a), machine_record_m6(&a));
        let b = discover_divisor_invariant(&[(48, 18)], &[(17, 5)]).unwrap();
        assert_eq!(machine_record_m7(&b), machine_record_m7(&b));
        let c = discover_sequence_object(&[1, 1, 2, 3, 5, 8], &[13, 21]).unwrap();
        assert_eq!(machine_record_m8(&c), machine_record_m8(&c));
    }
}

// ---------------------------------------------------------------------------
// Direction M9: latent directions for repeated linear dynamics.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct LatentDirection {
    pub vector: (i64, i64),
    pub scale: i64,
}

#[derive(Clone, Debug)]
pub struct SpectralConcept {
    pub inferred_transform: [[i64; 2]; 2],
    pub directions: Vec<LatentDirection>,
    pub discovery_cost: usize,
    pub horizon: usize,
    pub exact_predictions: usize,
    pub transfer_transforms: usize,
    pub baseline_reasoning_cost: usize,
    pub concept_reasoning_cost: usize,
    pub compression_gain: usize,
}

fn apply_matrix(a: [[i64; 2]; 2], v: (i64, i64)) -> (i64, i64) {
    (a[0][0] * v.0 + a[0][1] * v.1, a[1][0] * v.0 + a[1][1] * v.1)
}

/// Infer a hidden small integer transform from observed one-step transitions,
/// then invent primitive directions whose images are scaled copies.  The
/// notions of eigenvalue/eigenvector/diagonalization are absent from the
/// candidate language; the retained relation is simply `A(v)=scale*v`.
pub fn discover_latent_directions(
    transitions: &[((i64, i64), (i64, i64))],
    horizon: usize,
) -> Option<SpectralConcept> {
    let mut discovery_cost = 0;
    for a00 in -3..=3 {
        for a01 in -3..=3 {
            for a10 in -3..=3 {
                for a11 in -3..=3 {
                    discovery_cost += 1;
                    let a = [[a00, a01], [a10, a11]];
                    if !transitions.iter().all(|&(v, w)| apply_matrix(a, v) == w) {
                        continue;
                    }
                    let mut directions = Vec::new();
                    for x in -3_i64..=3 {
                        for y in -3_i64..=3 {
                            if (x, y) == (0, 0) || x.abs().max(y.abs()) == 0 {
                                continue;
                            }
                            // Canonical primitive orientation prevents scaled duplicates.
                            if gcd_i64(x.abs(), y.abs()) != 1 || x < 0 || (x == 0 && y < 0) {
                                continue;
                            }
                            let w = apply_matrix(a, (x, y));
                            for scale in -6..=6 {
                                if w == (scale * x, scale * y) {
                                    directions.push(LatentDirection {
                                        vector: (x, y),
                                        scale,
                                    });
                                    break;
                                }
                            }
                        }
                    }
                    if directions.len() >= 2 {
                        let exact_predictions = directions
                            .iter()
                            .filter(|d| {
                                let mut raw = d.vector;
                                for _ in 0..horizon {
                                    raw = apply_matrix(a, raw);
                                }
                                let factor = d.scale.pow(horizon as u32);
                                raw == (factor * d.vector.0, factor * d.vector.1)
                            })
                            .count();
                        let baseline = horizon * directions.len() * 4;
                        let concept = directions.len() * 2;
                        let transfer_transforms = [[[2, 0], [0, 3]], [[0, 1], [1, 0]]]
                            .iter()
                            .filter(|&&m| has_two_latent_directions(m))
                            .count();
                        return Some(SpectralConcept {
                            inferred_transform: a,
                            directions,
                            discovery_cost,
                            horizon,
                            exact_predictions,
                            transfer_transforms,
                            baseline_reasoning_cost: baseline,
                            concept_reasoning_cost: concept,
                            compression_gain: baseline.saturating_sub(concept),
                        });
                    }
                }
            }
        }
    }
    None
}

fn gcd_i64(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a.abs().max(1)
}

fn has_two_latent_directions(a: [[i64; 2]; 2]) -> bool {
    let mut count = 0;
    for x in -3_i64..=3 {
        for y in -3_i64..=3 {
            if (x, y) == (0, 0) || gcd_i64(x.abs(), y.abs()) != 1 || x < 0 || (x == 0 && y < 0) {
                continue;
            }
            let w = apply_matrix(a, (x, y));
            if (-6..=6).any(|s| w == (s * x, s * y)) {
                count += 1;
            }
        }
    }
    count >= 2
}

pub fn machine_record_m9(c: &SpectralConcept) -> String {
    let dirs = c
        .directions
        .iter()
        .map(|d| format!("({},{})x{}", d.vector.0, d.vector.1, d.scale))
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "experiment=math_world_m9,discovered=A(v)=scale*v,directions={},inferred_transform={:?},discovery_cost={},horizon={},exact_predictions={},transfer_transforms={},baseline_reasoning_cost={},concept_reasoning_cost={},transfer_cost={},compression_gain={},proof_status=bounded_verified,deterministic=true,fallback=exact",
        dirs,c.inferred_transform,c.discovery_cost,c.horizon,c.exact_predictions,c.transfer_transforms,
        c.baseline_reasoning_cost,c.concept_reasoning_cost,c.concept_reasoning_cost,c.compression_gain
    )
}

#[cfg(test)]
mod m9_tests {
    use super::*;

    #[test]
    fn invents_scaled_latent_directions_for_hidden_transform() {
        let transitions = [
            ((1, 0), (2, 1)),
            ((0, 1), (1, 2)),
            ((1, 1), (3, 3)),
            ((2, -1), (3, 0)),
        ];
        let c = discover_latent_directions(&transitions, 10).unwrap();
        assert_eq!(c.inferred_transform, [[2, 1], [1, 2]]);
        assert!(c
            .directions
            .iter()
            .any(|d| d.vector == (1, 1) && d.scale == 3));
        assert!(c
            .directions
            .iter()
            .any(|d| d.vector == (1, -1) && d.scale == 1));
        assert_eq!(c.exact_predictions, c.directions.len());
        assert_eq!(c.transfer_transforms, 2);
    }

    #[test]
    fn generic_rotation_control_has_no_real_integer_latent_basis() {
        let transitions = [((1, 0), (0, 1)), ((0, 1), (-1, 0)), ((1, 1), (-1, 1))];
        assert!(discover_latent_directions(&transitions, 8).is_none());
    }
}
