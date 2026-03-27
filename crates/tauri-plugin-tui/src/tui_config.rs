//! TUI plugin configuration.
//!
//! Provides [`TuiConfig`] with sensible defaults for widget mappings, theme,
//! and runtime behaviour.  Pass a custom config to [`init_with_config`] when
//! the defaults are not appropriate.
//!
//! # Example
//!
//! ```rust,ignore
//! use tauri_plugin_tui::{TuiConfig, TuiTheme, init_with_config};
//!
//! let config = TuiConfig {
//!     theme: TuiTheme {
//!         primary_fg: Some((200, 200, 200)),
//!         primary_bg: None,
//!         accent: Some((100, 149, 237)),  // cornflower blue
//!         border_style: BorderStyle::Rounded,
//!     },
//!     startup_delay_ms: 800,
//!     ..TuiConfig::default()
//! };
//!
//! tauri::Builder::default()
//!     .plugin(init_with_config(config))
//!     .run(tauri::generate_context!())
//!     .expect("error running app");
//! ```

// ── Border style ─────────────────────────────────────────────────────────────

/// Terminal border character set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderStyle {
    /// Plain `+`, `-`, `|` ASCII characters.
    Plain,
    /// Single-line Unicode box-drawing characters (default).
    #[default]
    Single,
    /// Double-line Unicode box-drawing characters.
    Double,
    /// Rounded corner single-line box-drawing characters.
    Rounded,
    /// Thick single-line box-drawing characters.
    Thick,
}

// ── Theme ─────────────────────────────────────────────────────────────────────

/// Terminal color and decoration theme.
///
/// All color values are optional; `None` means "use the terminal default".
/// RGB triplets map to ratatui's `Color::Rgb(r, g, b)`.
#[derive(Debug, Clone, Default)]
pub struct TuiTheme {
    /// Default foreground color for text content.
    pub primary_fg: Option<(u8, u8, u8)>,
    /// Default background color for the root container.
    pub primary_bg: Option<(u8, u8, u8)>,
    /// Accent color used for focused widgets, borders, and headings.
    pub accent: Option<(u8, u8, u8)>,
    /// Character set for block borders.
    pub border_style: BorderStyle,
}

impl TuiTheme {
    /// A dark-mode theme with a blue accent — useful as a starting point.
    ///
    /// ```
    /// use tauri_plugin_tui::TuiTheme;
    /// let t = TuiTheme::dark();
    /// assert_eq!(t.border_style, tauri_plugin_tui::BorderStyle::Rounded);
    /// ```
    pub fn dark() -> Self {
        Self {
            primary_fg: Some((220, 220, 220)),
            primary_bg: Some((20, 20, 30)),
            accent: Some((100, 149, 237)),
            border_style: BorderStyle::Rounded,
        }
    }

    /// A light-mode theme.
    ///
    /// ```
    /// use tauri_plugin_tui::TuiTheme;
    /// let t = TuiTheme::light();
    /// assert_eq!(t.border_style, tauri_plugin_tui::BorderStyle::Single);
    /// ```
    pub fn light() -> Self {
        Self {
            primary_fg: Some((20, 20, 20)),
            primary_bg: Some((245, 245, 245)),
            accent: Some((0, 100, 200)),
            border_style: BorderStyle::Single,
        }
    }
}

// ── Widget mapping overrides ──────────────────────────────────────────────────

/// Per-HTML-element widget mapping overrides.
///
/// Each field, if `Some`, replaces the default HTML-tag → ratatui-widget
/// mapping for that element class.  Currently only `heading_bold` is
/// configurable; more will be added as the compiler matures.
#[derive(Debug, Clone, Default)]
pub struct WidgetOverrides {
    /// When `true`, headings (`<h1>`–`<h6>`) are rendered with the BOLD
    /// modifier in addition to the accent color.  Default: `true`.
    pub heading_bold: Option<bool>,
    /// When `true`, `<code>` / `<pre>` blocks receive a contrasting
    /// background.  Default: `true`.
    pub code_block_bg: Option<bool>,
}

// ── Main config ───────────────────────────────────────────────────────────────

/// Top-level configuration for the TUI plugin.
///
/// Construct with [`TuiConfig::default()`] and override individual fields as
/// needed. The default uses sensible terminal defaults.
#[derive(Debug, Clone)]
pub struct TuiConfig {
pub struct TuiConfig {
    /// Colour and decoration theme.
    pub theme: TuiTheme,
    /// Per-element widget mapping overrides.
    pub widget_overrides: WidgetOverrides,
    /// Milliseconds to wait after Tauri starts before injecting TUI scripts.
    /// Lower values speed up startup but may cause a blank first frame if
    /// Svelte has not finished mounting.  Default: `800`.
    pub startup_delay_ms: u64,
    /// Target frame rate.  The event loop sleeps for `1000 / fps` ms between
    /// frames.  Default: `60`.
    pub target_fps: u32,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            theme: TuiTheme::default(),
            widget_overrides: WidgetOverrides {
                heading_bold: Some(true),
                code_block_bg: Some(true),
            },
            startup_delay_ms: 800,
            target_fps: 60,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_sensible_values() {
        let cfg = TuiConfig::default();
        assert_eq!(cfg.startup_delay_ms, 800);
        assert_eq!(cfg.target_fps, 60);
        assert_eq!(cfg.widget_overrides.heading_bold, Some(true));
        assert_eq!(cfg.widget_overrides.code_block_bg, Some(true));
    }

    #[test]
    fn dark_theme_has_rounded_borders() {
        assert_eq!(TuiTheme::dark().border_style, BorderStyle::Rounded);
    }

    #[test]
    fn light_theme_has_single_borders() {
        assert_eq!(TuiTheme::light().border_style, BorderStyle::Single);
    }
}
