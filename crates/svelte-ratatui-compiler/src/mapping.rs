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
    Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, Wrap,
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
        // Block containers
        "div" | "section" | "main" | "article" | "nav" | "aside" | "footer" | "header" => {
            render_container(frame, area, el);
        }

        // Text elements
        "p" | "pre" | "code" | "blockquote" => {
            render_paragraph(frame, area, el);
        }

        // Headings
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            render_heading(frame, area, el);
        }

        // Inline text
        "span" | "strong" | "em" | "b" | "i" | "u" | "a" => {
            // Inline elements rendered as paragraph when they appear as block
            render_paragraph(frame, area, el);
        }

        // Lists
        "ul" | "ol" => {
            render_list(frame, area, el);
        }

        // Tables
        "table" => {
            render_table(frame, area, el);
        }

        // Design-dojo TUI table (pre-rendered box-drawing)
        _ if el.has_class("tui-table") => {
            render_tui_table_passthrough(frame, area, el);
        }

        // Inputs (simplified)
        "input" | "textarea" => {
            render_input(frame, area, el);
        }

        // Buttons
        "button" => {
            render_button(frame, area, el);
        }

        // Status bar (design-dojo)
        _ if el.has_class("statusbar") && el.has_class("tui") => {
            render_status_bar(frame, area, el);
        }

        // Fallback: render children in vertical layout
        _ => {
            render_container(frame, area, el);
        }
    }
}

// ── Container (div, section, etc.) ───────────────────────────────────────────

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

// ── Paragraph (p, pre, code) ─────────────────────────────────────────────────

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

// ── Headings ─────────────────────────────────────────────────────────────────

fn render_heading(frame: &mut Frame, area: Rect, el: &IrElement) {
    let mut style = IrStyle::from_element(el);
    // Headings always bold
    if !style.modifiers.contains(&IrModifier::Bold) {
        style.modifiers.push(IrModifier::Bold);
    }

    let text = collect_styled_text(el);
    let para = Paragraph::new(text)
        .style(to_ratatui_style(&style))
        .wrap(Wrap { trim: true });

    frame.render_widget(para, area);
}

// ── Lists (ul, ol) ──────────────────────────────────────────────────────────

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

// ── HTML Table ───────────────────────────────────────────────────────────────

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

// ── Input ────────────────────────────────────────────────────────────────────

fn render_input(frame: &mut Frame, area: Rect, el: &IrElement) {
    let value = el.attr("value").unwrap_or("");
    let placeholder = el.attr("placeholder").unwrap_or("");
    let display = if value.is_empty() { placeholder } else { value };

    let block = Block::default().borders(Borders::ALL).title("input");
    let para = Paragraph::new(display).block(block);
    frame.render_widget(para, area);
}

// ── Button ───────────────────────────────────────────────────────────────────

fn render_button(frame: &mut Frame, area: Rect, el: &IrElement) {
    let style = IrStyle::from_element(el);
    let mut label = String::new();
    for child in &el.children {
        label.push_str(&child.text_content());
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(to_ratatui_style(&style));

    let para = Paragraph::new(format!(" {label} "))
        .block(block)
        .style(to_ratatui_style(&style));

    frame.render_widget(para, area);
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
}
