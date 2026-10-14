//! Editor UI zoom: a single process-wide factor drives the [`z`] metric
//! helper the editor's layout code uses for literal sizes that should grow
//! and shrink with zoom. Runtime coordinates stay `px`.
//!
//! Extracted from abstract's `zoom` module, minus the app-level syncing of
//! the window rem size and the component theme (the host owns those; call
//! [`set_factor`] from the same place).

use std::sync::atomic::{AtomicU32, Ordering};

use gpui::{Pixels, px};

/// Clamps for the zoom factor: 50%–200%.
pub const MIN: f32 = 0.5;
pub const MAX: f32 = 2.0;

static FACTOR: AtomicU32 = AtomicU32::new(1.0f32.to_bits());

/// The current factor: `1.0` is 100%.
pub fn factor() -> f32 {
    f32::from_bits(FACTOR.load(Ordering::Relaxed))
}

/// Set the zoom factor (clamped to 50%–200%). Affects every editor in the
/// process on its next repaint.
pub fn set_factor(f: f32) {
    FACTOR.store(f.clamp(MIN, MAX).to_bits(), Ordering::Relaxed);
}

/// `z(v)` is a zoom-scaled `px(v)`.
pub fn z(v: f32) -> Pixels {
    px(v * factor())
}
