//! svelte-ratatui CLI
//!
//! Commands:
//!   svelte-ratatui build   [--dir <dir>] [--out-dir <dir>] [--verbose]
//!   svelte-ratatui check   [--dir <dir>] [--verbose]
//!   svelte-ratatui dev     [--dir <dir>] [--out-dir <dir>] [--verbose]
//!   svelte-ratatui preview <input.svelte> [--verbose]
//!   svelte-ratatui scaffold [--with-tui] [--no-tui]

mod commands;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "svelte-ratatui",
    about = "Compile Svelte components to ratatui widget trees and scaffold TUI-capable projects",
    version
)]
struct Cli {
    /// Enable verbose output (includes IR dump and extra diagnostics).
    #[arg(long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Compile all Svelte components in a directory to ratatui Rust source files.
    ///
    /// Walks the source directory for `*.svelte` files, compiles each one, and writes the
    /// resulting Rust source alongside the input (or into `--out-dir` when specified).
    /// Reports per-file timing and a total summary on completion.
    Build {
        /// Directory containing Svelte components (defaults to current directory).
        #[arg(short, long, default_value = ".")]
        dir: String,

        /// Directory to write generated `.rs` files into.
        /// When omitted, each `.rs` file is placed next to its `.svelte` source.
        #[arg(short, long)]
        out_dir: Option<String>,
    },

    /// Validate Svelte components for TUI compatibility.
    ///
    /// Reports unsupported elements, styles, or language patterns that cannot be
    /// compiled to ratatui code.  Exit code is non-zero if any errors are found.
    Check {
        /// Directory containing Svelte components (defaults to current directory).
        #[arg(short, long, default_value = ".")]
        dir: String,
    },

    /// Watch mode: watch `.svelte` files and recompile on change.
    ///
    /// Performs an initial build of all components and then enters a watch loop.
    /// Any saved `.svelte` file triggers incremental recompilation of that file.
    Dev {
        /// Directory containing Svelte components (defaults to current directory).
        #[arg(short, long, default_value = ".")]
        dir: String,

        /// Directory to write generated `.rs` files into.
        #[arg(short, long)]
        out_dir: Option<String>,
    },

    /// Compile and display a single Svelte component (quick iteration).
    ///
    /// Compiles the given file and prints the generated Rust source to stdout.
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

    let result = match cli.command {
        Commands::Build { dir, out_dir } => {
            commands::build::run(&dir, out_dir.as_deref(), cli.verbose)
        }
        Commands::Check { dir } => commands::check::run(&dir, cli.verbose),
        Commands::Dev { dir, out_dir } => {
            commands::dev::run(&dir, out_dir.as_deref(), cli.verbose)
        }
        Commands::Preview { input } => commands::preview::run(&input, cli.verbose),
        Commands::Scaffold {
            project_dir,
            with_tui,
            no_tui,
        } => {
            let tui_enabled = with_tui && !no_tui;
            commands::scaffold::run(&project_dir, tui_enabled)
        }
    };

    if let Err(e) = result {
        print_error(&e);
        std::process::exit(1);
    }
}

/// Print a top-level error in red to stderr.
fn print_error(msg: &str) {
    eprintln!("\x1b[31merror\x1b[0m: {msg}");
}
