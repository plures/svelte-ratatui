//! `svelte-ratatui build` — one-shot compile all Svelte components to ratatui Rust source.

use std::time::Instant;

use svelte_ratatui_compiler::compile;

use super::{BOLD, CYAN, DIM, GREEN, RESET, collect_svelte_files, output_path};

/// Run the `build` command.
///
/// Walks `dir` for `*.svelte` files, compiles each one, and writes the
/// generated Rust source to the resolved output path.  Reports per-file
/// timing and a summary on completion.
///
/// # Errors
///
/// Returns an error string if `dir` is not accessible or if any `.rs` file
/// cannot be written.  Individual compile errors are printed to stderr but do
/// not abort the overall build.
pub fn run(dir: &str, out_dir: Option<&str>, verbose: bool) -> Result<(), String> {
    if let Some(d) = out_dir {
        std::fs::create_dir_all(d)
            .map_err(|e| format!("cannot create out-dir '{d}': {e}"))?;
    }

    let files = collect_svelte_files(dir);
    if files.is_empty() {
        eprintln!("{DIM}no .svelte files found in '{dir}'{RESET}");
        return Ok(());
    }

    let total_start = Instant::now();
    let mut compiled = 0usize;
    let mut errors = 0usize;

    for path in &files {
        let file_start = Instant::now();
        let display = path.display();

        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  {BOLD}error{RESET} reading {display}: {e}");
                errors += 1;
                continue;
            }
        };

        match compile(&source) {
            Ok(rs_source) => {
                let out = output_path(path, out_dir);
                if let Some(parent) = out.parent()
                    && let Err(e) = std::fs::create_dir_all(parent)
                {
                    eprintln!("  {BOLD}error{RESET} creating dirs for {}: {e}", out.display());
                    errors += 1;
                    continue;
                }
                if let Err(e) = std::fs::write(&out, &rs_source) {
                    eprintln!("  {BOLD}error{RESET} writing {}: {e}", out.display());
                    errors += 1;
                    continue;
                }
                let elapsed = file_start.elapsed();
                eprintln!(
                    "  {GREEN}✓{RESET} {display} {DIM}→ {} ({:.1}ms){RESET}",
                    out.display(),
                    elapsed.as_secs_f64() * 1000.0
                );
                if verbose && !rs_source.is_empty() {
                    eprintln!("{CYAN}--- IR dump: {display} ---{RESET}");
                    eprintln!("{rs_source}");
                }
                compiled += 1;
            }
            Err(e) => {
                eprintln!("  {BOLD}error{RESET} compiling {display}: {e}");
                errors += 1;
            }
        }
    }

    let total_ms = total_start.elapsed().as_secs_f64() * 1000.0;
    if errors == 0 {
        eprintln!(
            "\n{GREEN}{BOLD}compiled {compiled} component{} in {total_ms:.1}ms{RESET}",
            if compiled == 1 { "" } else { "s" }
        );
    } else {
        eprintln!(
            "\n{BOLD}compiled {compiled} component{}, {errors} error{}{RESET} in {total_ms:.1}ms",
            if compiled == 1 { "" } else { "s" },
            if errors == 1 { "" } else { "s" }
        );
        return Err(format!("{errors} file(s) failed to compile"));
    }

    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::path::Path;
    use super::*;

    #[test]
    fn build_empty_dir_succeeds() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        let result = run(tmp.path().to_str().unwrap(), None, false);
        assert!(result.is_ok(), "empty dir should not be an error: {result:?}");
    }

    #[test]
    fn build_writes_rs_file_next_to_svelte() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        let svelte = tmp.path().join("Hello.svelte");
        std::fs::write(&svelte, "<p>Hello</p>").unwrap();

        let result = run(tmp.path().to_str().unwrap(), None, false);
        assert!(result.is_ok(), "{result:?}");

        let rs = tmp.path().join("Hello.rs");
        assert!(rs.exists(), "Hello.rs should have been written");
    }

    #[test]
    fn build_writes_rs_file_into_out_dir() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        let out = tmp.path().join("out");
        let svelte = tmp.path().join("Foo.svelte");
        std::fs::write(&svelte, "<div>foo</div>").unwrap();

        let result = run(
            tmp.path().to_str().unwrap(),
            Some(out.to_str().unwrap()),
            false,
        );
        assert!(result.is_ok(), "{result:?}");
        assert!(out.join("Foo.rs").exists(), "Foo.rs should be in out dir");
    }

    #[test]
    fn build_verbose_does_not_panic() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        std::fs::write(tmp.path().join("A.svelte"), "<p>a</p>").unwrap();
        let result = run(tmp.path().to_str().unwrap(), None, true);
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn output_path_no_out_dir() {
        let p = Path::new("/src/Button.svelte");
        assert_eq!(output_path(p, None), Path::new("/src/Button.rs"));
    }

    #[test]
    fn output_path_with_out_dir() {
        let p = Path::new("/src/Button.svelte");
        assert_eq!(
            output_path(p, Some("/gen")),
            Path::new("/gen/Button.rs")
        );
    }
}
