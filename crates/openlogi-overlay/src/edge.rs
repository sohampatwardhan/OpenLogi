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

/// Distance from the chosen edge to the card window, in logical pixels.
///
/// The usable area already excludes the menu bar, notch, Dock, taskbar, or
/// panel. This is the gap inside what remains, so the card's shadow is not
/// flush with that chrome. Windows scales it with the monitor DPI.
pub(crate) const EDGE_MARGIN: f32 = 48.0;

/// Space between two cards stacked on the same edge, in logical pixels.
pub(crate) const STACK_GAP: f32 = 12.0;

/// Where a transient overlay card sits in its display.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Edge {
    /// The top of the usable area. The newest stacked card is the one nearest it.
    #[cfg_attr(
        all(not(test), not(target_os = "macos")),
        expect(dead_code, reason = "only macOS's platform default is the top")
    )]
    Top,
    /// The bottom of the usable area. The newest stacked card is the one nearest it.
    #[cfg_attr(
        all(not(test), target_os = "macos"),
        expect(dead_code, reason = "macOS's platform default is the top")
    )]
    Bottom,
    /// The centre of the whole display, on every operating system.
    ///
    /// The backlight effect list uses this. That list is the vertical
    /// switcher, and it stays centred instead of following
    /// [`Edge::platform_default`].
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
    #[cfg_attr(
        all(not(test), not(target_os = "linux")),
        expect(
            dead_code,
            reason = "only Linux layer-shell surfaces take a per-side margin"
        )
    )]
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

/// [`origin`] for a GPUI logical-pixel rectangle, `margin` from `edge`.
#[must_use]
pub(crate) fn origin_px(
    edge: Edge,
    area: Bounds<Pixels>,
    window: Size<Pixels>,
    margin: f32,
) -> Point<Pixels> {
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
        margin,
    );
    point(px(placed.x), px(placed.y))
}

/// A card already open on the platform edge: how far past [`EDGE_MARGIN`]
/// it sits, and its height, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Stacked {
    /// Distance from the margin to the card's edge-side border.
    pub(crate) offset: f32,
    /// The card's height.
    pub(crate) height: f32,
}

