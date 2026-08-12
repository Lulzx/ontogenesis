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
//! - [`fixpoint`]: pure-lambda fixed-point and mutual-recursion synthesis.
//! - [`recursion_search`]: fair universal search for recursive functionals.
//! - [`representation`]: invention of anonymous data encodings and eliminators.
//! - [`universal`]: uncapped lambda-term enumeration with resource dovetailing.
//! - [`ontology_guidance`]: measured ontology-biased recursive discovery with
//!   the fair universal schedule retained as a fallback.
//! - [`learned_allocation`]: utility-ledger learning and deterministic resource
//!   allocation over ontology concepts.
//! - [`search_accounting`]: labeled cross-engine experiment accounting without
//!   conflating universal enumeration and behavior-bank work units.
//! - [`contextual_allocation`]: frozen context- and interaction-aware utility
//!   policies with explicit leakage rejection.
//! - [`contextual_guidance`]: heterogeneous recursive holdouts that measure
//!   contextual allocation against global, uniform, oracle, and controls.
//! - [`learned_context`]: regret-selected representations of raw observable
//!   task structure, frozen before they condition contextual allocation.
//! - [`feature_invention`]: executable context predicates synthesized from a
//!   lower-level raw tree substrate and retained by allocation regret.
//! - [`universal_property`]: bounded relational discovery and validation of an
//!   anonymous product-like universal factorization in pure lambda calculus.

pub mod acquire;
pub mod bank;
pub mod bootstrap;
pub mod canon;
pub mod contextual_allocation;
pub mod contextual_guidance;
pub mod coproduct_property;
pub mod fixpoint;
pub mod feature_invention;
pub mod learned_allocation;
pub mod learned_context;
pub mod nbe;
pub mod ontology_guidance;
pub mod parse;
pub mod recurrence;
pub mod recursion_search;
pub mod representation;
pub mod search_accounting;
pub mod term;
pub mod transform;
pub mod typed;
pub mod universal;
pub mod universal_property;
