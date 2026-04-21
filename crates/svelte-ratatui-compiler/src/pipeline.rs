//! Compilation pipeline — transforms a Svelte component source string into
//! a Rust widget-tree source string for ratatui.
//!
//! Pipeline stages:
//! 1. Extract template HTML from .svelte source
//! 2. Parse HTML → IR tree (reuses adapter's html_parser logic)
//! 3. Run dialect check (validate TUI compatibility)
//! 4. Generate Rust source via codegen

use std::collections::HashMap;

use crate::codegen;
use crate::ir::{IrElement, IrNode};

/// Compile a Svelte component source string to a Rust widget-tree source string.
///
/// The `component_name` is used for the generated struct name.
///
/// # Errors
///
/// Returns an error string describing any compilation failure.
pub fn compile(source: &str) -> Result<String, String> {
    compile_named(source, "Component")
}

/// Compile with an explicit component name.
pub fn compile_named(source: &str, component_name: &str) -> Result<String, String> {
    // Stage 1: Extract template HTML
    let template = extract_template(source);
    if template.trim().is_empty() {
        return Err("empty template — no HTML content found".into());
    }

    // Stage 2: Parse HTML → IR
    let ir = parse_html_to_ir(&template);

    // Stage 3: Generate Rust source
    let rust_source = codegen::generate(component_name, &ir);

    Ok(rust_source)
}

/// Extract the template (HTML) portion of a .svelte file.
///
/// Svelte files have three sections: `<script>`, `<style>`, and template HTML.
/// The template is everything that's NOT inside `<script>` or `<style>` tags.
fn extract_template(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut i = 0;
    let bytes = source.as_bytes();

    while i < bytes.len() {
        // Check for <script or <style tags
        if bytes[i] == b'<' {
            let rest = &source[i..];
            if rest.starts_with("<script") || rest.starts_with("<style") {
                // Find the matching closing tag
                let close_tag = if rest.starts_with("<script") {
                    "</script>"
                } else {
                    "</style>"
                };
                if let Some(end_pos) = source[i..].find(close_tag) {
                    i += end_pos + close_tag.len();
                    continue;
                }
            }
        }
        result.push(source[i..].chars().next().unwrap_or(' '));
        i += source[i..].chars().next().map_or(1, |c| c.len_utf8());
    }

    result
}

/// Parse HTML string into an IR node tree.
///
/// This is a simplified HTML parser for the constrained subset that
/// TUI-mode Svelte components produce. It handles standard tags,
/// attributes, inline styles, and text content.
fn parse_html_to_ir(html: &str) -> IrNode {
    let mut parser = SimpleHtmlParser::new(html);
    let children = parser.parse_nodes();

    // If there's exactly one root element, return it directly
    if children.len() == 1 {
        return children.into_iter().next().unwrap();
    }

    // Otherwise wrap in a root div
    IrNode::Element(IrElement {
        tag: "div".into(),
        attrs: HashMap::new(),
        styles: HashMap::new(),
        children,
    })
}

// ── Minimal HTML parser (inlined to avoid circular dependency with adapter) ──

