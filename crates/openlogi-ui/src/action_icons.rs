//! The vendored glyphs the overlay draws — Actions Ring slots and its
//! notices — as a GPUI `AssetSource` and as plain bytes.
//!
//! Both frontends draw ring slots, so both need these bytes; nothing else in
//! either app's artwork is shared, which is why this source serves *only*
//! `action-icons/` and reports every other path as absent. A frontend needing
//! more composes this with its own source rather than growing this one —
//! anything added here is added to the overlay too. The agent reads the same
//! bytes through [`embedded_icon`] to draw keypad LCD keys.
//!
//! Embedding via `include_bytes!` means the paths resolve identically inside a
//! packaged `.app` and from a dev build; a filesystem path would not.

#[cfg(feature = "gpui")]
use std::borrow::Cow;

#[cfg(feature = "gpui")]
use gpui::{AssetSource, Result, SharedString};
use openlogi_core::device::ConnectionLink;

/// Vendored icons, embedded so they resolve identically in a packaged `.app`
/// and a dev build. Served under the `action-icons/` path prefix and rendered
/// via `svg().path(..)` or `Icon::empty().path(..)`.
///
/// Most of the set is [Lucide](https://lucide.dev) (ISC license): command
/// glyphs for the binding menus (paste / cut / volume / lock / …), plus the
/// chrome marks gpui-component's bundled `IconName` set does not cover —
/// About-page icons, and `mouse-left` / `mouse-right`, which are Lucide's
/// `mouse` with one button filled in, and `lightbulb`, which heads the
/// backlight effect list. The backlight level glyphs (`backlight-*`) are
/// [Material Symbols](https://fonts.google.com/icons) by Google, Apache
/// License 2.0. The lock-key glyphs (`arrow-big-up-dash-{filled,duotone}`)
/// are [Lucide Extended](https://github.com/applifted/lucide-extended) (ISC
/// license). The rest of the app stays on Lucide.
///
/// The battery cards draw Lucide's `battery-{full,medium,low,warning,charging}`.
/// [Material Symbols](https://fonts.google.com/icons) (Apache-2.0) supply the
/// connection popup's vertical battery and link glyphs, each in a `-rounded`
/// and a `-sharp` cut. `bolt.svg` and `unifying.svg` are the Logi Bolt and
/// Unifying receiver marks. Every other icon stays Lucide.
///
/// `x.svg` duplicates `IconName::Close` on purpose: the overlay does not depend
/// on gpui-component, and its ring cancel target has to match the settings-side
/// preview of the same ring exactly.
#[rustfmt::skip]
const ACTION_ICONS: &[(&str, &[u8])] = &[
    ("action-icons/arrow-left.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/arrow-left.svg"))),
    ("action-icons/arrow-big-up-dash-duotone.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/arrow-big-up-dash-duotone.svg"))),
    ("action-icons/arrow-big-up-dash-filled.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/arrow-big-up-dash-filled.svg"))),
    ("action-icons/arrow-right.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/arrow-right.svg"))),
    ("action-icons/backlight-high-off-rounded.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/backlight-high-off-rounded.svg"))),
    ("action-icons/backlight-high-off-sharp.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/backlight-high-off-sharp.svg"))),
    ("action-icons/backlight-high-rounded.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/backlight-high-rounded.svg"))),
    ("action-icons/backlight-high-sharp.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/backlight-high-sharp.svg"))),
    ("action-icons/backlight-low-rounded.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/backlight-low-rounded.svg"))),
    ("action-icons/backlight-low-sharp.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/backlight-low-sharp.svg"))),
    ("action-icons/ban.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/ban.svg"))),
    ("action-icons/battery-0-bar-rounded.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/battery-0-bar-rounded.svg"))),
    ("action-icons/battery-0-bar-sharp.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/battery-0-bar-sharp.svg"))),
    ("action-icons/battery-1-bar-rounded.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/battery-1-bar-rounded.svg"))),
    ("action-icons/battery-1-bar-sharp.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/battery-1-bar-sharp.svg"))),
    ("action-icons/battery-2-bar-rounded.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/battery-2-bar-rounded.svg"))),
    ("action-icons/battery-2-bar-sharp.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/battery-2-bar-sharp.svg"))),
    ("action-icons/battery-3-bar-rounded.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/battery-3-bar-rounded.svg"))),
    ("action-icons/battery-3-bar-sharp.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/battery-3-bar-sharp.svg"))),
    ("action-icons/battery-4-bar-rounded.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/battery-4-bar-rounded.svg"))),
    ("action-icons/battery-4-bar-sharp.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/battery-4-bar-sharp.svg"))),
    ("action-icons/battery-5-bar-rounded.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/battery-5-bar-rounded.svg"))),
    ("action-icons/battery-5-bar-sharp.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/battery-5-bar-sharp.svg"))),
    ("action-icons/battery-6-bar-rounded.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/battery-6-bar-rounded.svg"))),
    ("action-icons/battery-6-bar-sharp.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/battery-6-bar-sharp.svg"))),
    ("action-icons/battery-charging.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/battery-charging.svg"))),
    ("action-icons/battery-full.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/battery-full.svg"))),
    ("action-icons/battery-low.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/battery-low.svg"))),
    ("action-icons/battery-medium.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/battery-medium.svg"))),
    ("action-icons/battery-warning.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/battery-warning.svg"))),
    ("action-icons/battery-full-rounded.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/battery-full-rounded.svg"))),
    ("action-icons/battery-full-sharp.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/battery-full-sharp.svg"))),
    ("action-icons/bell.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/bell.svg"))),
    ("action-icons/bluetooth-rounded.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/bluetooth-rounded.svg"))),
    ("action-icons/bluetooth-sharp.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/bluetooth-sharp.svg"))),
    ("action-icons/bluetooth.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/bluetooth.svg"))),
    ("action-icons/bolt.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/bolt.svg"))),
    ("action-icons/book-open.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/book-open.svg"))),
    ("action-icons/bug.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/bug.svg"))),
    ("action-icons/calendar.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/calendar.svg"))),
    ("action-icons/camera.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/camera.svg"))),
    ("action-icons/chevron-left.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/chevron-left.svg"))),
    ("action-icons/chevron-right.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/chevron-right.svg"))),
    ("action-icons/chevrons-down.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/chevrons-down.svg"))),
    ("action-icons/chevrons-left.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/chevrons-left.svg"))),
    ("action-icons/chevrons-right.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/chevrons-right.svg"))),
    ("action-icons/chevrons-up.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/chevrons-up.svg"))),
    ("action-icons/circle-arrow-left.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/circle-arrow-left.svg"))),
    ("action-icons/circle-arrow-right.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/circle-arrow-right.svg"))),
    ("action-icons/circle-dot.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/circle-dot.svg"))),
    ("action-icons/clipboard-paste.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/clipboard-paste.svg"))),
    ("action-icons/copy.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/copy.svg"))),
    ("action-icons/dot.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/dot.svg"))),
    ("action-icons/file.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/file.svg"))),
    ("action-icons/folder.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/folder.svg"))),
    ("action-icons/gauge.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/gauge.svg"))),
    ("action-icons/globe.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/globe.svg"))),
    ("action-icons/grid-3x3.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/grid-3x3.svg"))),
    ("action-icons/heart.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/heart.svg"))),
    ("action-icons/keyboard.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/keyboard.svg"))),
    ("action-icons/layers.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/layers.svg"))),
    ("action-icons/layout-grid.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/layout-grid.svg"))),
    ("action-icons/lightbulb.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/lightbulb.svg"))),
    ("action-icons/list-checks.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/list-checks.svg"))),
    ("action-icons/lock.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/lock.svg"))),
    ("action-icons/monitor.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/monitor.svg"))),
    ("action-icons/moon.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/moon.svg"))),
    ("action-icons/mouse-left.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/mouse-left.svg"))),
    ("action-icons/mouse-pointer-click.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/mouse-pointer-click.svg"))),
    ("action-icons/mouse-right.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/mouse-right.svg"))),
    ("action-icons/mouse.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/mouse.svg"))),
    ("action-icons/move.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/move.svg"))),
    ("action-icons/palette.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/palette.svg"))),
    ("action-icons/pencil.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/pencil.svg"))),
    ("action-icons/play.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/play.svg"))),
    ("action-icons/redo-2.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/redo-2.svg"))),
    ("action-icons/refresh-cw.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/refresh-cw.svg"))),
    ("action-icons/rotate-ccw.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/rotate-ccw.svg"))),
    ("action-icons/rotate-cw.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/rotate-cw.svg"))),
    ("action-icons/save.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/save.svg"))),
    ("action-icons/scissors.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/scissors.svg"))),
    ("action-icons/scroll-text.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/scroll-text.svg"))),
    ("action-icons/search.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/search.svg"))),
    ("action-icons/settings-input-antenna-rounded.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/settings-input-antenna-rounded.svg"))),
    ("action-icons/settings-input-antenna-sharp.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/settings-input-antenna-sharp.svg"))),
    ("action-icons/settings.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/settings.svg"))),
    ("action-icons/skip-back.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/skip-back.svg"))),
    ("action-icons/skip-forward.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/skip-forward.svg"))),
    ("action-icons/square-arrow-left.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/square-arrow-left.svg"))),
    ("action-icons/square-arrow-right.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/square-arrow-right.svg"))),
    ("action-icons/square-plus.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/square-plus.svg"))),
    ("action-icons/square-terminal.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/square-terminal.svg"))),
    ("action-icons/square-x.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/square-x.svg"))),
    ("action-icons/star.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/star.svg"))),
    ("action-icons/terminal.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/terminal.svg"))),
    ("action-icons/undo-2.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/undo-2.svg"))),
    ("action-icons/unifying.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/unifying.svg"))),
    ("action-icons/usb-rounded.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/usb-rounded.svg"))),
    ("action-icons/usb-sharp.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/usb-sharp.svg"))),
    ("action-icons/usb.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/usb.svg"))),
    ("action-icons/user.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/user.svg"))),
    ("action-icons/volume-1.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/volume-1.svg"))),
    ("action-icons/volume-2.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/volume-2.svg"))),
    ("action-icons/volume-x.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/volume-x.svg"))),
    ("action-icons/x.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/x.svg"))),
];

