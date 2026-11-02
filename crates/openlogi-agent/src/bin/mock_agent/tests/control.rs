//! The control console's commands, parsed as a developer types them and
//! applied through the mock as a client would then see them.

use openlogi_ipc::AgentClient;

use super::super::control::{Command, Fault, parse};
use super::super::*;
use super::{demo_state, mouse_route, test_state};

const CAPS_ON: LockKeyChange = LockKeyChange {
    key: LockKey::CapsLock,
    engaged: true,
};

fn discharging(percentage: u8) -> BatteryInfo {
    battery_at(percentage, BatteryStatus::Discharging)
}

fn muted() -> AppSettings {
    AppSettings {
        low_battery_notifications: false,
        lock_key_notifications: false,
        ..AppSettings::default()
    }
}

/// The number, from 0, that the console gives the device `route` names.
async fn position_of(agent: &MockAgent, route: &DeviceRoute) -> usize {
    agent
        .state
        .lock()
        .await
        .targets()
        .iter()
        .position(|target| target.route == *route)
        .expect("the device is listed")
}

/// The paired device `route` names, as a client's snapshot shows it.
async fn observed(agent: &MockAgent, route: &DeviceRoute) -> PairedDevice {
    agent
        .current()
        .await
        .snapshot
        .inventory
        .iter()
        .find_map(|inventory| {
            inventory.paired.iter().find(|device| {
                DeviceRoute::for_slot(inventory, device.slot).as_ref() == Some(route)
            })
        })
        .cloned()
        .expect("the snapshot has the device")
}

async fn read_dpi(agent: &MockAgent) -> Result<DpiInfo, WriteError> {
    agent
        .clone()
        .read_dpi(tarpc::context::current(), mouse_route())
        .await
}

#[test]
fn device_commands_parse() {
    let cases = [
        (
            "battery 1 5",
            Some(Command::Battery {
                device: 0,
                reading: Some(BatteryInfo {
                    percentage: Some(5),
                    level: BatteryLevel::Critical,
                    status: BatteryStatus::Discharging,
                }),
            }),
        ),
        (
            "battery 2 40 charging",
            Some(Command::Battery {
                device: 1,
                reading: Some(BatteryInfo {
                    percentage: Some(40),
                    level: BatteryLevel::Good,
                    status: BatteryStatus::Charging,
                }),
            }),
        ),
        (
            "battery 1 clear",
            Some(Command::Battery {
                device: 0,
                reading: None,
            }),
        ),
        (
            "offline 3",
            Some(Command::Link {
                device: 2,
                online: false,
            }),
        ),
        (
            "  online   3 ",
            Some(Command::Link {
                device: 2,
                online: true,
            }),
        ),
        (
            "fail 1 unreachable",
            Some(Command::Fault {
                device: 0,
                fault: Some(Fault::Unreachable),
            }),
        ),
        (
            "fail 1 error",
            Some(Command::Fault {
                device: 0,
                fault: Some(Fault::Error),
            }),
        ),
        (
            "fail 1 clear",
            Some(Command::Fault {
                device: 0,
                fault: None,
            }),
        ),
    ];
    for (line, expected) in cases {
        assert_eq!(parse(line), Ok(expected), "{line:?}");
    }
}

#[test]
fn indicator_and_session_commands_parse() {
    let cases = [
        ("", None),
        ("   ", None),
        ("help", Some(Command::Help)),
        ("?", Some(Command::Help)),
        ("devices", Some(Command::Devices)),
        ("lock caps on", Some(Command::LockKey(CAPS_ON))),
        (
            "lock scroll off",
            Some(Command::LockKey(LockKeyChange {
                key: LockKey::ScrollLock,
                engaged: false,
            })),
        ),
        (
            "lock num on",
            Some(Command::LockKey(LockKeyChange {
                key: LockKey::NumLock,
                engaged: true,
            })),
        ),
        (
            "backlight 3 4",
            Some(Command::Backlight {
                level: 3,
                levels: 4,
            }),
        ),
        (
            "effect breathing",
            Some(Command::Effect(
                openlogi_core::hid::BacklightEffect::Breathing,
            )),
        ),
        ("effect off", Some(Command::EffectOff)),
        ("radius 0", Some(Command::Radius(0))),
        ("radius 12", Some(Command::Radius(12))),
        (
            "advance 30",
            Some(Command::Advance(Duration::from_secs(30))),
        ),
        ("disconnect", Some(Command::Disconnect)),
    ];
    for (line, expected) in cases {
        assert_eq!(parse(line), Ok(expected), "{line:?}");
    }
}

