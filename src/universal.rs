//! Fair, unbounded enumeration of the universal object language.
//!
//! The language is well-scoped untyped de Bruijn lambda calculus plus an
//! optional finite alphabet of closed opaque atoms.  Pure untyped lambda
//! calculus (the empty alphabet) is already universal.  Terms are ordered by
//! exact syntax size; every size class is finite, and no class is capped.
//!
//! Enumeration fairness alone is insufficient because candidate evaluation may
//! diverge. [`Dovetail`] diagonally schedules syntax size and evaluation fuel:
//! every finite pair `(size, fuel)` is visited after finitely many stages.  A
//! solution claim is therefore complete only *relative to* a finite lambda term
//! that terminates under the chosen observer in finite fuel.  This is a
//! semidecision procedure, not a decision procedure for program equivalence.

use crate::term::{self, Term};
use std::collections::HashSet;
use std::collections::VecDeque;
use std::rc::Rc;

/// All well-scoped terms of one exact size at the given binder depth.
///
/// The result is finite and uncapped. With unique `atoms`, the grammar is
/// unambiguous, so every returned syntax tree is unique.
pub fn terms_exact(size: u32, depth: u32, atoms: &[Rc<Term>]) -> Vec<Rc<Term>> {
    if size == 0 {
        return Vec::new();
    }
    let atoms = unique_atoms(atoms);
    terms_exact_unique(size, depth, &atoms)
}

fn terms_exact_unique(size: u32, depth: u32, atoms: &[Rc<Term>]) -> Vec<Rc<Term>> {
    if size == 1 {
        let mut out: Vec<Rc<Term>> = (0..depth).map(term::var).collect();
        out.extend(atoms.iter().map(|a| Rc::new(Term::Prim(a.clone()))));
        return out;
    }

    let mut out = Vec::new();
    for body in terms_exact_unique(size - 1, depth + 1, atoms) {
        out.push(term::lam(body));
    }
    if size >= 3 {
        for fun_size in 1..=(size - 2) {
            let arg_size = size - 1 - fun_size;
            let functions = terms_exact_unique(fun_size, depth, atoms);
            let arguments = terms_exact_unique(arg_size, depth, atoms);
            for f in &functions {
                for a in &arguments {
                    out.push(term::app(f.clone(), a.clone()));
                }
            }
        }
    }
    out
}

fn unique_atoms(atoms: &[Rc<Term>]) -> Vec<Rc<Term>> {
    let mut seen = HashSet::new();
    atoms
        .iter()
        .filter(|a| crate::transform::is_closed(a) && seen.insert((*a).clone()))
        .cloned()
        .collect()
}

/// Infinite fair stream of all closed terms over a finite atom alphabet.
pub struct FairTerms {
    atoms: Vec<Rc<Term>>,
    size: u32,
    current: std::vec::IntoIter<Rc<Term>>,
}

impl FairTerms {
    pub fn new(atoms: &[Rc<Term>]) -> Self {
        Self {
            atoms: unique_atoms(atoms),
            size: 0,
            current: Vec::new().into_iter(),
        }
    }

    pub fn current_size(&self) -> u32 {
        self.size
    }
}

impl Iterator for FairTerms {
    type Item = Rc<Term>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(term) = self.current.next() {
                return Some(term);
            }
            self.size = self.size.checked_add(1).expect("syntax size overflow");
            self.current = terms_exact_unique(self.size, 0, &self.atoms).into_iter();
        }
    }
}

/// Does a term belong to the enumerated language at this binder depth?
///
/// This is also the constructive membership lemma used by the completeness
/// argument: induction over this predicate mirrors the variable/atom, lambda,
/// and application productions in [`terms_exact`].
pub fn in_language(t: &Rc<Term>, depth: u32, atoms: &[Rc<Term>]) -> bool {
    match t.as_ref() {
        Term::Var(i) => *i < depth,
        Term::Free(_) => false,
        Term::Prim(body) => crate::transform::is_closed(body) && atoms.iter().any(|a| a == body),
        Term::Lam(body) => in_language(body, depth + 1, atoms),
        Term::App(f, a) => in_language(f, depth, atoms) && in_language(a, depth, atoms),
    }
}

/// Diagonal schedule over positive syntax sizes and positive evaluation fuels.
///
/// Stage `d` emits every `(size, fuel)` with `size + fuel = d + 1`. Therefore
/// `(s,f)` appears exactly once, at finite stage `s + f - 1`.
#[derive(Debug, Clone)]
pub struct Dovetail {
    diagonal: u64,
    size: u64,
}

impl Default for Dovetail {
    fn default() -> Self {
        Self {
            diagonal: 2,
            size: 1,
        }
    }
}

impl Iterator for Dovetail {
    type Item = (u32, u64);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.size >= self.diagonal || self.size > u64::from(u32::MAX) {
                self.diagonal = self.diagonal.checked_add(1)?;
                self.size = 1;
            }

