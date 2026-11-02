//! Connect and disconnect transitions the overlay announces.
//!
//! The devices seen on the first inventory after the agent starts are a
//! baseline, as are those on the first inventory after a system resume or a
//! settings-confirmation replay. After that, a device that comes online is
//! announced as connected, and one that goes offline or leaves the
//! inventory as disconnected, on the first inventory that shows it. A device
//! that stays online but moves to another link — Easy-Switch from its Bolt
//! receiver to Bluetooth, say — is announced as connected over the new link.
//!
//! A device paired on several links is in the inventory once per link, all
//! under its one key: it is online when any of those links is, and reaches
//! the host over the one that is.
//!
//! There is one card. A quick flap — a disconnect and then a reconnect
//! before the card ends — updates that card to the latest state and restarts
//! its time, rather than queueing a second card.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use openlogi_core::config::AppSettings;
use openlogi_core::device::{BatteryInfo, ConnectionLink, DeviceKind};
use openlogi_ipc::{
    ConnectionEvent, ConnectionObservation, ConnectionPopup, Generation, OBSERVE_HOLD,
};
use tokio::sync::watch;

use crate::backlight_overlay::DISPLAY_LIFETIME;

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

/// What the sessions remember of a device between passes.
#[derive(Clone, Debug)]
struct Known {
    online: bool,
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
}

/// `samples` with one entry per device id, in the order each id first
/// appears. A device in the inventory on several links is online when any of
/// them is, and takes the link, battery and name of the one it is online on.
fn per_device(samples: &[ConnectionSample]) -> Vec<ConnectionSample> {
    let mut devices: Vec<ConnectionSample> = Vec::with_capacity(samples.len());
    for sample in samples {
        match devices.iter_mut().find(|device| device.id == sample.id) {
            None => devices.push(sample.clone()),
            Some(device) if sample.online && !device.online => {
                let name = sample.name.clone().or_else(|| device.name.take());
                let battery = sample.battery.clone().or_else(|| device.battery.take());
                *device = ConnectionSample {
                    name,
                    battery,
                    ..sample.clone()
                };
            }
            Some(device) => {
                if device.name.is_none() {
                    device.name.clone_from(&sample.name);
                }
            }
        }
    }
    devices
}

/// Per-device link state: whether each device was online on the last pass,
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

    /// Take the devices in the inventory now. Returns a popup for every
    /// device whose link moved since the last pass, in inventory order and
    /// then departed devices by id; empty on a baseline pass.
    ///
    /// A device first seen online after the baseline has connected. One that
    /// is missing from `samples` has left; it is remembered as offline, with
    /// its last battery, so it is announced again when it returns.
    pub fn note(&mut self, samples: &[ConnectionSample]) -> Vec<ConnectionPopup> {
        let announce = std::mem::replace(&mut self.baselined, true);
        let samples = per_device(samples);
        let mut popups = Vec::new();
        for sample in &samples {
            let previous = self.devices.get(&sample.id);
            let event = match (
                previous.map(|known| (known.online, known.link)),
                sample.online,
            ) {
                (None | Some((false, _)), true) => Some(ConnectionEvent::Connected),
                (Some((true, link)), true) if link != sample.link => {
                    Some(ConnectionEvent::Connected)
                }
                (Some((true, _)), false) => Some(ConnectionEvent::Disconnected),
                _ => None,
            };
            let known = Known {
                online: sample.online,
                name: sample
                    .name
                    .clone()
                    .or_else(|| previous.and_then(|known| known.name.clone())),
                kind: sample.kind,
                // Offline on every link, the device is still the one that was
                // last online on its old link.
                link: match previous {
                    Some(known) if !sample.online => known.link,
                    _ => sample.link,
                },
                battery: sample
                    .battery
                    .clone()
                    .or_else(|| previous.and_then(|known| known.battery.clone())),
            };
            if let Some(event) = event {
                popups.push(known.popup(event));
            }
            self.devices.insert(sample.id.clone(), known);
        }
        let mut departed: Vec<(&String, &mut Known)> = self
            .devices
            .iter_mut()
            .filter(|(id, known)| known.online && !samples.iter().any(|s| &s.id == *id))
            .collect();
        departed.sort_by_key(|(id, _)| *id);
        for (_, known) in departed {
            known.online = false;
            popups.push(known.popup(ConnectionEvent::Disconnected));
        }
        if announce { popups } else { Vec::new() }
    }
}

struct Cell {
    sessions: ConnectionSessions,
    enabled: bool,
    language: Option<String>,
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
            })),
            published,
        }
    }
}

impl ConnectionIndicator {
    /// Record `samples` under `settings`. A connect or disconnect is shown
    /// at once, unless the user turned the popup off; the sessions still
    /// advance either way, so turning it back on does not replay an old
    /// change. Several changes on one pass show the last, in the one card.
    ///
    /// Returns the popups the changes produced while the popup is on, in
    /// order; the last is the one showing.
    #[expect(
        clippy::must_use_candidate,
        reason = "showing the popup is the effect; the returned list only describes what was shown"
    )]
    pub fn note(
        &self,
        samples: &[ConnectionSample],
        settings: &AppSettings,
    ) -> Vec<ConnectionPopup> {
        let (popups, language) = {
            let mut cell = self.cell();
            cell.enabled = settings.connection_notifications;
            cell.language.clone_from(&settings.language);
            let popups = cell.sessions.note(samples);
            if !cell.enabled {
                return Vec::new();
            }
            (popups, cell.language.clone())
        };
        if let Some(last) = popups.last() {
            self.show(last, language);
        }
        popups
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

#[cfg(test)]
mod tests;
