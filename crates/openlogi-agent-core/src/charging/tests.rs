//! Charging sessions: baselines, every direction, and repeated readings.

use openlogi_core::config::AppSettings;
use openlogi_core::device::{BatteryInfo, BatteryLevel, BatteryStatus, DeviceKind};
use openlogi_ipc::ChargingEvent;

use super::{ChargingIndicator, ChargingSample, ChargingSessions};

fn battery(percentage: u8, status: BatteryStatus) -> BatteryInfo {
    BatteryInfo {
        percentage: Some(percentage),
        level: BatteryLevel::Good,
        status,
    }
}

fn sample(id: &str, status: BatteryStatus, percentage: u8) -> ChargingSample {
    ChargingSample {
        id: id.to_owned(),
        name: Some("MX Anywhere 3S".to_owned()),
        kind: DeviceKind::Mouse,
        battery: Some(battery(percentage, status)),
    }
}

fn note_one(
    sessions: &mut ChargingSessions,
    reading: &ChargingSample,
) -> Option<openlogi_ipc::ChargingPopup> {
    let shown = sessions.note(std::slice::from_ref(reading));
    assert!(shown.len() <= 1, "{shown:?}");
    shown.into_iter().next()
}

#[test]
fn the_first_reading_is_not_a_change_in_any_phase() {
    for status in [
        BatteryStatus::Discharging,
        BatteryStatus::Charging,
        BatteryStatus::ChargingSlow,
        BatteryStatus::Full,
    ] {
        let mut sessions = ChargingSessions::default();
        let reading = sample("mouse", status, 80);
        assert!(
            note_one(&mut sessions, &reading).is_none(),
            "{status:?} at connect is a baseline"
        );
        assert!(
            note_one(&mut sessions, &reading).is_none(),
            "{status:?} read again is still the baseline"
        );
    }
}

#[test]
fn a_device_that_connects_later_does_not_announce_its_first_reading() {
    let mut sessions = ChargingSessions::default();
    let mouse = sample("mouse", BatteryStatus::Discharging, 70);
    note_one(&mut sessions, &mouse);

    let keyboard = sample("keyboard", BatteryStatus::Charging, 40);
    let shown = sessions.note(&[mouse, keyboard]);
    assert!(
        shown.is_empty(),
        "a device's first sighting is a baseline, {shown:?}"
    );
}

#[test]
fn every_phase_change_is_announced_on_its_first_reading() {
    let cases = [
        (
            BatteryStatus::Discharging,
            BatteryStatus::Charging,
            ChargingEvent::Started,
        ),
        (
            BatteryStatus::Discharging,
            BatteryStatus::ChargingSlow,
            ChargingEvent::Started,
        ),
        (
            BatteryStatus::Discharging,
            BatteryStatus::Full,
            ChargingEvent::Full,
        ),
        (
            BatteryStatus::Charging,
            BatteryStatus::Discharging,
            ChargingEvent::Stopped,
        ),
        (
            BatteryStatus::Charging,
            BatteryStatus::Full,
            ChargingEvent::Full,
        ),
        (
            BatteryStatus::Full,
            BatteryStatus::Discharging,
            ChargingEvent::Stopped,
        ),
        (
            BatteryStatus::ChargingSlow,
            BatteryStatus::Full,
            ChargingEvent::Full,
        ),
    ];
    for (from, to, event) in cases {
        let mut sessions = ChargingSessions::default();
        note_one(&mut sessions, &sample("mouse", from, 80));
        let shown = note_one(&mut sessions, &sample("mouse", to, 81))
            .unwrap_or_else(|| panic!("{from:?} -> {to:?} is shown at once"));
        assert_eq!(shown.event, event);
        assert_eq!(shown.percentage, Some(81));
        assert_eq!(shown.name.as_deref(), Some("MX Anywhere 3S"));
    }
}

#[test]
fn a_full_device_topping_up_on_the_cable_stays_full() {
    let mut sessions = ChargingSessions::default();
    note_one(&mut sessions, &sample("mouse", BatteryStatus::Charging, 99));
    assert_eq!(
        note_one(&mut sessions, &sample("mouse", BatteryStatus::Full, 100))
            .expect("charge completed")
            .event,
        ChargingEvent::Full
    );
    for status in [BatteryStatus::Charging, BatteryStatus::ChargingSlow] {
        assert!(
            note_one(&mut sessions, &sample("mouse", status, 96)).is_none(),
            "{status:?} after full without unplugging is a top-up"
        );
        assert!(
            note_one(&mut sessions, &sample("mouse", BatteryStatus::Full, 100)).is_none(),
            "full again after a top-up is not a new completion"
        );
    }
    assert_eq!(
        note_one(
            &mut sessions,
            &sample("mouse", BatteryStatus::Discharging, 100)
        )
        .expect("unplugged")
        .event,
        ChargingEvent::Stopped
    );
    assert_eq!(
        note_one(&mut sessions, &sample("mouse", BatteryStatus::Charging, 99))
            .expect("plugged back in")
            .event,
        ChargingEvent::Started,
        "a discharge ends the full phase"
    );
}

