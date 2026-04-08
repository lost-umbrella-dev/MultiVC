//! Cores tab view.

mod rows;
use rows::{AvailableRowAction, available_core_row, installed_core_row};

use std::collections::HashSet;

use eframe::egui;
use egui_toast::Toasts;

use clients::github::GitHubListOptions;
use clients::hash::Hash;
use clients::item::{BUILD_AVAILABLE, RELEASE_AVAILABLE};
use composer::lock::ValidateReason;
use composer::message::Command;
use composer::worker::WorkerHandle;

use crate::ui::icons;
use crate::ui::lang::{self, Lang};
use crate::ui::settings::InstallMode;
use crate::ui::state::CoresTabState;
use crate::ui::toasts;
use crate::ui::widgets::{confirm_dialog, icon_button, tab_toolbar};

// ── Helper functions ─────────────────────────────────────────────────

/// Результат рендера — действия, которые должен обработать App.
pub struct CoresTabAction {
    /// Переключить на вкладку Instances с предзаполненным ядром.
    pub switch_to_instances_with_core: Option<usize>,
}

// ── Main render ──────────────────────────────────────────────────────

/// Renders the Cores tab content.
pub fn render(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut CoresTabState,
    handle: &WorkerHandle,
    toasts_out: &mut Toasts,
    install_mode: InstallMode,
    lang: Lang,
) -> CoresTabAction {
    let mut action = CoresTabAction {
        switch_to_instances_with_core: None,
    };

    // ── Auto-fetch on first visit ────────────────────────────────
    if !state.fetched_once && !state.busy {
        state.fetched_once = true;
        state.busy = true;
        handle.try_send(Command::FetchCoresList {
            search_version: GitHubListOptions {
                search_version: vec![],
            },
        });
    }

    let global_busy =
        state.busy || state.downloads.has_active() || state.builds.has_active() || state.deps_modal.is_some();

    // ── Toolbar (right-aligned icons) ────────────────────────────
    tab_toolbar(ui, lang::t("tab.cores", lang), |ui| {
        // Validate
        if ui
            .add_enabled(!global_busy, icon_button(icons::ICON_VALIDATE))
            .on_hover_text(lang::t("tip.validate_cores", lang))
            .clicked()
        {
            state.busy = true;
            handle.try_send(Command::ValidateCores);
            toasts::info(toasts_out, lang::t("status.validating", lang));
        }

        // Refresh
        if ui
            .add_enabled(!global_busy, icon_button(icons::ICON_REFRESH))
            .on_hover_text(lang::t("tip.fetch_github", lang))
            .clicked()
        {
            state.busy = true;
            handle.try_send(Command::FetchCoresList {
                search_version: GitHubListOptions {
                    search_version: vec![],
                },
            });
            toasts::info(toasts_out, lang::t("status.fetching", lang));
        }

        // Download selected — only shown when there are pending items
        let pending_count = state.pending_installs.len();
        if pending_count > 0 {
            let label = format!("{} ({})", lang::t("action.download", lang), pending_count);
            if ui.button(label).on_hover_text(lang::t("tip.download_selected", lang)).clicked() {
                let items: Vec<_> = state.pending_installs.drain(..).collect();
                let requests: Vec<_> =
                    items.into_iter().map(|item| state.downloads.start(&item, ctx)).collect();
                handle.try_send(Command::InstallCores {
                    requests,
                });
            }
        }

        // Build selected — only shown when there are pending build items
        let pending_build_count = state.pending_builds.len();
        if pending_build_count > 0 {
            let label = format!("{} ({})", lang::t("action.build", lang), pending_build_count);
            if ui
                .add_enabled(!global_busy, egui::Button::new(label))
                .on_hover_text(lang::t("tip.build", lang))
                .clicked()
            {
                state.busy = true;
                handle.try_send(Command::CheckBuildDeps);
            }
        }
    });

    // ── Delete confirmation modal ────────────────────────────────
    if let Some((ref hash, ref display_name)) = state.confirm_remove.clone() {
        let msg = format!("{} \"{}\"?", lang::t("confirm.delete_core", lang), display_name);
        if let Some(confirmed) = confirm_dialog(
            ui.ctx(),
            lang::t("confirm.delete_core", lang),
            &msg,
            lang::t("action.yes_delete", lang),
            lang::t("action.cancel", lang),
        ) {
            if confirmed {
                handle.try_send(Command::RemoveCore {
                    hash: hash.clone(),
                });
            }
            state.confirm_remove = None;
        }
    }

    // ── Deps installation confirmation modal ────────────────────
    if let Some(ref deps) = state.deps_modal {
        let mut close = false;
        egui::Window::new(lang::t("modal.deps_title", lang))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.label(lang::t("modal.deps_message", lang));
                ui.add_space(4.0);
                for dep in &deps.missing {
                    ui.label(format!("  \u{2022} {dep}"));
                }
                if let Some(ref cmd) = deps.install_command {
                    ui.add_space(4.0);
                    ui.label(lang::t("modal.deps_command", lang));
                    ui.code(cmd);
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(lang::t("modal.deps_confirm", lang)).clicked() {
                        state.busy = true;
                        handle.try_send(Command::InstallBuildDeps);
                        close = true;
                    }
                    if ui.button(lang::t("action.cancel", lang)).clicked() {
                        close = true;
                    }
                });
            });
        if close {
            state.deps_modal = None;
        }
    }

    // ── Installed cores (max 10 rows, full width) ────────────────
    if !state.installed.is_empty() {
        ui.separator();
        ui.label(egui::RichText::new(lang::t("section.installed", lang)).strong().size(14.0));

        // Header
        ui.horizontal(|ui| {
            let actions_width = 60.0;
            let version_width = 80.0;
            let type_width = 60.0;
            let hash_width = (ui.available_width()
                - version_width
                - type_width
                - actions_width
                - ui.spacing().item_spacing.x * 4.0)
                .max(80.0);
            let row_height = ui.text_style_height(&egui::TextStyle::Body);

            ui.add_sized(
                [version_width, row_height],
                egui::Label::new(egui::RichText::new(lang::t("col.version", lang)).strong()),
            );

            ui.add_sized(
                [type_width, row_height],
                egui::Label::new(egui::RichText::new(lang::t("col.type", lang)).strong()),
            );

            // Hash header
            ui.add_sized(
                [hash_width, row_height],
                egui::Label::new(egui::RichText::new(lang::t("col.hash", lang)).strong()),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(lang::t("col.actions", lang)).strong());
            });
        });

        let row_height = ui.text_style_height(&egui::TextStyle::Body) + 12.0;
        let max_rows = 10;
        let max_height = row_height * max_rows as f32;

        // Build set of invalid core hashes from validation results
        let invalid_hashes: HashSet<&Hash> = state
            .validation
            .iter()
            .map(|r| match r {
                ValidateReason::HashNotMatcher(h, _) | ValidateReason::NotFound(h, _) => h,
            })
            .collect();

        egui::ScrollArea::vertical().id_salt("installed_cores_scroll").max_height(max_height).show(
            ui,
            |ui| {
                ui.set_width(ui.available_width());

                let mut request_remove: Option<(Hash, String)> = None;

                for (row_idx, (hash, lock_item)) in state.installed.iter().enumerate() {
                    let hash_str = hash.to_string();
                    let short = if hash_str.len() > 20 {
                        format!("{}...", &hash_str[..20])
                    } else {
                        hash_str.clone()
                    };

                    let dependents =
                        state.core_dependents.get(hash).map(|v| v.as_slice()).unwrap_or(&[]);
                    let is_invalid = invalid_hashes.contains(hash);
                    let is_downloading = state.downloads.is_active(&lock_item.item);
                    let download_fraction = if is_downloading {
                        state.downloads.fraction(&lock_item.item)
                    } else {
                        None
                    };

                    let row_actions = installed_core_row(
                        ui,
                        &lock_item.item.version.to_string(),
                        &short,
                        &hash_str,
                        dependents,
                        is_invalid,
                        is_downloading,
                        download_fraction,
                        row_idx % 2 == 1,
                        lock_item.origin,
                        lang,
                    );

                    if row_actions.delete && !global_busy {
                        let display = format!("{} {}", lock_item.item.name, lock_item.item.version);
                        request_remove = Some((hash.clone(), display));
                    }
                    if row_actions.redownload {
                        let request = state.downloads.start(&lock_item.item, ctx);
                        handle.try_send(Command::InstallCores {
                            requests: vec![request],
                        });
                    }
                    if row_actions.create_instance {
                        action.switch_to_instances_with_core = Some(row_idx);
                    }
                }

                if let Some(rm) = request_remove {
                    state.confirm_remove = Some(rm);
                }
            },
        );
    }

    // ── Validation results ───────────────────────────────────────
    if !state.validation.is_empty() {
        ui.separator();
        ui.label(egui::RichText::new(lang::t("section.validation", lang)).strong().size(14.0));
        for reason in &state.validation {
            match reason {
                ValidateReason::HashNotMatcher(hash, item) => {
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        format!(
                            "{}: {} {} ({})",
                            lang::t("validation.hash_mismatch", lang),
                            item.item.name,
                            item.item.version,
                            hash
                        ),
                    );
                },
                ValidateReason::NotFound(hash, item) => {
                    ui.colored_label(
                        egui::Color32::RED,
                        format!(
                            "{}: {} {} ({})",
                            lang::t("validation.not_found", lang),
                            item.item.name,
                            item.item.version,
                            hash
                        ),
                    );
                },
            }
        }
    }

    // ── Available versions (fills remaining space, full width) ────
    if !state.available.is_empty() {
        ui.separator();

        ui.label(egui::RichText::new(lang::t("section.available", lang)).strong().size(14.0));

        let show_release = install_mode.has_release() && RELEASE_AVAILABLE;
        let show_build = install_mode.has_build() && BUILD_AVAILABLE;

        // Column headers
        ui.horizontal(|ui| {
            let rb_width = 30.0;
            let size_width = 80.0;

            let trailing = if show_build { rb_width } else { 0.0 }
                + if show_release { rb_width } else { 0.0 };
            let spacing_count =
                1.0 + if show_build { 1.0 } else { 0.0 } + if show_release { 1.0 } else { 0.0 };

            let version_col_width = (ui.available_width()
                - size_width
                - trailing
                - ui.spacing().item_spacing.x * spacing_count)
                .max(80.0);
            let row_height = ui.text_style_height(&egui::TextStyle::Body);

            ui.add_sized(
                [version_col_width, row_height],
                egui::Label::new(egui::RichText::new(lang::t("col.version", lang)).strong()),
            );

            // Size header
            ui.add_sized(
                [size_width, row_height],
                egui::Label::new(egui::RichText::new(lang::t("col.size", lang)).strong()),
            );

            if show_release {
                ui.add_sized(
                    [rb_width, row_height],
                    egui::Label::new(egui::RichText::new("R").strong()),
                )
                .on_hover_text(lang::t("tip.release", lang));
            }

            if show_build {
                ui.add_sized(
                    [rb_width, row_height],
                    egui::Label::new(egui::RichText::new("B").strong()),
                )
                .on_hover_text(lang::t("tip.build", lang));
            }
        });

        // Build sets of installed names by origin
        let installed_release: HashSet<&str> = state
            .installed
            .iter()
            .filter(|(_, li)| li.origin == clients::item::CoreOrigin::Release)
            .map(|(_, li)| li.item.name.as_str())
            .collect();
        let installed_build: HashSet<&str> = state
            .installed
            .iter()
            .filter(|(_, li)| li.origin == clients::item::CoreOrigin::Build)
            .map(|(_, li)| li.item.name.as_str())
            .collect();

        egui::ScrollArea::vertical().id_salt("available_cores_scroll").show(ui, |ui| {
            ui.set_width(ui.available_width());

            for (row_idx, item) in state.available.iter().enumerate() {
                let is_downloading = state.downloads.is_active(item);
                let is_building = state.builds.is_active(item);
                let build_progress = state.builds.progress(item);
                let is_installed_release = installed_release.contains(item.name.as_str());
                let is_installed_build = installed_build.contains(item.name.as_str());
                let is_selected_download = state
                    .pending_installs
                    .iter()
                    .any(|p| p.name == item.name && p.version == item.version);
                let is_selected_build = state
                    .pending_builds
                    .iter()
                    .any(|p| p.name == item.name && p.version == item.version);
                let has_source = state.source_versions.contains(&item.version.to_string());

                let row_action = available_core_row(
                    ui,
                    &item.version.to_string(),
                    &crate::ui::format_size(item.size),
                    is_downloading,
                    if is_downloading {
                        state.downloads.fraction(item)
                    } else {
                        None
                    },
                    is_building,
                    build_progress,
                    is_installed_release,
                    is_installed_build,
                    is_selected_download,
                    is_selected_build,
                    show_release,
                    show_build,
                    has_source,
                    row_idx % 2 == 1,
                    lang,
                );

                match row_action {
                    AvailableRowAction::SelectDownload => {
                        if !is_selected_download {
                            state.pending_installs.push(item.clone());
                        }
                    },
                    AvailableRowAction::DeselectDownload => {
                        state
                            .pending_installs
                            .retain(|p| !(p.name == item.name && p.version == item.version));
                    },
                    AvailableRowAction::SelectBuild => {
                        if !is_selected_build {
                            state.pending_builds.push(item.clone());
                        }
                    },
                    AvailableRowAction::DeselectBuild => {
                        state
                            .pending_builds
                            .retain(|p| !(p.name == item.name && p.version == item.version));
                    },
                    AvailableRowAction::DeleteSource => {
                        handle.try_send(Command::RemoveSource {
                            version: item.version.clone(),
                        });
                    },
                    AvailableRowAction::None => {},
                }
            }
        });
    } else if state.busy {
        ui.separator();
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(lang::t("status.loading", lang));
        });
    }

    action
}
