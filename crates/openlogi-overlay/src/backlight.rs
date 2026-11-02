//! Compact transient keyboard-backlight indicator.

use std::sync::Arc;

use gpui::{
    AnyWindowHandle, App, AppContext as _, Context, FontFeatures, IntoElement, ParentElement,
    Render, Styled, Window, WindowHandle, div, px, size, svg,
};
use openlogi_core::hid::backlight::max_backlight_level;
use openlogi_ipc::BacklightObservation;
use openlogi_ui::action_icons;
use tracing::warn;

use crate::chrome;
use crate::osd;

/// The catalog sentence for the effect key while the backlight is off.
pub(crate) const OFF_HINT_KEY: &str = "keyboard.backlight_effect_while_off";

pub(crate) struct BacklightView {
    pub(crate) observation: BacklightObservation,
}

impl BacklightView {
    pub(crate) fn new(observation: BacklightObservation) -> Self {
        Self { observation }
    }
}

impl Render for BacklightView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (width, height) = card_size(&self.observation);
        let max_level = max_backlight_level(self.observation.levels);
        let corner = chrome::corner(cx);
        let icon = if self.observation.off_hint {
            // The sentence already says the backlight is off.
            action_icons::backlight_level_icon(0, max_level, corner)
        } else {
            action_icons::backlight_level_icon(self.observation.current_level, max_level, corner)
        };
        let card = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(osd::GAP))
            .w(px(width))
            .h(px(height))
            .px(px(osd::PAD))
            .rounded(px(osd::RADIUS))
            .bg(osd::PANEL)
            .text_color(osd::TEXT)
            .font_weight(osd::TITLE)
            .child(
                svg()
                    .path(icon)
                    .size(px(ICON))
                    .flex_none()
                    .text_color(osd::TEXT),
            );
        if self.observation.off_hint {
            card.child(
                div()
                    .min_w_0()
                    .whitespace_nowrap()
                    .child(rust_i18n::t!(OFF_HINT_KEY).into_owned()),
            )
        } else {
            card.child(brightness_bar(relative_level(
                self.observation.current_level,
                max_level,
            )))
            .child(
                div()
                    .flex_none()
                    .text_right()
                    .whitespace_nowrap()
                    .font_features(FontFeatures(Arc::new(vec![("tnum".into(), 1)])))
                    .child(level_label(self.observation.current_level)),
            )
        }
    }
}

/// The level at the end of the row. Zero is the localized word "Off"; every
/// other level is just its number.
fn level_label(level: u8) -> String {
    if level == 0 {
        rust_i18n::t!("common.off").into_owned()
    } else {
        level.to_string()
    }
}

/// One row: the glyph, the bar, and the level. Same chrome as the effect list.
const WIDTH: f32 = 300.0;
const HEIGHT: f32 = 64.0;
/// Wide enough for the English off-hint sentence on one line, beside the icon.
const OFF_HINT_WIDTH: f32 = 640.0;
const OFF_HINT_HEIGHT: f32 = 64.0;
const ICON: f32 = 24.0;

fn brightness_bar(filled: f32) -> gpui::Div {
    div()
        .flex_1()
        .h(px(8.0))
        .rounded(px(4.0))
        .bg(osd::TRACK)
        .child(
            div()
                .h_full()
                .w(gpui::relative(filled))
                .rounded(px(4.0))
                .bg(osd::accent()),
        )
}

/// The largest card the indicator draws: the off hint. A centred card that
/// must not cover the indicator keeps clear of this.
pub(crate) const LARGEST_CARD: (f32, f32) = (OFF_HINT_WIDTH, OFF_HINT_HEIGHT);

pub(crate) fn card_size(observation: &BacklightObservation) -> (f32, f32) {
    if observation.off_hint {
        (OFF_HINT_WIDTH, OFF_HINT_HEIGHT)
    } else {
        (WIDTH, HEIGHT)
    }
}

