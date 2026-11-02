//! Where a transient overlay card sits, and the point that puts it there.
//!
//! Notices, the backlight level, and the lock-key indicator ask
//! [`Edge::platform_default`] and then [`origin`]. macOS keeps those cards
//! under the menu bar and the notch. Windows and Linux keep today's place,
//! above the taskbar or panel. The backlight effect list is the exception:
//! it passes [`Edge::Centre`] and stays in the middle of the display on every
//! OS. The overlay does not animate a card onto the screen. A card is shown
//! already in place, so a later slide or fade would come from the top on
//! macOS and from the bottom on the other two. A centred card fades in the
//! middle.

#[cfg(any(target_os = "windows", test))]
use gpui::DevicePixels;
use gpui::{Bounds, Pixels, Point, Size, point, px};

/// [`Edge::Top`] as a value, so every host's bin constructs it.
pub(crate) const TOP: Edge = Edge::Top;
/// [`Edge::Bottom`] as a value, so every host's bin constructs it.
pub(crate) const BOTTOM: Edge = Edge::Bottom;
/// The backlight effect list: centred on every operating system.
///
/// That list does not follow [`Edge::platform_default`]. Pass this to
/// [`crate::popup::open_at`].
pub(crate) const EFFECT_LIST: Edge = Edge::Centre;

/// Return `edge`.
///
/// The match names [`TOP`], [`BOTTOM`], and [`EFFECT_LIST`], which is what
/// keeps every variant constructed on hosts whose [`Edge::platform_default`]
/// is only one of them.
#[must_use]
pub(crate) const fn selected(edge: Edge) -> Edge {
    match edge {
        Edge::Top => TOP,
        Edge::Bottom => BOTTOM,
        Edge::Centre => EFFECT_LIST,
    }
}

/// Distance from the chosen edge to the card window, in logical pixels.
///
/// The usable area already excludes the menu bar, notch, Dock, taskbar, or
/// panel. This is the gap inside what remains, so the card's shadow is not
/// flush with that chrome. Windows scales it with the monitor DPI.
pub(crate) const EDGE_MARGIN: f32 = 48.0;

/// Where a transient overlay card sits in its display.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Edge {
    /// The top of the usable area. The newest stacked card is the one nearest it.
    Top,
    /// The bottom of the usable area. The newest stacked card is the one nearest it.
    Bottom,
    /// The centre of the whole display, on every operating system.
    ///
    /// The backlight effect list uses this. That list is the vertical
    /// switcher, and it stays centred instead of following
    /// [`Edge::platform_default`]. Pass it to [`crate::popup::open_at`].
    Centre,
}

impl Edge {
    /// The edge transient cards use on this operating system.
    ///
    /// macOS is [`Edge::Top`]. Windows and Linux are [`Edge::Bottom`].
    /// [`Edge::Centre`] is never the default; a card opts into it.
    #[must_use]
    pub(crate) const fn platform_default() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self::Top
        }
        #[cfg(not(target_os = "macos"))]
        {
            Self::Bottom
        }
    }

    /// Whether a top-to-bottom column lists the newest card first.
    ///
    /// The newest card sits against the screen edge: first when the edge is
    /// the top, so older cards flow downward, and last when the edge is the
    /// bottom, which is the order the stack has always used. A centred card
    /// is not a stack, so it keeps oldest-first.
    #[must_use]
    pub(crate) const fn newest_first(self) -> bool {
        matches!(self, Self::Top)
    }

    /// Whether the card is placed on the whole display rather than the usable area.
    ///
    /// Top and bottom use the usable area, which already excludes the menu
    /// bar, notch, Dock, taskbar, or panel. Centre uses the whole display,
    /// which is where a centred window sits today.
    #[must_use]
    pub(crate) const fn in_full_display(self) -> bool {
        matches!(self, Self::Centre)
    }

    /// Layer-shell margins in CSS order: top, right, bottom, left.
    ///
    /// Only the side this edge anchors to receives `margin`. Centre anchors
    /// to no side, so every component is zero.
    #[must_use]
    pub(crate) const fn inset(self, margin: f32) -> (f32, f32, f32, f32) {
        match self {
            Self::Top => (margin, 0.0, 0.0, 0.0),
            Self::Bottom => (0.0, 0.0, margin, 0.0),
            Self::Centre => (0.0, 0.0, 0.0, 0.0),
        }
    }
}

