//! UI strings for the editor's menus and placeholders.
//!
//! Extracted from abstract's `i18n` module, English only. The host can
//! override every string at runtime with [`set_labels`]; [`t`] keeps the
//! call shape abstract's editor code already uses.

use std::sync::RwLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    AddLink,
    AddWikiLink,
    Bold,
    BulletList,
    Callout,
    CodeBlock,
    Comment,
    Copy,
    CopyAsHtml,
    Cut,
    Divider,
    EditorAria,
    EditorPlaceholder,
    ExportAsHtml,
    ExportAsPdf,
    FindInNote,
    Footnote,
    Format,
    Heading1,
    Heading2,
    Heading3,
    Highlight,
    HighlightColor,
    InlineCode,
    Insert,
    InsertImage,
    Italic,
    MathBlock,
    MathInline,
    NumberedList,
    Paragraph,
    Paste,
    PastePlain,
    PlainText,
    Quote,
    SearchSelection,
    SelectAll,
    SpellAddWord,
    Strikethrough,
    Table,
    TableColumn,
    TableDeleteColumn,
    TableDeleteRow,
    TableInsertColumnLeft,
    TableInsertColumnRight,
    TableInsertRowAbove,
    TableInsertRowBelow,
    TaskList,
    TintBlue,
    TintGreen,
    TintOrange,
    TintPurple,
    TintRed,
    TintYellow,
}

fn en(k: Key) -> &'static str {
    match k {
        Key::AddLink => "Add external link",
        Key::AddWikiLink => "Add note link",
        Key::Bold => "Bold",
        Key::BulletList => "Bulleted list",
        Key::Callout => "Callout",
        Key::CodeBlock => "Code block",
        Key::Comment => "Hidden comment",
        Key::Copy => "Copy",
        Key::CopyAsHtml => "Copy as HTML",
        Key::Cut => "Cut",
        Key::Divider => "Divider",
        Key::EditorAria => "Markdown editor",
        Key::EditorPlaceholder => "Start writing…",
        Key::ExportAsHtml => "Export as HTML…",
        Key::ExportAsPdf => "Export as PDF…",
        Key::FindInNote => "Find in note…",
        Key::Footnote => "Footnote",
        Key::Format => "Format",
        Key::Heading1 => "Heading 1",
        Key::Heading2 => "Heading 2",
        Key::Heading3 => "Heading 3",
        Key::Highlight => "Highlight",
        Key::HighlightColor => "Highlight color",
        Key::InlineCode => "Code",
        Key::Insert => "Insert",
        Key::InsertImage => "Image…",
        Key::Italic => "Italic",
        Key::MathBlock => "Math block",
        Key::MathInline => "Inline math",
        Key::NumberedList => "Numbered list",
        Key::Paragraph => "Paragraph",
        Key::Paste => "Paste",
        Key::PastePlain => "Paste as plain text",
        Key::PlainText => "Body text",
        Key::Quote => "Quote",
        Key::SearchSelection => "Search for selection",
        Key::SelectAll => "Select all",
        Key::SpellAddWord => "Add to dictionary",
        Key::Strikethrough => "Strikethrough",
        Key::Table => "Table",
        Key::TableColumn => "Column {n}",
        Key::TableDeleteColumn => "Delete column",
        Key::TableDeleteRow => "Delete row",
        Key::TableInsertColumnLeft => "Insert column to the left",
        Key::TableInsertColumnRight => "Insert column to the right",
        Key::TableInsertRowAbove => "Insert row above",
        Key::TableInsertRowBelow => "Insert row below",
        Key::TaskList => "Checklist",
        Key::TintBlue => "Blue",
        Key::TintGreen => "Green",
        Key::TintOrange => "Orange",
        Key::TintPurple => "Purple",
        Key::TintRed => "Red",
        Key::TintYellow => "Yellow",
    }
}

/// Host-provided string overrides: replace the built-in English text per
/// [`Key`] (localization, branding, wording tweaks).
static LABELS: RwLock<Vec<(Key, &'static str)>> = RwLock::new(Vec::new());

/// Replace all string overrides.
pub fn set_labels(overrides: Vec<(Key, &'static str)>) {
    *LABELS.write().unwrap() = overrides;
}

pub fn t(k: Key) -> &'static str {
    if let Ok(labels) = LABELS.read() {
        if let Some((_, s)) = labels.iter().find(|(key, _)| *key == k) {
            return s;
        }
    }
    en(k)
}

/// `t` with `{name}` placeholders replaced from `args`. `{MOD}` expands to
/// `Cmd` on macOS, `Ctrl` elsewhere.
pub fn tf(k: Key, args: &[(&str, &str)]) -> String {
    let mut s = t(k).to_string();
    s = s.replace(
        "{MOD}",
        if cfg!(target_os = "macos") {
            "Cmd"
        } else {
            "Ctrl"
        },
    );
    for (name, value) in args {
        s = s.replace(&format!("{{{name}}}"), value);
    }
    s
}
