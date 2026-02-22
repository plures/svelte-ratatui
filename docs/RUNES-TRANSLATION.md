# Runes Translation Specification

**Version:** 1.0  
**Status:** Normative  
**Date:** 2026-02-22

This document defines the formal contract for translating Svelte 5 Runes into
Rust state management and ratatui's immediate-mode rendering loop. It is the
spec that the svelte-ratatui compiler implements.

---

## The Core Insight: Immediate Mode Works In Our Favor

Svelte Runes are fine-grained reactive signals that surgically update
individual DOM nodes. ratatui is immediate mode — every frame, the entire UI
redraws from current state.

This mismatch is actually an **advantage**: the compiler does not need to emit
a reactive dependency graph. Any state mutation simply sets a dirty flag and
the runtime redraws on the next tick:

```
state mutation → set dirty flag → event loop redraws next tick
```

---

## Rune → Rust Mapping

### `$state(T)` → struct field + setter method

```svelte
let count = $state(0);
```

```rust
struct ComponentState {
    count: i32,
}

impl ComponentState {
    fn set_count(&mut self, v: i32) {
        self.count = v;
        // compiler-injected effect calls go here (see $effect below)
    }
}
```

**Rules:**
- Primitive types (`bool`, `i32`, `f64`, `String`) map directly.
- Arrays / `Vec<T>` are allowed and map to `Vec<T>`.
- Plain structs (no methods, all public fields) are allowed.
- Types that require runtime reflection or dynamic dispatch are **not** allowed
  (see `ALLOWED-DIALECT.md`).

---

### `$derived(expr)` → computed method

```svelte
let doubled = $derived(count * 2);
```

```rust
impl ComponentState {
    fn doubled(&self) -> i32 {
        self.count * 2   // recomputed on access — cheap for simple exprs
    }
}
```

For expensive derivations the compiler emits a cached field + dirty flag:

```rust
struct ComponentState {
    count: i32,
    _doubled_cache: i32,
    _doubled_dirty: bool,
}

impl ComponentState {
    fn doubled(&mut self) -> i32 {
        if self._doubled_dirty {
            self._doubled_cache = self.count * 2;
            self._doubled_dirty = false;
        }
        self._doubled_cache
    }

    fn set_count(&mut self, v: i32) {
        self.count = v;
        self._doubled_dirty = true;
    }
}
```

**Rules:**
- Derived expressions must be pure (no side effects, no `await`).
- Self-referential derivations are a compile error.

---

### `$props()` → struct constructor arguments

```svelte
let { label, onClick } = $props();
```

```rust
struct ButtonComponent {
    label: String,
    on_click: Box<dyn Fn()>,
}

impl ButtonComponent {
    pub fn new(label: String, on_click: Box<dyn Fn()>) -> Self {
        Self { label, on_click }
    }
}
```

---

### `$bindable(T)` → (value, setter) pair

Two-way binding: the parent passes the current value *and* a setter callback.

```svelte
let value = $bindable("");
```

```rust
struct InputComponent {
    value: String,
    on_change: Box<dyn Fn(String)>,
}
```

When the input changes internally the component calls `(self.on_change)(new_value)`
to propagate the update back to the parent.

---

### `$effect(fn)` — two cases

#### Sync effect (focus, animation, local side effect)

The compiler injects a call into every setter method that touches a dependency
of the effect:

```rust
fn set_count(&mut self, v: i32) {
    self.count = v;
    self.effect_on_count_change();  // compiler-injected
}

fn effect_on_count_change(&mut self) {
    // body of $effect
}
```

#### Async effect — PluresDB subscription or IPC (sanctioned pattern)

```svelte
$effect(() => {
  const unsub = db.subscribe("messages", msgs => { messages = msgs; });
  return unsub;
});
```

Generated Rust uses a `tokio` task and an `mpsc` channel, polled non-blockingly
in the `poll_async` method:

```rust
// On component struct:
rx_messages: tokio::sync::mpsc::Receiver<Vec<Message>>,

// Spawned in on_mount():
let (tx, rx) = tokio::sync::mpsc::channel::<Vec<Message>>(64);
self.rx_messages = rx;
tokio::spawn(async move {
    db.subscribe("messages", move |msgs| { tx.send(msgs).ok(); }).await;
});

// poll_async() — called every event-loop tick:
fn poll_async(&mut self) -> bool {
    if let Ok(msgs) = self.rx_messages.try_recv() {
        self.state.messages = msgs;
        return true;   // redraw needed
    }
    false
}
```

**Arbitrary `async`/`await` that does not follow the `db.subscribe()` or
`ipc()` pattern is a compile error (E001).**

---

## The `SvelteComponent` Trait (runtime crate)

Generated components implement this trait. The runtime drives the event loop —
generated code never touches terminal setup or event polling directly.

```rust
// svelte-ratatui-runtime
pub trait SvelteComponent {
    fn render(&self, frame: &mut Frame, area: Rect);
    fn handle_event(&mut self, event: Event) -> bool;  // true = consumed
    fn poll_async(&mut self) -> bool;                  // true = state changed
    fn on_mount(&mut self) {}    // lifecycle — optional
    fn on_destroy(&mut self) {}  // lifecycle — cleanup tokio tasks
}
```

### Runtime event loop

```rust
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
```

---

## Lifecycle Summary

| Svelte hook | Rust equivalent |
|-------------|-----------------|
| Component creation | `ComponentState::new()` / `ComponentName::new()` |
| `onMount` | `SvelteComponent::on_mount()` |
| `onDestroy` | `SvelteComponent::on_destroy()` |
| Reactive update | setter method → dirty flag → next-tick redraw |
| `$effect` cleanup | return value from effect body → called in `on_destroy()` |

---

## Complete Example

### Svelte source

```svelte
<script>
  let count = $state(0);
  let doubled = $derived(count * 2);

  function increment() { count += 1; }
</script>

<p>Count: {count}, doubled: {doubled}</p>
<button onclick={increment}>+1</button>
```

### Generated Rust

```rust
use ratatui::{prelude::*, widgets::*};
use crossterm::event::{Event, KeyCode};
use svelte_ratatui_runtime::SvelteComponent;

pub struct CounterState {
    count: i32,
}

impl CounterState {
    pub fn new() -> Self {
        Self { count: 0 }
    }

    pub fn doubled(&self) -> i32 {
        self.count * 2
    }

    pub fn set_count(&mut self, v: i32) {
        self.count = v;
    }

    pub fn increment(&mut self) {
        self.set_count(self.count + 1);
    }
}

pub struct CounterComponent {
    state: CounterState,
}

impl CounterComponent {
    pub fn new() -> Self {
        Self { state: CounterState::new() }
    }
}

impl SvelteComponent for CounterComponent {
    fn render(&self, frame: &mut Frame, area: Rect) {
        let text = format!(
            "Count: {}, doubled: {}",
            self.state.count,
            self.state.doubled(),
        );
        frame.render_widget(Paragraph::new(text), area);
    }

    fn handle_event(&mut self, event: Event) -> bool {
        if let Event::Key(k) = event {
            if k.code == KeyCode::Enter {
                self.state.increment();
                return true;
            }
        }
        false
    }

    fn poll_async(&mut self) -> bool {
        false
    }
}
```

---

*See also: [`ALLOWED-DIALECT.md`](ALLOWED-DIALECT.md) for the complete
constraint table and all E00x error codes.*
