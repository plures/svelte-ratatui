# svelte-ratatui Roadmap

## Role in OASIS
svelte‑ratatui enables the multi‑GUI promise for OASIS: one Svelte codebase that runs as GUI, TUI, and native terminal (via ratatui). It is the bridge that keeps terminal experiences first‑class alongside desktop and mobile.

## Current State
- Pre‑alpha architecture with compiler/runtime/adapter crates in place.
- Input/event translation implemented; CLI scaffolding exists.
- Open issues focused on compiler pipeline and end‑to‑end TUI test.

## Phase 1 — Compiler Pipeline
- Wire parser + IR + codegen into a full `compile()` pipeline.
- Implement Rust codegen from IR tree.
- End‑to‑end test: `.svelte` → running TUI binary.

## Phase 2 — Widget & Layout Coverage
- Expand widget mapping coverage for design‑dojo primitives.
- Add layout parity (flex/grid basics, spacing tokens).
- Snapshot‑based rendering tests for parity validation.

## Phase 3 — Tooling & Integration
- Vite plugin + Cargo build hooks.
- `tauri-plugin-tui` ergonomics + docs for Radix/Modulus apps.
- Performance profiling and incremental re‑rendering.

## Phase 4 — Production Readiness
- Stable IR schema and compatibility guarantees.
- Accessibility and focus model for terminal UX.
- Release path for TUI‑capable OASIS apps (RuneBook, Netops, etc.).