#[test]
fn malformed_commands_are_refused() {
    for line in [
        "jump",
        "battery",
        "battery 0 5",
        "battery x 5",
        "battery 1",
        "battery 1 101",
        "battery 1 5 slowly",
        "online",
        "offline -1",
        "fail 1",
        "fail 1 maybe",
        "lock caps",
        "lock shift on",
        "lock caps maybe",
        "backlight 4 4",
        "backlight 0 0",
        "backlight 1",
        "backlight 300 400",
        "effect",
        "effect none",
        "effect off now",
        "radius",
        "radius -1",
        "advance",
        "advance -1",
        "advance soon",
        "devices now",
        "disconnect all",
    ] {
        parse(line).expect_err(line);
    }
}

#[tokio::test]
async fn devices_are_numbered_from_one_in_the_listing() {
    let agent = MockAgent::new(test_state());
    let listing = agent
        .apply(Command::Devices, &AppSettings::default())
        .await
        .expect("devices lists");
    let targets = agent.state.lock().await.targets();
    assert_eq!(listing.lines().count(), targets.len());
    let mouse = position_of(&agent, &mouse_route()).await;
    let line = listing.lines().nth(mouse).expect("the mouse has a line");
    assert!(
        line.trim_start().starts_with(&format!("{}  ", mouse + 1)),
        "{line}"
    );
    assert!(line.contains(&mouse_route().to_string()), "{line}");
}

#[tokio::test]
async fn a_battery_reading_is_reported_and_announced_once_per_discharge() {
    let agent = MockAgent::new(test_state());
    let settings = AppSettings::default();
    let mouse = position_of(&agent, &mouse_route()).await;
    let recorded = observed(&agent, &mouse_route()).await.battery;

    let reply = agent
        .apply(
            Command::Battery {
                device: mouse,
                reading: Some(discharging(15)),
            },
            &settings,
        )
        .await
        .expect("battery set");
    assert!(reply.ends_with("posted 1 low-battery notice(s)"), "{reply}");
    assert_eq!(
        observed(&agent, &mouse_route()).await.battery,
        Some(discharging(15))
    );
    let notices = agent.notices.observe(0).await.notices;
    assert_eq!(notices.len(), 1);
    let device = observed(&agent, &mouse_route()).await;
    assert_eq!(
        notices[0].kind,
        NoticeKind::LowBattery {
            name: device.codename.as_deref().map(prettify_codename),
            kind: device.kind,
            battery: discharging(15),
        }
    );

    let reply = agent
        .apply(
            Command::Battery {
                device: mouse,
                reading: Some(discharging(9)),
            },
            &settings,
        )
        .await
        .expect("battery set");
    assert!(!reply.contains("posted"), "the same discharge: {reply}");

    agent
        .apply(
            Command::Battery {
                device: mouse,
                reading: None,
            },
            &settings,
        )
        .await
        .expect("battery cleared");
    assert_eq!(observed(&agent, &mouse_route()).await.battery, recorded);
}

#[tokio::test]
async fn a_low_battery_is_muted_while_the_user_has_the_notices_off() {
    let agent = MockAgent::new(test_state());
    let mouse = position_of(&agent, &mouse_route()).await;
    let low = Command::Battery {
        device: mouse,
        reading: Some(discharging(15)),
    };
    let reply = agent
        .apply(low.clone(), &muted())
        .await
        .expect("battery set");
    assert!(!reply.contains("posted"), "{reply}");
    assert!(agent.notices.observe(0).await.notices.is_empty());

    let reply = agent
        .apply(low, &AppSettings::default())
        .await
        .expect("battery set");
    assert!(reply.ends_with("posted 1 low-battery notice(s)"), "{reply}");
}

