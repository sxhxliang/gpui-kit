//! Optional frosted-glass text treatment for the editor.
//!
//! Extracted from abstract's `glass` module: only the pieces the editor
//! reads. When the host installs a [`Glass`] global with `enabled: true`,
//! editor text gets an alpha multiplier and (optionally) a contrast nudge
//! toward the palette foreground so it stays readable over translucent
//! window materials. With no global installed, output is identical to
//! plain opaque rendering.

use gpui::Global;

/// Resolved glass configuration, kept in a global so render sites only
/// read it.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Glass {
    pub enabled: bool,
    /// Editor text opacity 0..=1.
    pub text_opacity: f32,
    /// Push editor text toward the strongest foreground colour.
    pub text_contrast: bool,
}

impl Global for Glass {}

impl Default for Glass {
    fn default() -> Self {
        Self {
            enabled: false,
            text_opacity: 1.,
            text_contrast: false,
        }
    }
}

impl Glass {
    pub fn new(text_opacity: f32, text_contrast: bool) -> Self {
        Self {
            enabled: true,
            text_opacity: text_opacity.clamp(0., 1.),
            text_contrast,
        }
    }

    /// Text alpha multiplier: `text_opacity` when glass is on, else 1.
    pub fn text_alpha(&self) -> f32 {
        if self.enabled { self.text_opacity } else { 1. }
    }

    /// Whether the contrast nudge applies.
    pub fn text_contrast(&self) -> bool {
        self.enabled && self.text_contrast
    }
}

pub fn mix_u32(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0., 1.);
    let ch = |shift: u32| {
        let x = ((a >> shift) & 0xff) as f32;
        let y = ((b >> shift) & 0xff) as f32;
        (x + (y - x) * t).round() as u32
    };
    (ch(16) << 16) | (ch(8) << 8) | ch(0)
}

/// High-contrast text: pull `c` toward the theme foreground, then toward
/// pure black or white (whichever is farther from `bg`), so body and muted
/// text both read clearly over translucent glass.
pub fn contrast_u32(c: u32, fg: u32, bg: u32) -> u32 {
    let luma = |x: u32| {
        let ch = |s: u32| ((x >> s) & 0xff) as f32 / 255.;
        0.2126 * ch(16) + 0.7152 * ch(8) + 0.0722 * ch(0)
    };
    let extreme = if luma(bg) > 0.5 { 0x000000 } else { 0xffffff };
    mix_u32(mix_u32(c, fg, 0.5), extreme, 0.5)
}