/// The Actions Ring's centre cancel mark. Both processes draw that target —
/// the overlay for real, the settings app as a preview — and they have to be
/// the same mark, so neither side names the asset itself.
pub const RING_CANCEL_ICON: &str = "action-icons/x.svg";

/// Corner radius gpui-component gives a theme that does not set its own, in
/// whole pixels.
///
/// `Theme` starts at 6px, and applying a theme file replaces that only when
/// the file sets `radius`. No theme OpenLogi ships sets it, so an unset
/// `ui_radius` is this value.
pub const DEFAULT_THEME_RADIUS_PX: u8 = 6;

/// [`DEFAULT_THEME_RADIUS_PX`] as the `f32` the theme engine works in.
pub const DEFAULT_THEME_RADIUS: f32 = DEFAULT_THEME_RADIUS_PX as f32;

/// The effective radius: the user's override, or `theme_radius_px` when they
/// left it to the theme.
#[must_use]
pub const fn effective_radius(ui_radius: Option<u8>, theme_radius_px: u8) -> u8 {
    match ui_radius {
        Some(radius) => radius,
        None => theme_radius_px,
    }
}

/// Which Material Symbols cut a popup glyph uses.
///
/// The effective UI corner radius decides. Radius 0 is [`Self::Sharp`].
/// Every other radius — 6, 12, or a theme's own radius — is [`Self::Rounded`].
///
/// Popup glyphs that ship both cuts take this and call [`Self::path`]. The
/// backlight, effect-list, and lock-key glyphs build their path from a stem
/// with [`material_icon_path`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaterialSymbolStyle {
    /// `-rounded`, for any corner radius other than 0.
    Rounded,
    /// `-sharp`, for a corner radius of 0.
    Sharp,
}

