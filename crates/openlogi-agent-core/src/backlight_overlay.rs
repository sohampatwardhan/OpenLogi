//! Agent-owned state for the transient keyboard backlight indicator.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use openlogi_core::hid::BacklightEffect;
use openlogi_ipc::{BacklightEffectObservation, BacklightObservation, Generation, OBSERVE_HOLD};
use tokio::sync::watch;

use crate::backlight_effect::{BacklightChanges, BacklightEffectIndicator, BacklightIndication};

/// How long the indicator shows one level change.
pub(crate) const DISPLAY_LIFETIME: Duration = Duration::from_millis(1500);

/// One keyboard backlight broadcast, for [`BacklightOverlayManager::show_event`].
#[derive(Clone, Copy)]
pub struct BacklightReading {
    /// HID++ device index that emitted the event.
    pub device_index: u8,
    /// Current backlight level.
    pub current_level: u8,
    /// Number of selectable levels, including zero.
    pub levels: u8,
    /// Whether the backlight is lit.
    pub visible: bool,
    /// The effect the keyboard is running now.
    pub effect: BacklightEffect,
    /// Effect-list bits from `getBacklightConfig`, once read.
    pub supported: Option<u16>,
}

impl BacklightReading {
    /// Whether the keys are lit: a level above off, and the firmware has not
    /// turned the backlight off.
    #[must_use]
    pub const fn is_lit(self) -> bool {
        self.current_level > 0 && self.visible
    }
}

#[derive(Clone)]
pub struct BacklightOverlayManager {
    state: Arc<Mutex<State>>,
    published: watch::Sender<BacklightObservation>,
    effects: BacklightEffectIndicator,
    changes: BacklightChanges,
}

struct State {
    generation: Generation,
    /// The configured UI locale, kept here so a key press never waits on
    /// the orchestrator to learn it. `None` follows the overlay host's.
    language: Option<String>,
}

impl Default for BacklightOverlayManager {
    fn default() -> Self {
        let initial = BacklightObservation {
            generation: 0,
            current_level: 0,
            levels: 0,
            visible: false,
            off_hint: false,
            language: None,
        };
        let (published, _) = watch::channel(initial);
        Self {
            state: Arc::new(Mutex::new(State {
                generation: 0,
                language: None,
            })),
            published,
            effects: BacklightEffectIndicator::default(),
            changes: BacklightChanges::default(),
        }
    }
}

impl BacklightOverlayManager {
    /// Name both cards in `language`, the configured UI locale, or `None` to
    /// follow the overlay host's. The agent sets it at startup and on every
    /// config reload.
    pub fn set_language(&self, language: Option<String>) {
        self.state().language = language;
    }

    /// Show a backlight broadcast: the level card, unless only the effect
    /// changed on a device whose last reading this manager already saw.
    ///
    /// Call it inside a Tokio runtime.
    pub fn show_event(&self, reading: BacklightReading) {
        match self.changes.classify(
            reading.device_index,
            reading.current_level,
            reading.levels,
            reading.visible,
            reading.effect,
        ) {
            BacklightIndication::Level {
                current_level,
                levels,
                visible,
            } => {
                // Lights out ends an effect list that is still on screen.
                if !reading.is_lit() {
                    self.effects.dismiss();
                }
                self.show(current_level, levels, visible);
            }
            // Only a reported effect change while the keys are lit. Brightness
            // 0 and a backlight the firmware has turned off do not cycle, and
            // a report that arrives anyway must not name a stale effect.
            BacklightIndication::Effect(effect)
                if reading.is_lit() && effect != BacklightEffect::None =>
            {
                let supported = reading
                    .supported
                    .map(BacklightEffect::in_cycle_order)
                    .unwrap_or_default();
                let language = self.state().language.clone();
                self.effects.show(effect, supported, language);
            }
            BacklightIndication::Effect(_) => self.effects.dismiss(),
        }
    }

    /// Show `effect` in `supported`, without comparing it to a previous reading.
    ///
    /// The mock console uses this so a developer can preview the popup
    /// without a keyboard. An empty `supported` names only `effect`. Call it
    /// inside a Tokio runtime.
    pub fn show_effect(
        &self,
        effect: BacklightEffect,
        supported: Vec<BacklightEffect>,
        language: Option<String>,
    ) {
        self.effects.show(effect, supported, language);
    }

