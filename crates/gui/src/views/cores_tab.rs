//! Cores tab view.

use std::collections::HashSet;

use eframe::egui;
use egui_toast::Toasts;

use clients::github::GitHubListOptions;
use clients::hash::Hash;
use composer::lock::ValidateReason;
use composer::message::Command;
use composer::worker::WorkerHandle;

use crate::state::CoresTabState;
use crate::toasts;
use crate::widgets::ProgressRing;

/// Результат рендера — действия, которые должен обработать App.
pub struct CoresTabAction {
    /// Переключить на вкладку Instances с предзаполненным ядром.
    pub switch_to_instances_with_core: Option<usize>,
}

// ── Row helpers ──────────────────────────────────────────────────────

#[derive(Default)]
struct RowActions {
    delete: bool,
    create_instance: bool,
}

fn installed_core_row(
    ui: &mut egui::Ui,
    name: &str,
    version: &str,
    hash_short: &str,
    hash_full: &str,
    dependents: &[String],
    striped_bg: bool,
) -> RowActions {
    let mut actions = RowActions::default();
    let has_dependents = !dependents.is_empty();

    let frame = if striped_bg {
        egui::Frame::NONE.fill(ui.visuals().faint_bg_color)
    } else {
        egui::Frame::NONE
    };

    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let actions_width = 60.0;
            let version_width = 80.0;
            let hash_width = 160.0;
            let name_width =
                (ui.available_width() - version_width - hash_width - actions_width - ui.spacing().item_spacing.x * 4.0)
                    .max(80.0);

            ui.add_sized([name_width, ui.available_height()], egui::Label::new(name).truncate());
            ui.add_sized([version_width, ui.available_height()], egui::Label::new(version));
            ui.add_sized([hash_width, ui.available_height()], egui::Label::new(hash_short))
                .on_hover_text(hash_full);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("+").on_hover_text("Create instance").clicked() {
                    actions.create_instance = true;
                }

                if has_dependents {
                    let tooltip = format!("Used by: [ {} ]", dependents.join(", "));
                    let btn = egui::Button::new(egui::RichText::new("\u{1F5D1}").color(egui::Color32::YELLOW));
                    ui.add_enabled(false, btn).on_disabled_hover_text(tooltip);
                } else if ui.button("\u{1F5D1}").on_hover_text("Delete").clicked() {
                    actions.delete = true;
                }
            });
        });
    });

    actions
}

