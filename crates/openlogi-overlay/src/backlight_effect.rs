//! Compact transient backlight-effect indicator: "Breathing" and the like,
//! shown when a keyboard switches effect without changing brightness.
//!
//! The agent decides what shows and for how long
//! ([`openlogi_ipc::BacklightEffectObservation`]); this module only draws it.
//! The window opens with the first effect, shows each later one in place, and
//! closes when the agent reports nothing showing.

use gpui::{
    AnyWindowHandle, App, AppContext as _, Context, IntoElement, ParentElement, Render, Styled,
    Window, WindowHandle, div, px, size, svg,
};
use openlogi_core::hid::BacklightEffect;
use openlogi_ui::action_icons::backlight_effect_icon;
use tracing::warn;

use crate::osd;

/// The narrowest the list gets, so a short cycle still reads as a card.
const MIN_WIDTH: f32 = 160.0;
/// The widest the list grows, the lock-key card's width: a longer
/// translated name than this still fits on one line there.
const MAX_WIDTH: f32 = 360.0;
/// Room each side of a name inside its row, so the current effect's pill
/// does not hug the text.
const PILL_PAD: f32 = 16.0;
/// One row. Seven of these, plus the card padding, still sit in the centre.
const ROW: f32 = 36.0;
/// How many rows the card shows before it keeps only a window around the
/// current effect. Seven is every `0x1982` effect, so a real keyboard fits.
const MAX_ROWS: usize = 7;

/// The catalog key for the name of `effect`.
pub(crate) const fn title_key(effect: BacklightEffect) -> &'static str {
    match effect {
        BacklightEffect::Static => "keyboard.backlight_effect_static",
        BacklightEffect::None => "keyboard.backlight_effect_none",
        BacklightEffect::Breathing => "keyboard.backlight_effect_breathing",
        BacklightEffect::Contrast => "keyboard.backlight_effect_contrast",
        BacklightEffect::Reaction => "keyboard.backlight_effect_reaction",
        BacklightEffect::Random => "keyboard.backlight_effect_random",
        BacklightEffect::Waves => "keyboard.backlight_effect_waves",
    }
}

/// Rows to draw: the whole cycle when it fits, otherwise a window of
/// [`MAX_ROWS`] that still contains `current`.
pub(crate) fn visible_rows(
    effects: &[BacklightEffect],
    current: BacklightEffect,
) -> &[BacklightEffect] {
    if effects.len() <= MAX_ROWS {
        return effects;
    }
    let index = effects
        .iter()
        .position(|effect| *effect == current)
        .unwrap_or(0);
    let start = index
        .saturating_sub(MAX_ROWS / 2)
        .min(effects.len() - MAX_ROWS);
    &effects[start..start + MAX_ROWS]
}

/// The card's height for `rows` names.
fn card_height(rows: usize) -> f32 {
    let rows = f32::from(u8::try_from(rows.max(1)).unwrap_or(u8::MAX));
    osd::PAD * 2.0 + ICON + osd::GAP + ROW * rows + osd::GAP * (rows - 1.0).max(0.0)
}

const ICON: f32 = 24.0;

/// The rows the list draws for `effect` in `supported`.
fn cycle_rows(effect: BacklightEffect, supported: &[BacklightEffect]) -> Vec<BacklightEffect> {
    // None is not a cycle step. An unknown list names only the reported
    // effect, and only when that effect is one the key can select.
    let cycle = if supported.is_empty() {
        if effect == BacklightEffect::None {
            Vec::new()
        } else {
            vec![effect]
        }
    } else {
        supported
            .iter()
            .copied()
            .filter(|effect| *effect != BacklightEffect::None)
            .collect()
    };
    if cycle.is_empty() {
        return cycle;
    }
    visible_rows(&cycle, effect).to_vec()
}

/// The list width that fits a name `widest` pixels wide in its pill, within
/// [`MIN_WIDTH`]..=[`MAX_WIDTH`].
pub(crate) fn fit_list_width(widest: f32) -> f32 {
    (osd::PAD * 2.0 + PILL_PAD * 2.0 + widest + osd::TEXT_SLACK).clamp(MIN_WIDTH, MAX_WIDTH)
}

