//! Intermediate Representation for the Svelte→ratatui pipeline.
//!
//! The IR sits between two input paths:
//! - **Adapter path (runtime):** HTML string → [`IrNode`] tree
//! - **Compiler path (build-time):** Svelte AST → [`IrNode`] tree
//!
//! …and a single output path:
//! - [`IrNode`] tree → ratatui widget tree (see `mapping` module)
//!
//! The IR deliberately mirrors a simplified HTML DOM: elements have a tag,
//! attributes, inline styles, and children.  Text nodes carry a plain string.
//! This keeps the adapter path trivial (HTML parse → IR is almost 1:1) while
//! giving the compiler path a stable target to emit.

use std::collections::HashMap;

// ── Node types ───────────────────────────────────────────────────────────────

/// A single node in the IR tree.
#[derive(Debug, Clone, PartialEq)]
pub enum IrNode {
    /// An element node (mirrors an HTML element).
    Element(IrElement),
    /// A text node (literal string content).
    Text(String),
}

/// An element in the IR tree.
#[derive(Debug, Clone, PartialEq)]
pub struct IrElement {
    /// Lowercase tag name, e.g. `"div"`, `"p"`, `"table"`.
    pub tag: String,
    /// HTML attributes (class, role, aria-*, data-*, etc.).
    pub attrs: HashMap<String, String>,
    /// Parsed inline styles as key→value.
    pub styles: HashMap<String, String>,
    /// Child nodes.
    pub children: Vec<IrNode>,
}

// ── Style types ──────────────────────────────────────────────────────────────

/// Terminal-safe color, derived from CSS color values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrColor {
    /// One of the 16 base terminal colors.
    Named(NamedColor),
    /// 24-bit RGB color.
    Rgb(u8, u8, u8),
    /// Use the terminal's default fg or bg.
    Default,
}

/// Named terminal color constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

/// Text modifiers that map to ratatui `Modifier` flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrModifier {
    Bold,
    Italic,
    Underline,
    Dim,
    Strikethrough,
    Reversed,
}

/// Resolved style information for a single element.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IrStyle {
    pub fg: Option<IrColor>,
    pub bg: Option<IrColor>,
    pub modifiers: Vec<IrModifier>,
    pub text_align: Option<Alignment>,
}

/// Text alignment, matching ratatui's `Alignment`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
}

/// Layout direction for flex containers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// `flex-direction: row` → horizontal.
    Horizontal,
    /// `flex-direction: column` or default block flow → vertical.
    #[default]
    Vertical,
}

/// A layout constraint for sizing, derived from CSS width/height/flex.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Constraint {
    /// Fixed character count (from `width: Npx` where N is divided by char width).
    Length(u16),
    /// Percentage of parent (from `width: N%`).
    Percentage(u16),
    /// Minimum size.
    Min(u16),
    /// Maximum size.
    Max(u16),
    /// Fill remaining space (from `flex-grow`).
    Fill(u16),
}

// ── Constructors ─────────────────────────────────────────────────────────────

impl IrNode {
    /// Create a text node.
    pub fn text(s: impl Into<String>) -> Self {
        IrNode::Text(s.into())
    }

    /// Create an element node with no attributes, styles, or children.
    pub fn element(tag: impl Into<String>) -> Self {
        IrNode::Element(IrElement {
            tag: tag.into(),
            attrs: HashMap::new(),
            styles: HashMap::new(),
            children: Vec::new(),
        })
    }

    /// Return the element mutably, panics if this is a Text node.
    pub fn as_element_mut(&mut self) -> &mut IrElement {
        match self {
            IrNode::Element(el) => el,
            IrNode::Text(_) => panic!("called as_element_mut on Text node"),
        }
    }

    /// Return the element reference, or `None` for text nodes.
    pub fn as_element(&self) -> Option<&IrElement> {
        match self {
            IrNode::Element(el) => Some(el),
            IrNode::Text(_) => None,
        }
    }

    /// Collect all text content from this node and descendants.
    pub fn text_content(&self) -> String {
        match self {
            IrNode::Text(s) => s.clone(),
            IrNode::Element(el) => el
                .children
                .iter()
                .map(|c| c.text_content())
                .collect::<Vec<_>>()
                .join(""),
        }
    }
}

impl IrElement {
    /// Get an attribute value.
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.get(key).map(|s| s.as_str())
    }

    /// Check if the element has a given CSS class.
    pub fn has_class(&self, class: &str) -> bool {
        self.attrs
            .get("class")
            .map_or(false, |c| c.split_whitespace().any(|w| w == class))
    }

    /// Get an inline style value.
    pub fn style(&self, key: &str) -> Option<&str> {
        self.styles.get(key).map(|s| s.as_str())
    }

    /// Push a child node.
    pub fn push_child(&mut self, child: IrNode) {
        self.children.push(child);
    }
}

// ── Style resolution ─────────────────────────────────────────────────────────

