//! Bottom navigation bar.

use eframe::egui;

use crate::app::Tab;
use crate::download_tracker::DownloadTracker;

/// Renders the bottom navigation bar.
pub fn render(ui: &mut egui::Ui, current_tab: &mut Tab, downloads: &DownloadTracker) {
    ui.horizontal(|ui| {
        ui.heading("MultiVC");
        ui.selectable_value(current_tab, Tab::Cores, "\u{2699} Cores");
        ui.selectable_value(current_tab, Tab::Instances, "\u{1F4E6} Instances");

        // Spacer — push download indicator to the right
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if downloads.has_active() {
                ui.label(format!("\u{2B07} {}", downloads.active_count()));
                ui.spinner();
            }
        });
    });
}
