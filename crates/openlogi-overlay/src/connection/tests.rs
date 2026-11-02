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
