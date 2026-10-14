# gpui-markdown-editor

A Typora-style live Markdown editor component for GPUI Kit, extracted from
the [abstract](https://github.com/fireflylabss/abstract) notes app
(Apache-2.0) and decoupled from its application shell.

The editor edits plain Markdown source but renders it in place: headings,
emphasis, `==highlights==`, syntax-highlighted code blocks, pipe tables as
grids, task lists, `> [!note]` callouts, footnotes and math approximations
render inline, and syntax markers conceal themselves until the selection
touches them. Analysis is tree-sitter based; there is no separate preview.

## Usage

```rust
use gpui_markdown_editor::{Changed, MarkdownEditor, MarkdownEditorState};

// Once at startup, alongside gpui_kit::init:
gpui_markdown_editor::bind_keys(cx);

// Per document:
let state = cx.new(MarkdownEditorState::new);
cx.subscribe(&state, |_, state, _: &Changed, cx| {
    let text = state.read(cx).text().to_string();
    // persist `text`
});

// In a view's Render:
div().child(MarkdownEditor::new(&state))
```

## Host contract

The component owns editing and rendering. Everything app-shaped is the
host's job, through these seams:

| Concern | Seam |
| --- | --- |
| Colors | `MarkdownEditorState::set_theme(Palette)` — `DARK` (default) and `LIGHT` ship in `theme`; all fields are public `u32`s, so a host can adapt its own theme. Deliberately a record-like token set, not the component `Theme`. |
| Typefaces | `MarkdownEditorState::set_fonts(Fonts)` (default: Noto Sans / Noto Sans Mono). |
| UI strings | English built in; override per string with `i18n::set_labels(...)`. |
| Zoom | `zoom::set_factor(f)` drives the editor's `z()` metrics (the host still owns window rem size and theme font sizes). |
| Link clicks | `OpenLink` event from the state (wiki-link completion data stays with the host; `wiki_prefix` / `complete_wiki` / `set_completing` are the editor side). |
| Drops & pastes | `Attach` event carrying `attach::Incoming` items; the host writes files and inserts the returned Markdown. |
| Host commands | The context menu dispatches `actions::` actions (`FindInNote`, `SearchSelection`, `InsertImage`, `CopyAsHtml`, `ExportHtml`, `ExportPdf`, namespace `markdown_editor`); bind and handle them in the host. |
| Spell checking | Off until the host installs a `spell::SpellChecker` with `spell::set_spell_checker(...)` and enables it via `set_spell`. No dictionary ships with the component. |
| Math & diagrams | Always on, no host wiring. Inline `$…$` and block `$$…$$` math typeset with RaTeX, and ` ```mermaid ` / ` ```plantuml ` blocks render as diagrams (mermaid-rs-renderer / plantuml-little). ` ```smiles ` blocks render molecule / reaction structures with OMGKit (`render::smiles_svg` / `smiles_figure`), drawn in the theme's bg/fg colors; `set_molecular_formulas(true)` adds a molecular-formula caption under each structure (measured with the system Noto Sans). A block shows its figure while the selection is outside it; entering it reveals the source. Sources that fail to render fall back to the previous display (Unicode math approximation / plain code block) — source text is never hidden without a figure. `render::latex_svg` / `diagram_svg` are public for hosts that want the SVG itself (e.g. for HTML export). Note: plantuml-little's build downloads a prebuilt Graphviz binary on first build (set `GRAPHVIZ_ANYWHERE_DIR` for offline builds). |
| Images | `set_dirs(note_dir, root)` tells the editor where relative image sources resolve; the host loads and caches the images themselves. |
| Glass / translucency | Optional: install a `glass::Glass` global to fade/contrast-boost editor text over translucent window materials. |

## What did not come over from abstract

- The spellbook-based engine, its dictionaries and the user-dictionary
  file (replaced by the `SpellChecker` seam above).
- Vault-backed link resolution, completion candidates, backlink index and
  on-disk relinking (the text-level `links::parse` / `links::retarget`
  did come over).
- Attachment file writing (`attach::Incoming` / `candidates` / `rebase`
  are pure and included; writing bytes to `attachments/` is the host's).
- abstract's settings store, spaces/vault management, HTML/PDF exporters,
  updater, search and sidebar — all application layer.

## Status

Verified against gpui-kit 0.7.3 (GPUI `gpui-pre` 0.3.9): `cargo check -p
gpui-markdown-editor` is clean (0 errors, 0 warnings), `cargo test -p
gpui-markdown-editor` passes 91/91, and the story page (`Markdown Editor`
in the story gallery) compiles. abstract was built against gpui-kit 0.6.1
/ `gpui-pre` 0.3.4; the only drift was one missing `FluentBuilder` import
in `ctx_menu`.
