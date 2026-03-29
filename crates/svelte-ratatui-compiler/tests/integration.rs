//! Integration tests — HTML → adapter → mapping → rendered terminal output.
//!
//! These tests exercise the full runtime path that a deployed svelte-ratatui
//! app takes: raw HTML produced by a Svelte component is parsed by the adapter
//! into an IR tree, and the IR tree is rendered to a ratatui TestBackend.
//! Snapshots capture the exact terminal output for regression detection.

use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use svelte_ratatui_compiler::render_ir;

fn render_html_snapshot(html: &str, width: u16, height: u16) -> String {
    // Parse HTML → IR via the adapter crate.
    let ir = svelte_ratatui_adapter::parse_html(html);

    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame: &mut Frame| {
            let area = frame.area();
            render_ir(frame, area, &ir);
        })
        .unwrap();
    let buf = terminal.backend().buffer().clone();
    let mut out = String::new();
    for y in 0..height {
        for x in 0..width {
            out.push(buf[(x, y)].symbol().chars().next().unwrap_or(' '));
        }
        out.push('\n');
    }
    out
}

// ── End-to-end rendering snapshots ───────────────────────────────────────────

#[test]
fn e2e_paragraph_html_renders() {
    let out = render_html_snapshot("<p>Hello from Svelte</p>", 30, 1);
    insta::assert_snapshot!(out);
}

#[test]
fn e2e_unordered_list_html_renders() {
    let out = render_html_snapshot(
        r#"<ul>
  <li>First item</li>
  <li>Second item</li>
  <li>Third item</li>
</ul>"#,
        24,
        5,
    );
    insta::assert_snapshot!(out);
}

#[test]
fn e2e_form_with_input_renders() {
    let out = render_html_snapshot(
        r#"<form>
  <input type="text" value="my text" />
</form>"#,
        30,
        5,
    );
    insta::assert_snapshot!(out);
}

#[test]
fn e2e_heading_hierarchy_renders() {
    // Each heading level renders without panicking; h1 and h2 carry underline.
    let h1 = render_html_snapshot("<h1>Page Title</h1>", 20, 1);
    let h3 = render_html_snapshot("<h3>Section</h3>", 20, 1);
    insta::assert_snapshot!("h1", h1);
    insta::assert_snapshot!("h3", h3);
}

#[test]
fn e2e_code_block_has_border() {
    let out = render_html_snapshot("<code>let x = 42;</code>", 20, 3);
    // The rendered output must contain at least one box-drawing corner,
    // confirming that the border was applied.
    assert!(
        out.contains('┌') || out.contains('╔'),
        "code block must have a border; got: {out:?}"
    );
    insta::assert_snapshot!(out);
}

#[test]
fn e2e_progress_gauge_renders() {
    let out = render_html_snapshot(r#"<progress value="75" max="100"></progress>"#, 30, 3);
    insta::assert_snapshot!(out);
}

#[test]
fn e2e_details_element_closed() {
    let out = render_html_snapshot(
        r#"<details>
  <summary>Toggle me</summary>
  <p>Hidden content</p>
</details>"#,
        30,
        5,
    );
    // When closed the body must not appear.
    assert!(
        !out.contains("Hidden content"),
        "closed <details> must not show body; got: {out:?}"
    );
    insta::assert_snapshot!(out);
}

#[test]
fn e2e_styled_svelte_output_renders() {
    // Simulate a design-dojo TUI component's actual HTML output.
    let html = r#"<div class="tui-card" style="color: cyan;">
  <h2>Dashboard</h2>
  <p>Welcome to the TUI</p>
</div>"#;
    let out = render_html_snapshot(html, 30, 8);
    insta::assert_snapshot!(out);
}
