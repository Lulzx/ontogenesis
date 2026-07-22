//! Normalization by evaluation, call-by-need, fuel-limited.
//!
//! Semantics mirror the Lamb interpreter (`lam`): normal-order β-reduction
//! to full β-normal form, no η. Fuel counts β-steps so divergent candidates
//! die quickly instead of hanging the search.

use crate::term::{app, lam, var, Term};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
pub enum Val {
    /// A closure: captured environment + body.
    Lam(Env, Rc<Term>),
    /// A neutral: head applied to a spine of (lazy) arguments.
    Neu(Head, Vec<Thunk>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Head {
    /// Bank-context binder (free constant shared across all tests).
    Ctx(u32),
    /// λ-binder entered during quoting, identified by level.
    Bound(u32),
}

pub type Env = Rc<Vec<Thunk>>;

#[derive(Debug)]
pub enum Th {
    Delayed(Env, Rc<Term>),
    Done(Rc<Val>),
    Busy,
}

pub type Thunk = Rc<RefCell<Th>>;

pub fn thunk_of_val(v: Val) -> Thunk {
    Rc::new(RefCell::new(Th::Done(Rc::new(v))))
}

pub fn thunk_delayed(env: Env, t: Rc<Term>) -> Thunk {
    Rc::new(RefCell::new(Th::Delayed(env, t)))
}

/// Evaluation aborted: out of fuel, or a self-forcing thunk (divergence).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Abort;

pub struct Fuel(pub i64);

impl Fuel {
    fn spend(&mut self) -> Result<(), Abort> {
        self.0 -= 1;
        if self.0 < 0 {
            Err(Abort)
        } else {
            Ok(())
        }
    }
}

pub fn force(th: &Thunk, fuel: &mut Fuel) -> Result<Rc<Val>, Abort> {
    let taken = std::mem::replace(&mut *th.borrow_mut(), Th::Busy);
    match taken {
        Th::Done(v) => {
            *th.borrow_mut() = Th::Done(v.clone());
            Ok(v)
        }
        Th::Busy => Err(Abort),
        Th::Delayed(env, t) => match eval(&env, &t, fuel) {
            Ok(v) => {
                *th.borrow_mut() = Th::Done(v.clone());
                Ok(v)
            }
            Err(e) => {
                // Restore so a shared thunk isn't poisoned for later callers.
                *th.borrow_mut() = Th::Delayed(env, t);
                Err(e)
            }
        },
    }
}

pub fn eval(env: &Env, t: &Rc<Term>, fuel: &mut Fuel) -> Result<Rc<Val>, Abort> {
    match t.as_ref() {
        Term::Var(i) => {
            let idx = env.len() - 1 - *i as usize;
            force(&env[idx], fuel)
        }
        Term::Free(i) => Ok(Rc::new(Val::Neu(Head::Ctx(*i), Vec::new()))),
        Term::Lam(b) => Ok(Rc::new(Val::Lam(env.clone(), b.clone()))),
        Term::App(f, a) => {
            let fv = eval(env, f, fuel)?;
            let arg = thunk_delayed(env.clone(), a.clone());
            apply(fv, arg, fuel)
        }
    }
}

pub fn apply(fv: Rc<Val>, arg: Thunk, fuel: &mut Fuel) -> Result<Rc<Val>, Abort> {
    match fv.as_ref() {
        Val::Lam(cenv, body) => {
            fuel.spend()?;
            let mut env2 = (**cenv).clone();
            env2.push(arg);
            eval(&Rc::new(env2), body, fuel)
        }
        Val::Neu(h, sp) => {
            let mut sp2 = sp.clone();
            sp2.push(arg);
            Ok(Rc::new(Val::Neu(*h, sp2)))
        }
    }
}

/// Read a value back into a β-normal term. `depth` counts λ-binders entered
/// during quoting; `Head::Bound(l)` becomes de Bruijn index `depth - 1 - l`.
pub fn quote(v: &Val, depth: u32, fuel: &mut Fuel) -> Result<Rc<Term>, Abort> {
    // Charge fuel per node read back, so normal-form *size* is bounded even
    // when it was cheap to compute (e.g. shared numeral exponentiation).
    fuel.spend()?;
    match v {
        Val::Lam(env, body) => {
            let fresh = thunk_of_val(Val::Neu(Head::Bound(depth), Vec::new()));
            let mut env2 = (**env).clone();
            env2.push(fresh);
            let bv = eval(&Rc::new(env2), body, fuel)?;
            Ok(lam(quote(&bv, depth + 1, fuel)?))
        }
        Val::Neu(h, sp) => {
            let mut t = match h {
                Head::Ctx(i) => Rc::new(Term::Free(*i)),
                Head::Bound(l) => var(depth - 1 - l),
            };
            for th in sp {
                let av = force(th, fuel)?;
                t = app(t, quote(&av, depth, fuel)?);
            }
            Ok(t)
        }
    }
}

/// Normalize a term under an environment: eval then quote.
pub fn normalize(env: &Env, t: &Rc<Term>, fuel: &mut Fuel) -> Result<Rc<Term>, Abort> {
    let v = eval(env, t, fuel)?;
    quote(&v, 0, fuel)
}
