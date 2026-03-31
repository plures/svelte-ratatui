//! Terminal input → DOM event translation.
//!
//! Maps crossterm keyboard/mouse events to JavaScript event dispatch calls
//! that can be `eval()`'d in the Tauri webview.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

use crate::dom_reader;

/// Translate a crossterm terminal event into a JavaScript string to eval
/// in the webview. Returns `None` for events that don't map to DOM actions.
pub fn event_to_js(event: &Event) -> Option<String> {
    match event {
        Event::Key(key) => key_to_js(key),
        Event::Mouse(mouse) => mouse_to_js(mouse),
        _ => None,
    }
}

fn key_to_js(key: &KeyEvent) -> Option<String> {
    // Tab → focus navigation
    if key.code == KeyCode::Tab {
        return if key.modifiers.contains(KeyModifiers::SHIFT) {
            Some(dom_reader::FOCUS_PREV_JS.to_string())
        } else {
            Some(dom_reader::FOCUS_NEXT_JS.to_string())
        };
    }

    let (js_key, js_code) = match key.code {
        KeyCode::Char(c) => (c.to_string(), format!("Key{}", c.to_uppercase())),
        KeyCode::Enter => ("Enter".into(), "Enter".into()),
        KeyCode::Esc => ("Escape".into(), "Escape".into()),
        KeyCode::Backspace => ("Backspace".into(), "Backspace".into()),
        KeyCode::Delete => ("Delete".into(), "Delete".into()),
        KeyCode::Left => ("ArrowLeft".into(), "ArrowLeft".into()),
        KeyCode::Right => ("ArrowRight".into(), "ArrowRight".into()),
        KeyCode::Up => ("ArrowUp".into(), "ArrowUp".into()),
        KeyCode::Down => ("ArrowDown".into(), "ArrowDown".into()),
        KeyCode::Home => ("Home".into(), "Home".into()),
        KeyCode::End => ("End".into(), "End".into()),
        KeyCode::PageUp => ("PageUp".into(), "PageUp".into()),
        KeyCode::PageDown => ("PageDown".into(), "PageDown".into()),
        KeyCode::F(n) => (format!("F{n}"), format!("F{n}")),
        _ => return None,
    };

    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);

    Some(dom_reader::build_dispatch_key_js(
        &js_key, &js_code, shift, ctrl, alt, false,
    ))
}

fn mouse_to_js(mouse: &MouseEvent) -> Option<String> {
    // Convert terminal coordinates to approximate CSS pixel coordinates.
    // Assumes ~8px per character width, ~16px per row.
    let css_x = mouse.column as f64 * 8.0;
    let css_y = mouse.row as f64 * 16.0;

    match mouse.kind {
        MouseEventKind::Down(button) => {
            let btn = dom_reader::mouse_button_index(button);
            Some(dom_reader::build_dispatch_mousedown_js(css_x, css_y, btn))
        }
        MouseEventKind::Up(button) => {
            let btn = dom_reader::mouse_button_index(button);
            Some(dom_reader::build_dispatch_mouseup_js(css_x, css_y, btn))
        }
        MouseEventKind::ScrollUp => Some(build_wheel_js(css_x, css_y, 0.0, -120.0)),
        MouseEventKind::ScrollDown => Some(build_wheel_js(css_x, css_y, 0.0, 120.0)),
        MouseEventKind::ScrollLeft => Some(build_wheel_js(css_x, css_y, -120.0, 0.0)),
        MouseEventKind::ScrollRight => Some(build_wheel_js(css_x, css_y, 120.0, 0.0)),
        _ => None,
    }
}

/// Build a `WheelEvent` dispatch snippet at the given coordinates.
///
/// Sets `cancelable: true` so that `event.preventDefault()` works in handlers,
/// and includes `clientX`/`clientY` for completeness with other mouse events.
fn build_wheel_js(x: f64, y: f64, delta_x: f64, delta_y: f64) -> String {
    format!(
        r#"(function(){{
    const el = document.elementFromPoint({x}, {y}) || document.activeElement || document.body;
    el.dispatchEvent(new WheelEvent('wheel', {{
        clientX: {x},
        clientY: {y},
        deltaX: {delta_x},
        deltaY: {delta_y},
        bubbles: true,
        cancelable: true
    }}));
}})()"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn enter_key_maps_to_keydown() {
        let ev = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        let js = event_to_js(&ev).unwrap();
        assert!(js.contains("'Enter'"));
        assert!(js.contains("keydown"));
    }

    #[test]
    fn tab_maps_to_focus_next() {
        let ev = Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        let js = event_to_js(&ev).unwrap();
        assert!(js.contains("focusable"));
        assert!(js.contains("next"));
    }

    #[test]
    fn shift_tab_maps_to_focus_prev() {
        let ev = Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT));
        let js = event_to_js(&ev).unwrap();
        assert!(js.contains("prev"));
    }

    #[test]
    fn char_key_includes_modifiers() {
        let ev = Event::Key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        let js = event_to_js(&ev).unwrap();
        assert!(js.contains("'s'"));
        assert!(js.contains("ctrlKey: true"));
    }
}
