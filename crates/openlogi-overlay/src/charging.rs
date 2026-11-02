//! Compact transient charging indicator.
//!
//! The agent decides what shows and for how long
//! ([`openlogi_ipc::ChargingObservation`]); this module only draws it. The
//! panel, radius, colours, and accent are the backlight and lock-key
//! indicators' ([`crate::osd`]).

use std::sync::Arc;

use gpui::{
    AnyWindowHandle, App, AppContext as _, Context, FontFeatures, Hsla, IntoElement, ParentElement,
    Render, Styled, Window, div, px, svg,
};
use openlogi_core::device::BatteryLevel;
use openlogi_ipc::{ChargingEvent, ChargingPopup};
use openlogi_ui::action_icons::{
    battery_charging_icon, battery_full_icon, battery_level_icon, low_battery_icon,
};
use tracing::warn;

use crate::osd;

/// The words on a battery card: the device (and its status, if any) on the top row,
/// the charge at the end of the bar row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BatteryCopy {
    /// `MX Mechanical · Charging`.
    pub(crate) headline: String,
    /// `95%`, or the level word when the reading has no percentage.
    pub(crate) charge: String,
}

/// `{device} · {status}`, the top row of a battery card that has a status.
pub(crate) fn headline(device: &str, status: &str) -> String {
    format!("{device} · {status}")
}

/// The charge at the end of the bar: the percentage, else the level word.
pub(crate) fn charge_label(percentage: Option<u8>, level: BatteryLevel) -> String {
    match percentage {
        Some(percentage) => format!("{percentage}%"),
        None => level_word(level),
    }
}

/// The words on `popup`, in the active locale.
pub(crate) fn charging_copy(popup: &ChargingPopup) -> BatteryCopy {
    let device = popup
        .name
        .clone()
        .unwrap_or_else(|| rust_i18n::t!(popup.kind.translation_key()).into_owned());
    // Unplugged reads as the device alone: the bar row already says how
    // much charge is left, and a "not charging" label adds nothing.
    let headline = match popup.event {
        ChargingEvent::Started => headline(&device, &rust_i18n::t!("device.charging")),
        ChargingEvent::Stopped => device,
        ChargingEvent::Full => headline(&device, &rust_i18n::t!("device.fully_charged")),
    };
    BatteryCopy {
        headline,
        charge: charge_label(popup.percentage, popup.level),
    }
}

fn level_word(level: BatteryLevel) -> String {
    match level {
        BatteryLevel::Full => rust_i18n::t!("device.full").into_owned(),
        BatteryLevel::Good => rust_i18n::t!("device.battery_good").into_owned(),
        BatteryLevel::Low | BatteryLevel::Critical => {
            rust_i18n::t!("device.low_battery").into_owned()
        }
        BatteryLevel::Unknown => "—".to_owned(),
    }
}

/// How full the level bar is, when the reading has a percentage.
pub(crate) fn charge_fraction(percentage: Option<u8>) -> Option<f32> {
    percentage.map(|percentage| f32::from(percentage.min(100)) / 100.0)
}

/// Lucide's battery glyph for `popup`. A level-only reading draws the third
/// its level stands for; a critical one draws the warning.
fn glyph(popup: &ChargingPopup) -> &'static str {
    match popup.event {
        ChargingEvent::Started => battery_charging_icon(),
        ChargingEvent::Full => battery_full_icon(),
        ChargingEvent::Stopped => match popup.percentage {
            Some(percentage) => battery_level_icon(percentage),
            None => match popup.level {
                BatteryLevel::Full => battery_full_icon(),
                BatteryLevel::Good => battery_level_icon(50),
                BatteryLevel::Low | BatteryLevel::Unknown => battery_level_icon(0),
                BatteryLevel::Critical => low_battery_icon(),
            },
        },
    }
}

/// The glyph and bar colour: the status green once the battery is full
/// (Fully charged, or a reading of 100%), the backlight bar's accent before.
fn glyph_colour(popup: &ChargingPopup) -> gpui::Hsla {
    osd::battery_colour(
        popup.event == ChargingEvent::Full || osd::is_full_reading(popup.percentage, popup.level),
    )
}

/// The indicator window's view: the popup the agent is showing.
pub(crate) struct ChargingView {
    popup: ChargingPopup,
}

impl ChargingView {
    pub(crate) fn new(popup: ChargingPopup) -> Self {
        Self { popup }
    }
}