impl IrStyle {
    /// Resolve style from an element's inline styles and class hints.
    pub fn from_element(el: &IrElement) -> Self {
        let mut style = IrStyle::default();

        // Foreground color
        if let Some(color_str) = el.style("color") {
            style.fg = Some(parse_color(color_str));
        }

        // Background color
        if let Some(bg_str) = el.style("background-color").or_else(|| el.style("background")) {
            style.bg = Some(parse_color(bg_str));
        }

        // Font weight
        if let Some(fw) = el.style("font-weight") {
            if fw == "bold" || fw == "700" || fw == "800" || fw == "900" {
                style.modifiers.push(IrModifier::Bold);
            }
        }

        // Font style
        if let Some(fs) = el.style("font-style") {
            if fs == "italic" {
                style.modifiers.push(IrModifier::Italic);
            }
        }

        // Text decoration
        if let Some(td) = el.style("text-decoration") {
            if td.contains("underline") {
                style.modifiers.push(IrModifier::Underline);
            }
            if td.contains("line-through") {
                style.modifiers.push(IrModifier::Strikethrough);
            }
        }

        // Text alignment
        if let Some(ta) = el.style("text-align") {
            style.text_align = Some(match ta {
                "center" => Alignment::Center,
                "right" => Alignment::Right,
                _ => Alignment::Left,
            });
        }

        // Class-based hints from design-dojo TUI mode
        if el.has_class("selected") {
            style.modifiers.push(IrModifier::Reversed);
        }

        style
    }
}

// ── Color parsing ────────────────────────────────────────────────────────────

/// Parse a CSS color string into an [`IrColor`].
pub fn parse_color(s: &str) -> IrColor {
    let s = s.trim().to_lowercase();

    // Hex colors
    if let Some(hex) = s.strip_prefix('#') {
        if let Some(c) = parse_hex_color(hex) {
            return c;
        }
    }

    // rgb(r, g, b)
    if let Some(inner) = s.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 3 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                parts[0].trim().parse::<u8>(),
                parts[1].trim().parse::<u8>(),
                parts[2].trim().parse::<u8>(),
            ) {
                return IrColor::Rgb(r, g, b);
            }
        }
    }

    // Named colors
    match s.as_str() {
        "black" => IrColor::Named(NamedColor::Black),
        "red" => IrColor::Named(NamedColor::Red),
        "green" => IrColor::Named(NamedColor::Green),
        "yellow" => IrColor::Named(NamedColor::Yellow),
        "blue" => IrColor::Named(NamedColor::Blue),
        "magenta" | "fuchsia" => IrColor::Named(NamedColor::Magenta),
        "cyan" | "aqua" => IrColor::Named(NamedColor::Cyan),
        "white" => IrColor::Named(NamedColor::White),
        "gray" | "grey" => IrColor::Named(NamedColor::BrightBlack),
        _ => IrColor::Default,
    }
}

fn parse_hex_color(hex: &str) -> Option<IrColor> {
    let hex = hex.trim();
    match hex.len() {
        // #RGB → expand to #RRGGBB
        3 => {
            let chars: Vec<char> = hex.chars().collect();
            let r = u8::from_str_radix(&format!("{}{}", chars[0], chars[0]), 16).ok()?;
            let g = u8::from_str_radix(&format!("{}{}", chars[1], chars[1]), 16).ok()?;
            let b = u8::from_str_radix(&format!("{}{}", chars[2], chars[2]), 16).ok()?;
            Some(IrColor::Rgb(r, g, b))
        }
        // #RRGGBB
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(IrColor::Rgb(r, g, b))
        }
        _ => None,
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_content_of_nested_elements() {
        let mut div = IrElement {
            tag: "div".into(),
            attrs: HashMap::new(),
            styles: HashMap::new(),
            children: vec![
                IrNode::text("hello "),
                IrNode::Element(IrElement {
                    tag: "span".into(),
                    attrs: HashMap::new(),
                    styles: HashMap::new(),
                    children: vec![IrNode::text("world")],
                }),
            ],
        };
        let node = IrNode::Element(div);
        assert_eq!(node.text_content(), "hello world");
    }

    #[test]
    fn parse_hex_colors() {
        assert_eq!(parse_color("#ff0000"), IrColor::Rgb(255, 0, 0));
        assert_eq!(parse_color("#0f0"), IrColor::Rgb(0, 255, 0));
        assert_eq!(parse_color("#1a1a2e"), IrColor::Rgb(26, 26, 46));
    }

    #[test]
    fn parse_rgb_color() {
        assert_eq!(parse_color("rgb(255, 128, 0)"), IrColor::Rgb(255, 128, 0));
    }

    #[test]
    fn parse_named_colors() {
        assert_eq!(parse_color("red"), IrColor::Named(NamedColor::Red));
        assert_eq!(parse_color("cyan"), IrColor::Named(NamedColor::Cyan));
        assert_eq!(parse_color("aqua"), IrColor::Named(NamedColor::Cyan));
    }

    #[test]
    fn has_class() {
        let mut el = IrElement {
            tag: "div".into(),
            attrs: HashMap::from([("class".into(), "tui-row selected".into())]),
            styles: HashMap::new(),
            children: vec![],
        };
        assert!(el.has_class("selected"));
        assert!(el.has_class("tui-row"));
        assert!(!el.has_class("gui"));
    }

    #[test]
    fn style_resolution_from_element() {
        let el = IrElement {
            tag: "span".into(),
            attrs: HashMap::from([("class".into(), "selected".into())]),
            styles: HashMap::from([
                ("color".into(), "#00d4ff".into()),
                ("font-weight".into(), "bold".into()),
                ("text-align".into(), "center".into()),
            ]),
            children: vec![],
        };
        let style = IrStyle::from_element(&el);
        assert_eq!(style.fg, Some(IrColor::Rgb(0, 212, 255)));
        assert!(style.modifiers.contains(&IrModifier::Bold));
        assert!(style.modifiers.contains(&IrModifier::Reversed));
        assert_eq!(style.text_align, Some(Alignment::Center));
    }
}
