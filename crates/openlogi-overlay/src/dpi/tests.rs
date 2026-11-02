use openlogi_core::hid::Dpi;

use super::*;

fn preset(dpi: u16, presets: &[u16]) -> DpiPopup {
    DpiPopup::Preset {
        dpi: Dpi::new(dpi),
        presets: presets.iter().copied().map(Dpi::new).collect(),
    }
}

#[test]
fn a_preset_lists_the_presets_with_the_current_one_marked() {
    let row = row(&preset(1600, &[800, 1600, 3200, 6400]));
    assert_eq!(row.label, "DPI");
    assert_eq!(row.options, ["800", "1600", "3200", "6400"]);
    assert_eq!(row.current, 1);
    assert_eq!(row.icon, DPI_ICON);
}

#[test]
fn unknown_presets_name_the_dpi_alone() {
    for popup in [preset(1000, &[]), preset(1000, &[800, 1600])] {
        let row = row(&popup);
        assert_eq!(row.options, ["1000"], "{popup:?}");
        assert_eq!(row.current, 0);
    }
}

#[test]
fn many_presets_show_a_window_around_the_current_one() {
    let presets: Vec<u16> = (1..=10).map(|step| step * 400).collect();
    let row = row(&preset(3200, &presets));
    assert_eq!(row.options.len(), MAX_PRESETS);
    assert_eq!(row.options[row.current], "3200");
}

#[test]
fn precision_lists_on_and_off_with_the_new_state_marked() {
    let on = row(&DpiPopup::Precision { engaged: true });
    assert_eq!(on.label, "Precision Mode");
    assert_eq!(on.options, ["On", "Off"]);
    assert_eq!(on.current, 0);
    assert_eq!(on.icon, PRECISION_ICON);
    let off = row(&DpiPopup::Precision { engaged: false });
    assert_eq!(off.current, 1);
}

/// The popup each open indicator window shows.
fn shown(cx: &mut gpui::TestAppContext) -> Vec<DpiPopup> {
    cx.update(|cx| {
        cx.windows()
            .iter()
            .filter_map(AnyWindowHandle::downcast::<DpiView>)
            .filter_map(|window| window.read(cx).ok().map(|view| view.popup.clone()))
            .collect()
    })
}

#[gpui::test]
fn one_window_shows_each_press_until_nothing_shows(cx: &mut gpui::TestAppContext) {
    let first = preset(800, &[800, 1600, 3200]);
    let next = preset(1600, &[800, 1600, 3200]);
    let precision = DpiPopup::Precision { engaged: true };

    cx.update(|cx| present(cx, None));
    assert!(shown(cx).is_empty(), "nothing showing opens nothing");

    cx.update(|cx| present(cx, Some(first.clone())));
    assert_eq!(shown(cx), [first]);

    cx.update(|cx| present(cx, Some(next.clone())));
    cx.run_until_parked();
    assert_eq!(shown(cx), [next], "the next preset replaces the card");

    cx.update(|cx| present(cx, Some(precision.clone())));
    cx.run_until_parked();
    assert_eq!(shown(cx), [precision], "one DPI card at a time");

    cx.update(|cx| present(cx, None));
    cx.run_until_parked();
    assert!(shown(cx).is_empty(), "nothing showing closes the window");
}
