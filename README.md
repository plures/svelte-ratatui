# svelte-ratatui

> Render Svelte components natively in terminal UIs via ratatui.

A **build-time compiler** that transforms Svelte components into ratatui widget trees, enabling the same UI code to run in both Tauri (GUI) and terminal (TUI) environments.

## Why?

Terminal browsers (w3m, lynx, carbonyl) already solve HTML→terminal rendering. We apply the same principle at compile time: Svelte AST → intermediate representation → ratatui widgets. No JavaScript runtime needed in the terminal.

## Architecture

```
Svelte Source (.svelte)
    ↓ svelte/compiler
Svelte AST
    ↓ svelte-ratatui-compiler
Terminal IR (element → widget mapping)
    ↓ codegen
Rust source (ratatui widgets + state)
    ↓ cargo build
Native terminal binary
```

At runtime, the **adapter path** takes over for Tauri apps:

```
Tauri webview (headless, --tui flag)
    ↓ DOM serialisation
HTML snapshot
    ↓ svelte-ratatui-adapter
IR tree
    ↓ svelte-ratatui-compiler (mapping)
ratatui widgets → terminal
```

## Crates

| Crate | Description |
|---|---|
| `svelte-ratatui-compiler` | AST→IR→ratatui code generation |
| `svelte-ratatui-runtime` | Event loop, state, rendering bridge |
| `svelte-ratatui-adapter` | Runtime bridge: headless Tauri webview → ratatui terminal |
| `tauri-plugin-tui` | Tauri 2 plugin — wire TUI mode into any Svelte-Tauri app |
| `svelte-ratatui-cli` | CLI: compile, watch, preview, scaffold |

## TUI Mode

### Adding TUI support to a Tauri app

1. Add `tauri-plugin-tui` to your `src-tauri/Cargo.toml`:

   ```toml
   [dependencies]
   tauri-plugin-tui = { git = "https://github.com/plures/svelte-ratatui" }
   ```

2. Register the plugin in `src-tauri/src/main.rs`, gated behind the `--tui` flag:

   ```rust
   fn main() {
       let args: Vec<String> = std::env::args().collect();
       let mut builder = tauri::Builder::default();

       if args.contains(&"--tui".to_string()) {
           builder = builder.plugin(tauri_plugin_tui::init());
       }

       builder
           .run(tauri::generate_context!())
           .expect("error running app");
   }
   ```

3. Run in TUI mode:

   ```sh
   cargo tauri dev -- -- --tui
   # or after build:
   ./my-app --tui
   ```

### Custom frame rate

```rust
use tauri_plugin_tui::{TuiConfig, init_with_config};

tauri::Builder::default()
    .plugin(init_with_config(TuiConfig {
        target_fps: 30,
        ..TuiConfig::default()
    }))
    .run(tauri::generate_context!())
    .expect("error running app");
```

> **Note:** `TuiConfig` also accepts `theme` (`TuiTheme`) and `widget_overrides` fields — these are stored and validated but theme application to ratatui widgets is not yet implemented. They are available for forward-compatible configuration today.

### Svelte component conventions

Svelte components that need to behave differently in TUI mode can check the
`window.__TUI_MODE__` flag or react to the `tui-mode` CSS class on
`<html>`:

```svelte
<script lang="ts">
  const isTui = typeof window !== 'undefined' && window.__TUI_MODE__;
</script>

{#if isTui}
  <!-- terminal-optimised markup -->
  <div class="counter-tui">Count: {count}</div>
{:else}
  <!-- full GUI component -->
  <Button on:click={() => count++}>Increment</Button>
{/if}
```

### Running the demo

```sh
cargo run -p tui-demo
```

The demo shows a counter and list that respond to keyboard input entirely
in the terminal using the [`SvelteComponent`] trait from
`svelte-ratatui-runtime`.

### CLI scaffolding

```sh
# Scaffold TUI integration into an existing svelte-tauri-template project:
svelte-ratatui scaffold --with-tui ./my-project

# Opt out of TUI scaffolding:
svelte-ratatui scaffold --no-tui ./my-project
```

## Status

🚧 **Pre-alpha** — Architecture and design phase.

See [design doc](https://github.com/plures/svelte-ratatui/blob/main/docs/DESIGN.md) for full architecture.

## Part of Pares

Part of the [Pares](https://github.com/plures/pares) TUI framework, enabling [RuneBook](https://github.com/plures/runebook) to run in both GUI and terminal mode from the same Svelte source.

## License

AGPL-3.0
