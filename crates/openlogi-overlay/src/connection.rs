//! Compact transient connection indicator.
//!
//! The agent decides what shows and for how long
//! ([`openlogi_ipc::ConnectionObservation`]); this module only draws it, in
//! the charging card's chrome ([`crate::osd`]). One row: the device on the
//! left, and on the right either the word Disconnected, or Connected with
//! the link glyph and, when the device has reported one, its charge and the
//! vertical battery glyph.
//!
//! ```text
//! [ MX Master 3S        Connected (link) 80% (battery) ]
//! [ MX Keys S                              Disconnected ]
//! ```

use std::sync::Arc;

use gpui::{
    AnyWindowHandle, App, AppContext as _, Context, FontFeatures, FontWeight, IntoElement,
    ParentElement, Pixels, Render, Styled, Window, div, px, svg,
};
use openlogi_core::device::{BatteryInfo, BatteryLevel};
use openlogi_ipc::{ConnectionEvent, ConnectionPopup};
use openlogi_ui::action_icons::{MaterialSymbolStyle, battery_bar_icon, connection_link_icon};
use tracing::warn;

use crate::charging::charge_label;
use crate::osd;

/// The words on a connection card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ConnectionCopy {
    /// The device's name, else its kind: `MX Master 3S`, `Mouse`.
    pub(crate) name: String,
    /// `Connected` or `Disconnected`.
    pub(crate) status: String,
    /// `80%`, or the level word when the reading has no percentage. `None`
    /// for a disconnect, and for a device that has reported no battery,
    /// which draws no battery.
    pub(crate) charge: Option<String>,
}

/// The words on `popup`, in the active locale.
pub(crate) fn connection_copy(popup: &ConnectionPopup) -> ConnectionCopy {
    let name = popup
        .name
        .clone()
        .unwrap_or_else(|| rust_i18n::t!(popup.kind.translation_key()).into_owned());
    let (status, charge) = match popup.event {
        ConnectionEvent::Connected => (
            rust_i18n::t!("device.connected"),
            popup
                .battery
                .as_ref()
                .map(|battery| charge_label(battery.percentage, battery.level)),
        ),
        ConnectionEvent::Disconnected => (rust_i18n::t!("device.disconnected"), None),
    };
    ConnectionCopy {
        name,
        status: status.into_owned(),
        charge,
    }
}

/// The vertical battery glyph for `battery`. A level-only reading draws the
/// bars its level stands for.
pub(crate) fn battery_glyph(battery: &BatteryInfo, style: MaterialSymbolStyle) -> &'static str {
    let percentage = battery.percentage.unwrap_or(match battery.level {
        BatteryLevel::Full => 100,
        BatteryLevel::Good => 70,
        BatteryLevel::Low => 25,
        BatteryLevel::Critical | BatteryLevel::Unknown => 0,
    });
    battery_bar_icon(percentage, style)
}

/// Glyph size, the other cards'.
const ICON: f32 = 24.0;
/// Inset between the card edge and its row, the other cards' padding.
const PAD: f32 = 20.0;
/// Gap between the name and the status, and around the link glyph.
const ROW_GAP: f32 = 8.0;
/// Gap between the charge and the battery glyph, tighter than [`ROW_GAP`]
/// so the two read as one reading.
const CHARGE_GAP: f32 = 2.0;
/// The cards' text size: GPUI's default `1rem`.
const TEXT_SIZE: f32 = 16.0;
/// Room for the rounding and kerning a sum of glyph advances leaves out.
const TEXT_SLACK: f32 = 4.0;
/// The widest the card grows to fit a name. A name longer than this still
/// truncates, so one odd codename cannot span the display.
const MAX_WIDTH: f32 = 720.0;

/// The widths of what one card row holds, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RowWidths {
    /// The device name, in the title weight.
    pub(crate) name: f32,
    /// `Connected` or `Disconnected`.
    pub(crate) status: f32,
    /// Whether the link glyph follows the status.
    pub(crate) link: bool,
    /// The charge text, when the battery reading and its glyph follow.
    pub(crate) charge: Option<f32>,
}

/// The card width that shows `row` without truncating the name: never
/// narrower than [`osd::CONNECTION_WIDTH`], and at most [`MAX_WIDTH`].
pub(crate) fn fit_width(row: RowWidths) -> f32 {
    let mut width = PAD * 2.0 + row.name + ROW_GAP + row.status;
    if row.link {
        width += ROW_GAP + ICON;
    }
    if let Some(charge) = row.charge {
        width += ROW_GAP + charge + CHARGE_GAP + ICON;
    }
    (width + TEXT_SLACK).clamp(osd::CONNECTION_WIDTH, MAX_WIDTH)
}

