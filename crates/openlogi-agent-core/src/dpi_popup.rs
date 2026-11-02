//! The DPI popup: the DPI a preset action set, and precision mode turning on
//! or off.
//!
//! Only a press shows it. The actions that pick a preset
//! (`CycleDpiPresets`, `NextDpiPreset`, `PreviousDpiPreset`,
//! `SetDpiPreset`) show the DPI once the mouse accepted it, and a precision
//! press shows the new state at once, since precision mode is the agent's own
//! state. A DPI the settings app writes, a held DPI shift, and the settings
//! replayed on connect show nothing.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use openlogi_core::config::AppSettings;
use openlogi_ipc::{DpiObservation, DpiPopup, Generation, OBSERVE_HOLD};
use tokio::sync::watch;

use crate::backlight_overlay::DISPLAY_LIFETIME;

struct Cell {
    enabled: bool,
    language: Option<String>,
}

/// The DPI popup the overlay shows.
#[derive(Clone)]
pub struct DpiIndicator {
    cell: Arc<Mutex<Cell>>,
    published: watch::Sender<DpiObservation>,
}

impl Default for DpiIndicator {
    fn default() -> Self {
        // Generation 1, not 0: 0 is the client sentinel for "I have seen
        // nothing", so a first `observe(0)` answers at once.
        let (published, _) = watch::channel(DpiObservation {
            generation: 1,
            shown: None,
            language: None,
        });
        Self {
            cell: Arc::new(Mutex::new(Cell {
                enabled: true,
                language: None,
            })),
            published,
        }
    }
}

impl DpiIndicator {
    /// Take the user's switch for this popup and the UI language.
    pub fn configure(&self, settings: &AppSettings) {
        let mut cell = self.cell();
        cell.enabled = settings.dpi_notifications;
        cell.language.clone_from(&settings.language);
    }

    /// Show `popup` for the indicators' display lifetime, replacing whatever was showing,
    /// unless the user turned the popup off. Returns whether it is shown.
    ///
    /// Call it inside the agent's runtime: a task ends the popup when its
    /// time is up.
    #[expect(
        clippy::must_use_candidate,
        reason = "showing the popup is the effect; the flag only reports it"
    )]
    pub fn show(&self, popup: DpiPopup) -> bool {
        let (enabled, language) = {
            let cell = self.cell();
            (cell.enabled, cell.language.clone())
        };
        if !enabled {
            return false;
        }
        let mut generation = 0;
        self.published.send_modify(|observation| {
            observation.generation = observation.generation.wrapping_add(1);
            observation.shown = Some(popup);
            observation.language = language;
            generation = observation.generation;
        });
        if tokio::runtime::Handle::try_current().is_err() {
            return true;
        }
        let published = self.published.clone();
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
        true
    }

    /// Long-poll the popup.
    pub async fn observe(&self, since: Generation) -> DpiObservation {
        let mut receiver = self.published.subscribe();
        let changed = receiver.wait_for(|observation| observation.generation != since);
        match tokio::time::timeout(OBSERVE_HOLD, changed).await {
            Ok(Ok(observation)) => observation.clone(),
            Ok(Err(_)) | Err(_) => self.published.borrow().clone(),
        }
    }

    fn cell(&self) -> MutexGuard<'_, Cell> {
        self.cell.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests;
