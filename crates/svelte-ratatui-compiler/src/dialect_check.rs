//! Dialect validator for the allowed Svelte 5 Runes subset.
//!
//! Svelte-ratatui only supports a constrained dialect of Svelte 5. This module
//! performs a source-level scan and emits hard errors (E001–E004) for patterns
//! that cannot be compiled to ratatui TUI code.
//!
//! | Code | Trigger |
//! |------|---------|
//! | E001 | `$effect` with async logic not using `db.subscribe()` or `ipc()` |
//! | E002 | `<svelte:component>` dynamic dispatch |
//! | E003 | `{@html …}` raw HTML injection |
//! | E004 | External JS library imported and called inside a template expression |

/// A violation of the allowed dialect constraint.
#[derive(Debug, PartialEq)]
pub struct DialectError {
    /// Error code (e.g. `"E001"`).
    pub code: &'static str,
    /// 1-based line number where the violation was detected.
    pub line: usize,
    /// Human-readable description of the violation.
    pub message: String,
}

impl std::fmt::Display for DialectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] line {}: {}", self.code, self.line, self.message)
    }
}

/// Validate a Svelte source file against the allowed-dialect constraints.
///
/// Returns a (possibly empty) list of [`DialectError`]s. An empty list means
/// the source is within the allowed dialect and may proceed to compilation.
pub fn check(source: &str) -> Vec<DialectError> {
    let mut errors = Vec::new();
    check_e001_async_effects(source, &mut errors);
    check_e002_dynamic_components(source, &mut errors);
    check_e003_raw_html(source, &mut errors);
    check_e004_external_libs(source, &mut errors);
    errors
}

// ── E001 ─────────────────────────────────────────────────────────────────────

/// E001 — `$effect` blocks that contain `async`/`await` without using the
/// sanctioned `db.subscribe()` or `ipc()` patterns.
fn check_e001_async_effects(source: &str, errors: &mut Vec<DialectError>) {
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim_start().starts_with("$effect(") {
            let start_line = i + 1; // 1-based
            let block = collect_effect_body(&lines, i);
            let has_async = block.contains("async ") || block.contains("await ");
            let has_allowed = block.contains("db.subscribe(") || block.contains("ipc(");
            if has_async && !has_allowed {
                errors.push(DialectError {
                    code: "E001",
                    line: start_line,
                    message: "E001: async effects must use db.subscribe() or ipc() patterns"
                        .to_string(),
                });
            }
        }
        i += 1;
    }
}

/// Collect the full text of the `$effect(…)` call starting at `start_line`.
///
/// Scans lines from `start_line`, counting `(` / `)` to find the matching
/// closing parenthesis of the outer `$effect(` call.
fn collect_effect_body(lines: &[&str], start_line: usize) -> String {
    let mut result = String::new();
    let mut depth: i32 = 0;
    let mut started = false;

    for line in &lines[start_line..] {
        result.push_str(line);
        result.push('\n');
        for ch in line.chars() {
            match ch {
                '(' => {
                    depth += 1;
                    started = true;
                }
                ')' => {
                    depth -= 1;
                    if started && depth == 0 {
                        return result;
                    }
                }
                _ => {}
            }
        }
    }
    result
}

// ── E002 ─────────────────────────────────────────────────────────────────────

/// E002 — `<svelte:component>` dynamic dispatch, which has no TUI equivalent.
fn check_e002_dynamic_components(source: &str, errors: &mut Vec<DialectError>) {
    for (i, line) in source.lines().enumerate() {
        if line.contains("<svelte:component") {
            errors.push(DialectError {
                code: "E002",
                line: i + 1,
                message: "E002: dynamic components not supported in TUI mode".to_string(),
            });
        }
    }
}

// ── E003 ─────────────────────────────────────────────────────────────────────

/// E003 — `{@html …}` raw HTML injection, which has no TUI equivalent.
fn check_e003_raw_html(source: &str, errors: &mut Vec<DialectError>) {
    for (i, line) in source.lines().enumerate() {
        if line.contains("{@html") {
            errors.push(DialectError {
                code: "E003",
                line: i + 1,
                message: "E003: raw HTML has no TUI equivalent".to_string(),
            });
        }
    }
}

// ── E004 ─────────────────────────────────────────────────────────────────────

