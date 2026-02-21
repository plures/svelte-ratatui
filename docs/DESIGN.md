# Svelte-Ratatui Compiler Design Document

**Version:** 1.0  
**Date:** 2026-02-20  
**Project:** svelte-ratatui  
**Organization:** Plures  

## Executive Summary

The Svelte-Ratatui Compiler is a build-time transpiler that enables Svelte components to render natively in terminal user interfaces via ratatui (Rust). This compiler bridges the gap between modern web development paradigms and terminal-based applications, allowing the same Svelte components to render in both GUI (Tauri webview) and terminal (ratatui) environments while sharing a common PluresDB backend.

## 1. Problem Statement and Motivation

### 1.1 The Challenge

Modern applications increasingly need to support multiple rendering targets:
- **Web browsers** (traditional web apps)
- **Desktop applications** (Electron, Tauri)
- **Terminal interfaces** (CLI tools, TUIs)
- **Mobile platforms** (native apps)

Currently, developers must maintain separate codebases for each target, leading to:
- **Code duplication** and maintenance overhead
- **Inconsistent user experiences** across platforms
- **Fragmented business logic** scattered across implementations
- **Slower development cycles** due to platform-specific rewrites

### 1.2 The Vision

The Svelte-Ratatui Compiler enables a "write once, render everywhere" approach:

```
Svelte Components → Compiler → Multiple Targets
                              ├─ Tauri (GUI)
                              ├─ Ratatui (TUI)
                              ├─ Web (Browser)
                              └─ Future targets...
```

### 1.3 Core Benefits

1. **Code Reuse**: Same business logic, same component structure
2. **Consistency**: Identical behavior across GUI and TUI modes
3. **Developer Experience**: Familiar Svelte development patterns
4. **Performance**: Native terminal rendering without JavaScript overhead
5. **Integration**: Seamless PluresDB backend sharing

## 2. Prior Art Analysis

### 2.1 Terminal Browsers

**Lynx, w3m, elinks, links2:**
- Convert HTML to terminal-renderable text
- Handle basic CSS styling (colors, bold, alignment)
- Static rendering without JavaScript execution
- **Key Insight**: Prove HTML→Terminal conversion is feasible

**Carbonyl (Chromium-based):**
- Full Chromium engine rendering to terminal
- Handles complex layouts and JavaScript
- High resource usage but comprehensive compatibility
- **Key Insight**: Complex layouts can be approximated in terminals

**Browsh (Firefox-based):**
- Uses headless Firefox for rendering
- Converts to terminal via pixel approximation
- Maintains interactive capabilities
- **Key Insight**: Interactive web apps can work in terminals

### 2.2 React-to-Terminal Solutions

**Ink (React→Terminal):**
- Runtime React reconciler for terminal rendering
- Uses Yoga layout engine for flexbox-like positioning
- Real-time component updates and state management
- **Limitations**: Runtime overhead, React-specific

**React-Blessed:**
- React renderer for blessed terminal library
- Component-based terminal UI development
- **Limitations**: Deprecated, limited widget set

### 2.3 Rust TUI Frameworks

**Ratatui:**
- Modern, performant terminal UI framework
- Widget-based architecture
- Excellent event handling and styling
- **Strengths**: Our target rendering engine

**Dioxus TUI:**
- React-like framework with terminal renderer
- RSX syntax similar to JSX
- **Insight**: Proves React-like patterns work in terminals

**Yew TUI (experimental):**
- Web framework with terminal backend
- Component-driven development
- **Insight**: Web patterns translate to terminals

### 2.4 Svelte Compilation Pipeline

**Svelte Compiler:**
- Compiles `.svelte` files to vanilla JavaScript
- Generates optimized component code
- Produces intermediate representations (AST/IR)
- **Key Opportunity**: We can hook into this pipeline

## 3. Architecture Overview

### 3.1 Compilation Pipeline

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   .svelte       │───▶│  Svelte Compiler │───▶│ JavaScript IR   │
│   Components    │    │                  │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                                        │
                                                        ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Rust Code     │◀───│ Svelte-Ratatui   │◀───│ Analysis &      │
│  (ratatui)      │    │    Compiler      │    │ Transformation  │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

### 3.2 Core Components

