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
    ParentElement, Render, Styled, Window, div, px, svg,
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

fn glyph(path: &'static str) -> gpui::Svg {
    svg()
        .path(path)
        .size(px(ICON))
        .flex_none()
        .text_color(osd::accent())
}

/// The connection card, shared by the indicator window and the preview
/// board.
pub(crate) fn connection_card(popup: &ConnectionPopup, style: MaterialSymbolStyle) -> gpui::Div {
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
        .w(px(osd::CONNECTION_WIDTH))
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
}

impl ConnectionView {
    pub(crate) fn new(popup: ConnectionPopup) -> Self {
        Self { popup }
    }
}

impl Render for ConnectionView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        connection_card(&self.popup, crate::icons::style(cx))
    }
}

/// Show `popup`, or close the indicator when it is `None`.
///
/// A later popup replaces the card in place, so a device that flaps shows
/// one card with its latest state rather than a window closing and opening.
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
    if open.is_empty() {
        if let Err(error) = crate::popup::open(
            cx,
            "openlogi-connection-osd",
            gpui::size(px(osd::CONNECTION_WIDTH), px(osd::CONNECTION_HEIGHT)),
            move |_, cx| cx.new(|_| ConnectionView::new(popup.clone())),
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