impl MaterialSymbolStyle {
    /// The cut for `radius_px`.
    #[must_use]
    pub const fn from_radius(radius_px: f32) -> Self {
        if radius_px == 0.0 {
            Self::Sharp
        } else {
            Self::Rounded
        }
    }

    /// The cut for a whole-pixel radius, as the agent reports it.
    #[must_use]
    pub const fn from_radius_px(radius_px: u8) -> Self {
        if radius_px == 0 {
            Self::Sharp
        } else {
            Self::Rounded
        }
    }

    /// The cut for the stored override.
    ///
    /// `ui_radius` is `0`, `6`, or `12` when the Appearance page set one.
    /// `None` keeps `theme_radius_px`, the active theme's own radius.
    /// Callers that do not have the theme engine pass [`DEFAULT_THEME_RADIUS`].
    #[must_use]
    pub const fn from_ui_radius(ui_radius: Option<u8>, theme_radius_px: f32) -> Self {
        match ui_radius {
            // `f32::from` is not stably const. Every `u8` is exactly
            // representable, so the cast does not round.
            Some(radius) => Self::from_radius(radius as f32),
            None => Self::from_radius(theme_radius_px),
        }
    }

    /// `rounded` or `sharp`, the filename suffix of this cut.
    #[must_use]
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Rounded => "rounded",
            Self::Sharp => "sharp",
        }
    }

    /// The asset path of this cut. `rounded` and `sharp` are the two files
    /// of one glyph.
    #[must_use]
    pub const fn path(self, rounded: &'static str, sharp: &'static str) -> &'static str {
        match self {
            Self::Rounded => rounded,
            Self::Sharp => sharp,
        }
    }
}

