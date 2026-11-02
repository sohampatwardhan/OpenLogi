//! Compact transient DPI indicator: the DPI a preset button set among the
//! mouse's presets, or precision mode turning on or off.
//!
//! The agent decides what shows and for how long
//! ([`openlogi_ipc::DpiObservation`]); this module only draws it, as an
//! option card ([`crate::option_row`]). A preset lists the presets with the
//! current DPI in the pill, at most [`MAX_PRESETS`] around it. Precision mode
//! lists On and Off.

use gpui::{
    AnyWindowHandle, App, AppContext as _, Context, IntoElement, Render, Window, WindowHandle, px,
    size,
};
use openlogi_ipc::DpiPopup;
use openlogi_ui::action_icons::{DPI_ICON, PRECISION_ICON};
use tracing::warn;

use crate::option_row::{OptionRow, card_width, option_card, window};
use crate::osd;

/// The most presets one card lists.
const MAX_PRESETS: usize = 6;

/// What the card says for `popup`, in the active locale. Its glyphs are
/// Lucide's, which have one cut, so the card draws the same glyph at every
/// corner radius.
pub(crate) fn row(popup: &DpiPopup) -> OptionRow {
    match popup {
        DpiPopup::Preset { dpi, presets } => {
            // Unknown presets, or a DPI that is none of them, name the DPI
            // alone.
            let (options, current) = match presets.iter().position(|preset| preset == dpi) {
                Some(current) => {
                    let (start, len) = window(presets.len(), current, MAX_PRESETS);
                    (presets[start..start + len].to_vec(), current - start)
                }
                None => (vec![*dpi], 0),
            };
            OptionRow {
                icon: DPI_ICON,
                label: rust_i18n::t!("pointer.dpi").into_owned(),
                options: options.iter().map(ToString::to_string).collect(),
                current,
            }
        }
        DpiPopup::Precision { engaged } => OptionRow {
            icon: PRECISION_ICON,
            label: rust_i18n::t!("pointer.precision_mode").into_owned(),
            options: vec![
                rust_i18n::t!("common.on").into_owned(),
                rust_i18n::t!("common.off").into_owned(),
            ],
            current: usize::from(!*engaged),
        },
    }
}

/// The indicator window's view: the popup the agent is showing.
pub(crate) struct DpiView {
    popup: DpiPopup,
    /// The window's width, measured when it opened.
    width: f32,
}

impl Render for DpiView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        option_card(&row(&self.popup), self.width)
    }
}

/// Show `shown`, or close the indicator when it is `None`.
///
/// A press while the window is open replaces its popup in place, so cycling
/// through the presets moves the pill along one card. A popup that needs
/// another width reopens the card instead.
pub(crate) fn present(cx: &mut App, shown: Option<DpiPopup>) {
    let open: Vec<WindowHandle<DpiView>> = cx
        .windows()
        .iter()
        .filter_map(AnyWindowHandle::downcast::<DpiView>)
        .collect();
    let Some(popup) = shown else {
        for view in open {
            let _ = view.update(cx, |_, window, _| window.remove_window());
        }
        return;
    };
    let width = card_width(cx, &row(&popup));
    // Resizing in place keeps the top-left, so the card would drift.
    let reopen = open.is_empty()
        || open.iter().any(|view| {
            view.read_with(cx, |view, _| (view.width - width).abs() > f32::EPSILON)
                .unwrap_or(true)
        });
    if !reopen {
        for view in open {
            let popup = popup.clone();
            let _ = view.update(cx, |view, _, cx| {
                view.popup = popup;
                cx.notify();
            });
        }
        return;
    }
    for view in open {
        let _ = view.update(cx, |_, window, _| window.remove_window());
    }
    if let Err(error) = crate::popup::open(
        cx,
        "openlogi-dpi-osd",
        size(px(width), px(osd::LOCK_HEIGHT)),
        move |_, cx| {
            cx.new(|_| DpiView {
                popup: popup.clone(),
                width,
            })
        },
    ) {
        warn!(%error, "could not open the DPI indicator window");
    }
}

#[cfg(test)]
mod tests;
