//! Log tab for the instance panel.

use eframe::egui;
use egui_toast::Toasts;

use crate::ui::icons;
use crate::ui::lang::{self, Lang};
use crate::ui::state::InstancesTabState;
use crate::ui::widgets::open_folder;

pub(super) fn render(
    ui: &mut egui::Ui,
    state: &mut InstancesTabState,
    name: &str,
    is_running: bool,
    pid: Option<u32>,
    toasts_out: &mut Toasts,
    lang: Lang,
) {
    let log_path = std::path::Path::new("instances").join(name).join("latest.log");

    // Header: status + open file button + log filters
    ui.horizontal(|ui| {
        if is_running {
            ui.spinner();
            if let Some(pid) = pid {
                ui.colored_label(
                    egui::Color32::GREEN,
                    format!("{} (PID {})", lang::t("status.running", lang), pid),
                );
            } else {
                ui.colored_label(egui::Color32::GREEN, lang::t("status.running", lang));
            }
            ui.separator();
        }

        if ui
            .add_enabled(!is_running, egui::Button::new(icons::LABEL_OPEN_FILE))
            .on_hover_text(if is_running {
                lang::t("tip.stop_first", lang)
            } else {
                lang::t("tip.open_editor", lang)
            })
            .clicked()
        {
            open_folder(&log_path, toasts_out);
        }

        ui.separator();

        // Log level filters
        let panel = state.instance_panel.as_mut().unwrap();
        ui.checkbox(&mut panel.log_filter_info, lang::t("log.filter_info", lang));
        ui.checkbox(&mut panel.log_filter_warn, lang::t("log.filter_warn", lang));
        ui.checkbox(&mut panel.log_filter_error, lang::t("log.filter_error", lang));
    });

    ui.separator();

    let log_content = if log_path.exists() {
        std::fs::read_to_string(&log_path).unwrap_or_else(|e| format!("Error reading log: {e}"))
    } else {
        lang::t("status.no_log", lang).to_owned()
    };

    let panel = state.instance_panel.as_ref().unwrap();

    egui::ScrollArea::both()
        .id_salt("log_viewer_scroll")
        .auto_shrink(false)
        .stick_to_bottom(true)
        .show(ui, |ui| {
            for line in log_content.lines() {
                let is_error = line.starts_with("[E]");
                let is_warn = line.starts_with("[W]");
                let is_info = line.starts_with("[I]");

                // Apply filter
                if is_error && !panel.log_filter_error {
                    continue;
                }
                if is_warn && !panel.log_filter_warn {
                    continue;
                }
                if is_info && !panel.log_filter_info {
                    continue;
                }

                let color = if is_error {
                    egui::Color32::RED
                } else if is_warn {
                    egui::Color32::YELLOW
                } else {
                    ui.visuals().text_color()
                };
                ui.label(egui::RichText::new(line).monospace().color(color));
            }
        });

    // Auto-refresh while running
    if is_running {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
    }
}