    /// Show the brightness card's off hint and take down an effect list.
    ///
    /// Level 0 of 8 is the MX Mechanical scale, so the bar is empty at 0/7.
    /// The mock console calls this for `effect off`. A device event does
    /// not: the effect key sends no HID++ report while the backlight is off.
    pub fn show_off_hint(&self) {
        self.effects.dismiss();
        self.publish(0, 8, true, true);
    }

    /// Show the latest level and hide it after the transient display lifetime.
    pub fn show(&self, current_level: u8, levels: u8, visible: bool) {
        self.publish(current_level, levels, visible, false);
    }

    fn publish(&self, current_level: u8, levels: u8, visible: bool, off_hint: bool) {
        let generation = {
            let mut state = self.state();
            state.generation = state.generation.wrapping_add(1);
            self.published.send_replace(BacklightObservation {
                generation: state.generation,
                current_level,
                levels,
                visible,
                off_hint,
                language: state.language.clone(),
            });
            state.generation
        };
        let published = self.published.clone();
        let state = Arc::clone(&self.state);
        tokio::spawn(async move {
            tokio::time::sleep(DISPLAY_LIFETIME).await;
            let mut state = state.lock().unwrap_or_else(PoisonError::into_inner);
            if state.generation == generation {
                state.generation = state.generation.wrapping_add(1);
                published.send_modify(|observation| {
                    observation.generation = state.generation;
                    observation.visible = false;
                });
            }
        });
    }

    /// Observe the current level-indicator state.
    pub async fn observe(&self, since: Generation) -> BacklightObservation {
        let mut receiver = self.published.subscribe();
        let changed = receiver.wait_for(|state| state.generation != since);
        match tokio::time::timeout(OBSERVE_HOLD, changed).await {
            Ok(Ok(state)) => state.clone(),
            Ok(Err(_)) | Err(_) => self.published.borrow().clone(),
        }
    }

    /// Observe the current effect-indicator state.
    pub async fn observe_effect(&self, since: Generation) -> BacklightEffectObservation {
        self.effects.observe(since).await
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn a_new_press_after_expiry_has_a_new_generation() {
        let manager = BacklightOverlayManager::default();
        manager.show(3, 8, true);
        let shown = manager.observe(0).await;
        assert!(shown.visible);

        tokio::task::yield_now().await;
        tokio::time::advance(DISPLAY_LIFETIME).await;
        let hidden = manager.observe(shown.generation).await;
        assert!(!hidden.visible);

        manager.show(4, 8, true);
        let next = manager.observe(hidden.generation).await;
        assert!(next.visible);
        assert_eq!(next.current_level, 4);
        assert!(next.generation > hidden.generation);
    }

    #[tokio::test(start_paused = true)]
    async fn a_new_press_extends_the_display_lifetime() {
        let manager = BacklightOverlayManager::default();
        manager.show(3, 8, true);
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(1)).await;

        manager.show(4, 8, true);
        let latest = manager.observe(1).await;
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(500)).await;
        assert_eq!(manager.published.borrow().generation, latest.generation);
        assert!(manager.published.borrow().visible);

