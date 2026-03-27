//! Core plugin implementation.
//!
//! Uses Tauri's event system for bidirectional communication:
//! - Rust emits `tui://request-snapshot` → JS listener captures DOM → emits `tui://snapshot`
//! - Rust listens for `tui://snapshot` events containing the serialized DOM

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Emitter, Listener, Manager, Runtime, WebviewWindow};

use svelte_ratatui_adapter::dom_reader::DomSnapshot;
use svelte_ratatui_adapter::html_parser::parse_html;
use svelte_ratatui_adapter::input::event_to_js;
use svelte_ratatui_compiler::mapping::render_ir;

/// Initialize the TUI plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("tui")
        .setup(|app, _api| {
            let app_handle = app.clone();

            std::thread::spawn(move || {
                // Give Svelte time to mount
                std::thread::sleep(Duration::from_millis(800));

                if let Some(window) = app_handle.get_webview_window("main") {
                    // Hide the GUI window
                    let _ = window.hide();

                    // Inject TUI mode flag and snapshot listener into the webview
                    if let Err(e) = window.eval(TUI_INIT_JS) {
                        log::error!("Failed to inject TUI initialization script into webview: {e}");
                        let _ = app_handle.exit(1);
                        return;
                    }

                    // Small delay for JS to register the listener
                    std::thread::sleep(Duration::from_millis(100));

                    // Run the terminal render loop
                    if let Err(e) = run_tui_loop(&window) {
                        log::error!("TUI loop exited with error: {e}");
                    }

                    let _ = app_handle.exit(0);
                }
            });

            Ok(())
        })
        .build()
}

/// JavaScript injected into the webview to:
/// 1. Set TUI mode flag
/// 2. Listen for snapshot requests from Rust
/// 3. Respond with DOM snapshots via Tauri events
const TUI_INIT_JS: &str = r#"
(function() {
    // Signal TUI mode to the Svelte app
    window.__TUI_MODE__ = true;
    document.documentElement.classList.add('tui-mode');

    // Listen for snapshot requests from the Rust TUI adapter
    const { listen, emit } = window.__TAURI__.event;

    listen('tui://request-snapshot', () => {
        const root = document.querySelector('#app') || document.body;
        const focused = document.activeElement;
        let focusedSelector = null;
        if (focused && focused !== document.body) {
            if (focused.id) {
                focusedSelector = '#' + focused.id;
            } else if (focused.getAttribute('data-tui-id')) {
                focusedSelector = '[data-tui-id="' + focused.getAttribute('data-tui-id') + '"]';
            }
        }
        emit('tui://snapshot', {
            html: root.innerHTML,
            width: window.innerWidth,
            height: window.innerHeight,
            focused: focusedSelector
        });
    });

    console.log('[tui-plugin] Snapshot listener registered');
})();
"#;

/// RAII guard that calls `ratatui::restore()` when dropped.
///
/// This guarantees the terminal is restored to its original state even when
/// `run_tui_loop` exits early via a `?` operator or a panic.
struct RestoreOnDrop;

impl Drop for RestoreOnDrop {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

/// Target frame duration (~60 fps).
const FRAME_DURATION: Duration = Duration::from_millis(16);

/// The main terminal render loop.
fn run_tui_loop<R: Runtime>(window: &WebviewWindow<R>) -> std::io::Result<()> {
    let mut terminal = ratatui::init();

    // Ensures ratatui::restore() is called on every exit path, including
    // early returns from `?` and panics.
    let _restore = RestoreOnDrop;

    // Shared snapshot buffer — updated by event listener, read by render loop
    let snapshot_buf: Arc<Mutex<Option<DomSnapshot>>> = Arc::new(Mutex::new(None));
    let buf_clone = snapshot_buf.clone();

    // Listen for snapshot events from the webview
    let _listener = window.listen("tui://snapshot", move |event| {
        if let Ok(payload) = serde_json::from_str::<DomSnapshot>(event.payload()) {
            if let Ok(mut buf) = buf_clone.lock() {
                *buf = Some(payload);
            }
        }
    });

    let running = Arc::new(AtomicBool::new(true));
    let mut last_html = String::new();
    let mut frame_count: u64 = 0;

    while running.load(Ordering::Relaxed) {
        let frame_start = Instant::now();

        // Request a DOM snapshot once per frame.
        let _ = window.emit("tui://request-snapshot", ());

        // Poll for input for the remainder of the frame budget (~16 ms).
        // This provides frame pacing and gives the webview time to respond
        // to the snapshot request before we read the buffer below.
        let poll_timeout = FRAME_DURATION.saturating_sub(frame_start.elapsed());
        if event::poll(poll_timeout)? {
            let ev = event::read()?;

            // Ctrl+C exits
            if let Event::Key(key) = &ev {
                if key.code == KeyCode::Char('c')
                    && key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL)
                {
                    running.store(false, Ordering::Relaxed);
                    break;
                }
            }

            // Forward input to webview
            if let Some(js) = event_to_js(&ev) {
                let _ = window.eval(&js);
            }
        }

        // Process any snapshot that arrived during this frame
        let new_html = {
            let mut buf = snapshot_buf.lock().unwrap_or_else(|e| e.into_inner());
            buf.take().map(|s| s.html)
        };

        // Re-render if DOM changed
        if let Some(html) = new_html {
            if html != last_html {
                last_html = html;
                let ir = parse_html(&last_html);
                terminal.draw(|frame| {
                    render_ir(frame, frame.area(), &ir);
                })?;
            }
        } else if frame_count == 0 {
            // First frame: show loading message
            terminal.draw(|frame| {
                let area = frame.area();
                let msg = ratatui::widgets::Paragraph::new("Loading...");
                frame.render_widget(msg, area);
            })?;
        }

        frame_count += 1;
    }

    Ok(())
    // `_restore` drops here (or on any `?` return above), calling ratatui::restore()
}
