//! Info tab for the instance panel.

use eframe::egui;

use std::path::Path;

use clients::hash::Hash;
use composer::item::LockItem;
use composer::message::Command;
use composer::worker::WorkerHandle;

use crate::ui::icons;
use crate::ui::lang::{self, Lang};
use crate::ui::state::InstancesTabState;
use crate::ui::widgets::{icon_button, image_from_base64};

#[allow(clippy::too_many_arguments)]
pub(super) fn render(
    ui: &mut egui::Ui,
    state: &mut InstancesTabState,
    handle: &WorkerHandle,
    installed_cores: &[(Hash, LockItem)],
    name: &str,
    is_running: bool,
    pid: Option<u32>,
    instances_dir: &Path,
    lang: Lang,
) {
    let meta = state.installed.iter().find(|(n, _)| n == name).map(|(_, m)| m);
    let Some(meta) = meta else {
        ui.label("Instance not found");
        return;
    };
    let meta = meta.clone();

    let icon_size = 48.0;
    let banner_height = 120.0;

    // Icon + name (bottom-aligned) | Banner (right)
    ui.horizontal(|ui| {
        // Left side: icon + name
        image_from_base64(
            ui,
            &format!("instance/{name}/icon"),
            &meta.icon,
            egui::vec2(icon_size, icon_size),
            icons::DEFAULT_ICON_SVG,
        );
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Max), |ui| {
            ui.vertical(|ui| {
                ui.heading(name);
                if is_running && let Some(pid) = pid {
                    ui.colored_label(
                        egui::Color32::GREEN,
                        format!("{} (PID {})", lang::t("status.running", lang), pid),
                    );
                }
            });
        });

        // Right side: banner
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let banner_width = ui.available_width().min(600.0);
            image_from_base64(
                ui,
                &format!("instance/{name}/banner"),
                &meta.banner,
                egui::vec2(banner_width, banner_height),
                icons::DEFAULT_BANNER_SVG,
            );
        });
    });

    ui.separator();

    // Description
    let description = state
        .viewing
        .as_ref()
        .and_then(|inst| inst.description.as_deref())
        .unwrap_or(lang::t("info.no_description", lang));
    ui.horizontal(|ui| {
        ui.strong(format!("{}:", lang::t("info.description", lang)));
        ui.label(description);
    });

    // Core version
    if let Some(inst) = &state.viewing {
        let (core_label, hash) = installed_cores
            .iter()
            .find(|(h, _)| *h == inst.core_version)
            .map(|(h, li)| (li.item.version.to_string(), h.to_string()))
            .unwrap_or_else(|| {
                (
                    inst.core_version.to_string(),
                    lang::t("info.core_hash_not_found", lang).to_owned(),
                )
            });

        ui.horizontal(|ui| {
            ui.strong(format!("{}:", lang::t("info.core_version", lang)));
            ui.label(core_label);
        });

        ui.horizontal(|ui| {
            ui.strong(format!("{}:", lang::t("info.core_hash", lang)));
            ui.label(hash);
        });
    }

    // Created at
    if let Some(created) = meta.created_at {
        ui.horizontal(|ui| {
            ui.strong(format!("{}:", lang::t("info.created_at", lang)));
            ui.label(created.format("%Y-%m-%d %H:%M").to_string());
        });
    }

    // Disk size
    ui.horizontal(|ui| {
        ui.strong(format!("{}:", lang::t("info.disk_size", lang)));
        let panel = state.instance_panel.as_ref().unwrap();
        match panel.dir_size {
            Some(bytes) => ui.label(crate::ui::format_size(bytes)),
            None => {
                ui.label(lang::t("info.calculating", lang));
                // Request dir size calculation
                handle.try_send(Command::GetInstanceDirSize {
                    name: name.to_owned(),
                });
                ui.spinner()
            },
        };
    });

    // Dependencies
    if let Some(inst) = &state.viewing {
        ui.horizontal(|ui| {
            ui.strong(format!("{}:", lang::t("info.dependencies", lang)));
            if inst.dependencies.is_empty() {
                ui.label(lang::t("info.no_dependencies", lang));
            } else {
                ui.label(format!("{}", inst.dependencies.len()));
            }
        });
    }

    // Path + open button
    ui.horizontal(|ui| {
        ui.strong(format!("{}:", lang::t("info.path", lang)));
        let absolute = instances_dir.join(name);
        ui.label(absolute.display().to_string());
        if ui
            .add(icon_button(icons::ICON_FOLDER))
            .on_hover_text(lang::t("tip.open_folder", lang))
            .clicked()
        {
            let _ = open::that(&absolute);
        }
    });
}
