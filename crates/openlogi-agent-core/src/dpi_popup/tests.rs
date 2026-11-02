//! The DPI popup: presets, precision mode, and the user's switch.

use openlogi_core::config::AppSettings;
use openlogi_core::hid::Dpi;
use openlogi_ipc::DpiPopup;

use super::DpiIndicator;
use crate::DpiCycles;
use crate::backlight_overlay::DISPLAY_LIFETIME;

#[test]
fn the_popup_lists_the_presets_of_the_pressed_or_selected_mouse() {
    let mut cycles = DpiCycles::default();
    let presets = vec![Dpi::new(800), Dpi::new(1600), Dpi::new(3200)];
    cycles.by_key.insert(
        "mouse".to_owned(),
        crate::DpiCycleState {
            presets: presets.clone(),
            ..crate::DpiCycleState::default()
        },
    );
    assert_eq!(cycles.presets_of(Some("mouse")), presets);
    assert!(cycles.presets_of(Some("other")).is_empty());
    assert!(cycles.presets_of(None).is_empty(), "nothing selected");
    cycles.selected = Some("mouse".to_owned());
    assert_eq!(cycles.presets_of(None), presets, "the OS hook's selection");
}

#[tokio::test(start_paused = true)]
async fn each_press_replaces_the_popup_until_its_time_is_up() {
    let indicator = DpiIndicator::default();
    indicator.configure(&AppSettings {
        language: Some("fr".to_owned()),
        ..AppSettings::default()
    });
    let preset = DpiPopup::Preset {
        dpi: Dpi::new(1600),
        presets: vec![Dpi::new(800), Dpi::new(1600), Dpi::new(3200)],
    };
    assert!(indicator.show(preset.clone()));
    let shown = indicator.observe(0).await;
    assert_eq!(shown.shown, Some(preset));
    assert_eq!(shown.language.as_deref(), Some("fr"));

    let precision = DpiPopup::Precision { engaged: true };
    assert!(indicator.show(precision.clone()));
    let replaced = indicator.observe(shown.generation).await;
    assert_eq!(replaced.shown, Some(precision.clone()));

    tokio::task::yield_now().await;
    tokio::time::advance(DISPLAY_LIFETIME / 2).await;
    tokio::task::yield_now().await;
    assert_eq!(
        indicator.observe(0).await.shown,
        Some(precision),
        "the first popup's timer does not end the one that replaced it"
    );
    tokio::time::advance(DISPLAY_LIFETIME).await;
    let ended = indicator.observe(replaced.generation).await;
    assert_eq!(ended.shown, None);
}

#[tokio::test(start_paused = true)]
async fn the_popup_stays_off_when_turned_off() {
    let indicator = DpiIndicator::default();
    indicator.configure(&AppSettings {
        dpi_notifications: false,
        ..AppSettings::default()
    });
    let start = indicator.observe(0).await;
    assert!(!indicator.show(DpiPopup::Precision { engaged: false }));
    let still = tokio::time::timeout(DISPLAY_LIFETIME, indicator.observe(start.generation))
        .await
        .unwrap_or_else(|_| start.clone());
    assert_eq!(still.shown, None);

    indicator.configure(&AppSettings::default());
    assert!(indicator.show(DpiPopup::Precision { engaged: false }));
}
