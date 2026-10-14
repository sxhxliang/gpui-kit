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
        Key::AddLink => "Adicionar link externo",
        Key::AddWikiLink => "Adicionar link de nota",
        Key::Bold => "Negrito",
        Key::BulletList => "Lista com marcadores",
        Key::Callout => "Destaque",
        Key::CodeBlock => "Bloco de código",
        Key::Comment => "Comentário oculto",
        Key::Copy => "Copiar",
        Key::CopyAsHtml => "Copiar como HTML",
        Key::Cut => "Recortar",
        Key::Divider => "Divisória",
        Key::EditorAria => "Editor Markdown",
        Key::EditorPlaceholder => "Comece a escrever…",
        Key::ExportAsHtml => "Exportar como HTML…",
        Key::ExportAsPdf => "Exportar como PDF…",
        Key::FindInNote => "Buscar na nota…",
        Key::Footnote => "Nota de rodapé",
        Key::Format => "Formatar",
        Key::Heading1 => "Título 1",
        Key::Heading2 => "Título 2",
        Key::Heading3 => "Título 3",
        Key::Highlight => "Marca-texto",
        Key::HighlightColor => "Cor do marca-texto",
        Key::InlineCode => "Código",
        Key::Insert => "Inserir",
        Key::InsertImage => "Imagem…",
        Key::Italic => "Itálico",
        Key::MathBlock => "Math em bloco",
        Key::MathInline => "Math inline",
        Key::NumberedList => "Lista numerada",
        Key::Paragraph => "Parágrafo",
        Key::Paste => "Colar",
        Key::PastePlain => "Colar como texto simples",
        Key::PlainText => "Texto normal",
        Key::Quote => "Citação",
        Key::SearchSelection => "Buscar pela seleção",
        Key::SelectAll => "Selecionar tudo",
        Key::SpellAddWord => "Adicionar ao dicionário",
        Key::Strikethrough => "Tachado",
        Key::Table => "Tabela",
        Key::TableColumn => "Coluna {n}",
        Key::TableDeleteColumn => "Apagar coluna",
        Key::TableDeleteRow => "Apagar linha",
        Key::TableInsertColumnLeft => "Inserir coluna à esquerda",
        Key::TableInsertColumnRight => "Inserir coluna à direita",
        Key::TableInsertRowAbove => "Inserir linha acima",
        Key::TableInsertRowBelow => "Inserir linha abaixo",
        Key::TaskList => "Lista de tarefas",
        Key::TintBlue => "Azul",
        Key::TintGreen => "Verde",
        Key::TintOrange => "Laranja",
        Key::TintPurple => "Roxo",
        Key::TintRed => "Vermelho",
        Key::TintYellow => "Amarelo",
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
