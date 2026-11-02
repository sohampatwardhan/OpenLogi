//! Connection popups from inventory refreshes: baselines, and immediate
//! connects and disconnects.

use std::sync::Arc;

use openlogi_core::config::Config;
use openlogi_core::device::{BatteryInfo, BatteryLevel, BatteryStatus, ConnectionLink, DeviceKind};
use openlogi_ipc::ConnectionEvent;

use super::direct_inventory;
use crate::observable::ObservableState;
use crate::orchestrator::Orchestrator;

fn battery(percentage: u8) -> BatteryInfo {
    BatteryInfo {
        percentage: Some(percentage),
        level: BatteryLevel::Good,
        status: BatteryStatus::Discharging,
    }
}

fn inventory(online: bool, reading: Option<BatteryInfo>) -> openlogi_core::device::DeviceInventory {
    let mut inventory = direct_inventory(Some("ABC123"), [1, 2, 3, 4]);
    let paired = &mut inventory.paired[0];
    paired.online = online;
    paired.battery = reading;
    paired.codename = Some("MX Master 3S".to_owned());
    inventory
}

fn refresh(orch: &mut Orchestrator, online: bool, reading: Option<BatteryInfo>) {
    orch.refresh_inventory(&[inventory(online, reading)], &[], false);
}

fn wired(config: Config) -> (Orchestrator, Arc<ObservableState>) {
    let observable = Arc::new(ObservableState::new("test".to_owned()));
    (
        Orchestrator::new(config, Arc::clone(&observable)),
        observable,
    )
}

#[tokio::test(start_paused = true)]
async fn the_devices_at_startup_are_not_a_connection_popup() {
    for online in [true, false] {
        let (mut orch, observable) = wired(Config::default());
        refresh(&mut orch, online, Some(battery(80)));
        let observed = observable.connection().observe(0).await;
        assert!(observed.shown.is_none(), "{online} {observed:?}");
        assert_eq!(observed.generation, 1);
    }
}

#[tokio::test(start_paused = true)]
async fn a_device_going_offline_and_back_shows_disconnected_then_connected() {
    let (mut orch, observable) = wired(Config::default());
    refresh(&mut orch, true, Some(battery(80)));

    refresh(&mut orch, false, None);
    let dropped = observable.connection().observe(1).await;
    let popup = dropped.shown.clone().expect("disconnected");
    assert_eq!(popup.event, ConnectionEvent::Disconnected);
    assert_eq!(popup.name.as_deref(), Some("MX Master 3S"));
    assert_eq!(popup.kind, DeviceKind::Mouse);
    assert_eq!(popup.link, ConnectionLink::Bluetooth);
    assert_eq!(
        popup.battery.and_then(|battery| battery.percentage),
        Some(80),
        "a device that leaves shows its last battery"
    );

    refresh(&mut orch, true, Some(battery(79)));
    let back = observable.connection().observe(dropped.generation).await;
    let popup = back.shown.expect("connected");
    assert_eq!(popup.event, ConnectionEvent::Connected);
    assert_eq!(
        popup.battery.and_then(|battery| battery.percentage),
        Some(79)
    );
}

#[tokio::test(start_paused = true)]
async fn an_unchanged_refresh_shows_nothing() {
    let (mut orch, observable) = wired(Config::default());
    refresh(&mut orch, true, Some(battery(80)));
    refresh(&mut orch, true, Some(battery(78)));
    assert!(observable.connection().observe(0).await.shown.is_none());
}

#[tokio::test(start_paused = true)]
async fn a_resume_and_a_settings_confirmation_are_baselines() {
    let (mut orch, observable) = wired(Config::default());
    refresh(&mut orch, true, Some(battery(80)));

    orch.reapply_after_system_resume();
    refresh(&mut orch, false, None);
    assert!(
        observable.connection().observe(0).await.shown.is_none(),
        "a device that dropped across sleep is not news"
    );

    orch.refresh_inventory_for_settings_confirmation(&[inventory(true, None)], &[], false);
    assert!(
        observable.connection().observe(0).await.shown.is_none(),
        "the settings replay pass records without announcing"
    );

    refresh(&mut orch, false, None);
    assert_eq!(
        observable
            .connection()
            .observe(0)
            .await
            .shown
            .expect("a real change after the baseline")
            .event,
        ConnectionEvent::Disconnected
    );
}

#[tokio::test(start_paused = true)]
async fn the_popup_stays_off_when_the_user_turns_it_off() {
    let mut config = Config::default();
    config.app_settings.connection_notifications = false;
    let (mut orch, observable) = wired(config);
    refresh(&mut orch, true, Some(battery(80)));
    refresh(&mut orch, false, None);
    refresh(&mut orch, true, None);
    assert!(observable.connection().observe(0).await.shown.is_none());
}
