//! Open a transient indicator card.
//!
//! The backlight level and the lock-key indicator use [`open`], which follows
//! [`Edge::platform_default`]: the top of the usable area on macOS, the bottom
//! on Windows and Linux. The backlight effect list is the exception. It stays
//! centred on every OS, so it calls [`open_at`] with [`Edge::Centre`].

use gpui::{App, Entity, Pixels, Render, Size, Window, WindowHandle};

use crate::edge::{self, Edge};

/// Open a card on [`Edge::platform_default`].
pub(crate) fn open<V>(
    cx: &mut App,
    app_id: &str,
    size: Size<Pixels>,
    build: impl FnMut(&mut Window, &mut App) -> Entity<V>,
) -> anyhow::Result<WindowHandle<V>>
where
    V: 'static + Render,
{
    open_at(cx, app_id, size, Edge::platform_default(), build)
}

/// Open a card on `edge`.
///
/// [`Edge::Centre`] keeps the backlight effect list in the middle of the
/// primary display on macOS, Windows, and Linux. Every other card passes
/// [`Edge::platform_default`].
pub(crate) fn open_at<V>(
    cx: &mut App,
    app_id: &str,
    size: Size<Pixels>,
    edge: Edge,
    mut build: impl FnMut(&mut Window, &mut App) -> Entity<V>,
) -> anyhow::Result<WindowHandle<V>>
where
    V: 'static + Render,
{
    let edge = edge::selected(edge);
    crate::platform::open_first(
        crate::platform::card_attempts(cx, app_id, size, edge)
            .into_iter()
            .map(|options| vec![options])
            .collect(),
        |options| cx.open_window(options, |window, cx| build(window, cx)),
    )
    .ok_or_else(|| anyhow::anyhow!("could not open the overlay card"))
}
