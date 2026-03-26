//! tauri-plugin-tui — Render a Tauri + Svelte app in the terminal.
//!
//! # Usage (in your Tauri app's `src-tauri/src/main.rs`)
//!
//! ```rust,ignore
//! fn main() {
//!     let mut builder = tauri::Builder::default();
//!
//!     // Check if --tui flag is present
//!     let args: Vec<String> = std::env::args().collect();
//!     if args.contains(&"--tui".to_string()) {
//!         builder = builder.plugin(tauri_plugin_tui::init());
//!     }
//!
//!     builder
//!         .run(tauri::generate_context!())
//!         .expect("error running app");
//! }
//! ```
//!
//! When the plugin is active:
//! 1. The main window is hidden
//! 2. The terminal is initialized with ratatui/crossterm
//! 3. A render loop polls the webview DOM and draws to the terminal
//! 4. Terminal input is forwarded back to the webview as DOM events
//! 5. Ctrl+C or 'q' exits the app

mod plugin;

pub use plugin::init;
