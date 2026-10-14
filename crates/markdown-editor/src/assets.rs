//! Motion constants shared by the editor's animations.
//!
//! Extracted from abstract's `assets` module (only the pieces the editor
//! uses).

/// Standard ease-in duration for editor animations, in milliseconds.
pub(crate) const MOTION_IN_MS: u64 = 160;

pub(crate) fn ease_out_quint(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(5)
}