/// Asset path `action-icons/{stem}-{sharp|rounded}.svg`.
///
/// `stem` is the Material Symbol name without its cut suffix, such as
/// `backlight-low` or `lightbulb`.
#[must_use]
pub fn material_icon_path(stem: &str, style: MaterialSymbolStyle) -> String {
    format!("action-icons/{stem}-{}.svg", style.suffix())
}

/// Both cuts of one battery glyph.
#[derive(Clone, Copy)]
struct BatteryGlyph {
    rounded: &'static str,
    sharp: &'static str,
}

impl BatteryGlyph {
    const fn path(self, style: MaterialSymbolStyle) -> &'static str {
        style.path(self.rounded, self.sharp)
    }
}

/// Lucide's battery glyphs for the horizontal battery cards. Lucide ships
/// one cut, so these do not follow [`MaterialSymbolStyle`].
const BATTERY_LOW: &str = "action-icons/battery-low.svg";
const BATTERY_MEDIUM: &str = "action-icons/battery-medium.svg";
const BATTERY_FULL: &str = "action-icons/battery-full.svg";
const BATTERY_WARNING: &str = "action-icons/battery-warning.svg";
const BATTERY_CHARGING: &str = "action-icons/battery-charging.svg";

/// The low-battery notice glyph: `battery-warning`.
#[must_use]
pub const fn low_battery_icon() -> &'static str {
    BATTERY_WARNING
}

/// The fully-charged glyph: `battery-full`.
#[must_use]
pub const fn battery_full_icon() -> &'static str {
    BATTERY_FULL
}

/// The discharging battery glyph for `percentage` (`0..=100`), in thirds:
///
/// | charge | glyph |
/// | --- | --- |
/// | 0..=33 | `battery-low` |
/// | 34..=66 | `battery-medium` |
/// | 67..=100 | `battery-full` |
#[must_use]
pub const fn battery_level_icon(percentage: u8) -> &'static str {
    match percentage {
        0..=33 => BATTERY_LOW,
        34..=66 => BATTERY_MEDIUM,
        _ => BATTERY_FULL,
    }
}

/// The charging battery glyph: `battery-charging`. The card's bar shows the
/// charge, so the glyph does not step with it.
#[must_use]
pub const fn battery_charging_icon() -> &'static str {
    BATTERY_CHARGING
}

/// Material Symbol for a backlight at the low end of its scale.
pub const BACKLIGHT_LOW_ICON: &str = "action-icons/backlight-low-rounded.svg";

/// Material Symbol for a backlight at the high end of its scale.
pub const BACKLIGHT_HIGH_ICON: &str = "action-icons/backlight-high-rounded.svg";

/// Material Symbol for a backlight that is off, including the effect key
/// pressed while the backlight is off.
pub const BACKLIGHT_OFF_ICON: &str = "action-icons/backlight-high-off-rounded.svg";

