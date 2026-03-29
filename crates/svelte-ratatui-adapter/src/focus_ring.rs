//! Focus ring overlay widget.
//!
//! Renders a highlighted border around the currently-focused widget's area in
//! the ratatui buffer. Components use this by rendering it last on top of the
//! focused widget's [`Rect`], typically driven by the `focused` field of a
//! [`crate::dom_reader::DomSnapshot`].
//!
//! # Example
//!
//! ```rust,ignore
//! // Inside a SvelteComponent::render implementation:
//! if let Some(focused_rect) = self.focused_rect {
//!     FocusRing::default().render(focused_rect, frame.buffer_mut());
//! }
//! ```

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};

/// A focus ring overlay that renders a single-cell border around a widget area.
///
/// The default style uses a bright yellow foreground to make the focus
/// indicator clearly visible against most terminal colour schemes.
///
/// Use the builder methods to customise the style before rendering.
#[derive(Debug, Clone)]
pub struct FocusRing {
    style: Style,
}

impl Default for FocusRing {
    fn default() -> Self {
        Self::new()
    }
}

impl FocusRing {
    /// Create a new [`FocusRing`] with the default bright-yellow style.
    pub fn new() -> Self {
        Self {
            style: Style::default().fg(Color::Yellow),
        }
    }

    /// Override the border style (colour, modifiers, etc.).
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Widget for FocusRing {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Block::new()
            .borders(Borders::ALL)
            .border_style(self.style)
            .render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn focus_ring_renders_border() {
        let backend = TestBackend::new(10, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                FocusRing::default().render(frame.area(), frame.buffer_mut());
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        // Top-left corner of the border should be present
        assert_eq!(buf[(0, 0)].symbol(), "┌");
        // Top-right corner
        assert_eq!(buf[(9, 0)].symbol(), "┐");
        // Bottom-left corner
        assert_eq!(buf[(0, 4)].symbol(), "└");
    }

    #[test]
    fn focus_ring_custom_style() {
        let ring = FocusRing::new().style(Style::default().fg(Color::Cyan));
        assert_eq!(ring.style.fg, Some(Color::Cyan));
    }

    #[test]
    fn focus_ring_default_style_is_yellow() {
        let ring = FocusRing::default();
        assert_eq!(ring.style.fg, Some(Color::Yellow));
    }
}
