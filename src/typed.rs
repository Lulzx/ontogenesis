//! Small simply-typed, beta-normal proposal enumeration.
//!
//! Types prune meaningless applications; they do not encode target operations.
//! The same generator searches for any requested interface from the currently
//! acquired atoms.  In particular it contains no productions named map,
//! reverse, append, fold, or reduce.

use crate::term::{self, Term};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Atom(u32),
    Arrow(Box<Type>, Box<Type>),
}

impl Type {
    pub fn arrow(a: Type, b: Type) -> Type {
        Type::Arrow(Box::new(a), Box::new(b))
    }
}

#[derive(Debug, Clone)]
pub struct Atom {
    pub body: Rc<Term>,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct Found {
    pub term: Rc<Term>,
    pub size: u32,
    pub generated: u64,
}

#[derive(Debug, Clone)]
pub struct Enumeration {
    pub terms: Vec<Rc<Term>>,
    /// Terms materialized across all memoized type/context/size cells.
    pub generated: u64,
    pub max_size: u32,
    pub per_cell_cap: usize,
    /// At least one otherwise-new inhabitant may have been excluded by a cap.
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Key {
    target: Type,
    context: Vec<Type>,
    size: u32,
}

struct Enumerator<'a> {
    atoms: &'a [Atom],
    memo: HashMap<Key, Vec<Rc<Term>>>,
    generated: u64,
    per_cell_cap: usize,
    truncated: bool,
}

/// Search closed beta-normal terms by increasing syntax size.
pub fn find_closed(
    target: &Type,
    atoms: &[Atom],
    max_size: u32,
    per_cell_cap: usize,
    mut accepts: impl FnMut(&Rc<Term>) -> bool,
) -> Option<Found> {
    let mut e = Enumerator {
        atoms,
        memo: HashMap::new(),
        generated: 0,
        per_cell_cap,
        truncated: false,
    };
    for size in 1..=max_size {
        for candidate in e.terms(target, &[], size) {
            if accepts(&candidate) {
                return Some(Found {
                    term: candidate,
                    size,
                    generated: e.generated,
                });
            }
        }
    }
    None
}

/// Enumerate every distinct closed beta-normal term within the declared size
/// and per-cell boundaries. This is used when a verifier must quantify over
/// all bounded inhabitants (for example, observational mediator uniqueness)
/// rather than stop at the first accepted program.
pub fn enumerate_closed(
    target: &Type,
    atoms: &[Atom],
    max_size: u32,
    per_cell_cap: usize,
) -> Enumeration {
    let mut e = Enumerator {
        atoms,
        memo: HashMap::new(),
        generated: 0,
        per_cell_cap,
        truncated: false,
    };
    let mut terms = Vec::new();
    for size in 1..=max_size {
        for candidate in e.terms(target, &[], size) {
            if !terms.contains(&candidate) {
                terms.push(candidate);
            }
        }
    }
    Enumeration {
        terms,
        generated: e.generated,
        max_size,
        per_cell_cap,
        truncated: e.truncated,
    }
}

impl Enumerator<'_> {
    fn terms(&mut self, target: &Type, context: &[Type], size: u32) -> Vec<Rc<Term>> {
        let key = Key {
            target: target.clone(),
            context: context.to_vec(),
            size,
        };
        if let Some(found) = self.memo.get(&key) {
            return found.clone();
        }

        let mut out = Vec::new();
        if let Type::Arrow(arg, result) = target {
            if size >= 2 {
                let mut inner = context.to_vec();
                inner.push((**arg).clone());
                for body in self.terms(result, &inner, size - 1) {
                    if !push_unique(&mut out, term::lam(body), self.per_cell_cap) {
                        self.truncated = true;
                    }
                }
            }
        }

        // Neutral forms may themselves have function type (e.g. an acquired
        // higher-order primitive passed as an argument) and need not be
        // eta-expanded.
        let mut heads: Vec<(Rc<Term>, Type)> = self
            .atoms
            .iter()
            .map(|a| (Rc::new(Term::Prim(a.body.clone())), a.ty.clone()))
            .collect();
        heads.extend(
            context
                .iter()
                .rev()
                .enumerate()
                .map(|(i, ty)| (term::var(i as u32), ty.clone())),
        );

        for (head, head_ty) in heads {
            if let Some(args) = arguments_to(head_ty, target) {
                let overhead = 1 + args.len() as u32;
                if size < overhead + args.len() as u32 {
                    continue;
                }
                let argument_budget = size - overhead;
                for sizes in positive_compositions(argument_budget, args.len()) {
                    let choices: Vec<Vec<Rc<Term>>> = args
                        .iter()
                        .zip(sizes)
                        .map(|(ty, n)| self.terms(ty, context, n))
                        .collect();
                    if choices.iter().any(Vec::is_empty) {
                        continue;
                    }
                    let mut products = vec![Vec::<Rc<Term>>::new()];
                    for choice in choices {
                        let mut next = Vec::new();
                        for prefix in &products {
                            for item in &choice {
                                let mut p = prefix.clone();
                                p.push(item.clone());
                                next.push(p);
                                if next.len() >= self.per_cell_cap {
                                    self.truncated = true;
                                    break;
                                }
                            }
                            if next.len() >= self.per_cell_cap {
                                break;
                            }
                        }
                        products = next;
                    }
                    for product in products {
                        let candidate = product.into_iter().fold(head.clone(), term::app);
                        if !push_unique(&mut out, candidate, self.per_cell_cap) {
                            self.truncated = true;
                        }
                        if out.len() >= self.per_cell_cap {
                            break;
                        }
                    }
                }
            }
        }
        self.generated += out.len() as u64;
        self.memo.insert(key, out.clone());
        out
    }
}

