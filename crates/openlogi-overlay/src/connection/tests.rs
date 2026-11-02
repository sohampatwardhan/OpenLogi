use openlogi_core::device::{BatteryStatus, ConnectionLink, DeviceKind};
use openlogi_ui::action_icons::battery_full_icon;

use super::*;

fn battery(percentage: Option<u8>, level: BatteryLevel) -> BatteryInfo {
    BatteryInfo {
        percentage,
        level,
        status: BatteryStatus::Discharging,
    }
}

fn popup(event: ConnectionEvent, battery: Option<BatteryInfo>) -> ConnectionPopup {
    ConnectionPopup {
        name: Some("MX Master 3S".to_owned()),
        kind: DeviceKind::Mouse,
        event,
        link: ConnectionLink::Bolt,
        battery,
    }
}

#[test]
fn a_connect_names_the_device_its_status_and_its_charge() {
    let connected = connection_copy(&popup(
        ConnectionEvent::Connected,
        Some(battery(Some(80), BatteryLevel::Good)),
    ));
    assert_eq!(connected.name, "MX Master 3S");
    assert_eq!(connected.status, "Connected");
    assert_eq!(connected.charge.as_deref(), Some("80%"));
}

#[test]
fn a_disconnect_names_only_the_device_and_its_status() {
    let disconnected = connection_copy(&popup(
        ConnectionEvent::Disconnected,
        Some(battery(Some(55), BatteryLevel::Good)),
    ));
    assert_eq!(disconnected.name, "MX Master 3S");
    assert_eq!(disconnected.status, "Disconnected");
    assert_eq!(
        disconnected.charge, None,
        "the last known battery is not drawn once the device is gone"
    );
}

#[test]
fn a_device_without_a_name_uses_its_kind() {
    let mut unnamed = popup(ConnectionEvent::Connected, None);
    unnamed.name = None;
    unnamed.kind = DeviceKind::Keyboard;
    assert_eq!(connection_copy(&unnamed).name, "Keyboard");
}

#[test]
fn a_level_only_reading_shows_the_level_word_and_no_reading_shows_none() {
    let level_only = popup(
        ConnectionEvent::Connected,
        Some(battery(None, BatteryLevel::Good)),
    );
    assert_eq!(connection_copy(&level_only).charge.as_deref(), Some("Good"));
    assert_eq!(
        connection_copy(&popup(ConnectionEvent::Connected, None)).charge,
        None
    );
}

#[test]
fn the_battery_glyph_follows_the_reading_and_the_cut() {
    let rounded = MaterialSymbolStyle::Rounded;
    let sharp = MaterialSymbolStyle::Sharp;
    assert_eq!(
        battery_glyph(&battery(Some(100), BatteryLevel::Full), rounded),
        battery_bar_icon(100, rounded)
    );
    assert_ne!(
        battery_glyph(&battery(Some(10), BatteryLevel::Low), rounded),
        battery_glyph(&battery(Some(90), BatteryLevel::Good), rounded)
    );
    assert_eq!(
        battery_glyph(&battery(None, BatteryLevel::Low), rounded),
        battery_bar_icon(25, rounded)
    );
    assert_ne!(
        battery_glyph(&battery(Some(50), BatteryLevel::Good), sharp),
        battery_glyph(&battery(Some(50), BatteryLevel::Good), rounded)
    );
    assert_ne!(battery_bar_icon(100, rounded), battery_full_icon(rounded));
}

fn shown(cx: &mut gpui::TestAppContext) -> Vec<ConnectionEvent> {
    cx.update(|cx| {
        cx.windows()
            .iter()
            .filter_map(AnyWindowHandle::downcast::<ConnectionView>)
            .filter_map(|window| window.read(cx).ok().map(|view| view.popup.event))
            .collect()
    })
}

#[gpui::test]
fn one_window_shows_the_latest_change_in_place_until_nothing_shows(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| present(cx, None));
    assert!(shown(cx).is_empty());

    cx.update(|cx| present(cx, Some(popup(ConnectionEvent::Disconnected, None))));
    assert_eq!(shown(cx), [ConnectionEvent::Disconnected]);

    cx.update(|cx| present(cx, Some(popup(ConnectionEvent::Connected, None))));
    cx.run_until_parked();
    assert_eq!(shown(cx), [ConnectionEvent::Connected]);

    cx.update(|cx| present(cx, None));
    cx.run_until_parked();
    assert!(shown(cx).is_empty());
}

#[test]
fn the_card_widens_to_fit_the_whole_name() {
    let short = RowWidths {
        name: 60.0,
        status: 80.0,
        link: true,
        charge: Some(36.0),
    };
    assert_eq!(
        fit_width(short),
        osd::CONNECTION_WIDTH,
        "a row that fits keeps the narrowest card"
    );

    let long = RowWidths {
        name: 160.0,
        ..short
    };
    let needed =
        PAD * 2.0 + 160.0 + ROW_GAP + 80.0 + ROW_GAP + ICON + ROW_GAP + 36.0 + CHARGE_GAP + ICON;
    assert!(
        fit_width(long) >= needed,
        "{} must hold the name, status, link, charge and battery ({needed})",
        fit_width(long)
    );
    assert!(fit_width(long) > osd::CONNECTION_WIDTH);
}

#[test]
fn a_disconnect_needs_no_room_for_the_link_or_the_battery() {
    let disconnected = RowWidths {
        name: 220.0,
        status: 100.0,
        link: false,
        charge: None,
    };
    let connected = RowWidths {
        link: true,
        charge: Some(36.0),
        ..disconnected
    };
    assert!(fit_width(disconnected) < fit_width(connected));
}

#[test]
fn a_very_long_name_stops_at_the_widest_card() {
    let row = RowWidths {
        name: 2000.0,
        status: 80.0,
        link: true,
        charge: Some(36.0),
    };
    assert_eq!(fit_width(row), MAX_WIDTH);
}

#[gpui::test]
fn a_measured_name_is_wider_than_nothing(cx: &mut gpui::TestAppContext) {
    let named = popup(
        ConnectionEvent::Connected,
        Some(battery(Some(100), BatteryLevel::Full)),
    );
    let width = cx.update(|cx| card_width(cx, &named));
    assert!(
        (osd::CONNECTION_WIDTH..=MAX_WIDTH).contains(&width),
        "{width}"
    );
}
