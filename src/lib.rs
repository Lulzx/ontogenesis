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
//! - [`recursive_signature`]: U4 bounded inference of an anonymous polynomial
//!   recursive signature and its carrier, constructor, and mediator generator.
//! - [`open_signature`]: U5 provisional open-world signature ranking and
//!   structural ontology revision from a temporal evidence stream.
//! - [`causal_ontology`]: Direction F causal inference from intervention
//!   responses on a tiny deterministic 3-variable system.
//! - [`world_model`]: Direction G world-model ontogenesis — factored state
//!   compression and invented reversible-counter transfer in a persistent
//!   deterministic world.
//! - [`math_world`]: Directions M1--M9 mathematical ontogenesis -- arithmetic
//!   concept/law invention through latent linear representations.
//! - [`proposition_world`]: Direction M10 answer-blind theorem reformulation
//!   search with independently checked modular proof certificates.
//! - [`euclid_world`]: Direction M11 auxiliary-object invention and a checked
//!   finite-prime-list escape certificate.
//! - [`irrational_world`]: Direction M12 intermediate-representation invention
//!   and a checked prime-valuation contradiction for square roots.
//! - [`polynomial_world`]: Direction M13b permutation-invariant root-feature
//!   invention with exact multivariable factor-ideal checking.
//! - [`symmetry_world`]: Direction M14 generic transformation-action invention,
//!   exact response checking, and measured cross-domain reuse.
//! - [`fourier_world`]: Direction M15 recurrence-generated oscillatory
//!   coordinate invention with exact shift dynamics and M9 meta-transfer.

pub mod acquire;
pub mod active_experimentation;
pub mod bank;
pub mod bootstrap;
pub mod canon;
pub mod causal_ontology;
pub mod contextual_allocation;
pub mod contextual_guidance;
pub mod concept_migration;
pub mod coproduct_property;
pub mod euclid_world;
pub mod fixpoint;
pub mod feature_invention;
pub mod initial_algebra;
pub mod irrational_world;
pub mod polynomial_world;
pub mod symmetry_world;
pub mod fourier_world;
pub mod learned_allocation;
pub mod learned_context;
pub mod nbe;
pub mod ontology_guidance;
pub mod ontology_repair;
pub mod probe_invention;
pub mod open_signature;
pub mod parse;
pub mod proposition_world;
pub mod recurrence;
pub mod recursion_search;
pub mod recursive_signature;
pub mod representation;
pub mod search_accounting;
pub mod term;
pub mod transform;
pub mod typed;
pub mod universal;
pub mod universal_property;
pub mod world_model;
pub mod math_world;