/// The width of the list for `effect` in `supported`: its longest visible
/// name, measured in the title weight the rows draw with.
fn list_width(cx: &App, effect: BacklightEffect, supported: &[BacklightEffect]) -> f32 {
    let widest = cycle_rows(effect, supported)
        .into_iter()
        .map(|row| osd::text_width(cx, &rust_i18n::t!(title_key(row)), osd::TITLE))
        .fold(0.0, f32::max);
    fit_list_width(widest)
}

/// The indicator window's view: the cycle, with the current effect in a pill.
pub(crate) struct BacklightEffectView {
    effect: BacklightEffect,
    supported: Vec<BacklightEffect>,
    /// The window's width, which [`list_width`] chose when it opened.
    width: f32,
}

impl BacklightEffectView {
    fn rows(&self) -> Vec<BacklightEffect> {
        cycle_rows(self.effect, &self.supported)
    }
}

impl Render for BacklightEffectView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let rows = self.rows();
        let mut card = div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(osd::GAP))
            .w(px(self.width))
            .h(px(card_height(rows.len())))
            .px(px(osd::PAD))
            .rounded(px(osd::RADIUS))
            .bg(osd::PANEL)
            .text_color(osd::TEXT)
            .font_weight(osd::TITLE)
            .child(
                svg()
                    .path(backlight_effect_icon())
                    .size(px(ICON))
                    .flex_none()
                    .text_color(osd::TEXT),
            );
        for effect in rows {
            let name = rust_i18n::t!(title_key(effect)).into_owned();
            let row = div()
                .w_full()
                .h(px(ROW))
                .flex()
                .items_center()
                .justify_center()
                .text_center()
                .rounded(px(8.0));
            card = if effect == self.effect {
                card.child(
                    row.bg(osd::PILL)
                        .border_2()
                        .border_color(osd::accent())
                        .child(name),
                )
            } else {
                card.child(row.child(name))
            };
        }
        card
    }
}

/// How many rows the window is sized for while `effect` shows.
fn window_rows(effect: BacklightEffect, supported: &[BacklightEffect]) -> usize {
    if supported.is_empty() {
        1
    } else {
        visible_rows(supported, effect).len()
    }
}

