//! Svelte-to-ratatui compiler.
//!
//! Transforms Svelte component ASTs into ratatui widget trees.
//! See design doc: SVELTE-RATATUI-COMPILER.md

pub mod ir;
pub mod mapping;
pub mod pipeline;

pub use pipeline::compile;
