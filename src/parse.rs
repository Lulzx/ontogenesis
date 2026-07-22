//! Parser for Lamb terms as they appear in .tsk files (test expressions and
//! expected outputs), and the .tsk task format itself.

use crate::term::{app, lam, var, Term};
use std::rc::Rc;

/// Free constants standing for a test's outer binders (the `λA.λB.` wrappers
/// around the @main call) live at `Term::Free(TEST_FREE_BASE + level)`, far
/// above the bank's own context constants.
pub const TEST_FREE_BASE: u32 = 1 << 20;

#[derive(Debug, Clone)]
pub struct Test {
    /// Arguments applied to @main. Test exprs have the shape
    /// `λA1...λAm. @main(a1, ..., ak)`; the outer binders appear in the args
    /// as `Free(TEST_FREE_BASE + level)` constants.
    pub args: Vec<Rc<Term>>,
    /// Expected output, as written (a closed term; normalized by the caller).
    pub want: Rc<Term>,
    /// Number of outer binders m wrapped around the @main call.
    pub outer: u32,
}

#[derive(Debug)]
pub struct Task {
    pub id: String,
    pub desc: String,
    pub tests: Vec<Test>,
    pub arity: usize,
}

#[derive(Debug)]
pub enum TaskError {
    Parse(String),
    /// Test expressions don't fit the directed-search shape v0 supports.
    Unsupported(String),
}

struct P<'a> {
    s: &'a [char],
    i: usize,
}

impl<'a> P<'a> {
    fn skip(&mut self) {
        while self.i < self.s.len() {
            let c = self.s[self.i];
            if c == '/' && self.s.get(self.i + 1) == Some(&'/') {
                while self.i < self.s.len() && self.s[self.i] != '\n' {
                    self.i += 1;
                }
            } else if c == ' ' || c == '\n' || c == '\t' || c == '\r' {
                self.i += 1;
            } else {
                break;
            }
        }
    }

    fn expect(&mut self, c: char) -> Result<(), String> {
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("expected '{c}' at {}", self.i))
        }
    }

    fn name(&mut self) -> String {
        let mut out = String::new();
        while let Some(&c) = self.s.get(self.i) {
            if c.is_ascii_alphanumeric() || c == '_' {
                out.push(c);
                self.i += 1;
            } else {
                break;
            }
        }
        out
    }

    /// Parse a term. Variables resolve against `ctx` (innermost last).
    /// `@name` references are only allowed for names in `refs_ok`; they parse
    /// into the placeholder returned by `on_ref`.
    fn term(&mut self, ctx: &mut Vec<String>) -> Result<Expr, String> {
        self.skip();
        match self.s.get(self.i) {
            Some('λ') => {
                self.i += 1;
                let n = self.name();
                self.expect('.')?;
                ctx.push(n);
                let body = self.term(ctx)?;
                ctx.pop();
                self.calls(Expr::Lam(Box::new(body)), ctx)
            }
            Some('@') => {
                self.i += 1;
                let n = self.name();
                self.calls(Expr::Ref(n), ctx)
            }
            Some('(') => {
                self.i += 1;
                let inner = self.term(ctx)?;
                self.skip();
                self.expect(')')?;
                self.calls(inner, ctx)
            }
            _ => {
                let n = self.name();
                if n.is_empty() {
                    return Err(format!("unexpected char at {}", self.i));
                }
                let pos = ctx
                    .iter()
                    .rposition(|b| *b == n)
                    .ok_or_else(|| format!("unbound variable '{n}'"))?;
                let idx = (ctx.len() - 1 - pos) as u32;
                self.calls(Expr::Var(idx), ctx)
            }
        }
    }

    fn calls(&mut self, mut f: Expr, ctx: &mut Vec<String>) -> Result<Expr, String> {
        loop {
            if self.s.get(self.i) != Some(&'(') {
                return Ok(f);
            }
            self.i += 1;
            let mut args = Vec::new();
            loop {
                self.skip();
                if self.s.get(self.i) == Some(&')') {
                    self.i += 1;
                    break;
                }
                if !args.is_empty() {
                    self.expect(',')?;
                }
                args.push(self.term(ctx)?);
            }
            for a in args {
                f = Expr::App(Box::new(f), Box::new(a));
            }
        }
    }
}

/// Surface expression: like Term but with named @refs still present.
#[derive(Debug)]
pub enum Expr {
    Var(u32),
    Lam(Box<Expr>),
    App(Box<Expr>, Box<Expr>),
    Ref(String),
}

pub fn parse_expr(src: &str) -> Result<Expr, String> {
    let chars: Vec<char> = src.chars().collect();
    let mut p = P { s: &chars, i: 0 };
    let mut ctx = Vec::new();
    let e = p.term(&mut ctx)?;
    p.skip();
    if p.i != chars.len() {
        return Err(format!("trailing input at {}", p.i));
    }
    Ok(e)
}

/// Lower an Expr containing no @refs into a closed Term.
pub fn to_term(e: &Expr) -> Result<Rc<Term>, String> {
    match e {
        Expr::Var(i) => Ok(var(*i)),
        Expr::Lam(b) => Ok(lam(to_term(b)?)),
        Expr::App(f, a) => Ok(app(to_term(f)?, to_term(a)?)),
        Expr::Ref(n) => Err(format!("unexpected reference @{n}")),
    }
}

