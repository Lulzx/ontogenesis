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
//! - [`fourier_world`]: Direction M15/M15b recurrence-generated oscillatory
//!   coordinate invention with exact shift dynamics, M9 meta-transfer, and
//!   conditional closed-shift routing.
//! - [`spectral_world`]: Direction M16 toy spectral regularity -- unlabelled
//!   structural predicate invention with exact orthogonal latent directions,
//!   rational decomposition checking, and conditional long-horizon routing.
//! - [`euler_world`]: Direction M17 finite Euler product -- irreducible
//!   inference from multiplication behavior, local-factor invention, and
//!   exact product/special-value transfer.
//! - [`zeta_world`]: Direction M18 toy zeta object -- exact rational special
//!   values, exponent-parameterized local factors, and formal pole/reflection
//!   certificates.
//! - [`functional_world`]: Direction M19 toy functional equation -- affine
//!   reflection discovery and factor-program invention over exact completed
//!   object values.
//! - [`completion_world`]: Direction M20 toy completed object -- auxiliary
//!   completion-factor invention for the maximally simple symmetry
//!   Xi(1-s)=Xi(s).
//! - [`locus_world`]: Direction M21 toy critical locus -- zero-set locus
//!   invention on an integer lattice under reflection and conjugation.
//! - [`hidden_zeros_world`]: Direction M22 hidden toy zeros -- oscillator
//!   invention and location recovery from exact arithmetic signals.
//! - [`conjecture_world`]: Direction M23 toy RH-like conjecture -- frozen
//!   scoring of the strongest supported predicate from partial zero evidence.
//! - [`equivalence_world`]: Direction M24 toy-RH equivalence -- novel
//!   equivalent predicate invention with exhaustive bidirectional proof
//!   certificates.
//! - [`making_object_world`]: Direction M25 toy RH-making object -- a
//!   signal-derived Hankel object whose rank-one property forces toy-RH.

pub mod acquire;
pub mod active_experimentation;
pub mod bank;
pub mod bootstrap;
pub mod canon;
pub mod causal_ontology;
pub mod contextual_allocation;
pub mod contextual_guidance;
pub mod conjecture_world;
pub mod concept_migration;
pub mod completion_world;
pub mod coproduct_property;
pub mod euclid_world;
pub mod equivalence_world;
pub mod euler_world;
pub mod fixpoint;
pub mod functional_world;
pub mod hidden_zeros_world;
pub mod feature_invention;
pub mod initial_algebra;
pub mod irrational_world;
pub mod polynomial_world;
pub mod symmetry_world;
pub mod fourier_world;
pub mod learned_allocation;
pub mod learned_context;
pub mod locus_world;
pub mod making_object_world;
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
pub mod spectral_world;
pub mod term;
pub mod transform;
pub mod typed;
pub mod universal;
pub mod universal_property;
pub mod world_model;
pub mod zeta_world;
pub mod math_world;
