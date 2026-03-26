//! svelte-ratatui-adapter — Runtime TUI backend for Tauri apps.
//!
//! This crate provides the bridge between a headless Tauri webview and a
//! terminal rendered by ratatui. The flow:
//!
//! 1. Tauri app starts in `--tui terminal` mode (no visible window)
//! 2. Svelte app runs normally in the hidden webview (with `tui=true`)
//! 3. This adapter:
//!    - Reads the DOM from the webview (via `eval()` / JS serialization)
//!    - Parses HTML into [`IrNode`] trees
//!    - Renders to terminal via ratatui (using the `mapping` module)
//!    - Captures terminal input and dispatches back as DOM events
//!
//! The adapter is designed to be embedded as a Tauri plugin so that any
//! svelte-tauri-template app gets `--tui terminal` for free.

pub mod dom_reader;
pub mod html_parser;
pub mod input;

pub use dom_reader::DomSnapshot;
pub use html_parser::parse_html;
