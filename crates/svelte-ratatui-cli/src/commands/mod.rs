pub mod build;
pub mod check;
pub mod dev;
pub mod preview;
pub mod scaffold;

// ── Shared utilities ──────────────────────────────────────────────────────────

use std::path::Path;
use walkdir::WalkDir;

/// Collect every `*.svelte` file under `root`, following symlinks.
pub(crate) fn collect_svelte_files(root: &str) -> Vec<std::path::PathBuf> {
    WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path().extension().and_then(|s| s.to_str()) == Some("svelte")
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

/// Resolve the output `.rs` path for a given `.svelte` input.
///
/// - When `out_dir` is `Some`, the output is placed inside that directory,
///   keeping only the file stem (e.g. `Button.svelte` → `<out_dir>/Button.rs`).
/// - When `out_dir` is `None`, the output sits next to the input.
pub(crate) fn output_path(input: &Path, out_dir: Option<&str>) -> std::path::PathBuf {
    let stem = input
        .file_stem()
        .unwrap_or(std::ffi::OsStr::new("out"))
        .to_string_lossy();
    let rs_name = format!("{stem}.rs");
    match out_dir {
        Some(dir) => Path::new(dir).join(&rs_name),
        None => input.with_extension("rs"),
    }
}

// ── ANSI helpers ──────────────────────────────────────────────────────────────

pub(crate) const RESET: &str = "\x1b[0m";
pub(crate) const BOLD: &str = "\x1b[1m";
pub(crate) const RED: &str = "\x1b[31m";
pub(crate) const GREEN: &str = "\x1b[32m";
pub(crate) const YELLOW: &str = "\x1b[33m";
pub(crate) const CYAN: &str = "\x1b[36m";
pub(crate) const DIM: &str = "\x1b[2m";
