//! The corner radius the agent last reported, for Material Symbol icons.

use gpui::{App, Global};
use openlogi_ui::material::{DEFAULT_THEME_RADIUS_PX, MaterialCorner};

/// The effective UI radius, in pixels.
pub(crate) struct OverlayChrome {
    /// `0` selects sharp icons.
    pub radius_px: u8,
}

impl Global for OverlayChrome {}

/// The corner style to draw now. Before the agent has answered, this is the
/// theme default, which is rounded.
pub(crate) fn corner(cx: &App) -> MaterialCorner {
    let radius = if cx.has_global::<OverlayChrome>() {
        cx.global::<OverlayChrome>().radius_px
    } else {
        DEFAULT_THEME_RADIUS_PX
    };
    MaterialCorner::from_radius(radius)
}

/// Remember `radius_px` and repaint every open indicator.
pub(crate) fn apply(cx: &mut App, radius_px: u8) {
    cx.set_global(OverlayChrome { radius_px });
    cx.refresh_windows();
}
