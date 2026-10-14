//! Source → SVG → rasterized [`Figure`] rendering for math and diagrams.
//!
//! LaTeX goes through RaTeX (`ratex_parser` → `ratex_layout` → `ratex_svg`,
//! glyphs embedded as paths so rasterization never needs KaTeX fonts);
//! Mermaid and PlantUML diagrams go through their pure-Rust renderers. Every
//! producer emits an SVG string; the SVG is parsed once and rasterized with
//! resvg into a [`gpui_kit::RenderImage`], which the editor can paint with its
//! existing image path.
//!
//! This module is deliberately self-contained: it owns no editor state and
//! touches no editor internals. The caller decides when to render (e.g. only
//! for blocks the caret isn't inside) and caches [`Figure`]s keyed by
//! [`cache_key`]; [`latex_svg`]/[`diagram_svg`] return `None` on any failure
//! so callers can fall back to source text.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock};

mod smiles;

/// Which diagram renderer produced a figure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DiagramKind {
    Mermaid,
    PlantUml,
}

/// A rasterized figure plus its logical display size in pt.
#[derive(Clone)]
pub struct Figure {
    pub image: Arc<gpui_kit::RenderImage>,
    /// Display width, in pt.
    pub width: f32,
    /// Display height, in pt.
    pub height: f32,
}

/// User units per em in the SVGs [`latex_svg`] emits (`SvgOptions::font_size`).
const LATEX_FONT_SIZE: f64 = 40.0;
/// Padding around LaTeX output, in SVG user units.
const LATEX_PADDING: f64 = 6.0;
/// Upper bound on either rasterized dimension, in device pixels.
const MAX_RASTER_PX: u32 = 8192;

/// System font database for `<text>` in diagram SVGs, loaded once.
fn fontdb() -> Arc<usvg::fontdb::Database> {
    static DB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = usvg::fontdb::Database::new();
        db.load_system_fonts();
        Arc::new(db)
    })
    .clone()
}

fn svg_options() -> usvg::Options<'static> {
    let mut opt = usvg::Options::default();
    opt.fontdb = fontdb();
    opt
}

/// LaTeX source → standalone SVG string.
///
/// `fg` is a `0xRRGGBB` foreground color; `display` selects Display style,
/// otherwise Text style. Returns `None` when the source is empty or
/// parse/layout fails.
pub fn latex_svg(src: &str, display: bool, fg: u32) -> Option<String> {
    if src.trim().is_empty() {
        return None;
    }
    let nodes = ratex_parser::parse(src).ok()?;
    let layout_opts = ratex_layout::LayoutOptions {
        style: if display {
            ratex_types::MathStyle::Display
        } else {
            ratex_types::MathStyle::Text
        },
        color: ratex_types::Color {
            r: ((fg >> 16) & 0xff) as f32 / 255.0,
            g: ((fg >> 8) & 0xff) as f32 / 255.0,
            b: (fg & 0xff) as f32 / 255.0,
            a: 1.0,
        },
        ..Default::default()
    };
    let layout_box = ratex_layout::layout(&nodes, &layout_opts);
    let list = ratex_layout::to_display_list(&layout_box);
    // `embed-fonts` turns on `standalone`: glyphs are emitted as paths, so
    // rasterization needs no KaTeX fonts at runtime.
    let svg_opts = ratex_svg::SvgOptions {
        font_size: LATEX_FONT_SIZE,
        padding: LATEX_PADDING,
        embed_glyphs: true,
        ..Default::default()
    };
    Some(ratex_svg::render_to_svg(&list, &svg_opts))
}

/// Diagram source → SVG string. Returns `None` when the source is empty or
/// the renderer rejects it.
pub fn diagram_svg(kind: DiagramKind, src: &str) -> Option<String> {
    if src.trim().is_empty() {
        return None;
    }
    match kind {
        DiagramKind::Mermaid => mermaid_rs_renderer::render(src).ok(),
        DiagramKind::PlantUml => plantuml_little::convert(src).ok(),
    }
}

/// Rasterize an already-parsed SVG tree. `pt_scale` converts SVG user units
/// to pt; `raster_scale` converts user units to device pixels.
fn rasterize_tree(tree: &usvg::Tree, pt_scale: f64, raster_scale: f64) -> Option<Figure> {
    if !(pt_scale > 0.0) || !(raster_scale > 0.0) {
        return None;
    }
    let size = tree.size();
    let (uw, uh) = (f64::from(size.width()), f64::from(size.height()));
    if uw <= 0.0 || uh <= 0.0 || !uw.is_finite() || !uh.is_finite() {
        return None;
    }
    // Clamp the raster scale so no dimension exceeds MAX_RASTER_PX.
    let mut s = raster_scale;
    let max_px = uw.max(uh) * s;
    if max_px > f64::from(MAX_RASTER_PX) {
        s *= f64::from(MAX_RASTER_PX) / max_px;
    }
    let w_px = (uw * s).round().max(1.0) as u32;
    let h_px = (uh * s).round().max(1.0) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(w_px, h_px)?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(s as f32, s as f32),
        &mut pixmap.as_mut(),
    );
    // GPUI's image pipeline expects RGBA with premultiplied alpha, which is
    // exactly what tiny-skia's pixmap holds.
    let rgba = image::RgbaImage::from_raw(w_px, h_px, pixmap.take())?;
    let frame = image::Frame::new(rgba);
    Some(Figure {
        image: Arc::new(gpui_kit::RenderImage::new(vec![frame])),
        width: (uw * pt_scale) as f32,
        height: (uh * pt_scale) as f32,
    })
}

/// Parse and rasterize an SVG string.
fn rasterize(svg: &str, pt_scale: f64, raster_scale: f64) -> Option<Figure> {
    let tree = usvg::Tree::from_str(svg, &svg_options()).ok()?;
    rasterize_tree(&tree, pt_scale, raster_scale)
}

