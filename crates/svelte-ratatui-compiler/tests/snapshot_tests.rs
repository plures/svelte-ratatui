//! Snapshot-based rendering tests for the compiler crate.
//!
//! Uses `insta` to capture the text output of rendered IR trees so that any
//! unintended visual regressions are caught immediately.

use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use std::collections::HashMap;
use svelte_ratatui_compiler::ir::{IrElement, IrNode};
use svelte_ratatui_compiler::{check_dialect, compile, render_ir};

// ── helpers ───────────────────────────────────────────────────────────────────

/// Render `node` into a `width × height` TestBackend and return the buffer as
/// a multi-line string.  Each terminal row becomes one line.
fn render_to_string(width: u16, height: u16, node: &IrNode) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame: &mut Frame| {
            let area = frame.area();
            render_ir(frame, area, node);
        })
        .unwrap();
    let buf = terminal.backend().buffer().clone();
    let mut output = String::new();
    for y in 0..height {
        for x in 0..width {
            output.push(buf[(x, y)].symbol().chars().next().unwrap_or(' '));
        }
        output.push('\n');
    }
    output
}

fn el(tag: &str, attrs: &[(&str, &str)], children: Vec<IrNode>) -> IrElement {
    IrElement {
        tag: tag.into(),
        attrs: attrs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        styles: HashMap::new(),
        children,
    }
}

fn styled_el(
    tag: &str,
    attrs: &[(&str, &str)],
    styles: &[(&str, &str)],
    children: Vec<IrNode>,
) -> IrElement {
    IrElement {
        tag: tag.into(),
        attrs: attrs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        styles: styles
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        children,
    }
}

// ── Widget mapping snapshot tests ─────────────────────────────────────────────

#[test]
fn snapshot_text_node_render() {
    let node = IrNode::text("Hello, terminal!");
    let out = render_to_string(20, 1, &node);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_paragraph_render() {
    let node = IrNode::Element(el("p", &[], vec![IrNode::text("Test paragraph")]));
    let out = render_to_string(24, 1, &node);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_heading_h1_render() {
    let node = IrNode::Element(el("h1", &[], vec![IrNode::text("Main Title")]));
    let out = render_to_string(20, 1, &node);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_heading_h2_render() {
    let node = IrNode::Element(el("h2", &[], vec![IrNode::text("Sub Title")]));
    let out = render_to_string(20, 1, &node);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_heading_h3_render() {
    let node = IrNode::Element(el("h3", &[], vec![IrNode::text("Section")]));
    let out = render_to_string(20, 1, &node);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_list_ul_render() {
    let node = IrNode::Element(el(
        "ul",
        &[],
        vec![
            IrNode::Element(el("li", &[], vec![IrNode::text("Alpha")])),
            IrNode::Element(el("li", &[], vec![IrNode::text("Beta")])),
            IrNode::Element(el("li", &[], vec![IrNode::text("Gamma")])),
        ],
    ));
    let out = render_to_string(20, 5, &node);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_list_ol_render() {
    let node = IrNode::Element(el(
        "ol",
        &[],
        vec![
            IrNode::Element(el("li", &[], vec![IrNode::text("First")])),
            IrNode::Element(el("li", &[], vec![IrNode::text("Second")])),
        ],
    ));
    let out = render_to_string(20, 4, &node);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_code_render() {
    let node = IrNode::Element(el("code", &[], vec![IrNode::text("let x = 1;")]));
    let out = render_to_string(20, 3, &node);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_input_with_value_render() {
    let node = IrNode::Element(el("input", &[("type", "text"), ("value", "hello")], vec![]));
    let out = render_to_string(20, 3, &node);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_button_render() {
    let node = IrNode::Element(el("button", &[], vec![IrNode::text("Submit")]));
    let out = render_to_string(20, 3, &node);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_progress_render() {
    let node = IrNode::Element(el("progress", &[("value", "42"), ("max", "100")], vec![]));
    let out = render_to_string(30, 3, &node);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_details_closed_render() {
    let node = IrNode::Element(el(
        "details",
        &[],
        vec![
            IrNode::Element(el("summary", &[], vec![IrNode::text("Click me")])),
            IrNode::Element(el("p", &[], vec![IrNode::text("Hidden body")])),
        ],
    ));
    let out = render_to_string(30, 5, &node);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_details_open_render() {
    let node = IrNode::Element(el(
        "details",
        &[("open", "")],
        vec![
            IrNode::Element(el("summary", &[], vec![IrNode::text("Info")])),
            IrNode::Element(el("p", &[], vec![IrNode::text("Visible detail")])),
        ],
    ));
    let out = render_to_string(30, 6, &node);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_hr_render() {
    let node = IrNode::Element(el("hr", &[], vec![]));
    let out = render_to_string(20, 2, &node);
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_styled_span_render() {
    let node = IrNode::Element(styled_el(
        "span",
        &[],
        &[("color", "red"), ("font-weight", "bold")],
        vec![IrNode::text("Styled text")],
    ));
    let out = render_to_string(20, 1, &node);
    insta::assert_snapshot!(out);
}

// ── Dialect check snapshot tests ──────────────────────────────────────────────

#[test]
fn snapshot_dialect_check_valid_source() {
    let src = r#"
<script>
  let count = $state(0);
  let doubled = $derived(count * 2);
</script>
<p>{count} doubled is {doubled}</p>
"#;
    let errors = check_dialect(src);
    insta::assert_debug_snapshot!(errors);
}

#[test]
fn snapshot_dialect_check_e001_async_effect() {
    let src = r#"
<script>
  $effect(async () => {
    const data = await fetch('/api/data');
  });
</script>
"#;
    let errors = check_dialect(src);
    insta::assert_debug_snapshot!(errors);
}

#[test]
fn snapshot_dialect_check_e002_dynamic_component() {
    let src = r#"
<script>
  let comp = MyComp;
</script>
<svelte:component this={comp} />
"#;
    let errors = check_dialect(src);
    insta::assert_debug_snapshot!(errors);
}

#[test]
fn snapshot_dialect_check_e003_raw_html() {
    let src = r#"
<script>
  let html = "<b>bold</b>";
</script>
<div>{@html html}</div>
"#;
    let errors = check_dialect(src);
    insta::assert_debug_snapshot!(errors);
}

#[test]
fn snapshot_dialect_check_multiple_violations() {
    let src = r#"
<script>
  $effect(async () => {
    await fetch('/api');
  });
</script>
<svelte:component this={comp} />
{@html rawHtml}
"#;
    let errors = check_dialect(src);
    insta::assert_debug_snapshot!(errors);
}

// ── Pipeline snapshot tests ───────────────────────────────────────────────────

#[test]
fn snapshot_compile_empty_source() {
    let result = compile("");
    insta::assert_debug_snapshot!(result);
}

#[test]
fn snapshot_compile_simple_template() {
    // The compiler pipeline is a stub; this test establishes the baseline
    // contract so any future output changes are caught.
    let src = r#"
<script>
  let message = $state("hello");
</script>
<p>{message}</p>
"#;
    let result = compile(src);
    insta::assert_debug_snapshot!(result);
}
