//! Connection sessions: baselines, both directions, departures, the
//! disconnect grace, and flaps.

use std::time::{Duration, Instant};

use openlogi_core::config::AppSettings;
use openlogi_core::device::{BatteryInfo, BatteryLevel, BatteryStatus, ConnectionLink, DeviceKind};
use openlogi_ipc::ConnectionEvent;

use super::{ConnectionIndicator, ConnectionSample, ConnectionSessions, DISCONNECT_GRACE};

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

/// A clock for the sans-I/O sessions: `at(s)` is `s` seconds after start.
struct Clock(Instant);

impl Clock {
    fn new() -> Self {
        Self(Instant::now())
    }

    fn at(&self, seconds: u64) -> Instant {
        self.0 + Duration::from_secs(seconds)
    }
}

fn events(
    sessions: &mut ConnectionSessions,
    samples: &[ConnectionSample],
    now: Instant,
) -> Vec<ConnectionEvent> {
    sessions
        .note(samples, now)
        .into_iter()
        .map(|popup| popup.event)
        .collect()
}

fn due(sessions: &mut ConnectionSessions, now: Instant) -> Vec<ConnectionEvent> {
    sessions
        .due(now)
        .into_iter()
        .map(|popup| popup.event)
        .collect()
}

const GRACE: u64 = DISCONNECT_GRACE.as_secs();

#[test]
fn the_first_pass_is_a_baseline_whatever_it_holds() {
    let clock = Clock::new();
    let mut sessions = ConnectionSessions::default();
    let first = [
        sample("mouse", true, Some(80)),
        sample("keyboard", false, None),
    ];
    assert!(events(&mut sessions, &first, clock.at(0)).is_empty());
    assert!(
        events(&mut sessions, &first, clock.at(1)).is_empty(),
        "the same inventory again is no change"
    );
    assert!(due(&mut sessions, clock.at(GRACE * 2)).is_empty());
}

#[test]
fn coming_online_is_connected_at_once() {
    let clock = Clock::new();
    let mut sessions = ConnectionSessions::default();
    events(
        &mut sessions,
        &[sample("mouse", false, Some(60))],
        clock.at(0),
    );

    let shown = sessions.note(&[sample("mouse", true, Some(60))], clock.at(1));
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].event, ConnectionEvent::Connected);
    assert_eq!(shown[0].name.as_deref(), Some("MX mouse"));
    assert_eq!(shown[0].link, ConnectionLink::Bolt);
    assert_eq!(shown[0].battery, Some(battery(60)));
}

#[test]
fn going_offline_is_disconnected_only_once_the_grace_runs_out() {
    let clock = Clock::new();
    let mut sessions = ConnectionSessions::default();
    events(
        &mut sessions,
        &[sample("mouse", true, Some(60))],
        clock.at(0),
    );

    assert!(
        events(&mut sessions, &[sample("mouse", false, None)], clock.at(1)).is_empty(),
        "a drop is not announced while it could still be the device dozing"
    );
    assert_eq!(sessions.next_due(), Some(clock.at(1) + DISCONNECT_GRACE));
    assert!(due(&mut sessions, clock.at(GRACE)).is_empty(), "not yet");
    assert!(
        events(
            &mut sessions,
            &[sample("mouse", false, None)],
            clock.at(GRACE)
        )
        .is_empty(),
        "another offline pass keeps the time it left"
    );

    assert_eq!(
        due(&mut sessions, clock.at(1 + GRACE)),
        [ConnectionEvent::Disconnected]
    );
    assert_eq!(sessions.next_due(), None);
    assert!(
        due(&mut sessions, clock.at(2 + GRACE)).is_empty(),
        "it left once"
    );
    assert_eq!(
        events(&mut sessions, &[sample("mouse", true, None)], clock.at(60)),
        [ConnectionEvent::Connected],
        "a device announced as gone is announced on its return"
    );
}

#[test]
fn a_device_back_within_the_grace_is_neither_gone_nor_back() {
    let clock = Clock::new();
    let mut sessions = ConnectionSessions::default();
    events(
        &mut sessions,
        &[sample("mouse", true, Some(60))],
        clock.at(0),
    );

    events(&mut sessions, &[sample("mouse", false, None)], clock.at(1));
    assert!(
        events(
            &mut sessions,
            &[sample("mouse", true, Some(60))],
            clock.at(5)
        )
        .is_empty(),
        "a sleep and wake inside the grace shows nothing"
    );
    assert_eq!(sessions.next_due(), None);
    assert!(due(&mut sessions, clock.at(1 + GRACE)).is_empty());
}

#[test]
fn an_overdue_departure_is_announced_before_the_return_it_precedes() {
    let clock = Clock::new();
    let mut sessions = ConnectionSessions::default();
    events(&mut sessions, &[sample("mouse", true, None)], clock.at(0));
    events(&mut sessions, &[sample("mouse", false, None)], clock.at(1));

    // The timer has not fired yet, but the grace is over when the device
    // comes back: it really was gone.
    assert_eq!(
        events(
            &mut sessions,
            &[sample("mouse", true, None)],
            clock.at(2 + GRACE)
        ),
        [ConnectionEvent::Disconnected, ConnectionEvent::Connected]
    );
}

#[test]
fn a_device_that_appears_after_the_baseline_has_connected() {
    let clock = Clock::new();
    let mut sessions = ConnectionSessions::default();
    events(&mut sessions, &[], clock.at(0));
    assert_eq!(
        events(&mut sessions, &[sample("mouse", true, None)], clock.at(1)),
        [ConnectionEvent::Connected]
    );
    assert!(
        events(
            &mut sessions,
            &[sample("keyboard", false, None)],
            clock.at(2)
        )
        .is_empty(),
        "a device first seen offline is not a connect"
    );
}

