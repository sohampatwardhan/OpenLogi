//! Semantic keyboard-backlight types, re-exported from `openlogi-core`.
//!
//! The conversion from the HID++ effect lives here. Transport stays in this
//! crate's write layer.

pub use openlogi_core::hid::backlight::{
    BacklightChange, BacklightControls, BacklightDuration, BacklightDurationError, BacklightEffect,
    BacklightMode, BacklightState, BacklightStatus,
};

use hidpp::feature::backlight::BacklightEffect as FirmwareEffect;

/// Map the fork's `0x1982` effect onto [`BacklightEffect`].
///
/// The source enum is `#[non_exhaustive]`. An effect this crate does not name
/// yet maps to [`BacklightEffect::Static`], the firmware default, until the
/// variant is appended.
pub(crate) fn effect_from_firmware(effect: FirmwareEffect) -> BacklightEffect {
    match effect {
        FirmwareEffect::None => BacklightEffect::None,
        FirmwareEffect::Breathing => BacklightEffect::Breathing,
        FirmwareEffect::Contrast => BacklightEffect::Contrast,
        FirmwareEffect::Reaction => BacklightEffect::Reaction,
        FirmwareEffect::Random => BacklightEffect::Random,
        FirmwareEffect::Waves => BacklightEffect::Waves,
        // [`FirmwareEffect::Static`] is the firmware default. A future effect
        // this crate does not name yet takes the same path until its variant
        // is appended.
        _ => BacklightEffect::Static,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_firmware_effect_keeps_its_name() {
        let expected = [
            (0, BacklightEffect::Static),
            (1, BacklightEffect::None),
            (2, BacklightEffect::Breathing),
            (3, BacklightEffect::Contrast),
            (4, BacklightEffect::Reaction),
            (5, BacklightEffect::Random),
            (6, BacklightEffect::Waves),
        ];
        for (byte, effect) in expected {
            let firmware = FirmwareEffect::try_from(byte).expect("a named effect byte");
            assert_eq!(effect_from_firmware(firmware), effect, "{byte}");
        }
    }
}