/// LaTeX, one step: typeset at `em_pt` pt and rasterize at `scale` device
/// pixels per pt. The returned pt size honors the `em_pt` sizing.
pub fn latex_figure(src: &str, display: bool, fg: u32, em_pt: f32, scale: f32) -> Option<Figure> {
    let svg = latex_svg(src, display, fg)?;
    // 1 em == SvgOptions::font_size user units.
    let unit = f64::from(em_pt) / LATEX_FONT_SIZE;
    rasterize(&svg, unit, unit * f64::from(scale))
}

/// Diagram, one step: `target_w` is the desired display width in pt
/// (diagram SVGs are already in pt units). The figure is fitted into
/// `target_w` preserving aspect ratio — width is `min(natural, target_w)` —
/// and rasterized at `scale` device pixels per pt of the fitted size.
pub fn diagram_figure(kind: DiagramKind, src: &str, target_w: f32, scale: f32) -> Option<Figure> {
    let svg = diagram_svg(kind, src)?;
    let tree = usvg::Tree::from_str(&svg, &svg_options()).ok()?;
    let size = tree.size();
    let (w, h) = (size.width(), size.height());
    if w <= 0.0 || h <= 0.0 || !w.is_finite() || !h.is_finite() {
        return None;
    }
    let fit = f64::from((target_w / w).min(1.0));
    rasterize_tree(&tree, fit, fit * f64::from(scale))
}

/// A SMILES / reaction-SMILES source as an SVG document plus its display
/// size in pt, drawn in the given theme colors; `formulas` adds a
/// molecular-formula caption under each structure. Returns `None` for
/// anything that isn't one complete, renderable molecule or reaction —
/// the caller then keeps the source visible.
pub fn smiles_svg(
    src: &str,
    background: u32,
    foreground: u32,
    formulas: bool,
) -> Option<(String, gpui_kit::Size<f32>)> {
    smiles::render_with_formulas(src, background, foreground, formulas)
}

/// SMILES, one step: fitted into `target_w` like [`diagram_figure`].
pub fn smiles_figure(
    src: &str,
    background: u32,
    foreground: u32,
    formulas: bool,
    target_w: f32,
    scale: f32,
) -> Option<Figure> {
    let (svg, _) = smiles_svg(src, background, foreground, formulas)?;
    let tree = usvg::Tree::from_str(&svg, &svg_options()).ok()?;
    let size = tree.size();
    let (w, h) = (size.width(), size.height());
    if w <= 0.0 || h <= 0.0 || !w.is_finite() || !h.is_finite() {
        return None;
    }
    let fit = f64::from((target_w / w).min(1.0));
    rasterize_tree(&tree, fit, fit * f64::from(scale))
}

/// Cache key over the render inputs: a `tag` for the producer (e.g. 0 for
/// LaTeX display, 1 for inline, 2/3 for diagrams), the source, the foreground
/// color, and the width bucket (pt, rounded down by the caller).
pub fn cache_key(tag: u8, src: &str, fg: u32, width_bucket: u32) -> u64 {
    let mut h = DefaultHasher::new();
    tag.hash(&mut h);
    src.hash(&mut h);
    fg.hash(&mut h);
    width_bucket.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latex_svg_emits_paths() {
        let svg = latex_svg("\\frac{a}{b}", true, 0).expect("latex renders");
        assert!(svg.contains("<svg"), "not an svg document");
        assert!(svg.contains("<path"), "glyphs should be embedded as paths");
    }

    #[test]
    fn latex_figure_sizes() {
        let fig = latex_figure("\\frac{a}{b}", true, 0, 16.0, 2.0).expect("figure");
        assert!(fig.width > 0.0 && fig.height > 0.0);
        let px = fig.image.size(0);
        let (pw, ph) = (px.width.0, px.height.0);
        assert!(
            (f64::from(pw) - f64::from(fig.width) * 2.0).abs() <= 2.0,
            "pixel width {pw} not ~= pt {} x 2",
            fig.width
        );
        assert!(
            (f64::from(ph) - f64::from(fig.height) * 2.0).abs() <= 2.0,
            "pixel height {ph} not ~= pt {} x 2",
            fig.height
        );
    }

    #[test]
    fn mermaid_renders() {
        let fig = diagram_figure(DiagramKind::Mermaid, "graph TD; A-->B", 400.0, 1.0)
            .expect("mermaid renders");
        assert!(fig.width > 0.0 && fig.height > 0.0);
        assert!(fig.width <= 400.0);
    }

    #[test]
    fn plantuml_renders() {
        let fig = diagram_figure(
            DiagramKind::PlantUml,
            "@startuml\nAlice -> Bob: hello\n@enduml",
            400.0,
            1.0,
        )
        .expect("plantuml renders");
        assert!(fig.width > 0.0 && fig.height > 0.0);
    }

    #[test]
    fn mermaid_rejects_garbage() {
        assert!(diagram_svg(DiagramKind::Mermaid, "这不是 mermaid 源码 !!!").is_none());
    }

    #[test]
    fn cache_key_stable_and_distinct() {
        let a = cache_key(0, "x^2", 0xff_ff_ff, 100);
        assert_eq!(a, cache_key(0, "x^2", 0xff_ff_ff, 100));
        assert_ne!(a, cache_key(0, "x^3", 0xff_ff_ff, 100));
        assert_ne!(a, cache_key(0, "x^2", 0x00_00_00, 100));
        assert_ne!(a, cache_key(1, "x^2", 0xff_ff_ff, 100));
        assert_ne!(a, cache_key(0, "x^2", 0xff_ff_ff, 200));
    }
}
