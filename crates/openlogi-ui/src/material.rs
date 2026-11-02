//! Which drawing of a Material Symbol an overlay popup uses.
//!
//! The charging indicator and the backlight popups share this so a corner
//! radius of 0 is sharp everywhere, and every other radius is rounded.

/// Sharp or rounded drawing of a Material Symbol.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialCorner {
    /// Effective corner radius is 0.
    Sharp,
    /// Any effective corner radius other than 0.
    Rounded,
}

/// The gpui-component theme default, which OpenLogi's own themes keep by not
/// setting a radius. [`AppSettings::ui_radius`](openlogi_core::config::AppSettings::ui_radius)
/// `None` means this, unless the caller knows a theme that sets another.
pub const DEFAULT_THEME_RADIUS_PX: u8 = 6;

impl MaterialCorner {
    /// Sharp only when `radius_px` is 0.
    #[must_use]
    pub const fn from_radius(radius_px: u8) -> Self {
        if radius_px == 0 {
            Self::Sharp
        } else {
            Self::Rounded
        }
    }

    /// `ui_radius` when the user set one, otherwise the active theme's own
    /// radius.
    #[must_use]
    pub const fn from_setting(ui_radius: Option<u8>, theme_radius: u8) -> Self {
        Self::from_radius(match ui_radius {
            Some(radius) => radius,
            None => theme_radius,
        })
    }

    /// The filename suffix, `sharp` or `rounded`.
    #[must_use]
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Sharp => "sharp",
            Self::Rounded => "rounded",
        }
    }
}

/// The effective radius: the user's override, or `theme_radius` when they
/// left it to the theme.
#[must_use]
pub const fn effective_radius(ui_radius: Option<u8>, theme_radius: u8) -> u8 {
    match ui_radius {
        Some(radius) => radius,
        None => theme_radius,
    }
}

/// Asset path `action-icons/{stem}-{sharp|rounded}.svg`.
///
/// `stem` is the Material Symbol name without its corner suffix, such as
/// `backlight-low` or `battery-charging-full`.
#[must_use]
pub fn material_icon_path(stem: &str, corner: MaterialCorner) -> String {
    format!("action-icons/{stem}-{}.svg", corner.suffix())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radius_zero_is_sharp_and_every_other_radius_is_rounded() {
        assert_eq!(MaterialCorner::from_radius(0), MaterialCorner::Sharp);
        for radius in [1_u8, 6, 12, 255] {
            assert_eq!(
                MaterialCorner::from_radius(radius),
                MaterialCorner::Rounded,
                "{radius}"
            );
        }
    }

    #[test]
    fn an_unset_radius_uses_the_active_theme() {
        assert_eq!(MaterialCorner::from_setting(None, 0), MaterialCorner::Sharp);
        assert_eq!(
            MaterialCorner::from_setting(None, DEFAULT_THEME_RADIUS_PX),
            MaterialCorner::Rounded
        );
        assert_eq!(
            MaterialCorner::from_setting(Some(0), DEFAULT_THEME_RADIUS_PX),
            MaterialCorner::Sharp,
            "an explicit 0 wins over the theme"
        );
        assert_eq!(
            MaterialCorner::from_setting(Some(12), 0),
            MaterialCorner::Rounded,
            "an explicit radius wins over a sharp theme"
        );
        assert_eq!(effective_radius(None, 8), 8);
        assert_eq!(effective_radius(Some(0), 8), 0);
    }

    #[test]
    fn the_path_names_the_stem_and_the_corner() {
        assert_eq!(
            material_icon_path("lightbulb", MaterialCorner::Rounded),
            "action-icons/lightbulb-rounded.svg"
        );
        assert_eq!(
            material_icon_path("backlight-high-off", MaterialCorner::Sharp),
            "action-icons/backlight-high-off-sharp.svg"
        );
    }
}
