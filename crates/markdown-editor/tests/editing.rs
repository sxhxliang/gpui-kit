//! Regression test: the public `MarkdownEditor` element must be editable,
//! not just paintable.
//!
//! `MarkdownEditor` once rendered the bare `EditorElement`, bypassing
//! `LiveEditor::render` — the div that carries focus tracking, the
//! `LiveEditor` key context, every editing action handler, and the mouse
//! and scroll handlers. The document painted correctly but no keystroke,
//! click, or IME input could change it. This test drives the real key
//! dispatch chain (keybindings → actions → buffer edits, plus text
//! insertion through the input handler) and asserts the text changes.

use gpui::{Context, Entity, IntoElement, Render, TestAppContext, Window, div, prelude::*};
use gpui_markdown_editor::{MarkdownEditor, MarkdownEditorState};

struct Host {
    state: Entity<MarkdownEditorState>,
}

impl Render for Host {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(MarkdownEditor::new(&self.state))
    }
}

fn text(host: &Entity<Host>, window: &mut gpui::VisualTestContext) -> String {
    let mut out = String::new();
    window.update(|_, cx| {
        out = host.read(cx).state.read(cx).text().to_string();
    });
    out
}

#[gpui::test]
fn typing_and_editing_actions_change_the_text(cx: &mut TestAppContext) {
    let (host, window) = cx.add_window_view(|window, cx| {
        gpui_markdown_editor::bind_keys(cx);
        let state = cx.new(|cx| MarkdownEditorState::new(cx));
        state.update(cx, |ed, cx| ed.focus(window, cx));
        Host { state }
    });
    window.update(|window, cx| window.draw(cx).clear(cx));

    // Plain typing reaches the buffer through the input handler.
    window.simulate_input("hello");
    assert_eq!(text(&host, window), "hello");

    // Backspace is an action dispatched through the `LiveEditor` key
    // context — the chain the bare element was missing entirely.
    window.simulate_keystrokes("backspace");
    assert_eq!(text(&host, window), "hell");

    // Caret movement + insertion in the middle.
    window.simulate_keystrokes("left left");
    window.simulate_input("Y");
    assert_eq!(text(&host, window), "heYll");

    // Enter splits the line; select-all + backspace empties the document.
    window.simulate_keystrokes("end enter");
    window.simulate_input("world");
    assert_eq!(text(&host, window), "heYll\nworld");
    window.simulate_keystrokes("ctrl-a backspace");
    assert_eq!(text(&host, window), "");
}