/// Show `shown`, or close the indicator when it is `None`.
///
/// A change while the window is open replaces its text in place, so presses
/// in quick succession do not flash the window closed and open again.
pub(crate) fn present(cx: &mut App, shown: Option<BacklightEffect>, supported: &[BacklightEffect]) {
    let open: Vec<WindowHandle<BacklightEffectView>> = cx
        .windows()
        .iter()
        .filter_map(AnyWindowHandle::downcast::<BacklightEffectView>)
        .collect();
    let Some(effect) = shown else {
        for view in open {
            let _ = view.update(cx, |_, window, _| window.remove_window());
        }
        return;
    };
    let supported = supported.to_vec();
    let rows = window_rows(effect, &supported);
    let width = list_width(cx, effect, &supported);
    // A list that gains or loses a row, or needs another width, is reopened
    // rather than resized: resizing in place keeps the top-left, so a
    // centred list would drift off centre.
    let reopen = open.is_empty()
        || open.iter().any(|view| {
            view.read_with(cx, |view, _| {
                window_rows(view.effect, &view.supported) != rows
                    || view.width.to_bits() != width.to_bits()
            })
            .unwrap_or(true)
        });
    if reopen {
        for view in open {
            let _ = view.update(cx, |_, window, _| window.remove_window());
        }
        if let Err(error) = crate::popup::open_centred_clear_of(
            cx,
            "openlogi-backlight-effect-osd",
            size(px(width), px(card_height(rows))),
            size(
                px(crate::backlight::LARGEST_CARD.0),
                px(crate::backlight::LARGEST_CARD.1),
            ),
            move |_, cx| {
                cx.new(|_| BacklightEffectView {
                    effect,
                    supported: supported.clone(),
                    width,
                })
            },
        ) {
            warn!(%error, "could not open the backlight-effect indicator window");
        }
        return;
    }
    for view in open {
        let supported = supported.clone();
        let _ = view.update(cx, |view, _, cx| {
            view.effect = effect;
            view.supported = supported;
            cx.notify();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_effect_announces_its_own_name() {
        let expected = [
            (BacklightEffect::Static, "keyboard.backlight_effect_static"),
            (BacklightEffect::None, "keyboard.backlight_effect_none"),
            (
                BacklightEffect::Breathing,
                "keyboard.backlight_effect_breathing",
            ),
            (
                BacklightEffect::Contrast,
                "keyboard.backlight_effect_contrast",
            ),
            (
                BacklightEffect::Reaction,
                "keyboard.backlight_effect_reaction",
            ),
            (BacklightEffect::Random, "keyboard.backlight_effect_random"),
            (BacklightEffect::Waves, "keyboard.backlight_effect_waves"),
        ];
        let _: fn(BacklightEffect) = |effect| match effect {
            BacklightEffect::Static
            | BacklightEffect::None
            | BacklightEffect::Breathing
            | BacklightEffect::Contrast
            | BacklightEffect::Reaction
            | BacklightEffect::Random
            | BacklightEffect::Waves => {}
        };
        for (effect, key) in expected {
            assert_eq!(title_key(effect), key, "{effect:?}");
        }
    }

    /// A name missing from the catalog would show the raw key on screen. The
    /// other locales are held to the same keys by `catalog_parity`.
    #[test]
    fn every_effect_name_is_in_the_catalog() {
        for effect in BacklightEffect::ALL {
            let title = title_key(effect);
            assert_ne!(rust_i18n::t!(title, locale = "en"), title);
        }
    }

    #[test]
    fn a_short_cycle_is_shown_whole_and_a_long_one_keeps_the_current_row() {
        let four = [
            BacklightEffect::Random,
            BacklightEffect::Contrast,
            BacklightEffect::Waves,
            BacklightEffect::Static,
        ];
        assert_eq!(visible_rows(&four, BacklightEffect::Waves), &four);

        let long = BacklightEffect::CYCLE
            .into_iter()
            .chain(BacklightEffect::CYCLE)
            .collect::<Vec<_>>();
        let window = visible_rows(&long, BacklightEffect::Random);
        assert_eq!(window.len(), MAX_ROWS);
        assert!(window.contains(&BacklightEffect::Random));
    }

    #[test]
    fn the_current_pill_uses_the_shared_accent_and_panel() {
        assert_eq!(osd::accent(), openlogi_ui::color::accent());
        assert_ne!(osd::PILL, osd::PANEL);
        assert_eq!(osd::TEXT.l, 0.96);
    }

    /// The effect each open indicator window shows.
    fn shown(cx: &mut gpui::TestAppContext) -> Vec<BacklightEffect> {
        cx.update(|cx| {
            cx.windows()
                .iter()
                .filter_map(AnyWindowHandle::downcast::<BacklightEffectView>)
                .filter_map(|window| window.read(cx).ok().map(|view| view.effect))
                .collect()
        })
    }

    #[gpui::test]
    fn one_window_shows_each_effect_in_place_until_nothing_shows(cx: &mut gpui::TestAppContext) {
        let breathing = BacklightEffect::Breathing;
        let waves = BacklightEffect::Waves;

        let cycle = [
            BacklightEffect::Random,
            BacklightEffect::Contrast,
            BacklightEffect::Waves,
            BacklightEffect::Static,
        ];
        cx.update(|cx| present(cx, None, &[]));
        assert!(shown(cx).is_empty(), "nothing showing opens nothing");

        cx.update(|cx| present(cx, Some(breathing), &[]));
        assert_eq!(shown(cx), [breathing]);

        cx.update(|cx| present(cx, Some(waves), &cycle));
        cx.run_until_parked();
        assert_eq!(shown(cx), [waves], "the open window shows the newer effect");

        cx.update(|cx| present(cx, None, &[]));
        cx.run_until_parked();
        assert!(shown(cx).is_empty(), "nothing showing closes the window");
    }

    #[test]
    fn the_list_fits_its_longest_name_within_its_bounds() {
        let fitted = fit_list_width(90.0);
        assert!(
            fitted >= osd::PAD * 2.0 + PILL_PAD * 2.0 + 90.0,
            "{fitted} holds the name in its pill"
        );
        assert!(fitted < MAX_WIDTH, "a short cycle is narrower than before");
        assert_eq!(
            fit_list_width(0.0),
            MIN_WIDTH,
            "never narrower than the floor"
        );
        assert_eq!(
            fit_list_width(1000.0),
            MAX_WIDTH,
            "never wider than the cap"
        );
    }

    #[gpui::test]
    fn the_cycle_is_measured_in_the_title_weight(cx: &mut gpui::TestAppContext) {
        let width =
            cx.update(|cx| list_width(cx, BacklightEffect::Static, &BacklightEffect::CYCLE));
        assert!((MIN_WIDTH..=MAX_WIDTH).contains(&width), "{width}");
    }
}
