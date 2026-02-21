//! Runtime bridge for svelte-ratatui.
//!
//! Provides the event loop, state management, and rendering pipeline
//! that connects compiled Svelte widget trees to ratatui's terminal backend.

pub mod app;
pub mod events;
pub mod state;
