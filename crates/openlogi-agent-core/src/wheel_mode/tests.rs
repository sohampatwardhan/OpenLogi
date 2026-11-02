//! Wheel modes: baselines, firmware switches, toggles, the settings app's
//! writes, and the user's switch.

use openlogi_core::config::AppSettings;
use openlogi_core::hid::{DeviceRoute, SmartShiftMode};

use super::{WheelModeIndicator, WheelModes, WheelNode};
use crate::backlight_overlay::DISPLAY_LIFETIME;

const BOLT: WheelNode = WheelNode {
    vendor_id: 0x046d,
    product_id: 0xc548,
    device_index: 2,
};

fn bolt_route() -> DeviceRoute {
    DeviceRoute::Bolt {
        receiver_uid: "ABCD".to_owned(),
        slot: 2,
    }
}

fn direct_route() -> DeviceRoute {
    DeviceRoute::Direct {
        vendor_id: 0x046d,
        product_id: 0xb037,
    }
}

const DIRECT: WheelNode = WheelNode {
    vendor_id: 0x046d,
    product_id: 0xb037,
    device_index: 0xff,
};

fn modes_with_bolt() -> WheelModes {
    let mut modes = WheelModes::default();
    modes.routes.insert(bolt_route(), BOLT);
    modes
}

#[test]
fn the_mode_at_connect_is_a_baseline() {
    let mut modes = WheelModes::default();
    modes.remember(BOLT, SmartShiftMode::Ratchet);
    assert_eq!(
        modes.switched(BOLT, SmartShiftMode::Free),
        Some(SmartShiftMode::Free),
        "the first switch after the baseline is shown"
    );
}

#[test]
fn a_switch_with_no_known_mode_is_a_baseline() {
    let mut modes = WheelModes::default();
    assert_eq!(modes.switched(BOLT, SmartShiftMode::Free), None);
    assert_eq!(
        modes.switched(BOLT, SmartShiftMode::Ratchet),
        Some(SmartShiftMode::Ratchet)
    );
}

#[test]
fn a_repeated_mode_is_not_shown_again() {
    // The ratchet switch also fires when SmartShift disengages on a fast
    // spin; the SmartShift mode read back has not moved then.
    let mut modes = WheelModes::default();
    modes.remember(BOLT, SmartShiftMode::Ratchet);
    assert_eq!(modes.switched(BOLT, SmartShiftMode::Ratchet), None);
    assert_eq!(
        modes.switched(BOLT, SmartShiftMode::Free),
        Some(SmartShiftMode::Free)
    );
    assert_eq!(modes.switched(BOLT, SmartShiftMode::Free), None);
}

#[test]
fn each_mouse_has_its_own_mode() {
    let mut modes = WheelModes::default();
    modes.remember(BOLT, SmartShiftMode::Ratchet);
    assert_eq!(
        modes.switched(DIRECT, SmartShiftMode::Free),
        None,
        "another mouse's first reading is its own baseline"
    );
    assert_eq!(modes.switched(BOLT, SmartShiftMode::Ratchet), None);
}

#[test]
fn a_mode_the_settings_app_writes_is_a_baseline_for_its_broadcast() {
    let mut modes = modes_with_bolt();
    modes.remember(BOLT, SmartShiftMode::Ratchet);
    modes.written(&bolt_route(), SmartShiftMode::Free);
    assert_eq!(
        modes.switched(BOLT, SmartShiftMode::Free),
        None,
        "the mouse's broadcast for the app's write repeats the known mode"
    );
    assert_eq!(
        modes.switched(BOLT, SmartShiftMode::Ratchet),
        Some(SmartShiftMode::Ratchet),
        "the wheel button after it still shows"
    );
}

#[test]
fn a_direct_route_names_its_node_without_the_inventory() {
    let mut modes = WheelModes::default();
    modes.remember(DIRECT, SmartShiftMode::Ratchet);
    modes.written(&direct_route(), SmartShiftMode::Free);
    assert_eq!(modes.switched(DIRECT, SmartShiftMode::Free), None);
}

#[test]
fn a_failed_write_makes_the_next_reading_a_baseline() {
    let mut modes = modes_with_bolt();
    modes.written(&bolt_route(), SmartShiftMode::Free);
    modes.forget(&bolt_route());
    assert_eq!(modes.switched(BOLT, SmartShiftMode::Ratchet), None);
}

#[test]
fn a_toggle_is_shown_and_its_broadcast_is_not() {
    let mut modes = modes_with_bolt();
    modes.remember(BOLT, SmartShiftMode::Ratchet);
    assert_eq!(
        modes.toggled(&bolt_route(), SmartShiftMode::Free),
        SmartShiftMode::Free
    );
    assert_eq!(modes.switched(BOLT, SmartShiftMode::Free), None);
}

#[test]
fn a_toggle_on_an_unmapped_route_is_still_shown() {
    let mut modes = WheelModes::default();
    let unknown = DeviceRoute::Bolt {
        receiver_uid: "other".to_owned(),
        slot: 1,
    };
    assert_eq!(
        modes.toggled(&unknown, SmartShiftMode::Ratchet),
        SmartShiftMode::Ratchet
    );
    assert!(modes.modes.is_empty(), "no node to remember it under");
}

#[tokio::test(start_paused = true)]
async fn the_indicator_shows_a_switch_in_its_language_and_hides_it() {
    let indicator = WheelModeIndicator::default();
    indicator.configure(&AppSettings {
        language: Some("de".to_owned()),
        ..AppSettings::default()
    });
    indicator.connected(BOLT, SmartShiftMode::Ratchet);
    let baseline = indicator.observe(0).await;
    assert_eq!(baseline.shown, None, "the baseline shows nothing");

    assert!(indicator.switched(BOLT, SmartShiftMode::Free));
    let shown = indicator.observe(baseline.generation).await;
    assert_eq!(shown.shown, Some(SmartShiftMode::Free));
    assert_eq!(shown.language.as_deref(), Some("de"));

    assert!(!indicator.switched(BOLT, SmartShiftMode::Free));

    tokio::task::yield_now().await;
    tokio::time::advance(DISPLAY_LIFETIME).await;
    let ended = indicator.observe(shown.generation).await;
    assert_eq!(ended.shown, None);
}

#[tokio::test(start_paused = true)]
async fn the_popup_stays_off_when_turned_off_and_does_not_replay() {
    let indicator = WheelModeIndicator::default();
    indicator.configure(&AppSettings {
        wheel_mode_notifications: false,
        ..AppSettings::default()
    });
    indicator.connected(BOLT, SmartShiftMode::Ratchet);
    let start = indicator.observe(0).await;

    assert!(!indicator.switched(BOLT, SmartShiftMode::Free));
    assert!(!indicator.toggled(&direct_route(), SmartShiftMode::Free));

    indicator.configure(&AppSettings::default());
    assert!(
        !indicator.switched(BOLT, SmartShiftMode::Free),
        "the switch made while off is the known mode now"
    );
    let still = tokio::time::timeout(DISPLAY_LIFETIME, indicator.observe(start.generation))
        .await
        .unwrap_or_else(|_| start.clone());
    assert_eq!(still.shown, None);

    assert!(indicator.switched(BOLT, SmartShiftMode::Ratchet));
}
