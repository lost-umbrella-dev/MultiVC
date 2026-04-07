//! Row helper widgets for the Cores tab.

use eframe::egui;

use crate::ui::icons;
use crate::ui::lang::{self, Lang};
use crate::ui::widgets::{ProgressRing, icon_button, striped_frame};

// ── Installed row ─────────────────────────────────────────────────────

#[derive(Default)]
pub(super) struct RowActions {
    pub(super) delete: bool,
    pub(super) create_instance: bool,
    pub(super) redownload: bool,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn installed_core_row(
    ui: &mut egui::Ui,
    name: &str,
    version: &str,
    hash_short: &str,
    hash_full: &str,
    dependents: &[String],
    is_invalid: bool,
    is_downloading: bool,
    download_fraction: Option<f32>,
    striped_bg: bool,
    lang: Lang,
) -> RowActions {
    let mut actions = RowActions::default();
    let has_dependents = !dependents.is_empty();

    let frame = striped_frame(striped_bg, ui);

    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let actions_width = 60.0;
            let version_width = 80.0;
            let hash_width = 160.0;
            let name_width = (ui.available_width()
                - version_width
                - hash_width
                - actions_width
                - ui.spacing().item_spacing.x * 4.0)
                .max(80.0);

            ui.add_sized([name_width, ui.available_height()], egui::Label::new(name).truncate());
            ui.add_sized([version_width, ui.available_height()], egui::Label::new(version));
            ui.add_sized([hash_width, ui.available_height()], egui::Label::new(hash_short))
                .on_hover_text(hash_full);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // First button slot: create instance OR redownload/progress
                if is_downloading {
                    ui.add(ProgressRing::new(download_fraction));
                } else if is_invalid {
                    if ui
                        .add(icon_button(
                            egui::RichText::new(icons::ICON_DOWNLOAD)
                                .color(egui::Color32::from_rgb(255, 165, 0)),
                        ))
                        .on_hover_text(lang::t("tip.redownload", lang))
                        .clicked()
                    {
                        actions.redownload = true;
                    }
                } else if ui
                    .add(icon_button("+"))
                    .on_hover_text(lang::t("tip.create_instance", lang))
                    .clicked()
                {
                    actions.create_instance = true;
                }

                // Second button slot: delete (always shown)
                if has_dependents {
                    let tooltip = format!("Used by: [ {} ]", dependents.join(", "));
                    let btn = icon_button(
                        egui::RichText::new(icons::ICON_DELETE).color(egui::Color32::YELLOW),
                    );
                    ui.add_enabled(false, btn).on_disabled_hover_text(tooltip);
                } else if ui
                    .add(icon_button(icons::ICON_DELETE))
                    .on_hover_text(lang::t("tip.delete", lang))
                    .clicked()
                {
                    actions.delete = true;
                }
            });
        });
    });

    actions
}

// ── Available row ─────────────────────────────────────────────────────

#[derive(PartialEq)]
pub(super) enum AvailableRowAction {
    None,
    Select,
    Deselect,
}

#[allow(clippy::too_many_arguments)]
/// Renders a row in the available versions list.
pub(super) fn available_core_row(
    ui: &mut egui::Ui,
    name: &str,
    version: &str,
    size_text: &str,
    is_downloading: bool,
    download_fraction: Option<f32>,
    is_installed: bool,
    is_selected: bool,
    striped_bg: bool,
    lang: Lang,
) -> AvailableRowAction {
    let mut action = AvailableRowAction::None;

    let frame = striped_frame(striped_bg, ui);

    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let status_width = 30.0;
            let size_width = 80.0;
            let version_width = 80.0;
            let name_width = (ui.available_width()
                - version_width
                - size_width
                - status_width
                - ui.spacing().item_spacing.x * 4.0)
                .max(80.0);

            ui.add_sized([name_width, ui.available_height()], egui::Label::new(name).truncate());
            ui.add_sized([version_width, ui.available_height()], egui::Label::new(version));
            ui.add_sized([size_width, ui.available_height()], egui::Label::new(size_text));

            if is_downloading {
                ui.add_sized(
                    [status_width, ui.available_height()],
                    ProgressRing::new(download_fraction),
                );
            } else if is_installed {
                ui.add_sized(
                    [status_width, ui.available_height()],
                    egui::Label::new(
                        egui::RichText::new(icons::ICON_VALIDATE)
                            .color(egui::Color32::GREEN)
                            .strong(),
                    ),
                )
                .on_hover_text(lang::t("tip.installed", lang));
            } else {
                let mut selected = is_selected;
                let cb = ui.add_sized(
                    [status_width, ui.available_height()],
                    egui::Checkbox::without_text(&mut selected),
                );
                if cb.clicked() {
                    action = if selected {
                        AvailableRowAction::Select
                    } else {
                        AvailableRowAction::Deselect
                    };
                }
            }
        });
    });

    action
}
