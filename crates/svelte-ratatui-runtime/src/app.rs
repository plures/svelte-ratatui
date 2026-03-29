//! Application runtime — `SvelteComponent` trait and event loop.
//!
//! Generated components implement [`SvelteComponent`]. The [`run`] function
//! drives the terminal event loop; generated code never touches terminal setup
//! or event polling directly.

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode};
use ratatui::Frame;
use ratatui::layout::Rect;

/// The trait every compiled Svelte component must implement.
///
/// The runtime calls these methods in the event loop to render, handle input,
/// and propagate async state updates. Lifecycle hooks (`on_mount` /
/// `on_destroy`) have empty default implementations and are opt-in.
pub trait SvelteComponent {
    /// Draw the component into `area` of the given `frame`.
    fn render(&self, frame: &mut Frame, area: Rect);

    /// Handle a terminal event.
    ///
    /// Returns `true` if the event was consumed and the component needs to be
    /// redrawn on the next tick.
    fn handle_event(&mut self, event: Event) -> bool;

    /// Poll all pending async channels (PluresDB subscriptions, IPC, …).
    ///
    /// Returns `true` if any state changed and a redraw is needed.
    fn poll_async(&mut self) -> bool;

    /// Called once after the terminal is initialised, before the first render.
    fn on_mount(&mut self) {}

    /// Called once after the event loop exits, before the terminal is restored.
    fn on_destroy(&mut self) {}
}

/// Drive the terminal event loop for `component`.
///
/// Initialises the terminal via [`ratatui::init`], runs the loop at ~60 fps,
/// and restores the terminal via [`ratatui::restore`] on exit.
///
/// The loop exits when the user presses `q`.
///
/// # Errors
///
/// Returns any I/O error produced by the terminal or event subsystem.
pub fn run<C: SvelteComponent>(mut component: C) -> io::Result<()> {
    let mut terminal = ratatui::init();
    component.on_mount();
    let mut needs_redraw = true;

    loop {
        let async_changed = component.poll_async();
        if async_changed || needs_redraw {
            terminal.draw(|frame| {
                let area = frame.area();
                component.render(frame, area);
            })?;
            needs_redraw = false;
        }

        if event::poll(Duration::from_millis(16))? {
            let ev = event::read()?;
            if let Event::Key(key) = &ev
                && key.code == KeyCode::Char('q')
            {
                break;
            }
            if component.handle_event(ev) {
                needs_redraw = true;
            }
        }
    }

    component.on_destroy();
    ratatui::restore();
    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::widgets::Paragraph;

    // ── Mock component used across all lifecycle tests ────────────────────────

    struct MockComponent {
        mount_called: bool,
        destroy_called: bool,
        last_event: Option<KeyCode>,
    }

    impl MockComponent {
        fn new() -> Self {
            Self {
                mount_called: false,
                destroy_called: false,
                last_event: None,
            }
        }
    }

    impl SvelteComponent for MockComponent {
        fn render(&self, frame: &mut Frame, area: Rect) {
            frame.render_widget(Paragraph::new("mock"), area);
        }

        fn handle_event(&mut self, event: Event) -> bool {
            if let Event::Key(key) = event {
                self.last_event = Some(key.code);
                return true;
            }
            false
        }

        fn poll_async(&mut self) -> bool {
            false
        }

        fn on_mount(&mut self) {
            self.mount_called = true;
        }

        fn on_destroy(&mut self) {
            self.destroy_called = true;
        }
    }

    // ── Trait method tests ────────────────────────────────────────────────────

    #[test]
    fn mock_component_render_writes_to_buffer() {
        let component = MockComponent::new();
        let backend = TestBackend::new(20, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                component.render(frame, area);
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        let row: String = (0..20u16)
            .map(|x| buf[(x, 0)].symbol().chars().next().unwrap_or(' '))
            .collect();
        assert!(row.contains("mock"), "render should write component text");
    }

    #[test]
    fn mock_component_handle_event_consumes_key() {
        let mut component = MockComponent::new();
        let ev = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        let consumed = component.handle_event(ev);
        assert!(consumed, "key event should be consumed");
        assert_eq!(component.last_event, Some(KeyCode::Enter));
    }

    #[test]
    fn mock_component_handle_non_key_event_returns_false() {
        let mut component = MockComponent::new();
        let ev = Event::Resize(80, 24);
        let consumed = component.handle_event(ev);
        assert!(!consumed, "resize event should not be consumed");
        assert_eq!(component.last_event, None);
    }

    #[test]
    fn mock_component_poll_async_returns_false() {
        let mut component = MockComponent::new();
        assert!(!component.poll_async());
    }

    #[test]
    fn mock_component_on_mount_sets_flag() {
        let mut component = MockComponent::new();
        assert!(!component.mount_called, "mount_called should start false");
        component.on_mount();
        assert!(component.mount_called, "on_mount should set mount_called");
    }

    #[test]
    fn mock_component_on_destroy_sets_flag() {
        let mut component = MockComponent::new();
        assert!(!component.destroy_called);
        component.on_destroy();
        assert!(
            component.destroy_called,
            "on_destroy should set destroy_called"
        );
    }

    #[test]
    fn default_lifecycle_hooks_are_no_ops() {
        // A minimal component with default lifecycle hooks must compile and
        // call on_mount/on_destroy without panicking.
        struct Minimal;
        impl SvelteComponent for Minimal {
            fn render(&self, _frame: &mut Frame, _area: Rect) {}
            fn handle_event(&mut self, _ev: Event) -> bool {
                false
            }
            fn poll_async(&mut self) -> bool {
                false
            }
        }
        let mut c = Minimal;
        c.on_mount();
        c.on_destroy();
    }
}
