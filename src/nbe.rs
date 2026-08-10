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

thread_local! {
    static STACK_BASE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

// Ablation metering: coarse counters for where the "materializing semantic
// values" wall physically sits. Off by default (near-zero overhead when off);
// enabled only by the `--ablation` probe so the hot search paths stay untouched.
thread_local! {
    static METER_ON: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static BETA_STEPS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static QUOTE_NODES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static EVAL_ABORTS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

pub fn meter_on(on: bool) {
    METER_ON.with(|m| m.set(on));
}
pub fn meter_reset() {
    BETA_STEPS.with(|m| m.set(0));
    QUOTE_NODES.with(|m| m.set(0));
    EVAL_ABORTS.with(|m| m.set(0));
}
pub fn beta_steps() -> u64 {
    BETA_STEPS.with(|m| m.get())
}
pub fn quote_nodes() -> u64 {
    QUOTE_NODES.with(|m| m.get())
}
pub fn eval_aborts() -> u64 {
    EVAL_ABORTS.with(|m| m.get())
}

#[inline]
fn meter_beta() {
    if METER_ON.with(|m| m.get()) {
        BETA_STEPS.with(|m| m.set(m.get() + 1));
    }
}
#[inline]
fn meter_quote() {
    if METER_ON.with(|m| m.get()) {
        QUOTE_NODES.with(|m| m.set(m.get() + 1));
    }
}

/// Abort before the recursion blows the worker's 1GB stack: measure stack
/// growth from the first eval on this thread and bail past ~700MB.
#[inline]
fn stack_guard() -> Result<(), Abort> {
    let here = {
        let probe = 0u8;
        &probe as *const u8 as usize
    };
    STACK_BASE.with(|b| {
        let base = b.get();
        if base == 0 {
            b.set(here);
            Ok(())
        } else if base.saturating_sub(here) > 700_000_000 {
            Err(Abort)
        } else {
            Ok(())
        }
    })
}

pub fn eval(env: &Env, t: &Rc<Term>, fuel: &mut Fuel) -> Result<Rc<Val>, Abort> {
    stack_guard()?;
    match t.as_ref() {
        Term::Var(i) => {
            let idx = env.len() - 1 - *i as usize;
            force(&env[idx], fuel)
        }
        Term::Free(i) => Ok(Rc::new(Val::Neu(Head::Ctx(*i), Vec::new()))),
        Term::Lam(b) => Ok(Rc::new(Val::Lam(env.clone(), b.clone()))),
        Term::Prim(b) => eval(env, b, fuel),
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
            meter_beta();
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
    meter_quote();
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

/// Stream a value's normal form into a hasher without materializing it.
pub fn quote_hash<H: std::hash::Hasher>(
    v: &Val,
    depth: u32,
    fuel: &mut Fuel,
    h: &mut H,
) -> Result<(), Abort> {
    fuel.spend()?;
    meter_quote();
    match v {
        Val::Lam(env, body) => {
            h.write_u8(0);
            let fresh = thunk_of_val(Val::Neu(Head::Bound(depth), Vec::new()));
            let mut env2 = (**env).clone();
            env2.push(fresh);
            let bv = eval(&Rc::new(env2), body, fuel)?;
            quote_hash(&bv, depth + 1, fuel, h)
        }
        Val::Neu(head, sp) => {
            match head {
                Head::Ctx(i) => {
                    h.write_u8(1);
                    h.write_u32(*i);
                }
                Head::Bound(l) => {
                    h.write_u8(2);
                    h.write_u32(depth - 1 - l);
                }
            }
            for th in sp {
                h.write_u8(3);
                let av = force(th, fuel)?;
                quote_hash(&av, depth, fuel, h)?;
            }
            h.write_u8(4);
            Ok(())
        }
    }
}

/// Structurally compare a value's normal form against a target term while
/// quoting, without materializing the normal form.
pub fn quote_eq(v: &Val, t: &Term, depth: u32, fuel: &mut Fuel) -> Result<bool, Abort> {
    fuel.spend()?;
    match (v, t) {
        (Val::Lam(env, body), Term::Lam(tb)) => {
            let fresh = thunk_of_val(Val::Neu(Head::Bound(depth), Vec::new()));
            let mut env2 = (**env).clone();
            env2.push(fresh);
            let bv = eval(&Rc::new(env2), body, fuel)?;
            quote_eq(&bv, tb, depth + 1, fuel)
        }
        (Val::Neu(head, sp), _) => {
            // Peel the target's application spine to match ours.
            let mut targs: Vec<&Term> = Vec::new();
            let mut cur = t;
            while let Term::App(f, a) = cur {
                targs.push(a);
                cur = f;
            }
            targs.reverse();
            if targs.len() != sp.len() {
                return Ok(false);
            }
            let head_ok = match (head, cur) {
                (Head::Ctx(i), Term::Free(j)) => i == j,
                (Head::Bound(l), Term::Var(x)) => depth - 1 - l == *x,
                _ => false,
            };
            if !head_ok {
                return Ok(false);
            }
            for (th, ta) in sp.iter().zip(targs) {
                let av = force(th, fuel)?;
                if !quote_eq(&av, ta, depth, fuel)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}
