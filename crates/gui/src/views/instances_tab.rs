//! Instances tab view.

use eframe::egui;
use egui_toast::Toasts;

use clients::hash::Hash;
use composer::item::LockItem;
use composer::lock::instance::Instance;
use composer::lock::instances::{InstanceValidateReason, InstancesItem};
use composer::message::Command;
use composer::worker::WorkerHandle;

use crate::state::{InstanceForm, InstancesTabState};
use crate::toasts;
use crate::widgets::icon_button;

// ── Row helpers ──────────────────────────────────────────────────────

#[derive(Default)]
struct InstanceRowActions {
    launch: bool,
    stop: bool,
    open_folder: bool,
    delete: bool,
    view_log: bool,
}

fn instance_row(
    ui: &mut egui::Ui,
    name: &str,
    _meta: &InstancesItem,
    is_busy: bool,
    is_running: bool,
    striped_bg: bool,
) -> InstanceRowActions {
    let mut actions = InstanceRowActions::default();

    let frame = if striped_bg {
        egui::Frame::NONE.fill(ui.visuals().faint_bg_color)
    } else {
        egui::Frame::NONE
    };

    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let actions_width = 100.0;
            let name_width = (ui.available_width() - actions_width - ui.spacing().item_spacing.x).max(80.0);

            ui.add_sized([name_width, ui.available_height()], egui::Label::new(name).truncate());

            // Actions — right side
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Delete (right-most) — disabled when running
                if ui
                    .add_enabled(!is_busy && !is_running, icon_button("\u{1F5D1}"))
                    .on_hover_text(if is_running { "Stop instance first" } else { "Delete" })
                    .clicked()
                {
                    actions.delete = true;
                }

                // Open folder
                if ui
                    .add(icon_button("\u{1F4C2}"))
                    .on_hover_text("Open folder")
                    .clicked()
                {
                    actions.open_folder = true;
                }

                // View log
                if ui
                    .add(icon_button("\u{1F4C4}"))
                    .on_hover_text("View log")
                    .clicked()
                {
                    actions.view_log = true;
                }

                // Launch / Stop
                if is_running {
                    if ui
                        .add(icon_button(egui::RichText::new("\u{23F9}").color(egui::Color32::RED)))
                        .on_hover_text("Stop")
                        .clicked()
                    {
                        actions.stop = true;
                    }
                    ui.colored_label(egui::Color32::GREEN, "Running");
                } else if ui
                    .add_enabled(!is_busy, icon_button("\u{25B6}"))
                    .on_hover_text("Launch")
                    .clicked()
                {
                    actions.launch = true;
                }
            });
        });
    });

    actions
}

// ── Main render ──────────────────────────────────────────────────────

