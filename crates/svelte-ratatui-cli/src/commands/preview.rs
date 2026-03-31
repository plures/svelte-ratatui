//! `svelte-ratatui preview` — compile a single Svelte file and print the output.

use svelte_ratatui_compiler::compile;

use super::{BOLD, CYAN, DIM, GREEN, RESET};

/// Run the `preview` command.
///
/// Reads `input`, compiles it, and writes the generated Rust source to stdout.
/// When `verbose` is set the raw source is also echoed to stderr for
/// side-by-side comparison.
///
/// # Errors
///
/// Returns an error string if the file cannot be read or if compilation fails.
pub fn run(input: &str, verbose: bool) -> Result<(), String> {
    let source = std::fs::read_to_string(input)
        .map_err(|e| format!("cannot read '{input}': {e}"))?;

    if verbose {
        eprintln!("{CYAN}--- source: {input} ---{RESET}");
        eprintln!("{DIM}{source}{RESET}");
    }

    let rs_source = compile(&source).map_err(|e| format!("compile error in '{input}': {e}"))?;

    if rs_source.is_empty() {
        eprintln!("{DIM}(compiler produced no output — pipeline not yet fully implemented){RESET}");
    } else {
        eprintln!("{GREEN}{BOLD}preview:{RESET} {input}");
        print!("{rs_source}");
    }

    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_valid_file_ok() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        let f = tmp.path().join("Demo.svelte");
        std::fs::write(&f, "<p>hello</p>").unwrap();
        assert!(run(f.to_str().unwrap(), false).is_ok());
    }

    #[test]
    fn preview_missing_file_errors() {
        let result = run("/nonexistent/path/Demo.svelte", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot read"));
    }

    #[test]
    fn preview_verbose_does_not_panic() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        let f = tmp.path().join("Demo.svelte");
        std::fs::write(&f, "<p>verbose</p>").unwrap();
        assert!(run(f.to_str().unwrap(), true).is_ok());
    }
}
