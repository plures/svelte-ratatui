//! Runtime bridge for svelte-ratatui.
//!
//! Provides the event loop, state management, and rendering pipeline
//! that connects compiled Svelte widget trees to ratatui's terminal backend.
//!
//! # Quick start
//!
//! ```no_run
//! use svelte_ratatui_runtime::{run, SvelteComponent};
//! use ratatui::{Frame, layout::Rect};
//! use crossterm::event::Event;
//!
//! struct MyComponent;
//!
//! impl SvelteComponent for MyComponent {
//!     fn render(&self, _frame: &mut Frame, _area: Rect) {}
//!     fn handle_event(&mut self, _event: Event) -> bool { false }
//!     fn poll_async(&mut self) -> bool { false }
//! }
//!
//! // run(MyComponent).unwrap();
//! ```

pub mod app;
pub mod events;
pub mod state;

pub use app::{SvelteComponent, run};