/// Renders the Instances tab content.
pub fn render(
    ui: &mut egui::Ui,
    state: &mut InstancesTabState,
    handle: &WorkerHandle,
    installed_cores: &[(Hash, LockItem)],
    toasts_out: &mut Toasts,
) {
    // ── Toolbar ──────────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.heading("Instances");

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(!state.busy, icon_button("\u{2714}"))
                .on_hover_text("Validate instances")
                .clicked()
            {
                state.busy = true;
                handle.try_send(Command::ValidateInstances);
            }

            if ui
                .add(icon_button("+"))
                .on_hover_text("New instance")
                .clicked()
                && state.create_form.is_none()
            {
                state.create_form = Some(InstanceForm::default());
            }
        });
    });

    ui.separator();

    // ── Create instance modal ────────────────────────────────────
    let mut should_create_instance: Option<(String, Hash, Option<String>, String, String)> = None;
    let mut should_cancel = false;

    if state.create_form.is_some() {
        let mut open = true;
        egui::Window::new("New instance")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui.ctx(), |ui| {
                let form = state.create_form.as_mut().unwrap();

                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut form.name);
                });

                ui.horizontal(|ui| {
                    ui.label("Description:");
                    ui.text_edit_singleline(&mut form.description);
                });

                ui.horizontal(|ui| {
                    ui.label("Core:");

                    let selected_text = match form.selected_core_idx {
                        Some(idx) if idx < installed_cores.len() => {
                            let (_, li) = &installed_cores[idx];
                            format!("{} v{}", li.item.name, li.item.version)
                        },
                        _ => "(select core)".to_owned(),
                    };

                    egui::ComboBox::from_id_salt("core_selector")
                        .selected_text(selected_text)
                        .show_ui(ui, |ui| {
                            for (idx, (hash, li)) in installed_cores.iter().enumerate() {
                                let label = format!("{} v{}", li.item.name, li.item.version);
                                let hash_short = {
                                    let s = hash.to_string();
                                    if s.len() > 12 { format!("{}...", &s[..12]) } else { s }
                                };
                                let display = format!("{label}  ({hash_short})");
                                ui.selectable_value(&mut form.selected_core_idx, Some(idx), display);
                            }
                        });
                });

                ui.add_space(8.0);

                let can_create = !form.name.is_empty() && form.selected_core_idx.is_some();

                ui.horizontal(|ui| {
                    if ui.add_enabled(can_create, egui::Button::new("Create")).clicked()
                        && let Some(idx) = form.selected_core_idx
                        && let Some((hash, _)) = installed_cores.get(idx)
                    {
                        should_create_instance = Some((
                            form.name.clone(),
                            hash.clone(),
                            if form.description.is_empty() {
                                None
                            } else {
                                Some(form.description.clone())
                            },
                            form.icon.clone(),
                            form.banner.clone(),
                        ));
                    }
                    if ui.button("Cancel").clicked() {
                        should_cancel = true;
                    }
                });
            });
        if !open {
            state.create_form = None;
        }
    }

    if should_cancel {
        state.create_form = None;
    }

    if let Some((name, core_hash, description, icon, banner)) = should_create_instance {
        let config = Instance {
            description,
            core_version: core_hash,
            dependencies: vec![],
        };
        let meta = InstancesItem { icon, banner };
        handle.try_send(Command::CreateInstance { name, config, meta });
        toasts::info(toasts_out, "Creating instance...");
        state.create_form = None;
    }

    // ── Validation results ───────────────────────────────────────
    if !state.validation.is_empty() {
        ui.heading("Validation results");
        for reason in &state.validation {
            match reason {
                InstanceValidateReason::NotFound(name, _meta) => {
                    ui.colored_label(egui::Color32::RED, format!("Directory not found: {name}"));
                },
            }
        }
    }

    // ── Instance list (fills all remaining space, full width) ────
    if state.installed.is_empty() && state.create_form.is_none() {
        ui.centered_and_justified(|ui| {
            ui.label("No instances. Press \"+\" to create one.");
        });
    } else if !state.installed.is_empty() {
        // Column header
        ui.horizontal(|ui| {
            let actions_width = 100.0;
            let name_width = (ui.available_width() - actions_width - ui.spacing().item_spacing.x).max(80.0);

            ui.add_sized(
                [name_width, ui.available_height()],
                egui::Label::new(egui::RichText::new("Name").strong()),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("Actions").strong());
            });
        });

        // Scrollable rows — fills all remaining vertical space
        egui::ScrollArea::vertical()
            .id_salt("instances_list_scroll")
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                let mut to_remove: Option<String> = None;

                for (idx, (name, meta)) in state.installed.iter().enumerate() {
                    let is_busy = state.busy || state.busy_instances.contains(name);
                    let is_running = state.running_instances.contains_key(name);

                    let row_actions = instance_row(ui, name, meta, is_busy, is_running, idx % 2 == 1);

                    if row_actions.launch {
                        handle.try_send(Command::LaunchInstance { name: name.clone() });
                    }

                    if row_actions.stop {
                        handle.try_send(Command::StopInstance { name: name.clone() });
                    }

                    if row_actions.open_folder {
                        let folder = std::path::Path::new("instances").join(name);
                        if folder.exists() {
                            if let Err(e) = open::that(&folder) {
                                toasts::error(toasts_out, format!("Failed to open folder: {e}"));
                            }
                        } else {
                            toasts::warning(toasts_out, format!("Folder does not exist: {}", folder.display()));
                        }
                    }

                    if row_actions.delete {
                        to_remove = Some(name.clone());
                    }

                    if row_actions.view_log {
                        state.log_viewer = Some(name.clone());
                    }
                }

                if let Some(name) = to_remove {
                    state.confirm_remove = Some(name);
                }
            });
    }

    // ── Delete confirmation modal ────────────────────────────────
    if let Some(ref name) = state.confirm_remove.clone() {
        let mut open = true;
        egui::Window::new("Delete instance?")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui.ctx(), |ui| {
                ui.label(format!("Are you sure you want to delete \"{}\"?", name));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Yes, delete").clicked() {
                        state.busy_instances.insert(name.clone());
                        handle.try_send(Command::RemoveInstance { name: name.clone() });
                        state.confirm_remove = None;
                    }
                    if ui.button("Cancel").clicked() {
                        state.confirm_remove = None;
                    }
                });
            });
        if !open {
            state.confirm_remove = None;
        }
    }

    // ── Log viewer modal ─────────────────────────────────────────
    if let Some(ref instance_name) = state.log_viewer.clone() {
        let mut open = true;
        let log_path = std::path::Path::new("instances").join(instance_name).join("latest.log");

        let is_instance_running = state.running_instances.contains_key(instance_name);

        egui::Window::new(format!("Log: {}", instance_name))
            .collapsible(true)
            .resizable(true)
            .default_size([700.0, 450.0])
            .default_pos([100.0, 100.0])
            .open(&mut open)
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    if is_instance_running {
                        ui.spinner();
                        ui.colored_label(egui::Color32::GREEN, "Running");
                        ui.separator();
                    }

                    // Open file — disabled when running (file may be locked)
                    if ui
                        .add_enabled(!is_instance_running, egui::Button::new("\u{1F4C2} Open file"))
                        .on_hover_text(if is_instance_running {
                            "Stop instance first"
                        } else {
                            "Open in external editor"
                        })
                        .clicked()
                        && log_path.exists()
                        && let Err(e) = open::that(&log_path)
                    {
                        toasts::error(toasts_out, format!("Failed to open log: {e}"));
                    }
                });

                ui.separator();

                let log_content = if log_path.exists() {
                    std::fs::read_to_string(&log_path).unwrap_or_else(|e| format!("Error reading log: {e}"))
                } else {
                    "No log file found. Launch the instance first.".to_owned()
                };

                egui::ScrollArea::both()
                    .id_salt("log_viewer_scroll")
                    .auto_shrink(false)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for line in log_content.lines() {
                            let color = if line.starts_with("[E]") {
                                egui::Color32::RED
                            } else if line.starts_with("[W]") {
                                egui::Color32::YELLOW
                            } else {
                                ui.visuals().text_color()
                            };
                            ui.label(egui::RichText::new(line).monospace().color(color));
                        }
                    });
            });

        // Auto-refresh: repaint every 500ms while instance is running
        if is_instance_running {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
        }

        if !open {
            state.log_viewer = None;
        }
    }
}
