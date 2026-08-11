//! Construction of data representations from anonymous algebraic signatures.
//!
//! A signature is only a vector of constructor field counts. No names such as
//! `nil`, `cons`, `zero`, `succ`, `list`, or `tree` enter the construction.
//! [`invent`] creates a fresh Scott-style representation and its eliminator in
//! the universal lambda substrate. Recursive behavior is deliberately absent:
//! a separately synthesized fixed point may consume values of this invented
//! representation.

use crate::{
    nbe,
    term::{self, Term},
    transform,
};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct Representation {
    /// Constructor `i` accepts `arities[i]` fields.
    pub arities: Vec<usize>,
    pub constructors: Vec<Rc<Term>>,
    /// `λvalue.λcase0...caseN. value case0 ... caseN`.
    pub eliminator: Rc<Term>,
}

#[derive(Clone, Debug)]
pub struct RepresentationProbe {
    pub fields_by_variant: Vec<Vec<Rc<Term>>>,
    pub handlers: Vec<Rc<Term>>,
}

/// Invent a closed Scott encoding for a nonempty finite sum-of-products
/// signature. Constructor names and an existing representation are unnecessary.
pub fn invent(arities: &[usize]) -> Option<Representation> {
    let variants = arities.len();
    if arities.is_empty()
        || u32::try_from(variants).is_err()
        || arities.iter().any(|&fields| {
            u32::try_from(fields).is_err()
                || variants
                    .checked_add(fields)
                    .is_none_or(|binders| u32::try_from(binders).is_err())
        })
    {
        return None;
    }
    let constructors = arities
        .iter()
        .enumerate()
        .map(|(variant, &fields)| constructor(variants, variant, fields))
        .collect();
    let eliminator = eliminator(variants);
    Some(Representation {
        arities: arities.to_vec(),
        constructors,
        eliminator,
    })
}

fn constructor(variants: usize, variant: usize, fields: usize) -> Rc<Term> {
    // λfield0...fieldM.λcase0...caseN. case_i field0 ... fieldM
    let handler_index = (variants - 1 - variant) as u32;
    let mut body = term::var(handler_index);
    for field in 0..fields {
        let field_index = (variants + fields - 1 - field) as u32;
        body = term::app(body, term::var(field_index));
    }
    (0..fields + variants).fold(body, |body, _| term::lam(body))
}

fn eliminator(variants: usize) -> Rc<Term> {
    let mut body = term::var(variants as u32);
    for variant in 0..variants {
        body = term::app(body, term::var((variants - 1 - variant) as u32));
    }
    (0..=variants).fold(body, |body, _| term::lam(body))
}

/// Apply one invented constructor to its fields.
pub fn construct(
    representation: &Representation,
    variant: usize,
    fields: &[Rc<Term>],
) -> Option<Rc<Term>> {
    if representation.arities.get(variant).copied()? != fields.len() {
        return None;
    }
    let constructor = representation.constructors.get(variant)?.clone();
    Some(
        fields
            .iter()
            .fold(constructor, |f, x| term::app(f, x.clone())),
    )
}

/// Validate the representation's elimination law on independent field and
/// handler probes. Each probe supplies fields for every constructor and one
/// handler per constructor. This detects wrong branch selection, permutation,
/// dropped fields, and constant encodings whenever the probes distinguish them.
pub fn laws_hold(
    representation: &Representation,
    probes: &[RepresentationProbe],
    fuel: i64,
) -> bool {
    if probes.is_empty()
        || representation.arities.is_empty()
        || representation.constructors.len() != representation.arities.len()
        || !representation
            .constructors
            .iter()
            .chain(std::iter::once(&representation.eliminator))
            .all(transform::is_closed)
    {
        return false;
    }
    probes.iter().all(|probe| {
        probe.fields_by_variant.len() == representation.arities.len()
            && probe.handlers.len() == representation.arities.len()
            && probe.handlers.iter().all(transform::is_closed)
            && probe
                .fields_by_variant
                .iter()
                .flatten()
                .all(transform::is_closed)
            && probe
                .fields_by_variant
                .iter()
                .enumerate()
                .all(|(variant, fields)| {
                    let Some(value) = construct(representation, variant, fields) else {
                        return false;
                    };
                    let observed = probe.handlers.iter().fold(
                        term::app(representation.eliminator.clone(), value),
                        |f, handler| term::app(f, handler.clone()),
                    );
                    let expected = fields
                        .iter()
                        .fold(probe.handlers[variant].clone(), |f, field| {
                            term::app(f, field.clone())
                        });
                    normalize(&observed, fuel)
                        .zip(normalize(&expected, fuel))
                        .is_some_and(|(a, b)| a == b)
                })
    })
}

