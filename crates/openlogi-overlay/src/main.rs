//! Lightweight GPUI host for the cursor-centred Actions Ring, the notices the
//! agent posts, and its transient keyboard indicators.
//!
//! This process is a pure IPC client. The agent owns HID++, session validation,
//! haptic output, and action execution; the overlay only renders the
//! agent-snapshotted actions and reports hover/activate/cancel interactions,
//! and draws whichever notices and indicators the agent says are current.

#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

// `t!` resolves against a backend the invoking crate must generate itself, so
// both binaries expand `i18n!` over the one catalog in `openlogi-ui` — the same
// crate this one already depends on for locale negotiation.
rust_i18n::i18n!("../openlogi-ui/locales", fallback = "en");

mod backlight;
mod backlight_effect;
mod chrome;
mod edge;
mod ipc;
mod lock_key;
mod notice;
mod osd;
mod platform;
mod popup;
mod ring;
mod session;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Result;
use gpui::AppContext as _;
use tracing::{debug, warn};

use openlogi_core::action_ring::DISPLAY_LIFETIME;

use crate::ipc::OverlayCommand;
use crate::notice::NoticeShelf;
use crate::platform::RingPlacement;
use crate::ring::{RingPointer, RingView};
use crate::session::{
    ClickAwaySession, claim_the_role, close_all_ring_windows, close_ring_windows,
    spawn_click_away_dismissal,
};

