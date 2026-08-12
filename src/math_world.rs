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
            push(Expr::Sqrt(Box::new(e.clone())), &mut seen, &mut by_size[size]);
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
                    push(Expr::Add(Box::new(a.clone()), Box::new(b.clone())), &mut seen, &mut by_size[size]);
                    push(Expr::Sub(Box::new(a.clone()), Box::new(b.clone())), &mut seen, &mut by_size[size]);
                    push(Expr::Mul(Box::new(a.clone()), Box::new(b.clone())), &mut seen, &mut by_size[size]);
                    push(Expr::Div(Box::new(a.clone()), Box::new(b.clone())), &mut seen, &mut by_size[size]);
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
                let generalizes = held_out
                    .iter()
                    .all(|&(x, y, d)| {
                        let v = e.eval(x, y);
                        !v.is_nan() && (v - d).abs() < 1e-6
                    });
                return Some(Concept { expr: e.clone(), discovery_cost: enumerated, generalizes });
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
        assert!(c.generalizes, "discovered concept must generalize to held-out");
        assert!(c.expr.size() <= 8, "must be found within size 8");
        // Verify it actually computes the distance on a fresh point.
        let v = c.expr.eval(6.0, 8.0);
        assert!((v - 10.0).abs() < 1e-6, "must compute distance(6,8)=10, got {v}");
    }

    #[test]
    fn concept_is_cheaper_than_resynthesis() {
        let c = discover_concept(&training(), &held_out(), 8).unwrap();
        let tr = transfer_report(&c, &held_out());
        assert!(tr.transfer_saving > 0, "concept must be cheaper than resynthesis");
        assert_eq!(tr.concept_reasoning_cost, held_out().len());
        assert_eq!(tr.baseline_reasoning_cost, c.discovery_cost + held_out().len());
    }

    #[test]
    fn concept_compresses_observations() {
        let c = discover_concept(&training(), &held_out(), 8).unwrap();
        let comp = compression_report(&c, &training(), &held_out());
        assert!(comp.compression_gain > 0, "concept must compress the observations");
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
        let bad_training = vec![(1.0, 1.0, 2.0), (2.0, 2.0, 3.0), (3.0, 3.0, 4.0), (4.0, 4.0, 5.0)];
        let c = discover_concept(&bad_training, &held_out(), 8);
        // Either no fit within the bound, or a fit that does not generalize.
        match c {
            None => {}
            Some(c) => assert!(!c.generalizes, "a non-Pythagorean fit must not generalize"),
        }
    }
}
