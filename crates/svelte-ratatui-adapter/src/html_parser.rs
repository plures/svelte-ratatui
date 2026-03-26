//! Lightweight HTML → IR parser.
//!
//! Parses the simplified HTML that design-dojo TUI-mode components produce
//! into an [`IrNode`] tree. This is NOT a full HTML5 parser — it handles the
//! constrained subset that TUI-mode Svelte components emit:
//!
//! - Standard tags (div, p, span, table, ul, etc.)
//! - Class attributes and inline styles
//! - Text content
//! - Self-closing tags
//! - No script/style blocks (these are stripped before we see the DOM)

use std::collections::HashMap;
use svelte_ratatui_compiler::ir::{IrElement, IrNode};

/// Parse an HTML string into an IR node tree.
///
/// Returns a root `div` element containing the parsed children.
pub fn parse_html(html: &str) -> IrNode {
    let mut parser = HtmlParser::new(html);
    let children = parser.parse_nodes();
    IrNode::Element(IrElement {
        tag: "div".into(),
        attrs: HashMap::new(),
        styles: HashMap::new(),
        children,
    })
}

struct HtmlParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> HtmlParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn remaining(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn starts_with(&self, s: &str) -> bool {
        self.remaining().starts_with(s)
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
    }

    fn consume_char(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn consume_until(&mut self, target: &str) -> String {
        let mut result = String::new();
        while !self.starts_with(target) && self.pos < self.input.len() {
            if let Some(c) = self.consume_char() {
                result.push(c);
            }
        }
        result
    }

    fn parse_nodes(&mut self) -> Vec<IrNode> {
        let mut nodes = Vec::new();
        while self.pos < self.input.len() {
            // Skip whitespace only when it sits between two tags — i.e. the
            // first non-whitespace character is '<'. Whitespace that precedes
            // actual text content is preserved (important for <pre>/<code>).
            if self.peek().is_some_and(|c| c.is_whitespace()) {
                let saved = self.pos;
                self.skip_whitespace();
                if !self.starts_with("<") {
                    // Whitespace is part of text content — restore position.
                    self.pos = saved;
                }
            }

            if self.pos >= self.input.len() {
                break;
            }

            if self.starts_with("</") {
                // Closing tag — parent will handle this
                break;
            } else if self.starts_with("<!--") {
                // Skip comments
                self.consume_until("-->");
                if self.starts_with("-->") {
                    self.pos += 3;
                }
            } else if self.starts_with("<") {
                if let Some(el) = self.parse_element() {
                    nodes.push(el);
                }
            } else {
                // Text node — preserve content exactly, only drop truly empty strings
                let text = self.consume_until("<");
                let decoded = decode_entities(&text);
                if !decoded.is_empty() {
                    nodes.push(IrNode::text(decoded));
                }
            }
        }
        nodes
    }

    fn parse_element(&mut self) -> Option<IrNode> {
        // Consume '<'
        self.consume_char()?;

        // Parse tag name
        let tag = self.parse_tag_name();
        if tag.is_empty() {
            return None;
        }

        // Parse attributes
        let (attrs, styles) = self.parse_attributes();

        // Check for self-closing
        self.skip_whitespace();
        let self_closing = if self.starts_with("/>") {
            self.pos += 2;
            true
        } else if self.starts_with(">") {
            self.pos += 1;
            false
        } else {
            // Malformed — try to recover
            self.consume_until(">");
            if self.starts_with(">") {
                self.pos += 1;
            }
            false
        };

        // Void elements
        let is_void = matches!(
            tag.as_str(),
            "br" | "hr" | "img" | "input" | "meta" | "link" | "area" | "base" | "col"
        );

        let children = if self_closing || is_void {
            Vec::new()
        } else {
            let children = self.parse_nodes();
            // Consume closing tag
            let close_tag = format!("</{tag}>");
            if self.starts_with(&close_tag) {
                self.pos += close_tag.len();
            } else if self.starts_with("</") {
                // Mismatched close tag — consume it anyway
                self.consume_until(">");
                if self.starts_with(">") {
                    self.pos += 1;
                }
            }
            children
        };

        Some(IrNode::Element(IrElement {
            tag,
            attrs,
            styles,
            children,
        }))
    }

    fn parse_tag_name(&mut self) -> String {
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                name.push(c);
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        name.to_lowercase()
    }

    fn parse_attributes(&mut self) -> (HashMap<String, String>, HashMap<String, String>) {
        let mut attrs = HashMap::new();
        let mut styles = HashMap::new();

        loop {
            self.skip_whitespace();
            if self.starts_with(">") || self.starts_with("/>") || self.pos >= self.input.len() {
                break;
            }

            let key = self.parse_attr_name();
            if key.is_empty() {
                // Skip unknown character
                self.consume_char();
                continue;
            }

            self.skip_whitespace();
            let value = if self.starts_with("=") {
                self.consume_char(); // '='
                self.skip_whitespace();
                self.parse_attr_value()
            } else {
                // Boolean attribute
                String::new()
            };

            if key == "style" {
                // Parse inline style into key-value pairs; normalize keys to
                // lowercase so lookups like `el.style("color")` always work
                // regardless of the HTML author's casing.
                for part in value.split(';') {
                    let part = part.trim();
                    if let Some((k, v)) = part.split_once(':') {
                        styles.insert(k.trim().to_lowercase(), v.trim().to_string());
                    }
                }
            } else {
                attrs.insert(key, value);
            }
        }

        (attrs, styles)
    }

    fn parse_attr_name(&mut self) -> String {
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ':' {
                name.push(c);
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        name.to_lowercase()
    }

    fn parse_attr_value(&mut self) -> String {
        if self.starts_with("\"") {
            self.consume_char(); // opening "
            let val = self.consume_until("\"");
            if self.starts_with("\"") {
                self.consume_char(); // closing "
            }
            decode_entities(&val)
        } else if self.starts_with("'") {
            self.consume_char(); // opening '
            let val = self.consume_until("'");
            if self.starts_with("'") {
                self.consume_char(); // closing '
            }
            decode_entities(&val)
        } else {
            // Unquoted attribute value
            let mut val = String::new();
            while let Some(c) = self.peek() {
                if c.is_whitespace() || c == '>' || c == '/' {
                    break;
                }
                val.push(c);
                self.pos += c.len_utf8();
            }
            val
        }
    }
}

/// Decode basic HTML entities.
fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_paragraph() {
        let ir = parse_html("<p>Hello world</p>");
        let root = ir.as_element().unwrap();
        assert_eq!(root.children.len(), 1);
        let p = root.children[0].as_element().unwrap();
        assert_eq!(p.tag, "p");
        assert_eq!(p.children[0].text_content(), "Hello world");
    }

    #[test]
    fn parse_nested_elements() {
        let ir = parse_html("<div><p>A</p><p>B</p></div>");
        let root = ir.as_element().unwrap();
        let div = root.children[0].as_element().unwrap();
        assert_eq!(div.tag, "div");
        assert_eq!(div.children.len(), 2);
    }

    #[test]
    fn parse_attributes_and_styles() {
        let ir = parse_html(r#"<div class="tui-row selected" style="color: #00d4ff; font-weight: bold;">text</div>"#);
        let root = ir.as_element().unwrap();
        let div = root.children[0].as_element().unwrap();
        assert!(div.has_class("tui-row"));
        assert!(div.has_class("selected"));
        assert_eq!(div.style("color"), Some("#00d4ff"));
        assert_eq!(div.style("font-weight"), Some("bold"));
    }

    #[test]
    fn parse_self_closing() {
        let ir = parse_html("<div><br/><hr /></div>");
        let root = ir.as_element().unwrap();
        let div = root.children[0].as_element().unwrap();
        assert_eq!(div.children.len(), 2);
    }

    #[test]
    fn parse_design_dojo_tui_table() {
        let html = r#"<div class="tui-table" role="grid" aria-label="table">
            <div class="tui-border" aria-hidden="true">╔════╦════╗</div>
            <div class="tui-header" role="row">║ A  ║ B  ║</div>
            <div class="tui-border" aria-hidden="true">╠════╬════╣</div>
            <div class="tui-row" role="row">║ 1  ║ 2  ║</div>
            <div class="tui-border" aria-hidden="true">╚════╩════╝</div>
        </div>"#;
        let ir = parse_html(html);
        let root = ir.as_element().unwrap();
        let table = root.children[0].as_element().unwrap();
        assert!(table.has_class("tui-table"));
        assert_eq!(table.children.len(), 5);
    }

    #[test]
    fn entity_decoding() {
        let ir = parse_html("<p>&lt;hello&gt; &amp; &quot;world&quot;</p>");
        let root = ir.as_element().unwrap();
        let text = root.children[0].text_content();
        assert_eq!(text, r#"<hello> & "world""#);
    }

    #[test]
    fn parse_html_comment_skipped() {
        let ir = parse_html("<div><!-- comment --><p>visible</p></div>");
        let root = ir.as_element().unwrap();
        let div = root.children[0].as_element().unwrap();
        assert_eq!(div.children.len(), 1);
        assert_eq!(div.children[0].text_content(), "visible");
    }

    #[test]
    fn space_before_inline_tag_is_preserved() {
        // "Hello " is consumed by consume_until('<'), so the trailing space
        // becomes part of the text node — not dropped by whitespace skipping.
        let ir = parse_html("<p>Hello <b>world</b></p>");
        let root = ir.as_element().unwrap();
        let p = root.children[0].as_element().unwrap();
        // First child must be text "Hello " (with space)
        let text_content = p.children[0].text_content();
        assert!(
            text_content.ends_with(' '),
            "expected trailing space in text node, got {:?}",
            text_content
        );
        assert!(text_content.contains("Hello"));
    }

    #[test]
    fn style_key_mixed_case_normalised_to_lowercase() {
        // Inline style keys like "Color" or "Font-Weight" must be stored as
        // lowercase so that IrStyle::from_element lookups work correctly.
        let ir =
            parse_html(r#"<div style="Color: red; Font-Weight: bold;">text</div>"#);
        let root = ir.as_element().unwrap();
        let div = root.children[0].as_element().unwrap();
        assert_eq!(div.style("color"), Some("red"));
        assert_eq!(div.style("font-weight"), Some("bold"));
    }
}
