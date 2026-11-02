//! The mock's control console: one command per stdin line changes what the
//! mock reports while the GUI and overlay stay connected.
//!
//! The words are a manual development aid. Tests build [`Command`] values and
//! call [`MockAgent::apply`] directly, so no test depends on the text.

use std::io::{BufRead as _, Write as _};
use std::time::Duration;

use openlogi_core::config::AppSettings;
use openlogi_core::device::{BatteryInfo, BatteryLevel, BatteryStatus, LockKey};
use openlogi_core::hid::BacklightEffect;
use openlogi_hid::{DeviceRoute, WriteError};
use openlogi_ipc::{LockKeyChange, NoticeKind};
use tokio::sync::mpsc;
use tracing::{info, warn};

use super::{MockAgent, State, Target, battery_at};

/// What `help` prints.
const HELP: &str = "\
devices                          number the devices the commands below address
battery <n> <0-100> [charging]   report a paired device's battery
battery <n> clear                report the profile's battery again
online <n> | offline <n>         bring a device online or take it offline
connect <n> | disconnect <n>     the same, named for the connection popup
fail <n> unreachable|error       fail every read and write to an online device
fail <n> clear                   answer reads and writes again
lock caps|num|scroll on|off      a keyboard turns a lock key on or off
backlight <level> <levels>       a keyboard reports its backlight level
effect <name>                    a keyboard switches backlight effect
                                 static contrast breathing waves reaction random
effect off                       the effect key while the backlight is off
radius <px>                      the UI corner radius, 0 for sharp icons
charge <n> [percent]             the device starts charging
unplug <n> [percent]             the device is unplugged and discharging
full <n> [percent]               the device is fully charged, still on power
advance <seconds>                move frozen test time (--fixture runs only)
disconnect                       with no device, close every client connection
help                             show this list";

/// One console command.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Command {
    /// List the commands.
    Help,
    /// Number the devices the other commands address.
    Devices,
    /// Report `reading` for a paired device, or its profile's battery again
    /// when `None`.
    Battery {
        /// Position in the [`Command::Devices`] listing, from 0.
        device: usize,
        reading: Option<BatteryInfo>,
    },
    /// Bring a device online or take it offline: `online`/`connect` and
    /// `offline`/`disconnect` with a device number.
    Link {
        /// Position in the [`Command::Devices`] listing, from 0.
        device: usize,
        online: bool,
    },
    /// Make every read and write to an online device fail with `fault`, or
    /// answer again when `None`.
    Fault {
        /// Position in the [`Command::Devices`] listing, from 0.
        device: usize,
        fault: Option<Fault>,
    },
    /// A keyboard reports a lock key turning on or off.
    LockKey(LockKeyChange),
    /// A keyboard reports its backlight `level` of `levels`, level zero
    /// included.
    Backlight { level: u8, levels: u8 },
    /// A keyboard switches to a predefined backlight effect.
    Effect(BacklightEffect),
    /// The effect key, while the backlight is off. Shows the off hint
    /// instead of the effect list.
    EffectOff,
    /// The corner radius the overlay should draw, in pixels. `0` is sharp.
    Radius(u8),
    /// Set a device's charging phase. `percentage` keeps the current reading
    /// when `None`.
    Power {
        /// Position in the [`Command::Devices`] listing, from 0.
        device: usize,
        status: BatteryStatus,
        percentage: Option<u8>,
    },
    /// Move frozen test time forward.
    Advance(Duration),
    /// Close every client connection, as an agent restart does.
    Disconnect,
}

/// How a device the console made fail answers a read or write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Fault {
    /// It stops answering, as a device asleep or out of range does.
    Unreachable,
    /// It answers every request with an HID++ error.
    Error,
}

impl Fault {
    /// The error a read or write to `route` fails with.
    pub(super) fn error(self, route: &DeviceRoute) -> WriteError {
        match self {
            Self::Unreachable => WriteError::DeviceUnreachable {
                index: route.device_index(),
            },
            Self::Error => WriteError::Hidpp("the mock console made this device fail".to_owned()),
        }
    }
}

