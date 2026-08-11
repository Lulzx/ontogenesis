//! The supsearch engine: behavior-keyed λ-term synthesis with concept acquisition.
//!
//! This is the general mechanism layer, exposed as a library so experiment drivers
//! (e.g. `demo/arc-1/`) can reuse it without living inside the binary. The binary
//! (`main.rs`) also declares these modules directly; each crate compiles its own
//! copy, which is fine — nothing here is stateful across crates.
//!
//! Modules:
//! - [`term`]: λ-terms (de Bruijn) as `Rc<Term>`, plus `lam`/`app`/`var` builders.
//! - [`parse`]: parser from source strings to `Expr`/`Term`, and `Task`/`Test` types.
//! - [`nbe`]: normalizer with beta/quote/eval meters (the value-execution layer).
//! - [`canon`]: canonical observation (Church-numeral / structural-hash value keys).
//! - [`bank`]: the search — raw `solve` plus the quotient-aware `concept_solve`.
//! - [`bootstrap`]: probe universes, compression mining, Church-numeral encoding.
//! - [`transform`]: generic program-transformation meta-search (context abstraction
//!   + composition) — the machinery that invents primitives by restructuring code.
//! - [`recurrence`]: generic invariant-context induction across finite unrollings.
//! - [`typed`]: operation-blind simply-typed beta-normal proposal enumeration.

pub mod acquire;
pub mod bank;
pub mod bootstrap;
pub mod canon;
pub mod nbe;
pub mod parse;
pub mod recurrence;
pub mod term;
pub mod transform;
pub mod typed;
