//! Concrete provider implementations.
//!
//! This crate is imported only by `api/` and executable entrypoints.

pub mod openai;

pub use openai::OpenAiAdapter;