/// The card width for `popup`, measured in the system UI font the card
/// draws with.
pub(crate) fn card_width(cx: &App, popup: &ConnectionPopup) -> f32 {
    let ConnectionCopy {
        name,
        status,
        charge,
    } = connection_copy(popup);
    let connected = popup.event == ConnectionEvent::Connected;
    fit_width(RowWidths {
        name: text_width(cx, &name, FontWeight::SEMIBOLD),
        status: text_width(cx, &status, FontWeight::NORMAL),
        link: connected,
        charge: charge
            .filter(|_| connected && popup.battery.is_some())
            .map(|charge| text_width(cx, &charge, FontWeight::NORMAL)),
    })
}

/// The width of `text` at [`TEXT_SIZE`] in `weight`, from its glyph
/// advances. A glyph the font lacks counts as one em.
fn text_width(cx: &App, text: &str, weight: FontWeight) -> f32 {
    let text_system = cx.text_system();
    let font_id = text_system.resolve_font(&gpui::Font {
        weight,
        ..gpui::font(".SystemUIFont")
    });
    let size = px(TEXT_SIZE);
    let em = text_system
        .em_width(font_id, size)
        .map_or(TEXT_SIZE, Pixels::as_f32);
    text.chars()
        .map(|ch| {
            text_system
                .advance(font_id, size, ch)
                .map_or(em, |advance| advance.width.as_f32())
        })
        .sum()
}

fn glyph(path: &'static str) -> gpui::Svg {
    svg()
        .path(path)
        .size(px(ICON))
        .flex_none()
        .text_color(osd::accent())
}

/// The connection card, `width` wide, shared by the indicator window and the
/// preview board. [`card_width`] is the width that fits the whole name.
pub(crate) fn connection_card(
    popup: &ConnectionPopup,
    style: MaterialSymbolStyle,
    width: f32,
) -> gpui::Div {
    let ConnectionCopy {
        name,
        status,
        charge,
    } = connection_copy(popup);
    let mut status_row = div()
        .flex()
        .flex_row()
        .flex_none()
        .items_center()
        .gap(px(ROW_GAP))
        .child(div().whitespace_nowrap().child(status));
    if popup.event == ConnectionEvent::Connected {
        status_row = status_row.child(glyph(connection_link_icon(popup.link, style)));
        if let (Some(battery), Some(charge)) = (popup.battery.as_ref(), charge) {
            status_row = status_row.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(CHARGE_GAP))
                    .child(
                        div()
                            .whitespace_nowrap()
                            .font_features(FontFeatures(Arc::new(vec![("tnum".into(), 1)])))
                            .child(charge),
                    )
                    .child(glyph(battery_glyph(battery, style))),
            );
        }
    }
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(ROW_GAP))
        .w(px(width))
        .h(px(osd::CONNECTION_HEIGHT))
        .px(px(PAD))
        .rounded(px(osd::RADIUS))
        .bg(osd::PANEL)
        .text_color(osd::TEXT)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .font_weight(FontWeight::SEMIBOLD)
                .child(name),
        )
        .child(status_row)
}

/// The indicator window's view: the popup the agent is showing.
pub(crate) struct ConnectionView {
    popup: ConnectionPopup,
    /// The window's width, which [`card_width`] chose when it opened.
    width: f32,
}

impl ConnectionView {
    pub(crate) fn new(popup: ConnectionPopup, width: f32) -> Self {
        Self { popup, width }
    }
}

impl Render for ConnectionView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        connection_card(&self.popup, crate::icons::style(cx), self.width)
    }
}

/// Show `popup`, or close the indicator when it is `None`.
///
/// A later popup that fits the same width replaces the card in place, so a
/// device that flaps shows one card with its latest state.
pub(crate) fn present(cx: &mut App, popup: Option<ConnectionPopup>) {
    let open: Vec<gpui::WindowHandle<ConnectionView>> = cx
        .windows()
        .iter()
        .filter_map(AnyWindowHandle::downcast::<ConnectionView>)
        .collect();
    let Some(popup) = popup else {
        for view in open {
            let _ = view.update(cx, |_, window, _| window.remove_window());
        }
        return;
    };
    let width = card_width(cx, &popup);
    // A card that needs another width is reopened rather than resized:
    // resizing in place keeps the top-left, which would shift the card off
    // centre. The name and the battery stay the same across a flap, so this
    // is the status changing width.
    let reopen = open.is_empty()
        || open.iter().any(|view| {
            view.read_with(cx, |view, _| view.width.to_bits() != width.to_bits())
                .unwrap_or(true)
        });
    if reopen {
        for view in open {
            let _ = view.update(cx, |_, window, _| window.remove_window());
        }
        if let Err(error) = crate::popup::open(
            cx,
            "openlogi-connection-osd",
            gpui::size(px(width), px(osd::CONNECTION_HEIGHT)),
            move |_, cx| cx.new(|_| ConnectionView::new(popup.clone(), width)),
        ) {
            warn!(%error, "could not open the connection indicator window");
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
mod tests;
