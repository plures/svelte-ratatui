//! Snapshot-based tests for the adapter crate.
//!
//! Covers HTML parsing (html_parser) and input event translation (input) so
//! that any change to how DOM snippets are parsed — or how keyboard/mouse
//! events are translated to JavaScript — is captured in a checked-in snapshot.

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use std::collections::BTreeMap;
use svelte_ratatui_adapter::{input::event_to_js, parse_html};
use svelte_ratatui_compiler::ir::IrNode;

// ── Stable wrapper types for deterministic snapshots ─────────────────────────
//
// IrElement uses HashMap internally; HashMap debug output is non-deterministic.
// These wrappers mirror the IR types with BTreeMap so snapshots are stable.

#[derive(Debug)]
enum SnapIrNode {
    Element(SnapIrElement),
    Text(String),
}

#[derive(Debug)]
struct SnapIrElement {
    tag: String,
    attrs: BTreeMap<String, String>,
    styles: BTreeMap<String, String>,
    children: Vec<SnapIrNode>,
}

fn to_snap(ir: &IrNode) -> SnapIrNode {
    match ir {
        IrNode::Text(s) => SnapIrNode::Text(s.clone()),
        IrNode::Element(el) => SnapIrNode::Element(SnapIrElement {
            tag: el.tag.clone(),
            attrs: el
                .attrs
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            styles: el
                .styles
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            children: el.children.iter().map(to_snap).collect(),
        }),
    }
}

// ── HTML parser snapshot tests ────────────────────────────────────────────────

#[test]
fn snapshot_parse_simple_paragraph() {
    let ir = parse_html("<p>Hello world</p>");
    insta::assert_debug_snapshot!(to_snap(&ir));
}

#[test]
fn snapshot_parse_styled_div() {
    let ir = parse_html(
        r#"<div class="tui-row selected" style="color: #00d4ff; font-weight: bold;">Styled</div>"#,
    );
    insta::assert_debug_snapshot!(to_snap(&ir));
}

#[test]
fn snapshot_parse_nested_list() {
    let ir = parse_html(
        r#"<ul>
  <li>Alpha</li>
  <li>Beta</li>
  <li>Gamma</li>
</ul>"#,
    );
    insta::assert_debug_snapshot!(to_snap(&ir));
}

#[test]
fn snapshot_parse_table_html() {
    let ir = parse_html(
        r#"<table>
  <thead><tr><th>Name</th><th>Score</th></tr></thead>
  <tbody>
    <tr><td>Alice</td><td>95</td></tr>
    <tr><td>Bob</td><td>88</td></tr>
  </tbody>
</table>"#,
    );
    insta::assert_debug_snapshot!(to_snap(&ir));
}

#[test]
fn snapshot_parse_design_dojo_tui_table() {
    let html = r#"<div class="tui-table" role="grid" aria-label="Results">
  <div class="tui-border" aria-hidden="true">╔════╦════╗</div>
  <div class="tui-header" role="row">║ A  ║ B  ║</div>
  <div class="tui-border" aria-hidden="true">╠════╬════╣</div>
  <div class="tui-row" role="row">║ 1  ║ 2  ║</div>
  <div class="tui-border" aria-hidden="true">╚════╩════╝</div>
</div>"#;
    let ir = parse_html(html);
    insta::assert_debug_snapshot!(to_snap(&ir));
}

#[test]
fn snapshot_parse_input_elements() {
    let html = r#"<form>
  <input type="text" value="hello" placeholder="Enter text" />
  <input type="password" value="secret" />
  <button>Submit</button>
</form>"#;
    let ir = parse_html(html);
    insta::assert_debug_snapshot!(to_snap(&ir));
}

#[test]
fn snapshot_parse_entity_decoding() {
    let ir = parse_html(r#"<p>&lt;hello&gt; &amp; &quot;world&quot;</p>"#);
    insta::assert_debug_snapshot!(to_snap(&ir));
}

// ── Input event translation snapshot tests ───────────────────────────────────

#[test]
fn snapshot_event_enter_key() {
    let ev = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let js = event_to_js(&ev);
    insta::assert_snapshot!(js.unwrap());
}

#[test]
fn snapshot_event_escape_key() {
    let ev = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    let js = event_to_js(&ev);
    insta::assert_snapshot!(js.unwrap());
}

#[test]
fn snapshot_event_arrow_left() {
    let ev = Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
    let js = event_to_js(&ev);
    insta::assert_snapshot!(js.unwrap());
}

#[test]
fn snapshot_event_arrow_up() {
    let ev = Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    let js = event_to_js(&ev);
    insta::assert_snapshot!(js.unwrap());
}

#[test]
fn snapshot_event_ctrl_s() {
    let ev = Event::Key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
    let js = event_to_js(&ev);
    insta::assert_snapshot!(js.unwrap());
}

#[test]
fn snapshot_event_shift_tab() {
    let ev = Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT));
    let js = event_to_js(&ev);
    insta::assert_snapshot!(js.unwrap());
}

#[test]
fn snapshot_event_tab() {
    let ev = Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    let js = event_to_js(&ev);
    insta::assert_snapshot!(js.unwrap());
}

#[test]
fn snapshot_event_mouse_click() {
    let ev = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: 5,
        modifiers: KeyModifiers::NONE,
    });
    let js = event_to_js(&ev);
    insta::assert_snapshot!(js.unwrap());
}

#[test]
fn snapshot_event_f1_key() {
    let ev = Event::Key(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE));
    let js = event_to_js(&ev);
    insta::assert_snapshot!(js.unwrap());
}

#[test]
fn snapshot_event_backspace() {
    let ev = Event::Key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
    let js = event_to_js(&ev);
    insta::assert_snapshot!(js.unwrap());
}