1. **AST Parser**: Analyzes Svelte's compiled output
2. **Element Mapper**: Maps HTML elements to ratatui widgets
3. **Style Processor**: Converts CSS subset to ratatui styles
4. **State Manager**: Handles Svelte reactivity in terminal context
5. **Event Bridge**: Maps terminal events to Svelte handlers
6. **Code Generator**: Produces optimized Rust code

### 3.3 Runtime Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   PluresDB      │◀───│  Pares TUI      │───▶│   Terminal      │
│   Backend       │    │  Framework      │    │   Display       │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         ▲                       ▲                       ▲
         │              ┌─────────────────┐              │
         └──────────────│ Compiled Rust   │──────────────┘
                        │   Components    │
                        └─────────────────┘
```

## 4. Element Mapping Strategy

### 4.1 HTML Element → Ratatui Widget Mapping

| HTML Element | Ratatui Widget | Notes |
|--------------|----------------|-------|
| `<div>` | `Block` + `Layout` | Container with optional borders |
| `<p>` | `Paragraph` | Text content with styling |
| `<h1>-<h6>` | `Paragraph` | With bold/underline styling |
| `<span>` | Inline `Span` | Styled text segment |
| `<input>` | Custom `Input` | Text input widget |
| `<button>` | `Block` + `Paragraph` | Clickable text block |
| `<ul>/<ol>` | `List` | Bullet or numbered lists |
| `<li>` | `ListItem` | List item |
| `<table>` | `Table` | Tabular data display |
| `<tr>` | `Row` | Table row |
| `<td>/<th>` | `Cell` | Table cell |
| `<textarea>` | Custom `TextArea` | Multi-line text input |
| `<select>` | Custom `Select` | Dropdown/picker widget |
| `<pre>` | `Paragraph` | Preserve whitespace |
| `<code>` | `Paragraph` | Monospace styling |

### 4.2 Layout System

**Flexbox → Ratatui Layout:**
- `display: flex` → `Layout::default()`
- `flex-direction: row` → `Direction::Horizontal`
- `flex-direction: column` → `Direction::Vertical`
- `justify-content` → `Alignment` configuration
- `flex-grow` → `Constraint::Percentage` or `Constraint::Ratio`

**Grid → Nested Layouts:**
- CSS Grid translated to nested `Layout` widgets
- Grid areas mapped to specific layout regions

## 5. CSS Subset Mapping

### 5.1 Supported CSS Properties

| CSS Property | Ratatui Style | Example |
|--------------|---------------|---------|
| `color` | `Style::fg()` | `color: red` → `Style::default().fg(Color::Red)` |
| `background-color` | `Style::bg()` | `background: blue` → `Style::default().bg(Color::Blue)` |
| `font-weight: bold` | `Style::add_modifier(Modifier::BOLD)` | Bold text |
| `font-style: italic` | `Style::add_modifier(Modifier::ITALIC)` | Italic text |
| `text-decoration: underline` | `Style::add_modifier(Modifier::UNDERLINED)` | Underlined text |
| `text-align` | `Alignment` | `text-align: center` → `Alignment::Center` |
| `width` | `Constraint::Length` | `width: 100px` → `Constraint::Length(100)` |
| `height` | `Constraint::Length` | `height: 50px` → `Constraint::Length(50)` |
| `margin` | `Margin` | Spacing around widgets |
| `padding` | `Padding` | Internal widget spacing |
| `border` | `Block::border_type()` | Border styles |
| `display` | Layout behavior | `flex`, `block`, `inline` |

### 5.2 Color System

```rust
// Named colors
color: red → Color::Red
color: blue → Color::Blue
color: green → Color::Green

// Hex colors
color: #ff0000 → Color::Rgb(255, 0, 0)
color: #00ff00 → Color::Rgb(0, 255, 0)

// RGB/HSL
color: rgb(255, 0, 0) → Color::Rgb(255, 0, 0)
color: hsl(0, 100%, 50%) → Color::Rgb(255, 0, 0) // converted
```

## 6. Reactivity Model Mapping

### 6.1 Svelte Reactivity → Ratatui State

**Svelte State:**
```javascript
let count = $state(0);
let doubled = $derived(count * 2);
```

**Generated Rust:**
```rust
#[derive(Clone)]
struct ComponentState {
    count: i32,
    doubled: i32,
}