fn normalize(t: &Rc<Term>, fuel: i64) -> Option<Rc<Term>> {
    nbe::normalize(&Rc::new(Vec::new()), t, &mut nbe::Fuel(fuel)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixpoint;

    fn church_bool(value: bool) -> Rc<Term> {
        if value {
            term::lam(term::lam(term::var(1)))
        } else {
            term::lam(term::lam(term::var(0)))
        }
    }

    fn church_numeral(n: u32) -> Rc<Term> {
        let body = (0..n).fold(term::var(0), |body, _| term::app(term::var(1), body));
        term::lam(term::lam(body))
    }

    fn const_handler(arity: usize, result: Rc<Term>) -> Rc<Term> {
        (0..arity).fold(result, |body, _| term::lam(body))
    }

    #[test]
    fn invents_and_independently_validates_anonymous_representation() {
        // Anonymous variants with 0, 1, and 2 fields; no representation is supplied.
        let rep = invent(&[0, 1, 2]).unwrap();
        let make_probe = |offset, select_second| {
            let fields = vec![
                vec![],
                vec![church_numeral(offset + 3)],
                vec![church_numeral(offset + 5), church_numeral(offset + 7)],
            ];
            let handlers = vec![
                const_handler(0, church_numeral(offset)),
                term::lam(term::var(0)),
                if select_second {
                    term::lam(term::lam(term::var(0)))
                } else {
                    term::lam(term::lam(term::var(1)))
                },
            ];
            RepresentationProbe {
                fields_by_variant: fields,
                handlers,
            }
        };
        let discovery = make_probe(0, false);
        let held_out = make_probe(11, true);
        assert!(laws_hold(&rep, &[discovery.clone()], 100_000));
        assert!(laws_hold(&rep, &[held_out.clone()], 100_000));

        let mut drops_fields = rep.clone();
        drops_fields.constructors[2] = drops_fields.constructors[0].clone();
        assert!(!laws_hold(
            &drops_fields,
            &[discovery.clone(), held_out.clone()],
            100_000
        ));

        let mut selects_wrong_branch = rep.clone();
        selects_wrong_branch.constructors.swap(0, 1);
        assert!(!laws_hold(
            &selects_wrong_branch,
            &[discovery, held_out],
            100_000
        ));
    }

    #[test]
    fn synthesized_recursion_consumes_an_invented_tree_encoding() {
        // Anonymous signature [payload, left×right]. The fixed-point functional
        // computes whether any leaf payload is true.
        let rep = invent(&[1, 2]).unwrap();
        let leaf = |value| construct(&rep, 0, &[church_bool(value)]).unwrap();
        let node = |left, right| construct(&rep, 1, &[left, right]).unwrap();

        // λr.λtree. tree (λpayload.payload)
        //                 (λleft.λright. (r left) true (r right))
        let leaf_handler = term::lam(term::var(0));
        let node_handler = term::lam(term::lam(term::app(
            term::app(term::app(term::var(3), term::var(1)), church_bool(true)),
            term::app(term::var(3), term::var(0)),
        )));
        let functional = term::lam(term::lam(term::app(
            term::app(term::var(0), leaf_handler),
            node_handler,
        )));
        let any = fixpoint::synthesize(&functional).unwrap();

        let shallow = node(leaf(false), leaf(true));
        let deep_false = node(
            node(leaf(false), node(leaf(false), leaf(false))),
            node(leaf(false), leaf(false)),
        );
        let deep_true = node(deep_false.clone(), node(leaf(false), shallow));
        for (tree, expected) in [(leaf(false), false), (deep_false, false), (deep_true, true)] {
            let got = normalize(&term::app(any.clone(), tree), 5_000_000).unwrap();
            let want = normalize(&church_bool(expected), 100).unwrap();
            assert_eq!(got, want);
        }
    }

    #[test]
    fn rejects_empty_signatures_and_wrong_constructor_arities() {
        assert!(invent(&[]).is_none());
        let rep = invent(&[0, 2]).unwrap();
        assert!(construct(&rep, 1, &[church_bool(true)]).is_none());
    }

    #[test]
    fn rejects_malformed_representations_and_incomplete_or_open_probes() {
        let rep = invent(&[0, 1]).unwrap();
        assert!(!laws_hold(&rep, &[], 10_000));

        let valid_probe = RepresentationProbe {
            fields_by_variant: vec![vec![], vec![church_bool(true)]],
            handlers: vec![church_bool(false), term::lam(term::var(0))],
        };
        let mut missing_constructor = rep.clone();
        missing_constructor.constructors.pop();
        assert!(!laws_hold(
            &missing_constructor,
            &[valid_probe.clone()],
            10_000
        ));

        let open_probe = RepresentationProbe {
            fields_by_variant: vec![vec![], vec![term::var(0)]],
            handlers: valid_probe.handlers,
        };
        assert!(!laws_hold(&rep, &[open_probe], 10_000));
    }
}
