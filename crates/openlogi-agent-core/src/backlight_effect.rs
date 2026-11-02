//! The transient indicator that names a keyboard's new backlight effect.
//!
//! HID++ `0x1982` `backlightInfoEvent` carries the brightness and the effect
//! together. A brightness change keeps the level card. An effect change that
//! leaves the level and visibility alone is named here instead, the way the
//! lock-key indicator names Caps Lock. The agent remembers the last reading
//! per device, because the broadcast only carries the new state.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use openlogi_core::hid::BacklightEffect;
use openlogi_ipc::{BacklightEffectObservation, Generation, OBSERVE_HOLD};
use tokio::sync::watch;

use crate::backlight_overlay::DISPLAY_LIFETIME;

/// What one backlight broadcast should put on screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BacklightIndication {
    /// The brightness card, including the first reading from a device.
    Level {
        /// Current backlight level.
        current_level: u8,
        /// Number of selectable levels, including zero.
        levels: u8,
        /// Whether the backlight is lit.
        visible: bool,
    },
    /// The effect name. The level and visibility did not change.
    Effect(BacklightEffect),
}

/// The last backlight reading seen from one keyboard.
#[derive(Clone, Copy)]
struct Seen {
    level: u8,
    visible: bool,
    effect: BacklightEffect,
}

/// Last effect and level per HID++ device index.
#[derive(Clone, Default)]
pub(crate) struct BacklightChanges {
    seen: std::sync::Arc<Mutex<HashMap<u8, Seen>>>,
}

impl BacklightChanges {
    /// Decide which indicator `reading` should raise, and remember it.
    pub(crate) fn classify(
        &self,
        device_index: u8,
        current_level: u8,
        levels: u8,
        visible: bool,
        effect: BacklightEffect,
    ) -> BacklightIndication {
        let mut seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        let next = Seen {
            level: current_level,
            visible,
            effect,
        };
        let previous = seen.insert(device_index, next);
        match previous {
            Some(previous)
                if previous.level == current_level
                    && previous.visible == visible
                    && previous.effect != effect =>
            {
                BacklightIndication::Effect(effect)
            }
            _ => BacklightIndication::Level {
                current_level,
                levels,
                visible,
            },
        }
    }
}

/// The backlight-effect indicator's cell.
#[derive(Clone)]
pub(crate) struct BacklightEffectIndicator {
    published: watch::Sender<BacklightEffectObservation>,
}

impl Default for BacklightEffectIndicator {
    fn default() -> Self {
        // Generation 1, not 0: 0 is the client sentinel for "I have seen
        // nothing", so a first `observe(0)` answers at once.
        let (published, _) = watch::channel(BacklightEffectObservation {
            generation: 1,
            shown: None,
            language: None,
            supported: Vec::new(),
        });
        Self { published }
    }
}

impl BacklightEffectIndicator {
    /// Hide the indicator immediately. A brightness-off report uses this so a
    /// list that is still showing does not name an effect the keys have left.
    pub(crate) fn dismiss(&self) {
        self.published.send_if_modified(|observation| {
            if observation.shown.is_none() {
                return false;
            }
            observation.generation = observation.generation.wrapping_add(1);
            observation.shown = None;
            observation.supported.clear();
            true
        });
    }

    /// Show `effect` for [`DISPLAY_LIFETIME`], replacing whatever was showing.
    ///
    /// `language` is the configured UI locale, or `None` to follow the
    /// overlay host's. Call it inside a Tokio runtime: a task ends the
    /// change when its time is up.
    pub(crate) fn show(
        &self,
        effect: BacklightEffect,
        supported: Vec<BacklightEffect>,
        language: Option<String>,
    ) {
        let mut generation = 0;
        self.published.send_modify(|observation| {
            observation.generation = observation.generation.wrapping_add(1);
            observation.shown = Some(effect);
            observation.supported = supported;
            observation.language = language;
            generation = observation.generation;
        });
        if tokio::runtime::Handle::try_current().is_err() {
            return;
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
    }

    /// Long-poll the indicator. Answers as soon as it differs from generation
    /// `since`, or with the unchanged indicator after [`OBSERVE_HOLD`].
    pub(crate) async fn observe(&self, since: Generation) -> BacklightEffectObservation {
        let mut receiver = self.published.subscribe();
        let changed = receiver.wait_for(|observation| observation.generation != since);
        match tokio::time::timeout(OBSERVE_HOLD, changed).await {
            Ok(Ok(observation)) => observation.clone(),
            Ok(Err(_)) | Err(_) => self.published.borrow().clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_effect_change_on_a_known_device_names_the_effect() {
        let changes = BacklightChanges::default();
        let level = |effect| changes.classify(1, 3, 8, true, effect);

        assert_eq!(
            level(BacklightEffect::Static),
            BacklightIndication::Level {
                current_level: 3,
                levels: 8,
                visible: true,
            },
            "the first reading has no previous effect to compare"
        );
        assert_eq!(
            level(BacklightEffect::Breathing),
            BacklightIndication::Effect(BacklightEffect::Breathing)
        );
        assert_eq!(
            changes.classify(1, 4, 8, true, BacklightEffect::Waves),
            BacklightIndication::Level {
                current_level: 4,
                levels: 8,
                visible: true,
            },
            "a brightness change keeps the level card, and the new effect is remembered"
        );
        assert_eq!(
            changes.classify(1, 4, 8, true, BacklightEffect::Random),
            BacklightIndication::Effect(BacklightEffect::Random)
        );
        assert_eq!(
            changes.classify(2, 4, 8, true, BacklightEffect::Contrast),
            BacklightIndication::Level {
                current_level: 4,
                levels: 8,
                visible: true,
            },
            "another keyboard has its own last effect"
        );
    }

    #[test]
    fn a_visibility_change_stays_on_the_level_card() {
        let changes = BacklightChanges::default();
        let _ = changes.classify(1, 3, 8, true, BacklightEffect::Static);
        assert_eq!(
            changes.classify(1, 3, 8, false, BacklightEffect::None),
            BacklightIndication::Level {
                current_level: 3,
                levels: 8,
                visible: false,
            }
        );
    }

    #[tokio::test(start_paused = true)]
    async fn an_effect_shows_in_its_language_until_its_time_is_up() {
        let indicator = BacklightEffectIndicator::default();
        indicator.show(
            BacklightEffect::Waves,
            vec![
                BacklightEffect::Random,
                BacklightEffect::Waves,
                BacklightEffect::Static,
            ],
            Some("de".to_owned()),
        );
        let shown = indicator.observe(0).await;
        assert_eq!(shown.shown, Some(BacklightEffect::Waves));
        assert_eq!(
            shown.supported,
            [
                BacklightEffect::Random,
                BacklightEffect::Waves,
                BacklightEffect::Static,
            ]
        );
        assert_eq!(shown.language.as_deref(), Some("de"));

        tokio::task::yield_now().await;
        tokio::time::advance(DISPLAY_LIFETIME).await;
        let ended = indicator.observe(shown.generation).await;
        assert_eq!(ended.shown, None);
        assert_eq!(
            ended.language.as_deref(),
            Some("de"),
            "the end keeps the language of what it ends"
        );

        indicator.show(BacklightEffect::Reaction, Vec::new(), None);
        let next = indicator.observe(ended.generation).await;
        assert_eq!(next.shown, Some(BacklightEffect::Reaction));
        assert!(next.generation > ended.generation);
    }
}
