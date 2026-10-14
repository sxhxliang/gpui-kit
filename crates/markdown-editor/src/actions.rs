//! Host-level commands the editor's context menu dispatches.
//!
//! In abstract these lived in the app's keymap (namespace `abstract_app`);
//! the component owns them now (namespace `markdown_editor`). The editor
//! only *dispatches* them from its menus — binding keys and handling them
//! is the host's job, e.g.:
//!
//! - `FindInNote` — open the find bar for the focused editor
//! - `SearchSelection` — search the workspace for the selected text
//! - `InsertImage` — pick an image file and insert it
//! - `CopyAsHtml` / `ExportHtml` / `ExportPdf` — convert the document
//!   (the host owns conversion; the editor's text is on the state)

use gpui_kit::*;

actions!(
    markdown_editor,
    [
        SearchSelection,
        InsertImage,
        FindInNote,
        CopyAsHtml,
        ExportHtml,
        ExportPdf,
    ]
);