#[tokio::test]
async fn an_offline_device_fails_reads_until_it_is_back_and_then_announces_its_battery() {
    let agent = MockAgent::new(test_state());
    let settings = AppSettings::default();
    let mouse = position_of(&agent, &mouse_route()).await;

    agent
        .apply(
            Command::Link {
                device: mouse,
                online: false,
            },
            &settings,
        )
        .await
        .expect("taken offline");
    assert!(!observed(&agent, &mouse_route()).await.online);
    assert_eq!(
        read_dpi(&agent).await,
        Err(WriteError::DeviceUnreachable {
            index: mouse_route().device_index()
        })
    );
    let reply = agent
        .apply(
            Command::Battery {
                device: mouse,
                reading: Some(discharging(15)),
            },
            &settings,
        )
        .await
        .expect("battery set");
    assert!(
        !reply.contains("posted"),
        "an offline reading is not news: {reply}"
    );

    let reply = agent
        .apply(
            Command::Link {
                device: mouse,
                online: true,
            },
            &settings,
        )
        .await
        .expect("brought online");
    assert!(reply.contains("posted 1 low-battery notice(s)"), "{reply}");
    assert!(observed(&agent, &mouse_route()).await.online);
    read_dpi(&agent).await.expect("an online device answers");
}

#[tokio::test]
async fn a_failing_device_stays_online_and_fails_every_read_and_write() {
    let agent = MockAgent::new(test_state());
    let settings = AppSettings::default();
    let mouse = position_of(&agent, &mouse_route()).await;
    let fail = |fault| Command::Fault {
        device: mouse,
        fault,
    };

    agent
        .apply(fail(Some(Fault::Unreachable)), &settings)
        .await
        .expect("fault set");
    assert!(observed(&agent, &mouse_route()).await.online);
    assert_eq!(
        read_dpi(&agent).await,
        Err(WriteError::DeviceUnreachable {
            index: mouse_route().device_index()
        })
    );
    let write = agent
        .clone()
        .set_dpi(tarpc::context::current(), mouse_route(), Dpi::new(800))
        .await;
    assert_eq!(
        write,
        Err(WriteError::DeviceUnreachable {
            index: mouse_route().device_index()
        })
    );

    agent
        .apply(fail(Some(Fault::Error)), &settings)
        .await
        .expect("fault replaced");
    assert!(
        matches!(read_dpi(&agent).await, Err(WriteError::Hidpp(_))),
        "the newer fault replaces the older one"
    );

    agent
        .apply(fail(None), &settings)
        .await
        .expect("fault cleared");
    read_dpi(&agent).await.expect("the device answers again");
}

#[tokio::test]
async fn devices_out_of_range_or_without_a_battery_are_refused() {
    let agent = MockAgent::new(test_state());
    let settings = AppSettings::default();
    let targets = agent.state.lock().await.targets();
    let standalone = targets
        .iter()
        .position(|target| !target.paired)
        .expect("the built-in profile has a standalone device");

    let error = agent
        .apply(
            Command::Battery {
                device: standalone,
                reading: Some(discharging(15)),
            },
            &settings,
        )
        .await
        .expect_err("a standalone device reports no battery");
    assert!(error.contains("reports no battery"), "{error}");

    let error = agent
        .apply(
            Command::Link {
                device: targets.len(),
                online: false,
            },
            &settings,
        )
        .await
        .expect_err("past the listing");
    assert_eq!(
        error,
        format!("no device {}; type `devices`", targets.len() + 1)
    );
}

#[tokio::test]
async fn unpairing_drops_what_the_console_set_for_the_slot() {
    let agent = MockAgent::new(test_state());
    let settings = AppSettings::default();
    let mouse = position_of(&agent, &mouse_route()).await;
    agent
        .apply(
            Command::Battery {
                device: mouse,
                reading: Some(discharging(15)),
            },
            &settings,
        )
        .await
        .expect("battery set");
    agent
        .apply(
            Command::Fault {
                device: mouse,
                fault: Some(Fault::Error),
            },
            &settings,
        )
        .await
        .expect("fault set");

    let mut state = agent.state.lock().await;
    state.unpair(&mouse_route()).expect("the mouse unpairs");
    assert!(state.battery_overrides.is_empty());
    assert!(state.faults.is_empty());
}

