//! Charging transitions the overlay announces.
//!
//! A device's first charging state — at agent startup, or the first reading
//! after it connects — is a baseline, not a change. Every later change of
//! charging phase is announced on the first reading that shows it. A reading
//! of the phase already announced shows nothing. `Charging` and
//! `ChargingSlow` are one phase, and a full device that dips back into
//! charging on the cable is still full: it tops itself up without ever
//! leaving power, so only a discharge ends the full phase.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use openlogi_core::config::AppSettings;
use openlogi_core::device::{BatteryInfo, BatteryStatus, DeviceKind};
use openlogi_ipc::{ChargingEvent, ChargingObservation, ChargingPopup, Generation, OBSERVE_HOLD};
use tokio::sync::watch;

use crate::backlight_overlay::DISPLAY_LIFETIME;

/// One online device's latest battery, addressed by the key the agent already
/// uses for that device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChargingSample {
    /// Stable id for this connection. A device that leaves and comes back is
    /// a new baseline.
    pub id: String,
    /// The name to show: the user's name, else the tidied codename.
    pub name: Option<String>,
    /// Kind, so the overlay can name a device that has neither.
    pub kind: DeviceKind,
    /// The latest reading. `None` keeps a device that is online but has not
    /// reported a battery on this pass.
    pub battery: Option<BatteryInfo>,
}

/// The three phases a popup can announce. Slow charging is [`Self::Charging`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Discharging,
    Charging,
    Full,
}

impl Phase {
    fn from_status(status: BatteryStatus) -> Option<Self> {
        match status {
            BatteryStatus::Discharging => Some(Self::Discharging),
            BatteryStatus::Charging | BatteryStatus::ChargingSlow => Some(Self::Charging),
            BatteryStatus::Full => Some(Self::Full),
            BatteryStatus::Error | BatteryStatus::Unknown => None,
        }
    }

    fn event(self) -> ChargingEvent {
        match self {
            Self::Charging => ChargingEvent::Started,
            Self::Discharging => ChargingEvent::Stopped,
            Self::Full => ChargingEvent::Full,
        }
    }
}

/// Per-device charging phases: the last one each device reported.
#[derive(Default)]
pub struct ChargingSessions {
    devices: HashMap<String, Phase>,
}

impl ChargingSessions {
    /// Take the devices that are online now. A device missing from `samples`
    /// has disconnected; its next reading is a baseline. Returns a popup for
    /// every device whose phase differs from its last reading.
    pub fn note(&mut self, samples: &[ChargingSample]) -> Vec<ChargingPopup> {
        let present: Vec<&str> = samples.iter().map(|sample| sample.id.as_str()).collect();
        self.devices.retain(|id, _| present.contains(&id.as_str()));
        samples
            .iter()
            .filter_map(|sample| self.note_one(sample))
            .collect()
    }

    fn note_one(&mut self, sample: &ChargingSample) -> Option<ChargingPopup> {
        let battery = sample.battery.as_ref()?;
        let phase = Phase::from_status(battery.status)?;
        let Some(previous) = self.devices.get_mut(&sample.id) else {
            self.devices.insert(sample.id.clone(), phase);
            return None;
        };
        // Full, then charging again without a discharge between, is the
        // firmware topping up on the cable: the device never left power.
        if *previous == phase || (*previous == Phase::Full && phase == Phase::Charging) {
            return None;
        }
        *previous = phase;
        Some(ChargingPopup {
            name: sample.name.clone(),
            kind: sample.kind,
            event: phase.event(),
            percentage: battery.percentage,
            level: battery.level,
        })
    }
}

struct Cell {
    sessions: ChargingSessions,
    enabled: bool,
    language: Option<String>,
}

/// The charging popup the overlay shows, and the sessions that decide when.
#[derive(Clone)]
pub struct ChargingIndicator {
    cell: Arc<Mutex<Cell>>,
    published: watch::Sender<ChargingObservation>,
}

impl Default for ChargingIndicator {
    fn default() -> Self {
        let (published, _) = watch::channel(ChargingObservation {
            generation: 1,
            shown: None,
            language: None,
        });
        Self {
            cell: Arc::new(Mutex::new(Cell {
                sessions: ChargingSessions::default(),
                enabled: true,
                language: None,
            })),
            published,
        }
    }
}

impl ChargingIndicator {
    /// Record `samples` under `settings`. A phase change is shown at once,
    /// unless the user turned the popup off; the sessions still advance
    /// either way, so turning it back on does not replay an old change.
    ///
    /// Returns the popups shown.
    #[expect(
        clippy::must_use_candidate,
        reason = "showing the popup is the effect; the returned list only describes what was shown"
    )]
    pub fn note(&self, samples: &[ChargingSample], settings: &AppSettings) -> Vec<ChargingPopup> {
        let (events, language) = {
            let mut cell = self.cell();
            cell.enabled = settings.charging_notifications;
            cell.language.clone_from(&settings.language);
            let events = cell.sessions.note(samples);
            (events, cell.language.clone())
        };
        self.publish(&events, language.as_deref())
    }

    /// Long-poll the popup.
    pub async fn observe(&self, since: Generation) -> ChargingObservation {
        let mut receiver = self.published.subscribe();
        let changed = receiver.wait_for(|observation| observation.generation != since);
        match tokio::time::timeout(OBSERVE_HOLD, changed).await {
            Ok(Ok(observation)) => observation.clone(),
            Ok(Err(_)) | Err(_) => self.published.borrow().clone(),
        }
    }

    /// Show `events` when the user wants the popup. Returns those shown.
    fn publish(&self, events: &[ChargingPopup], language: Option<&str>) -> Vec<ChargingPopup> {
        let enabled = self.cell().enabled;
        if !enabled {
            return Vec::new();
        }
        for event in events {
            self.show(event, language.map(str::to_owned));
        }
        events.to_vec()
    }

    fn show(&self, popup: &ChargingPopup, language: Option<String>) {
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

    fn schedule_hide(generation: Generation, published: watch::Sender<ChargingObservation>) {
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
