//! Instances tab view.

use eframe::egui;
use egui_toast::Toasts;

use clients::hash::Hash;
use composer::item::LockItem;
use composer::lock::instance::Instance;
use composer::lock::instances::{InstanceValidateReason, InstancesItem};
use composer::message::Command;
use composer::worker::WorkerHandle;

use crate::icons;
use crate::state::{InstanceForm, InstancesSortColumn, InstancesTabState, SortDir};
use crate::toasts;
use crate::widgets::{confirm_dialog, form_row, icon_button, open_folder, striped_frame, tab_toolbar};

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
    meta: &InstancesItem,
    is_busy: bool,
    is_running: bool,
    striped_bg: bool,
) -> InstanceRowActions {
    let mut actions = InstanceRowActions::default();

    let frame = striped_frame(striped_bg, ui);

    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let last_launch_width = 140.0;
            let actions_width = 100.0;
            let name_width =
                (ui.available_width() - last_launch_width - actions_width - ui.spacing().item_spacing.x * 2.0)
                    .max(80.0);

            ui.add_sized([name_width, ui.available_height()], egui::Label::new(name).truncate());

            let last_launch_str = match meta.last_launch {
                Some(dt) => dt.format("%Y-%m-%d %H:%M").to_string(),
                None => "—".to_owned(),
            };
            ui.add_sized(
                [last_launch_width, ui.available_height()],
                egui::Label::new(&last_launch_str).truncate(),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Delete (right-most) — disabled when running
                if ui
                    .add_enabled(!is_busy && !is_running, icon_button(icons::ICON_DELETE))
                    .on_hover_text(if is_running { "Stop instance first" } else { "Delete" })
                    .clicked()
                {
                    actions.delete = true;
                }

                // Open folder
                if ui
                    .add(icon_button(icons::ICON_FOLDER))
                    .on_hover_text("Open folder")
                    .clicked()
                {
                    actions.open_folder = true;
                }

                // View log
                if ui.add(icon_button(icons::ICON_LOG)).on_hover_text("View log").clicked() {
                    actions.view_log = true;
                }

                // Launch / Stop
                if is_running {
                    if ui
                        .add(icon_button(
                            egui::RichText::new(icons::ICON_STOP).color(egui::Color32::RED),
                        ))
                        .on_hover_text("Stop")
                        .clicked()
                    {
                        actions.stop = true;
                    }
                    ui.colored_label(egui::Color32::GREEN, "Running");
                } else if ui
                    .add_enabled(!is_busy, icon_button(icons::ICON_LAUNCH))
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
    tab_toolbar(ui, "Instances", |ui| {
        if ui
            .add_enabled(!state.busy, icon_button(icons::ICON_VALIDATE))
            .on_hover_text("Validate instances")
            .clicked()
        {
            state.busy = true;
            handle.try_send(Command::ValidateInstances);
        }

        if ui.add(icon_button("+")).on_hover_text("New instance").clicked() && state.create_form.is_none() {
            state.create_form = Some(InstanceForm::default());
        }
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

                form_row(ui, "Name:", &mut form.name);
                form_row(ui, "Description:", &mut form.description);

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
        // Column header with sorting
        ui.horizontal(|ui| {
            let last_launch_width = 140.0;
            let actions_width = 100.0;
            let name_width = (ui.available_width() - last_launch_width - actions_width - ui.spacing().item_spacing.x * 2.0).max(80.0);

            // Name header
            let is_active_name = state.sort_col == InstancesSortColumn::Name;
            let indicator_name = match (is_active_name, state.sort_dir) {
                (true, SortDir::Ascending) => " ▲",
                (true, SortDir::Descending) => " ▼",
                _ => "",
            };
            let label_name = format!("Name{}", indicator_name);

            if ui.add_sized([name_width, ui.available_height()],
                egui::Button::new(egui::RichText::new(label_name).strong()).frame(false)
            ).clicked() {
                if state.sort_col == InstancesSortColumn::Name {
                    state.sort_dir = state.sort_dir.cycle();
                    if state.sort_dir == SortDir::None {
                        state.sort_col = InstancesSortColumn::default();
                    }
                } else {
                    state.sort_col = InstancesSortColumn::Name;
                    state.sort_dir = SortDir::Ascending;
                }
            }

            // Last Launch header
            let is_active_launch = state.sort_col == InstancesSortColumn::LastLaunch;
            let indicator_launch = match (is_active_launch, state.sort_dir) {
                (true, SortDir::Ascending) => " ▲",
                (true, SortDir::Descending) => " ▼",
                _ => "",
            };
            let label_launch = format!("Last Launch{}", indicator_launch);

            if ui.add_sized([last_launch_width, ui.available_height()],
                egui::Button::new(egui::RichText::new(label_launch).strong()).frame(false)
            ).clicked() {
                if state.sort_col == InstancesSortColumn::LastLaunch {
                    state.sort_dir = state.sort_dir.cycle();
                    if state.sort_dir == SortDir::None {
                        state.sort_col = InstancesSortColumn::default();
                    }
                } else {
                    state.sort_col = InstancesSortColumn::LastLaunch;
                    state.sort_dir = SortDir::Descending;  // most recent first by default
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("Actions").strong());
            });
        });

        // Apply sorting
        let mut sorted_indices: Vec<usize> = (0..state.installed.len()).collect();

        sorted_indices.sort_by(|&a, &b| {
            let (name_a, item_a) = &state.installed[a];
            let (name_b, item_b) = &state.installed[b];

            match (state.sort_col, state.sort_dir) {
                (InstancesSortColumn::LastLaunch, SortDir::Descending) => {
                    item_b.last_launch.cmp(&item_a.last_launch)  // reversed for descending
                }
                (InstancesSortColumn::LastLaunch, SortDir::Ascending) => {
                    item_a.last_launch.cmp(&item_b.last_launch)
                }
                (InstancesSortColumn::Name, SortDir::Ascending) => {
                    name_a.cmp(name_b)
                }
                (InstancesSortColumn::Name, SortDir::Descending) => {
                    name_b.cmp(name_a)
                }
                _ => std::cmp::Ordering::Equal,
            }
        });

        // Scrollable rows — fills all remaining vertical space
        egui::ScrollArea::vertical()
            .id_salt("instances_list_scroll")
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                let mut to_remove: Option<String> = None;

                for (display_idx, &idx) in sorted_indices.iter().enumerate() {
                    let (name, meta) = &state.installed[idx];
                    let is_busy = state.busy || state.busy_instances.contains(name);
                    let is_running = state.running_instances.contains_key(name);

                    let row_actions = instance_row(ui, name, meta, is_busy, is_running, display_idx % 2 == 1);

                    if row_actions.launch {
                        handle.try_send(Command::LaunchInstance { name: name.clone() });
                    }

                    if row_actions.stop {
                        handle.try_send(Command::StopInstance { name: name.clone() });
                    }

                    if row_actions.open_folder {
                        let folder = std::path::Path::new("instances").join(name);
                        open_folder(&folder, toasts_out);
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
        let msg = format!("Are you sure you want to delete \"{}\"?", name);
        if let Some(confirmed) = confirm_dialog(ui.ctx(), "Delete instance?", &msg) {
            if confirmed {
                state.busy_instances.insert(name.clone());
                handle.try_send(Command::RemoveInstance { name: name.clone() });
            }
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
                        .add_enabled(!is_instance_running, egui::Button::new(icons::LABEL_OPEN_FILE))
                        .on_hover_text(if is_instance_running {
                            "Stop instance first"
                        } else {
                            "Open in external editor"
                        })
                        .clicked()
                    {
                        open_folder(&log_path, toasts_out);
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
