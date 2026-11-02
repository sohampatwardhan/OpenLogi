//! Connection sessions: baselines, both directions, departures, and flaps.

use openlogi_core::config::AppSettings;
use openlogi_core::device::{BatteryInfo, BatteryLevel, BatteryStatus, ConnectionLink, DeviceKind};
use openlogi_ipc::ConnectionEvent;

use super::{ConnectionIndicator, ConnectionSample, ConnectionSessions};

fn battery(percentage: u8) -> BatteryInfo {
    BatteryInfo {
        percentage: Some(percentage),
        level: BatteryLevel::Good,
        status: BatteryStatus::Discharging,
    }
}

fn sample(id: &str, online: bool, percentage: Option<u8>) -> ConnectionSample {
    ConnectionSample {
        id: id.to_owned(),
        name: Some(format!("MX {id}")),
        kind: DeviceKind::Mouse,
        link: ConnectionLink::Bolt,
        online,
        battery: percentage.map(battery),
    }
}

fn events(sessions: &mut ConnectionSessions, samples: &[ConnectionSample]) -> Vec<ConnectionEvent> {
    sessions
        .note(samples)
        .into_iter()
        .map(|popup| popup.event)
        .collect()
}

#[test]
fn the_first_pass_is_a_baseline_whatever_it_holds() {
    let mut sessions = ConnectionSessions::default();
    let first = [
        sample("mouse", true, Some(80)),
        sample("keyboard", false, None),
    ];
    assert!(events(&mut sessions, &first).is_empty());
    assert!(
        events(&mut sessions, &first).is_empty(),
        "the same inventory again is no change"
    );
}

#[test]
fn coming_online_is_connected_and_going_offline_is_disconnected() {
    let mut sessions = ConnectionSessions::default();
    events(&mut sessions, &[sample("mouse", false, Some(60))]);

    let shown = sessions.note(&[sample("mouse", true, Some(60))]);
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].event, ConnectionEvent::Connected);
    assert_eq!(shown[0].name.as_deref(), Some("MX mouse"));
    assert_eq!(shown[0].link, ConnectionLink::Bolt);
    assert_eq!(shown[0].battery, Some(battery(60)));

    assert_eq!(
        events(&mut sessions, &[sample("mouse", false, None)]),
        [ConnectionEvent::Disconnected]
    );
    assert!(events(&mut sessions, &[sample("mouse", false, None)]).is_empty());
}

#[test]
fn a_device_that_appears_after_the_baseline_has_connected() {
    let mut sessions = ConnectionSessions::default();
    events(&mut sessions, &[]);
    assert_eq!(
        events(&mut sessions, &[sample("mouse", true, None)]),
        [ConnectionEvent::Connected]
    );
    assert!(
        events(&mut sessions, &[sample("keyboard", false, None)])
            .iter()
            .all(|event| *event == ConnectionEvent::Disconnected),
        "a device first seen offline is not a connect"
    );
}

#[test]
fn a_device_that_leaves_is_disconnected_with_its_last_battery_and_returns_connected() {
    let mut sessions = ConnectionSessions::default();
    events(&mut sessions, &[sample("mouse", true, Some(45))]);

    let gone = sessions.note(&[]);
    assert_eq!(gone.len(), 1);
    assert_eq!(gone[0].event, ConnectionEvent::Disconnected);
    assert_eq!(
        gone[0].battery,
        Some(battery(45)),
        "the last known battery rides on the disconnect"
    );
    assert_eq!(gone[0].name.as_deref(), Some("MX mouse"));
    assert!(sessions.note(&[]).is_empty(), "it left once");

    assert_eq!(
        events(&mut sessions, &[sample("mouse", true, None)]),
        [ConnectionEvent::Connected]
    );
}

#[test]
fn an_offline_reading_without_a_battery_keeps_the_last_one() {
    let mut sessions = ConnectionSessions::default();
    events(&mut sessions, &[sample("mouse", true, Some(30))]);
    let shown = sessions.note(&[sample("mouse", false, None)]);
    assert_eq!(shown[0].battery, Some(battery(30)));
}

#[test]
fn a_rebaseline_records_without_announcing() {
    let mut sessions = ConnectionSessions::default();
    events(&mut sessions, &[sample("mouse", true, None)]);
    sessions.rebaseline();
    assert!(
        events(&mut sessions, &[sample("mouse", false, None)]).is_empty(),
        "the pass after a resume is a baseline"
    );
    assert_eq!(
        events(&mut sessions, &[sample("mouse", true, None)]),
        [ConnectionEvent::Connected],
        "the baseline recorded the offline state"
    );
}

