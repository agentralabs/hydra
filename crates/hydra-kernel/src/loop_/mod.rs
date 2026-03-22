//! The cognitive loop — Hydra's active processing pipeline.
//!
//! Each cycle: perceive -> route -> prompt -> llm -> deliver.
//! Middlewares hook into 5 points along the pipeline.

pub mod deliver;
pub mod llm;
pub mod middleware;
pub mod middlewares;
pub mod perceive;
pub mod prompt;
pub mod route;
pub mod types;