#[tokio::test(start_paused = true)]
async fn a_lock_key_shows_only_while_the_user_wants_the_indicator() {
    let agent = MockAgent::new(test_state());
    let reply = agent
        .apply(Command::LockKey(CAPS_ON), &muted())
        .await
        .expect("lock key applied");
    assert!(reply.starts_with("not shown"), "{reply}");
    assert_eq!(agent.lock_keys.observe(0).await.shown, None);

    let reply = agent
        .apply(Command::LockKey(CAPS_ON), &AppSettings::default())
        .await
        .expect("lock key applied");
    assert_eq!(reply, "showing CapsLock on");
    assert_eq!(agent.lock_keys.observe(0).await.shown, Some(CAPS_ON));
}

#[tokio::test(start_paused = true)]
async fn a_backlight_level_reaches_the_overlay() {
    let agent = MockAgent::new(test_state());
    agent
        .apply(
            Command::Backlight {
                level: 2,
                levels: 4,
            },
            &AppSettings::default(),
        )
        .await
        .expect("backlight shown");
    let observed = agent
        .clone()
        .observe_backlight(tarpc::context::current(), 0)
        .await;
    assert_eq!(
        (observed.current_level, observed.levels, observed.visible),
        (2, 4, true)
    );
    assert!(!observed.off_hint);
}

#[tokio::test(start_paused = true)]
async fn a_backlight_effect_reaches_the_overlay() {
    let agent = MockAgent::new(test_state());
    let settings = AppSettings {
        language: Some("ja".to_owned()),
        ..AppSettings::default()
    };
    let reply = agent
        .apply(
            Command::Effect(openlogi_core::hid::BacklightEffect::Waves),
            &settings,
        )
        .await
        .expect("effect shown");
    assert_eq!(reply, "showing backlight effect waves");
    let observed = agent
        .clone()
        .observe_backlight_effect(tarpc::context::current(), 0)
        .await;
    assert_eq!(
        observed.shown,
        Some(openlogi_core::hid::BacklightEffect::Waves)
    );
    assert_eq!(
        observed.supported,
        openlogi_core::hid::BacklightEffect::CYCLE
    );
    assert_eq!(observed.language.as_deref(), Some("ja"));
    let level = agent
        .clone()
        .observe_backlight(tarpc::context::current(), 0)
        .await;
    assert!(
        !level.off_hint,
        "a real effect change is the list, not the off hint"
    );
}

#[tokio::test(start_paused = true)]
async fn effect_off_shows_the_hint_and_not_the_effect_list() {
    let agent = MockAgent::new(test_state());
    agent
        .apply(
            Command::Effect(openlogi_core::hid::BacklightEffect::Waves),
            &AppSettings::default(),
        )
        .await
        .expect("effect shown");
    let listed = agent
        .clone()
        .observe_backlight_effect(tarpc::context::current(), 0)
        .await;

    let reply = agent
        .apply(Command::EffectOff, &AppSettings::default())
        .await
        .expect("off hint shown");
    assert_eq!(reply, "showing the backlight-off effect hint");
    let hint = agent
        .clone()
        .observe_backlight(tarpc::context::current(), 0)
        .await;
    assert!(hint.visible);
    assert!(hint.off_hint);
    assert_eq!((hint.current_level, hint.levels), (0, 8));
    let closed = agent
        .clone()
        .observe_backlight_effect(tarpc::context::current(), listed.generation)
        .await;
    assert_eq!(closed.shown, None);
}

#[tokio::test]
async fn radius_reaches_the_overlay() {
    let agent = MockAgent::new(test_state());
    let initial = agent
        .clone()
        .observe_ui_chrome(tarpc::context::current(), 0)
        .await;
    assert_eq!(initial.radius_px, 6);
    let reply = agent
        .apply(Command::Radius(0), &AppSettings::default())
        .await
        .expect("radius applied");
    assert_eq!(reply, "overlay corner radius is 0px");
    let sharp = agent
        .clone()
        .observe_ui_chrome(tarpc::context::current(), initial.generation)
        .await;
    assert_eq!(sharp.radius_px, 0);
}

