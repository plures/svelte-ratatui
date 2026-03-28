//! Element mapping — transforms [`IrNode`] trees into ratatui widget trees.
//!
//! This module implements Table 4.1 from the design doc:
//! HTML Element → ratatui Widget.
//!
//! The mapping is used by:
//! - **Runtime adapter**: HTML from headless Tauri → IR → this module → terminal
//! - **Compiler**: Svelte source → IR → this module's logic emitted as Rust code

use ratatui::layout::{Constraint as RatConstraint, Direction as RatDirection, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block,
    Borders,
    Cell,
    Gauge,
    List,
    ListItem,
    Paragraph,
    Row,
    Table,
    Wrap,
};
use ratatui::Frame;

use crate::ir::{
    Alignment, Direction, IrColor, IrElement, IrModifier, IrNode, IrStyle, NamedColor,
};

// ── Public API ───────────────────────────────────────────────────────────────

/// Render an IR tree into a ratatui frame at the given area.
pub fn render_ir(frame: &mut Frame, area: Rect, root: &IrNode) {
    match root {
        IrNode::Text(s) => {
            frame.render_widget(Paragraph::new(s.as_str()), area);
        }
        IrNode::Element(el) => render_element(frame, area, el),
    }
}

// ── Element dispatch ─────────────────────────────────────────────────────────

fn render_element(frame: &mut Frame, area: Rect, el: &IrElement) {
    match el.tag.as_str() {
        // ── Block containers ──────────────────────────────────────────────────
        // <div>, <section>, <main>, etc. → Block with optional border/padding.
        "div" | "section" | "main" | "article" | "nav" | "aside" | "footer" | "header" => {
            render_container(frame, area, el);
        }

        // Form containers — rendered as bordered blocks.
        // Example: <form> / <fieldset> act as logical grouping containers.
        "form" | "fieldset" => {
            render_container(frame, area, el);
        }

        // ── Text elements ─────────────────────────────────────────────────────
        // <p>, <pre>, <blockquote> → Paragraph / Text.
        "p" | "pre" | "blockquote" => {
            render_paragraph(frame, area, el);
        }

        // <code> → bordered Paragraph (monospace-style, visually distinct).
        // Example: <code>let x = 1;</code>
        "code" => {
            render_code(frame, area, el);
        }

        // <label> → inline Paragraph (associated label text).
        // Example: <label for="name">Name:</label>
        "label" => {
            render_paragraph(frame, area, el);
        }

        // ── Headings ──────────────────────────────────────────────────────────
        // <h1>–<h6> → styled Paragraph (bold; h1/h2 also underlined for hierarchy).
        // Example: <h1>Page Title</h1>
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            render_heading(frame, area, el);
        }

        // ── Inline text ───────────────────────────────────────────────────────
        // Inline elements rendered as Paragraph when they appear as block roots.
        // Example: <span class="highlight">text</span>
        "span" | "strong" | "em" | "b" | "i" | "u" | "a" => {
            render_paragraph(frame, area, el);
        }

        // ── Lists ─────────────────────────────────────────────────────────────
        // <ul> → List with bullet (•) prefix per item.
        // <ol> → List with numeric (1. 2. …) prefix per item.
        "ul" | "ol" => {
            render_list(frame, area, el);
        }

        // ── Tables ────────────────────────────────────────────────────────────
        // <table> → ratatui Table with header row and body rows.
        // Example: <table><thead>…</thead><tbody>…</tbody></table>
        "table" => {
            render_table(frame, area, el);
        }

        // Design-dojo TUI table (pre-rendered box-drawing passthrough).
        _ if el.has_class("tui-table") => {
            render_tui_table_passthrough(frame, area, el);
        }

        // ── Interactive ───────────────────────────────────────────────────────
        // <input> / <textarea> → bordered Paragraph with cursor indicator.
        // Example: <input type="text" value="hello" placeholder="Enter text" />
        "input" | "textarea" => {
            render_input(frame, area, el);
        }

        // <button> → bordered Block; `selected` class → reversed highlight;
        // `disabled` attribute → DIM modifier.
        // Example: <button class="selected">Click me</button>
        "button" => {
            render_button(frame, area, el);
        }

        // <select> → List with <option> children; `value` attr highlights the
        // currently selected option with a REVERSED style.
        // Example: <select value="b"><option value="a">A</option>…</select>
        "select" => {
            render_select(frame, area, el);
        }

        // ── Data display ──────────────────────────────────────────────────────
        // <progress> → Gauge.  `value` / `max` attrs control fill ratio.
        // Example: <progress value="42" max="100" />
        "progress" => {
            render_progress(frame, area, el);
        }

        // <details> → collapsible Block.  `open` attr shows/hides body content;
        // <summary> child supplies the block title.
        // Example: <details open=""><summary>Info</summary><p>…</p></details>
        "details" => {
            render_details(frame, area, el);
        }

        // <summary> is normally consumed by render_details.  When it appears
        // outside <details> fall back to a plain Paragraph.
        "summary" => {
            render_paragraph(frame, area, el);
        }

        // <hr> → a single-line horizontal separator (top-border only Block).
        "hr" => {
            render_hr(frame, area, el);
        }

        // Status bar (design-dojo).
        _ if el.has_class("statusbar") && el.has_class("tui") => {
            render_status_bar(frame, area, el);
        }

        // ── Fallback ──────────────────────────────────────────────────────────
        // Unknown elements render as a plain Block container so content is
        // never silently dropped.
        _ => {
            render_container(frame, area, el);
        }
    }
}

