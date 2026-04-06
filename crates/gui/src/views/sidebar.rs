//! Bottom navigation bar.

use eframe::egui;

use crate::app::Tab;
use crate::download_tracker::DownloadTracker;
use crate::icons;
use crate::lang;

/// Renders the bottom navigation bar.
pub fn render(
    ui: &mut egui::Ui,
    current_tab: &mut Tab,
    downloads: &DownloadTracker,
    settings_state: &mut crate::state::SettingsState,
) {
    let lang = settings_state.lock.language;

    ui.horizontal(|ui| {
        ui.heading("MultiVC");
        ui.separator();

        let cores_label = format!("{} {}", icons::ICON_TAB_CORES, lang::t("tab.cores", lang));
        let instances_label = format!("{} {}", icons::ICON_TAB_INSTANCES, lang::t("tab.instances", lang));

        ui.selectable_value(current_tab, Tab::Cores, cores_label);
        ui.selectable_value(current_tab, Tab::Instances, instances_label);

        // Spacer — push download indicator to the right
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Settings button
            if ui
                .add(crate::widgets::icon_button(icons::ICON_SETTINGS))
                .on_hover_text(lang::t("tip.settings", lang))
                .clicked()
            {
                settings_state.open = true;
            }
            ui.separator();
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
                ui.label(format!("{} {}", icons::ICON_DOWNLOAD, downloads.active_count()));
                ui.spinner();
            }
        });
    });
}
