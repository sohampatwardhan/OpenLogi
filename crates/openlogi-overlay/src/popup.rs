//! Open a transient indicator card.
//!
//! The backlight level and the lock-key indicator use [`open`], which follows
//! [`Edge::platform_default`]: the top of the usable area on macOS, the bottom
//! on Windows and Linux. Cards
//! up at the same time stack away from that edge instead of opening on top of
//! each other. The backlight effect list is the exception. It stays centred
//! on every OS, so it calls [`open_centred_clear_of`], which opens at
//! [`Edge::Centre`] and keeps clear of the brightness card on the platform
//! edge when the list is tall enough to reach it.

use gpui::{AnyWindowHandle, App, Entity, Global, Pixels, Render, Size, Window, WindowHandle};

use crate::edge::{self, EDGE_MARGIN, Edge, Stacked};

/// The cards open on the platform edge and where each sits, so the next one
/// opens clear of them. A closed card's entry goes on the next [`open`].
#[derive(Default)]
struct EdgeStack(Vec<(AnyWindowHandle, Stacked)>);

impl Global for EdgeStack {}

/// Open a card on [`Edge::platform_default`], past any card already there.
pub(crate) fn open<V>(
    cx: &mut App,
    app_id: &str,
    size: Size<Pixels>,
    build: impl FnMut(&mut Window, &mut App) -> Entity<V>,
) -> anyhow::Result<WindowHandle<V>>
where
    V: 'static + Render,
{
    let live = cx.windows();
    let stack = cx.default_global::<EdgeStack>();
    stack.0.retain(|(handle, _)| live.contains(handle));
    let open: Vec<Stacked> = stack.0.iter().map(|(_, card)| *card).collect();
    let height = size.height.as_f32();
    let card = Stacked {
        offset: edge::stack_offset(&open, height),
        height,
    };
    let handle = open_with(
        cx,
        app_id,
        size,
        Edge::platform_default(),
        EDGE_MARGIN + card.offset,
        None,
        build,
    )?;
    cx.default_global::<EdgeStack>()
        .0
        .push((handle.into(), card));
    Ok(handle)
}

/// Open a card at [`Edge::Centre`] that does not cover an `edge_card` sized
/// card on [`Edge::platform_default`].
///
/// The backlight effect list passes the brightness card's largest size, so
/// the two never overlap when both are up. A list short enough to clear that
/// card stays exactly centred, on macOS, Windows, and Linux.
pub(crate) fn open_centred_clear_of<V>(
    cx: &mut App,
    app_id: &str,
    size: Size<Pixels>,
    edge_card: Size<Pixels>,
    build: impl FnMut(&mut Window, &mut App) -> Entity<V>,
) -> anyhow::Result<WindowHandle<V>>
where
    V: 'static + Render,
{
    open_with(
        cx,
        app_id,
        size,
        Edge::Centre,
        EDGE_MARGIN,
        Some(edge_card),
        build,
    )
}

/// Open a card `margin` from `edge`, keeping a centred one clear of
/// `keep_clear`.
fn open_with<V>(
    cx: &mut App,
    app_id: &str,
    size: Size<Pixels>,
    edge: Edge,
    margin: f32,
    keep_clear: Option<Size<Pixels>>,
    mut build: impl FnMut(&mut Window, &mut App) -> Entity<V>,
) -> anyhow::Result<WindowHandle<V>>
where
    V: 'static + Render,
{
    crate::platform::open_first(
        crate::platform::card_attempts(cx, app_id, size, edge, margin, keep_clear)
            .into_iter()
            .map(|options| vec![options])
            .collect(),
        |options| cx.open_window(options, |window, cx| build(window, cx)),
    )
    .ok_or_else(|| anyhow::anyhow!("could not open the overlay card"))
}
