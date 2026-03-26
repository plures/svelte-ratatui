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

/// JavaScript template for dispatching a keyboard event into the webview.
///
/// Call with `format!(DISPATCH_KEY_JS, key, code, shift, ctrl, alt, meta)`.
pub const DISPATCH_KEY_JS: &str = r#"
(function() {
    const target = document.activeElement || document.body;
    target.dispatchEvent(new KeyboardEvent('keydown', {{
        key: '{}',
        code: '{}',
        shiftKey: {},
        ctrlKey: {},
        altKey: {},
        metaKey: {},
        bubbles: true,
        cancelable: true
    }}));
}})()
"#;

/// JavaScript template for dispatching a click event at coordinates.
pub const DISPATCH_CLICK_JS: &str = r#"
(function() {
    const el = document.elementFromPoint({}, {});
    if (el) {
        el.click();
        el.dispatchEvent(new MouseEvent('click', {
            clientX: {},
            clientY: {},
            bubbles: true,
            cancelable: true
        }));
    }
})()
"#;

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