/// A rectangle in a top-left-origin space.
///
/// The origin may be negative. A display to the left of the primary one has
/// a negative `x`, and a display above it has a negative `y`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rect {
    /// Left edge.
    pub(crate) x: f32,
    /// Top edge.
    pub(crate) y: f32,
    /// Width. Zero is an empty area.
    pub(crate) width: f32,
    /// Height. Zero is an empty area.
    pub(crate) height: f32,
}

/// Width and height of the window being placed, in the same unit as [`Rect`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Span {
    /// Window width.
    pub(crate) width: f32,
    /// Window height.
    pub(crate) height: f32,
}

/// Top-left of a window after [`origin`] places it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Origin {
    /// Left edge of the window.
    pub(crate) x: f32,
    /// Top edge of the window.
    pub(crate) y: f32,
}

/// Top-left of `window` on `edge` of `area`.
///
/// Top and bottom centre the window on the other axis and leave `margin`
/// between it and that edge. [`Edge::Centre`] centres it on both axes and
/// ignores `margin`. The window is kept inside `area`. When the window is
/// larger than the area, or the margin would push it out, it anchors to the
/// area's top-left rather than extending outside.
#[must_use]
pub(crate) fn origin(edge: Edge, area: Rect, window: Span, margin: f32) -> Origin {
    let raw_x = area.x + (area.width - window.width) / 2.0;
    let raw_y = match edge {
        Edge::Top => area.y + margin,
        Edge::Bottom => area.y + area.height - window.height - margin,
        Edge::Centre => area.y + (area.height - window.height) / 2.0,
    };
    let max_x = (area.x + area.width - window.width).max(area.x);
    let max_y = (area.y + area.height - window.height).max(area.y);
    Origin {
        x: raw_x.clamp(area.x, max_x),
        y: raw_y.clamp(area.y, max_y),
    }
}

/// [`origin`] for a GPUI logical-pixel rectangle, inset by [`EDGE_MARGIN`].
#[must_use]
pub(crate) fn origin_px(edge: Edge, area: Bounds<Pixels>, window: Size<Pixels>) -> Point<Pixels> {
    let placed = origin(
        edge,
        Rect {
            x: area.origin.x.as_f32(),
            y: area.origin.y.as_f32(),
            width: area.size.width.as_f32(),
            height: area.size.height.as_f32(),
        },
        Span {
            width: window.width.as_f32(),
            height: window.height.as_f32(),
        },
        EDGE_MARGIN,
    );
    point(px(placed.x), px(placed.y))
}

/// [`origin`] for Windows physical pixels.
///
/// The centre is the exact half of the leftover width, then rounded to the
/// nearest physical pixel. The margin is already in physical pixels.
///
/// The Windows module calls this. A non-Windows bin does not compile that
/// module; tests do, on every host.
#[cfg(any(target_os = "windows", test))]
#[must_use]
pub(crate) fn origin_device(
    edge: Edge,
    area: Bounds<DevicePixels>,
    window: Size<DevicePixels>,
    margin: DevicePixels,
) -> Point<DevicePixels> {
    let placed = origin(
        edge,
        Rect {
            x: f32_from_device(area.origin.x),
            y: f32_from_device(area.origin.y),
            width: f32_from_device(area.size.width),
            height: f32_from_device(area.size.height),
        },
        Span {
            width: f32_from_device(window.width),
            height: f32_from_device(window.height),
        },
        f32_from_device(margin),
    );
    point(device_from_f32(placed.x), device_from_f32(placed.y))
}

/// `cards` is oldest-first. The returned order paints top-to-bottom so the
/// newest card is the one against `edge`.
#[must_use]
pub(crate) fn newest_against_edge<T>(edge: Edge, oldest_first: &[T]) -> Vec<&T> {
    if edge.newest_first() {
        oldest_first.iter().rev().collect()
    } else {
        oldest_first.iter().collect()
    }
}

