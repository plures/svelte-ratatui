# Allowed Svelte Dialect for TUI Compilation

**Version:** 1.0  
**Status:** Normative  
**Date:** 2026-02-22

The svelte-ratatui compiler targets only the *design-dojo* component subset.
This constrained dialect makes the compilation guarantee possible: every
allowed construct has an unambiguous ratatui equivalent, and every disallowed
construct produces a **hard compiler error** with a stable error code.

---

## Constraint Table

| Pattern | Allowed | Error if violated |
|---------|:-------:|-------------------|
| `$state` with primitive types (`bool`, `i32`, `f64`, `String`) | ✅ | — |
| `$state` with arrays / `Vec<T>` | ✅ | — |
| `$state` with plain structs (all-public fields, no methods) | ✅ | — |
| `$derived` with pure expressions (no side effects, no `await`) | ✅ | — |
| `$effect` — sync side effect | ✅ | — |
| `$effect` — PluresDB subscription via `db.subscribe()` | ✅ | — |
| `$effect` — IPC subscription via `ipc()` | ✅ | — |
| `$effect` — arbitrary `async`/`await` | ❌ | **E001** |
| `$props()` and `$bindable()` | ✅ | — |
| `{#snippet}` / `{@render}` | ✅ | maps to render sub-region |
| `{#if}` / `{#each}` / `{#key}` | ✅ | — |
| `<svelte:component>` dynamic dispatch | ❌ | **E002** |
| `{@html …}` raw HTML | ❌ | **E003** |
| External NPM library call inside template `{…}` | ❌ | **E004** |
| `<svelte:window>` / `<svelte:document>` | ❌ | *(future: E005)* |
| `<svelte:head>` | ❌ | *(future: E006)* |

---

## Error Codes

### E001 — Unsanctioned async effect

**Message:** `E001: async effects must use db.subscribe() or ipc() patterns`

ratatui uses a synchronous, tick-based event loop. Arbitrary `async`/`await`
inside `$effect` cannot be safely mapped to this model.

**Allowed pattern** (PluresDB subscription):

```svelte
$effect(() => {
  const unsub = db.subscribe("messages", msgs => { messages = msgs; });
  return unsub;   // cleanup is called on component destroy
});
```

**Allowed pattern** (IPC channel):

```svelte
$effect(() => {
  const unsub = ipc("my-channel", data => { value = data; });
  return unsub;
});
```

**Disallowed** — triggers E001:

```svelte
// ❌ arbitrary fetch inside $effect
$effect(async () => {
  const res = await fetch('/api/data');
  data = await res.json();
});
```

**Fix:** Use `db.subscribe()` or `ipc()` and handle the async work in the
subscribed callback, or move async work outside `$effect` entirely.

---

### E002 — Dynamic component dispatch

**Message:** `E002: dynamic components not supported in TUI mode`

`<svelte:component this={…}>` selects a component at runtime. ratatui renders
a static widget tree at compile time; there is no equivalent construct.

**Disallowed** — triggers E002:

```svelte
<script>
  let comp = condition ? FooComponent : BarComponent;
</script>
<!-- ❌ -->
<svelte:component this={comp} />
```

**Fix:** Use `{#if}` / `{:else}` branching:

```svelte
{#if condition}
  <FooComponent />
{:else}
  <BarComponent />
{/if}
```

---

### E003 — Raw HTML injection

**Message:** `E003: raw HTML has no TUI equivalent`

`{@html …}` injects arbitrary HTML markup. The terminal has no HTML renderer;
there is no ratatui widget that can accept raw HTML.

**Disallowed** — triggers E003:

```svelte
<script>
  let markup = "<strong>bold</strong>";
</script>
<!-- ❌ -->
<div>{@html markup}</div>
```

**Fix:** Use plain text interpolation (`{markup}`) or a ratatui `Span` with
explicit styling.

---

### E004 — External library call in template

**Message:** `E004: only design-dojo components allowed in TUI views (found external library call: <name>)`

Template expressions (`{…}`) must not call functions imported from external
NPM packages. The compiler cannot reason about arbitrary JavaScript library
semantics when emitting Rust.

**Disallowed** — triggers E004:

```svelte
<script>
  import { groupBy } from 'lodash';     // ❌ external package
  import moment from 'moment';          // ❌ external package
  let items = $state([]);
</script>
<div>{groupBy(items, 'type')}</div>
<p>{moment(ts).fromNow()}</p>
```

**Allowed imports:**

| Import source | Allowed |
|---------------|:-------:|
| Relative path (`./utils`, `../shared`) | ✅ |
| `$lib/…` (SvelteKit internal alias) | ✅ |
| `svelte` / `svelte/*` built-ins | ✅ |
| Any external NPM package used in template | ❌ |

**Fix:** Move data transformation into state setters or `$derived` expressions
that call helper functions defined within the component or in a relative
`$lib/…` module.

---

## Scope of Validation

The dialect checker (`svelte-ratatui-compiler::dialect_check`) performs a
**source-level scan** of the `.svelte` file. It does not perform full semantic
analysis; it uses textual heuristics that are accurate for well-formed
design-dojo components.

The checker is intentionally conservative: it emits a false positive rather
than silently accepting a construct that cannot be compiled. If you believe a
check is incorrect for your use case, file an issue with a minimal reproducer.

---

## Running the Checker

The dialect check runs automatically as the first pass of `svelte-ratatui compile`.
To run it in isolation:

```
svelte-ratatui check <input.svelte>
```

Errors are printed to stderr in the format:

```
[E001] line 12: E001: async effects must use db.subscribe() or ipc() patterns
[E003] line 27: E003: raw HTML has no TUI equivalent
```

A non-zero exit code is returned when any error is found.

---

*See also: [`RUNES-TRANSLATION.md`](RUNES-TRANSLATION.md) for the full
Rune → Rust mapping specification.*