/// Glyph size on the top row, the brightness card's.
const ICON: f32 = 24.0;
/// Gap between the glyph and the headline, and between the two rows.
const ROW_GAP: f32 = 8.0;

/// What a two-row battery card draws.
pub(crate) struct BatteryCard {
    /// The Material Symbol on the top row.
    pub(crate) icon: &'static str,
    /// The glyph's colour, which the bar fill shares.
    pub(crate) colour: Hsla,
    pub(crate) copy: BatteryCopy,
    /// How full the bar is. `None` draws no bar, only the charge word.
    pub(crate) fraction: Option<f32>,
}

/// The battery card the charging popup and the low-battery notice share.
///
/// Row 1, centred as a group: the glyph, then `{device} · {status}`.
/// Row 2: the bar filling the width, then the charge such as `95%`, like
/// the brightness card's one row.
pub(crate) fn battery_card(card: BatteryCard) -> gpui::Div {
    let BatteryCard {
        icon,
        colour,
        copy: BatteryCopy { headline, charge },
        fraction,
    } = card;
    let charge = div()
        .flex_none()
        .text_right()
        .whitespace_nowrap()
        .font_features(FontFeatures(Arc::new(vec![("tnum".into(), 1)])))
        .child(charge);
    let bar_row = div().flex().flex_row().items_center().gap(px(osd::GAP));
    let bar_row = match fraction {
        Some(fraction) => bar_row.child(level_bar(fraction, colour)).child(charge),
        None => bar_row.justify_center().child(charge),
    };
    div()
        .flex()
        .flex_col()
        .justify_center()
        .gap(px(ROW_GAP))
        .w(px(osd::CHARGING_WIDTH))
        .h(px(osd::CHARGING_HEIGHT))
        .px(px(osd::PAD))
        .rounded(px(osd::RADIUS))
        .bg(osd::PANEL)
        .text_color(osd::TEXT)
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_center()
                .gap(px(ROW_GAP))
                .min_w_0()
                .child(
                    svg()
                        .path(icon)
                        .size(px(ICON))
                        .flex_none()
                        .text_color(colour),
                )
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .font_weight(osd::TITLE)
                        .child(headline),
                ),
        )
        .child(bar_row)
}

fn level_bar(fraction: f32, fill: Hsla) -> gpui::Div {
    div()
        .flex_1()
        .h(px(osd::BAR_HEIGHT))
        .rounded(px(osd::BAR_RADIUS))
        .bg(osd::TRACK)
        .child(
            div()
                .h_full()
                .w(gpui::relative(fraction))
                .rounded(px(osd::BAR_RADIUS))
                .bg(fill),
        )
}

/// The charging card, shared by the indicator window and the preview board.
pub(crate) fn charging_card(popup: &ChargingPopup) -> gpui::Div {
    battery_card(BatteryCard {
        icon: glyph(popup),
        colour: glyph_colour(popup),
        copy: charging_copy(popup),
        fraction: charge_fraction(popup.percentage),
    })
}

impl Render for ChargingView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        charging_card(&self.popup)
    }
}

/// Show `popup`, or close the indicator when it is `None`.
///
/// A later popup replaces the text in place, so two devices that settle
/// together do not flash the window closed and open again.
pub(crate) fn present(cx: &mut App, popup: Option<ChargingPopup>) {
    let open: Vec<gpui::WindowHandle<ChargingView>> = cx
        .windows()
        .iter()
        .filter_map(AnyWindowHandle::downcast::<ChargingView>)
        .collect();
    let Some(popup) = popup else {
        for view in open {
            let _ = view.update(cx, |_, window, _| window.remove_window());
        }
        return;
    };
    if open.is_empty() {
        if let Err(error) = crate::popup::open(
            cx,
            "openlogi-charging-osd",
            gpui::size(px(osd::CHARGING_WIDTH), px(osd::CHARGING_HEIGHT)),
            move |_, cx| cx.new(|_| ChargingView::new(popup.clone())),
        ) {
            warn!(%error, "could not open the charging indicator window");
        }
        return;
    }
    for view in open {
        let _ = view.update(cx, |view, _, cx| {
            view.popup = popup.clone();
            cx.notify();
        });
    }
}

#[cfg(test)]
mod tests {
    use openlogi_core::device::{BatteryLevel, DeviceKind};
    use openlogi_ipc::{ChargingEvent, ChargingPopup};

    use super::*;