/// E004 — an identifier imported from an external NPM package appears inside
/// a template expression in the markup section.
fn check_e004_external_libs(source: &str, errors: &mut Vec<DialectError>) {
    let external_imports = collect_external_imports(source);
    if external_imports.is_empty() {
        return;
    }

    let template = extract_template(source);
    let template_line_start = count_lines_before_template(source);

    for (offset, line) in template.lines().enumerate() {
        for import in &external_imports {
            if contains_word(line, import) {
                errors.push(DialectError {
                    code: "E004",
                    line: template_line_start + offset + 1,
                    message: format!(
                        "E004: only design-dojo components allowed in TUI views \
                         (found external library call: {import})"
                    ),
                });
                break; // one E004 error per template line
            }
        }
    }
}

/// Collect identifiers imported from external NPM packages (i.e. not relative
/// paths, not `$lib/…`, not `svelte` built-ins).
fn collect_external_imports(source: &str) -> Vec<String> {
    let mut imports = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("import ") || !trimmed.contains(" from ") {
            continue;
        }
        let Some(from_pos) = trimmed.rfind(" from ") else {
            continue;
        };
        let module = trimmed[from_pos + 6..]
            .trim()
            .trim_matches(|c| matches!(c, '\'' | '"' | ';'));

        // Skip relative paths and known Svelte/internal packages
        if module.starts_with('.')
            || module.starts_with('/')
            || module.starts_with("$lib")
            || module.starts_with("svelte")
        {
            continue;
        }

        // Extract imported identifier(s) from the import clause
        let import_clause = trimmed["import ".len()..from_pos].trim();
        // Handle both `import Foo from` and `import { foo, bar } from`
        let inner = import_clause
            .trim_matches(|c| matches!(c, '{' | '}'))
            .trim();
        for part in inner.split(',') {
            // Handle `foo as bar` aliasing — use the local name (after `as`)
            let ident = part
                .trim()
                .rsplit(" as ")
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches(|c| matches!(c, '{' | '}' | ' '));
            if !ident.is_empty() {
                imports.push(ident.to_string());
            }
        }
    }
    imports
}

/// Return the portion of `source` that follows the closing `</script>` tag
/// (i.e. the markup/template section).
fn extract_template(source: &str) -> &str {
    match source.rfind("</script>") {
        Some(pos) => &source[pos + "</script>".len()..],
        None => source,
    }
}

/// Count the number of lines in `source` up to and including `</script>`.
fn count_lines_before_template(source: &str) -> usize {
    match source.rfind("</script>") {
        Some(pos) => source[..pos + "</script>".len()].lines().count(),
        None => 0,
    }
}

