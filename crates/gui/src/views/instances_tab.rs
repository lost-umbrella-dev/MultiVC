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
use crate::lang::{self, Lang};
use crate::state::{
    InstanceForm, InstancePanelState, InstancePanelTab, InstancesSortColumn, InstancesTabState, SortDir,
};
use crate::toasts;
use crate::widgets::{
    confirm_dialog, form_row, icon_button, image_from_base64, open_folder, pick_image_as_base64, start_image_pick,
    striped_frame, tab_toolbar,
};

// ── Row helpers ──────────────────────────────────────────────────────

#[derive(Default)]
struct InstanceRowActions {
    launch: bool,
    stop: bool,
    open_folder: bool,
    delete: bool,
    view_info: bool,
    view_log: bool,
}

fn instance_row(
    ui: &mut egui::Ui,
    name: &str,
    meta: &InstancesItem,
    is_busy: bool,
    is_running: bool,
    striped_bg: bool,
    lang: Lang,
) -> InstanceRowActions {
    let mut actions = InstanceRowActions::default();

    let frame = striped_frame(striped_bg, ui);

    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let last_launch_width = 140.0;
            let actions_width = 190.0;
            let icon_width = 24.0;
            let name_width = (ui.available_width()
                - icon_width
                - last_launch_width
                - actions_width
                - ui.spacing().item_spacing.x * 4.0)
                .max(80.0);

            // Small icon
            image_from_base64(
                ui,
                &format!("row/{name}/icon"),
                &meta.icon,
                egui::vec2(icon_width, icon_width),
                icons::DEFAULT_ICON_SVG,
            );

            ui.add_sized([name_width, ui.available_height()], egui::Label::new(name).truncate());

            let last_launch_str = match meta.last_launch {
                Some(dt) => dt.format("%Y-%m-%d %H:%M").to_string(),
                None => "\u{2014}".to_owned(),
            };
            ui.add_sized(
                [last_launch_width, ui.available_height()],
                egui::Label::new(&last_launch_str).truncate(),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Delete (right-most)
                if ui
                    .add_enabled(!is_busy && !is_running, icon_button(icons::ICON_DELETE))
                    .on_hover_text(if is_running {
                        lang::t("tip.stop_first", lang)
                    } else {
                        lang::t("tip.delete", lang)
                    })
                    .clicked()
                {
                    actions.delete = true;
                }

                // Open folder
                if ui
                    .add(icon_button(icons::ICON_FOLDER))
                    .on_hover_text(lang::t("tip.open_folder", lang))
                    .clicked()
                {
                    actions.open_folder = true;
                }

                // View log
                if ui
                    .add(icon_button(icons::ICON_LOG))
                    .on_hover_text(lang::t("tip.view_log", lang))
                    .clicked()
                {
                    actions.view_log = true;
                }

                // Info
                if ui
                    .add(icon_button(icons::ICON_INFO))
                    .on_hover_text(lang::t("tip.info", lang))
                    .clicked()
                {
                    actions.view_info = true;
                }

                // Launch / Stop
                if is_running {
                    if ui
                        .add(icon_button(
                            egui::RichText::new(icons::ICON_STOP).color(egui::Color32::RED),
                        ))
                        .on_hover_text(lang::t("tip.stop", lang))
                        .clicked()
                    {
                        actions.stop = true;
                    }
                    ui.colored_label(egui::Color32::GREEN, lang::t("status.running", lang));
                } else if ui
                    .add_enabled(!is_busy, icon_button(icons::ICON_LAUNCH))
                    .on_hover_text(lang::t("tip.launch", lang))
                    .clicked()
                {
                    actions.launch = true;
                }
            });
        });
    });

    actions
}

// ── Instance panel (tabbed window) ──────────────────────────────────

