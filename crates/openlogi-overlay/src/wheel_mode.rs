//! Compact transient wheel-mode indicator, shown when a mouse's wheel
//! switches between free spin and ratchet.
//!
//! The agent decides what shows and for how long
//! ([`openlogi_ipc::WheelModeObservation`]); this module only draws it, as an
//! option card ([`crate::option_row`]): SmartShift, then both modes with the
//! current one in the pill. The words are the ones the settings app's Wheel
//! mode control uses, beside the Toggle SmartShift action's glyph
//! ([`WHEEL_MODE_ICON`]). That glyph is Lucide's, which has one cut, so the
//! card draws the same glyph at every corner radius.

use gpui::{
    AnyWindowHandle, App, AppContext as _, Context, IntoElement, Render, Window, WindowHandle, px,
    size,
};
use openlogi_core::hid::SmartShiftMode;
use openlogi_ui::action_icons::WHEEL_MODE_ICON;
use tracing::warn;

use crate::option_row::{OptionRow, card_width, option_card};
use crate::osd;

/// The modes in the order the card lists them.
const MODES: [SmartShiftMode; 2] = [SmartShiftMode::Free, SmartShiftMode::Ratchet];

/// The catalog key for the name of `mode`.
pub(crate) const fn title_key(mode: SmartShiftMode) -> &'static str {
    match mode {
        SmartShiftMode::Free => "pointer.free_spin",
        SmartShiftMode::Ratchet => "pointer.ratchet",
    }
}

/// What the card says while `mode` is current, in the active locale.
pub(crate) fn row(mode: SmartShiftMode) -> OptionRow {
    OptionRow {
        icon: WHEEL_MODE_ICON,
        label: rust_i18n::t!("pointer.smartshift").into_owned(),
        options: MODES
            .iter()
            .map(|mode| rust_i18n::t!(title_key(*mode)).into_owned())
            .collect(),
        current: MODES.iter().position(|each| *each == mode).unwrap_or(0),
    }
}

/// The indicator window's view: the mode the agent is showing.
pub(crate) struct WheelModeView {
    mode: SmartShiftMode,
    /// The window's width, measured when it opened.
    width: f32,
}

impl Render for WheelModeView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        option_card(&row(self.mode), self.width)
    }
}

/// Show `shown`, or close the indicator when it is `None`.
///
/// A switch while the window is open replaces its mode in place, so presses
/// in quick succession do not flash the window closed and open again. Both
/// modes are always listed, so the width only moves with the language.
pub(crate) fn present(cx: &mut App, shown: Option<SmartShiftMode>) {
    let open: Vec<WindowHandle<WheelModeView>> = cx
        .windows()
        .iter()
        .filter_map(AnyWindowHandle::downcast::<WheelModeView>)
        .collect();
    let Some(mode) = shown else {
        for view in open {
            let _ = view.update(cx, |_, window, _| window.remove_window());
        }
        return;
    };
    let width = card_width(cx, &row(mode));
    // A card that needs another width is reopened rather than resized:
    // resizing in place keeps the top-left, so the card would drift.
    let reopen = open.is_empty()
        || open.iter().any(|view| {
            view.read_with(cx, |view, _| (view.width - width).abs() > f32::EPSILON)
                .unwrap_or(true)
        });
    if !reopen {
        for view in open {
            let _ = view.update(cx, |view, _, cx| {
                view.mode = mode;
                cx.notify();
            });
        }
        return;
    }
    for view in open {
        let _ = view.update(cx, |_, window, _| window.remove_window());
    }
    if let Err(error) = crate::popup::open(
        cx,
        "openlogi-wheel-mode-osd",
        size(px(width), px(osd::LOCK_HEIGHT)),
        move |_, cx| cx.new(|_| WheelModeView { mode, width }),
    ) {
        warn!(%error, "could not open the wheel-mode indicator window");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_card_lists_both_modes_and_marks_the_current_one() {
        let free = row(SmartShiftMode::Free);
        assert_eq!(free.label, "SmartShift");
        assert_eq!(free.options, ["Free spin", "Ratchet"]);
        assert_eq!(free.current, 0);
        assert_eq!(row(SmartShiftMode::Ratchet).current, 1);
        assert_eq!(free.icon, WHEEL_MODE_ICON);
    }

    #[test]
    fn each_mode_is_named_from_the_catalog() {
        for mode in MODES {
            let key = title_key(mode);
            assert_ne!(rust_i18n::t!(key, locale = "de"), key);
        }
        assert_ne!(
            rust_i18n::t!("pointer.smartshift", locale = "de"),
            "pointer.smartshift"
        );
    }

    /// The mode each open indicator window shows.
    fn shown(cx: &mut gpui::TestAppContext) -> Vec<SmartShiftMode> {
        cx.update(|cx| {
            cx.windows()
                .iter()
                .filter_map(AnyWindowHandle::downcast::<WheelModeView>)
                .filter_map(|window| window.read(cx).ok().map(|view| view.mode))
                .collect()
        })
    }

    #[gpui::test]
    fn one_window_shows_each_switch_in_place_until_nothing_shows(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| present(cx, None));
        assert!(shown(cx).is_empty(), "nothing showing opens nothing");

        cx.update(|cx| present(cx, Some(SmartShiftMode::Free)));
        assert_eq!(shown(cx), [SmartShiftMode::Free]);

        cx.update(|cx| present(cx, Some(SmartShiftMode::Ratchet)));
        cx.run_until_parked();
        assert_eq!(
            shown(cx),
            [SmartShiftMode::Ratchet],
            "the open window shows the newer mode"
        );

        cx.update(|cx| present(cx, None));
        cx.run_until_parked();
        assert!(shown(cx).is_empty(), "nothing showing closes the window");
    }
}
