//! Chrome shared by the transient indicators: brightness, lock keys, the
//! backlight-effect list, charging, and connection.
//!
//! They float over whatever is on the desktop, so they stay a dark card in
//! both appearances — the same choice the notices make. The blue is the brand
//! accent ([`openlogi_ui::color`]), not a second blue local to one popup.

use gpui::{FontWeight, Hsla};
use openlogi_ui::color;

/// Corner radius of every indicator card.
pub(crate) const RADIUS: f32 = 12.0;
/// Inset between the card edge and its contents. The lock-key card's padding.
pub(crate) const PAD: f32 = 20.0;
/// Gap between the lock-key glyph and its label, and between effect rows.
pub(crate) const GAP: f32 = 12.0;
/// The lock-key title's weight. Effect names use it too.
pub(crate) const TITLE: FontWeight = FontWeight::SEMIBOLD;

/// The card fill: the brightness and lock-key panels.
pub(crate) const PANEL: Hsla = neutral(0.08, 0.94);
/// Text on that panel.
pub(crate) const TEXT: Hsla = neutral(0.96, 1.0);
/// The brightness bar's empty track.
pub(crate) const TRACK: Hsla = neutral(0.25, 1.0);
/// The current-effect pill. Near `#444`, darker than the panel.
pub(crate) const PILL: Hsla = neutral(0.27, 1.0);
/// A lock key that just turned off.
pub(crate) const MUTED: Hsla = neutral(0.60, 1.0);

/// Lock-key indicator size. Wide enough for the longest translated title on
/// one line.
pub(crate) const LOCK_WIDTH: f32 = 360.0;
pub(crate) const LOCK_HEIGHT: f32 = 72.0;

/// Charging uses the lock-key width and a taller card: a name, a status
/// line, and a level bar in the same chrome.
pub(crate) const CHARGING_WIDTH: f32 = LOCK_WIDTH;
pub(crate) const CHARGING_HEIGHT: f32 = 88.0;

/// Level bar thickness, shared by the brightness and charging cards.
pub(crate) const BAR_HEIGHT: f32 = 8.0;
pub(crate) const BAR_RADIUS: f32 = 4.0;

const fn neutral(lightness: f32, alpha: f32) -> Hsla {
    Hsla {
        h: 0.0,
        s: 0.0,
        l: lightness,
        a: alpha,
    }
}

/// The brand accent, the blue the effect pill outlines and the brightness
/// bar fills.
#[must_use]
pub(crate) fn accent() -> Hsla {
    color::accent()
}
