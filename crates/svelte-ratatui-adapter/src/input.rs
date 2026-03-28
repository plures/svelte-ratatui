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
    match mouse.kind {
        MouseEventKind::Down(_) => {
            let x = mouse.column;
            let y = mouse.row;
            // Convert terminal coordinates to approximate CSS pixel coordinates.
            // Assumes ~8px per character width, ~16px per row.
            let css_x = x as f64 * 8.0;
            let css_y = y as f64 * 16.0;
            Some(dom_reader::build_dispatch_click_js(css_x, css_y))
        }
        MouseEventKind::ScrollUp => Some(
            r#"(function(){
    const target = document.activeElement || document.body;
    target.dispatchEvent(new WheelEvent('wheel', { deltaY: -120, bubbles: true }));
})()"#
                .to_string(),
        ),
        MouseEventKind::ScrollDown => Some(
            r#"(function(){
    const target = document.activeElement || document.body;
    target.dispatchEvent(new WheelEvent('wheel', { deltaY: 120, bubbles: true }));
})()"#
                .to_string(),
        ),
        _ => None,
    }
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