fn render_instance_panel(
    ui: &mut egui::Ui,
    state: &mut InstancesTabState,
    handle: &WorkerHandle,
    installed_cores: &[(Hash, LockItem)],
    toasts_out: &mut Toasts,
    lang: Lang,
) {
    let Some(ref panel) = state.instance_panel else {
        return;
    };

    let mut open = true;
    let instance_name = panel.name.clone();
    let current_tab = panel.tab;
    let is_running = state.running_instances.contains_key(&instance_name);
    let pid = state.running_instances.get(&instance_name).copied();

    let mut new_tab = current_tab;

    egui::Window::new(&instance_name)
        .collapsible(true)
        .resizable(true)
        .default_size([700.0, 500.0])
        .default_pos([100.0, 80.0])
        .open(&mut open)
        .show(ui.ctx(), |ui| {
            // ── Tab bar ─────────────────────────────────────────
            ui.horizontal(|ui| {
                ui.selectable_value(&mut new_tab, InstancePanelTab::Info, lang::t("panel.info", lang));
                ui.selectable_value(&mut new_tab, InstancePanelTab::Log, lang::t("panel.log", lang));
                ui.selectable_value(
                    &mut new_tab,
                    InstancePanelTab::Settings,
                    lang::t("panel.settings", lang),
                );
            });
            ui.separator();

            match new_tab {
                InstancePanelTab::Info => {
                    render_info_tab(
                        ui,
                        state,
                        handle,
                        installed_cores,
                        &instance_name,
                        is_running,
                        pid,
                        lang,
                    );
                },
                InstancePanelTab::Log => {
                    render_log_tab(ui, state, &instance_name, is_running, pid, toasts_out, lang);
                },
                InstancePanelTab::Settings => {
                    render_settings_tab(ui, state, handle, &instance_name, toasts_out, lang);
                },
            }
        });

    // Update tab selection
    if let Some(ref mut panel) = state.instance_panel {
        panel.tab = new_tab;
    }

    if !open {
        state.instance_panel = None;
    }
}

