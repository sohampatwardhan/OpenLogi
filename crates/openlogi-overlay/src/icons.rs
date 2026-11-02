//! Which Material Symbol cut the popups draw.
//!
//! The Appearance corner radius decides. A radius of 0 is the sharp cut;
//! every other radius is the rounded cut. The agent owns that radius: it
//! starts from `ui_radius` in `config.toml`, republishes it on a config
//! reload, and relays the effective radius the desktop applies (a theme's own
//! radius included) through [`openlogi_ipc::UiChromeObservation`]. The
//! overlay only applies what it observes, so every popup — brightness, the
//! effect list, lock keys, charging, and notices — draws the same cut.

use gpui::{App, Global};
use openlogi_ui::action_icons::MaterialSymbolStyle;

/// The cut the open popups draw. Missing until the agent's first answer or
/// a preview sets it, in which case the popups use the rounded cut: the
/// theme default radius is not 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PopupIcons(pub(crate) MaterialSymbolStyle);

impl Global for PopupIcons {}

/// The cut to draw now.
pub(crate) fn style(cx: &App) -> MaterialSymbolStyle {
    cx.try_global::<PopupIcons>()
        .map_or(MaterialSymbolStyle::Rounded, |icons| icons.0)
}

/// The cut for an effective corner radius in pixels.
#[must_use]
pub(crate) const fn style_for_radius(radius_px: u8) -> MaterialSymbolStyle {
    MaterialSymbolStyle::from_radius_px(radius_px)
}

/// Remember the cut for `radius_px` and repaint every open popup when it
/// changed.
pub(crate) fn apply_radius(cx: &mut App, radius_px: u8) {
    let next = style_for_radius(radius_px);
    if cx.try_global::<PopupIcons>().map(|icons| icons.0) == Some(next) {
        return;
    }
    cx.set_global(PopupIcons(next));
    cx.refresh_windows();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_sharp_and_every_other_radius_is_rounded() {
        assert_eq!(style_for_radius(0), MaterialSymbolStyle::Sharp);
        for radius in [1, 6, 12] {
            assert_eq!(style_for_radius(radius), MaterialSymbolStyle::Rounded);
        }
    }
}