/// How far past [`EDGE_MARGIN`] a `height` card opens so it covers none of
/// `open`: the gap nearest the edge that fits it, with [`STACK_GAP`] to
/// each neighbour, else past the farthest card.
///
/// The cards are separate windows that come and go on their own clocks, so
/// a card that closes leaves its gap, and the next card fills it.
#[must_use]
pub(crate) fn stack_offset(open: &[Stacked], height: f32) -> f32 {
    let mut taken = open.to_vec();
    taken.sort_by(|a, b| a.offset.total_cmp(&b.offset));
    let mut offset = 0.0_f32;
    for card in taken {
        if offset + height + STACK_GAP <= card.offset {
            break;
        }
        offset = offset.max(card.offset + card.height + STACK_GAP);
    }
    offset
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

/// Space kept between a centred card and an edge card it was moved clear of.
pub(crate) const CLEARANCE: f32 = 12.0;

/// `placed`, a window of `window` size, moved off `keep_out` and kept
/// inside `area`.
///
/// A window that does not overlap `keep_out` (with [`CLEARANCE`] around it)
/// stays where it is. One that does moves on the vertical axis only, to the
/// far side of `keep_out` from that card's centre: below a card at the top,
/// above one at the bottom. When `area` has no room there, the window is
/// clamped to `area` rather than leaving it.
#[must_use]
pub(crate) fn clear_of(placed: Origin, window: Span, area: Rect, keep_out: Rect) -> Origin {
    let overlaps_x = placed.x < keep_out.x + keep_out.width && keep_out.x < placed.x + window.width;
    let overlaps_y = placed.y < keep_out.y + keep_out.height + CLEARANCE
        && keep_out.y - CLEARANCE < placed.y + window.height;
    if !(overlaps_x && overlaps_y) {
        return placed;
    }
    let keep_out_centre = keep_out.y + keep_out.height / 2.0;
    let window_centre = placed.y + window.height / 2.0;
    let y = if keep_out_centre <= window_centre {
        keep_out.y + keep_out.height + CLEARANCE
    } else {
        keep_out.y - CLEARANCE - window.height
    };
    let max_y = (area.y + area.height - window.height).max(area.y);
    Origin {
        x: placed.x,
        y: y.clamp(area.y, max_y),
    }
}

/// Top-left of a [`Edge::Centre`] card on `display` that does not cover an
/// `edge_card` sized card on `edge` of `usable`.
///
/// The backlight effect list is centred while the brightness card follows
/// [`Edge::platform_default`], and both can be up at once: one key changes
/// the level, another the effect. On macOS that card sits under the menu bar,
/// so a list tall enough to reach it is pushed down below it instead.
/// [`Edge::Centre`] as `edge` keeps nothing clear.
#[must_use]
pub(crate) fn centre_clear_of(
    display: Rect,
    usable: Rect,
    window: Span,
    edge: Edge,
    edge_card: Span,
) -> Origin {
    let placed = origin(Edge::Centre, display, window, 0.0);
    if edge == Edge::Centre {
        return placed;
    }
    let card = origin(edge, usable, edge_card, EDGE_MARGIN);
    clear_of(
        placed,
        window,
        display,
        Rect {
            x: card.x,
            y: card.y,
            width: edge_card.width,
            height: edge_card.height,
        },
    )
}

/// [`centre_clear_of`] for GPUI logical-pixel rectangles, keeping clear of
/// a card on [`Edge::platform_default`].
#[must_use]
pub(crate) fn centre_clear_of_px(
    display: Bounds<Pixels>,
    usable: Bounds<Pixels>,
    window: Size<Pixels>,
    edge_card: Size<Pixels>,
) -> Point<Pixels> {
    let rect = |area: Bounds<Pixels>| Rect {
        x: area.origin.x.as_f32(),
        y: area.origin.y.as_f32(),
        width: area.size.width.as_f32(),
        height: area.size.height.as_f32(),
    };
    let span = |size: Size<Pixels>| Span {
        width: size.width.as_f32(),
        height: size.height.as_f32(),
    };
    let placed = centre_clear_of(
        rect(display),
        rect(usable),
        span(window),
        Edge::platform_default(),
        span(edge_card),
    );
    point(px(placed.x), px(placed.y))
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
            origin_px(Edge::Top, usable, card, EDGE_MARGIN),
            point(px(148.0), px(1128.0))
        );
        assert_eq!(
            origin_px(Edge::Bottom, usable, card, EDGE_MARGIN),
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

    #[test]
    fn a_short_centred_list_stays_centred_beside_a_top_card() {
        // A 1440x900 display under a 25pt menu bar.
        let display = area(0.0, 0.0, 1440.0, 900.0);
        let usable = area(0.0, 25.0, 1440.0, 875.0);
        let list = window(360.0, 280.0);
        let card = window(640.0, 64.0);
        assert_eq!(
            centre_clear_of(display, usable, list, Edge::Top, card),
            origin(Edge::Centre, display, list, 0.0)
        );
    }

    #[test]
    fn a_tall_centred_list_moves_below_the_top_card() {
        // Card: 25 + 48 = 73 to 137. A 700-tall list would start at 100.
        let display = area(0.0, 0.0, 1440.0, 900.0);
        let usable = area(0.0, 25.0, 1440.0, 875.0);
        let list = window(360.0, 700.0);
        let card = window(640.0, 64.0);
        let placed = centre_clear_of(display, usable, list, Edge::Top, card);
        assert_eq!(placed.x, 540.0);
        assert_eq!(placed.y, 73.0 + 64.0 + CLEARANCE);
    }

    #[test]
    fn a_list_that_cannot_clear_the_card_still_stays_on_the_display() {
        let display = area(0.0, 0.0, 1440.0, 900.0);
        let usable = area(0.0, 25.0, 1440.0, 875.0);
        let list = window(360.0, 880.0);
        let card = window(640.0, 64.0);
        let placed = centre_clear_of(display, usable, list, Edge::Top, card);
        assert_eq!(placed.y, 20.0);
    }

    #[test]
    fn a_tall_centred_list_moves_above_a_bottom_card() {
        // Card: 1080 - 48 (panel) - 48 - 64 = 920 to 984.
        let display = area(0.0, 0.0, 1920.0, 1080.0);
        let usable = area(0.0, 0.0, 1920.0, 1032.0);
        let list = window(360.0, 900.0);
        let card = window(300.0, 64.0);
        let placed = centre_clear_of(display, usable, list, Edge::Bottom, card);
        assert_eq!(placed.y, 920.0 - CLEARANCE - 900.0);
    }

    #[test]
    fn a_centre_edge_keeps_nothing_clear() {
        let display = area(0.0, 0.0, 1440.0, 900.0);
        let list = window(360.0, 700.0);
        assert_eq!(
            centre_clear_of(display, display, list, Edge::Centre, window(640.0, 64.0)),
            origin(Edge::Centre, display, list, 0.0)
        );
    }

    #[test]
    fn a_card_beside_the_window_is_not_in_the_way() {
        let display = area(0.0, 0.0, 1000.0, 1000.0);
        let placed = Origin { x: 0.0, y: 0.0 };
        let beside = Rect {
            x: 600.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        assert_eq!(
            clear_of(placed, window(300.0, 300.0), display, beside),
            placed
        );
    }

    #[test]
    fn a_card_opens_past_the_cards_already_on_its_edge() {
        let card = |offset, height| Stacked { offset, height };
        assert_eq!(
            stack_offset(&[], 60.0),
            0.0,
            "an empty edge is at the margin"
        );
        assert_eq!(
            stack_offset(&[card(0.0, 60.0)], 80.0),
            60.0 + STACK_GAP,
            "the second card sits past the first"
        );
        assert_eq!(
            stack_offset(&[card(60.0 + STACK_GAP, 80.0), card(0.0, 60.0)], 40.0),
            60.0 + STACK_GAP + 80.0 + STACK_GAP,
            "order of the open cards does not matter"
        );
    }

    #[test]
    fn a_closed_card_leaves_a_gap_only_a_card_that_fits_takes() {
        let card = |offset, height| Stacked { offset, height };
        // The 60px card at the margin closed; an 80px card stays past it.
        let open = [card(60.0 + STACK_GAP, 80.0)];
        assert_eq!(
            stack_offset(&open, 60.0),
            0.0,
            "a card that fits fills the gap"
        );
        assert_eq!(
            stack_offset(&open, 100.0),
            60.0 + STACK_GAP + 80.0 + STACK_GAP,
            "a taller card would cover its neighbour, so it goes past it"
        );
    }
}
