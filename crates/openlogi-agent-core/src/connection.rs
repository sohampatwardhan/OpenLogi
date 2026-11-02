//! Connect and disconnect transitions the overlay announces.
//!
//! The devices seen on the first inventory after the agent starts are a
//! baseline, as are those on the first inventory after a system resume or a
//! settings-confirmation replay. After that, a device that comes online is
//! announced as connected on the first inventory that shows it.
//!
//! A device that goes offline is announced only once it has stayed away for
//! [`DISCONNECT_GRACE`]. A Bolt or Unifying device reports offline whenever
//! it falls asleep, and HID++ cannot tell that from one switched off, so
//! announcing every drop would put a card up each time the mouse idles. A
//! device back before the grace ends was never announced as gone, and its
//! return is not announced either.
//!
//! There is one card. A disconnect and then a reconnect before the card
//! ends updates that card to the latest state and restarts its time, rather
//! than queueing a second card.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use openlogi_core::config::AppSettings;
use openlogi_core::device::{BatteryInfo, ConnectionLink, DeviceKind};
use openlogi_ipc::{
    ConnectionEvent, ConnectionObservation, ConnectionPopup, Generation, OBSERVE_HOLD,
};
use tokio::sync::watch;

use crate::backlight_overlay::DISPLAY_LIFETIME;

/// How long a device stays offline before its disconnect is announced.
///
/// Long enough to swallow a radio dropping and re-linking while the device
/// dozes off, short enough that switching a device off still reads as
/// immediate.
pub const DISCONNECT_GRACE: Duration = Duration::from_secs(10);

/// One device in the inventory, online or not, addressed by the key the
/// agent already uses for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionSample {
    /// Stable id for the device across passes.
    pub id: String,
    /// The name to show: the user's name, else the tidied codename.
    pub name: Option<String>,
    /// Kind, for the glyph and for a device with no name.
    pub kind: DeviceKind,
    /// How the device reaches the host.
    pub link: ConnectionLink,
    /// Whether it is online on this pass.
    pub online: bool,
    /// Its battery on this pass. `None` keeps the last one seen.
    pub battery: Option<BatteryInfo>,
}

/// Where a device stands between passes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Presence {
    /// Online on the last pass.
    Online,
    /// Offline since `since`, not yet announced.
    Leaving { since: Instant },
    /// Offline and announced as gone, or offline when first seen.
    Away,
}

/// What the sessions remember of a device between passes.
#[derive(Clone, Debug)]
struct Known {
    presence: Presence,
    name: Option<String>,
    kind: DeviceKind,
    link: ConnectionLink,
    battery: Option<BatteryInfo>,
}

impl Known {
    fn popup(&self, event: ConnectionEvent) -> ConnectionPopup {
        ConnectionPopup {
            name: self.name.clone(),
            kind: self.kind,
            event,
            link: self.link,
            battery: self.battery.clone(),
        }
    }

    /// The device went offline at `now`. One already leaving keeps the time
    /// it left.
    fn leave(&mut self, now: Instant) {
        if self.presence == Presence::Online {
            self.presence = Presence::Leaving { since: now };
        }
    }
}

/// Per-device link state: whether each device is online, leaving, or away,
/// and the last name and battery it had.
#[derive(Default)]
pub struct ConnectionSessions {
    devices: HashMap<String, Known>,
    /// Whether a baseline has been taken since startup or the last
    /// [`Self::rebaseline`]. Until it has, a pass records without
    /// announcing.
    baselined: bool,
}

impl ConnectionSessions {
    /// Make the next pass a baseline: it records every device's state and
    /// announces none.
    pub fn rebaseline(&mut self) {
        self.baselined = false;
    }

