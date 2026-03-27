# svelte-ratatui Roadmap

## Role in Plures Ecosystem
svelte-ratatui provides the terminal rendering layer for the Plures UI stack. It enables Svelte-based apps (design-dojo, Tauri frontends) to run in terminal environments, unlocking “design once, run everywhere” across GUI and TUI.

## Current State
The repo contains compiler, runtime, CLI, adapter, and Tauri plugin crates. A pre-alpha architecture is documented, and the runtime adapter path is taking shape (HTML→IR→ratatui rendering). The compiler pipeline is still early, and widget coverage, event handling, and layout parity need expansion.

## Milestones

### Near-term (Q2 2026)
- Expand widget mapping coverage for common design-dojo components.
- Stabilize HTML→IR parsing and style resolution for terminal rendering.
- Implement robust input/event translation (keyboard, mouse, focus).
- Add end-to-end demos with netops-toolkit-app TUI mode.
- Improve CLI tooling (watch mode, diagnostics, perf tracing).

### Mid-term (Q3–Q4 2026)
- Implement layout system parity with Svelte (flex/grid basics).
- Establish formal IR schema and compatibility guarantees.
- Optimize rendering pipeline (diffed updates, partial redraws).
- Integrate build toolchain (Vite plugin, Cargo build hooks).
- Expand test coverage with snapshot-based rendering tests.

### Long-term
- Mature compile-time Svelte→ratatui path as optional tier.
- Support accessibility-tree transport as an alternate rendering input.
- Provide a stable SDK for third-party terminal widgets.
- Production-grade TUI support for Plures apps (RuneBook, netops, etc.).
