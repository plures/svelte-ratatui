//! Svelte-to-ratatui compiler.
//!
//! Transforms Svelte component ASTs into ratatui widget trees.
//! See design doc: docs/RUNES-TRANSLATION.md

pub mod dialect_check;
pub mod ir;
pub mod mapping;
pub mod pipeline;

pub use dialect_check::check as check_dialect;
pub use pipeline::compile;