/// Show `observation`, or close the card when it is no longer visible.
///
/// A later level, including the off hint, replaces the open card in place.
pub(crate) fn present(cx: &mut App, observation: &BacklightObservation) {
    let open: Vec<WindowHandle<BacklightView>> = cx
        .windows()
        .iter()
        .filter_map(AnyWindowHandle::downcast::<BacklightView>)
        .collect();
    if !observation.visible {
        for view in open {
            let _ = view.update(cx, |_, window, _| window.remove_window());
        }
        return;
    }
    let next = card_size(observation);
    // A card that changes size is reopened rather than resized: the platform
    // edge placement is worked out for one size, and resizing in place keeps
    // the top-left, which would shift a wider card off centre.
    let reopen = open.is_empty()
        || open.iter().any(|view| {
            view.read_with(cx, |view, _| card_size(&view.observation) != next)
                .unwrap_or(true)
        });
    if reopen {
        for view in open {
            let _ = view.update(cx, |_, window, _| window.remove_window());
        }
        if let Err(error) = open_card(cx, observation) {
            warn!(%error, "could not open the backlight indicator window");
        }
        return;
    }
    for view in open {
        let observation = observation.clone();
        let _ = view.update(cx, |view, _, cx| {
            view.observation = observation;
            cx.notify();
        });
    }
}

fn relative_level(level: u8, max_level: u8) -> f32 {
    if max_level == 0 {
        0.0
    } else {
        f32::from(level.min(max_level)) / f32::from(max_level)
    }
}

/// Open the indicator on the platform edge of the primary display.
fn open_card(
    cx: &mut App,
    observation: &BacklightObservation,
) -> anyhow::Result<WindowHandle<BacklightView>> {
    let (width, height) = card_size(observation);
    let observation = observation.clone();
    crate::popup::open(
        cx,
        "openlogi-backlight-osd",
        size(px(width), px(height)),
        move |_, cx| cx.new(|_| BacklightView::new(observation.clone())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(level: u8, off_hint: bool) -> BacklightObservation {
        BacklightObservation {
            generation: 1,
            current_level: level,
            levels: 8,
            visible: true,
            off_hint,
            language: None,
        }
    }

    #[test]
    fn brightness_zero_says_off_and_a_level_is_just_the_number() {
        assert_eq!(level_label(0), "Off");
        assert_eq!(level_label(4), "4");
        assert_eq!(rust_i18n::t!("common.off", locale = "de"), "Aus");
    }

    #[test]
    fn highest_reported_level_fills_the_bar() {
        assert_eq!(relative_level(7, max_backlight_level(8)), 1.0);
        assert_eq!(relative_level(0, max_backlight_level(8)), 0.0);
    }

    #[test]
    fn the_off_hint_is_the_brightness_card_at_level_zero() {
        let hint = observation(0, true);
        let level = observation(6, false);
        assert_eq!(card_size(&level), (WIDTH, HEIGHT));
        assert_ne!(card_size(&hint), card_size(&level));
        assert_eq!(
            relative_level(hint.current_level, max_backlight_level(8)),
            0.0
        );
    }

    /// The sentence is specified in English. The other locales carry the same
    /// key; `catalog_parity` holds them to it.
    #[test]
    fn the_off_hint_sentence_is_the_english_wording() {
        assert_eq!(
            rust_i18n::t!(OFF_HINT_KEY, locale = "en"),
            "Backlight is off. Increase brightness to change effects."
        );
    }

    fn open(cx: &mut gpui::TestAppContext) -> Vec<BacklightObservation> {
        cx.update(|cx| {
            cx.windows()
                .iter()
                .filter_map(AnyWindowHandle::downcast::<BacklightView>)
                .filter_map(|window| window.read(cx).ok().map(|view| view.observation.clone()))
                .collect()
        })
    }

    #[gpui::test]
    fn the_off_hint_replaces_the_level_card_in_place(cx: &mut gpui::TestAppContext) {
        let hidden = BacklightObservation {
            visible: false,
            ..observation(0, false)
        };
        cx.update(|cx| present(cx, &hidden));
        assert!(open(cx).is_empty());

        let hint = observation(0, true);
        cx.update(|cx| present(cx, &hint));
        assert_eq!(open(cx), [hint]);

        let level = observation(4, false);
        cx.update(|cx| present(cx, &level));
        cx.run_until_parked();
        assert_eq!(open(cx), [level], "the open card shows the level");

        cx.update(|cx| present(cx, &hidden));
        cx.run_until_parked();
        assert!(open(cx).is_empty());
    }
}
