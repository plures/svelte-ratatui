//! `svelte-ratatui scaffold` — scaffold TUI integration into a svelte-tauri project.

use super::{BOLD, DIM, GREEN, RESET};

/// Run the `scaffold` command.
///
/// Currently reports what would be done (dry-run).  Actual file generation
/// will be added in a future release.
pub fn run(project_dir: &str, tui_enabled: bool) -> Result<(), String> {
    if tui_enabled {
        eprintln!("{GREEN}{BOLD}scaffold{RESET}: {project_dir}  {DIM}[TUI enabled]{RESET}  (coming soon)");
    } else {
        eprintln!("{BOLD}scaffold{RESET}: {project_dir}  {DIM}[TUI disabled]{RESET}  (coming soon)");
    }
    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_with_tui_ok() {
        assert!(run(".", true).is_ok());
    }

    #[test]
    fn scaffold_no_tui_ok() {
        assert!(run(".", false).is_ok());
    }
}
