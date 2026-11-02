//! Charging popups from inventory refreshes: baselines, and immediate changes.

use std::sync::Arc;

use openlogi_core::config::Config;
use openlogi_core::device::{BatteryInfo, BatteryLevel, BatteryStatus, DeviceKind};
use openlogi_ipc::{ChargingEvent, ChargingPopup};

use super::direct_inventory;
use crate::observable::ObservableState;
use crate::orchestrator::Orchestrator;

fn battery(percentage: u8, status: BatteryStatus) -> BatteryInfo {
    BatteryInfo {
        percentage: Some(percentage),
        level: if status == BatteryStatus::Full {
            BatteryLevel::Full
        } else {
            BatteryLevel::Good
        },
        status,
    }
}

fn refresh(orch: &mut Orchestrator, reading: BatteryInfo, online: bool, codename: &str) {
    let mut inventory = direct_inventory(Some("ABC123"), [1, 2, 3, 4]);
    let paired = &mut inventory.paired[0];
    paired.battery = Some(reading);
    paired.online = online;
    paired.codename = Some(codename.to_owned());
    orch.refresh_inventory(&[inventory], &[], false);
}

fn wired(config: Config) -> (Orchestrator, Arc<ObservableState>) {
    let observable = Arc::new(ObservableState::new("test".to_owned()));
    (
        Orchestrator::new(config, Arc::clone(&observable)),
        observable,
    )
}

fn expect_popup(popup: &ChargingPopup, event: ChargingEvent, percentage: u8, name: &str) {
    assert_eq!(popup.event, event);
    assert_eq!(popup.percentage, Some(percentage));
    assert_eq!(popup.name.as_deref(), Some(name));
    assert_eq!(popup.kind, DeviceKind::Mouse);
}

#[tokio::test(start_paused = true)]
async fn the_phase_at_startup_is_not_a_charging_popup() {
    for status in [
        BatteryStatus::Charging,
        BatteryStatus::Discharging,
        BatteryStatus::Full,
    ] {
        let (mut orch, observable) = wired(Config::default());
        refresh(&mut orch, battery(80, status), true, "MX Anywhere 3S");
        let observed = observable.charging().observe(0).await;
        assert!(observed.shown.is_none(), "{status:?} {observed:?}");
        assert_eq!(observed.generation, 1);
    }
}

#[tokio::test(start_paused = true)]
async fn a_device_that_connects_does_not_announce_the_phase_it_arrives_with() {
    let (mut orch, observable) = wired(Config::default());
    refresh(
        &mut orch,
        battery(40, BatteryStatus::Discharging),
        false,
        "MX Anywhere 3S",
    );
    refresh(
        &mut orch,
        battery(40, BatteryStatus::Charging),
        true,
        "MX Anywhere 3S",
    );
    assert!(observable.charging().observe(0).await.shown.is_none());
}

#[tokio::test(start_paused = true)]
async fn plugging_in_unplugging_and_finishing_each_show_at_once() {
    let mut config = Config::default();
    config.app_settings.language = Some("de".to_owned());
    let (mut orch, observable) = wired(config);

    refresh(
        &mut orch,
        battery(80, BatteryStatus::Discharging),
        true,
        "MX Anywhere 3S",
    );
    refresh(
        &mut orch,
        battery(80, BatteryStatus::Charging),
        true,
        "MX Anywhere 3S",
    );
    let charging = observable.charging().observe(0).await;
    expect_popup(
        &charging.shown.expect("charging"),
        ChargingEvent::Started,
        80,
        "MX Anywhere 3S",
    );
    assert_eq!(charging.language.as_deref(), Some("de"));

    refresh(
        &mut orch,
        battery(100, BatteryStatus::Full),
        true,
        "MX Anywhere 3S",
    );
    let full = observable.charging().observe(charging.generation).await;
    expect_popup(
        &full.shown.expect("full"),
        ChargingEvent::Full,
        100,
        "MX Anywhere 3S",
    );

    refresh(
        &mut orch,
        battery(100, BatteryStatus::Discharging),
        true,
        "MX Anywhere 3S",
    );
    let unplugged = observable.charging().observe(full.generation).await;
    expect_popup(
        &unplugged.shown.expect("unplugged"),
        ChargingEvent::Stopped,
        100,
        "MX Anywhere 3S",
    );
}