/// Test whether `word` appears in `text` as a whole word (not a substring of
/// a longer identifier).
fn contains_word(text: &str, word: &str) -> bool {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .any(|token| token == word)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn codes(errors: &[DialectError]) -> Vec<&str> {
        errors.iter().map(|e| e.code).collect()
    }

    // ── valid source ─────────────────────────────────────────────────────────

    #[test]
    fn valid_state_and_derived_produce_no_errors() {
        let src = r#"
<script>
  let count = $state(0);
  let doubled = $derived(count * 2);
</script>
<p>{count} doubled is {doubled}</p>
        "#;
        assert!(check(src).is_empty());
    }

    #[test]
    fn valid_db_subscribe_effect_is_allowed() {
        let src = r#"
<script>
  let messages = $state([]);
  $effect(() => {
    const unsub = db.subscribe("messages", msgs => { messages = msgs; });
    return unsub;
  });
</script>
<ul>{#each messages as m}<li>{m.text}</li>{/each}</ul>
        "#;
        assert!(
            check(src).is_empty(),
            "db.subscribe() pattern must be allowed"
        );
    }

    #[test]
    fn valid_ipc_effect_is_allowed() {
        let src = r#"
<script>
  let value = $state("");
  $effect(() => {
    const unsub = ipc("channel", data => { value = data; });
    return unsub;
  });
</script>
<p>{value}</p>
        "#;
        assert!(check(src).is_empty(), "ipc() pattern must be allowed");
    }

    #[test]
    fn valid_snippet_and_render_produce_no_errors() {
        let src = r#"
<script>
  let label = $state("hello");
</script>
{#snippet btn(text)}<button>{text}</button>{/snippet}
{@render btn(label)}
        "#;
        assert!(check(src).is_empty());
    }

    // ── E001 ─────────────────────────────────────────────────────────────────

    #[test]
    fn e001_async_arrow_function_in_effect() {
        let src = r#"
<script>
  $effect(async () => {
    const data = await fetch('/api/data');
  });
</script>
        "#;
        let errors = check(src);
        assert!(
            codes(&errors).contains(&"E001"),
            "expected E001, got {errors:?}"
        );
    }

    #[test]
    fn e001_await_without_subscribe_or_ipc() {
        let src = r#"
<script>
  $effect(() => {
    const result = await someAsyncFn();
  });
</script>
        "#;
        let errors = check(src);
        assert!(
            codes(&errors).contains(&"E001"),
            "expected E001, got {errors:?}"
        );
    }

    #[test]
    fn e001_carries_correct_line_number() {
        let src = "line1\nline2\n$effect(async () => {\n  await fetch('/');\n});\n";
        let errors = check(src);
        let e001: Vec<_> = errors.iter().filter(|e| e.code == "E001").collect();
        assert!(!e001.is_empty());
        assert_eq!(e001[0].line, 3, "E001 should point to the $effect line");
    }

    // ── E002 ─────────────────────────────────────────────────────────────────

    #[test]
    fn e002_svelte_component_dynamic_dispatch() {
        let src = r#"
<script>
  let comp = MyComp;
</script>
<svelte:component this={comp} />
        "#;
        let errors = check(src);
        assert_eq!(codes(&errors), vec!["E002"]);
    }

    #[test]
    fn e002_carries_correct_line_number() {
        let src = "line1\nline2\n<svelte:component this={c} />\n";
        let errors = check(src);
        let e002: Vec<_> = errors.iter().filter(|e| e.code == "E002").collect();
        assert!(!e002.is_empty());
        assert_eq!(e002[0].line, 3);
    }

    // ── E003 ─────────────────────────────────────────────────────────────────

    #[test]
    fn e003_raw_html_injection() {
        let src = r#"
<script>
  let html = "<b>bold</b>";
</script>
<div>{@html html}</div>
        "#;
        let errors = check(src);
        assert_eq!(codes(&errors), vec!["E003"]);
    }

    #[test]
    fn e003_carries_correct_line_number() {
        let src = "line1\nline2\n<div>{@html raw}</div>\n";
        let errors = check(src);
        let e003: Vec<_> = errors.iter().filter(|e| e.code == "E003").collect();
        assert!(!e003.is_empty());
        assert_eq!(e003[0].line, 3);
    }

    // ── E004 ─────────────────────────────────────────────────────────────────

    #[test]
    fn e004_external_library_in_template() {
        let src = r#"
<script>
  import { groupBy } from 'lodash';
  let items = $state([]);
</script>
<div>{groupBy(items, 'category')}</div>
        "#;
        let errors = check(src);
        assert!(
            codes(&errors).contains(&"E004"),
            "expected E004, got {errors:?}"
        );
    }

    #[test]
    fn e004_default_import_in_template() {
        let src = r#"
<script>
  import moment from 'moment';
  let ts = $state(0);
</script>
<p>{moment(ts).fromNow()}</p>
        "#;
        let errors = check(src);
        assert!(
            codes(&errors).contains(&"E004"),
            "expected E004, got {errors:?}"
        );
    }

    #[test]
    fn e004_relative_import_is_not_flagged() {
        let src = r#"
<script>
  import { fmt } from './utils';
  let val = $state(0);
</script>
<p>{fmt(val)}</p>
        "#;
        // Relative imports are allowed (design-dojo internal modules)
        let errors = check(src);
        assert!(
            !codes(&errors).contains(&"E004"),
            "relative imports must not trigger E004"
        );
    }

    #[test]
    fn e004_svelte_import_is_not_flagged() {
        let src = r#"
<script>
  import { onMount } from 'svelte';
</script>
<p>hello</p>
        "#;
        let errors = check(src);
        assert!(
            !codes(&errors).contains(&"E004"),
            "svelte built-ins must not trigger E004"
        );
    }

    // ── multiple violations ───────────────────────────────────────────────────

    #[test]
    fn multiple_violations_all_reported() {
        let src = r#"
<script>
  $effect(async () => {
    await fetch('/api');
  });
</script>
<svelte:component this={comp} />
{@html rawHtml}
        "#;
        let errors = check(src);
        let c = codes(&errors);
        assert!(c.contains(&"E001"), "expected E001");
        assert!(c.contains(&"E002"), "expected E002");
        assert!(c.contains(&"E003"), "expected E003");
    }

    // ── Display impl ─────────────────────────────────────────────────────────

    #[test]
    fn display_format_includes_code_and_line() {
        let err = DialectError {
            code: "E002",
            line: 7,
            message: "E002: dynamic components not supported in TUI mode".to_string(),
        };
        let s = err.to_string();
        assert!(s.contains("E002"));
        assert!(s.contains('7'));
    }
}
