//! svelte-ratatui CLI
//!
//! Usage:
//!   svelte-ratatui compile <input.svelte> -o <output.rs>
//!   svelte-ratatui watch <dir>
//!   svelte-ratatui preview <input.svelte>
//!   svelte-ratatui scaffold [--with-tui] [--no-tui]

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "svelte-ratatui",
    about = "Compile Svelte components to ratatui widget trees and scaffold TUI-capable projects",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Compile a single Svelte component to a Rust widget-tree source file.
    Compile {
        /// Input Svelte source file.
        input: String,
        /// Output Rust source file.
        #[arg(short, long, default_value = "out.rs")]
        output: String,
    },

    /// Watch a directory and recompile on change.
    Watch {
        /// Directory containing Svelte components.
        dir: String,
    },

    /// Preview a component in the terminal without a full Tauri build.
    Preview {
        /// Input Svelte source file.
        input: String,
    },

    /// (Planned) Scaffold TUI integration files into an existing svelte-tauri-template project.
    ///
    /// This command is not yet implemented and currently only reports what it would do.
    /// In a future release it will:
    ///   - Create `plugins/svelte-ratatui/` with Cargo and Vite config
    ///   - Add a `src/lib/components/TuiDemo.svelte` demo component
    ///   - Add a `src/routes/tui-demo/` page route
    ///   - Wire `tauri-plugin-tui` into `src-tauri/Cargo.toml`
    ///
    /// TUI support is enabled by default (matches Plures convention).
    /// Pass `--no-tui` to skip TUI scaffolding when the feature is implemented.
    Scaffold {
        /// Target project directory (defaults to current directory).
        #[arg(default_value = ".")]
        project_dir: String,

        /// Enable TUI support via svelte-ratatui (default: enabled).
        #[arg(long, default_value_t = true, overrides_with = "no_tui")]
        with_tui: bool,

        /// Disable TUI support scaffolding.
        #[arg(long)]
        no_tui: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compile { input, output } => {
            eprintln!("compile: {input} → {output}  (coming soon)");
        }
        Commands::Watch { dir } => {
            eprintln!("watch: {dir}  (coming soon)");
        }
        Commands::Preview { input } => {
            eprintln!("preview: {input}  (coming soon)");
        }
        Commands::Scaffold {
            project_dir,
            with_tui,
            no_tui,
        } => {
            let tui_enabled = with_tui && !no_tui;
            if tui_enabled {
                eprintln!("scaffold: {project_dir}  [TUI enabled]  (coming soon)");
            } else {
                eprintln!("scaffold: {project_dir}  [TUI disabled]  (coming soon)");
            }
        }
    }
}
