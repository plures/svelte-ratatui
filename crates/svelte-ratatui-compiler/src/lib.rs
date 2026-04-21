//! Svelte-to-ratatui compiler.
//!
//! Transforms Svelte component ASTs into ratatui widget trees.
//! See design doc: docs/RUNES-TRANSLATION.md

pub mod codegen;
pub mod dialect_check;
pub mod ir;
pub mod mapping;
pub mod pipeline;

pub use dialect_check::check as check_dialect;
pub use ir::{IrColor, IrElement, IrModifier, IrNode, IrStyle};
pub use mapping::render_ir;
pub use pipeline::compile;