impl ComponentState {
    fn update_count(&mut self, new_count: i32) {
        self.count = new_count;
        self.doubled = new_count * 2; // Auto-update derived
    }
}
```

### 6.2 Reactive Updates

1. **State Changes**: Trigger full or partial re-renders
2. **Derived Values**: Automatically recomputed when dependencies change
3. **Component Communication**: Props and events handled via message passing
4. **Batched Updates**: Multiple state changes batched for performance

### 6.3 Component Lifecycle

```rust
trait SvelteComponent {
    fn on_mount(&mut self);
    fn on_update(&mut self);
    fn on_destroy(&mut self);
    fn render(&self, area: Rect) -> Vec<Widget>;
    fn handle_event(&mut self, event: Event) -> EventResult;
}
```

## 7. Event Model Mapping

### 7.1 DOM Events → Terminal Events

| DOM Event | Terminal Event | Mapping |
|-----------|----------------|---------|
| `click` | `MouseEvent::Down` | Click coordinates → widget bounds check |
| `keydown` | `KeyEvent` | Direct key mapping |
| `input` | `KeyEvent` + `Input` | Character input handling |
| `focus` | Custom focus system | Widget focus management |
| `blur` | Custom focus system | Focus loss handling |
| `change` | Custom change events | Value change notifications |
| `submit` | `KeyEvent::Enter` | Form submission |

### 7.2 Event Handling Pipeline

```rust
pub enum ComponentEvent {
    Click { x: u16, y: u16 },
    KeyPress(KeyEvent),
    Input(String),
    Focus,
    Blur,
    Custom(String, Value),
}

impl Component {
    pub fn handle_event(&mut self, event: ComponentEvent) -> EventResult {
        match event {
            ComponentEvent::Click { x, y } => self.on_click(x, y),
            ComponentEvent::KeyPress(key) => self.on_key(key),
            // ... other events
        }
    }
}
```

### 7.3 Event Binding

**Svelte:**
```javascript
<button on:click={handleClick}>Click me</button>
```

**Generated Rust:**
```rust
Button::new("Click me")
    .on_click(|component| component.handle_click())
```

## 8. Integration with Pares TUI Framework

### 8.1 Split-Pane Chat Architecture

The compiler must generate components compatible with the Pares TUI's split-pane chat system:

```rust
pub struct ChatPane {
    messages: Vec<Message>,
    participants: Vec<Participant>,
    layout: PaneLayout, // Left/Right split
}

pub struct Message {
    content: String,
    author: String,
    timestamp: DateTime<Utc>,
    recipients: Vec<String>, // @directive targets
}
```

### 8.2 PluresDB Integration

Components connect to PluresDB via the unified local-first API:

```rust
use plures_db::{LocalFirstBackend, LocalFirstOptions};

pub struct ComponentDB {
    backend: Box<dyn LocalFirstBackend>,
}

impl ComponentDB {
    pub async fn put_message(&self, message: Message) -> Result<String, Error> {
        self.backend.put(&message.id, &message).await
    }
    
    pub async fn get_messages(&self) -> Result<Vec<Message>, Error> {
        // Vector search for relevant messages
        self.backend.vector_search("", 100).await
    }
}
```

### 8.3 Plures Procedures

Generated components can trigger Plures Procedures:

```rust
pub trait ProcedureTrigger {
    async fn trigger_procedure(&self, name: &str, data: Value) -> Result<(), Error>;
}

// Usage in generated component
impl ChatComponent {
    async fn send_message(&mut self, content: String) {
        let message = Message::new(content);
        self.db.put_message(message).await?;
        
        // Trigger message routing procedure
        self.trigger_procedure("route_message", json!({
            "message_id": message.id,
            "content": message.content
        })).await?;
    }
}
```

### 8.4 Command System Integration

Support for @ and / commands in the TUI framework:

```rust
pub enum Command {
    AtCommand { target: String, message: String },  // @agent hello
    SlashCommand { action: String, args: Vec<String> }, // /clear
    RegularMessage(String),
}

pub fn parse_input(input: &str) -> Command {
    if input.starts_with('@') {
        // Parse @target message
    } else if input.starts_with('/') {
        // Parse /command args
    } else {
        Command::RegularMessage(input.to_string())
    }
}
```

## 9. Compiler Implementation

### 9.1 Crate Structure

```
svelte-ratatui/
├── svelte-ratatui-compiler/    # Main compiler crate
├── svelte-ratatui-runtime/     # Runtime bridge and utilities
└── svelte-ratatui-cli/         # CLI tool for compilation
```

### 9.2 Compilation Process

1. **Parse Svelte IR**: Extract component structure from Svelte's compiled output
2. **Analyze Dependencies**: Identify state variables, derived values, and props
3. **Map Elements**: Convert HTML elements to ratatui widgets
4. **Process Styles**: Transform CSS to ratatui styling
5. **Generate State Management**: Create reactive state handling code
6. **Wire Events**: Connect terminal events to component handlers
7. **Output Rust**: Generate optimized Rust source code

### 9.3 Configuration

```toml
# svelte-ratatui.toml
[compiler]
target = "ratatui"
optimize = true
debug = false

