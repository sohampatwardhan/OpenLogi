//! The corner radius the overlay uses for Material Symbol icons.
//!
//! The desktop resolves `ui_radius` against the active theme and pushes that
//! radius; it is the only writer that knows a theme's own radius. The agent
//! seeds the radius from `config.toml` at startup, and a reload republishes
//! only a pinned `ui_radius`. An unset one leaves the desktop's radius alone.

use std::sync::{Mutex, MutexGuard, PoisonError};

use openlogi_ipc::{Generation, OBSERVE_HOLD, UiChromeObservation};
use openlogi_ui::action_icons::{DEFAULT_THEME_RADIUS_PX, effective_radius};
use tokio::sync::watch;

/// The radius the overlay is drawing with.
#[derive(Clone)]
pub struct UiChrome {
    state: std::sync::Arc<Mutex<State>>,
    published: watch::Sender<UiChromeObservation>,
}

struct State {
    generation: Generation,
}

impl UiChrome {
    /// Start from `ui_radius`, using [`DEFAULT_THEME_RADIUS_PX`] when it is
    /// unset. Generation 1 so an overlay that has seen nothing reads it at
    /// once.
    #[must_use]
    pub fn from_setting(ui_radius: Option<u8>) -> Self {
        Self::from_radius(effective_radius(ui_radius, DEFAULT_THEME_RADIUS_PX))
    }

    fn from_radius(radius_px: u8) -> Self {
        let (published, _) = watch::channel(UiChromeObservation {
            generation: 1,
            radius_px,
        });
        Self {
            state: std::sync::Arc::new(Mutex::new(State { generation: 1 })),
            published,
        }
    }

    /// Publish a reloaded `ui_radius` when it pins a radius.
    ///
    /// `None` defers to the active theme, whose radius only the desktop
    /// knows: falling back to [`DEFAULT_THEME_RADIUS_PX`] here would undo the
    /// radius the desktop pushed on every unrelated settings save.
    pub fn set_setting(&self, ui_radius: Option<u8>) {
        if let Some(radius_px) = ui_radius {
            self.set_radius(radius_px);
        }
    }

    /// Publish `radius_px` when it differs from the one the overlay has.
    pub fn set_radius(&self, radius_px: u8) {
        let mut state = self.state();
        let current = *self.published.borrow();
        if current.radius_px == radius_px {
            return;
        }
        state.generation = state.generation.wrapping_add(1);
        self.published.send_replace(UiChromeObservation {
            generation: state.generation,
            radius_px,
        });
    }

    /// Observe the radius. Answers as soon as it differs from `since`.
    pub async fn observe(&self, since: Generation) -> UiChromeObservation {
        let mut receiver = self.published.subscribe();
        let changed = receiver.wait_for(|state| state.generation != since);
        match tokio::time::timeout(OBSERVE_HOLD, changed).await {
            Ok(Ok(state)) => *state,
            Ok(Err(_)) | Err(_) => *self.published.borrow(),
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_radius_change_reaches_the_next_observation() {
        let chrome = UiChrome::from_setting(None);
        let first = chrome.observe(0).await;
        assert_eq!(first.radius_px, DEFAULT_THEME_RADIUS_PX);

        chrome.set_radius(0);
        let sharp = chrome.observe(first.generation).await;
        assert_eq!(sharp.radius_px, 0);
        assert_ne!(sharp.generation, first.generation);

        chrome.set_setting(Some(12));
        let round = chrome.observe(sharp.generation).await;
        assert_eq!(round.radius_px, 12);
    }

    #[tokio::test]
    async fn an_unset_setting_keeps_the_radius_the_desktop_pushed() {
        let chrome = UiChrome::from_setting(None);
        chrome.set_radius(0);
        let sharp = chrome.observe(0).await;

        chrome.set_setting(None);
        assert_eq!(
            *chrome.published.borrow(),
            sharp,
            "a reload with no override must not fall back to the default radius"
        );
    }

    #[tokio::test]
    async fn the_same_radius_does_not_wake_an_observer() {
        let chrome = UiChrome::from_setting(Some(6));
        let first = chrome.observe(0).await;
        chrome.set_radius(6);
        assert_eq!(chrome.published.borrow().generation, first.generation);
    }
}
