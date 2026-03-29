//! DOM snapshot reader — JavaScript that runs in the Tauri webview to
//! serialize the current DOM state into a compact JSON representation.
//!
//! The Rust adapter calls `webview.eval(DOM_SNAPSHOT_JS)` to get a
//! [`DomSnapshot`] which is then converted to [`IrNode`] trees.

use serde::{Deserialize, Serialize};

/// A serialized DOM snapshot returned by the webview JS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomSnapshot {
    /// The serialized HTML of the app root element.
    pub html: String,
    /// Viewport width in CSS pixels (for layout hints).
    pub width: u32,
    /// Viewport height in CSS pixels.
    pub height: u32,
    /// The currently focused element's selector, if any.
    pub focused: Option<String>,
}

/// JavaScript injected into the webview to capture a DOM snapshot.
///
/// This script serializes the app root's innerHTML plus viewport dimensions
/// into a JSON string. It's designed to be called via `webview.eval()`.
///
/// Returns a JSON string that deserializes into [`DomSnapshot`].
pub const DOM_SNAPSHOT_JS: &str = r#"
(function() {
    const root = document.querySelector('#app') || document.body;
    const focused = document.activeElement;
    let focusedSelector = null;
    if (focused && focused !== document.body) {
        // Build a simple selector for the focused element
        if (focused.id) {
            focusedSelector = '#' + focused.id;
        } else if (focused.getAttribute('data-tui-id')) {
            focusedSelector = '[data-tui-id="' + focused.getAttribute('data-tui-id') + '"]';
        } else {
            // Use nth-child as fallback
            const parent = focused.parentElement;
            if (parent) {
                const idx = Array.from(parent.children).indexOf(focused);
                focusedSelector = focused.tagName.toLowerCase() + ':nth-child(' + (idx + 1) + ')';
            }
        }
    }
    return JSON.stringify({
        html: root.innerHTML,
        width: window.innerWidth,
        height: window.innerHeight,
        focused: focusedSelector
    });
})()
"#;

/// Escape a Rust `&str` for safe embedding in a single-quoted JavaScript string.
///
/// Escapes backslashes, single quotes, and all characters that would break a
/// JS string literal or enable injection, including ASCII control characters
/// and the Unicode line/paragraph separator code points U+2028 and U+2029
/// (which are line terminators in JS even inside string literals).
fn js_single_quoted(s: &str) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x08' => out.push_str("\\b"), // backspace
            '\x0C' => out.push_str("\\f"), // form feed
            // NUL: use \\x00 rather than \\0 to avoid octal mis-interpretation
            // if the escaped string is followed by a digit (e.g. \01 = octal).
            '\x00' => out.push_str("\\x00"),
            // U+2028 LINE SEPARATOR and U+2029 PARAGRAPH SEPARATOR are
            // treated as line terminators inside JS string literals.
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            // Remaining ASCII control characters (U+0001–U+001F, U+007F)
            c if (c as u32) < 0x20 || c == '\x7F' => {
                write!(out, "\\u{:04X}", c as u32).unwrap();
            }
            c => out.push(c),
        }
    }
    out
}

/// Build JavaScript for dispatching a keyboard event into the webview.
///
/// Dispatches both a `keydown` and a matching `keyup` event to the active
/// element, matching the browser's natural key-press sequence.
///
/// Returns a snippet suitable for passing to `webview.eval(...)`.
pub fn build_dispatch_key_js(
    key: &str,
    code: &str,
    shift: bool,
    ctrl: bool,
    alt: bool,
    meta: bool,
) -> String {
    let key = js_single_quoted(key);
    let code = js_single_quoted(code);
    format!(
        r#"(function(){{
    const target = document.activeElement || document.body;
    const init = {{
        key: '{key}',
        code: '{code}',
        shiftKey: {shift},
        ctrlKey: {ctrl},
        altKey: {alt},
        metaKey: {meta},
        bubbles: true,
        cancelable: true
    }};
    target.dispatchEvent(new KeyboardEvent('keydown', init));
    target.dispatchEvent(new KeyboardEvent('keyup', init));
}})()"#
    )
}

/// Build JavaScript for dispatching a click event at viewport coordinates.
///
/// Triggers `el.click()` (synthetic click that fires event handlers) and then
/// focuses the element so subsequent keyboard events are routed correctly.
///
/// `x` and `y` are CSS pixel coordinates in the client viewport.
pub fn build_dispatch_click_js(x: f64, y: f64) -> String {
    format!(
        r#"(function(){{
    const el = document.elementFromPoint({x}, {y});
    if (el) {{
        el.click();
        el.focus();
    }}
}})()"#
    )
}

/// Build JavaScript for dispatching a `mousedown` event at viewport coordinates.
///
/// Uses the standard `MouseEvent` constructor so listeners registered with
/// `addEventListener('mousedown', …)` and Svelte's `on:mousedown` are triggered.
///
/// `x` and `y` are CSS pixel coordinates. `button` is the standard DOM button
/// index (0 = left, 1 = middle, 2 = right).
pub fn build_dispatch_mousedown_js(x: f64, y: f64, button: u8) -> String {
    // `buttons` bitmask: left=1, right=2, middle=4
    let buttons: u8 = match button {
        0 => 1,
        2 => 2,
        1 => 4,
        _ => 0,
    };
    format!(
        r#"(function(){{
    const el = document.elementFromPoint({x}, {y});
    if (el) {{
        el.dispatchEvent(new MouseEvent('mousedown', {{
            clientX: {x},
            clientY: {y},
            button: {button},
            buttons: {buttons},
            bubbles: true,
            cancelable: true
        }}));
        el.focus();
    }}
}})()"#
    )
}

/// Build JavaScript for dispatching a `mouseup` and `click` event sequence at
/// viewport coordinates — matching the browser's natural button-release order.
///
/// `x` and `y` are CSS pixel coordinates. `button` is the standard DOM button
/// index (0 = left, 1 = middle, 2 = right).
pub fn build_dispatch_mouseup_js(x: f64, y: f64, button: u8) -> String {
    format!(
        r#"(function(){{
    const el = document.elementFromPoint({x}, {y});
    if (el) {{
        el.dispatchEvent(new MouseEvent('mouseup', {{
            clientX: {x},
            clientY: {y},
            button: {button},
            bubbles: true,
            cancelable: true
        }}));
        el.dispatchEvent(new MouseEvent('click', {{
            clientX: {x},
            clientY: {y},
            button: {button},
            bubbles: true,
            cancelable: true
        }}));
    }}
}})()"#
    )
}

/// JavaScript for dispatching a focus event to the next focusable element.
pub const FOCUS_NEXT_JS: &str = r#"
(function() {
    const focusable = Array.from(document.querySelectorAll(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    )).filter(el => !el.disabled && el.offsetParent !== null);
    const current = document.activeElement;
    const idx = focusable.indexOf(current);
    const next = focusable[(idx + 1) % focusable.length];
    if (next) next.focus();
})()
"#;

/// JavaScript for dispatching a focus event to the previous focusable element.
pub const FOCUS_PREV_JS: &str = r#"
(function() {
    const focusable = Array.from(document.querySelectorAll(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    )).filter(el => !el.disabled && el.offsetParent !== null);
    const current = document.activeElement;
    const idx = focusable.indexOf(current);
    const prev = focusable[(idx - 1 + focusable.length) % focusable.length];
    if (prev) prev.focus();
})()
"#;