#[tokio::test(start_paused = true)]
async fn the_indicator_shows_a_change_at_once_and_hides_it() {
    let indicator = ConnectionIndicator::default();
    let settings = AppSettings::default();
    assert!(
        indicator
            .note(&[sample("mouse", true, Some(80))], &settings)
            .is_empty()
    );
    assert_eq!(indicator.observe(0).await.generation, 1, "baseline");

    let shown_now = indicator.note(&[sample("mouse", false, None)], &settings);
    assert_eq!(shown_now.len(), 1, "shown without waiting");
    let shown = indicator.observe(0).await;
    let popup = shown.shown.expect("disconnected");
    assert_eq!(popup.event, ConnectionEvent::Disconnected);
    assert_eq!(popup.battery, Some(battery(80)));

    tokio::task::yield_now().await;
    tokio::time::advance(super::DISPLAY_LIFETIME).await;
    let hidden = indicator.observe(shown.generation).await;
    assert!(hidden.shown.is_none());
}

#[tokio::test(start_paused = true)]
async fn a_flap_updates_the_one_card_and_restarts_its_time() {
    let indicator = ConnectionIndicator::default();
    let settings = AppSettings::default();
    indicator.note(&[sample("mouse", true, Some(70))], &settings);

    indicator.note(&[sample("mouse", false, None)], &settings);
    let dropped = indicator.observe(0).await;
    assert_eq!(
        dropped.shown.as_ref().map(|popup| popup.event),
        Some(ConnectionEvent::Disconnected)
    );

    tokio::task::yield_now().await;
    tokio::time::advance(super::DISPLAY_LIFETIME / 2).await;
    indicator.note(&[sample("mouse", true, Some(70))], &settings);
    let back = indicator.observe(dropped.generation).await;
    assert_eq!(
        back.generation,
        dropped.generation + 1,
        "the reconnect replaced the card in one step"
    );
    assert_eq!(
        back.shown.as_ref().map(|popup| popup.event),
        Some(ConnectionEvent::Connected)
    );

    // The disconnect's time running out does not close the reconnect card.
    tokio::task::yield_now().await;
    tokio::time::advance(super::DISPLAY_LIFETIME / 2).await;
    tokio::task::yield_now().await;
    let still = indicator.observe(0).await;
    assert_eq!(still.generation, back.generation);
    assert!(still.shown.is_some());

    tokio::time::advance(super::DISPLAY_LIFETIME / 2).await;
    tokio::task::yield_now().await;
    let hidden = indicator.observe(back.generation).await;
    assert!(hidden.shown.is_none());
}

#[tokio::test(start_paused = true)]
async fn several_changes_on_one_pass_show_the_last_in_one_card() {
    let indicator = ConnectionIndicator::default();
    let settings = AppSettings::default();
    indicator.note(
        &[sample("a", true, None), sample("b", false, None)],
        &settings,
    );
    let shown = indicator.note(
        &[sample("a", false, None), sample("b", true, None)],
        &settings,
    );
    assert_eq!(shown.len(), 2);
    let observed = indicator.observe(0).await;
    assert_eq!(observed.generation, 2, "one card change for the pass");
    let popup = observed.shown.expect("a card");
    assert_eq!(popup.name.as_deref(), Some("MX b"));
    assert_eq!(popup.event, ConnectionEvent::Connected);
}

#[tokio::test(start_paused = true)]
async fn the_popup_stays_off_when_the_user_turns_it_off_and_does_not_replay() {
    let indicator = ConnectionIndicator::default();
    let muted = AppSettings {
        connection_notifications: false,
        ..AppSettings::default()
    };
    indicator.note(&[sample("mouse", true, None)], &muted);
    assert!(
        indicator
            .note(&[sample("mouse", false, None)], &muted)
            .is_empty()
    );
    let quiet = indicator.observe(0).await;
    assert!(quiet.shown.is_none());
    assert_eq!(
        quiet.generation, 1,
        "a muted transition does not move the cell"
    );

    indicator.note(&[sample("mouse", false, None)], &AppSettings::default());
    assert_eq!(
        indicator.observe(0).await.generation,
        1,
        "turning the popup on does not replay the transition it missed"
    );

    indicator.note(&[sample("mouse", true, None)], &AppSettings::default());
    let shown = indicator.observe(1).await;
    assert_eq!(
        shown.shown.expect("the next change").event,
        ConnectionEvent::Connected
    );
}
