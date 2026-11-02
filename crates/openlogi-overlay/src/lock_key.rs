//! Compact transient lock-key indicator: "Caps Lock On" and the like, shown
//! when a keyboard reports one of its lock keys turning on or off.
//!
//! Caps Lock, Num Lock, and Scroll Lock share the Caps Lock badge: filled
//! while the key is on, outlined while it is off. Material Symbols has no
//! num-lock or scroll-lock glyph, and the label still names the key.
//!
//! The agent decides what shows and for how long
//! ([`openlogi_ipc::LockKeyObservation`]); this module only draws it. The window
//! opens with the first change, shows each later change in place, and closes
//! when the agent reports nothing showing.

use gpui::{
    AnyWindowHandle, App, AppContext as _, Context, Hsla, IntoElement, ParentElement, Render,
    Styled, Window, WindowHandle, div, px, size, svg,
};
use openlogi_core::device::LockKey;
use openlogi_ipc::LockKeyChange;
use openlogi_ui::action_icons::lock_key_icon;
use tracing::warn;

use crate::chrome;
use crate::osd;

/// Wide enough for the longest translated title on one line.
const WIDTH: f32 = 360.0;
const HEIGHT: f32 = 72.0;

/// The catalog key for the text that announces `change`.
pub(crate) const fn title_key(change: LockKeyChange) -> &'static str {
    match (change.key, change.engaged) {
        (LockKey::CapsLock, true) => "keyboard.caps_lock_on",
        (LockKey::CapsLock, false) => "keyboard.caps_lock_off",
        (LockKey::NumLock, true) => "keyboard.num_lock_on",
        (LockKey::NumLock, false) => "keyboard.num_lock_off",
        (LockKey::ScrollLock, true) => "keyboard.scroll_lock_on",
        (LockKey::ScrollLock, false) => "keyboard.scroll_lock_off",
    }
}

/// The colour of the glyph for a key that is now `engaged` or not.
fn glyph_colour(engaged: bool) -> Hsla {
    if engaged { osd::accent() } else { osd::MUTED }
}

/// The indicator window's view: the latest change the agent is showing.
pub(crate) struct LockKeyView {
    change: LockKeyChange,
}

impl Render for LockKeyView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .gap(px(osd::GAP))
            .w(px(WIDTH))
            .h(px(HEIGHT))
            .px(px(osd::PAD))
            .rounded(px(osd::RADIUS))
            .bg(osd::PANEL)
            .text_color(osd::TEXT)
            .child(
                svg()
                    .path(lock_key_icon(self.change.engaged, chrome::corner(cx)))
                    .size(px(24.0))
                    .flex_none()
                    .text_color(glyph_colour(self.change.engaged)),
            )
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .font_weight(osd::TITLE)
                    .child(rust_i18n::t!(title_key(self.change)).into_owned()),
            )
    }
}

/// Show `shown`, or close the indicator when it is `None`.
///
/// A change while the window is open replaces its text in place, so presses
/// in quick succession do not flash the window closed and open again.
pub(crate) fn present(cx: &mut App, shown: Option<LockKeyChange>) {
    let open: Vec<WindowHandle<LockKeyView>> = cx
        .windows()
        .iter()
        .filter_map(AnyWindowHandle::downcast::<LockKeyView>)
        .collect();
    let Some(change) = shown else {
        for view in open {
            let _ = view.update(cx, |_, window, _| window.remove_window());
        }
        return;
    };
    if open.is_empty() {
        if let Err(error) = crate::popup::open(
            cx,
            "openlogi-lock-key-osd",
            size(px(WIDTH), px(HEIGHT)),
            move |_, cx| cx.new(|_| LockKeyView { change }),
        ) {
            warn!(%error, "could not open the lock-key indicator window");
        }
        return;
    }
    for view in open {
        let _ = view.update(cx, |view, _, cx| {
            view.change = change;
            cx.notify();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn change(key: LockKey, engaged: bool) -> LockKeyChange {
        LockKeyChange { key, engaged }
    }

    #[test]
    fn each_change_announces_its_own_key_and_state() {
        let expected = [
            (change(LockKey::CapsLock, true), "keyboard.caps_lock_on"),
            (change(LockKey::CapsLock, false), "keyboard.caps_lock_off"),
            (change(LockKey::NumLock, true), "keyboard.num_lock_on"),
            (change(LockKey::NumLock, false), "keyboard.num_lock_off"),
            (change(LockKey::ScrollLock, true), "keyboard.scroll_lock_on"),
            (
                change(LockKey::ScrollLock, false),
                "keyboard.scroll_lock_off",
            ),
        ];
        // Exhaustive on purpose: a new lock key stops this compiling until it
        // is listed above.
        let _: fn(LockKey) = |key| match key {
            LockKey::CapsLock | LockKey::NumLock | LockKey::ScrollLock => {}
        };
        for (change, key) in expected {
            assert_eq!(title_key(change), key, "{change:?}");
        }
    }

    /// A key missing from the catalog would show the raw key on screen. The
    /// other locales are held to the same keys by `catalog_parity`.
    #[test]
    fn every_announcement_is_in_the_catalog() {
        for key in [LockKey::CapsLock, LockKey::NumLock, LockKey::ScrollLock] {
            for engaged in [true, false] {
                let title = title_key(change(key, engaged));
                assert_ne!(rust_i18n::t!(title, locale = "en"), title);
            }
        }
    }

    #[test]
    fn a_key_turning_on_is_drawn_apart_from_one_turning_off() {
        assert_eq!(glyph_colour(true), osd::accent());
        assert_eq!(glyph_colour(false), osd::MUTED);
        assert_ne!(osd::accent(), osd::MUTED);
    }

    /// The change each open indicator window shows.
    fn shown(cx: &mut gpui::TestAppContext) -> Vec<LockKeyChange> {
        cx.update(|cx| {
            cx.windows()
                .iter()
                .filter_map(AnyWindowHandle::downcast::<LockKeyView>)
                .filter_map(|window| window.read(cx).ok().map(|view| view.change))
                .collect()
        })
    }

    #[gpui::test]
    fn one_window_shows_each_change_in_place_until_nothing_shows(cx: &mut gpui::TestAppContext) {
        let on = change(LockKey::CapsLock, true);
        let off = change(LockKey::CapsLock, false);

        cx.update(|cx| present(cx, None));
        assert!(shown(cx).is_empty(), "nothing showing opens nothing");

        cx.update(|cx| present(cx, Some(on)));
        assert_eq!(shown(cx), [on]);

        cx.update(|cx| present(cx, Some(off)));
        cx.run_until_parked();
        assert_eq!(shown(cx), [off], "the open window shows the newer change");

        cx.update(|cx| present(cx, None));
        cx.run_until_parked();
        assert!(shown(cx).is_empty(), "nothing showing closes the window");
    }
}
