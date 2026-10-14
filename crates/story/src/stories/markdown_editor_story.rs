use std::sync::atomic::{AtomicBool, Ordering};

use gpui_kit::{
    App, AppContext as _, Context, Entity, IntoElement, ParentElement, Render, Styled,
    Subscription, Window, div,
};

use gpui_kit::component::{label::Label, v_flex};
use gpui_markdown_editor::{Changed, MarkdownEditor, MarkdownEditorState};

const SAMPLE: &str = "\
# Markdown Editor

A **Typora-style** live editor: the Markdown source stays plain text, but
renders *in place* — syntax markers conceal themselves until your selection
touches them. Try `==highlights==`, tasks, tables and code blocks.

## Tasks

- [x] Tree-sitter analysis per keystroke
- [ ] Click a checkbox to toggle it
- [ ] Right-click for the format menu

> [!note]
> Callouts render with a tone accent, and a footnote reference[^1] jumps to
> its definition at the end of the note.

## Table

| Feature | State |
| ------- | ----- |
| Tables as grids | done |
| Inline math $a^2 + b^2 = c^2$ | done |

## Code

```rust
fn main() {
    println!(\"syntax highlighting included\");
}
```

## Math

Inline math typesets in place — $\\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}$ inside a
sentence — and block math becomes a figure of its own:

$$
\\int_0^\\infty e^{-x^2} dx = \\frac{\\sqrt{\\pi}}{2}
$$

## Diagrams

```mermaid
graph TD
    A[Source] --> B{Render}
    B -->|LaTeX| C[RaTeX]
    B -->|Diagram| D[SVG]
    C --> E[Figure]
    D --> E
```

```plantuml
@startuml
Alice -> Bob: Hello
Bob --> Alice: Hi there
@enduml
```

## Chemistry

```smiles
CC(=O)Oc1ccccc1C(=O)O
```

```smiles
[C:1](=[O:2])[OH:3].[OH:4][C:5]>>[C:1](=[O:2])[O:4][C:5]
```

[^1]: Footnotes render inline too.
";

pub struct MarkdownEditorStory {
    state: Entity<MarkdownEditorState>,
    bytes: usize,
    _subscription: Subscription,
}

impl super::Story for MarkdownEditorStory {
    fn title() -> &'static str {
        "Markdown Editor"
    }

    fn description() -> &'static str {
        "Typora-style live Markdown editor with concealed syntax, inline rendering, tables, code blocks, callouts and footnotes."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl MarkdownEditorStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // The editor's key bindings are process-global; bind once.
        static KEYS_BOUND: AtomicBool = AtomicBool::new(false);
        if !KEYS_BOUND.swap(true, Ordering::SeqCst) {
            gpui_markdown_editor::bind_keys(cx);
        }
        let state = cx.new(|cx| {
            let mut ed = MarkdownEditorState::new(cx);
            ed.set_text(SAMPLE.to_string(), cx);
            ed
        });
        state.update(cx, |ed, cx| ed.focus(window, cx));
        let bytes = state.read(cx).text().len();
        let _subscription = cx.subscribe(&state, |this, state, _: &Changed, cx| {
            this.bytes = state.read(cx).text().len();
            cx.notify();
        });
        Self {
            state,
            bytes,
            _subscription,
        }
    }
}

impl Render for MarkdownEditorStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(div().flex_1().child(MarkdownEditor::new(&self.state)))
            .child(Label::new(format!("{} bytes", self.bytes)))
    }
}