#[allow(clippy::too_many_arguments)]
fn render_info_tab(
    ui: &mut egui::Ui,
    state: &mut InstancesTabState,
    handle: &WorkerHandle,
    installed_cores: &[(Hash, LockItem)],
    name: &str,
    is_running: bool,
    pid: Option<u32>,
    lang: Lang,
) {
    let meta = state.installed.iter().find(|(n, _)| n == name).map(|(_, m)| m);
    let Some(meta) = meta else {
        ui.label("Instance not found");
        return;
    };
    let meta = meta.clone();

    let icon_size = 48.0;
    let banner_height = 80.0;

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
            let banner_width = ui.available_width().min(300.0);
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
        let core_label = installed_cores
            .iter()
            .find(|(h, _)| *h == inst.core_version)
            .map(|(h, li)| format!("{} v{} ({})", li.item.name, li.item.version, h))
            .unwrap_or_else(|| inst.core_version.to_string());
        ui.horizontal(|ui| {
            ui.strong(format!("{}:", lang::t("info.core_version", lang)));
            ui.label(core_label);
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
            Some(bytes) => ui.label(crate::app::format_size(bytes)),
            None => {
                ui.label(lang::t("info.calculating", lang));
                // Request dir size calculation
                handle.try_send(Command::GetInstanceDirSize { name: name.to_owned() });
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
        let relative = std::path::Path::new("instances").join(name);
        let absolute = std::env::current_dir()
            .map(|cwd| cwd.join(&relative))
            .unwrap_or_else(|_| relative.clone());
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

fn render_log_tab(
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

fn render_settings_tab(
    ui: &mut egui::Ui,
    state: &mut InstancesTabState,
    handle: &WorkerHandle,
    name: &str,
    toasts_out: &mut Toasts,
    lang: Lang,
) {
    let meta = state.installed.iter().find(|(n, _)| n == name).map(|(_, m)| m.clone());
    let Some(meta) = meta else {
        ui.label("Instance not found");
        return;
    };

    let panel = state.instance_panel.as_mut().unwrap();

    // Initialize edit state on first open
    if panel.edit_description.is_none() {
        panel.edit_description = Some(
            state
                .viewing
                .as_ref()
                .and_then(|inst| inst.description.clone())
                .unwrap_or_default(),
        );
    }
    if panel.edit_icon.is_none() {
        panel.edit_icon = Some(meta.icon.clone());
    }
    if panel.edit_banner.is_none() {
        panel.edit_banner = Some(meta.banner.clone());
    }

    // Poll in-flight image tasks
    if let Some(ref task) = panel.icon_pick {
        if let Some(result) = task.poll() {
            match result {
                Ok(b64) => panel.edit_icon = Some(b64),
                Err(e) => tracing::error!("Icon pick failed: {e}"),
            }
            panel.icon_pick = None;
        }
    }
    if let Some(ref task) = panel.banner_pick {
        if let Some(result) = task.poll() {
            match result {
                Ok(b64) => panel.edit_banner = Some(b64),
                Err(e) => tracing::error!("Banner pick failed: {e}"),
            }
            panel.banner_pick = None;
        }
    }

    let edit_desc = panel.edit_description.as_mut().unwrap();
    let edit_icon = panel.edit_icon.clone().unwrap_or_default();
    let edit_banner = panel.edit_banner.clone().unwrap_or_default();

    // Check if anything has changed from the original
    let orig_desc = state
        .viewing
        .as_ref()
        .and_then(|inst| inst.description.clone())
        .unwrap_or_default();
    let changed = *edit_desc != orig_desc || edit_icon != meta.icon || edit_banner != meta.banner;

    // Description editor
    ui.horizontal(|ui| {
        ui.label(lang::t("info.description", lang));
        ui.text_edit_singleline(edit_desc);
    });

    ui.separator();

    // Icon picker
    ui.horizontal(|ui| {
        image_from_base64(
            ui,
            &format!("instance/{name}/icon/settings"),
            &edit_icon,
            egui::vec2(48.0, 48.0),
            icons::DEFAULT_ICON_SVG,
        );
        if panel.icon_pick.is_some() {
            ui.spinner();
        } else if ui
            .button(lang::t("isettings.change_icon", lang))
            .on_hover_text(lang::t("tip.formats", lang))
            .clicked()
        {
            panel.icon_pick = start_image_pick(ui.ctx());
        }
    });

    ui.add_space(4.0);

    // Banner picker
    image_from_base64(
        ui,
        &format!("instance/{name}/banner/settings"),
        &edit_banner,
        egui::vec2(ui.available_width().min(400.0), 80.0),
        icons::DEFAULT_BANNER_SVG,
    );
    if panel.banner_pick.is_some() {
        ui.spinner();
    } else if ui
        .button(lang::t("isettings.change_banner", lang))
        .on_hover_text(lang::t("tip.formats", lang))
        .clicked()
    {
        panel.banner_pick = start_image_pick(ui.ctx());
    }

    ui.separator();

    // Save button
    if ui
        .add_enabled(changed, egui::Button::new(lang::t("isettings.save", lang)))
        .clicked()
    {
        let description = panel.edit_description.clone().unwrap_or_default();
        let icon = panel.edit_icon.clone().unwrap_or_default();
        let banner = panel.edit_banner.clone().unwrap_or_default();

        if let Some(inst) = &state.viewing {
            let config = Instance {
                description: if description.is_empty() {
                    None
                } else {
                    Some(description)
                },
                core_version: inst.core_version.clone(),
                dependencies: inst.dependencies.clone(),
            };
            let new_meta = InstancesItem {
                icon,
                banner,
                last_launch: meta.last_launch,
                created_at: meta.created_at,
            };
            handle.try_send(Command::EditInstance {
                name: name.to_owned(),
                config,
                meta: new_meta.clone(),
            });
            // Update local cache
            if let Some((_, m)) = state.installed.iter_mut().find(|(n, _)| n == name) {
                *m = new_meta;
            }
            toasts::success(toasts_out, lang::t("toast.instances_saved", lang));
        }
    }
}

// ── Main render ──────────────────────────────────────────────────────

/// Renders the Instances tab content.

pub fn render(
    ui: &mut egui::Ui,
    state: &mut InstancesTabState,
    handle: &WorkerHandle,
    installed_cores: &[(Hash, LockItem)],
    toasts_out: &mut Toasts,
    lang: Lang,
) {
    // ── Toolbar ──────────────────────────────────────────────────
    tab_toolbar(ui, lang::t("tab.instances", lang), |ui| {
        if ui
            .add_enabled(!state.busy, icon_button(icons::ICON_VALIDATE))
            .on_hover_text(lang::t("tip.validate_instances", lang))
            .clicked()
        {
            state.busy = true;
            handle.try_send(Command::ValidateInstances);
        }

        if ui
            .add(icon_button("+"))
            .on_hover_text(lang::t("action.new_instance", lang))
            .clicked()
            && state.create_form.is_none()
        {
            state.create_form = Some(InstanceForm::default());
        }
    });

    ui.separator();

    // ── Create instance modal ────────────────────────────────────
    let mut should_create_instance: Option<(String, Hash, Option<String>, String, String)> = None;
    let mut should_cancel = false;

    if state.create_form.is_some() {
        let mut open = true;
        egui::Window::new(lang::t("form.new_instance", lang))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui.ctx(), |ui| {
                let form = state.create_form.as_mut().unwrap();

                form_row(ui, lang::t("form.name", lang), &mut form.name);
                form_row(ui, lang::t("form.description", lang), &mut form.description);

                ui.horizontal(|ui| {
                    ui.label(lang::t("form.core", lang));

                    let selected_text = match form.selected_core_idx {
                        Some(idx) if idx < installed_cores.len() => {
                            let (_, li) = &installed_cores[idx];
                            format!("{} v{}", li.item.name, li.item.version)
                        },
                        _ => lang::t("form.select_core", lang).to_owned(),
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

                // Icon picker
                ui.horizontal(|ui| {
                    if form.icon.is_empty() {
                        ui.label(lang::t("isettings.no_icon", lang));
                    } else {
                        image_from_base64(
                            ui,
                            "create_form/icon",
                            &form.icon,
                            egui::vec2(32.0, 32.0),
                            icons::DEFAULT_ICON_SVG,
                        );
                    }
                    if ui
                        .button(lang::t("isettings.change_icon", lang))
                        .on_hover_text(lang::t("tip.formats", lang))
                        .clicked()
                    {
                        if let Some(b64) = pick_image_as_base64() {
                            form.icon = b64;
                        }
                    }
                });

                // Banner picker
                ui.horizontal(|ui| {
                    if form.banner.is_empty() {
                        ui.label(lang::t("isettings.no_banner", lang));
                    } else {
                        image_from_base64(
                            ui,
                            "create_form/banner",
                            &form.banner,
                            egui::vec2(200.0, 60.0),
                            icons::DEFAULT_BANNER_SVG,
                        );
                    }
                    if ui
                        .button(lang::t("isettings.change_banner", lang))
                        .on_hover_text(lang::t("tip.formats", lang))
                        .clicked()
                    {
                        if let Some(b64) = pick_image_as_base64() {
                            form.banner = b64;
                        }
                    }
                });

                ui.add_space(8.0);

                let can_create = !form.name.is_empty() && form.selected_core_idx.is_some();

                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(can_create, egui::Button::new(lang::t("action.create", lang)))
                        .clicked()
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
                    if ui.button(lang::t("action.cancel", lang)).clicked() {
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
        let meta = InstancesItem {
            icon,
            banner,
            last_launch: None,
            created_at: None,
        };
        handle.try_send(Command::CreateInstance { name, config, meta });
        toasts::info(toasts_out, lang::t("status.creating", lang));
        state.create_form = None;
    }

    // ── Validation results ───────────────────────────────────────
    if !state.validation.is_empty() {
        ui.heading(lang::t("section.validation", lang));
        for reason in &state.validation {
            match reason {
                InstanceValidateReason::NotFound(name, _meta) => {
                    ui.colored_label(
                        egui::Color32::RED,
                        format!("{}: {name}", lang::t("validation.dir_not_found", lang)),
                    );
                },
            }
        }
    }

    // ── Instance list ───────────────────────────────────────────
    if state.installed.is_empty() && state.create_form.is_none() {
        ui.centered_and_justified(|ui| {
            ui.label(lang::t("status.no_instances", lang));
        });
    } else if !state.installed.is_empty() {
        // Column header with sorting
        ui.horizontal(|ui| {
            let last_launch_width = 140.0;
            let actions_width = 190.0;
            let icon_width = 24.0;
            let name_width = (ui.available_width()
                - icon_width
                - last_launch_width
                - actions_width
                - ui.spacing().item_spacing.x * 4.0)
                .max(80.0);

            // Empty space for icon column
            ui.add_space(icon_width + ui.spacing().item_spacing.x);

            let is_active_name = state.sort_col == InstancesSortColumn::Name;
            let indicator_name = match (is_active_name, state.sort_dir) {
                (true, SortDir::Ascending) => " \u{25B2}",
                (true, SortDir::Descending) => " \u{25BC}",
                _ => "",
            };
            let label_name = format!("{}{}", lang::t("col.name", lang), indicator_name);

            if ui
                .add_sized(
                    [name_width, ui.available_height()],
                    egui::Button::new(egui::RichText::new(label_name).strong()).frame(false),
                )
                .clicked()
            {
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

            let is_active_launch = state.sort_col == InstancesSortColumn::LastLaunch;
            let indicator_launch = match (is_active_launch, state.sort_dir) {
                (true, SortDir::Ascending) => " \u{25B2}",
                (true, SortDir::Descending) => " \u{25BC}",
                _ => "",
            };
            let label_launch = format!("{}{}", lang::t("col.last_launch", lang), indicator_launch);

            if ui
                .add_sized(
                    [last_launch_width, ui.available_height()],
                    egui::Button::new(egui::RichText::new(label_launch).strong()).frame(false),
                )
                .clicked()
            {
                if state.sort_col == InstancesSortColumn::LastLaunch {
                    state.sort_dir = state.sort_dir.cycle();
                    if state.sort_dir == SortDir::None {
                        state.sort_col = InstancesSortColumn::default();
                    }
                } else {
                    state.sort_col = InstancesSortColumn::LastLaunch;
                    state.sort_dir = SortDir::Descending;
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(lang::t("col.actions", lang)).strong());
            });
        });

        // Sorting
        let mut sorted_indices: Vec<usize> = (0..state.installed.len()).collect();

        sorted_indices.sort_by(|&a, &b| {
            let (name_a, item_a) = &state.installed[a];
            let (name_b, item_b) = &state.installed[b];

            match (state.sort_col, state.sort_dir) {
                (InstancesSortColumn::LastLaunch, SortDir::Descending) => item_b.last_launch.cmp(&item_a.last_launch),
                (InstancesSortColumn::LastLaunch, SortDir::Ascending) => item_a.last_launch.cmp(&item_b.last_launch),
                (InstancesSortColumn::Name, SortDir::Ascending) => name_a.cmp(name_b),
                (InstancesSortColumn::Name, SortDir::Descending) => name_b.cmp(name_a),
                _ => std::cmp::Ordering::Equal,
            }
        });

        // Scrollable rows
        egui::ScrollArea::vertical()
            .id_salt("instances_list_scroll")
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                let mut to_remove: Option<String> = None;

                for (display_idx, &idx) in sorted_indices.iter().enumerate() {
                    let (name, meta) = &state.installed[idx];
                    let is_busy = state.busy || state.busy_instances.contains(name);
                    let is_running = state.running_instances.contains_key(name);

                    let row_actions = instance_row(ui, name, meta, is_busy, is_running, display_idx % 2 == 1, lang);

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
                    if row_actions.view_info {
                        state.instance_panel = Some(InstancePanelState::new(name.clone(), InstancePanelTab::Info));
                        handle.try_send(Command::GetInstance { name: name.clone() });
                    }
                    if row_actions.view_log {
                        state.instance_panel = Some(InstancePanelState::new(name.clone(), InstancePanelTab::Log));
                    }
                }

                if let Some(name) = to_remove {
                    state.confirm_remove = Some(name);
                }
            });
    }

    // ── Delete confirmation modal ────────────────────────────────
    if let Some(ref name) = state.confirm_remove.clone() {
        let msg = format!("{} \"{}\"?", lang::t("confirm.delete_instance", lang), name);
        if let Some(confirmed) = confirm_dialog(ui.ctx(), lang::t("confirm.delete_instance", lang), &msg) {
            if confirmed {
                state.busy_instances.insert(name.clone());
                handle.try_send(Command::RemoveInstance { name: name.clone() });
            }
            state.confirm_remove = None;
        }
    }

    // ── Instance panel (tabbed floating window) ──────────────────
    render_instance_panel(ui, state, handle, installed_cores, toasts_out, lang);
}
