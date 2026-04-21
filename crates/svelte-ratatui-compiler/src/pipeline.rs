//! Compilation pipeline — transforms a Svelte component source string into
//! a Rust widget-tree source string for ratatui.

use crate::dialect_check::check;
use crate::ir::{IrElement, IrNode};
use std::collections::HashMap;
use std::fmt::Write;

/// Compile a Svelte component source string to a Rust widget-tree source string.
///
/// # Errors
///
/// Returns an error string describing any compilation failure.
pub fn compile(source: &str) -> Result<String, String> {
    let errors = check(source);
    if !errors.is_empty() {
        let message = errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        return Err(message);
    }

    let template = extract_template_html(source);
    let ir = parse_html(&template);
    Ok(generate_component_source(&ir))
}

fn generate_component_source(ir: &IrNode) -> String {
    let mut out = String::new();
    out.push_str("#[derive(Debug, Clone, PartialEq)]\n");
    out.push_str("pub enum IrNode {\n");
    out.push_str("    Element(IrElement),\n");
    out.push_str("    Text(String),\n");
    out.push_str("}\n\n");
    out.push_str("#[derive(Debug, Clone, PartialEq)]\n");
    out.push_str("pub struct IrElement {\n");
    out.push_str("    pub tag: String,\n");
    out.push_str("    pub attrs: std::collections::HashMap<String, String>,\n");
    out.push_str("    pub styles: std::collections::HashMap<String, String>,\n");
    out.push_str("    pub children: Vec<IrNode>,\n");
    out.push_str("}\n\n");
    out.push_str("pub struct Frame;\n");
    out.push_str("#[derive(Clone, Copy)]\n");
    out.push_str("pub struct Rect;\n");
    out.push_str("pub enum Event { Unknown }\n\n");
    out.push_str("pub trait SvelteComponent {\n");
    out.push_str("    fn render(&self, frame: &mut Frame, area: Rect);\n");
    out.push_str("    fn handle_event(&mut self, event: Event) -> bool;\n");
    out.push_str("    fn poll_async(&mut self) -> bool;\n");
    out.push_str("}\n\n");
    out.push_str("pub struct CompiledComponent;\n\n");
    out.push_str("impl CompiledComponent {\n");
    out.push_str("    pub fn new() -> Self {\n");
    out.push_str("        Self\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("impl SvelteComponent for CompiledComponent {\n");
    out.push_str("    fn render(&self, frame: &mut Frame, area: Rect) {\n");
    out.push_str("        let _ = (frame, area);\n");
    out.push_str("        let _ir = build_ir();\n");
    out.push_str("    }\n\n");
    out.push_str("    fn handle_event(&mut self, _event: Event) -> bool {\n");
    out.push_str("        false\n");
    out.push_str("    }\n\n");
    out.push_str("    fn poll_async(&mut self) -> bool {\n");
    out.push_str("        false\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("fn build_ir() -> IrNode {\n");
    write_ir_node(ir, &mut out, 1);
    out.push_str("\n}\n");
    out
}

fn write_ir_node(node: &IrNode, out: &mut String, indent: usize) {
    let pad = "    ".repeat(indent);
    match node {
        IrNode::Text(text) => {
            let _ = write!(
                out,
                "{pad}IrNode::Text({}.to_string())",
                rust_string_literal(text)
            );
        }
        IrNode::Element(el) => {
            let _ = writeln!(out, "{pad}IrNode::Element(");
            let _ = writeln!(out, "{pad}    IrElement {{");
            let _ = writeln!(
                out,
                "{pad}        tag: {}.to_string(),",
                rust_string_literal(&el.tag)
            );
            write_map(&el.attrs, out, indent + 2, "attrs");
            write_map(&el.styles, out, indent + 2, "styles");
            let _ = writeln!(out, "{pad}        children: vec![");
            for child in &el.children {
                write_ir_node(child, out, indent + 3);
                out.push_str(",\n");
            }
            let _ = writeln!(out, "{pad}        ],");
            let _ = writeln!(out, "{pad}    }},");
            let _ = write!(out, "{pad})");
        }
    }
}

fn write_map(map: &HashMap<String, String>, out: &mut String, indent: usize, field: &str) {
    let pad = "    ".repeat(indent);
    if map.is_empty() {
        let _ = writeln!(out, "{pad}{field}: std::collections::HashMap::new(),");
        return;
    }

    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    let _ = writeln!(out, "{pad}{field}: vec![");
    for (k, v) in entries {
        let _ = writeln!(
            out,
            "{pad}    ({}.to_string(), {}.to_string()),",
            rust_string_literal(k),
            rust_string_literal(v)
        );
    }
    let _ = writeln!(out, "{pad}].into_iter().collect(),");
}

fn rust_string_literal(s: &str) -> String {
    format!("{s:?}")
}

/// Remove all `<script ...>...</script>` blocks and return the remaining
/// template HTML.
fn extract_template_html(source: &str) -> String {
    let mut rest = source;
    let mut template = String::new();

    while let Some(script_start) = rest.find("<script") {
        template.push_str(&rest[..script_start]);

        let Some(after_open_rel) = rest[script_start..].find('>') else {
            return template;
        };
        let after_open = script_start + after_open_rel + 1;

        let Some(close_rel) = rest[after_open..].find("</script>") else {
            return template;
        };
        let after_close = after_open + close_rel + "</script>".len();
        rest = &rest[after_close..];
    }

    template.push_str(rest);
    template
}

fn parse_html(html: &str) -> IrNode {
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
            if self.peek().is_some_and(|c| c.is_whitespace()) {
                let saved = self.pos;
                self.skip_whitespace();
                let skipped = &self.input[saved..self.pos];
                // Preserve horizontal whitespace (e.g. between inline elements),
                // but drop indentation-only runs before the next opening tag.
                if !skipped.contains('\n') || !self.starts_with("<") {
                    self.pos = saved;
                }
            }

            if self.pos >= self.input.len() {
                break;
            }

            if self.starts_with("</") {
                break;
            } else if self.starts_with("<!--") {
                self.consume_until("-->");
                if self.starts_with("-->") {
                    self.pos += 3;
                }
            } else if self.starts_with("<") {
                if let Some(el) = self.parse_element() {
                    nodes.push(el);
                }
            } else {
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
        self.consume_char()?;
        let tag = self.parse_tag_name();
        if tag.is_empty() {
            return None;
        }

        let (attrs, styles) = self.parse_attributes();

        self.skip_whitespace();
        let self_closing = if self.starts_with("/>") {
            self.pos += 2;
            true
        } else if self.starts_with(">") {
            self.pos += 1;
            false
        } else {
            self.consume_until(">");
            if self.starts_with(">") {
                self.pos += 1;
            }
            false
        };

        let is_void = matches!(
            tag.as_str(),
            "br" | "hr" | "img" | "input" | "meta" | "link" | "area" | "base" | "col"
        );

        let children = if self_closing || is_void {
            Vec::new()
        } else {
            let children = self.parse_nodes();
            let close_tag = format!("</{tag}>");
            if self.starts_with(&close_tag) {
                self.pos += close_tag.len();
            } else if self.starts_with("</") {
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
                self.consume_char();
                continue;
            }

            self.skip_whitespace();
            let value = if self.starts_with("=") {
                self.consume_char();
                self.skip_whitespace();
                self.parse_attr_value()
            } else {
                String::new()
            };

            if key == "style" {
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
            self.consume_char();
            let val = self.consume_until("\"");
            if self.starts_with("\"") {
                self.consume_char();
            }
            decode_entities(&val)
        } else if self.starts_with("'") {
            self.consume_char();
            let val = self.consume_until("'");
            if self.starts_with("'") {
                self.consume_char();
            }
            decode_entities(&val)
        } else {
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

fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn compile_returns_non_empty_source_with_trait_impl() {
        let src = "<div><h1>Hello</h1><p>World</p></div>";
        let out = compile(src).expect("compile should succeed for static html");
        assert!(!out.trim().is_empty());
        assert!(out.contains("impl SvelteComponent for CompiledComponent"));
        assert!(out.contains("\"h1\""));
        assert!(out.contains("\"p\""));
    }

    #[test]
    fn extract_template_html_removes_script_blocks() {
        let src = r#"
<script>
  let msg = $state('x');
</script>
<div><p>Hello</p></div>
<script context=\"module\">
  export const ssr = false;
</script>
<span>tail</span>
"#;
        let template = extract_template_html(src);
        assert!(!template.contains("<script"));
        assert!(template.contains("<div><p>Hello</p></div>"));
        assert!(template.contains("<span>tail</span>"));
    }

    #[test]
    fn compile_rejects_dialect_violations() {
        let src = "<div>{@html raw}</div>";
        let err = compile(src).expect_err("compile should reject disallowed dialect");
        assert!(err.contains("E003"));
    }

    #[test]
    fn compile_output_compiles_with_rustc() {
        let src = "<div><h1>Hello</h1><p>World</p></div>";
        let out = compile(src).expect("compile should succeed for static html");

        let timestamp_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time should be after unix epoch")
            .as_nanos();
        let tmp = std::env::temp_dir();
        let source_path = tmp.join(format!("svelte-ratatui-generated-{timestamp_nanos}.rs"));
        let output_path = tmp.join(format!("svelte-ratatui-generated-{timestamp_nanos}.rlib"));

        fs::write(&source_path, out).expect("should write generated source file");

        let status = Command::new("rustc")
            .arg("--edition=2021")
            .arg("--crate-type=lib")
            .arg(&source_path)
            .arg("-o")
            .arg(&output_path)
            .status()
            .expect("rustc should be available");

        let _ = fs::remove_file(&source_path);
        let _ = fs::remove_file(&output_path);
        assert!(status.success(), "generated rust source should compile");
    }
}
