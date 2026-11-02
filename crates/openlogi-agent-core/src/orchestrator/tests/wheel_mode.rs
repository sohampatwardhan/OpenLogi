//! Wheel-mode popups from the orchestrator's side: the route table a write
//! is resolved with, and the settings it hands the popups.

use std::sync::Arc;

use openlogi_core::config::Config;
use openlogi_core::hid::SmartShiftMode;
use openlogi_hid::{DIRECT_DEVICE_INDEX, DeviceRoute};

use super::direct_inventory;
use crate::observable::ObservableState;
use crate::orchestrator::{Orchestrator, wheel_routes};
use crate::wheel_mode::WheelNode;

fn bolt_inventory() -> openlogi_core::device::DeviceInventory {
    let mut inventory = direct_inventory(Some("ABC123"), [1, 2, 3, 4]);
    inventory.receiver.product_id = 0xc548;
    inventory.receiver.unique_id = Some("BOLT1".to_owned());
    inventory.paired[0].slot = 2;
    inventory
}

#[test]
fn each_paired_route_reaches_its_receiver_node_and_slot() {
    let routes = wheel_routes(&[bolt_inventory(), direct_inventory(None, [0; 4])]);
    assert_eq!(
        routes,
        [
            (
                DeviceRoute::Bolt {
                    receiver_uid: "BOLT1".to_owned(),
                    slot: 2,
                },
                WheelNode {
                    vendor_id: 0x046d,
                    product_id: 0xc548,
                    device_index: 2,
                },
            ),
            (
                DeviceRoute::Direct {
                    vendor_id: 0x046d,
                    product_id: 0xb023,
                },
                WheelNode {
                    vendor_id: 0x046d,
                    product_id: 0xb023,
                    device_index: DIRECT_DEVICE_INDEX,
                },
            ),
        ]
    );
}

/// The settings app writes by route; the mouse's broadcast names its node.
/// The route table from the latest inventory joins them, so the broadcast
/// for the app's own write shows nothing.
#[tokio::test(start_paused = true)]
async fn a_settings_app_write_is_silent_once_the_inventory_names_its_node() {
    let observable = Arc::new(ObservableState::new("test".to_owned()));
    let mut orch = Orchestrator::new(Config::default(), Arc::clone(&observable));
    orch.refresh_inventory(&[bolt_inventory()], &[], false);
    let node = WheelNode {
        vendor_id: 0x046d,
        product_id: 0xc548,
        device_index: 2,
    };
    let route = DeviceRoute::Bolt {
        receiver_uid: "BOLT1".to_owned(),
        slot: 2,
    };
    let wheel_mode = observable.wheel_mode();
    wheel_mode.connected(node, SmartShiftMode::Ratchet);
    wheel_mode.written(&route, SmartShiftMode::Free);
    assert!(!wheel_mode.switched(node, SmartShiftMode::Free));
    assert!(wheel_mode.switched(node, SmartShiftMode::Ratchet));
}

#[tokio::test(start_paused = true)]
async fn a_reload_hands_the_popups_the_new_switches() {
    let observable = Arc::new(ObservableState::new("test".to_owned()));
    let mut config = Config::default();
    config.app_settings.dpi_notifications = false;
    config.app_settings.wheel_mode_notifications = false;
    let mut orch = Orchestrator::new(config.clone(), Arc::clone(&observable));
    let popup = openlogi_ipc::DpiPopup::Precision { engaged: true };
    assert!(
        !observable.dpi().show(popup.clone()),
        "off from the loaded config"
    );
    assert!(!observable.wheel_mode().toggled(
        &DeviceRoute::Direct {
            vendor_id: 0x046d,
            product_id: 0xb023,
        },
        SmartShiftMode::Free,
    ));

    config.app_settings.dpi_notifications = true;
    config.app_settings.wheel_mode_notifications = true;
    orch.reload_config(config);
    assert!(observable.dpi().show(popup), "on after the reload");
    assert!(observable.wheel_mode().toggled(
        &DeviceRoute::Direct {
            vendor_id: 0x046d,
            product_id: 0xb023,
        },
        SmartShiftMode::Ratchet,
    ));
}
