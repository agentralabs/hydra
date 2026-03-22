//! `hydra-axiom` — Universal reasoning primitives.
//!
//! The domain-free axiom category C_axiom provides the shared
//! vocabulary for all of Hydra's reasoning. Domain-specific concepts
//! are mapped into this universal space via functors.

pub mod constants;
pub mod errors;
pub mod functor;
pub mod morphisms;
pub mod primitives;
pub mod synthesis;

pub use errors::AxiomError;
pub use functor::{
    CrossDomainPattern, DeploymentFunctor, DomainFunctor, FinanceFunctor, FunctorMapping,
    FunctorRegistry,
};
pub use morphisms::AxiomMorphism;
pub use primitives::AxiomPrimitive;
pub use synthesis::{synthesize, SynthesizedCapability};
