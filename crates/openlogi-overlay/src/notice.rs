//! Notices: one card per notice, stacked on the platform screen edge, telling
//! the user something the agent saw — today, that a device's battery is low.
//!
//! The agent decides what is showing and for how long
//! ([`openlogi_ipc::NoticeObservation`]); this module only draws it. The window
//! opens when the first card arrives, is replaced whenever the cards change,
//! and closes when none are left. Clicking a card dismisses it here. The agent
//! still lists it until its time is up, so [`NoticeShelf`] remembers the
//! dismissal for exactly as long as the agent does.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use gpui::{
    AppContext as _, Context, Hsla, InteractiveElement, IntoElement, ParentElement, Pixels, Render,
    Size, StatefulInteractiveElement as _, Styled, Window, WindowBackgroundAppearance, WindowKind,
    WindowOptions, div, px, size,
};
use openlogi_ipc::{Notice, NoticeKind};
use openlogi_ui::action_icons::{MaterialSymbolStyle, low_battery_icon};
use tracing::warn;

use crate::edge::{self, Edge};
use crate::platform::NoticePlacement;

/// The most cards shown at once. Older ones wait out their time unseen rather
/// than grow the stack across the screen.
pub(crate) const MAX_CARDS: usize = 3;
/// The battery card's size, shared with the charging popup.
pub(crate) const CARD_WIDTH: f32 = crate::osd::CHARGING_WIDTH;
pub(crate) const CARD_HEIGHT: f32 = crate::osd::CHARGING_HEIGHT;
pub(crate) const CARD_GAP: f32 = 8.0;
/// Clear space around the stack, so the cards' shadow is not cut off at the
/// window edge.
pub(crate) const STACK_INSET: f32 = 12.0;
/// Window identity for notices, distinct from the ring's so a compositor or a
/// window rule can tell them apart.
pub(crate) const NOTICE_APP_ID: &str = "openlogi-notice";

/// The battery glyph and bar fill: amber, the colour of a warning that needs no action yet.
const WARNING: Hsla = Hsla {
    h: 38.0 / 360.0,
    s: 0.92,
    l: 0.56,
    a: 1.0,
};

/// What the overlay remembers between the agent's lists: the latest list,
/// which cards the user dismissed, and which cards are on screen.
#[derive(Debug, Default)]
pub(crate) struct NoticeShelf {
    dismissed: BTreeSet<u64>,
    /// The agent's latest list, including notices beyond [`MAX_CARDS`], so a
    /// dismissal can make room for one of them without a new list.
    current: Vec<Notice>,
    showing: Vec<Notice>,
}

impl NoticeShelf {
    /// Take the agent's latest list. Returns the cards to show when they
    /// differ from the cards on screen, and `None` when nothing visible
    /// changed.
    pub(crate) fn observe(&mut self, notices: Vec<Notice>) -> Option<Vec<Notice>> {
        // A dismissal only has to outlive the agent's own copy of the notice.
        self.dismissed
            .retain(|id| notices.iter().any(|notice| notice.id == *id));
        self.current = notices;
        let cards = visible(&self.current, &self.dismissed);
        (cards != self.showing).then(|| self.show(cards))
    }

    /// The user dismissed card `id`. Returns the cards to show now, including
    /// a waiting notice that moves into the freed place.
    pub(crate) fn dismiss(&mut self, id: u64) -> Vec<Notice> {
        self.dismissed.insert(id);
        let cards = visible(&self.current, &self.dismissed);
        self.show(cards)
    }

    fn show(&mut self, cards: Vec<Notice>) -> Vec<Notice> {
        self.showing.clone_from(&cards);
        cards
    }
}

/// The newest [`MAX_CARDS`] notices the user has not dismissed, oldest first.
fn visible(notices: &[Notice], dismissed: &BTreeSet<u64>) -> Vec<Notice> {
    let mut cards: Vec<Notice> = notices
        .iter()
        .filter(|notice| !dismissed.contains(&notice.id))
        .cloned()
        .collect();
    let excess = cards.len().saturating_sub(MAX_CARDS);
    cards.drain(..excess);
    cards
}

/// The window size that holds `count` cards and the clear space around them.
pub(crate) fn stack_size(count: usize) -> Size<Pixels> {
    let count = count.clamp(1, MAX_CARDS);
    // `count` is at most `MAX_CARDS`, so the conversion is exact.
    let cards = f32::from(u8::try_from(count).unwrap_or(u8::MAX));
    let stack_height = CARD_HEIGHT * cards + CARD_GAP * (cards - 1.0);
    size(
        px(CARD_WIDTH + 2.0 * STACK_INSET),
        px(stack_height + 2.0 * STACK_INSET),
    )
}

/// Appearance shared by every transient card window.
///
/// `app_id` is the window identity a compositor or a window rule matches.
/// Notices pass [`NOTICE_APP_ID`]; the backlight and lock-key indicators pass
/// their own.
pub(crate) fn card_window_options(app_id: &str) -> WindowOptions {
    WindowOptions {
        titlebar: None,
        focus: false,
        show: true,
        kind: WindowKind::PopUp,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Transparent,
        app_id: Some(app_id.to_string()),
        ..WindowOptions::default()
    }
}