/// Screen coordinates are a few tens of thousands, which are exact in `f32`.
#[cfg(any(target_os = "windows", test))]
#[expect(
    clippy::cast_precision_loss,
    reason = "physical screen coordinates are exact in f32 well past any monitor layout"
)]
fn f32_from_device(pixels: DevicePixels) -> f32 {
    pixels.0 as f32
}

/// The matching rounding of [`f32_from_device`]. Placement never produces a
/// value outside `i32`.
#[cfg(any(target_os = "windows", test))]
#[expect(
    clippy::cast_possible_truncation,
    reason = "rounded screen coordinates fit in i32; monitors do not reach the ends of that range"
)]
fn device_from_f32(value: f32) -> DevicePixels {
    DevicePixels(value.round() as i32)
}

#[cfg(test)]
mod tests {
    use gpui::size;

    use super::*;

    fn area(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    fn window(width: f32, height: f32) -> Span {
        Span { width, height }
    }

    #[test]
    fn the_host_picks_one_edge() {
        let expected = if cfg!(target_os = "macos") {
            Edge::Top
        } else {
            Edge::Bottom
        };
        assert_eq!(Edge::platform_default(), expected);
        assert_ne!(Edge::platform_default(), Edge::Centre);
        assert!(Edge::Top.newest_first());
        assert!(!Edge::Bottom.newest_first());
        assert!(!Edge::Centre.newest_first());
        assert!(Edge::Centre.in_full_display());
        assert!(!Edge::Top.in_full_display());
        assert!(!Edge::Bottom.in_full_display());
    }

    #[test]
    fn the_margin_insets_only_the_anchored_side() {
        assert_eq!(Edge::Top.inset(EDGE_MARGIN), (48.0, 0.0, 0.0, 0.0));
        assert_eq!(Edge::Bottom.inset(EDGE_MARGIN), (0.0, 0.0, 48.0, 0.0));
        assert_eq!(Edge::Centre.inset(EDGE_MARGIN), (0.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn a_centred_card_stays_in_the_middle_on_every_display() {
        // The same numbers on macOS, Windows, and Linux: centre does not
        // follow the platform edge, and the margin does not shift it.
        let primary = area(0.0, 0.0, 1512.0, 982.0);
        let card = window(344.0, 88.0);
        let middle = Origin { x: 584.0, y: 447.0 };
        assert_eq!(origin(Edge::Centre, primary, card, 0.0), middle);
        assert_eq!(origin(Edge::Centre, primary, card, EDGE_MARGIN), middle);
        // Above-left of the primary, menu bar already included in the full
        // display rectangle. (1920 - 344) / 2 = 788. (1055 - 88) / 2 = 483.5.
        let other = area(-2560.0, -1080.0, 1920.0, 1055.0);
        assert_eq!(
            origin(Edge::Centre, other, card, EDGE_MARGIN),
            Origin {
                x: -1772.0,
                y: -596.5
            }
        );
        let parked = Origin { x: 100.0, y: 50.0 };
        assert_eq!(
            origin(
                Edge::Centre,
                area(100.0, 50.0, 200.0, 60.0),
                card,
                EDGE_MARGIN
            ),
            parked
        );
    }

    #[test]
    fn a_card_sits_centred_on_the_top_or_the_bottom_of_the_usable_area() {
        // 1512×982 primary with a 37pt menu bar and a 70pt Dock, which is the
        // usable rectangle `usable_in_display` already produces.
        let usable = area(0.0, 37.0, 1512.0, 875.0);
        let card = window(344.0, 88.0);
        assert_eq!(
            origin(Edge::Bottom, usable, card, EDGE_MARGIN),
            Origin { x: 584.0, y: 776.0 },
            "bottom keeps today's place: (1512 - 344) / 2 and 37 + 875 - 88 - 48"
        );
        assert_eq!(
            origin(Edge::Top, usable, card, EDGE_MARGIN),
            Origin { x: 584.0, y: 85.0 },
            "top is the same centre, one margin below the menu bar"
        );
    }

    #[test]
    fn a_display_away_from_the_origin_keeps_its_own_edges() {
        // Above-left of the primary: CoreGraphics-style negative origin.
        let usable = area(-2560.0, -1080.0, 1920.0, 1055.0);
        let card = window(344.0, 88.0);
        assert_eq!(
            origin(Edge::Top, usable, card, EDGE_MARGIN),
            Origin {
                x: -1772.0,
                y: -1032.0
            }
        );
        assert_eq!(
            origin(Edge::Bottom, usable, card, EDGE_MARGIN),
            Origin {
                x: -1772.0,
                y: -161.0
            }
        );
        // Right of a 1920-wide primary, with a top inset that is not the
        // global origin (a menu bar on that display alone).
        let shifted = area(1920.0, 25.0, 2560.0, 1415.0);
        assert_eq!(origin(Edge::Top, shifted, card, EDGE_MARGIN).y, 73.0);
        assert_eq!(
            origin(Edge::Bottom, shifted, card, EDGE_MARGIN),
            Origin {
                x: 3028.0,
                y: 1304.0
            }
        );
    }

    #[test]
    fn a_card_larger_than_the_area_anchors_to_the_top_left() {
        let usable = area(100.0, 50.0, 200.0, 60.0);
        let card = window(344.0, 88.0);
        let parked = Origin { x: 100.0, y: 50.0 };
        assert_eq!(origin(Edge::Top, usable, card, EDGE_MARGIN), parked);
        assert_eq!(origin(Edge::Bottom, usable, card, EDGE_MARGIN), parked);
    }

    #[test]
    fn a_margin_that_does_not_fit_stays_inside_the_area() {
        // 70px tall, 40px window, 48px margin: the top request would end at
        // 248 and the bottom request at 182, both outside.
        let usable = area(100.0, 200.0, 400.0, 70.0);
        let card = window(80.0, 40.0);
        assert_eq!(
            origin(Edge::Top, usable, card, EDGE_MARGIN),
            Origin { x: 260.0, y: 230.0 }
        );
        assert_eq!(
            origin(Edge::Bottom, usable, card, EDGE_MARGIN),
            Origin { x: 260.0, y: 200.0 }
        );
    }

    #[test]
    fn logical_and_physical_pixels_follow_the_same_edge() {
        let usable = Bounds::new(point(px(-640.0), px(1080.0)), size(px(1920.0), px(1055.0)));
        let card = size(px(344.0), px(88.0));
        assert_eq!(
            origin_px(Edge::Top, usable, card),
            point(px(148.0), px(1128.0))
        );
        assert_eq!(
            origin_px(Edge::Bottom, usable, card),
            point(px(148.0), px(1999.0))
        );

        let work = Bounds::new(
            point(DevicePixels(-2560), DevicePixels(105)),
            size(DevicePixels(1920), DevicePixels(1392)),
        );
        let physical = size(DevicePixels(345), DevicePixels(89));
        // (1920 - 345) / 2 = 787.5, and -2560 + 787.5 = -1772.5, which
        // rounds halfway away from zero. 105 + 1392 - 89 - 48 = 1360.
        assert_eq!(
            origin_device(Edge::Top, work, physical, DevicePixels(48)),
            point(DevicePixels(-1773), DevicePixels(153))
        );
        assert_eq!(
            origin_device(Edge::Bottom, work, physical, DevicePixels(48)),
            point(DevicePixels(-1773), DevicePixels(1360))
        );
    }

    #[test]
    fn the_newest_card_is_the_one_against_the_edge() {
        let cards = [10_u64, 20, 30];
        assert_eq!(
            newest_against_edge(Edge::Bottom, &cards)
                .into_iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![10, 20, 30],
            "oldest first, so the newest paints nearest the bottom"
        );
        assert_eq!(
            newest_against_edge(Edge::Top, &cards)
                .into_iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![30, 20, 10],
            "newest first, so it paints nearest the top and older cards flow down"
        );
        assert_eq!(
            newest_against_edge(Edge::Centre, &cards)
                .into_iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![10, 20, 30]
        );
    }
}