        tokio::time::advance(Duration::from_secs(1)).await;
        let hidden = manager.observe(latest.generation).await;
        assert!(!hidden.visible);
    }

    #[tokio::test(start_paused = true)]
    async fn an_effect_only_change_names_the_effect_and_leaves_the_level_card() {
        let manager = BacklightOverlayManager::default();
        manager.set_language(Some("fr".into()));
        manager.show_event(BacklightReading {
            device_index: 1,
            current_level: 3,
            levels: 8,
            visible: true,
            effect: BacklightEffect::Static,
            supported: None,
        });
        let level = manager.observe(0).await;
        assert!(level.visible);
        assert_eq!(level.current_level, 3);
        assert_eq!(
            level.language.as_deref(),
            Some("fr"),
            "the level card carries the locale for its translated text"
        );
        assert_eq!(manager.observe_effect(0).await.shown, None);

        let bits = BacklightEffect::Breathing.list_bit() | BacklightEffect::Static.list_bit();
        manager.show_event(BacklightReading {
            device_index: 1,
            current_level: 3,
            levels: 8,
            visible: true,
            effect: BacklightEffect::Breathing,
            supported: Some(bits),
        });
        let effect = manager.observe_effect(1).await;
        assert_eq!(effect.shown, Some(BacklightEffect::Breathing));
        assert_eq!(
            effect.supported,
            [BacklightEffect::Static, BacklightEffect::Breathing]
        );
        assert_eq!(effect.language.as_deref(), Some("fr"));
        assert_eq!(
            manager.published.borrow().generation,
            level.generation,
            "the level card is not refreshed when only the effect changed"
        );
    }

    fn reading(
        level: u8,
        visible: bool,
        effect: BacklightEffect,
        supported: Option<u16>,
    ) -> BacklightReading {
        BacklightReading {
            device_index: 1,
            current_level: level,
            levels: 8,
            visible,
            effect,
            supported,
        }
    }

    #[tokio::test(start_paused = true)]
    async fn an_effect_change_while_dark_is_not_shown_and_does_not_guess_the_next() {
        let manager = BacklightOverlayManager::default();
        let bits = BacklightEffect::CYCLE
            .into_iter()
            .fold(0, |bits, effect| bits | effect.list_bit())
            | BacklightEffect::None.list_bit();

        manager.show_event(reading(3, true, BacklightEffect::Static, Some(bits)));
        manager.show_event(reading(3, true, BacklightEffect::Waves, Some(bits)));
        let shown = manager.observe_effect(0).await;
        assert_eq!(shown.shown, Some(BacklightEffect::Waves));
        assert_eq!(
            shown.supported,
            BacklightEffect::CYCLE.to_vec(),
            "none is not a row, and the walk is not the effect-id order"
        );

        // Brightness to 0 takes the list down. The level card is the other cell,
        // and it is the ordinary level card: the effect key itself is not a report.
        manager.show_event(reading(0, true, BacklightEffect::Waves, Some(bits)));
        let hidden = manager.observe_effect(shown.generation).await;
        assert_eq!(hidden.shown, None);
        assert!(!manager.published.borrow().off_hint);

        // A reported effect change while dark does not open a list, and does
        // not publish the next cycle step.
        manager.show_event(reading(0, true, BacklightEffect::Reaction, Some(bits)));
        assert_eq!(manager.observe_effect(hidden.generation).await.shown, None);
        manager.show_event(reading(4, false, BacklightEffect::Random, Some(bits)));
        assert!(
            !manager.published.borrow().visible,
            "a backlight the firmware turned off does not leave the level card showing"
        );
        let still = manager.observe_effect(hidden.generation).await;
        assert_eq!(still.shown, None);

        // Lit again: only a new reported effect opens the list, naming that
        // effect rather than whichever step follows the last one.
        manager.show_event(reading(4, true, BacklightEffect::Random, Some(bits)));
        manager.show_event(reading(4, true, BacklightEffect::Static, Some(bits)));
        let lit = manager.observe_effect(still.generation).await;
        assert_eq!(lit.shown, Some(BacklightEffect::Static));
        assert_eq!(lit.supported, BacklightEffect::CYCLE.to_vec());
    }

    #[tokio::test(start_paused = true)]
    async fn an_off_hint_uses_the_level_card_and_closes_the_effect_list() {
        let manager = BacklightOverlayManager::default();
        manager.show_effect(
            BacklightEffect::Waves,
            BacklightEffect::CYCLE.to_vec(),
            None,
        );
        let listed = manager.observe_effect(0).await;
        assert_eq!(listed.shown, Some(BacklightEffect::Waves));

        manager.show_off_hint();
        let hint = manager.observe(0).await;
        assert!(hint.visible);
        assert!(hint.off_hint);
        assert_eq!((hint.current_level, hint.levels), (0, 8));
        let closed = manager.observe_effect(listed.generation).await;
        assert_eq!(closed.shown, None, "the effect list stays down while off");

        manager.show(3, 8, true);
        let level = manager.observe(hint.generation).await;
        assert!(level.visible);
        assert!(!level.off_hint);
        assert_eq!(level.current_level, 3);
    }
}
