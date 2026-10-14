//! Editor color palette: the 28 colors the live editor paints with.
//!
//! Extracted from abstract's `theme` module. The host picks a palette with
//! `MarkdownEditorState::set_theme`; [`DARK`] is the default. Every field is
//! a `0xRRGGBB` (or `0xRRGGBBAA` where noted) `u32`, matching abstract.

use gpui::Global;

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Palette {
    pub bg: u32,
    pub panel: u32,
    pub hover: u32,
    pub active: u32,
    pub line: u32,
    pub fg: u32,
    pub body: u32,
    pub dim: u32,
    pub faint: u32,
    pub menu_bg: u32,
    pub menu_border: u32,
    pub frame_border: u32,
    pub head: u32,
    pub quote: u32,
    pub mark: u32,
    pub muted: u32,
    pub code_bg: u32,
    pub code_kw: u32,
    pub code_str: u32,
    pub code_comment: u32,
    pub code_num: u32,
    pub inline_code_bg: u32,
    pub rule: u32,
    pub caret: u32,
    /// rgba, alpha included.
    pub selection: u32,
    /// `==marked==` text background, rgba.
    pub highlight: u32,
    /// Active find match background, rgba.
    pub find_current: u32,
    /// Callout accents by `md::Tone`: note, tip, warning, danger.
    pub callout: [u32; 4],
    /// `=={tint}…==` backgrounds by `md::Tint`, rgba — the palette's accent
    /// hues at the highlight's alpha.
    pub marks: [u32; 6],
}

impl Global for Palette {}

impl Default for Palette {
    fn default() -> Self {
        DARK
    }
}

/// Pure monochrome, dark (abstract's default dark palette).
pub const DARK: Palette = Palette {
    bg: 0x0a0a0a,
    panel: 0x0f0f0f,
    hover: 0x1a1a1a,
    active: 0x222222,
    line: 0x1f1f1f,
    fg: 0xededed,
    body: 0xd4d4d4,
    dim: 0x8a8a8a,
    faint: 0x555555,
    menu_bg: 0x141414,
    menu_border: 0x2a2a2a,
    frame_border: 0x262626,
    head: 0xffffff,
    quote: 0x9a9a9a,
    mark: 0x4d4d4d,
    muted: 0x7a7a7a,
    code_bg: 0x161616,
    code_kw: 0xededed,
    code_str: 0xb8b8b8,
    code_comment: 0x6a6a6a,
    code_num: 0xcfcfcf,
    inline_code_bg: 0x1f1f1f,
    rule: 0x2e2e2e,
    caret: 0xf2f2f2,
    selection: 0xffffff26,
    highlight: 0xe5c07b40,
    find_current: 0xe5a13bb3,
    callout: [0x6ea8fe, 0x5fc88f, 0xe5b454, 0xf0736b],
    marks: [
        0xf0736b40, 0xd19a6640, 0xe5c07b40, 0x5fc88f40, 0x6ea8fe40, 0xb392f040,
    ],
};

/// Light palette.
pub const LIGHT: Palette = Palette {
    bg: 0xfafafa,
    panel: 0xf3f3f2,
    hover: 0xe9e9e7,
    active: 0xe0e0de,
    line: 0xe4e4e2,
    fg: 0x111111,
    body: 0x2b2b2b,
    dim: 0x6b6b6b,
    faint: 0x9a9a9a,
    menu_bg: 0xffffff,
    menu_border: 0xdedede,
    frame_border: 0xd6d6d6,
    head: 0x000000,
    quote: 0x5c5c5c,
    mark: 0xb5b5b5,
    muted: 0x777777,
    code_bg: 0xf0f0ee,
    code_kw: 0x111111,
    code_str: 0x4a4a4a,
    code_comment: 0x9a9a9a,
    code_num: 0x333333,
    inline_code_bg: 0xe8e8e6,
    rule: 0xdcdcda,
    caret: 0x111111,
    selection: 0x0000001f,
    highlight: 0xffd84a80,
    find_current: 0xff9f1ab3,
    callout: [0x2f6fd6, 0x1f8a52, 0xa66a00, 0xc4372f],
    marks: [
        0xc4372f80, 0xd9770680, 0xffd84a80, 0x1f8a5280, 0x2f6fd680, 0x7c3aed80,
    ],
};
