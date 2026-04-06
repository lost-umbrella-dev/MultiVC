//! Bottom navigation bar.

use eframe::egui;

use crate::app::Tab;
use crate::download_tracker::DownloadTracker;
use crate::icons;

/// Renders the bottom navigation bar.
pub fn render(ui: &mut egui::Ui, current_tab: &mut Tab, downloads: &DownloadTracker) {
    ui.horizontal(|ui| {
        ui.heading("MultiVC");
        ui.separator();
        ui.selectable_value(current_tab, Tab::Cores, "\u{2699} Cores");
        ui.selectable_value(current_tab, Tab::Instances, "\u{1F4E6} Instances");

        // Spacer — push download indicator to the right
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // GitHub icon button
            let github_image = egui::Image::from_bytes("bytes://github.svg", icons::GITHUB.as_bytes())
                .fit_to_exact_size(egui::vec2(22.0, 22.0))
                .tint(egui::Color32::WHITE);
            let github_button = egui::Button::image(github_image).frame(false);
            if ui
                .add(github_button)
                .on_hover_text("GitHub")
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                let _ = open::that("https://github.com/lost-umbrella-dev/MultiVC");
            }
            // LOUM icon button
            let loum_image = egui::Image::from_bytes("bytes://loum.svg", icons::LOUM.as_bytes())
                .fit_to_exact_size(egui::vec2(24.0, 24.0))
                .tint(egui::Color32::WHITE);
            let loum_button = egui::Button::image(loum_image).frame(false);
            if ui
                .add(loum_button)
                .on_hover_text("LOUM")
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                let _ = open::that("https://github.com/lost-umbrella-dev");
            }
            ui.separator();
            if downloads.has_active() {
                ui.label(format!("\u{2B07} {}", downloads.active_count()));
                ui.spinner();
            }
        });
    });
}
