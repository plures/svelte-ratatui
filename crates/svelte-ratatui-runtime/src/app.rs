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
