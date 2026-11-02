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

/// Vendored icons, embedded so they resolve identically in a packaged `.app`
/// and a dev build. Served under the `action-icons/` path prefix and rendered
/// via `svg().path(..)` or `Icon::empty().path(..)`.
///
/// Most of the set is [Lucide](https://lucide.dev) (ISC license): command
/// glyphs for the binding menus (paste / cut / volume / lock / …), plus the
/// chrome marks gpui-component's bundled `IconName` set does not cover —
/// About-page icons, and `mouse-left` / `mouse-right`, which are Lucide's
/// `mouse` with one button filled in. The overlay popup glyphs
/// (`backlight-*-rounded`, `keyboard-capslock-badge-*-rounded`) are
/// [Material Symbols](https://fonts.google.com/icons) by Google, Apache
/// License 2.0. The rest of the app stays on Lucide.
///
/// `x.svg` duplicates `IconName::Close` on purpose: the overlay does not depend
/// on gpui-component, and its ring cancel target has to match the settings-side
/// preview of the same ring exactly.
#[rustfmt::skip]
const ACTION_ICONS: &[(&str, &[u8])] = &[
    ("action-icons/arrow-left.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/arrow-left.svg"))),
    ("action-icons/arrow-right.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/arrow-right.svg"))),
    ("action-icons/backlight-high-off-rounded.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/backlight-high-off-rounded.svg"))),
    ("action-icons/backlight-high-off-sharp.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/backlight-high-off-sharp.svg"))),
    ("action-icons/backlight-high-rounded.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/backlight-high-rounded.svg"))),
    ("action-icons/backlight-high-sharp.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/backlight-high-sharp.svg"))),
    ("action-icons/backlight-low-rounded.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/backlight-low-rounded.svg"))),
    ("action-icons/backlight-low-sharp.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/backlight-low-sharp.svg"))),
    ("action-icons/ban.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/ban.svg"))),
    ("action-icons/battery-low.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/battery-low.svg"))),
    ("action-icons/bell.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/bell.svg"))),
    ("action-icons/bluetooth.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/bluetooth.svg"))),
    ("action-icons/book-open.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/book-open.svg"))),
    ("action-icons/bolt.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/bolt.svg"))),
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
    ("action-icons/keyboard-capslock-badge-outline-rounded.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/keyboard-capslock-badge-outline-rounded.svg"))),
    ("action-icons/keyboard-capslock-badge-outline-sharp.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/keyboard-capslock-badge-outline-sharp.svg"))),
    ("action-icons/keyboard-capslock-badge-rounded.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/keyboard-capslock-badge-rounded.svg"))),
    ("action-icons/keyboard-capslock-badge-sharp.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/keyboard-capslock-badge-sharp.svg"))),
    ("action-icons/keyboard.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/keyboard.svg"))),
    ("action-icons/layers.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/layers.svg"))),
    ("action-icons/layout-grid.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/layout-grid.svg"))),
    ("action-icons/lightbulb-rounded.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/lightbulb-rounded.svg"))),
    ("action-icons/lightbulb-sharp.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/lightbulb-sharp.svg"))),
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
    ("action-icons/settings.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/settings.svg"))),
    ("action-icons/skip-back.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/skip-back.svg"))),
    ("action-icons/skip-forward.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/skip-forward.svg"))),
    ("action-icons/square-arrow-left.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/square-arrow-left.svg"))),
    ("action-icons/square-arrow-right.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/square-arrow-right.svg"))),
    ("action-icons/square-plus.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/square-plus.svg"))),
    ("action-icons/square-terminal.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/square-terminal.svg"))),
    ("action-icons/square-x.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/square-x.svg"))),
    ("action-icons/terminal.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/terminal.svg"))),
    ("action-icons/star.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/star.svg"))),
    ("action-icons/undo-2.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/undo-2.svg"))),
    ("action-icons/unifying.svg", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/action-icons/unifying.svg"))),
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

/// The glyph on the overlay's low-battery notice.
pub const LOW_BATTERY_ICON: &str = "action-icons/battery-low.svg";

/// Material Symbol for a backlight at the low end of its scale.
pub const BACKLIGHT_LOW_ICON: &str = "action-icons/backlight-low-rounded.svg";

/// Material Symbol for a backlight at the high end of its scale.
pub const BACKLIGHT_HIGH_ICON: &str = "action-icons/backlight-high-rounded.svg";

/// Material Symbol for a backlight that is off, including the effect key
/// pressed while the backlight is off.
pub const BACKLIGHT_OFF_ICON: &str = "action-icons/backlight-high-off-rounded.svg";

/// Filled Caps Lock badge. Every lock-key popup uses it while the key is on.
///
/// Num Lock and Scroll Lock share it: Material Symbols has no num-lock or
/// scroll-lock glyph, and the label still names the key.
pub const LOCK_KEY_ON_ICON: &str = "action-icons/keyboard-capslock-badge-rounded.svg";

/// Outlined Caps Lock badge. Every lock-key popup uses it while the key is off.
pub const LOCK_KEY_OFF_ICON: &str = "action-icons/keyboard-capslock-badge-outline-rounded.svg";

/// The backlight glyph for `level` on a keyboard whose highest level is
/// `max_level`, in `corner`.
///
/// Level 0 is off. Above that, the low glyph covers the bottom of the scale
/// and the high glyph the rest, matching 1–3 low and 4–7 high when
/// `max_level` is 7: low while `level * 7 <= max_level * 3`.
#[must_use]
pub fn backlight_level_icon(
    level: u8,
    max_level: u8,
    corner: crate::material::MaterialCorner,
) -> String {
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
    crate::material::material_icon_path(stem, corner)
}

/// The lightbulb above the effect-cycle list, in `corner`.
#[must_use]
pub fn backlight_effect_icon(corner: crate::material::MaterialCorner) -> String {
    crate::material::material_icon_path("lightbulb", corner)
}

/// The lock-key glyph for a key that is now `engaged` or not, in `corner`.
///
/// Caps Lock, Num Lock, and Scroll Lock share the Caps Lock badge.
#[must_use]
pub fn lock_key_icon(engaged: bool, corner: crate::material::MaterialCorner) -> String {
    let stem = if engaged {
        "keyboard-capslock-badge"
    } else {
        "keyboard-capslock-badge-outline"
    };
    crate::material::material_icon_path(stem, corner)
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
    use crate::material::MaterialCorner;

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
        let loaded = ActionIcons.load(LOW_BATTERY_ICON);
        assert!(
            matches!(loaded, Ok(Some(_))),
            "{LOW_BATTERY_ICON} is missing"
        );
    }

    #[test]
    fn the_lock_key_indicator_icons_are_embedded() {
        for engaged in [true, false] {
            for corner in [MaterialCorner::Rounded, MaterialCorner::Sharp] {
                let path = lock_key_icon(engaged, corner);
                assert!(embedded_icon(&path).is_some(), "{path} is missing");
            }
        }
        assert_ne!(
            lock_key_icon(true, MaterialCorner::Rounded),
            lock_key_icon(false, MaterialCorner::Rounded)
        );
    }

    #[test]
    fn backlight_icons_follow_a_seven_step_scale() {
        let rounded = MaterialCorner::Rounded;
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
            backlight_level_icon(4, 7, MaterialCorner::Sharp),
            "action-icons/backlight-high-sharp.svg"
        );
        assert_eq!(
            backlight_effect_icon(rounded),
            "action-icons/lightbulb-rounded.svg"
        );
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
            LOCK_KEY_ON_ICON,
            "action-icons/keyboard-capslock-badge-sharp.svg",
            LOCK_KEY_OFF_ICON,
            "action-icons/keyboard-capslock-badge-outline-sharp.svg",
            "action-icons/lightbulb-rounded.svg",
            "action-icons/lightbulb-sharp.svg",
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
