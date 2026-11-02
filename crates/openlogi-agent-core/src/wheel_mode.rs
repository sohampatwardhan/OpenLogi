//! Wheel-mode switches the overlay announces.
//!
//! A mouse's wheel mode is its SmartShift mode (`0x2111`, or the older
//! `0x2110`): free spin or ratchet. Two kinds of press switch it, and both
//! show the new mode:
//!
//! - The mouse's own wheel-mode button. The firmware switches the wheel and
//!   broadcasts a `0x2121` ratchet switch, and the inventory layer reads the
//!   SmartShift mode back ([`WheelModeIndicator::switched`]). The broadcast
//!   also fires when SmartShift disengages the ratchet on a fast spin, but
//!   the SmartShift mode does not move then, so the read-back compared with
//!   the last known mode tells the two apart.
//! - A button bound to Toggle SmartShift, which the agent writes itself
//!   ([`WheelModeIndicator::toggled`]).
//!
//! Everything else is a baseline that only updates the last known mode: the
//! read when a mouse connects ([`WheelModeIndicator::connected`]), and a mode
//! the settings app or a settings replay writes
//! ([`WheelModeIndicator::written`]). The settings app writes the mode before
//! the mouse's broadcast for it can arrive, so the broadcast then repeats the
//! known mode and shows nothing.
//!
//! The firmware readings name the mouse by the HID node they arrived on and
//! its HID++ device index ([`WheelNode`]); the agent's writes name it by
//! route. The orchestrator publishes which node each route reaches with every
//! inventory ([`WheelModeIndicator::set_routes`]).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use openlogi_core::config::AppSettings;
use openlogi_core::hid::{DeviceRoute, SmartShiftMode};
use openlogi_ipc::{Generation, OBSERVE_HOLD, WheelModeObservation};
use tokio::sync::watch;

use crate::backlight_overlay::DISPLAY_LIFETIME;

/// The HID node a mouse's broadcasts arrive on, and its device index there:
/// a receiver and slot, or a directly attached mouse at the self index.
///
/// Two identical receivers on one host share a node, as they do for the
/// battery broadcasts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WheelNode {
    /// Vendor id of the HID node.
    pub vendor_id: u16,
    /// Product id of the HID node.
    pub product_id: u16,
    /// HID++ device index: a receiver slot, or `0xff` for a direct mouse.
    pub device_index: u8,
}

impl WheelNode {
    /// The node a direct `route` reaches, which the route itself names. A
    /// receiver route needs the inventory's receiver, so it is `None` here.
    fn of_direct(route: &DeviceRoute) -> Option<Self> {
        match route {
            DeviceRoute::Direct {
                vendor_id,
                product_id,
            } => Some(Self {
                vendor_id: *vendor_id,
                product_id: *product_id,
                device_index: route.device_index(),
            }),
            _ => None,
        }
    }
}

/// The last known wheel mode of each mouse, and which presses count as a
/// switch. No I/O: the indicator publishes what this decides.
#[derive(Default)]
struct WheelModes {
    modes: HashMap<WheelNode, SmartShiftMode>,
    routes: HashMap<DeviceRoute, WheelNode>,
}

impl WheelModes {
    fn node(&self, route: &DeviceRoute) -> Option<WheelNode> {
        self.routes
            .get(route)
            .copied()
            .or_else(|| WheelNode::of_direct(route))
    }

    /// A baseline: remember `mode`, show nothing.
    fn remember(&mut self, node: WheelNode, mode: SmartShiftMode) {
        self.modes.insert(node, mode);
    }

    /// The mode a broadcast read back, to show when it moved from a known
    /// one. With none known it is a baseline: nothing says it changed.
    fn switched(&mut self, node: WheelNode, mode: SmartShiftMode) -> Option<SmartShiftMode> {
        let previous = self.modes.insert(node, mode)?;
        (previous != mode).then_some(mode)
    }

    /// The mode a bound Toggle SmartShift wrote. Always shown: the user
    /// pressed for it.
    fn toggled(&mut self, route: &DeviceRoute, mode: SmartShiftMode) -> SmartShiftMode {
        if let Some(node) = self.node(route) {
            self.modes.insert(node, mode);
        }
        mode
    }

    fn written(&mut self, route: &DeviceRoute, mode: SmartShiftMode) {
        if let Some(node) = self.node(route) {
            self.modes.insert(node, mode);
        }
    }

    fn forget(&mut self, route: &DeviceRoute) {
        if let Some(node) = self.node(route) {
            self.modes.remove(&node);
        }
    }
}