/// Parse a .tsk file. Enforces the shape v0's directed search needs:
/// every test is `@main(a1, ..., ak)` with closed args and consistent arity.
pub fn parse_task(id: &str, text: &str) -> Result<Task, TaskError> {
    let secs: Vec<&str> = text.split("\n---\n").collect();
    if secs.len() != 2 {
        return Err(TaskError::Parse(format!(
            "{id}: expected 2 sections, got {}",
            secs.len()
        )));
    }
    let desc = secs[0].trim().to_string();
    let lines: Vec<&str> = secs[1]
        .trim()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();

    let mut tests = Vec::new();
    let mut arity: Option<usize> = None;
    let mut i = 0;
    while i < lines.len() {
        let expr_src = lines[i].trim();
        let want_line = lines
            .get(i + 1)
            .ok_or_else(|| TaskError::Parse(format!("{id}: missing '=' line")))?;
        if !want_line.trim_start().starts_with("= ") {
            return Err(TaskError::Parse(format!("{id}: expected '= ...' line")));
        }
        let want_src = want_line.trim_start()[2..].trim();

        let expr = parse_expr(expr_src).map_err(|e| TaskError::Parse(format!("{id}: {e}")))?;
        let (outer, args) = analyze_test(&expr).ok_or_else(|| {
            TaskError::Unsupported(format!("{id}: test is not λ*.@main(args...)"))
        })?;
        match arity {
            None => arity = Some(args.len()),
            Some(k) if k == args.len() => {}
            Some(k) => {
                return Err(TaskError::Unsupported(format!(
                    "{id}: inconsistent arity {k} vs {}",
                    args.len()
                )))
            }
        }
        let want = parse_expr(want_src)
            .and_then(|e| to_term(&e))
            .map_err(|e| TaskError::Parse(format!("{id}: {e}")))?;
        tests.push(Test { args, want, outer });
        i += 2;
    }

    let arity = arity.ok_or_else(|| TaskError::Parse(format!("{id}: no tests")))?;
    Ok(Task {
        id: id.to_string(),
        desc,
        tests,
        arity,
    })
}

/// If `e` is `λA1...λAm. @main(a1, ..., ak)`, return `(m, args)` where the
/// args are lowered to Terms in which references to the outer binders become
/// `Free(TEST_FREE_BASE + level)` constants (A1, the outermost, is level 0).
fn analyze_test(e: &Expr) -> Option<(u32, Vec<Rc<Term>>)> {
    let mut m = 0u32;
    let mut cur = e;
    while let Expr::Lam(b) = cur {
        m += 1;
        cur = b.as_ref();
    }
    let mut args_rev = Vec::new();
    loop {
        match cur {
            Expr::App(f, a) => {
                args_rev.push(a.as_ref());
                cur = f.as_ref();
            }
            Expr::Ref(n) if n == "main" => break,
            _ => return None,
        }
    }
    let mut out = Vec::new();
    for a in args_rev.iter().rev() {
        out.push(lower_with_outer(a, m, 0)?);
    }
    Some((m, out))
}

/// Lower an Expr to a Term, mapping variables that escape to the m stripped
/// outer binders onto Free constants. `d` = λ-depth inside this subterm.
fn lower_with_outer(e: &Expr, m: u32, d: u32) -> Option<Rc<Term>> {
    match e {
        Expr::Var(i) => {
            if *i < d {
                Some(var(*i))
            } else {
                let level = m.checked_sub(1 + (*i - d))?;
                Some(Rc::new(Term::Free(TEST_FREE_BASE + level)))
            }
        }
        Expr::Lam(b) => Some(lam(lower_with_outer(b, m, d + 1)?)),
        Expr::App(f, a) => Some(app(
            lower_with_outer(f, m, d)?,
            lower_with_outer(a, m, d)?,
        )),
        Expr::Ref(_) => None,
    }
}

/// Strip m outer lambdas from a (normalized, closed) expected term and
/// substitute the freed binders with the same Free constants used in the
/// test args. Returns None if the term has fewer than m leading lambdas
/// (in which case no candidate can ever match).
pub fn strip_outer(t: &Rc<Term>, m: u32) -> Option<Rc<Term>> {
    let mut cur = t;
    for _ in 0..m {
        match cur.as_ref() {
            Term::Lam(b) => cur = b,
            _ => return None,
        }
    }
    Some(subst_escaping(cur, m, 0))
}

fn subst_escaping(t: &Rc<Term>, m: u32, d: u32) -> Rc<Term> {
    match t.as_ref() {
        Term::Var(i) => {
            if *i < d {
                t.clone()
            } else {
                let level = m - 1 - (*i - d);
                Rc::new(Term::Free(TEST_FREE_BASE + level))
            }
        }
        Term::Free(_) => t.clone(),
        Term::Lam(b) => lam(subst_escaping(b, m, d + 1)),
        Term::App(f, a) => app(subst_escaping(f, m, d), subst_escaping(a, m, d)),
    }
}