#[test]
fn a_repeated_reading_of_the_new_phase_is_not_shown_again() {
    let mut sessions = ChargingSessions::default();
    note_one(&mut sessions, &sample("mouse", BatteryStatus::Charging, 80));
    let unplugged = sample("mouse", BatteryStatus::Discharging, 79);
    assert_eq!(
        note_one(&mut sessions, &unplugged)
            .expect("the unplug")
            .event,
        ChargingEvent::Stopped
    );
    assert!(note_one(&mut sessions, &unplugged).is_none());
    assert!(
        note_one(
            &mut sessions,
            &sample("mouse", BatteryStatus::Discharging, 78)
        )
        .is_none(),
        "the same phase at a lower percentage is not a change"
    );
}

#[test]
fn each_flip_is_its_own_change() {
    let mut sessions = ChargingSessions::default();
    let plugged = sample("mouse", BatteryStatus::Charging, 80);
    let unplugged = sample("mouse", BatteryStatus::Discharging, 80);
    note_one(&mut sessions, &plugged);
    assert_eq!(
        note_one(&mut sessions, &unplugged).expect("unplug").event,
        ChargingEvent::Stopped
    );
    assert_eq!(
        note_one(&mut sessions, &plugged).expect("replug").event,
        ChargingEvent::Started
    );
}

#[test]
fn slow_charging_is_the_same_phase_as_charging() {
    let mut sessions = ChargingSessions::default();
    note_one(&mut sessions, &sample("mouse", BatteryStatus::Charging, 40));
    assert!(
        note_one(
            &mut sessions,
            &sample("mouse", BatteryStatus::ChargingSlow, 41)
        )
        .is_none()
    );
}

#[test]
fn an_unknown_or_error_reading_is_not_a_phase_change() {
    let mut sessions = ChargingSessions::default();
    note_one(&mut sessions, &sample("mouse", BatteryStatus::Charging, 60));
    for status in [BatteryStatus::Error, BatteryStatus::Unknown] {
        assert!(note_one(&mut sessions, &sample("mouse", status, 60)).is_none());
    }
    let shown = note_one(
        &mut sessions,
        &sample("mouse", BatteryStatus::Discharging, 60),
    );
    assert_eq!(
        shown
            .expect("the error did not replace the last phase")
            .event,
        ChargingEvent::Stopped
    );
}

#[test]
fn a_device_that_disconnects_is_a_baseline_when_it_returns() {
    let mut sessions = ChargingSessions::default();
    note_one(&mut sessions, &sample("mouse", BatteryStatus::Charging, 90));
    assert!(sessions.note(&[]).is_empty());
    assert!(
        note_one(
            &mut sessions,
            &sample("mouse", BatteryStatus::Discharging, 88)
        )
        .is_none(),
        "reconnecting reports a baseline, not an unplug"
    );
}

#[tokio::test(start_paused = true)]
async fn the_indicator_shows_a_change_at_once_and_hides_it() {
    let indicator = ChargingIndicator::default();
    assert!(
        indicator
            .note(
                &[sample("mouse", BatteryStatus::Discharging, 80)],
                &AppSettings::default()
            )
            .is_empty()
    );

    let shown_now = indicator.note(
        &[sample("mouse", BatteryStatus::Charging, 80)],
        &AppSettings::default(),
    );
    assert_eq!(shown_now.len(), 1, "shown without waiting");

    let shown = indicator.observe(0).await;
    let popup = shown.shown.expect("charging");
    assert_eq!(popup.event, ChargingEvent::Started);
    assert_eq!(popup.percentage, Some(80));

    tokio::task::yield_now().await;
    tokio::time::advance(super::DISPLAY_LIFETIME).await;
    let hidden = indicator.observe(shown.generation).await;
    assert!(hidden.shown.is_none());
}

#[tokio::test(start_paused = true)]
async fn the_popup_stays_off_when_the_user_turns_it_off_and_does_not_replay() {
    let indicator = ChargingIndicator::default();
    let muted = AppSettings {
        charging_notifications: false,
        ..AppSettings::default()
    };
    indicator.note(&[sample("mouse", BatteryStatus::Discharging, 40)], &muted);
    assert!(
        indicator
            .note(&[sample("mouse", BatteryStatus::Charging, 40)], &muted)
            .is_empty()
    );

    let quiet = indicator.observe(0).await;
    assert!(quiet.shown.is_none());
    assert_eq!(
        quiet.generation, 1,
        "a muted transition does not move the cell"
    );

    indicator.note(
        &[sample("mouse", BatteryStatus::Charging, 40)],
        &AppSettings::default(),
    );
    let still = indicator.observe(0).await;
    assert!(
        still.shown.is_none(),
        "turning the popup on does not replay the transition it missed"
    );

    indicator.note(
        &[sample("mouse", BatteryStatus::Full, 100)],
        &AppSettings::default(),
    );
    let shown = indicator.observe(still.generation).await;
    assert_eq!(
        shown.shown.expect("the next change").event,
        ChargingEvent::Full
    );
}