// ── Container (div, section, form, etc.) ─────────────────────────────────────
//
// Maps `<div>`, `<section>`, `<main>`, `<form>`, `<fieldset>`, and similar
// block containers to a ratatui `Block`.
//
// A border is added when the element carries a CSS `border` property, a
// `bordered` CSS class, or `role="group"`.  Flex/grid layout direction is
// resolved from `display` and `flex-direction` styles.

fn render_container(frame: &mut Frame, area: Rect, el: &IrElement) {
    let style = IrStyle::from_element(el);
    let direction = resolve_direction(el);

    // Determine if this container has a border
    let has_border = el.style("border").is_some()
        || el.has_class("bordered")
        || el.attr("role") == Some("group");

    let inner_area = if has_border {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(to_ratatui_style(&style));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        inner
    } else {
        area
    };

    render_children_in_layout(frame, inner_area, &el.children, direction);
}

// ── Paragraph (p, pre, blockquote) ───────────────────────────────────────────
//
// Maps text-flow elements to a ratatui `Paragraph`.  Inline style properties
// (color, font-weight, text-align, etc.) are applied via `IrStyle`.
// `<pre>` content is not word-wrapped so whitespace is preserved.

fn render_paragraph(frame: &mut Frame, area: Rect, el: &IrElement) {
    let style = IrStyle::from_element(el);
    let text_content = collect_styled_text(el);

    let mut para = Paragraph::new(text_content).style(to_ratatui_style(&style));

    if let Some(align) = style.text_align {
        para = para.alignment(to_ratatui_alignment(align));
    }

    if el.tag != "pre" {
        para = para.wrap(Wrap { trim: true });
    }

    frame.render_widget(para, area);
}

// ── Headings (h1–h6) ─────────────────────────────────────────────────────────
//
// Maps `<h1>`–`<h6>` to a bold `Paragraph`.  To convey hierarchy in a
// terminal where font sizes are not available:
//   - h1 / h2: bold + underlined
//   - h3 / h4 / h5 / h6: bold only
//
// Example: `<h1>Dashboard</h1>` renders as an underlined bold line.

fn render_heading(frame: &mut Frame, area: Rect, el: &IrElement) {
    let mut style = IrStyle::from_element(el);
    // Headings are always bold.
    if !style.modifiers.contains(&IrModifier::Bold) {
        style.modifiers.push(IrModifier::Bold);
    }
    // h1 and h2 additionally get underline to visually distinguish hierarchy.
    if matches!(el.tag.as_str(), "h1" | "h2")
        && !style.modifiers.contains(&IrModifier::Underline)
    {
        style.modifiers.push(IrModifier::Underline);
    }

    let text = collect_styled_text(el);
    let para = Paragraph::new(text)
        .style(to_ratatui_style(&style))
        .wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}

// ── Lists (ul, ol) ───────────────────────────────────────────────────────────
//
// Maps `<ul>` / `<ol>` to a ratatui `List`.  Each `<li>` child becomes a
// `ListItem`; ordered lists prefix items with "1. 2. …" and unordered lists
// use a bullet "• ".
//
// Example:
//   <ul><li>Apples</li><li>Bananas</li></ul>
// renders as:
//   • Apples
//   • Bananas

fn render_list(frame: &mut Frame, area: Rect, el: &IrElement) {
    let style = IrStyle::from_element(el);
    let is_ordered = el.tag == "ol";

    let items: Vec<ListItem> = el
        .children
        .iter()
        .enumerate()
        .filter_map(|(i, child)| {
            let text = child.text_content();
            if text.trim().is_empty() {
                return None;
            }
            let prefix = if is_ordered {
                format!("{}. ", i + 1)
            } else {
                "• ".to_string()
            };
            Some(ListItem::new(format!("{prefix}{text}")))
        })
        .collect();

    let list = List::new(items).style(to_ratatui_style(&style));
    frame.render_widget(list, area);
}

// ── HTML Table ────────────────────────────────────────────────────────────────
//
// Maps `<table>` (with optional `<thead>` / `<tbody>` / `<tr>`) to a ratatui
// `Table` widget.  Column widths are distributed evenly by percentage.
//
// Example:
//   <table>
//     <thead><tr><th>Name</th><th>Score</th></tr></thead>
//     <tbody><tr><td>Alice</td><td>99</td></tr></tbody>
//   </table>

