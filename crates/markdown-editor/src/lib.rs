//! A Typora-style live Markdown editor for GPUI Kit.
//!
//! The editor edits plain Markdown source, but renders it inline the way
//! Typora does: headings, emphasis, `==highlights==`, code blocks with
//! syntax highlighting, pipe tables as grids, tasks, callouts, footnotes
//! and math approximations render in place, and the syntax markers conceal
//! themselves until the selection touches them. Analysis is tree-sitter
//! based ([`md`]); text storage is a gap-free [`buffer::Buffer`] with undo.
//!
//! Extracted from [abstract](https://github.com/fireflylabss/abstract)
//! (Apache-2.0) and decoupled from its app shell: colors come from a
//! [`Palette`] the state owns, strings from [`i18n`] with host overrides,
//! and everything app-shaped (opening links, dropped/pasted attachments,
//! export commands, spell checking) leaves through events, actions, or the
//! [`spell::SpellChecker`] seam.
//!
//! # Usage
//!
//! ```ignore
//! use gpui_markdown_editor::{Changed, MarkdownEditor, MarkdownEditorState};
//!
//! // Once, at startup, alongside gpui_kit::init:
//! gpui_markdown_editor::bind_keys(cx);
//!
//! // Per document:
//! let state = cx.new(MarkdownEditorState::new);
//! cx.subscribe(&state, |_, _, _: &Changed, cx| {
//!     // persist state.read(cx).text()
//! });
//!
//! // In a view's Render:
//! div().child(MarkdownEditor::new(&state))
//! ```
//!
//! The state defaults to the [`DARK`] palette and the editor paints no
//! background of its own, so set the palette for your color scheme with
//! `set_theme` at creation (and when it changes) — otherwise dark-palette
//! text sits unreadable on a light window. Likewise `set_fonts` to
//! families your app actually has: the defaults are abstract's bundled
//! Noto pair, and where they are missing the fallback face loses
//! bold/italic traits, so emphasis silently renders as regular text.
//!
//! Configuration lives on the state: `set_theme`, `set_fonts`,
//! `set_text_width` (via `set_max_col`), `set_smart_quotes`,
//! `set_focus_mode`, `set_raw_tables`, `set_dirs` (for relative image
//! resolution), `set_finds` (find-bar highlights). Spell checking is off
//! until a checker is installed with [`spell::set_spell_checker`] and
//! enabled with `MarkdownEditorState::set_spell`.

pub mod actions;
pub mod assets;
pub mod attach;
pub mod buffer;
pub mod code;
pub mod ctx_menu;
pub mod editor;
pub mod fonts;
pub mod footnote;
pub mod glass;
pub mod i18n;
pub mod lines;
pub mod links;
pub mod math;
pub mod md;
pub mod render;
pub mod smart;
pub mod spell;
pub mod table;
pub mod theme;
pub mod zoom;

pub use editor::{
    Attach, Changed, CompletionKey, CompletionMove, EditorElement, Escaped, HighlightTint,
    LiveEditor as MarkdownEditorState, OpenLink, SpellAddWord, SpellApply, TextWidth, bind_keys,
};
pub use fonts::Fonts;
pub use glass::Glass;
pub use render::{DiagramKind, Figure};
pub use theme::{DARK, LIGHT, Palette};

use gpui::{App, Entity, IntoElement, RenderOnce, Window};

/// The editor element: wraps a [`MarkdownEditorState`] entity. Build one
/// per render, the same shape as `gpui_component::input::Input`.
#[derive(IntoElement)]
pub struct MarkdownEditor {
    state: Entity<MarkdownEditorState>,
}

impl MarkdownEditor {
    pub fn new(state: &Entity<MarkdownEditorState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for MarkdownEditor {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        // Render the state entity itself, not the bare `EditorElement`:
        // `LiveEditor::render` supplies the interactive shell (focus
        // tracking, the `LiveEditor` key context, every editing action,
        // mouse and scroll handling, the context menu) around the element.
        // The element alone paints the document but accepts no input.
        self.state
    }
}
