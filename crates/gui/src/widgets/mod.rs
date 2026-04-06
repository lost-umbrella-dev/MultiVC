mod progress_ring;

pub use progress_ring::ProgressRing;

use eframe::egui;

/// Uniform min size for icon-only buttons (e.g. "\u{1F5D1}", "+", "\u{25B6}").
const ICON_BUTTON_SIZE: f32 = 24.0;

/// Creates a uniformly-sized icon button.
pub fn icon_button(icon: impl Into<egui::WidgetText>) -> egui::Button<'static> {
    egui::Button::new(icon).min_size(egui::vec2(ICON_BUTTON_SIZE, ICON_BUTTON_SIZE))
}