/// Renders a row in the available versions list.
///
/// - Installed items show a checkmark (non-interactive) + optional reinstall.
/// - Downloading items show a progress ring.
/// - Selectable items show a checkbox that toggles selection in `pending_installs`.
fn available_core_row(
    ui: &mut egui::Ui,
    name: &str,
    version: &str,
    size_text: &str,
    is_downloading: bool,
    download_fraction: Option<f32>,
    is_installed: bool,
    is_selected: bool,
    striped_bg: bool,
) -> AvailableRowAction {
    let mut action = AvailableRowAction::None;

    let frame = if striped_bg {
        egui::Frame::NONE.fill(ui.visuals().faint_bg_color)
    } else {
        egui::Frame::NONE
    };

    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let status_width = 30.0;
            let size_width = 80.0;
            let version_width = 80.0;
            let name_width =
                (ui.available_width() - version_width - size_width - status_width - ui.spacing().item_spacing.x * 4.0)
                    .max(80.0);

            // Data columns
            ui.add_sized([name_width, ui.available_height()], egui::Label::new(name).truncate());
            ui.add_sized([version_width, ui.available_height()], egui::Label::new(version));
            ui.add_sized([size_width, ui.available_height()], egui::Label::new(size_text));

            // Status column (rightmost): checkmark / checkbox / progress ring
            if is_downloading {
                ui.add_sized(
                    [status_width, ui.available_height()],
                    ProgressRing::new(download_fraction),
                );
            } else if is_installed {
                ui.add_sized(
                    [status_width, ui.available_height()],
                    egui::Label::new(egui::RichText::new("\u{2714}").color(egui::Color32::GREEN).strong()),
                )
                .on_hover_text("Installed");
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

#[derive(PartialEq)]
enum AvailableRowAction {
    None,
    Select,
    Deselect,
}

// ── Main render ──────────────────────────────────────────────────────

/// Renders the Cores tab content.
pub fn render(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut CoresTabState,
    handle: &WorkerHandle,
    toasts_out: &mut Toasts,
) -> CoresTabAction {
    let mut action = CoresTabAction {
        switch_to_instances_with_core: None,
    };

    // ── Auto-fetch on first visit ────────────────────────────────
    if !state.fetched_once && !state.busy {
        state.fetched_once = true;
        state.busy = true;
        handle.try_send(Command::FetchCoresList {
            search_version: GitHubListOptions { search_version: vec![] },
        });
    }

    let global_busy = state.busy || state.downloads.has_active();

    // ── Toolbar (right-aligned icons) ────────────────────────────
    ui.horizontal(|ui| {
        ui.heading("Cores");

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Validate
            if ui
                .add_enabled(!global_busy, egui::Button::new("\u{2714}"))
                .on_hover_text("Validate cores")
                .clicked()
            {
                state.busy = true;
                handle.try_send(Command::ValidateCores);
            }

            // Refresh
            if ui
                .add_enabled(!global_busy, egui::Button::new("\u{21BB}"))
                .on_hover_text("Fetch from GitHub")
                .clicked()
            {
                state.busy = true;
                handle.try_send(Command::FetchCoresList {
                    search_version: GitHubListOptions { search_version: vec![] },
                });
                toasts::info(toasts_out, "Fetching versions...");
            }

            // Download selected — only shown when there are pending items
            let pending_count = state.pending_installs.len();
            if pending_count > 0 {
                let label = format!("Download ({})", pending_count);
                if ui.button(label).on_hover_text("Download all selected cores").clicked() {
                    let items: Vec<_> = state.pending_installs.drain(..).collect();
                    let requests: Vec<_> = items
                        .into_iter()
                        .map(|item| state.downloads.start(&item, ctx))
                        .collect();
                    handle.try_send(Command::InstallCores { requests });
                }
            }
        });
    });

    ui.separator();

    // ── Delete confirmation modal ────────────────────────────────
    if let Some((ref hash, ref display_name)) = state.confirm_remove.clone() {
        let mut open = true;
        egui::Window::new("Delete core?")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui.ctx(), |ui| {
                ui.label(format!("Are you sure you want to delete \"{}\"?", display_name));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Yes, delete").clicked() {
                        handle.try_send(Command::RemoveCore { hash: hash.clone() });
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

    // ── Installed cores (max 10 rows, full width) ────────────────
    if !state.installed.is_empty() {
        ui.label(egui::RichText::new("Installed").strong().size(14.0));

        // Header
        ui.horizontal(|ui| {
            let actions_width = 60.0;
            let version_width = 80.0;
            let hash_width = 160.0;
            let name_width =
                (ui.available_width() - version_width - hash_width - actions_width - ui.spacing().item_spacing.x * 4.0)
                    .max(80.0);

            ui.add_sized(
                [name_width, ui.available_height()],
                egui::Label::new(egui::RichText::new("Name").strong()),
            );
            ui.add_sized(
                [version_width, ui.available_height()],
                egui::Label::new(egui::RichText::new("Version").strong()),
            );
            ui.add_sized(
                [hash_width, ui.available_height()],
                egui::Label::new(egui::RichText::new("Hash").strong()),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("Actions").strong());
            });
        });

        let row_height = ui.text_style_height(&egui::TextStyle::Body) + 12.0;
        let max_rows = 10;
        let max_height = row_height * max_rows as f32;

        egui::ScrollArea::vertical()
            .id_salt("installed_cores_scroll")
            .max_height(max_height)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                let mut request_remove: Option<(Hash, String)> = None;

                for (idx, (hash, lock_item)) in state.installed.iter().enumerate() {
                    let hash_str = hash.to_string();
                    let short = if hash_str.len() > 20 {
                        format!("{}...", &hash_str[..20])
                    } else {
                        hash_str.clone()
                    };

                    let dependents = state.core_dependents.get(hash).map(|v| v.as_slice()).unwrap_or(&[]);

                    let row_actions = installed_core_row(
                        ui,
                        &lock_item.item.name,
                        &lock_item.item.version,
                        &short,
                        &hash_str,
                        dependents,
                        idx % 2 == 1,
                    );

                    if row_actions.delete && !global_busy {
                        let display = format!("{} v{}", lock_item.item.name, lock_item.item.version);
                        request_remove = Some((hash.clone(), display));
                    }
                    if row_actions.create_instance {
                        action.switch_to_instances_with_core = Some(idx);
                    }
                }

                if let Some(rm) = request_remove {
                    state.confirm_remove = Some(rm);
                }
            });
    }

    // ── Validation results ───────────────────────────────────────
    if !state.validation.is_empty() {
        ui.separator();
        ui.label(egui::RichText::new("Validation results").strong().size(14.0));
        for reason in &state.validation {
            match reason {
                ValidateReason::HashNotMatcher(hash, item) => {
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        format!("Hash mismatch: {} v{} ({})", item.item.name, item.item.version, hash),
                    );
                },
                ValidateReason::NotFound(hash, item) => {
                    ui.colored_label(
                        egui::Color32::RED,
                        format!("Not found: {} v{} ({})", item.item.name, item.item.version, hash),
                    );
                },
            }
        }
    }

    // ── Available versions (fills remaining space, full width) ────
    if !state.available.is_empty() {
        ui.separator();

        ui.label(egui::RichText::new("Available versions").strong().size(14.0));

        // Column headers
        ui.horizontal(|ui| {
            let status_width = 30.0;
            let size_width = 80.0;
            let version_width = 80.0;
            let name_width =
                (ui.available_width() - version_width - size_width - status_width - ui.spacing().item_spacing.x * 4.0)
                    .max(80.0);

            ui.add_sized(
                [name_width, ui.available_height()],
                egui::Label::new(egui::RichText::new("Name").strong()),
            );
            ui.add_sized(
                [version_width, ui.available_height()],
                egui::Label::new(egui::RichText::new("Version").strong()),
            );
            ui.add_sized(
                [size_width, ui.available_height()],
                egui::Label::new(egui::RichText::new("Size").strong()),
            );
            // Status header (empty — checkboxes/icons are self-explanatory)
            ui.add_sized([status_width, ui.available_height()], egui::Label::new(""));
        });

        let installed_names: HashSet<&str> = state.installed.iter().map(|(_, li)| li.item.name.as_str()).collect();

        egui::ScrollArea::vertical()
            .id_salt("available_cores_scroll")
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                for (idx, item) in state.available.iter().enumerate() {
                    let is_downloading = state.downloads.is_active(item);
                    let is_installed = installed_names.contains(item.name.as_str());
                    let is_selected = state
                        .pending_installs
                        .iter()
                        .any(|p| p.name == item.name && p.version == item.version);

                    let row_action = available_core_row(
                        ui,
                        &item.name,
                        &item.version,
                        &crate::app::format_size(item.size),
                        is_downloading,
                        if is_downloading {
                            state.downloads.fraction(item)
                        } else {
                            None
                        },
                        is_installed,
                        is_selected,
                        idx % 2 == 1,
                    );

                    match row_action {
                        AvailableRowAction::Select => {
                            // Add to pending if not already there
                            if !is_selected {
                                state.pending_installs.push(item.clone());
                            }
                        },
                        AvailableRowAction::Deselect => {
                            // Remove from pending
                            state
                                .pending_installs
                                .retain(|p| !(p.name == item.name && p.version == item.version));
                        },
                        AvailableRowAction::None => {},
                    }
                }
            });
    } else if state.busy {
        ui.separator();
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Loading available versions...");
        });
    }

    action
}