#[tokio::test]
async fn advance_moves_test_time_and_refuses_demo_time() {
    let agent = MockAgent::new(test_state());
    let before = agent.current().await.snapshot.foreground;
    agent
        .apply(
            Command::Advance(FOREGROUND_SWITCH_PERIOD),
            &AppSettings::default(),
        )
        .await
        .expect("test time moves");
    assert_ne!(agent.current().await.snapshot.foreground, before);

    let error = agent
        .apply(Command::Advance(Duration::MAX), &AppSettings::default())
        .await
        .expect_err("test time cannot pass its range");
    assert!(error.contains("range"), "{error}");
    assert_eq!(
        agent.state.lock().await.clock.elapsed(),
        FOREGROUND_SWITCH_PERIOD
    );

    let demo = MockAgent::new(demo_state());
    let error = demo
        .apply(
            Command::Advance(FOREGROUND_SWITCH_PERIOD),
            &AppSettings::default(),
        )
        .await
        .expect_err("demo time follows the wall clock");
    assert!(error.contains("--fixture"), "{error}");
}

/// Open an in-memory client connection to `agent`, held as the socket loop
/// holds one.
fn connect(agent: &MockAgent) -> (AgentClient, tokio::task::JoinHandle<()>) {
    let (client_transport, server_transport) = tarpc::transport::channel::unbounded();
    let held = agent.clone();
    let connection = tokio::spawn(async move {
        let requests = BaseChannel::with_defaults(server_transport)
            .execute(held.clone().serve())
            .for_each(|response| async move {
                tokio::spawn(response);
            });
        held.hold_connection(requests).await;
    });
    let client = AgentClient::new(tarpc::client::Config::default(), client_transport).spawn();
    (client, connection)
}

#[tokio::test]
async fn disconnect_closes_open_connections_and_admits_new_ones() {
    let agent = MockAgent::new(test_state());
    let (client, connection) = connect(&agent);
    assert_eq!(
        client
            .protocol_version(tarpc::context::current())
            .await
            .expect("served before the disconnect"),
        PROTOCOL_VERSION
    );

    let reply = agent
        .apply(Command::Disconnect, &AppSettings::default())
        .await
        .expect("disconnected");
    assert_eq!(reply, "closed 1 client connection(s)");
    tokio::time::timeout(Duration::from_secs(5), connection)
        .await
        .expect("the disconnect ends the connection")
        .expect("the connection task finishes");
    client
        .protocol_version(tarpc::context::current())
        .await
        .expect_err("the connection is closed");

    let (later, _connection) = connect(&agent);
    assert_eq!(
        later
            .protocol_version(tarpc::context::current())
            .await
            .expect("a connection opened after the disconnect is served"),
        PROTOCOL_VERSION
    );
}

#[tokio::test]
async fn the_demo_caps_lock_follows_the_users_settings() {
    let mut agent = MockAgent::new(demo_state());
    agent.settings = || AppSettings {
        language: Some("de".to_owned()),
        ..AppSettings::default()
    };
    let shown = agent.observe_lock_keys(tarpc::context::current(), 0).await;
    assert_eq!(shown.shown, Some(CAPS_ON));
    assert_eq!(shown.language.as_deref(), Some("de"));

    let mut agent = MockAgent::new(demo_state());
    agent.settings = muted;
    let hidden = agent.observe_lock_keys(tarpc::context::current(), 0).await;
    assert_eq!(hidden.shown, None);
}

