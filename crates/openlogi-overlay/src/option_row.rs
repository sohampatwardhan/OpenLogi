//! The one-line option card the wheel-mode and DPI popups share: a glyph and
//! a label, then the choices across, the current one in the backlight-effect
//! list's pill.
//!
//! ```text
//! [ (⟳) SmartShift  ┃Free spin┃  Ratchet              ]
//! [ (◔) DPI  800  ┃1600┃  3200  6400                   ]
//! [ (+) Precision Mode  ┃On┃  Off                      ]
//! ```
//!
//! The card is as wide as its text, measured before its window opens, from
//! the lock-key card's width up to [`MAX_WIDTH`]; past that the label
//! truncates.

use gpui::{App, IntoElement as _, ParentElement, Styled, div, px, svg};

use crate::osd;

/// Glyph size, the other cards'.
const ICON: f32 = 24.0;
/// A pill's height, the effect list's row.
const PILL_HEIGHT: f32 = 36.0;
/// Inset between a pill's border and its text.
const PILL_PAD: f32 = 12.0;
/// The pill border, drawn clear on the choices that are not current so every
/// choice keeps its width.
const PILL_BORDER: f32 = 2.0;
/// Gap between two choices.
const PILL_GAP: f32 = 4.0;
/// The widest the card grows. A longer label truncates, so one odd
/// translation cannot span the display.
const MAX_WIDTH: f32 = 720.0;

/// What one option card says, in the active locale.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OptionRow {
    /// The glyph before the label.
    pub(crate) icon: &'static str,
    /// What is being chosen: `SmartShift`, `DPI`.
    pub(crate) label: String,
    /// The choices, in order.
    pub(crate) options: Vec<String>,
    /// Which of [`Self::options`] is current.
    pub(crate) current: usize,
}

/// At most `max` of `count` choices around `current`: all of them when they
/// fit, otherwise a window that still holds `current`. Returns the window's
/// first index and length.
pub(crate) fn window(count: usize, current: usize, max: usize) -> (usize, usize) {
    if count <= max {
        return (0, count);
    }
    let start = current.saturating_sub(max / 2).min(count - max);
    (start, max)
}

/// The card width for a label `label` wide and choices `options` wide, in
/// logical pixels: never narrower than the lock-key card, at most
/// [`MAX_WIDTH`].
pub(crate) fn fit_width(label: f32, options: &[f32]) -> f32 {
    let pills: f32 = options
        .iter()
        .map(|option| option + (PILL_PAD + PILL_BORDER) * 2.0)
        .sum();
    let gaps =
        PILL_GAP * f32::from(u8::try_from(options.len().saturating_sub(1)).unwrap_or(u8::MAX));
    let width = osd::PAD * 2.0 + ICON + osd::GAP + label + osd::GAP + pills + gaps;
    (width + osd::TEXT_SLACK).clamp(osd::LOCK_WIDTH, MAX_WIDTH)
}

/// The width that shows `row` whole, measured in the font the card draws.
pub(crate) fn card_width(cx: &App, row: &OptionRow) -> f32 {
    let options: Vec<f32> = row
        .options
        .iter()
        .map(|option| osd::text_width(cx, option, osd::TITLE))
        .collect();
    fit_width(osd::text_width(cx, &row.label, osd::TITLE), &options)
}

/// The option card, `width` wide and the one-row height.
pub(crate) fn option_card(row: &OptionRow, width: f32) -> gpui::Div {
    let pills = row.options.iter().enumerate().map(|(index, option)| {
        let pill = div()
            .flex_none()
            .h(px(PILL_HEIGHT))
            .px(px(PILL_PAD))
            .flex()
            .items_center()
            .rounded(px(8.0))
            .border(px(PILL_BORDER))
            .whitespace_nowrap();
        if index == row.current {
            pill.bg(osd::PILL)
                .border_color(osd::accent())
                .child(option.clone())
        } else {
            pill.border_color(gpui::transparent_black())
                .child(option.clone())
        }
        .into_any_element()
    });
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_center()
        .gap(px(osd::GAP))
        .w(px(width))
        .h(px(osd::LOCK_HEIGHT))
        .px(px(osd::PAD))
        .rounded(px(osd::RADIUS))
        .bg(osd::PANEL)
        .text_color(osd::TEXT)
        .font_weight(osd::TITLE)
        .child(
            svg()
                .path(row.icon)
                .size(px(ICON))
                .flex_none()
                .text_color(osd::accent()),
        )
        .child(div().min_w_0().truncate().child(row.label.clone()))
        .child(
            div()
                .flex()
                .flex_row()
                .flex_none()
                .items_center()
                .gap(px(PILL_GAP))
                .children(pills),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_choice_shows_while_they_fit_and_a_window_holds_the_current_one() {
        assert_eq!(window(4, 2, 6), (0, 4));
        assert_eq!(window(10, 0, 6), (0, 6));
        assert_eq!(window(10, 5, 6), (2, 6));
        assert_eq!(window(10, 9, 6), (4, 6), "the window stops at the end");
    }

    #[test]
    fn the_card_grows_with_its_text_between_its_bounds() {
        let short = fit_width(30.0, &[40.0, 40.0]);
        assert!(
            (short - osd::LOCK_WIDTH).abs() < f32::EPSILON,
            "a short row keeps the lock-key width: {short}"
        );
        let long = fit_width(120.0, &[60.0; 5]);
        let expected = osd::PAD * 2.0
            + ICON
            + osd::GAP
            + 120.0
            + osd::GAP
            + (60.0 + (PILL_PAD + PILL_BORDER) * 2.0) * 5.0
            + PILL_GAP * 4.0
            + osd::TEXT_SLACK;
        assert!((long - expected).abs() < 0.01, "{long} vs {expected}");
        let huge = fit_width(900.0, &[60.0]);
        assert!((huge - MAX_WIDTH).abs() < f32::EPSILON, "{huge}");
    }
}
