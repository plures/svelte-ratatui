//! TUI demo — counter + list component rendered in terminal mode.
//!
//! This example demonstrates a Svelte-like component (`TuiDemo`) that:
//! - Maintains reactive state (counter, list of items)
//! - Renders itself as ratatui widgets via the [`SvelteComponent`] trait
//! - Handles keyboard input (arrow keys, Enter, q)
//!
//! In a real svelte-tauri-template project this component would be written in
//! Svelte and compiled by svelte-ratatui-compiler.  Here we implement the
//! generated trait output by hand to illustrate what the compiler produces.
//!
//! # Running
//!
//! ```sh
//! cargo run -p tui-demo
//! ```
//!
//! Keys:
//!   `+` / `-`  — increment / decrement counter
//!   `↑` / `↓`  — move list selection
//!   `Enter`    — add counter value to list
//!   `q`        — quit

use crossterm::event::{Event, KeyCode};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use svelte_ratatui_runtime::{SvelteComponent, run};
// ── Component state ───────────────────────────────────────────────────────────

/// The TUI demo component — mirrors a `TuiDemo.svelte` in a template project.
///
/// State:
/// - `counter`: an integer that can be incremented or decremented
/// - `items`: a list of recorded counter values
/// - `list_state`: tracks the highlighted list row
struct TuiDemo {
    counter: i32,
    items: Vec<String>,
    list_state: ListState,
}

impl TuiDemo {
    fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(None);
        Self {
            counter: 0,
            items: vec![
                "Press + / - to change the counter".to_string(),
                "Press Enter to record the counter value".to_string(),
                "Press ↑ / ↓ to scroll this list".to_string(),
            ],
            list_state,
        }
    }

    fn select_next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => (i + 1).min(self.items.len().saturating_sub(1)),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn select_prev(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.list_state.select(Some(i));
    }
}

// ── SvelteComponent impl ──────────────────────────────────────────────────────

impl SvelteComponent for TuiDemo {
    fn render(&self, frame: &mut Frame, area: Rect) {
        // Split the terminal into two vertical panes:
        //   top 3 rows  → counter widget
        //   remaining   → list widget
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // ── Counter pane ──────────────────────────────────────────────────────
        let counter_block = Block::default()
            .title(" Counter ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let counter_text = Paragraph::new(Line::from(vec![
            Span::raw("Value: "),
            Span::styled(
                self.counter.to_string(),
                Style::default()
                    .fg(if self.counter >= 0 {
                        Color::Green
                    } else {
                        Color::Red
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   [+/-] change  [Enter] record  [q] quit"),
        ]))
        .block(counter_block);

        frame.render_widget(counter_text, chunks[0]);

        // ── List pane ─────────────────────────────────────────────────────────
        let list_block = Block::default()
            .title(" Items ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let selected = self.list_state.selected();

        let list_items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(idx, s)| {
                let content = s.as_str();
                if Some(idx) == selected {
                    ListItem::new(content).style(
                        Style::default()
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    ListItem::new(content)
                }
            })
            .collect();

        let list = List::new(list_items).block(list_block);

        frame.render_widget(list, chunks[1]);
    }

    fn handle_event(&mut self, event: Event) -> bool {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char('+') => {
                    self.counter += 1;
                    return true;
                }
                KeyCode::Char('-') => {
                    self.counter -= 1;
                    return true;
                }
                KeyCode::Enter => {
                    self.items.push(format!("Recorded: {}", self.counter));
                    return true;
                }
                KeyCode::Down => {
                    self.select_next();
                    return true;
                }
                KeyCode::Up => {
                    self.select_prev();
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    fn poll_async(&mut self) -> bool {
        false
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> std::io::Result<()> {
    run(TuiDemo::new())
}