    /// Take the devices in the inventory at `now`. Returns the disconnects
    /// whose grace ran out by `now` (by id), then a connect for every device
    /// that came back from away or appeared online (in inventory order);
    /// empty on a baseline pass.
    ///
    /// A device that goes offline, or is missing from `samples`, starts
    /// leaving; [`Self::due`] announces it once [`DISCONNECT_GRACE`] has
    /// passed. It is remembered with its last battery, so it is announced
    /// again when it returns.
    pub fn note(&mut self, samples: &[ConnectionSample], now: Instant) -> Vec<ConnectionPopup> {
        if !self.baselined {
            self.baselined = true;
            self.record_baseline(samples);
            return Vec::new();
        }
        let mut popups = self.due(now);
        for sample in samples {
            let previous = self.devices.get(&sample.id);
            let presence = previous.map(|known| known.presence);
            let mut known = Known {
                presence: presence.unwrap_or(Presence::Away),
                name: sample
                    .name
                    .clone()
                    .or_else(|| previous.and_then(|known| known.name.clone())),
                kind: sample.kind,
                link: sample.link,
                battery: sample
                    .battery
                    .clone()
                    .or_else(|| previous.and_then(|known| known.battery.clone())),
            };
            match (presence, sample.online) {
                (None | Some(Presence::Away), true) => {
                    known.presence = Presence::Online;
                    popups.push(known.popup(ConnectionEvent::Connected));
                }
                // Back before the grace ran out: it was never announced as
                // gone, so its return is no news either.
                (Some(Presence::Leaving { .. }), true) => known.presence = Presence::Online,
                (_, false) => known.leave(now),
                (Some(Presence::Online), true) => {}
            }
            self.devices.insert(sample.id.clone(), known);
        }
        for (id, known) in &mut self.devices {
            if !samples.iter().any(|sample| &sample.id == id) {
                known.leave(now);
            }
        }
        popups
    }

    /// Announce every device whose grace ran out by `now`, by id, and mark
    /// it away. Nothing while a baseline is pending: the baseline pass
    /// settles a device that dropped across a resume without announcing it.
    pub fn due(&mut self, now: Instant) -> Vec<ConnectionPopup> {
        if !self.baselined {
            return Vec::new();
        }
        let mut gone: Vec<(&String, &mut Known)> = self
            .devices
            .iter_mut()
            .filter(|(_, known)| match known.presence {
                Presence::Leaving { since } => now.duration_since(since) >= DISCONNECT_GRACE,
                Presence::Online | Presence::Away => false,
            })
            .collect();
        gone.sort_by_key(|(id, _)| *id);
        gone.into_iter()
            .map(|(_, known)| {
                known.presence = Presence::Away;
                known.popup(ConnectionEvent::Disconnected)
            })
            .collect()
    }

    /// When the next leaving device's grace runs out, if any is leaving.
    #[must_use]
    pub fn next_due(&self) -> Option<Instant> {
        self.devices
            .values()
            .filter_map(|known| match known.presence {
                Presence::Leaving { since } => Some(since + DISCONNECT_GRACE),
                Presence::Online | Presence::Away => None,
            })
            .min()
    }

    /// Record `samples` as they are, announcing nothing. A remembered device
    /// missing from them is away.
    fn record_baseline(&mut self, samples: &[ConnectionSample]) {
        for known in self.devices.values_mut() {
            known.presence = Presence::Away;
        }
        for sample in samples {
            let previous = self.devices.get(&sample.id);
            let known = Known {
                presence: if sample.online {
                    Presence::Online
                } else {
                    Presence::Away
                },
                name: sample
                    .name
                    .clone()
                    .or_else(|| previous.and_then(|known| known.name.clone())),
                kind: sample.kind,
                link: sample.link,
                battery: sample
                    .battery
                    .clone()
                    .or_else(|| previous.and_then(|known| known.battery.clone())),
            };
            self.devices.insert(sample.id.clone(), known);
        }
    }
}

struct Cell {
    sessions: ConnectionSessions,
    enabled: bool,
    language: Option<String>,
    /// When the pending departure check fires, so each deadline spawns one
    /// timer rather than one per inventory pass.
    wake: Option<Instant>,
}

/// The connection popup the overlay shows, and the sessions that decide
/// when.
#[derive(Clone)]
pub struct ConnectionIndicator {
    cell: Arc<Mutex<Cell>>,
    published: watch::Sender<ConnectionObservation>,
}