fn render_table(frame: &mut Frame, area: Rect, el: &IrElement) {
    let style = IrStyle::from_element(el);

    // Extract header and body rows
    let mut header_cells: Vec<String> = Vec::new();
    let mut body_rows: Vec<Vec<String>> = Vec::new();

    for child in &el.children {
        let Some(child_el) = child.as_element() else {
            continue;
        };
        match child_el.tag.as_str() {
            "thead" => {
                for tr in &child_el.children {
                    if let Some(tr_el) = tr.as_element() {
                        header_cells = tr_el
                            .children
                            .iter()
                            .map(|c| c.text_content())
                            .collect();
                    }
                }
            }
            "tbody" => {
                for tr in &child_el.children {
                    if let Some(tr_el) = tr.as_element() {
                        let row: Vec<String> =
                            tr_el.children.iter().map(|c| c.text_content()).collect();
                        body_rows.push(row);
                    }
                }
            }
            "tr" => {
                // Table without thead/tbody
                let row: Vec<String> =
                    child_el.children.iter().map(|c| c.text_content()).collect();
                if header_cells.is_empty() {
                    header_cells = row;
                } else {
                    body_rows.push(row);
                }
            }
            _ => {}
        }
    }

    let header = Row::new(header_cells.iter().map(|c| Cell::from(c.as_str())))
        .style(Style::default().add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = body_rows
        .iter()
        .map(|row| Row::new(row.iter().map(|c| Cell::from(c.as_str()))))
        .collect();

    let col_count = body_rows
        .iter()
        .map(|r| r.len())
        .max()
        .unwrap_or(0)
        .max(header_cells.len())
        .max(1);
    let widths: Vec<RatConstraint> = (0..col_count)
        .map(|_| RatConstraint::Percentage((100 / col_count as u16).max(1)))
        .collect();

    let table = Table::new(rows, widths)
        .header(header)
        .style(to_ratatui_style(&style))
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(table, area);
}

// ── Design-dojo TUI table (box-drawing passthrough) ──────────────────────────

/// Design-dojo's `<Table tui={true}>` pre-renders box-drawing characters.
/// We just pass the text through as a paragraph.
fn render_tui_table_passthrough(frame: &mut Frame, area: Rect, el: &IrElement) {
    let style = IrStyle::from_element(el);
    let lines: Vec<Line> = el
        .children
        .iter()
        .map(|child| {
            let text = child.text_content();
            let child_style = match child.as_element() {
                Some(child_el) => to_ratatui_style(&IrStyle::from_element(child_el)),
                None => Style::default(),
            };
            Line::from(Span::styled(text, child_style))
        })
        .collect();

    let para = Paragraph::new(lines).style(to_ratatui_style(&style));
    frame.render_widget(para, area);
}

// ── Status bar (design-dojo) ─────────────────────────────────────────────────

fn render_status_bar(frame: &mut Frame, area: Rect, el: &IrElement) {
    let style = IrStyle::from_element(el);
    // Status bar is a single-line horizontal layout
    let spans: Vec<Span> = el
        .children
        .iter()
        .filter_map(|child| {
            let text = child.text_content();
            if text.trim().is_empty() {
                return None;
            }
            let child_style = match child.as_element() {
                Some(child_el) => to_ratatui_style(&IrStyle::from_element(child_el)),
                None => Style::default(),
            };
            Some(Span::styled(text, child_style))
        })
        .collect();

    let line = Line::from(spans);
    let para = Paragraph::new(line).style(to_ratatui_style(&style));
    frame.render_widget(para, area);
}

// ── Input (input, textarea) ───────────────────────────────────────────────────
//
// Maps `<input>` and `<textarea>` to a bordered `Paragraph`.
// - `type="password"` masks the value with "•" characters.
// - A "│" cursor is appended to non-empty values to indicate editability.
// - Placeholder text is shown in a DIM style when the value is empty.
// - The block title comes from `aria-label`, `name`, or `type`.
//
// Example: `<input type="text" value="hello" placeholder="Enter text" />`

fn render_input(frame: &mut Frame, area: Rect, el: &IrElement) {
    let input_type = el.attr("type").unwrap_or("text");
    let value = el.attr("value").unwrap_or("");
    let placeholder = el.attr("placeholder").unwrap_or("");

    let (display, text_style) = if value.is_empty() {
        // Show placeholder text in a dimmed style.
        (placeholder.to_string(), Style::default().add_modifier(Modifier::DIM))
    } else if input_type == "password" {
        // Mask password content.
        ("•".repeat(value.chars().count()), Style::default())
    } else {
        // Show value with a cursor character at the end.
        (format!("{value}│"), Style::default())
    };

    let title = el
        .attr("aria-label")
        .or_else(|| el.attr("name"))
        .unwrap_or(input_type);

    let block = Block::default().borders(Borders::ALL).title(title);
    let para = Paragraph::new(display).block(block).style(text_style);
    frame.render_widget(para, area);
}

// ── Button ────────────────────────────────────────────────────────────────────
//
// Maps `<button>` to a bordered `Paragraph`.
// - `class="selected"` → REVERSED style (focus highlight via `IrStyle`).
// - `disabled` attribute → DIM modifier.
//
// Example: `<button class="selected">Save</button>`

fn render_button(frame: &mut Frame, area: Rect, el: &IrElement) {
    let style = IrStyle::from_element(el);
    let is_disabled = el.attr("disabled").is_some();

    let mut label = String::new();
    for child in &el.children {
        label.push_str(&child.text_content());
    }

    let mut ratatui_style = to_ratatui_style(&style);
    if is_disabled {
        ratatui_style = ratatui_style.add_modifier(Modifier::DIM);
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(ratatui_style);

    let para = Paragraph::new(format!(" {label} "))
        .block(block)
        .style(ratatui_style);

    frame.render_widget(para, area);
}

// ── Code (code) ───────────────────────────────────────────────────────────────
//
// Maps `<code>` to a bordered `Paragraph`, visually distinguishing it from
// plain prose.  Inline styles (e.g. color) are applied as usual.
//
// Example: `<code>fn main() {}</code>`

fn render_code(frame: &mut Frame, area: Rect, el: &IrElement) {
    let style = IrStyle::from_element(el);
    let text_content = collect_styled_text(el);

    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let para = Paragraph::new(text_content).style(to_ratatui_style(&style));
    frame.render_widget(para, inner);
}

// ── Select (select) ───────────────────────────────────────────────────────────
//
// Maps `<select>` to a bordered ratatui `List`.  Each `<option>` child
// becomes a `ListItem`.  The option whose `value` attribute matches the
// `<select>`'s own `value` attribute is highlighted with `REVERSED`.
//
// Example:
//   <select value="b">
//     <option value="a">Option A</option>
//     <option value="b">Option B</option>
//   </select>

fn render_select(frame: &mut Frame, area: Rect, el: &IrElement) {
    let style = IrStyle::from_element(el);
    let selected_value = el.attr("value").unwrap_or("");

    let items: Vec<ListItem> = el
        .children
        .iter()
        .filter_map(|child| {
            let text = child.text_content();
            if text.trim().is_empty() {
                return None;
            }
            let is_selected = child
                .as_element()
                .is_some_and(|opt_el| opt_el.attr("value").unwrap_or("") == selected_value);
            let item_style = if is_selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            Some(ListItem::new(text).style(item_style))
        })
        .collect();

    let title = el.attr("aria-label").unwrap_or("Select");
    let block = Block::default().borders(Borders::ALL).title(title);
    let list = List::new(items).block(block).style(to_ratatui_style(&style));
    frame.render_widget(list, area);
}

// ── Progress (progress) ───────────────────────────────────────────────────────
//
// Maps `<progress>` to a ratatui `Gauge`.
// - `value` attribute: current progress value (default 0).
// - `max` attribute: maximum value (default 100).
//
// The fill ratio is `value / max`, clamped to [0.0, 1.0].
//
// Example: `<progress value="42" max="100" />`

fn render_progress(frame: &mut Frame, area: Rect, el: &IrElement) {
    let style = IrStyle::from_element(el);

    let value: f64 = el
        .attr("value")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    let max: f64 = el
        .attr("max")
        .and_then(|v| v.parse().ok())
        .unwrap_or(100.0);

    let ratio = if max > 0.0 {
        (value / max).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL))
        .gauge_style(to_ratatui_style(&style))
        .ratio(ratio);

    frame.render_widget(gauge, area);
}

// ── Details / Summary (details, summary) ─────────────────────────────────────
//
// Maps `<details>` to a collapsible bordered `Block`.
// - The first `<summary>` child supplies the block title, prefixed with "▼"
//   (open) or "▶" (closed).
// - When the `open` attribute is present the remaining children are rendered
//   inside the block; otherwise only the title border is shown.
//
// Example:
//   <details open="">
//     <summary>More info</summary>
//     <p>Hidden detail content.</p>
//   </details>

fn render_details(frame: &mut Frame, area: Rect, el: &IrElement) {
    let style = IrStyle::from_element(el);
    let is_open = el.attr("open").is_some();

    // Use the first <summary> child as the block title.
    let summary_text: String = el
        .children
        .iter()
        .find(|c| c.as_element().is_some_and(|e| e.tag == "summary"))
        .map(|c| c.text_content())
        .unwrap_or_default();

    let indicator = if is_open { "▼" } else { "▶" };
    let title = format!("{indicator} {summary_text}");

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(to_ratatui_style(&style));

    if is_open {
        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Render all non-summary children inside the block.
        let content: Vec<&IrNode> = el
            .children
            .iter()
            .filter(|c| c.as_element().is_none_or(|e| e.tag != "summary"))
            .collect();
        render_children_in_layout(frame, inner, &content, Direction::Vertical);
    } else {
        frame.render_widget(block, area);
    }
}

// ── Horizontal rule (hr) ──────────────────────────────────────────────────────
//
// Maps `<hr>` to a single-line horizontal separator rendered as a `Block`
// with a top border.
//
// Example: `<hr />`

fn render_hr(frame: &mut Frame, area: Rect, el: &IrElement) {
    let style = IrStyle::from_element(el);
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(to_ratatui_style(&style));
    frame.render_widget(block, area);
}

// ── Layout helpers ───────────────────────────────────────────────────────────

fn render_children_in_layout(
    frame: &mut Frame,
    area: Rect,
    children: &[IrNode],
    direction: Direction,
) {
    // Filter out empty text nodes
    let visible: Vec<&IrNode> = children
        .iter()
        .filter(|c| match c {
            IrNode::Text(s) => !s.trim().is_empty(),
            IrNode::Element(_) => true,
        })
        .collect();

    if visible.is_empty() {
        return;
    }

    if visible.len() == 1 {
        render_ir(frame, area, visible[0]);
        return;
    }

    // Split the area evenly among children
    let rat_dir = match direction {
        Direction::Horizontal => RatDirection::Horizontal,
        Direction::Vertical => RatDirection::Vertical,
    };

    let constraints: Vec<RatConstraint> = visible
        .iter()
        .map(|_| RatConstraint::Ratio(1, visible.len() as u32))
        .collect();

    let chunks = Layout::default()
        .direction(rat_dir)
        .constraints(constraints)
        .split(area);

    for (node, chunk) in visible.iter().zip(chunks.iter()) {
        render_ir(frame, *chunk, node);
    }
}

fn resolve_direction(el: &IrElement) -> Direction {
    // An explicit flex-direction always takes priority.
    if let Some(fd) = el.style("flex-direction") {
        return match fd {
            "column" | "column-reverse" => Direction::Vertical,
            _ => Direction::Horizontal, // row, row-reverse, or unrecognised
        };
    }
    // No explicit flex-direction: `display:flex` mirrors the CSS default (row → horizontal).
    if let Some(display) = el.style("display")
        && (display == "flex" || display == "inline-flex")
    {
        return Direction::Horizontal;
    }
    Direction::Vertical
}

// ── Style conversion ─────────────────────────────────────────────────────────

fn to_ratatui_style(ir: &IrStyle) -> Style {
    let mut style = Style::default();

    if let Some(fg) = ir.fg {
        style = style.fg(to_ratatui_color(fg));
    }
    if let Some(bg) = ir.bg {
        style = style.bg(to_ratatui_color(bg));
    }

    for m in &ir.modifiers {
        style = match m {
            IrModifier::Bold => style.add_modifier(Modifier::BOLD),
            IrModifier::Italic => style.add_modifier(Modifier::ITALIC),
            IrModifier::Underline => style.add_modifier(Modifier::UNDERLINED),
            IrModifier::Dim => style.add_modifier(Modifier::DIM),
            IrModifier::Strikethrough => style.add_modifier(Modifier::CROSSED_OUT),
            IrModifier::Reversed => style.add_modifier(Modifier::REVERSED),
        };
    }

    style
}

fn to_ratatui_color(c: IrColor) -> Color {
    match c {
        IrColor::Rgb(r, g, b) => Color::Rgb(r, g, b),
        IrColor::Default => Color::Reset,
        IrColor::Named(n) => match n {
            NamedColor::Black => Color::Black,
            NamedColor::Red => Color::Red,
            NamedColor::Green => Color::Green,
            NamedColor::Yellow => Color::Yellow,
            NamedColor::Blue => Color::Blue,
            NamedColor::Magenta => Color::Magenta,
            NamedColor::Cyan => Color::Cyan,
            NamedColor::White => Color::White,
            NamedColor::BrightBlack => Color::DarkGray,
            NamedColor::BrightRed => Color::LightRed,
            NamedColor::BrightGreen => Color::LightGreen,
            NamedColor::BrightYellow => Color::LightYellow,
            NamedColor::BrightBlue => Color::LightBlue,
            NamedColor::BrightMagenta => Color::LightMagenta,
            NamedColor::BrightCyan => Color::LightCyan,
            NamedColor::BrightWhite => Color::White,
        },
    }
}

fn to_ratatui_alignment(a: Alignment) -> ratatui::layout::Alignment {
    match a {
        Alignment::Left => ratatui::layout::Alignment::Left,
        Alignment::Center => ratatui::layout::Alignment::Center,
        Alignment::Right => ratatui::layout::Alignment::Right,
    }
}

// ── Styled text collection ───────────────────────────────────────────────────

/// Collect text from element and children, preserving inline styling.
fn collect_styled_text(el: &IrElement) -> Text<'static> {
    // Build a `Text` with multiple `Line`s so that embedded `\n` in text nodes
    // (e.g. within `<pre>` or other multiline content) are represented
    // correctly in ratatui.
    let mut lines: Vec<Line<'static>> = vec![Line::default()];

    // Compute the root element's style and iterate its children directly by
    // reference — avoids cloning the entire element subtree.
    let element_style = to_ratatui_style(&IrStyle::from_element(el));
    let combined_style = match el.tag.as_str() {
        "strong" | "b" => element_style.add_modifier(Modifier::BOLD),
        "em" | "i" => element_style.add_modifier(Modifier::ITALIC),
        "u" => element_style.add_modifier(Modifier::UNDERLINED),
        _ => element_style,
    };
    for child in &el.children {
        collect_lines_with_style(child, &mut lines, combined_style);
    }

    Text::from(lines)
}