/// The filled Caps Lock arrow, Lucide Extended's `arrow-big-up-dash-filled`.
/// Every lock-key popup uses it while the key is on.
///
/// Num Lock and Scroll Lock share it: there is no num-lock or scroll-lock
/// glyph in the set, and the label still names the key.
pub const LOCK_KEY_ON_ICON: &str = "action-icons/arrow-big-up-dash-filled.svg";

/// The duotone Caps Lock arrow, `arrow-big-up-dash-duotone`. Every lock-key
/// popup uses it while the key is off.
pub const LOCK_KEY_OFF_ICON: &str = "action-icons/arrow-big-up-dash-duotone.svg";

/// Lucide's `lightbulb`, above the backlight effect list.
pub const BACKLIGHT_EFFECT_ICON: &str = "action-icons/lightbulb.svg";

/// The backlight glyph for `level` on a keyboard whose highest level is
/// `max_level`, in `style`.
///
/// Level 0 is off. Above that, the low glyph covers the bottom of the scale
/// and the high glyph the rest, matching 1–3 low and 4–7 high when
/// `max_level` is 7: low while `level * 7 <= max_level * 3`.
#[must_use]
pub fn backlight_level_icon(level: u8, max_level: u8, style: MaterialSymbolStyle) -> String {
    let stem = if level == 0 {
        "backlight-high-off"
    } else {
        let level = u16::from(level);
        let max_level = u16::from(max_level);
        if level.saturating_mul(7) <= max_level.saturating_mul(3) {
            "backlight-low"
        } else {
            "backlight-high"
        }
    };
    material_icon_path(stem, style)
}

/// The lightbulb above the effect-cycle list: Lucide's `lightbulb`, one cut.
#[must_use]
pub const fn backlight_effect_icon() -> &'static str {
    BACKLIGHT_EFFECT_ICON
}

/// The lock-key glyph for a key that is now `engaged` or not: filled while
/// it is on, duotone once it is off. Lucide Extended has one cut, so this
/// does not follow [`MaterialSymbolStyle`].
///
/// Caps Lock, Num Lock, and Scroll Lock share the Caps Lock arrow.
#[must_use]
pub const fn lock_key_icon(engaged: bool) -> &'static str {
    if engaged {
        LOCK_KEY_ON_ICON
    } else {
        LOCK_KEY_OFF_ICON
    }
}

/// A full vertical battery, the top of [`BATTERY_BARS`].
const BATTERY_BAR_FULL: BatteryGlyph = BatteryGlyph {
    rounded: "action-icons/battery-full-rounded.svg",
    sharp: "action-icons/battery-full-sharp.svg",
};

/// Vertical battery glyphs, empty to full, for the connection popup.
/// Index `0` is `battery-0-bar`; index `7` is [`BATTERY_BAR_FULL`].
/// `percentage * 8 / 100` (clamped to `0..=7`) splits `0..=100` into eight
/// equal bands.
const BATTERY_BARS: [BatteryGlyph; 8] = [
    BatteryGlyph {
        rounded: "action-icons/battery-0-bar-rounded.svg",
        sharp: "action-icons/battery-0-bar-sharp.svg",
    },
    BatteryGlyph {
        rounded: "action-icons/battery-1-bar-rounded.svg",
        sharp: "action-icons/battery-1-bar-sharp.svg",
    },
    BatteryGlyph {
        rounded: "action-icons/battery-2-bar-rounded.svg",
        sharp: "action-icons/battery-2-bar-sharp.svg",
    },
    BatteryGlyph {
        rounded: "action-icons/battery-3-bar-rounded.svg",
        sharp: "action-icons/battery-3-bar-sharp.svg",
    },
    BatteryGlyph {
        rounded: "action-icons/battery-4-bar-rounded.svg",
        sharp: "action-icons/battery-4-bar-sharp.svg",
    },
    BatteryGlyph {
        rounded: "action-icons/battery-5-bar-rounded.svg",
        sharp: "action-icons/battery-5-bar-sharp.svg",
    },
    BatteryGlyph {
        rounded: "action-icons/battery-6-bar-rounded.svg",
        sharp: "action-icons/battery-6-bar-sharp.svg",
    },
    BATTERY_BAR_FULL,
];

