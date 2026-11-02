//! Dev-only side-by-side of the transient indicators.
//!
//! `OPENLOGI_OSD_PREVIEW=light` or `dark` opens discharging, charging, full,
//! and low-battery indicators on one backdrop. `sharp` and `sharp-dark` draw
//! the same cards with the sharp battery cut. `OPENLOGI_OSD_RADIUS=0` forces
//! that cut on any of those modes. It does not connect to an agent.

use gpui::{
    AppContext as _, Bounds, Context, IntoElement, ParentElement, Render, Styled, Window,
    WindowBounds, WindowOptions, div, point, px, size,
};
use openlogi_core::device::{BatteryInfo, BatteryLevel, BatteryStatus, DeviceKind};
use openlogi_ipc::{ChargingEvent, ChargingPopup, Notice, NoticeKind};
use openlogi_ui::action_icons::MaterialSymbolStyle;

use crate::charging::charging_card;
use crate::notice::notice_card;
use crate::osd::{CHARGING_HEIGHT, CHARGING_WIDTH};

/// Width of the four-card rows, including the gaps between them.
const BOARD_WIDTH: f32 = CHARGING_WIDTH * 4.0 + 16.0 * 3.0 + 48.0;
const BOARD_HEIGHT: f32 = CHARGING_HEIGHT * 3.0 + 16.0 * 2.0 + 48.0;

/// Open the indicator cards on one backdrop and stay up until the process
/// is closed.
pub(crate) fn run(mode: &str) {
    let dark = mode.eq_ignore_ascii_case("dark") || mode.eq_ignore_ascii_case("sharp-dark");
    let sharp = mode.to_ascii_lowercase().contains("sharp") || radius_is_square();
    openlogi_core::locale::activate(Some("en"));
    let app = gpui::Application::new_inaccessible(gpui_platform::current_platform(false))
        .with_assets(openlogi_ui::action_icons::ActionIcons);
    app.run(move |cx| {
        let style = if sharp {
            MaterialSymbolStyle::Sharp
        } else {
            MaterialSymbolStyle::Rounded
        };
        cx.set_global(crate::icons::PopupIcons(style));
        let _board = cx.open_window(board_options(), |_, cx| cx.new(|_| Board { dark, style }));
    });
}

fn radius_is_square() -> bool {
    std::env::var("OPENLOGI_OSD_RADIUS").is_ok_and(|radius| radius == "0")
}

fn sample(event: ChargingEvent, percentage: u8) -> ChargingPopup {
    ChargingPopup {
        name: Some("MX Anywhere 3S".to_owned()),
        kind: DeviceKind::Mouse,
        event,
        percentage: Some(percentage),
        level: BatteryLevel::Good,
    }
}

fn board_options() -> WindowOptions {
    WindowOptions {
        titlebar: None,
        focus: false,
        show: true,
        is_movable: false,
        is_resizable: false,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(40.0), px(360.0)),
            size: size(px(BOARD_WIDTH), px(BOARD_HEIGHT)),
        })),
        ..WindowOptions::default()
    }
}

struct Board {
    dark: bool,
    style: MaterialSymbolStyle,
}

impl Render for Board {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let background = if self.dark {
            gpui::hsla(0.0, 0.0, 0.16, 1.0)
        } else {
            gpui::hsla(0.0, 0.0, 0.96, 1.0)
        };
        let style = self.style;
        let row = |events: [(ChargingEvent, u8); 4]| {
            div().flex().flex_row().gap(px(16.0)).children(
                events.map(|(event, percentage)| charging_card(&sample(event, percentage), style)),
            )
        };
        div()
            .size_full()
            .bg(background)
            .flex()
            .flex_col()
            .justify_center()
            .gap(px(16.0))
            .p(px(24.0))
            .child(row([
                (ChargingEvent::Stopped, 0),
                (ChargingEvent::Stopped, 25),
                (ChargingEvent::Stopped, 50),
                (ChargingEvent::Stopped, 90),
            ]))
            .child(row([
                (ChargingEvent::Started, 15),
                (ChargingEvent::Started, 45),
                (ChargingEvent::Started, 80),
                (ChargingEvent::Started, 100),
            ]))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(16.0))
                    .child(charging_card(&sample(ChargingEvent::Full, 100), style))
                    .child(notice_card(
                        &Notice {
                            id: 1,
                            kind: NoticeKind::LowBattery {
                                name: Some("MX Anywhere 3S".to_owned()),
                                kind: DeviceKind::Mouse,
                                battery: BatteryInfo {
                                    percentage: Some(12),
                                    level: BatteryLevel::Low,
                                    status: BatteryStatus::Discharging,
                                },
                            },
                        },
                        style,
                    )),
            )
    }
}