[style]
color_mode = "rgb"  # rgb, ansi, none
unicode = true
borders = true

[components]
input_dir = "src/components"
output_dir = "src/generated"

[integration]
pares_tui = true
plures_db = true
```

## 10. Example: Hello World Component

### 10.1 Svelte Source

```svelte
<!-- HelloWorld.svelte -->
<script>
  let name = $state('World');
  let count = $state(0);
  let greeting = $derived(`Hello, ${name}!`);
  
  function increment() {
    count += 1;
  }
</script>

<div class="container">
  <h1 style="color: blue;">{greeting}</h1>
  <p>Count: {count}</p>
  <button on:click={increment}>Increment</button>
  <input bind:value={name} placeholder="Enter name" />
</div>

<style>
  .container {
    padding: 2;
    border: 1px solid white;
    width: 60;
    height: 20;
  }
</style>
```

### 10.2 Generated Rust

```rust
use ratatui::{prelude::*, widgets::*};
use crossterm::event::{Event, KeyCode, MouseEvent};

#[derive(Clone)]
pub struct HelloWorldState {
    name: String,
    count: i32,
    greeting: String,
}

impl HelloWorldState {
    pub fn new() -> Self {
        let mut state = Self {
            name: "World".to_string(),
            count: 0,
            greeting: String::new(),
        };
        state.update_greeting();
        state
    }
    
    fn update_greeting(&mut self) {
        self.greeting = format!("Hello, {}!", self.name);
    }
    
    pub fn set_name(&mut self, name: String) {
        self.name = name;
        self.update_greeting();
    }
    
    pub fn increment(&mut self) {
        self.count += 1;
    }
}

pub struct HelloWorldComponent {
    state: HelloWorldState,
    focused_widget: usize,
}

impl HelloWorldComponent {
    pub fn new() -> Self {
        Self {
            state: HelloWorldState::new(),
            focused_widget: 0,
        }
    }
    
