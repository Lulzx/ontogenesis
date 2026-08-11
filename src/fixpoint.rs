//! Fixed-point synthesis in the universal lambda substrate.
//!
//! No fixed-point primitive or fold backend is installed. Given a closed
//! functional `F`, [`synthesize`] constructs the finite pure-lambda term
//!
//! ```text
//! (λx. F (x x)) (λx. F (x x))
//! ```
//!
//! whose one-step unfolding is `F (fix F)`. Call-by-need evaluation makes the
//! term useful on terminating recursive branches while fuel bounds divergent
//! candidates. The construction is independent of the recursive function's
//! arity, data representation, or descent relation.

use crate::{
    nbe,
    term::{self, Term},
    transform,
};
use std::rc::Rc;

/// Synthesize a fixed point of a closed functional without a `Y` seed or
/// runtime recursion primitive.
pub fn synthesize(functional: &Rc<Term>) -> Option<Rc<Term>> {
    if !transform::is_closed(functional) {
        return None;
    }
    let half = term::lam(term::app(
        functional.clone(),
        term::app(term::var(0), term::var(0)),
    ));
    Some(term::app(half.clone(), half))
}

/// The defining right-hand side `F (fix F)`.
pub fn unfold(functional: &Rc<Term>) -> Option<Rc<Term>> {
    Some(term::app(functional.clone(), synthesize(functional)?))
}

/// Check the fixed-point equation extensionally on finite probe argument lists.
/// Every probe is a vector because the invented recursive function may have any
/// finite arity.
pub fn equation_holds(functional: &Rc<Term>, probes: &[Vec<Rc<Term>>], fuel: i64) -> bool {
    let Some(fixed) = synthesize(functional) else {
        return false;
    };
    let Some(unfolded) = unfold(functional) else {
        return false;
    };
    probes.iter().all(|args| {
        let lhs = args
            .iter()
            .fold(fixed.clone(), |f, a| term::app(f, a.clone()));
        let rhs = args
            .iter()
            .fold(unfolded.clone(), |f, a| term::app(f, a.clone()));
        normalize(&lhs, fuel)
            .zip(normalize(&rhs, fuel))
            .is_some_and(|(a, b)| a == b)
    })
}

/// Synthesize all components of a mutually recursive tuple.
///
/// `functional` maps a tuple of recursive functions to a replacement tuple.
/// The tuple itself is Church encoded, so this introduces neither products nor
/// recursion as primitives. A single unrestricted fixed point ties every
/// component's knot simultaneously.
pub fn synthesize_mutual(functional: &Rc<Term>, arity: usize) -> Option<Vec<Rc<Term>>> {
    if arity == 0 {
        return None;
    }
    let tuple = synthesize(functional)?;
    Some(
        (0..arity)
            .map(|index| term::app(tuple_projection(arity, index), tuple.clone()))
            .collect(),
    )
}

/// Church-tuple projection `λtuple. tuple (λx0...xn. xi)`.
pub fn tuple_projection(arity: usize, index: usize) -> Rc<Term> {
    assert!(arity > 0 && index < arity);
    let selected = (arity - 1 - index) as u32;
    let selector = (0..arity).fold(term::var(selected), |body, _| term::lam(body));
    term::lam(term::app(term::var(0), selector))
}

/// Validate each component of a mutually recursive fixed-point equation.
/// `component_probes[i]` contains the argument lists used for component `i`.
pub fn mutual_equations_hold(
    functional: &Rc<Term>,
    arity: usize,
    component_probes: &[Vec<Vec<Rc<Term>>>],
    fuel: i64,
) -> bool {
    if component_probes.len() != arity {
        return false;
    }
    let Some(tuple) = synthesize(functional) else {
        return false;
    };
    let unfolded = term::app(functional.clone(), tuple.clone());
    (0..arity).all(|index| {
        let projection = tuple_projection(arity, index);
        let lhs = term::app(projection.clone(), tuple.clone());
        let rhs = term::app(projection, unfolded.clone());
        component_probes[index].iter().all(|args| {
            let lhs = args
                .iter()
                .fold(lhs.clone(), |f, a| term::app(f, a.clone()));
            let rhs = args
                .iter()
                .fold(rhs.clone(), |f, a| term::app(f, a.clone()));
            normalize(&lhs, fuel)
                .zip(normalize(&rhs, fuel))
                .is_some_and(|(a, b)| a == b)
        })
    })
}