struct Cell {
    modes: WheelModes,
    enabled: bool,
    language: Option<String>,
}

/// The wheel-mode popup the overlay shows, and the last known modes that
/// decide when.
#[derive(Clone)]
pub struct WheelModeIndicator {
    cell: Arc<Mutex<Cell>>,
    published: watch::Sender<WheelModeObservation>,
}

impl Default for WheelModeIndicator {
    fn default() -> Self {
        // Generation 1, not 0: 0 is the client sentinel for "I have seen
        // nothing", so a first `observe(0)` answers at once.
        let (published, _) = watch::channel(WheelModeObservation {
            generation: 1,
            shown: None,
            language: None,
        });
        Self {
            cell: Arc::new(Mutex::new(Cell {
                modes: WheelModes::default(),
                enabled: true,
                language: None,
            })),
            published,
        }
    }
}

impl WheelModeIndicator {
    /// Take the user's switch for this popup and the UI language. The modes
    /// still advance while it is off, so turning it on replays nothing.
    pub fn configure(&self, settings: &AppSettings) {
        let mut cell = self.cell();
        cell.enabled = settings.wheel_mode_notifications;
        cell.language.clone_from(&settings.language);
    }

    /// Replace which node each route reaches, from the latest inventory.
    pub fn set_routes(&self, routes: impl IntoIterator<Item = (DeviceRoute, WheelNode)>) {
        self.cell().modes.routes = routes.into_iter().collect();
    }

    /// The mode `node` reported when it connected: a baseline.
    pub fn connected(&self, node: WheelNode, mode: SmartShiftMode) {
        self.cell().modes.remember(node, mode);
    }

    /// The mode `node` reported after its wheel switched. Shown when it moved
    /// from the last known mode. Returns whether it is shown.
    #[expect(
        clippy::must_use_candidate,
        reason = "showing the popup is the effect; the flag only reports it"
    )]
    pub fn switched(&self, node: WheelNode, mode: SmartShiftMode) -> bool {
        let shown = self.cell().modes.switched(node, mode);
        self.publish(shown)
    }

    /// The mode a bound Toggle SmartShift wrote to `route`. Shown. Returns
    /// whether it is shown.
    #[expect(
        clippy::must_use_candidate,
        reason = "showing the popup is the effect; the flag only reports it"
    )]
    pub fn toggled(&self, route: &DeviceRoute, mode: SmartShiftMode) -> bool {
        let shown = self.cell().modes.toggled(route, mode);
        self.publish(Some(shown))
    }

    /// The settings app or a settings replay is writing `mode` to `route`: a
    /// baseline, recorded before the write so the mouse's broadcast for it
    /// shows nothing.
    pub fn written(&self, route: &DeviceRoute, mode: SmartShiftMode) {
        self.cell().modes.written(route, mode);
    }

    /// A write to `route` failed, so its mode is unknown: the next reading is
    /// a baseline.
    pub fn write_failed(&self, route: &DeviceRoute) {
        self.cell().modes.forget(route);
    }

    /// Long-poll the popup.
    pub async fn observe(&self, since: Generation) -> WheelModeObservation {
        let mut receiver = self.published.subscribe();
        let changed = receiver.wait_for(|observation| observation.generation != since);
        match tokio::time::timeout(OBSERVE_HOLD, changed).await {
            Ok(Ok(observation)) => observation.clone(),
            Ok(Err(_)) | Err(_) => self.published.borrow().clone(),
        }
    }

    /// Show `mode` when there is one and the user wants the popup.
    fn publish(&self, mode: Option<SmartShiftMode>) -> bool {
        let (enabled, language) = {
            let cell = self.cell();
            (cell.enabled, cell.language.clone())
        };
        let Some(mode) = mode.filter(|_| enabled) else {
            return false;
        };
        let mut generation = 0;
        self.published.send_modify(|observation| {
            observation.generation = observation.generation.wrapping_add(1);
            observation.shown = Some(mode);
            observation.language = language;
            generation = observation.generation;
        });
        if tokio::runtime::Handle::try_current().is_err() {
            return true;
        }
        let published = self.published.clone();
        tokio::spawn(async move {
            tokio::time::sleep(DISPLAY_LIFETIME).await;
            published.send_if_modified(|observation| {
                if observation.generation != generation {
                    return false;
                }
                observation.generation = observation.generation.wrapping_add(1);
                observation.shown = None;
                true
            });
        });
        true
    }

    fn cell(&self) -> MutexGuard<'_, Cell> {
        self.cell.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests;
