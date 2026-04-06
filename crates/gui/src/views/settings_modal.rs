//! Settings modal window for the launcher.

use eframe::egui;
use eframe::egui::Align2;

use crate::state::SettingsState;

/// Renders the settings modal window.
///
/// Opens when `state.open` is true. Displays build info, language selector, and folder buttons.
pub fn render(
    ui: &mut egui::Ui,
    state: &mut SettingsState,
    cores_count: usize,
    instances_count: usize,
    toasts: &mut egui_toast::Toasts,
) {
    if !state.open {
        return;
    }

    let lang = state.lock.language;
    let mut open = true;

    egui::Window::new(crate::lang::t("settings.title", lang))
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .open(&mut open)
        .show(ui.ctx(), |ui| {
            // ── Build info ──────────────────────────────────────
            ui.heading(crate::lang::t("settings.build_info", lang));
            ui.label(format!(
                "{}: {}",
                crate::lang::t("settings.version", lang),
                env!("CARGO_PKG_VERSION")
            ));
            ui.label(format!("Build: {}", env!("BUILD_TIMESTAMP")));

            ui.add_space(4.0);

            // Cores count + folder button on same row
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{}: {}",
                    crate::lang::t("settings.cores_count", lang),
                    cores_count,
                ));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    crate::widgets::open_folder_button(
                        ui,
                        "cores",
                        crate::lang::t("settings.open_cores", lang),
                        toasts,
                    );
                });
            });

            // Instances count + folder button on same row
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{}: {}",
                    crate::lang::t("settings.instances_count", lang),
                    instances_count,
                ));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    crate::widgets::open_folder_button(
                        ui,
                        "instances",
                        crate::lang::t("settings.open_instances", lang),
                        toasts,
                    );
                });
            });

            // Launcher root folder
            ui.horizontal(|ui| {
                ui.label(crate::lang::t("settings.open_root", lang));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    crate::widgets::open_folder_button(ui, ".", crate::lang::t("settings.open_root", lang), toasts);
                });
            });

            ui.separator();

            // ── Language selector ────────────────────────────────
            ui.heading(crate::lang::t("settings.language", lang));
            ui.horizontal(|ui| {
                ui.selectable_value(&mut state.lock.language, crate::lang::Lang::En, "EN");
                ui.selectable_value(&mut state.lock.language, crate::lang::Lang::Ru, "RU");
            });
        });

    if !open {
        state.open = false;
    }
}