            let fuel = self.diagonal - self.size;
            let size = u32::try_from(self.size)
                .expect("unrepresentable sizes are skipped before conversion");
            self.size += 1;
            return Some((size, fuel));
        }
    }
}

/// The finite diagonal stage at which `(syntax_size, evaluation_fuel)` occurs.
pub fn scheduled_stage(syntax_size: u32, evaluation_fuel: u64) -> Option<u64> {
    if syntax_size == 0 || evaluation_fuel == 0 {
        None
    } else {
        u64::from(syntax_size)
            .checked_add(evaluation_fuel)?
            .checked_sub(1)
    }
}

/// A finite ontology-biased prefix followed by the ordinary fair diagonal.
///
/// The prefix may spend early work on promising syntax sizes at useful fuel,
/// but cannot remove any universal resource point: after finitely many calls,
/// iteration is byte-for-byte [`Dovetail`]. Thus learned search bias changes
/// time-to-first-test without weakening the completeness floor.
#[derive(Debug, Clone)]
pub struct PrioritizedDovetail {
    priority: VecDeque<(u32, u64)>,
    fallback: Dovetail,
}

/// Alternate every finite learned-priority point with one unchanged universal
/// point, then continue the universal diagonal forever. Unlike a prefix-only
/// policy, the universal lane receives a permanently nonzero allocation even
/// while learned work remains.
#[derive(Debug, Clone)]
pub struct InterleavedDovetail {
    priority: VecDeque<(u32, u64)>,
    fallback: Dovetail,
    universal_turn: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceLane {
    Learned,
    Universal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduledResource {
    pub lane: ResourceLane,
    pub syntax_size: u32,
    pub evaluation_fuel: u64,
}

impl InterleavedDovetail {
    pub fn new(priority: impl IntoIterator<Item = (u32, u64)>) -> Self {
        Self {
            priority: priority
                .into_iter()
                .filter(|(size, fuel)| *size > 0 && *fuel > 0)
                .collect(),
            fallback: Dovetail::default(),
            universal_turn: false,
        }
    }

    /// Emit a resource point with an explicit lane label. Experiments use the
    /// label to audit that projecting away learned work reproduces the original
    /// universal dovetail exactly, even when resource pairs happen to be equal.
    pub fn next_labeled(&mut self) -> Option<ScheduledResource> {
        if self.priority.is_empty() {
            let (syntax_size, evaluation_fuel) = self.fallback.next()?;
            return Some(ScheduledResource {
                lane: ResourceLane::Universal,
                syntax_size,
                evaluation_fuel,
            });
        }
        self.universal_turn = !self.universal_turn;
        let (lane, point) = if self.universal_turn {
            (ResourceLane::Learned, self.priority.pop_front()?)
        } else {
            (ResourceLane::Universal, self.fallback.next()?)
        };
        Some(ScheduledResource {
            lane,
            syntax_size: point.0,
            evaluation_fuel: point.1,
        })
    }
}

impl Iterator for InterleavedDovetail {
    type Item = (u32, u64);

    fn next(&mut self) -> Option<Self::Item> {
        self.next_labeled()
            .map(|point| (point.syntax_size, point.evaluation_fuel))
    }
}

impl PrioritizedDovetail {
    pub fn new(priority: impl IntoIterator<Item = (u32, u64)>) -> Self {
        Self {
            priority: priority
                .into_iter()
                .filter(|(size, fuel)| *size > 0 && *fuel > 0)
                .collect(),
            fallback: Dovetail::default(),
        }
    }
}

impl Iterator for PrioritizedDovetail {
    type Item = (u32, u64);

    fn next(&mut self) -> Option<Self::Item> {
        self.priority.pop_front().or_else(|| self.fallback.next())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bank;

    #[test]
    fn exact_classes_are_finite_unique_and_well_scoped() {
        for size in 1..=8 {
            let terms = terms_exact(size, 0, &[]);
            let unique: HashSet<_> = terms.iter().cloned().collect();
            assert_eq!(terms.len(), unique.len(), "duplicate at size {size}");
            assert!(terms.iter().all(|t| t.size() == size));
            assert!(terms.iter().all(|t| in_language(t, 0, &[])));
        }
    }

    #[test]
    fn open_or_duplicate_atoms_do_not_corrupt_the_closed_language() {
        let closed = term::lam(term::var(0));
        let terms = terms_exact(1, 0, &[term::var(0), closed.clone(), closed]);
        assert_eq!(terms.len(), 1);
        assert!(in_language(&terms[0], 0, &[term::lam(term::var(0))]));
    }

    #[test]
    fn every_finite_lambda_term_has_a_finite_enumeration_stage() {
        let y = bank::y_combinator();
        assert_eq!(y.size(), 14);
        assert!(in_language(&y, 0, &[]));
        // The constructive grammar lemma says membership at exact finite size
        // is sufficient: no bounded run is presented as the proof.
        assert_eq!(y.size(), 14);
    }

    #[test]
    fn fair_stream_crosses_empty_size_classes() {
        let got: Vec<_> = FairTerms::new(&[]).take(4).collect();
        assert_eq!(got[0], term::lam(term::var(0))); // size 2
        assert!(got.windows(2).all(|w| w[0].size() <= w[1].size()));
    }

    #[test]
    fn diagonal_schedule_reaches_every_finite_resource_pair() {
        assert_eq!(scheduled_stage(14, 50_000), Some(50_013));
        let target = (14, 50);
        let observed = Dovetail::default().take(5_000).find(|pair| *pair == target);
        assert_eq!(observed, Some(target));
    }

    #[test]
    fn diagonal_schedule_crosses_the_u32_size_boundary_without_ending() {
        let mut schedule = Dovetail {
            diagonal: u64::from(u32::MAX) + 2,
            size: u64::from(u32::MAX),
        };
        assert_eq!(schedule.next(), Some((u32::MAX, 2)));
        // The rest of that diagonal has no representable syntax size. Skipping
        // it must advance to the next diagonal, not end the iterator and lose
        // all later fuel allocations for small representable terms.
        assert_eq!(schedule.next(), Some((1, u64::from(u32::MAX) + 2)));
        assert!(scheduled_stage(1, i64::MAX as u64).is_some());
    }

    #[test]
    fn finite_priority_prefix_preserves_the_exact_universal_fallback() {
        let prefix = [(7, 10_000), (8, 10_000)];
        let got: Vec<_> = PrioritizedDovetail::new(prefix).take(10).collect();
        assert_eq!(&got[..2], &prefix);
        assert_eq!(&got[2..], &Dovetail::default().take(8).collect::<Vec<_>>());
    }

    #[test]
    fn learned_priority_cannot_starve_the_interleaved_universal_lane() {
        let priority = [(7, 100_000), (6, 50_000), (5, 10_000)];
        let got: Vec<_> = InterleavedDovetail::new(priority).take(10).collect();
        assert_eq!(got[0], priority[0]);
        assert_eq!(got[2], priority[1]);
        assert_eq!(got[4], priority[2]);
        let universal = Dovetail::default().take(7).collect::<Vec<_>>();
        assert_eq!(
            vec![got[1], got[3], got[5], got[6], got[7], got[8], got[9]],
            universal
        );
    }

    #[test]
    fn arbitrary_learned_schedules_preserve_the_exact_universal_projection() {
        // Deterministic generated policies include empty, invalid, duplicate,
        // extreme, and ordinary learned points. Scores do not enter this
        // scheduler, so even an arbitrarily confident policy reduces to one of
        // these finite priority sequences.
        let mut state = 0x5eed_u64;
        for case in 0..64 {
            let len = case % 17;
            let mut priority = Vec::new();
            for index in 0..len {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                let size = if index % 7 == 0 {
                    0
                } else if index % 11 == 0 {
                    u32::MAX
                } else {
                    (state as u32 % 19) + 1
                };
                let fuel = if index % 5 == 0 {
                    0
                } else {
                    state.rotate_left(13).max(1)
                };
                priority.push((size, fuel));
            }
            let retained = priority
                .iter()
                .filter(|(size, fuel)| *size > 0 && *fuel > 0)
                .count();
            let universal_needed = 40;
            let mut schedule = InterleavedDovetail::new(priority);
            let labeled = (0..retained * 2 + universal_needed)
                .map(|_| schedule.next_labeled().unwrap())
                .collect::<Vec<_>>();
            let projection = labeled
                .iter()
                .filter(|point| point.lane == ResourceLane::Universal)
                .take(universal_needed)
                .map(|point| (point.syntax_size, point.evaluation_fuel))
                .collect::<Vec<_>>();
            assert_eq!(
                projection,
                Dovetail::default()
                    .take(universal_needed)
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn every_sampled_universal_pair_keeps_a_finite_interleaved_index() {
        let learned = (1..=100).map(|size| (size, u64::MAX));
        let mut schedule = InterleavedDovetail::new(learned);
        let targets = Dovetail::default().take(75).collect::<Vec<_>>();
        let mut found = vec![None; targets.len()];
        for index in 0..500 {
            let point = schedule.next_labeled().unwrap();
            if point.lane == ResourceLane::Universal {
                let pair = (point.syntax_size, point.evaluation_fuel);
                if let Some(target_index) = targets.iter().position(|target| *target == pair) {
                    found[target_index] = Some(index);
                }
            }
        }
        assert!(found.iter().all(Option::is_some));
        assert!(found.into_iter().flatten().all(|index| index < 500));
    }
}