fn collect_lines_with_style(
    node: &IrNode,
    lines: &mut Vec<Line<'static>>,
    parent_style: Style,
) {
    match node {
        IrNode::Text(s) => {
            // Split on explicit newlines and start a new `Line` for each `\n`,
            // preserving the accumulated style for each segment.
            let mut first_segment = true;
            for segment in s.split('\n') {
                if !first_segment {
                    // Start a new line after each newline character.
                    lines.push(Line::default());
                }
                first_segment = false;

                if segment.is_empty() {
                    // Empty segments still contribute to line structure (e.g.,
                    // consecutive or trailing newlines yield blank lines),
                    // but do not need an explicit span.
                    continue;
                }

                let span = Span::styled(segment.to_string(), parent_style);
                if let Some(current_line) = lines.last_mut() {
                    current_line.spans.push(span);
                } else {
                    lines.push(Line::from(span));
                }
            }
        }
        IrNode::Element(el) => {
            // Style computed from this element alone.
            let element_style = to_ratatui_style(&IrStyle::from_element(el));

            // Merge parent + element styles so that explicit element properties
            // override inherited ones while leaving others intact.
            let mut combined_style = parent_style.patch(element_style);

            // Semantic inline tags add modifiers regardless of child structure.
            combined_style = match el.tag.as_str() {
                "strong" | "b" => combined_style.add_modifier(Modifier::BOLD),
                "em" | "i" => combined_style.add_modifier(Modifier::ITALIC),
                "u" => combined_style.add_modifier(Modifier::UNDERLINED),
                _ => combined_style,
            };

            for child in &el.children {
                collect_lines_with_style(child, lines, combined_style);
            }
        }
    }
}
// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{IrElement, IrNode};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::collections::HashMap;

    fn test_frame_with<F: FnOnce(&mut Frame, Rect)>(width: u16, height: u16, f: F) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                f(frame, area);
            })
            .unwrap();
        // Extract buffer content
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

    #[test]
    fn renders_text_node() {
        let node = IrNode::text("Hello, terminal!");
        let output = test_frame_with(20, 1, |frame, area| {
            render_ir(frame, area, &node);
        });
        assert!(output.contains("Hello, terminal!"));
    }

    #[test]
    fn renders_paragraph_element() {
        let el = IrElement {
            tag: "p".into(),
            attrs: HashMap::new(),
            styles: HashMap::new(),
            children: vec![IrNode::text("Test paragraph")],
        };
        let node = IrNode::Element(el);
        let output = test_frame_with(20, 1, |frame, area| {
            render_ir(frame, area, &node);
        });
        assert!(output.contains("Test paragraph"));
    }

    #[test]
    fn renders_list() {
        let el = IrElement {
            tag: "ul".into(),
            attrs: HashMap::new(),
            styles: HashMap::new(),
            children: vec![
                IrNode::Element(IrElement {
                    tag: "li".into(),
                    attrs: HashMap::new(),
                    styles: HashMap::new(),
                    children: vec![IrNode::text("Item A")],
                }),
                IrNode::Element(IrElement {
                    tag: "li".into(),
                    attrs: HashMap::new(),
                    styles: HashMap::new(),
                    children: vec![IrNode::text("Item B")],
                }),
            ],
        };
        let node = IrNode::Element(el);
        let output = test_frame_with(20, 4, |frame, area| {
            render_ir(frame, area, &node);
        });
        assert!(output.contains("• Item A"));
        assert!(output.contains("• Item B"));
    }

    // ── resolve_direction regression tests ───────────────────────────────────

    fn make_div(styles: &[(&str, &str)]) -> IrElement {
        IrElement {
            tag: "div".into(),
            attrs: HashMap::new(),
            styles: styles
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            children: Vec::new(),
        }
    }

    #[test]
    fn flex_row_resolves_horizontal() {
        let el = make_div(&[("display", "flex"), ("flex-direction", "row")]);
        assert_eq!(resolve_direction(&el), Direction::Horizontal);
    }

    #[test]
    fn flex_row_reverse_resolves_horizontal() {
        let el = make_div(&[("display", "flex"), ("flex-direction", "row-reverse")]);
        assert_eq!(resolve_direction(&el), Direction::Horizontal);
    }

    #[test]
    fn flex_column_resolves_vertical() {
        let el = make_div(&[("display", "flex"), ("flex-direction", "column")]);
        assert_eq!(resolve_direction(&el), Direction::Vertical);
    }

    #[test]
    fn flex_column_reverse_resolves_vertical() {
        let el = make_div(&[("display", "flex"), ("flex-direction", "column-reverse")]);
        assert_eq!(resolve_direction(&el), Direction::Vertical);
    }

    #[test]
    fn flex_no_direction_defaults_to_horizontal() {
        // CSS default for display:flex with no explicit flex-direction is row.
        let el = make_div(&[("display", "flex")]);
        assert_eq!(resolve_direction(&el), Direction::Horizontal);
    }

    #[test]
    fn non_flex_defaults_to_vertical() {
        let el = make_div(&[]);
        assert_eq!(resolve_direction(&el), Direction::Vertical);
    }

    // ── render_table regression — body rows wider than header ────────────────

    #[test]
    fn renders_table_body_wider_than_header() {
        // Header has 1 cell; body rows have 3 cells each.
        // Previously col_count was clamped to header length (1),
        // silently truncating the extra body columns.
        let table_el = IrElement {
            tag: "table".into(),
            attrs: HashMap::new(),
            styles: HashMap::new(),
            children: vec![
                IrNode::Element(IrElement {
                    tag: "thead".into(),
                    attrs: HashMap::new(),
                    styles: HashMap::new(),
                    children: vec![IrNode::Element(IrElement {
                        tag: "tr".into(),
                        attrs: HashMap::new(),
                        styles: HashMap::new(),
                        children: vec![IrNode::Element(IrElement {
                            tag: "th".into(),
                            attrs: HashMap::new(),
                            styles: HashMap::new(),
                            children: vec![IrNode::text("H1")],
                        })],
                    })],
                }),
                IrNode::Element(IrElement {
                    tag: "tbody".into(),
                    attrs: HashMap::new(),
                    styles: HashMap::new(),
                    children: vec![IrNode::Element(IrElement {
                        tag: "tr".into(),
                        attrs: HashMap::new(),
                        styles: HashMap::new(),
                        children: vec![
                            IrNode::Element(IrElement {
                                tag: "td".into(),
                                attrs: HashMap::new(),
                                styles: HashMap::new(),
                                children: vec![IrNode::text("A")],
                            }),
                            IrNode::Element(IrElement {
                                tag: "td".into(),
                                attrs: HashMap::new(),
                                styles: HashMap::new(),
                                children: vec![IrNode::text("B")],
                            }),
                            IrNode::Element(IrElement {
                                tag: "td".into(),
                                attrs: HashMap::new(),
                                styles: HashMap::new(),
                                children: vec![IrNode::text("C")],
                            }),
                        ],
                    })],
                }),
            ],
        };
        // Render should not panic and should display the extra body cells.
        let node = IrNode::Element(table_el);
        let output = test_frame_with(60, 6, |frame, area| {
            render_ir(frame, area, &node);
        });
        assert!(output.contains("A"));
        assert!(output.contains("B"));
        assert!(output.contains("C"));
    }

    // ── New element mapping tests ─────────────────────────────────────────────

    fn make_el(tag: &str, attrs: &[(&str, &str)], children: Vec<IrNode>) -> IrElement {
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

    // ── <code> → bordered Paragraph ──────────────────────────────────────────

    #[test]
    fn renders_code_element_with_border() {
        // <code> must be rendered inside a border (Borders::ALL → corners '┌', '┐', '└', '┘').
        let el = make_el("code", &[], vec![IrNode::text("let x = 1;")]);
        let node = IrNode::Element(el);
        let output = test_frame_with(20, 3, |frame, area| {
            render_ir(frame, area, &node);
        });
        // Box-drawing corners confirm a border was rendered.
        assert!(output.contains('┌') || output.contains('╔'), "expected border");
        assert!(output.contains("let x = 1;"));
    }

    // ── <select> → List with selection highlight ──────────────────────────────

    #[test]
    fn renders_select_shows_all_options() {
        let el = make_el(
            "select",
            &[("value", "b")],
            vec![
                IrNode::Element(make_el("option", &[("value", "a")], vec![IrNode::text("Alpha")])),
                IrNode::Element(make_el("option", &[("value", "b")], vec![IrNode::text("Beta")])),
                IrNode::Element(make_el("option", &[("value", "c")], vec![IrNode::text("Gamma")])),
            ],
        );
        let node = IrNode::Element(el);
        let output = test_frame_with(30, 8, |frame, area| {
            render_ir(frame, area, &node);
        });
        assert!(output.contains("Alpha"));
        assert!(output.contains("Beta"));
        assert!(output.contains("Gamma"));
    }

    #[test]
    fn renders_select_empty_options_no_panic() {
        // A <select> with no children should render without panicking.
        let el = make_el("select", &[], vec![]);
        let node = IrNode::Element(el);
        let output = test_frame_with(20, 4, |frame, area| {
            render_ir(frame, area, &node);
        });
        // Just verify it renders (border present).
        assert!(!output.is_empty());
    }

    // ── <progress> → Gauge ───────────────────────────────────────────────────

    #[test]
    fn renders_progress_gauge_no_panic() {
        // Gauge with 50% fill — ratatui renders "50%" as the default label.
        let el = make_el("progress", &[("value", "50"), ("max", "100")], vec![]);
        let node = IrNode::Element(el);
        let output = test_frame_with(30, 3, |frame, area| {
            render_ir(frame, area, &node);
        });
        // The default gauge label is the percentage as "50%".
        assert!(output.contains("50%"), "expected gauge percentage label");
    }

    #[test]
    fn renders_progress_zero_max_no_panic() {
        // max=0 should not divide by zero.
        let el = make_el("progress", &[("value", "0"), ("max", "0")], vec![]);
        let node = IrNode::Element(el);
        test_frame_with(20, 3, |frame, area| {
            render_ir(frame, area, &node);
        });
    }

    #[test]
    fn renders_progress_default_values_no_panic() {
        // Missing value/max attributes default to 0/100.
        let el = make_el("progress", &[], vec![]);
        let node = IrNode::Element(el);
        test_frame_with(20, 3, |frame, area| {
            render_ir(frame, area, &node);
        });
    }

    // ── <details> → collapsible Block ────────────────────────────────────────

    #[test]
    fn renders_details_closed_shows_summary_only() {
        // Without `open` the body content must NOT appear.
        let el = make_el(
            "details",
            &[],
            vec![
                IrNode::Element(make_el("summary", &[], vec![IrNode::text("Click me")])),
                IrNode::Element(make_el("p", &[], vec![IrNode::text("Hidden body")])),
            ],
        );
        let node = IrNode::Element(el);
        let output = test_frame_with(30, 5, |frame, area| {
            render_ir(frame, area, &node);
        });
        // Collapsed indicator "▶" must appear.
        assert!(output.contains('▶'), "expected collapsed indicator ▶");
        assert!(output.contains("Click me"), "expected summary text");
        // Body content must NOT be rendered when closed.
        assert!(!output.contains("Hidden body"), "body should be hidden when closed");
    }

    #[test]
    fn renders_details_open_shows_body() {
        // With `open` attribute the body content must be visible.
        let el = make_el(
            "details",
            &[("open", "")],
            vec![
                IrNode::Element(make_el("summary", &[], vec![IrNode::text("Info")])),
                IrNode::Element(make_el("p", &[], vec![IrNode::text("Visible detail")])),
            ],
        );
        let node = IrNode::Element(el);
        let output = test_frame_with(30, 6, |frame, area| {
            render_ir(frame, area, &node);
        });
        // Open indicator "▼" must appear.
        assert!(output.contains('▼'), "expected open indicator ▼");
        assert!(output.contains("Info"), "expected summary text");
        assert!(output.contains("Visible detail"), "body should be visible when open");
    }

    // ── <hr> → horizontal separator ──────────────────────────────────────────

    #[test]
    fn renders_hr_no_panic() {
        // <hr> must render without panicking and produce some output.
        let el = make_el("hr", &[], vec![]);
        let node = IrNode::Element(el);
        let output = test_frame_with(20, 2, |frame, area| {
            render_ir(frame, area, &node);
        });
        assert!(!output.is_empty());
    }

    // ── <h1>/<h2> heading hierarchy ──────────────────────────────────────────

    #[test]
    fn renders_heading_h1() {
        let el = make_el("h1", &[], vec![IrNode::text("Main Title")]);
        let node = IrNode::Element(el);
        let output = test_frame_with(20, 1, |frame, area| {
            render_ir(frame, area, &node);
        });
        assert!(output.contains("Main Title"));
    }

    #[test]
    fn renders_heading_h3() {
        let el = make_el("h3", &[], vec![IrNode::text("Sub Heading")]);
        let node = IrNode::Element(el);
        let output = test_frame_with(20, 1, |frame, area| {
            render_ir(frame, area, &node);
        });
        assert!(output.contains("Sub Heading"));
    }

    // ── <input> improvements ─────────────────────────────────────────────────

    #[test]
    fn renders_input_with_value_and_cursor() {
        let el = make_el("input", &[("type", "text"), ("value", "hello")], vec![]);
        let node = IrNode::Element(el);
        let output = test_frame_with(20, 3, |frame, area| {
            render_ir(frame, area, &node);
        });
        assert!(output.contains("hello"));
        // Cursor character "│" should appear after the value.
        assert!(output.contains('│'), "expected cursor character");
    }

    #[test]
    fn renders_input_with_placeholder_when_empty() {
        let el = make_el(
            "input",
            &[("type", "text"), ("placeholder", "Enter name")],
            vec![],
        );
        let node = IrNode::Element(el);
        let output = test_frame_with(24, 3, |frame, area| {
            render_ir(frame, area, &node);
        });
        assert!(output.contains("Enter name"));
    }

    #[test]
    fn renders_input_password_masked() {
        let el = make_el(
            "input",
            &[("type", "password"), ("value", "secret")],
            vec![],
        );
        let node = IrNode::Element(el);
        let output = test_frame_with(20, 3, |frame, area| {
            render_ir(frame, area, &node);
        });
        // Raw password must NOT appear; masked bullets must.
        assert!(!output.contains("secret"), "password should be masked");
        assert!(output.contains('•'), "expected mask character");
    }

    // ── <button> disabled state ───────────────────────────────────────────────

    #[test]
    fn renders_button_label() {
        let el = make_el("button", &[], vec![IrNode::text("Submit")]);
        let node = IrNode::Element(el);
        let output = test_frame_with(20, 3, |frame, area| {
            render_ir(frame, area, &node);
        });
        assert!(output.contains("Submit"));
    }

    #[test]
    fn renders_disabled_button_no_panic() {
        let el = make_el("button", &[("disabled", "")], vec![IrNode::text("Off")]);
        let node = IrNode::Element(el);
        let output = test_frame_with(20, 3, |frame, area| {
            render_ir(frame, area, &node);
        });
        assert!(output.contains("Off"));
    }

    // ── Unknown element graceful fallback ─────────────────────────────────────

    #[test]
    fn unknown_element_falls_back_to_container() {
        // An unrecognised tag must render its text content without panicking.
        let el = make_el("custom-widget", &[], vec![IrNode::text("Fallback content")]);
        let node = IrNode::Element(el);
        let output = test_frame_with(20, 1, |frame, area| {
            render_ir(frame, area, &node);
        });
        assert!(output.contains("Fallback content"));
    }

    // ── <form> / <fieldset> / <label> explicit dispatch ───────────────────────

    #[test]
    fn renders_form_as_container() {
        let el = make_el("form", &[], vec![IrNode::text("Form content")]);
        let node = IrNode::Element(el);
        let output = test_frame_with(20, 1, |frame, area| {
            render_ir(frame, area, &node);
        });
        assert!(output.contains("Form content"));
    }

    #[test]
    fn renders_label_as_paragraph() {
        let el = make_el("label", &[], vec![IrNode::text("Name:")]);
        let node = IrNode::Element(el);
        let output = test_frame_with(20, 1, |frame, area| {
            render_ir(frame, area, &node);
        });
        assert!(output.contains("Name:"));
    }
}