#[test]
fn a_device_that_leaves_is_disconnected_with_its_last_battery_and_returns_connected() {
    let clock = Clock::new();
    let mut sessions = ConnectionSessions::default();
    events(
        &mut sessions,
        &[sample("mouse", true, Some(45))],
        clock.at(0),
    );

    assert!(sessions.note(&[], clock.at(1)).is_empty());
    let gone = sessions.due(clock.at(1 + GRACE));
    assert_eq!(gone.len(), 1);
    assert_eq!(gone[0].event, ConnectionEvent::Disconnected);
    assert_eq!(
        gone[0].battery,
        Some(battery(45)),
        "the last known battery rides on the disconnect"
    );
    assert_eq!(gone[0].name.as_deref(), Some("MX mouse"));
    assert!(sessions.note(&[], clock.at(30)).is_empty(), "it left once");

    assert_eq!(
        events(&mut sessions, &[sample("mouse", true, None)], clock.at(31)),
        [ConnectionEvent::Connected]
    );
}

#[test]
fn an_offline_reading_without_a_battery_keeps_the_last_one() {
    let clock = Clock::new();
    let mut sessions = ConnectionSessions::default();
    events(
        &mut sessions,
        &[sample("mouse", true, Some(30))],
        clock.at(0),
    );
    sessions.note(&[sample("mouse", false, None)], clock.at(1));
    let shown = sessions.due(clock.at(1 + GRACE));
    assert_eq!(shown[0].battery, Some(battery(30)));
}

#[test]
fn a_rebaseline_records_without_announcing() {
    let clock = Clock::new();
    let mut sessions = ConnectionSessions::default();
    events(&mut sessions, &[sample("mouse", true, None)], clock.at(0));
    events(&mut sessions, &[sample("mouse", false, None)], clock.at(1));
    sessions.rebaseline();
    assert!(
        due(&mut sessions, clock.at(1 + GRACE)).is_empty(),
        "a departure pending across a resume waits for the baseline"
    );
    assert!(
        events(
            &mut sessions,
            &[sample("mouse", false, None)],
            clock.at(2 + GRACE)
        )
        .is_empty(),
        "the pass after a resume is a baseline"
    );
    assert!(due(&mut sessions, clock.at(60)).is_empty());
    assert_eq!(
        events(&mut sessions, &[sample("mouse", true, None)], clock.at(61)),
        [ConnectionEvent::Connected],
        "the baseline recorded the offline state"
    );
}

#[tokio::test(start_paused = true)]
async fn the_indicator_shows_a_disconnect_after_the_grace_and_hides_it() {
    let indicator = ConnectionIndicator::default();
    let settings = AppSettings::default();
    assert!(
        indicator
            .note(&[sample("mouse", true, Some(80))], &settings)
            .is_empty()
    );
    assert_eq!(indicator.observe(0).await.generation, 1, "baseline");

    assert!(
        indicator
            .note(&[sample("mouse", false, None)], &settings)
            .is_empty(),
        "nothing shown while the grace runs"
    );
    tokio::time::advance(DISCONNECT_GRACE.saturating_sub(Duration::from_millis(1))).await;
    tokio::task::yield_now().await;
    assert_eq!(indicator.observe(0).await.generation, 1);

    tokio::time::advance(Duration::from_millis(1)).await;
    tokio::task::yield_now().await;
    let shown = indicator.observe(1).await;
    let popup = shown.shown.expect("disconnected");
    assert_eq!(popup.event, ConnectionEvent::Disconnected);
    assert_eq!(popup.battery, Some(battery(80)));

    tokio::time::advance(super::DISPLAY_LIFETIME).await;
    tokio::task::yield_now().await;
    let hidden = indicator.observe(shown.generation).await;
    assert!(hidden.shown.is_none());
}

#[tokio::test(start_paused = true)]
async fn a_doze_inside_the_grace_never_reaches_the_overlay() {
    let indicator = ConnectionIndicator::default();
    let settings = AppSettings::default();
    indicator.note(&[sample("mouse", true, Some(70))], &settings);

    indicator.note(&[sample("mouse", false, None)], &settings);
    tokio::time::advance(DISCONNECT_GRACE / 2).await;
    assert!(
        indicator
            .note(&[sample("mouse", true, Some(70))], &settings)
            .is_empty()
    );
    tokio::time::advance(DISCONNECT_GRACE).await;
    tokio::task::yield_now().await;
    let quiet = indicator.observe(0).await;
    assert_eq!(quiet.generation, 1, "the timer found nothing to announce");
    assert!(quiet.shown.is_none());
}

#[tokio::test(start_paused = true)]
async fn a_return_replaces_the_disconnect_card_and_restarts_its_time() {
    let indicator = ConnectionIndicator::default();
    let settings = AppSettings::default();
    indicator.note(&[sample("mouse", true, Some(70))], &settings);
    indicator.note(&[sample("mouse", false, None)], &settings);
    tokio::time::advance(DISCONNECT_GRACE).await;
    tokio::task::yield_now().await;
    let dropped = indicator.observe(1).await;
    assert_eq!(
        dropped.shown.as_ref().map(|popup| popup.event),
        Some(ConnectionEvent::Disconnected)
    );

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
        &[
            sample("a", false, None),
            sample("b", false, None),
            sample("c", true, None),
        ],
        &settings,
    );
    let shown = indicator.note(
        &[
            sample("a", true, None),
            sample("b", true, None),
            sample("c", true, None),
        ],
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
    indicator.note(&[sample("mouse", false, None)], &muted);
    tokio::time::advance(DISCONNECT_GRACE).await;
    tokio::task::yield_now().await;
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