/// Parse one console line. A blank line is no command.
pub(super) fn parse(line: &str) -> Result<Option<Command>, String> {
    let mut words = line.split_whitespace();
    let Some(verb) = words.next() else {
        return Ok(None);
    };
    let command = match verb {
        "help" | "?" => Command::Help,
        "devices" => Command::Devices,
        "battery" => parse_battery(&mut words)?,
        "online" | "offline" | "connect" => Command::Link {
            device: parse_device(words.next())?,
            online: verb != "offline",
        },
        // `disconnect` alone closes the client connections, as it did
        // before devices could be named.
        "disconnect" => match words.next() {
            None => Command::Disconnect,
            device => Command::Link {
                device: parse_device(device)?,
                online: false,
            },
        },
        "fail" => parse_fault(&mut words)?,
        "lock" => parse_lock_key(&mut words)?,
        "backlight" => parse_backlight(&mut words)?,
        "effect" => match words.next() {
            Some("off") => Command::EffectOff,
            word => Command::Effect(parse_effect(word)?),
        },
        "radius" => {
            let radius = words
                .next()
                .and_then(|word| word.parse::<u8>().ok())
                .ok_or("radius needs a whole number of pixels")?;
            Command::Radius(radius)
        }
        "charge" => parse_power(BatteryStatus::Charging, &mut words)?,
        "unplug" => parse_power(BatteryStatus::Discharging, &mut words)?,
        "full" => parse_power(BatteryStatus::Full, &mut words)?,
        "advance" => {
            let seconds = words
                .next()
                .and_then(|word| word.parse::<u64>().ok())
                .ok_or("advance needs a whole number of seconds")?;
            Command::Advance(Duration::from_secs(seconds))
        }
        other => return Err(format!("unknown command `{other}`; type `help`")),
    };
    if let Some(extra) = words.next() {
        return Err(format!("unexpected `{extra}`; type `help`"));
    }
    Ok(Some(command))
}

/// A device number as `devices` lists it, from 1, as a position from 0.
fn parse_device(word: Option<&str>) -> Result<usize, String> {
    word.and_then(|word| word.parse::<usize>().ok())
        .and_then(|number| number.checked_sub(1))
        .ok_or_else(|| "name a device by its number in `devices`".to_owned())
}

fn parse_battery<'a>(words: &mut impl Iterator<Item = &'a str>) -> Result<Command, String> {
    let device = parse_device(words.next())?;
    let reading = match words.next() {
        Some("clear") => None,
        Some(word) => {
            // The range a reading can hold. Whether it is low is
            // `BatteryInfo::needs_attention`'s to say.
            let charge = word
                .parse::<u8>()
                .ok()
                .filter(|charge| *charge <= 100)
                .ok_or("a battery reads 0 to 100 percent")?;
            let status = match words.next() {
                None => BatteryStatus::Discharging,
                Some("charging") => BatteryStatus::Charging,
                Some(other) => return Err(format!("unexpected `{other}`; expected `charging`")),
            };
            Some(battery_at(charge, status))
        }
        None => return Err("battery needs a percentage or `clear`".to_owned()),
    };
    Ok(Command::Battery { device, reading })
}

fn parse_fault<'a>(words: &mut impl Iterator<Item = &'a str>) -> Result<Command, String> {
    let device = parse_device(words.next())?;
    let fault = match words.next() {
        Some("unreachable") => Some(Fault::Unreachable),
        Some("error") => Some(Fault::Error),
        Some("clear") => None,
        _ => return Err("fail needs `unreachable`, `error`, or `clear`".to_owned()),
    };
    Ok(Command::Fault { device, fault })
}

fn parse_lock_key<'a>(words: &mut impl Iterator<Item = &'a str>) -> Result<Command, String> {
    let key = match words.next() {
        Some("caps") => LockKey::CapsLock,
        Some("num") => LockKey::NumLock,
        Some("scroll") => LockKey::ScrollLock,
        _ => return Err("lock needs `caps`, `num`, or `scroll`".to_owned()),
    };
    let engaged = match words.next() {
        Some("on") => true,
        Some("off") => false,
        _ => return Err("lock needs `on` or `off`".to_owned()),
    };
    Ok(Command::LockKey(LockKeyChange { key, engaged }))
}

