use std::fmt::Write as _;
use std::rc::Rc;

/// A pure λ-term in de Bruijn form.
///
/// `Free(i)` appears only inside normal-form *keys*: it stands for the i-th
/// context binder of the bank level a term was enumerated under. Candidate
/// solutions and test terms are always closed (no `Free`).
///
/// `Prim(body)` is a named primitive: a cost-1 leaf (it *is* the concept) whose
/// value is the normal form of the closed `body` it embeds. This is how a
/// mined/invented abstraction is "thought through" — using `Prim` costs 1
/// instead of re-deriving the body's expansion.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Term {
    Var(u32),
    Lam(Rc<Term>),
    App(Rc<Term>, Rc<Term>),
    Free(u32),
    /// A primitive concept: atomic (size 1), value = β-normal form of `body`.
    Prim(Rc<Term>),
}

impl Term {
    pub fn size(&self) -> u32 {
        match self {
            Term::Var(_) | Term::Free(_) | Term::Prim(_) => 1,
            Term::Lam(b) => 1 + b.size(),
            Term::App(f, a) => 1 + f.size() + a.size(),
        }
    }
}

pub fn var(i: u32) -> Rc<Term> {
    Rc::new(Term::Var(i))
}
pub fn lam(b: Rc<Term>) -> Rc<Term> {
    Rc::new(Term::Lam(b))
}
pub fn app(f: Rc<Term>, a: Rc<Term>) -> Rc<Term> {
    Rc::new(Term::App(f, a))
}

/// Canonical variable names by λ-depth: a, b, ..., z, aa, ab, ...
/// Mirrors `name_of` in the Lamb interpreter exactly.
pub fn name_of(n: u32) -> String {
    if n < 26 {
        ((b'a' + n as u8) as char).to_string()
    } else {
        format!("{}{}", name_of(n / 26 - 1), (b'a' + (n % 26) as u8) as char)
    }
}

/// Print a closed term exactly the way the Lamb interpreter prints normal
/// forms: canonical depth names, `f(x, y)` application spines, and parens
/// around a λ in head position.
pub fn show(term: &Term) -> String {
    let mut out = String::new();
    show_go(term, 0, &mut out);
    out
}

fn show_go(term: &Term, d: u32, out: &mut String) {
    match term {
        Term::Var(i) => {
            out.push_str(&name_of(d - 1 - i));
        }
        Term::Free(i) => {
            // Only reachable when printing bank keys during debugging.
            let _ = write!(out, "#{i}");
        }
        Term::Prim(b) => {
            // A named concept atom: print its body in braces so its compact
            // cost is visible while the expansion stays readable.
            let _ = write!(out, "{{{}}}", show(b));
        }
        Term::Lam(b) => {
            let _ = write!(out, "λ{}.", name_of(d));
            show_go(b, d + 1, out);
        }
        Term::App(_, _) => {
            let (fun, args) = unapp(term);
            if matches!(fun, Term::Lam(_)) {
                out.push('(');
                show_go(fun, d, out);
                out.push(')');
            } else {
                show_go(fun, d, out);
            }
            out.push('(');
            for (n, a) in args.iter().enumerate() {
                if n > 0 {
                    out.push_str(", ");
                }
                show_go(a, d, out);
            }
            out.push(')');
        }
    }
}

fn unapp(term: &Term) -> (&Term, Vec<&Term>) {
    let mut args = Vec::new();
    let mut t = term;
    while let Term::App(f, a) = t {
        args.push(a.as_ref());
        t = f.as_ref();
    }
    args.reverse();
    (t, args)
}