#[tokio::test(start_paused = true)]
async fn charge_unplug_and_full_commands_show_after_they_stay() {
    let agent = MockAgent::new(test_state());
    let mouse = position_of(&agent, &mouse_route()).await;
    let before = agent
        .clone()
        .observe_charging(tarpc::context::current(), 0)
        .await;
    assert!(
        before.shown.is_none(),
        "the profile's charging state is not a popup"
    );

    let charge = agent
        .apply(
            Command::Power {
                device: mouse,
                status: BatteryStatus::Charging,
                percentage: Some(80),
            },
            &AppSettings::default(),
        )
        .await
        .expect("charge");
    assert!(charge.contains("showing Started"), "{charge}");
    let charging = agent
        .clone()
        .observe_charging(tarpc::context::current(), before.generation)
        .await;
    assert_eq!(
        charging.shown.expect("charging").event,
        openlogi_ipc::ChargingEvent::Started
    );

    let unplug = agent
        .apply(
            Command::Power {
                device: mouse,
                status: BatteryStatus::Discharging,
                percentage: Some(80),
            },
            &AppSettings::default(),
        )
        .await
        .expect("unplug");
    assert!(unplug.contains("showing Stopped"), "{unplug}");
    let stopped = agent
        .clone()
        .observe_charging(tarpc::context::current(), charging.generation)
        .await;
    let unplugged = stopped.shown.expect("unplugged");
    assert_eq!(unplugged.event, openlogi_ipc::ChargingEvent::Stopped);
    assert_eq!(unplugged.percentage, Some(80));

    let full = agent
        .apply(
            Command::Power {
                device: mouse,
                status: BatteryStatus::Full,
                percentage: Some(100),
            },
            &AppSettings::default(),
        )
        .await
        .expect("full");
    assert!(full.contains("showing Full"), "{full}");
    let done = agent
        .clone()
        .observe_charging(tarpc::context::current(), stopped.generation)
        .await;
    assert_eq!(
        done.shown.expect("full").event,
        openlogi_ipc::ChargingEvent::Full
    );
}

#[test]
fn charge_unplug_and_full_parse() {
    assert!(matches!(
        parse("charge 1 80").expect("parse"),
        Some(Command::Power {
            device: 0,
            status: BatteryStatus::Charging,
            percentage: Some(80),
        })
    ));
    assert!(matches!(
        parse("unplug 2").expect("parse"),
        Some(Command::Power {
            device: 1,
            status: BatteryStatus::Discharging,
            percentage: None,
        })
    ));
    assert!(matches!(
        parse("full 1 100").expect("parse"),
        Some(Command::Power {
            device: 0,
            status: BatteryStatus::Full,
            percentage: Some(100),
        })
    ));
}

#[test]
fn connect_and_disconnect_name_a_device_and_disconnect_alone_closes_clients() {
    assert_eq!(
        parse("connect 2"),
        Ok(Some(Command::Link {
            device: 1,
            online: true,
        }))
    );
    assert_eq!(
        parse("disconnect 1"),
        Ok(Some(Command::Link {
            device: 0,
            online: false,
        }))
    );
    assert_eq!(parse("disconnect"), Ok(Some(Command::Disconnect)));
    for line in ["connect", "connect x", "disconnect 0", "disconnect 1 2"] {
        assert!(parse(line).is_err(), "{line:?}");
    }
}

#[tokio::test(start_paused = true)]
async fn disconnect_and_connect_commands_show_the_connection_popup() {
    let agent = MockAgent::new(test_state());
    let mouse = position_of(&agent, &mouse_route()).await;
    let before = agent
        .clone()
        .observe_connection(tarpc::context::current(), 0)
        .await;
    assert!(
        before.shown.is_none(),
        "the profile's devices are the baseline, not a popup"
    );

    let gone = agent
        .apply(
            Command::Link {
                device: mouse,
                online: false,
            },
            &AppSettings::default(),
        )
        .await
        .expect("disconnect");
    assert!(gone.contains("Disconnected shows in"), "{gone}");
    tokio::time::advance(openlogi_agent_core::connection::DISCONNECT_GRACE).await;
    tokio::task::yield_now().await;
    let dropped = agent
        .clone()
        .observe_connection(tarpc::context::current(), before.generation)
        .await;
    let popup = dropped.shown.expect("disconnected");
    assert_eq!(popup.event, openlogi_ipc::ConnectionEvent::Disconnected);

    let back = agent
        .apply(
            Command::Link {
                device: mouse,
                online: true,
            },
            &AppSettings::default(),
        )
        .await
        .expect("connect");
    assert!(back.contains("showing Connected"), "{back}");
    let connected = agent
        .clone()
        .observe_connection(tarpc::context::current(), dropped.generation)
        .await;
    assert_eq!(
        connected.shown.expect("connected").event,
        openlogi_ipc::ConnectionEvent::Connected
    );

    let quiet = AppSettings {
        connection_notifications: false,
        ..AppSettings::default()
    };
    let muted = agent
        .apply(
            Command::Link {
                device: mouse,
                online: false,
            },
            &quiet,
        )
        .await
        .expect("disconnect while muted");
    assert!(!muted.contains("Disconnected"), "{muted}");
}