fn parse_power<'a>(
    status: BatteryStatus,
    words: &mut impl Iterator<Item = &'a str>,
) -> Result<Command, String> {
    let device = parse_device(words.next())?;
    let percentage = match words.next() {
        None => None,
        Some(word) => Some(
            word.parse::<u8>()
                .ok()
                .filter(|charge| *charge <= 100)
                .ok_or("a battery reads 0 to 100 percent")?,
        ),
    };
    Ok(Command::Power {
        device,
        status,
        percentage,
    })
}

fn parse_backlight<'a>(words: &mut impl Iterator<Item = &'a str>) -> Result<Command, String> {
    let mut number = || words.next().and_then(|word| word.parse::<u8>().ok());
    let (Some(level), Some(levels)) = (number(), number()) else {
        return Err("backlight needs a level and the number of levels".to_owned());
    };
    if level >= levels {
        return Err(format!(
            "level {level} is not one of {levels} levels, which count from 0"
        ));
    }
    Ok(Command::Backlight { level, levels })
}

fn parse_effect(word: Option<&str>) -> Result<BacklightEffect, String> {
    let Some(word) = word else {
        return Err(effect_rejected());
    };
    BacklightEffect::CYCLE
        .into_iter()
        .find(|effect| effect.label() == word)
        .ok_or_else(effect_rejected)
}

/// Every effect the backlight key walks, as bits. [`BacklightEffect::None`]
/// is not among them.
fn mock_cycle_bits() -> u16 {
    BacklightEffect::CYCLE
        .into_iter()
        .fold(0, |bits, effect| bits | effect.list_bit())
}

fn effect_rejected() -> String {
    let names = BacklightEffect::CYCLE.map(BacklightEffect::label).join(" ");
    format!("effect needs one of: {names}")
}

impl MockAgent {
    /// Apply `command` under the user's `settings`, as the agent would apply
    /// the event it stands for. Answers what to tell the developer.
    pub(super) async fn apply(
        &self,
        command: Command,
        settings: &AppSettings,
    ) -> Result<String, String> {
        match command {
            Command::Help => Ok(HELP.to_owned()),
            Command::Devices => Ok(listing(&self.state.lock().await.targets())),
            Command::Battery { device, reading } => {
                let route = {
                    let mut state = self.state.lock().await;
                    let target = target(&state, device)?;
                    if !target.paired {
                        return Err(format!("{} reports no battery", target.route));
                    }
                    state.set_battery(target.route.clone(), reading);
                    target.route
                };
                let posted = self.announce_low_batteries(settings).await;
                let charging = self.note_charging(settings).await;
                Ok(format!("{route}: battery set{posted}{charging}"))
            }
            Command::Power {
                device,
                status,
                percentage,
            } => self.power(device, status, percentage, settings).await,
            Command::Link { device, online } => self.set_online(device, online, settings).await,
            Command::Fault { device, fault } => {
                let mut state = self.state.lock().await;
                let route = target(&state, device)?.route;
                state.faults.retain(|(failing, _)| *failing != route);
                let Some(fault) = fault else {
                    return Ok(format!("{route}: reads and writes answer again"));
                };
                let error = fault.error(&route);
                state.faults.push((route.clone(), fault));
                Ok(format!("{route}: reads and writes fail with \"{error}\""))
            }
            Command::LockKey(change) => {
                let state = if change.engaged { "on" } else { "off" };
                if self.lock_keys.announce(change, settings) {
                    Ok(format!("showing {:?} {state}", change.key))
                } else {
                    Ok(format!(
                        "not shown: lock_key_notifications is off, or this platform shows no {:?} indicator",
                        change.key
                    ))
                }
            }
            Command::Backlight { level, levels } => {
                self.backlight.show(level, levels, true);
                Ok(format!("showing backlight level {level} of {levels}"))
            }
            Command::Effect(effect) => {
                let supported = BacklightEffect::in_cycle_order(mock_cycle_bits());
                self.backlight
                    .show_effect(effect, supported, settings.language.clone());
                Ok(format!("showing backlight effect {}", effect.label()))
            }
            Command::EffectOff => {
                self.backlight.show_off_hint();
                Ok("showing the backlight-off effect hint".to_owned())
            }
            Command::Radius(radius) => {
                self.ui_chrome.set_radius(radius);
                Ok(format!("overlay corner radius is {radius}px"))
            }
            Command::Advance(duration) => {
                let mut state = self.state.lock().await;
                state.clock.advance(duration)?;
                Ok(format!("test time is {:?}", state.clock.elapsed()))
            }
            Command::Disconnect => {
                let connections = self.disconnects.receiver_count();
                self.disconnects.send_replace(());
                Ok(format!("closed {connections} client connection(s)"))
            }
        }
    }