impl Default for ConnectionIndicator {
    fn default() -> Self {
        let (published, _) = watch::channel(ConnectionObservation {
            generation: 1,
            shown: None,
            language: None,
        });
        Self {
            cell: Arc::new(Mutex::new(Cell {
                sessions: ConnectionSessions::default(),
                enabled: true,
                language: None,
                wake: None,
            })),
            published,
        }
    }
}

impl ConnectionIndicator {
    /// Record `samples` under `settings`. A connect is shown at once, and a
    /// disconnect once [`DISCONNECT_GRACE`] has passed, unless the user
    /// turned the popup off; the sessions still advance either way, so
    /// turning it back on does not replay an old change. Several changes
    /// show the last, in the one card.
    ///
    /// Returns the popups this pass shows, in order; the last is the one
    /// showing.
    #[expect(
        clippy::must_use_candidate,
        reason = "showing the popup is the effect; the returned list only describes what was shown"
    )]
    pub fn note(
        &self,
        samples: &[ConnectionSample],
        settings: &AppSettings,
    ) -> Vec<ConnectionPopup> {
        let now = now();
        let mut cell = self.cell();
        cell.enabled = settings.connection_notifications;
        cell.language.clone_from(&settings.language);
        let popups = cell.sessions.note(samples, now);
        self.present(cell, popups)
    }

    /// Make the next [`Self::note`] a baseline, after a system resume or a
    /// pass that replays settings.
    pub fn rebaseline(&self) {
        self.cell().sessions.rebaseline();
    }

    /// Long-poll the popup.
    pub async fn observe(&self, since: Generation) -> ConnectionObservation {
        let mut receiver = self.published.subscribe();
        let changed = receiver.wait_for(|observation| observation.generation != since);
        match tokio::time::timeout(OBSERVE_HOLD, changed).await {
            Ok(Ok(observation)) => observation.clone(),
            Ok(Err(_)) | Err(_) => self.published.borrow().clone(),
        }
    }

    /// Announce the disconnects whose grace has run out.
    fn settle(&self) {
        let mut cell = self.cell();
        cell.wake = None;
        let popups = cell.sessions.due(now());
        self.present(cell, popups);
    }

    /// Show the last of `popups` when the popup is on, and arm the timer
    /// for the next departure still in its grace. Returns what was shown.
    fn present(
        &self,
        mut cell: MutexGuard<'_, Cell>,
        popups: Vec<ConnectionPopup>,
    ) -> Vec<ConnectionPopup> {
        if let Some(at) = cell.sessions.next_due()
            && cell.wake.is_none_or(|wake| at < wake)
            && tokio::runtime::Handle::try_current().is_ok()
        {
            cell.wake = Some(at);
            let indicator = self.clone();
            tokio::spawn(async move {
                tokio::time::sleep_until(tokio::time::Instant::from_std(at)).await;
                indicator.settle();
            });
        }
        if !cell.enabled {
            return Vec::new();
        }
        let language = cell.language.clone();
        drop(cell);
        if let Some(last) = popups.last() {
            self.show(last, language);
        }
        popups
    }

    /// Show `popup` in the one card, replacing whatever it showed, and end
    /// it [`DISPLAY_LIFETIME`] after this, its latest change.
    fn show(&self, popup: &ConnectionPopup, language: Option<String>) {
        let mut generation = 0;
        self.published.send_modify(|observation| {
            observation.generation = observation.generation.wrapping_add(1);
            observation.shown = Some(popup.clone());
            observation.language = language;
            generation = observation.generation;
        });
        let published = self.published.clone();
        Self::schedule_hide(generation, published);
    }

    fn schedule_hide(generation: Generation, published: watch::Sender<ConnectionObservation>) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        tokio::spawn(async move {
            tokio::time::sleep(DISPLAY_LIFETIME).await;
            published.send_if_modified(|observation| {
                if observation.generation != generation {
                    return false;
                }
                observation.generation = observation.generation.wrapping_add(1);
                observation.shown = None;
                true
            });
        });
    }

    fn cell(&self) -> MutexGuard<'_, Cell> {
        self.cell.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The current instant on Tokio's clock, so a paused test clock moves the
/// grace along with the timer that ends it.
fn now() -> Instant {
    tokio::time::Instant::now().into_std()
}

#[cfg(test)]
mod tests;