/// The vertical battery glyph for `percentage` (`0..=100`) in `style`, in
/// the eight bands of [`BATTERY_BARS`].
#[must_use]
pub fn battery_bar_icon(percentage: u8, style: MaterialSymbolStyle) -> &'static str {
    let percentage = u16::from(percentage.min(100));
    let step = usize::from((percentage * 8 / 100).min(7));
    BATTERY_BARS[step].path(style)
}

/// The Logi Bolt receiver mark. One cut serves both styles.
pub const BOLT_RECEIVER_ICON: &str = "action-icons/bolt.svg";

/// The Unifying receiver mark. One cut serves both styles.
pub const UNIFYING_RECEIVER_ICON: &str = "action-icons/unifying.svg";

/// The glyph for how a device reaches the host, in `style`.
///
/// Bluetooth and USB are the Material Symbols of those names. Bolt and
/// Unifying use the receivers' own marks, which have no Material Symbol and
/// no sharp cut. Lightspeed has neither, so it uses the Material
/// `settings-input-antenna` glyph, a wireless dongle.
#[must_use]
pub const fn connection_link_icon(
    link: ConnectionLink,
    style: MaterialSymbolStyle,
) -> &'static str {
    match link {
        ConnectionLink::Bluetooth => style.path(
            "action-icons/bluetooth-rounded.svg",
            "action-icons/bluetooth-sharp.svg",
        ),
        ConnectionLink::Usb => {
            style.path("action-icons/usb-rounded.svg", "action-icons/usb-sharp.svg")
        }
        ConnectionLink::Lightspeed => style.path(
            "action-icons/settings-input-antenna-rounded.svg",
            "action-icons/settings-input-antenna-sharp.svg",
        ),
        ConnectionLink::Bolt => BOLT_RECEIVER_ICON,
        ConnectionLink::Unifying => UNIFYING_RECEIVER_ICON,
    }
}

/// The embedded SVG for an `action-icons/…` asset path — the same path
/// `ActionRingIcon::asset_path` names — or `None` for any other path.
#[must_use]
pub fn embedded_icon(path: &str) -> Option<&'static [u8]> {
    ACTION_ICONS
        .iter()
        .find(|(candidate, _)| *candidate == path)
        .map(|(_, bytes)| *bytes)
}

/// GPUI asset source for the embedded ring glyphs, and nothing else.
#[cfg(feature = "gpui")]
pub struct ActionIcons;