fn arguments_to(mut ty: Type, target: &Type) -> Option<Vec<Type>> {
    let mut args = Vec::new();
    loop {
        if &ty == target {
            return Some(args);
        }
        match ty {
            Type::Arrow(a, b) => {
                args.push(*a);
                ty = *b;
            }
            Type::Atom(_) => return None,
        }
    }
}

fn positive_compositions(total: u32, parts: usize) -> Vec<Vec<u32>> {
    fn go(total: u32, parts: usize, prefix: &mut Vec<u32>, out: &mut Vec<Vec<u32>>) {
        if parts == 0 {
            if total == 0 {
                out.push(prefix.clone());
            }
            return;
        }
        for first in 1..=total.saturating_sub(parts as u32 - 1) {
            prefix.push(first);
            go(total - first, parts - 1, prefix, out);
            prefix.pop();
        }
    }
    let mut out = Vec::new();
    go(total, parts, &mut Vec::new(), &mut out);
    out
}

fn push_unique(out: &mut Vec<Rc<Term>>, term: Rc<Term>, cap: usize) -> bool {
    if out.contains(&term) {
        true
    } else if out.len() < cap {
        out.push(term);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumerates_identity_in_beta_normal_form() {
        let a = Type::Atom(0);
        let found = find_closed(&Type::arrow(a.clone(), a), &[], 2, 100, |_| true).unwrap();
        assert_eq!(found.term, term::lam(term::var(0)));
        assert_eq!(found.size, 2);
    }

    #[test]
    fn bounded_enumeration_returns_all_sizes_deterministically() {
        let a = Type::Atom(0);
        let first = enumerate_closed(&Type::arrow(a.clone(), a.clone()), &[], 4, 100);
        let second = enumerate_closed(&Type::arrow(a.clone(), a.clone()), &[], 4, 100);
        assert_eq!(first.terms, second.terms);
        assert_eq!(first.generated, second.generated);
        assert!(!first.truncated);
        assert!(first.terms.contains(&term::lam(term::var(0))));
        assert!(first.terms.iter().all(|term| term.size() <= 4));

        let capped = enumerate_closed(
            &Type::arrow(a.clone(), Type::arrow(a.clone(), a)),
            &[],
            3,
            1,
        );
        assert!(capped.truncated);
    }
}