    /// Report `status` (and `percentage`, else the device's current charge)
    /// for the device at `device`, as a plug, unplug, or full charge would.
    async fn power(
        &self,
        device: usize,
        status: BatteryStatus,
        percentage: Option<u8>,
        settings: &AppSettings,
    ) -> Result<String, String> {
        let route = {
            let mut state = self.state.lock().await;
            let target = target(&state, device)?;
            if !target.paired {
                return Err(format!("{} reports no battery", target.route));
            }
            let percent = percentage.unwrap_or_else(|| {
                target
                    .battery
                    .as_ref()
                    .and_then(|battery| battery.percentage)
                    .unwrap_or(80)
            });
            let mut reading = battery_at(percent, status);
            if status == BatteryStatus::Full {
                reading.level = BatteryLevel::Full;
            }
            state.set_battery(target.route.clone(), Some(reading));
            target.route
        };
        let posted = self.announce_low_batteries(settings).await;
        let charging = self.note_charging(settings).await;
        Ok(format!("{route}: {status:?}{posted}{charging}"))
    }

    /// Post a notice for each online device whose battery has just come to
    /// need attention, as the agent does after an inventory change. Answers a
    /// note for the reply when it posted any.
    async fn announce_low_batteries(&self, settings: &AppSettings) -> String {
        let due = self.state.lock().await.low_battery_notices(settings);
        let count = due.len();
        for notice in due {
            self.notices.post(notice, settings.language.clone());
        }
        if count == 0 {
            String::new()
        } else {
            format!("; posted {count} low-battery notice(s)")
        }
    }

    /// Feed the online batteries to the charging indicator, as an inventory
    /// refresh does. Answers a note for the reply when a phase changed.
    async fn note_charging(&self, settings: &AppSettings) -> String {
        let samples = self.state.lock().await.charging_samples();
        let shown = self.charging.note(&samples, settings);
        if let Some(popup) = shown.first() {
            return format!("; showing {:?}", popup.event);
        }
        String::new()
    }

    /// Bring the device at `device` online or take it offline, then let the
    /// low-battery, charging, and connection indicators see the change.
    async fn set_online(
        &self,
        device: usize,
        online: bool,
        settings: &AppSettings,
    ) -> Result<String, String> {
        let route = {
            let mut state = self.state.lock().await;
            let route = target(&state, device)?.route;
            let flag = state
                .online_mut(&route)
                .ok_or_else(|| format!("{route} is not a profile device"))?;
            *flag = online;
            route
        };
        let posted = self.announce_low_batteries(settings).await;
        let charging = self.note_charging(settings).await;
        let mut connection = self.note_connections(settings).await;
        if !online && connection.is_empty() && settings.connection_notifications {
            connection = format!(
                "; Disconnected shows in {}s unless it comes back",
                openlogi_agent_core::connection::DISCONNECT_GRACE.as_secs()
            );
        }
        let link = if online { "online" } else { "offline" };
        Ok(format!("{route}: {link}{posted}{charging}{connection}"))
    }

