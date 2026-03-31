//! `svelte-ratatui dev` — watch mode with hot-reload on `.svelte` file changes.
//!
//! Performs an initial build of all Svelte components found under `dir`, then
//! enters a watch loop using the `notify` crate.  Any file-system event that
//! touches a `.svelte` file triggers incremental recompilation of that file
//! only, keeping feedback latency low.

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Instant;

use notify::{Event, EventKind, RecursiveMode, Watcher, recommended_watcher};
use svelte_ratatui_compiler::compile;

use super::{BOLD, CYAN, DIM, GREEN, RED, RESET, YELLOW, collect_svelte_files, output_path};

/// Run the `dev` command.
///
/// 1. Builds all `*.svelte` files found under `dir`.
/// 2. Watches `dir` recursively for changes.
/// 3. On each change to a `.svelte` file, recompiles that file and reports the
///    result.
///
/// The loop runs until the process is interrupted (Ctrl-C).
///
/// # Errors
///
/// Returns an error if the watcher cannot be initialised or if `dir` cannot be
/// resolved.
pub fn run(dir: &str, out_dir: Option<&str>, verbose: bool) -> Result<(), String> {
    if let Some(d) = out_dir {
        std::fs::create_dir_all(d)
            .map_err(|e| format!("cannot create out-dir '{d}': {e}"))?;
    }

    // ── initial build ────────────────────────────────────────────────────────
    let files = collect_svelte_files(dir);
    if files.is_empty() {
        eprintln!("{DIM}no .svelte files found in '{dir}'{RESET}");
    } else {
        eprintln!("{CYAN}{BOLD}svelte-ratatui dev{RESET} — initial build ({} component{})\n", files.len(), if files.len() == 1 { "" } else { "s" });
        let start = Instant::now();
        let mut ok = 0usize;
        let mut errs = 0usize;
        for path in &files {
            match compile_file(path, out_dir, verbose) {
                Ok(out) => {
                    eprintln!("  {GREEN}✓{RESET} {} {DIM}→ {}{RESET}", path.display(), out.display());
                    ok += 1;
                }
                Err(e) => {
                    eprintln!("  {RED}✗{RESET} {}: {e}", path.display());
                    errs += 1;
                }
            }
        }
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        if errs == 0 {
            eprintln!("\n{GREEN}{BOLD}compiled {ok} component{} in {ms:.1}ms{RESET}", if ok == 1 { "" } else { "s" });
        } else {
            eprintln!("\n{BOLD}compiled {ok}, {errs} error{}{RESET} in {ms:.1}ms", if errs == 1 { "" } else { "s" });
        }
    }

    // ── watch loop ───────────────────────────────────────────────────────────
    let (tx, rx) = mpsc::channel::<Result<Event, notify::Error>>();

    let mut watcher = recommended_watcher(tx)
        .map_err(|e| format!("failed to initialise file watcher: {e}"))?;

    let watch_dir = std::fs::canonicalize(dir)
        .map_err(|e| format!("cannot resolve watch directory '{dir}': {e}"))?;

    watcher
        .watch(&watch_dir, RecursiveMode::Recursive)
        .map_err(|e| format!("failed to watch '{dir}': {e}"))?;

    eprintln!(
        "\n{CYAN}watching {}{RESET} {DIM}(press Ctrl-C to stop){RESET}",
        watch_dir.display()
    );

    for res in rx {
        match res {
            Ok(event) => handle_event(&event, out_dir, verbose),
            Err(e) => eprintln!("{YELLOW}watch error{RESET}: {e}"),
        }
    }

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Handle a single `notify` event: recompile any `.svelte` paths involved.
fn handle_event(event: &Event, out_dir: Option<&str>, verbose: bool) {
    // Only act on create/modify events — ignore metadata, access, remove, etc.
    let is_relevant = matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_)
    );
    if !is_relevant {
        return;
    }

    for path in &event.paths {
        if path.extension().and_then(|s| s.to_str()) != Some("svelte") {
            continue;
        }
        let start = Instant::now();
        match compile_file(path, out_dir, verbose) {
            Ok(out) => {
                let ms = start.elapsed().as_secs_f64() * 1000.0;
                eprintln!(
                    "{GREEN}✓{RESET} {} {DIM}→ {} ({ms:.1}ms){RESET}",
                    path.display(),
                    out.display()
                );
            }
            Err(e) => {
                eprintln!("{RED}✗{RESET} {}: {e}", path.display());
            }
        }
    }
}

/// Compile a single `.svelte` file and write the `.rs` output.
///
/// Returns the path of the written output file on success.
fn compile_file(
    path: &std::path::Path,
    out_dir: Option<&str>,
    verbose: bool,
) -> Result<PathBuf, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("read error: {e}"))?;

    let rs_source = compile(&source)
        .map_err(|e| format!("compile error: {e}"))?;

    let out = output_path(path, out_dir);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create dirs: {e}"))?;
    }
    std::fs::write(&out, &rs_source)
        .map_err(|e| format!("write error: {e}"))?;

    if verbose && !rs_source.is_empty() {
        eprintln!("{CYAN}--- IR dump: {} ---{RESET}", path.display());
        eprintln!("{rs_source}");
    }

    Ok(out)
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// `compile_file` must produce an `.rs` next to its source when no out_dir.
    #[test]
    fn compile_file_writes_output() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        let svelte = tmp.path().join("Widget.svelte");
        std::fs::write(&svelte, "<p>widget</p>").unwrap();

        let result = compile_file(&svelte, None, false);
        assert!(result.is_ok(), "{result:?}");
        assert!(tmp.path().join("Widget.rs").exists());
    }

    /// `compile_file` writes into out_dir when specified.
    #[test]
    fn compile_file_out_dir() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        let out = tmp.path().join("gen");
        let svelte = tmp.path().join("Widget.svelte");
        std::fs::write(&svelte, "<p>widget</p>").unwrap();

        let result = compile_file(&svelte, Some(out.to_str().unwrap()), false);
        assert!(result.is_ok(), "{result:?}");
        assert!(out.join("Widget.rs").exists());
    }

    /// `compile_file` returns an error for a missing file.
    #[test]
    fn compile_file_missing_file_errors() {
        let result = compile_file(std::path::Path::new("/no/such/file.svelte"), None, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("read error"));
    }

    /// `handle_event` does not panic on an irrelevant event kind (Remove).
    #[test]
    fn handle_event_irrelevant_does_not_panic() {
        use notify::event::{RemoveKind};
        let event = Event {
            kind: EventKind::Remove(RemoveKind::File),
            paths: vec![std::path::PathBuf::from("/tmp/foo.svelte")],
            attrs: Default::default(),
        };
        // Should return without doing anything.
        handle_event(&event, None, false);
    }

    /// `handle_event` skips non-.svelte paths.
    #[test]
    fn handle_event_skips_non_svelte() {
        use notify::event::{ModifyKind};
        let event = Event {
            kind: EventKind::Modify(ModifyKind::Any),
            paths: vec![std::path::PathBuf::from("/tmp/foo.ts")],
            attrs: Default::default(),
        };
        handle_event(&event, None, false);
    }
}