    pub fn render(&self, area: Rect) -> Vec<Box<dyn Widget>> {
        let container = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
            .padding(Padding::uniform(2));
        
        let inner = container.inner(area);
        
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title
                Constraint::Length(1), // Count
                Constraint::Length(1), // Button
                Constraint::Length(1), // Input
            ])
            .split(inner);
        
        vec![
            Box::new(container),
            Box::new(Paragraph::new(self.state.greeting.clone())
                .style(Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD))
                .block(Block::default().title("Greeting"))),
            Box::new(Paragraph::new(format!("Count: {}", self.state.count))),
            Box::new(Paragraph::new("[ Increment ]")
                .alignment(Alignment::Center)
                .style(if self.focused_widget == 2 {
                    Style::default().bg(Color::Blue)
                } else {
                    Style::default()
                })),
            // Input widget would be more complex, simplified here
        ]
    }
    
    pub fn handle_event(&mut self, event: Event) -> bool {
        match event {
            Event::Key(key) => {
                match key.code {
                    KeyCode::Tab => {
                        self.focused_widget = (self.focused_widget + 1) % 3;
                        true
                    }
                    KeyCode::Enter if self.focused_widget == 2 => {
                        self.state.increment();
                        true
                    }
                    _ => false,
                }
            }
            Event::Mouse(mouse) => {
                // Handle mouse clicks on button
                false
            }
            _ => false,
        }
    }
}
```

## 11. Development Milestones and Roadmap

### 11.1 Phase 1: Foundation (4-6 weeks)

**Week 1-2: Core Infrastructure**
- [ ] Set up Rust workspace with three crates
- [ ] Implement basic Svelte IR parser
- [ ] Create element mapping framework
- [ ] Basic CLI tool structure

**Week 3-4: Basic Compilation**
- [ ] Simple element-to-widget mapping
- [ ] Basic style processing
- [ ] Static component generation
- [ ] Hello World example working

**Week 5-6: State Management**
- [ ] Implement $state mapping
- [ ] Basic $derived support
- [ ] Simple event handling
- [ ] Component lifecycle hooks

**Deliverables:**
- Working compiler for simple static components
- Basic CLI tool
- Initial documentation
- Simple examples

### 11.2 Phase 2: Reactivity (4-6 weeks)

**Week 7-8: Advanced State**
- [ ] Complex $derived expressions
- [ ] State change batching
- [ ] Component props system
- [ ] Parent-child communication

**Week 9-10: Event System**
- [ ] Comprehensive event mapping
- [ ] Focus management
- [ ] Form handling
- [ ] Custom events

**Week 11-12: Integration**
- [ ] Pares TUI framework integration
- [ ] PluresDB backend connection
- [ ] Split-pane chat components
- [ ] Command system (@ and /)

**Deliverables:**
- Fully reactive components
- Complete event system
- Pares TUI integration
- Advanced examples

### 11.3 Phase 3: Production Ready (4-6 weeks)

**Week 13-14: Performance**
- [ ] Compilation optimization
- [ ] Runtime performance tuning
- [ ] Memory usage optimization
- [ ] Incremental compilation

**Week 15-16: Developer Experience**
- [ ] Better error messages
- [ ] Development mode features
- [ ] Hot reload support
- [ ] IDE integration

**Week 17-18: Production Features**
- [ ] Build optimization
- [ ] Tree shaking
- [ ] Component library support
- [ ] Plugin system

**Deliverables:**
- Production-ready compiler
- Excellent developer experience
- Complete documentation
- Real-world applications

### 11.4 Future Enhancements

**Advanced Features:**
- Server-side rendering support
- Component code splitting
- Advanced layout systems
- Custom widget definitions
- Multi-platform optimization

**Ecosystem:**
- Svelte component library
- IDE plugins and extensions
- Build tool integrations
- Community contributions

## 12. Technical Challenges and Solutions

### 12.1 JavaScript → Rust Translation

**Challenge:** Converting dynamic JavaScript patterns to static Rust code.

**Solution:** 
- Use Rust's type system to enforce correctness
- Generate trait implementations for common patterns
- Provide runtime bridge for dynamic behaviors

### 12.2 Layout Complexity

**Challenge:** CSS layout is more flexible than ratatui's constraint-based system.

**Solution:**
- Define supported layout subset
- Provide fallbacks for unsupported features
- Generate warnings for incompatible CSS

### 12.3 Event Handling Differences

**Challenge:** DOM events don't directly map to terminal events.

**Solution:**
- Create event translation layer
- Implement virtual focus system
- Provide event composition utilities

### 12.4 Performance Optimization

**Challenge:** Maintaining performance while providing rich functionality.

**Solution:**
- Compile-time optimizations
- Lazy rendering strategies
- Efficient state diffing
- Memory pooling for widgets

## 13. Success Metrics

### 13.1 Technical Metrics

- **Compilation Speed:** < 5 seconds for medium projects
- **Runtime Performance:** 60+ FPS for typical UIs
- **Memory Usage:** < 10MB for complex applications
- **Binary Size:** < 5MB statically linked executables

### 13.2 Developer Experience Metrics

- **Learning Curve:** Svelte developers productive within 1 day
- **Error Quality:** Clear, actionable error messages
- **Documentation:** Complete examples and tutorials
- **Community:** Active contributors and users

### 13.3 Feature Completeness

- **Svelte Features:** 80%+ compatibility with Svelte 5
- **CSS Support:** All common styling properties
- **Widget Library:** Comprehensive ratatui widget coverage
- **Integration:** Seamless Pares TUI framework support

## 14. Conclusion

The Svelte-Ratatui Compiler represents a significant advancement in cross-platform application development, enabling developers to write once and deploy to both GUI and terminal environments. By leveraging the strengths of Svelte's component model and ratatui's performance, we can create a powerful tool that bridges the gap between web and terminal development.

The phased development approach ensures we deliver value incrementally while building toward a comprehensive solution. The integration with the Pares TUI framework and PluresDB backend creates a unified ecosystem for building modern, distributed applications.

This project has the potential to significantly impact how developers approach cross-platform development, providing a new paradigm that combines the best of web and terminal user interfaces.

---

**Document Status:** Draft v1.0  
**Next Review:** 2026-03-01  
**Maintained By:** Plures Development Team