#[expect(
    clippy::too_many_lines,
    reason = "the overlay bootstraps its independent IPC observation tasks"
)]
fn main() -> Result<()> {
    openlogi_core::logging::init_stderr();

    openlogi_core::locale::activate(None);
    // Held for the whole run: dropping it hands the role to the replacement.
    let _tenancy = claim_the_role()?;
    let ipc::Handle {
        mut invocations,
        mut backlights,
        mut notices,
        mut lock_keys,
        mut backlight_effects,
        mut ui_chrome,
        commands,
    } = ipc::spawn();

    // GPUI 0.3.4 never releases a window's AccessKit adapter, and the adapter
    // keeps the window's view, renderer and Metal surfaces alive, so every
    // ring the overlay opened stayed in memory (#1741). The ring exposes no
    // accessibility roles, so the overlay runs without AccessKit.
    let mut app = gpui::Application::new_inaccessible(gpui_platform::current_platform(false))
        .with_assets(openlogi_ui::action_icons::ActionIcons);
    // Application construction establishes GPUI's NSApplication subclass.
    // Asking AppKit for its singleton sooner creates a plain NSApplication,
    // which lacks the `platform` ivar GPUI needs. Set Accessory policy now:
    // GPUI owns the singleton, but `app.run` has not finished launching yet.
    platform::configure_application();
    app = app.with_quit_mode(gpui::QuitMode::Explicit);
    app.run(move |cx| {
        // GPUI's didFinishLaunching callback resets the policy to Regular.
        // Reapply the helper policy after that reset, without constructing
        // NSApplication before GPUI has installed its subclass.
        platform::configure_application();
        chrome::apply(cx, openlogi_ui::material::DEFAULT_THEME_RADIUS_PX);
        let live_session = Arc::new(ClickAwaySession::new());
        spawn_click_away_dismissal(cx, Arc::clone(&live_session));
        let shelf = Rc::new(RefCell::new(NoticeShelf::default()));
        cx.spawn(async move |cx| {
            while let Some(observed) = notices.recv().await {
                openlogi_core::locale::activate(observed.language.as_deref());
                let Some(cards) = shelf.borrow_mut().observe(observed.notices) else {
                    continue;
                };
                cx.update(|cx| notice::present(cx, &shelf, &cards));
            }
        })
        .detach();
        cx.spawn(async move |cx| {
            // The agent's timer is the indicator's only clock: it reports
            // nothing showing when the time is up, or when it is gone.
            while let Some(observed) = lock_keys.recv().await {
                openlogi_core::locale::activate(observed.language.as_deref());
                cx.update(|cx| lock_key::present(cx, observed.shown));
            }
        })
        .detach();
        cx.spawn(async move |cx| {
            while let Some(observed) = backlight_effects.recv().await {
                openlogi_core::locale::activate(observed.language.as_deref());
                let shown = observed.shown;
                let supported = observed.supported;
                cx.update(|cx| backlight_effect::present(cx, shown, &supported));
            }
        })
        .detach();
        cx.spawn(async move |cx| {
            while let Some(observed) = invocations.recv().await {
                // No ring is what a dismissal looks like: close whatever is
                // showing and open nothing. The agent has already forgotten the
                // session, so there is nothing to acknowledge either.
                let Some(invocation) = observed else {
                    cx.update(close_all_ring_windows);
                    continue;
                };
                openlogi_core::locale::activate(invocation.language.as_deref());
                cx.update(|cx| {
                    close_all_ring_windows(cx);
                    #[cfg_attr(
                        windows,
                        expect(
                            unused_mut,
                            reason = "only the Wayland placement drains its openings"
                        )
                    )]
                    let mut placement = match RingPlacement::capture(cx) {
                        Ok(placement) => placement,
                        Err(error) => {
                            warn!(%error, "could not locate Actions Ring display");
                            let _ = commands.send(OverlayCommand::Cancel {
                                session_id: invocation.session_id,
                            });
                            return;
                        }
                    };
                    let commands = commands.clone();
                    let timeout_commands = commands.clone();
                    let session_id = invocation.session_id;
                    // Wayland opens one window per display; they share where
                    // the pointer was heard, so only one of them draws the ring.
                    let pointer = cx.new(|_| RingPointer::default());
                    let primary = platform::open_first(placement.openings(), |(options, panel)| {
                        let invocation = invocation.clone();
                        let commands = commands.clone();
                        let pointer = pointer.clone();
                        cx.open_window(options, |window, cx| {
                            cx.new(|cx| {
                                RingView::new(
                                    invocation,
                                    commands,
                                    &live_session,
                                    panel,
                                    pointer,
                                    window,
                                    cx,
                                )
                            })
                        })
                    });
                    let Some(primary) = primary else {
                        return;
                    };
                    if let Err(error) = primary
                        .update(cx, |_, window, cx| {
                            // Native placement resizes the window inside this
                            // update, where GPUI's own resize/move callbacks
                            // cannot borrow the app. Resync its viewport and
                            // scale, or it keeps rendering at the bootstrap size.
                            placement.show(window).map(|()| window.bounds_changed(cx))
                        })
                        .and_then(std::convert::identity)
                    {
                        warn!(%error, "could not position Actions Ring window");
                        close_ring_windows(cx, session_id);
                        let _ = timeout_commands.send(OverlayCommand::Cancel { session_id });
                        return;
                    }
                    cx.spawn(async move |cx| {
                        cx.background_executor().timer(DISPLAY_LIFETIME).await;
                        if cx.update(|cx| close_ring_windows(cx, session_id)) {
                            let _ = timeout_commands.send(OverlayCommand::Cancel { session_id });
                        }
                    })
                    .detach();
                });
            }
        })
        .detach();
        cx.spawn(async move |cx| {
            while let Some(observation) = backlights.recv().await {
                debug!(
                    level = observation.current_level,
                    visible = observation.visible,
                    off_hint = observation.off_hint,
                    "backlight observation received"
                );
                // The card translates Off and the off hint.
                openlogi_core::locale::activate(observation.language.as_deref());
                cx.update(|cx| backlight::present(cx, &observation));
            }
        })
        .detach();
        cx.spawn(async move |cx| {
            while let Some(observation) = ui_chrome.recv().await {
                cx.update(|cx| chrome::apply(cx, observation.radius_px));
            }
        })
        .detach();
    });
    Ok(())
}

#[cfg(test)]
mod tests;