/// The words on a low-battery card: the device and "Low battery" on top,
/// the charge at the end of the bar.
pub(crate) type NoticeText = crate::charging::BatteryCopy;

/// The text of one notice, in the active locale.
pub(crate) fn notice_text(kind: &NoticeKind) -> NoticeText {
    match kind {
        NoticeKind::LowBattery {
            name,
            kind,
            battery,
        } => {
            let device = name
                .clone()
                .unwrap_or_else(|| rust_i18n::t!(kind.translation_key()).into_owned());
            NoticeText {
                headline: crate::charging::headline(&device, &rust_i18n::t!("device.low_battery")),
                charge: crate::charging::charge_label(battery.percentage, battery.level),
            }
        }
    }
}

/// Show `cards`, replacing any notice window that is open.
pub(crate) fn present(cx: &mut gpui::App, shelf: &Rc<RefCell<NoticeShelf>>, cards: &[Notice]) {
    close_notice_windows(cx);
    if cards.is_empty() {
        return;
    }
    #[cfg_attr(
        windows,
        expect(
            unused_mut,
            reason = "only the macOS and Linux placements drain their openings"
        )
    )]
    let mut placement = match NoticePlacement::capture(cx, stack_size(cards.len())) {
        Ok(placement) => placement,
        Err(error) => {
            warn!(%error, "could not locate the notice display");
            return;
        }
    };
    // Wayland offers a fallback for compositors without layer-shell: the
    // first window that opens is the notice.
    let alternatives = placement
        .openings()
        .into_iter()
        .map(|options| vec![options]);
    let opened = crate::platform::open_first(alternatives.collect(), |options| {
        let view = NoticeView {
            cards: cards.to_vec(),
            shelf: Rc::clone(shelf),
        };
        cx.open_window(options, |_, cx| cx.new(|_| view))
    });
    let Some(handle) = opened else {
        return;
    };
    let placed = handle
        .update(cx, |_, window, cx| {
            // As for the ring: native placement resizes the window inside
            // this update, so resync GPUI's viewport and scale.
            placement.show(window).map(|()| window.bounds_changed(cx))
        })
        .and_then(std::convert::identity);
    if let Err(error) = placed {
        warn!(%error, "could not position the notice window");
        let _ = handle.update(cx, |_, window, _| window.remove_window());
    }
}

/// Close every notice window.
fn close_notice_windows(cx: &mut gpui::App) {
    for handle in cx.windows() {
        if let Some(notice) = handle.downcast::<NoticeView>() {
            let _ = notice.update(cx, |_, window, _| window.remove_window());
        }
    }
}

/// The low-battery card, shared by the notice window and the preview board.
///
/// The same two rows as the charging popup, in the warning amber.
pub(crate) fn notice_card(notice: &Notice, style: MaterialSymbolStyle) -> gpui::Div {
    let fraction = match &notice.kind {
        NoticeKind::LowBattery { battery, .. } => {
            crate::charging::charge_fraction(battery.percentage)
        }
    };
    crate::charging::battery_card(crate::charging::BatteryCard {
        icon: low_battery_icon(style),
        colour: WARNING,
        copy: notice_text(&notice.kind),
        fraction,
    })
}

/// The notice window's view: the stacked cards, newest against the platform edge.
pub(crate) struct NoticeView {
    cards: Vec<Notice>,
    shelf: Rc<RefCell<NoticeShelf>>,
}

impl NoticeView {
    fn card(&self, notice: &Notice, style: MaterialSymbolStyle) -> impl IntoElement + use<> {
        let id = notice.id;
        let notice_shelf = Rc::clone(&self.shelf);
        notice_card(notice, style)
            .id(("notice", id))
            .cursor_pointer()
            .shadow_lg()
            .on_click(move |_, window, cx| {
                // This window is checked out of the app while its handler
                // runs, so close it here and redraw what is left afterwards.
                window.remove_window();
                let notice_shelf = Rc::clone(&notice_shelf);
                cx.defer(move |cx| {
                    let cards = notice_shelf.borrow_mut().dismiss(id);
                    present(cx, &notice_shelf, &cards);
                });
            })
    }
}

impl Render for NoticeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let style = crate::icons::style(cx);
        // Shown in place on the platform edge. There is no entrance animation;
        // the newest card is the one against that edge.
        let edge = Edge::platform_default();
        let cards = edge::newest_against_edge(edge, &self.cards);
        let stack = div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(CARD_GAP))
            .p(px(STACK_INSET));
        let stack = if edge.newest_first() {
            stack.justify_start()
        } else {
            stack.justify_end()
        };
        stack.children(cards.into_iter().map(|notice| self.card(notice, style)))
    }
}

#[cfg(test)]
mod tests;