    fn popup(event: ChargingEvent, percentage: Option<u8>) -> ChargingPopup {
        ChargingPopup {
            name: Some("MX Anywhere 3S".to_owned()),
            kind: DeviceKind::Mouse,
            event,
            percentage,
            level: BatteryLevel::Good,
        }
    }

    #[test]
    fn the_top_row_names_the_device_and_status_and_the_bar_row_the_charge() {
        let started = charging_copy(&popup(ChargingEvent::Started, Some(80)));
        assert_eq!(started.headline, "MX Anywhere 3S · Charging");
        assert_eq!(started.charge, "80%");

        let stopped = charging_copy(&popup(ChargingEvent::Stopped, Some(80)));
        assert_eq!(stopped.headline, "MX Anywhere 3S");
        assert_eq!(stopped.charge, "80%");

        let full = charging_copy(&popup(ChargingEvent::Full, Some(100)));
        assert_eq!(full.headline, "MX Anywhere 3S · Fully charged");
        assert_eq!(full.charge, "100%");
    }

    #[test]
    fn a_device_without_a_name_uses_its_kind() {
        let mut unnamed = popup(ChargingEvent::Started, Some(40));
        unnamed.name = None;
        assert_eq!(charging_copy(&unnamed).headline, "Mouse · Charging");
    }

    #[test]
    fn a_level_only_reading_uses_the_level_word() {
        let mut reading = popup(ChargingEvent::Stopped, None);
        reading.level = BatteryLevel::Good;
        assert_eq!(charging_copy(&reading).charge, "Good");
        assert!(charge_fraction(None).is_none());
        assert_eq!(charge_fraction(Some(80)), Some(0.8));
        assert_eq!(charge_fraction(Some(100)), Some(1.0));
    }

    #[test]
    fn a_full_battery_is_green_and_any_other_is_the_accent() {
        let green = openlogi_ui::color::status_green();
        assert_eq!(glyph_colour(&popup(ChargingEvent::Full, Some(100))), green);
        assert_eq!(
            glyph_colour(&popup(ChargingEvent::Full, Some(98))),
            green,
            "Fully charged is green whatever the reading rounds to"
        );
        assert_eq!(
            glyph_colour(&popup(ChargingEvent::Started, Some(100))),
            green
        );
        assert_eq!(
            glyph_colour(&popup(ChargingEvent::Stopped, Some(100))),
            green
        );
        assert_eq!(
            glyph_colour(&popup(ChargingEvent::Started, Some(80))),
            osd::accent()
        );
        assert_eq!(
            glyph_colour(&popup(ChargingEvent::Stopped, Some(99))),
            osd::accent()
        );
    }

    #[test]
    fn every_popup_picks_its_glyph() {
        for percentage in [Some(15), Some(100), None] {
            assert_eq!(
                glyph(&popup(ChargingEvent::Started, percentage)),
                battery_charging_icon(),
                "one charging glyph at {percentage:?}: the bar shows the charge"
            );
        }
        assert_eq!(
            glyph(&popup(ChargingEvent::Full, Some(100))),
            battery_full_icon()
        );
        assert_eq!(
            glyph(&popup(ChargingEvent::Stopped, Some(10))),
            battery_level_icon(10)
        );
        assert_eq!(
            glyph(&popup(ChargingEvent::Stopped, Some(90))),
            battery_level_icon(90)
        );
        assert_ne!(
            glyph(&popup(ChargingEvent::Stopped, Some(10))),
            glyph(&popup(ChargingEvent::Stopped, Some(90)))
        );
    }

    fn shown(cx: &mut gpui::TestAppContext) -> Vec<ChargingEvent> {
        cx.update(|cx| {
            cx.windows()
                .iter()
                .filter_map(AnyWindowHandle::downcast::<ChargingView>)
                .filter_map(|window| window.read(cx).ok().map(|view| view.popup.event))
                .collect()
        })
    }

    #[gpui::test]
    fn one_window_shows_each_popup_in_place_until_nothing_shows(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| present(cx, None));
        assert!(shown(cx).is_empty());

        cx.update(|cx| present(cx, Some(popup(ChargingEvent::Started, Some(80)))));
        assert_eq!(shown(cx), [ChargingEvent::Started]);

        cx.update(|cx| present(cx, Some(popup(ChargingEvent::Stopped, Some(80)))));
        cx.run_until_parked();
        assert_eq!(shown(cx), [ChargingEvent::Stopped]);

        cx.update(|cx| present(cx, None));
        cx.run_until_parked();
        assert!(shown(cx).is_empty());
    }
}