#[tokio::test(start_paused = true)]
async fn a_refresh_that_repeats_the_phase_shows_nothing_new() {
    let (mut orch, observable) = wired(Config::default());
    refresh(
        &mut orch,
        battery(80, BatteryStatus::Discharging),
        true,
        "MX Anywhere 3S",
    );
    refresh(
        &mut orch,
        battery(80, BatteryStatus::Charging),
        true,
        "MX Anywhere 3S",
    );
    let charging = observable.charging().observe(0).await;
    assert!(charging.shown.is_some(), "the plug-in shows at once");
    refresh(
        &mut orch,
        battery(81, BatteryStatus::Charging),
        true,
        "MX Anywhere 3S",
    );
    let after = observable.charging().observe(charging.generation).await;
    assert!(
        after.generation == charging.generation || after.shown.is_none(),
        "the repeat is not a second popup: {after:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn the_popup_is_quiet_while_the_user_has_it_off() {
    let mut config = Config::default();
    config.app_settings.charging_notifications = false;
    let (mut orch, observable) = wired(config);
    refresh(
        &mut orch,
        battery(55, BatteryStatus::Discharging),
        true,
        "MX Mechanical",
    );
    refresh(
        &mut orch,
        battery(55, BatteryStatus::Charging),
        true,
        "MX Mechanical",
    );
    let quiet = observable.charging().observe(0).await;
    assert!(quiet.shown.is_none());
    assert_eq!(quiet.generation, 1);
}

/// A battery broadcast from the direct test device, as the watcher forwards it.
fn broadcast(orch: &mut Orchestrator, reading: BatteryInfo) {
    orch.apply_battery_broadcast(0x046d, 0xb023, openlogi_hid::DIRECT_DEVICE_INDEX, reading);
}

#[tokio::test(start_paused = true)]
async fn a_charging_broadcast_shows_without_waiting_for_a_refresh() {
    let (mut orch, observable) = wired(Config::default());
    refresh(
        &mut orch,
        battery(80, BatteryStatus::Discharging),
        true,
        "MX Anywhere 3S",
    );
    broadcast(&mut orch, battery(80, BatteryStatus::Charging));
    let charging = observable.charging().observe(0).await;
    expect_popup(
        &charging.shown.expect("plugged in"),
        ChargingEvent::Started,
        80,
        "MX Anywhere 3S",
    );
    // The published inventory carries the reading too.
    let published = observable.read(|snapshot| snapshot.inventory[0].paired[0].battery.clone());
    assert_eq!(published, Some(battery(80, BatteryStatus::Charging)));

    broadcast(&mut orch, battery(80, BatteryStatus::Discharging));
    let unplugged = observable.charging().observe(charging.generation).await;
    expect_popup(
        &unplugged.shown.expect("unplugged"),
        ChargingEvent::Stopped,
        80,
        "MX Anywhere 3S",
    );

    // The reconciliation that follows confirms the phase: no second popup.
    refresh(
        &mut orch,
        battery(80, BatteryStatus::Discharging),
        true,
        "MX Anywhere 3S",
    );
    let after = observable.charging().observe(unplugged.generation).await;
    assert!(
        after.generation == unplugged.generation || after.shown.is_none(),
        "the confirming refresh is not a second popup: {after:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn a_broadcast_before_any_snapshot_or_from_an_unknown_device_shows_nothing() {
    let (mut orch, observable) = wired(Config::default());
    broadcast(&mut orch, battery(80, BatteryStatus::Charging));
    refresh(
        &mut orch,
        battery(80, BatteryStatus::Discharging),
        true,
        "MX Anywhere 3S",
    );
    // Another node, and another slot on this one.
    orch.apply_battery_broadcast(0x046d, 0xc548, 1, battery(80, BatteryStatus::Charging));
    orch.apply_battery_broadcast(0x046d, 0xb023, 1, battery(80, BatteryStatus::Charging));
    let observed = observable.charging().observe(0).await;
    assert!(observed.shown.is_none(), "{observed:?}");
}

#[tokio::test(start_paused = true)]
async fn a_broadcast_from_an_offline_device_shows_nothing() {
    let (mut orch, observable) = wired(Config::default());
    refresh(
        &mut orch,
        battery(80, BatteryStatus::Discharging),
        false,
        "MX Anywhere 3S",
    );
    broadcast(&mut orch, battery(80, BatteryStatus::Charging));
    assert!(observable.charging().observe(0).await.shown.is_none());
}
