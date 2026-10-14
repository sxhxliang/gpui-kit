//! Typefaces the editor shapes text with: a prose face (`sans`) and the
//! code/table face (`mono`).
//!
//! Extracted from abstract's `fonts` module. The editor state owns a
//! `Fonts` value (see `MarkdownEditorState::set_fonts`); the default is
//! abstract's bundled Noto pair.

use gpui::SharedString;

/// Deliberately record-like configuration (theme-token style): public
/// fields, with [`Fonts::new`] as the constructor.
#[derive(Clone)]
#[non_exhaustive]
pub struct Fonts {
    pub sans: SharedString,
    pub mono: SharedString,
}

impl Fonts {
    pub fn new(sans: impl Into<SharedString>, mono: impl Into<SharedString>) -> Self {
        Self {
            sans: sans.into(),
            mono: mono.into(),
        }
    }
}

impl Default for Fonts {
    fn default() -> Self {
        Self {
            sans: "Noto Sans".into(),
            mono: "Noto Sans Mono".into(),
        }
    }
}