fn normalize(t: &Rc<Term>, fuel: i64) -> Option<Rc<Term>> {
    let env: nbe::Env = Rc::new(Vec::new());
    nbe::normalize(&env, t, &mut nbe::Fuel(fuel)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scott_zero() -> Rc<Term> {
        // λz.λs.z
        term::lam(term::lam(term::var(1)))
    }

    fn scott_succ() -> Rc<Term> {
        // λn.λz.λs.s n
        term::lam(term::lam(term::lam(term::app(term::var(0), term::var(2)))))
    }

    fn scott(n: u32) -> Rc<Term> {
        (0..n).fold(scott_zero(), |x, _| term::app(scott_succ(), x))
    }

    fn recursive_scott_copy_functional() -> Rc<Term> {
        // F = λr.λn. n zero (λp. succ (r p))
        let step = term::lam(term::app(
            scott_succ(),
            term::app(term::var(2), term::var(0)),
        ));
        term::lam(term::lam(term::app(
            term::app(term::var(0), scott_zero()),
            step,
        )))
    }

    #[test]
    fn synthesizes_pure_lambda_fixed_point_without_seed() {
        let functional = recursive_scott_copy_functional();
        let fixed = synthesize(&functional).unwrap();
        assert!(transform::is_closed(&fixed));
        assert!(!contains_primitive(&fixed));
        for n in 0..=6 {
            let got = normalize(&term::app(fixed.clone(), scott(n)), 2_000_000).unwrap();
            let want = normalize(&scott(n), 2_000_000).unwrap();
            assert_eq!(got, want, "recursive Scott copy failed at {n}");
        }
    }

    #[test]
    fn validates_the_fixed_point_equation_extensionally() {
        let functional = recursive_scott_copy_functional();
        let probes: Vec<Vec<Rc<Term>>> = (0..=7).map(|n| vec![scott(n)]).collect();
        assert!(equation_holds(&functional, &probes, 2_000_000));
    }

    #[test]
    fn rejects_open_functionals() {
        assert!(synthesize(&term::var(0)).is_none());
    }

    fn church_true() -> Rc<Term> {
        term::lam(term::lam(term::var(1)))
    }

    fn church_false() -> Rc<Term> {
        term::lam(term::lam(term::var(0)))
    }

    fn mutual_parity_functional() -> Rc<Term> {
        // Φ = λpair.λselect. select even odd
        // even n = n true  (λpred. odd pred)
        // odd  n = n false (λpred. even pred)
        // Inside each step: pred=0, n=1, select=2, pair=3.
        let even_step = term::lam(term::app(
            term::app(tuple_projection(2, 1), term::var(3)),
            term::var(0),
        ));
        let even = term::lam(term::app(term::app(term::var(0), church_true()), even_step));
        let odd_step = term::lam(term::app(
            term::app(tuple_projection(2, 0), term::var(3)),
            term::var(0),
        ));
        let odd = term::lam(term::app(term::app(term::var(0), church_false()), odd_step));
        term::lam(term::lam(term::app(term::app(term::var(0), even), odd)))
    }

    #[test]
    fn synthesizes_and_validates_mutual_recursion() {
        let functional = mutual_parity_functional();
        let components = synthesize_mutual(&functional, 2).unwrap();
        assert_eq!(components.len(), 2);
        for n in 0..=9 {
            let even = normalize(&term::app(components[0].clone(), scott(n)), 5_000_000).unwrap();
            let odd = normalize(&term::app(components[1].clone(), scott(n)), 5_000_000).unwrap();
            let expected_even = normalize(
                &if n % 2 == 0 {
                    church_true()
                } else {
                    church_false()
                },
                100,
            )
            .unwrap();
            let expected_odd = normalize(
                &if n % 2 == 1 {
                    church_true()
                } else {
                    church_false()
                },
                100,
            )
            .unwrap();
            assert_eq!(even, expected_even, "even failed at {n}");
            assert_eq!(odd, expected_odd, "odd failed at {n}");
        }
        let probes: Vec<Vec<Rc<Term>>> = (0..=9).map(|n| vec![scott(n)]).collect();
        assert!(mutual_equations_hold(
            &functional,
            2,
            &[probes.clone(), probes],
            5_000_000
        ));
    }

    fn ackermann_functional() -> Rc<Term> {
        // A = λr.λm.λn.
        //   m (succ n)
        //     (λmp. n (r mp one) (λnp. r mp (r (succ mp) np)))
        let one = term::app(scott_succ(), scott_zero());
        let nested = term::lam(term::app(
            term::app(term::var(4), term::var(1)),
            term::app(
                term::app(term::var(4), term::app(scott_succ(), term::var(1))),
                term::var(0),
            ),
        ));
        let m_step = term::lam(term::app(
            term::app(
                term::var(1),
                term::app(term::app(term::var(3), term::var(0)), one),
            ),
            nested,
        ));
        term::lam(term::lam(term::lam(term::app(
            term::app(term::var(1), term::app(scott_succ(), term::var(0))),
            m_step,
        ))))
    }

    #[test]
    fn synthesizes_non_structural_nested_recursion() {
        let functional = ackermann_functional();
        let ack = synthesize(&functional).unwrap();
        let cases = [(0, 3, 4), (1, 4, 6), (2, 2, 7), (2, 3, 9)];
        for (m, n, expected) in cases {
            let call = term::app(term::app(ack.clone(), scott(m)), scott(n));
            let got = normalize(&call, 20_000_000).unwrap();
            let want = normalize(&scott(expected), 20_000_000).unwrap();
            assert_eq!(got, want, "Ackermann failed at ({m}, {n})");
        }
        let probes = cases
            .iter()
            .map(|(m, n, _)| vec![scott(*m), scott(*n)])
            .collect::<Vec<_>>();
        assert!(equation_holds(&functional, &probes, 20_000_000));
    }

    fn contains_primitive(t: &Rc<Term>) -> bool {
        match t.as_ref() {
            Term::Prim(_) => true,
            Term::Lam(b) => contains_primitive(b),
            Term::App(f, a) => contains_primitive(f) || contains_primitive(a),
            Term::Var(_) | Term::Free(_) => false,
        }
    }
}
