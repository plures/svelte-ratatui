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

## Crates

| Crate | Description |
|---|---|
| `svelte-ratatui-compiler` | AST→IR→ratatui code generation |
| `svelte-ratatui-runtime` | Event loop, state, rendering bridge |
| `svelte-ratatui-cli` | CLI tool: compile, watch, preview |

## Status

🚧 **Pre-alpha** — Architecture and design phase.

See [design doc](https://github.com/plures/svelte-ratatui/blob/main/docs/DESIGN.md) for full architecture.

## Part of Pares

Part of the [Pares](https://github.com/plures/pares) TUI framework, enabling [RuneBook](https://github.com/plures/runebook) to run in both GUI and terminal mode from the same Svelte source.

## License

AGPL-3.0
