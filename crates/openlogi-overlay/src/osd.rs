//! Chrome shared by the transient indicators: brightness, lock keys, and the
//! backlight-effect list.
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
