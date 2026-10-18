//! Regression coverage: drawing a document with inline math must not
//! panic, end to end through prepaint.
//!
//! The inline-figure path probes the shaped width of one NBSP to size its
//! placeholder. The probe's `TextRun` once used a length of 1 for the
//! two-byte `"\u{a0}"` string; platform text systems that split the text
//! at run boundaries (macOS CoreText) panicked with "end byte index 1 is
//! not a char boundary; it is inside '\u{a0}'" as soon as a line carrying
//! inline math was drawn. Text systems that tolerate the mismatch (Linux)
//! do not reproduce that panic here — the byte-length invariant itself is
//! enforced by construction (a single `NBSP` constant, lengths always
//! derived with `NBSP.len()`); this test keeps the draw path exercised.

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

#[gpui::test]
fn inline_math_document_draws(cx: &mut TestAppContext) {
    let (_, window) = cx.add_window_view(|window, cx| {
        gpui_markdown_editor::bind_keys(cx);
        let state = cx.new(|cx| {
            let mut ed = MarkdownEditorState::new(cx);
            ed.set_text(
                "Inline math $\\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}$ in a sentence.\n\n\
                 $x^2$ at the start of a line, and a table cell:\n\n\
                 | Cell | Note |\n\
                 | ---- | ---- |\n\
                 | $a^2 + b^2 = c^2$ and $\\frac{1}{2}$ | done |\n"
                    .to_string(),
                cx,
            );
            ed
        });
        state.update(cx, |ed, cx| ed.focus(window, cx));
        Host { state }
    });
    // Two draws: the first lays out with the figure cache cold, the second
    // paints over the cached placeholders.
    window.update(|window, cx| window.draw(cx).clear(cx));
    window.update(|window, cx| window.draw(cx).clear(cx));
}
