//! Compact transient keyboard-backlight indicator.

use gpui::{
    App, AppContext as _, Context, IntoElement, ParentElement, Render, Styled, Window, div, px,
    size,
};
use openlogi_core::hid::backlight::max_backlight_level;
use openlogi_ipc::BacklightObservation;

pub(crate) struct BacklightView {
    pub(crate) observation: BacklightObservation,
}

impl BacklightView {
    pub(crate) fn new(observation: BacklightObservation) -> Self {
        Self { observation }
    }
}

impl Render for BacklightView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let max_level = max_backlight_level(self.observation.levels);
        // The settings app's own "Backlight" label, already translated in
        // every catalog; the level and its maximum are plain numbers.
        let level = format!(
            "{} {} / {}",
            rust_i18n::t!("device.backlight"),
            self.observation.current_level,
            max_level
        );
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .w(px(280.0))
            .h(px(88.0))
            .rounded(px(12.0))
            .bg(gpui::hsla(0.0, 0.0, 0.08, 0.94))
            .text_color(gpui::hsla(0.0, 0.0, 0.96, 1.0))
            .child(level)
            .child(
                div()
                    .w(px(220.0))
                    .h(px(8.0))
                    .rounded(px(4.0))
                    .bg(gpui::hsla(0.0, 0.0, 0.25, 1.0))
                    .child(
                        div()
                            .h_full()
                            .w(px(220.0
                                * relative_level(
                                    self.observation.current_level,
                                    max_level,
                                )))
                            .rounded(px(4.0))
                            .bg(gpui::hsla(0.58, 0.75, 0.62, 1.0)),
                    ),
            )
    }
}

fn relative_level(level: u8, max_level: u8) -> f32 {
    if max_level == 0 {
        0.0
    } else {
        f32::from(level.min(max_level)) / f32::from(max_level)
    }
}

/// Open the indicator on the platform edge of the primary display.
pub(crate) fn open(
    cx: &mut App,
    observation: &BacklightObservation,
) -> anyhow::Result<gpui::WindowHandle<BacklightView>> {
    let observation = observation.clone();
    crate::popup::open(
        cx,
        "openlogi-backlight-osd",
        size(px(280.0), px(88.0)),
        move |_, cx| cx.new(|_| BacklightView::new(observation.clone())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highest_reported_level_fills_the_bar() {
        assert_eq!(relative_level(7, max_backlight_level(8)), 1.0);
        assert_eq!(relative_level(0, max_backlight_level(8)), 0.0);
    }
}
