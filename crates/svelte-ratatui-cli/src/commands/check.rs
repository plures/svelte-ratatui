//! `svelte-ratatui check` — validate Svelte components for TUI compatibility.

use svelte_ratatui_compiler::check_dialect;

use super::{BOLD, GREEN, RED, RESET, YELLOW, collect_svelte_files};

/// Run the `check` command.
///
/// Walks `dir` for `*.svelte` files, runs the dialect validator on each, and
/// reports any violations with file, line, and error-code context.
///
/// Returns `Ok(())` when no violations are found.  Returns `Err(_)` with a
/// summary message when at least one file has dialect errors.
pub fn run(dir: &str, verbose: bool) -> Result<(), String> {
    let files = collect_svelte_files(dir);
    if files.is_empty() {
        eprintln!("no .svelte files found in '{dir}'");
        return Ok(());
    }

    let mut total_errors = 0usize;

    for path in &files {
        let display = path.display();

        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  {BOLD}error{RESET} reading {display}: {e}");
                total_errors += 1;
                continue;
            }
        };

        let errors = check_dialect(&source);

        if errors.is_empty() {
            eprintln!("  {GREEN}✓{RESET} {display}");
            if verbose {
                eprintln!("    no dialect violations found");
            }
        } else {
            eprintln!("  {RED}✗{RESET} {display}");
            for err in &errors {
                eprintln!(
                    "    {YELLOW}{}{RESET} line {}: {}",
                    err.code, err.line, err.message
                );
            }
            total_errors += errors.len();
        }
    }

    if total_errors == 0 {
        eprintln!("\n{GREEN}{BOLD}no issues found in {} file{}{RESET}", files.len(), if files.len() == 1 { "" } else { "s" });
        Ok(())
    } else {
        Err(format!(
            "{total_errors} dialect violation{} found",
            if total_errors == 1 { "" } else { "s" }
        ))
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_empty_dir_ok() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        assert!(run(tmp.path().to_str().unwrap(), false).is_ok());
    }

    #[test]
    fn check_valid_file_ok() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        std::fs::write(tmp.path().join("Good.svelte"), "<p>hello</p>").unwrap();
        assert!(run(tmp.path().to_str().unwrap(), false).is_ok());
    }

    #[test]
    fn check_invalid_file_returns_error() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        std::fs::write(
            tmp.path().join("Bad.svelte"),
            "<svelte:component this={c} />\n{@html raw}",
        )
        .unwrap();
        let result = run(tmp.path().to_str().unwrap(), false);
        assert!(result.is_err(), "expected error for dialect violations");
        let msg = result.unwrap_err();
        assert!(msg.contains("violation"), "error message should mention violations: {msg}");
    }

    #[test]
    fn check_counts_violations_across_files() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        std::fs::write(tmp.path().join("A.svelte"), "<svelte:component this={c} />").unwrap();
        std::fs::write(tmp.path().join("B.svelte"), "{@html raw}").unwrap();
        let result = run(tmp.path().to_str().unwrap(), false);
        assert!(result.is_err());
        // Two violations (one E002 + one E003) → message mentions count ≥ 2
        let msg = result.unwrap_err();
        assert!(msg.starts_with("2") || msg.contains("2 dialect"), "unexpected: {msg}");
    }

    #[test]
    fn check_verbose_does_not_panic() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        std::fs::write(tmp.path().join("V.svelte"), "<p>ok</p>").unwrap();
        assert!(run(tmp.path().to_str().unwrap(), true).is_ok());
    }
}