struct SimpleHtmlParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> SimpleHtmlParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn parse_nodes(&mut self) -> Vec<IrNode> {
        let mut nodes = Vec::new();
        while self.pos < self.input.len() {
            self.skip_whitespace();
            if self.pos >= self.input.len() {
                break;
            }
            if self.starts_with("</") {
                break; // closing tag — return to parent
            }
            if self.starts_with("<") {
                if let Some(node) = self.parse_element() {
                    nodes.push(node);
                }
            } else {
                if let Some(text) = self.parse_text() {
                    if !text.trim().is_empty() {
                        nodes.push(IrNode::text(text));
                    }
                }
            }
        }
        nodes
    }

    fn parse_element(&mut self) -> Option<IrNode> {
        self.expect('<')?;
        let tag = self.parse_tag_name()?;
        let (attrs, styles) = self.parse_attributes();

        // Self-closing?
        self.skip_whitespace();
        if self.starts_with("/>") {
            self.pos += 2;
            return Some(IrNode::Element(IrElement {
                tag,
                attrs,
                styles,
                children: Vec::new(),
            }));
        }
        self.expect('>');

        // Void elements
        if matches!(
            tag.as_str(),
            "br" | "hr" | "img" | "input" | "meta" | "link"
        ) {
            return Some(IrNode::Element(IrElement {
                tag,
                attrs,
                styles,
                children: Vec::new(),
            }));
        }

        // Parse children
        let children = self.parse_nodes();

        // Consume closing tag
        if self.starts_with("</") {
            self.pos += 2;
            // Skip tag name and >
            while self.pos < self.input.len()
                && self.input.as_bytes().get(self.pos) != Some(&b'>')
            {
                self.pos += 1;
            }
            if self.pos < self.input.len() {
                self.pos += 1; // skip >
            }
        }

        Some(IrNode::Element(IrElement {
            tag,
            attrs,
            styles,
            children,
        }))
    }

    fn parse_tag_name(&mut self) -> Option<String> {
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.input.as_bytes()[self.pos];
            if ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            return None;
        }
        Some(self.input[start..self.pos].to_lowercase())
    }

    fn parse_attributes(&mut self) -> (HashMap<String, String>, HashMap<String, String>) {
        let mut attrs = HashMap::new();
        let mut styles = HashMap::new();

        loop {
            self.skip_whitespace();
            if self.pos >= self.input.len()
                || self.input.as_bytes()[self.pos] == b'>'
                || self.starts_with("/>")
            {
                break;
            }

            let name = match self.parse_attr_name() {
                Some(n) => n,
                None => break,
            };

            let value = if self.starts_with("=") {
                self.pos += 1;
                self.parse_attr_value()
            } else {
                String::new()
            };

            if name == "style" {
                // Parse inline styles
                for part in value.split(';') {
                    let part = part.trim();
                    if let Some((k, v)) = part.split_once(':') {
                        styles.insert(k.trim().to_string(), v.trim().to_string());
                    }
                }
            } else {
                attrs.insert(name, value);
            }
        }

        (attrs, styles)
    }

    fn parse_attr_name(&mut self) -> Option<String> {
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.input.as_bytes()[self.pos];
            if ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_' || ch == b':' {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            return None;
        }
        Some(self.input[start..self.pos].to_lowercase())
    }

    fn parse_attr_value(&mut self) -> String {
        self.skip_whitespace();
        if self.pos >= self.input.len() {
            return String::new();
        }

        let quote = self.input.as_bytes()[self.pos];
        if quote == b'"' || quote == b'\'' {
            self.pos += 1;
            let start = self.pos;
            while self.pos < self.input.len() && self.input.as_bytes()[self.pos] != quote {
                self.pos += 1;
            }
            let val = self.input[start..self.pos].to_string();
            if self.pos < self.input.len() {
                self.pos += 1; // skip closing quote
            }
            val
        } else {
            // Unquoted
            let start = self.pos;
            while self.pos < self.input.len()
                && !self.input.as_bytes()[self.pos].is_ascii_whitespace()
                && self.input.as_bytes()[self.pos] != b'>'
            {
                self.pos += 1;
            }
            self.input[start..self.pos].to_string()
        }
    }

    fn parse_text(&mut self) -> Option<String> {
        let start = self.pos;
        while self.pos < self.input.len() && self.input.as_bytes()[self.pos] != b'<' {
            self.pos += 1;
        }
        if self.pos == start {
            return None;
        }
        Some(self.input[start..self.pos].to_string())
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.input.as_bytes()[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn starts_with(&self, s: &str) -> bool {
        self.input[self.pos..].starts_with(s)
    }

    fn expect(&mut self, ch: char) -> Option<()> {
        if self.pos < self.input.len() && self.input.as_bytes()[self.pos] == ch as u8 {
            self.pos += 1;
            Some(())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_template_strips_script_and_style() {
        let source = r#"
<script>
  let count = 0;
</script>

<div>
  <h1>Hello</h1>
</div>

<style>
  h1 { color: red; }
</style>"#;
        let tmpl = extract_template(source);
        assert!(!tmpl.contains("let count"));
        assert!(!tmpl.contains("color: red"));
        assert!(tmpl.contains("<div>"));
        assert!(tmpl.contains("<h1>Hello</h1>"));
    }

    #[test]
    fn compile_simple_div() {
        let source = "<div><h1>Hello</h1><p>World</p></div>";
        let result = compile_named(source, "Hello").unwrap();
        assert!(result.contains("pub struct Hello;"));
        assert!(result.contains("impl SvelteComponent for Hello"));
        assert!(result.contains("render_ir(frame, area, &root)"));
        assert!(result.contains("\"h1\""));
        assert!(result.contains("\"Hello\""));
        assert!(result.contains("\"World\""));
    }

    #[test]
    fn compile_with_script_tag() {
        let source = r#"
<script>
  let name = "World";
</script>

<div>
  <h1>Hello</h1>
</div>"#;
        let result = compile_named(source, "Greeting").unwrap();
        assert!(result.contains("pub struct Greeting;"));
        assert!(result.contains("\"h1\""));
        assert!(!result.contains("let name"));
    }

    #[test]
    fn compile_with_styles() {
        let source = r#"<p style="color: red; font-weight: bold">Styled</p>"#;
        let result = compile_named(source, "Styled").unwrap();
        assert!(result.contains("\"color\""));
        assert!(result.contains("\"red\""));
        assert!(result.contains("\"font-weight\""));
        assert!(result.contains("\"bold\""));
    }

    #[test]
    fn compile_empty_returns_error() {
        let result = compile("");
        assert!(result.is_err());
    }

    #[test]
    fn compile_list() {
        let source = "<ul><li>A</li><li>B</li><li>C</li></ul>";
        let result = compile_named(source, "MyList").unwrap();
        assert!(result.contains("\"ul\""));
        assert!(result.contains("\"li\""));
        assert!(result.contains("\"A\""));
        assert!(result.contains("\"B\""));
        assert!(result.contains("\"C\""));
    }
}