    /// Feed every device's link to the connection indicator, as an
    /// inventory refresh does. Answers a note for the reply when one moved.
    async fn note_connections(&self, settings: &AppSettings) -> String {
        let samples = self.state.lock().await.connection_samples();
        let shown = self.connection.note(&samples, settings);
        if let Some(popup) = shown.last() {
            return format!("; showing {:?} over {:?}", popup.event, popup.link);
        }
        String::new()
    }
}

impl State {
    /// Report `reading` for the paired device `route`, or its profile's
    /// battery again when `None`.
    fn set_battery(&mut self, route: DeviceRoute, reading: Option<BatteryInfo>) {
        self.battery_overrides
            .retain(|(overridden, _)| *overridden != route);
        if let Some(reading) = reading {
            self.battery_overrides.push((route, reading));
        }
    }

    /// Take in every online device's battery as the agent's watch does, and
    /// answer the notices now due.
    fn low_battery_notices(&mut self, settings: &AppSettings) -> Vec<NoticeKind> {
        let mut due = Vec::new();
        for target in self.targets() {
            let (true, Some(battery)) = (target.online, target.battery) else {
                continue;
            };
            if self
                .low_battery
                .take_reading(settings, &target.route.to_string(), &battery)
            {
                due.push(NoticeKind::LowBattery {
                    name: target.name,
                    kind: target.kind,
                    battery,
                });
            }
        }
        due
    }
}

/// The device at `position` in the `devices` listing.
fn target(state: &State, position: usize) -> Result<Target, String> {
    state
        .targets()
        .into_iter()
        .nth(position)
        .ok_or_else(|| format!("no device {}; type `devices`", position + 1))
}

/// The `devices` listing, numbered from 1.
fn listing(targets: &[Target]) -> String {
    let mut lines = Vec::with_capacity(targets.len());
    for (position, target) in targets.iter().enumerate() {
        let link = if target.online { "online" } else { "offline" };
        let battery = target.battery.as_ref().map_or_else(String::new, |battery| {
            let charge = battery.percentage.map_or_else(
                || format!("{:?}", battery.level),
                |percent| format!("{percent}%"),
            );
            format!("  battery {charge} {:?}", battery.status)
        });
        lines.push(format!(
            "{number:>2}  {route}  {name}  {kind:?}  {link}{battery}",
            number = position + 1,
            route = target.route,
            name = target.name.as_deref().unwrap_or("(unnamed)"),
            kind = target.kind,
        ));
    }
    lines.join("\n")
}

/// Read console commands from stdin and apply them in order, replying on
/// stdout. Call it inside the Tokio runtime.
///
/// The read runs on a thread of its own: a blocking stdin read cannot be
/// cancelled, so on the runtime's blocking pool it would hold up shutdown.
/// When stdin closes, the console stops and the mock keeps serving.
pub(super) fn spawn_console(agent: MockAgent) {
    let (sender, mut lines) = mpsc::unbounded_channel::<String>();
    let reader = std::thread::Builder::new()
        .name("mock-console".to_owned())
        .spawn(move || {
            for line in std::io::stdin().lock().lines() {
                let Ok(line) = line else {
                    break;
                };
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
    if let Err(error) = reader {
        warn!(%error, "could not start the control console");
        return;
    }
    info!("control console ready: type `help` and press Enter");
    tokio::spawn(async move {
        while let Some(line) = lines.recv().await {
            let reply = match parse(&line) {
                Ok(None) => continue,
                Ok(Some(command)) => agent.apply(command, &(agent.settings)()).await,
                Err(error) => Err(error),
            };
            print_reply(reply);
        }
        info!("stdin closed: the control console stopped");
    });
}

fn print_reply(reply: Result<String, String>) {
    let mut out = std::io::stdout().lock();
    // A closed stdout leaves no one to tell.
    let _ = match reply {
        Ok(text) => writeln!(out, "{text}"),
        Err(error) => writeln!(out, "error: {error}"),
    };
}