#[cfg(feature = "gpui")]
impl AssetSource for ActionIcons {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(embedded_icon(path).map(Cow::Borrowed))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(ACTION_ICONS
            .iter()
            .filter(|(candidate, _)| candidate.starts_with(path))
            .map(|(candidate, _)| SharedString::from(*candidate))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use openlogi_core::binding::ActionRingIcon;

    use super::*;

    #[test]
    fn a_whole_pixel_radius_of_zero_is_sharp_and_every_other_is_rounded() {
        assert_eq!(
            MaterialSymbolStyle::from_radius_px(0),
            MaterialSymbolStyle::Sharp
        );
        for radius in [1_u8, 6, 12, 255] {
            assert_eq!(
                MaterialSymbolStyle::from_radius_px(radius),
                MaterialSymbolStyle::Rounded,
                "{radius}"
            );
        }
        assert_eq!(effective_radius(None, 8), 8);
        assert_eq!(effective_radius(Some(0), 8), 0);
        assert_eq!(f32::from(DEFAULT_THEME_RADIUS_PX), DEFAULT_THEME_RADIUS);
    }

    #[test]
    fn the_material_path_names_the_stem_and_the_cut() {
        assert_eq!(
            material_icon_path("backlight-low", MaterialSymbolStyle::Rounded),
            "action-icons/backlight-low-rounded.svg"
        );
        assert_eq!(
            material_icon_path("backlight-high-off", MaterialSymbolStyle::Sharp),
            "action-icons/backlight-high-off-sharp.svg"
        );
    }

    #[test]
    fn the_shared_ring_cancel_icon_is_embedded() {
        // Both processes render this path; a rename that missed the array
        // would blank the overlay's cancel target rather than fail to build.
        assert!(
            embedded_icon(RING_CANCEL_ICON).is_some(),
            "{RING_CANCEL_ICON} is missing"
        );
    }

    #[test]
    fn the_low_battery_notice_icon_is_embedded() {
        let path = low_battery_icon();
        let loaded = ActionIcons.load(path);
        assert!(matches!(loaded, Ok(Some(_))), "{path} is missing");
    }

    #[test]
    fn the_lock_key_indicator_icons_are_embedded() {
        for path in [
            lock_key_icon(true),
            lock_key_icon(false),
            backlight_effect_icon(),
        ] {
            let svg = embedded_icon(path)
                .and_then(|bytes| str::from_utf8(bytes).ok())
                .unwrap_or_else(|| panic!("{path} is missing"));
            assert!(
                svg.contains("currentColor"),
                "{path} must tint with the popup colour"
            );
        }
        assert_eq!(lock_key_icon(true), LOCK_KEY_ON_ICON);
        assert_eq!(lock_key_icon(false), LOCK_KEY_OFF_ICON);
    }

    #[test]
    fn backlight_icons_follow_a_seven_step_scale() {
        let rounded = MaterialSymbolStyle::Rounded;
        assert_eq!(backlight_level_icon(0, 7, rounded), BACKLIGHT_OFF_ICON);
        for level in 1..=3 {
            assert_eq!(
                backlight_level_icon(level, 7, rounded),
                BACKLIGHT_LOW_ICON,
                "{level}"
            );
        }
        for level in 4..=7 {
            assert_eq!(
                backlight_level_icon(level, 7, rounded),
                BACKLIGHT_HIGH_ICON,
                "{level}"
            );
        }
        // Twice the steps: 3/7 of 14 is 6, so 6 stays low and 7 is high.
        assert_eq!(backlight_level_icon(6, 14, rounded), BACKLIGHT_LOW_ICON);
        assert_eq!(backlight_level_icon(7, 14, rounded), BACKLIGHT_HIGH_ICON);
        assert_eq!(
            backlight_level_icon(4, 7, MaterialSymbolStyle::Sharp),
            "action-icons/backlight-high-sharp.svg"
        );
        assert_eq!(backlight_effect_icon(), "action-icons/lightbulb.svg");
    }

    #[test]
    fn material_popup_icons_keep_current_color() {
        for path in [
            BACKLIGHT_LOW_ICON,
            "action-icons/backlight-low-sharp.svg",
            BACKLIGHT_HIGH_ICON,
            "action-icons/backlight-high-sharp.svg",
            BACKLIGHT_OFF_ICON,
            "action-icons/backlight-high-off-sharp.svg",
        ] {
            let bytes = embedded_icon(path).expect(path);
            let svg = str::from_utf8(bytes).expect(path);
            assert!(
                svg.contains("fill=\"currentColor\""),
                "{path} must tint with the popup colour"
            );
        }
    }

    #[test]
    fn the_battery_glyphs_follow_the_percentage_thirds() {
        let level = |percentage, name| {
            assert_eq!(
                battery_level_icon(percentage),
                format!("action-icons/{name}.svg"),
                "{percentage}%"
            );
        };
        // Lower and upper edge of each third.
        level(0, "battery-low");
        level(33, "battery-low");
        level(34, "battery-medium");
        level(66, "battery-medium");
        level(67, "battery-full");
        level(100, "battery-full");
        level(250, "battery-full");

        assert_eq!(battery_full_icon(), "action-icons/battery-full.svg");
        assert_eq!(battery_charging_icon(), "action-icons/battery-charging.svg");
        assert_eq!(low_battery_icon(), "action-icons/battery-warning.svg");
        for path in [
            battery_level_icon(0),
            battery_level_icon(50),
            battery_full_icon(),
            battery_charging_icon(),
            low_battery_icon(),
        ] {
            assert!(embedded_icon(path).is_some(), "{path}");
        }
    }

    #[test]
    fn the_connection_glyphs_are_embedded_in_both_cuts() {
        let links = [
            ConnectionLink::Bluetooth,
            ConnectionLink::Bolt,
            ConnectionLink::Unifying,
            ConnectionLink::Lightspeed,
            ConnectionLink::Usb,
        ];
        for style in [MaterialSymbolStyle::Rounded, MaterialSymbolStyle::Sharp] {
            for link in links {
                let path = connection_link_icon(link, style);
                assert!(embedded_icon(path).is_some(), "{link:?}: {path}");
            }
            for percentage in 0..=100 {
                let path = battery_bar_icon(percentage, style);
                assert!(embedded_icon(path).is_some(), "{percentage}%: {path}");
            }
            let suffix = style.suffix();
            assert_eq!(
                battery_bar_icon(0, style),
                format!("action-icons/battery-0-bar-{suffix}.svg")
            );
            assert_eq!(
                battery_bar_icon(50, style),
                format!("action-icons/battery-4-bar-{suffix}.svg")
            );
            assert_eq!(
                battery_bar_icon(100, style),
                format!("action-icons/battery-full-{suffix}.svg")
            );
        }
        assert_eq!(
            connection_link_icon(ConnectionLink::Bolt, MaterialSymbolStyle::Sharp),
            BOLT_RECEIVER_ICON
        );
        assert_ne!(
            connection_link_icon(ConnectionLink::Bluetooth, MaterialSymbolStyle::Rounded),
            connection_link_icon(ConnectionLink::Bluetooth, MaterialSymbolStyle::Sharp)
        );
    }

    #[test]
    fn radius_zero_is_sharp_and_every_other_radius_is_rounded() {
        assert_eq!(
            MaterialSymbolStyle::from_radius(0.0),
            MaterialSymbolStyle::Sharp
        );
        for radius in [0.5, 6.0, 12.0, DEFAULT_THEME_RADIUS] {
            assert_eq!(
                MaterialSymbolStyle::from_radius(radius),
                MaterialSymbolStyle::Rounded,
                "{radius}"
            );
        }
        assert_eq!(
            MaterialSymbolStyle::from_ui_radius(Some(0), DEFAULT_THEME_RADIUS),
            MaterialSymbolStyle::Sharp
        );
        assert_eq!(
            MaterialSymbolStyle::from_ui_radius(Some(6), 0.0),
            MaterialSymbolStyle::Rounded
        );
        assert_eq!(
            MaterialSymbolStyle::from_ui_radius(Some(12), 0.0),
            MaterialSymbolStyle::Rounded
        );
        // None keeps the theme's own radius, even when that radius is square.
        assert_eq!(
            MaterialSymbolStyle::from_ui_radius(None, 0.0),
            MaterialSymbolStyle::Sharp
        );
        assert_eq!(
            MaterialSymbolStyle::from_ui_radius(None, DEFAULT_THEME_RADIUS),
            MaterialSymbolStyle::Rounded
        );
    }

    #[test]
    fn every_ring_gallery_icon_is_embedded() {
        for icon in ActionRingIcon::ALL {
            assert!(
                embedded_icon(icon.asset_path()).is_some(),
                "missing embedded asset for {icon:?}"
            );
        }
    }

    /// A glyph dropped into `action-icons/` but left out of [`ACTION_ICONS`]
    /// resolves to no bytes, which GPUI draws as an empty slot rather than
    /// reporting — the failure has no error path to catch it at runtime. The
    /// button picker's `moon.svg` shipped that way from the day it was added.
    #[test]
    fn every_bundled_glyph_is_embedded() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("action-icons");
        let entries = fs::read_dir(&dir).expect("action-icons/ is readable");
        for entry in entries {
            let name = entry.expect("readable directory entry").file_name();
            let name = name.to_string_lossy();
            if !name.ends_with(".svg") {
                continue;
            }
            let path = format!("action-icons/{name}");
            assert!(
                embedded_icon(&path).is_some(),
                "{path} is on disk but missing from ACTION_ICONS"
            );
        }
    }

    /// The GPUI source serves exactly the embedded table: a hit is the same
    /// bytes, and a path outside `action-icons/` is absent rather than an
    /// error, so a composed source falls through to the next one.
    #[cfg(feature = "gpui")]
    #[test]
    fn the_asset_source_serves_the_embedded_bytes_and_nothing_else() {
        let served = ActionIcons.load(RING_CANCEL_ICON).expect("load succeeds");
        assert_eq!(served.as_deref(), embedded_icon(RING_CANCEL_ICON));
        assert!(matches!(ActionIcons.load("logo.svg"), Ok(None)));
    }
}
