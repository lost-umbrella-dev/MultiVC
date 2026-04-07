//! Главный модуль GUI-приложения на egui.
//!
//! [`App`] — минимальный оркестратор: маршрутизирует [`Event`]-ы от worker'а
//! в [`UiState`], делегирует отрисовку [`crate::ui::render_ui`].

use eframe::egui;
use egui_toast::Toasts;

use composer::error::ComposerError;
use composer::message::{Command, CoresInstalledResult, Event};
use composer::worker::WorkerHandle;

use crate::ui::Tab;
use crate::ui::lang::{self, Lang};
use crate::ui::state::UiState;
use crate::ui::toasts;

// ── App ──────────────────────────────────────────────────────────────

/// Главная структура GUI-приложения.
pub struct App {
    pub handle: WorkerHandle,

    #[allow(dead_code)]
    runtime: tokio::runtime::Runtime,

    pub current_tab: Tab,
    pub state: UiState,
}

impl App {
    pub fn new(
        handle: WorkerHandle,
        runtime: tokio::runtime::Runtime,
    ) -> Self {
        handle.try_send(Command::GetCoresItems);
        handle.try_send(Command::GetInstancesItems);
        handle.try_send(Command::ValidateCores);

        let mut state = UiState::default();
        state.cores.busy = true;

        Self {
            handle,
            runtime,
            current_tab: Tab::default(),
            state,
        }
    }

    pub fn drain_and_apply_events(
        state: &mut UiState,
        handle: &mut WorkerHandle,
        toasts: &mut Toasts,
    ) {
        let events = handle.drain_events();
        let lang = state.settings.lock.language;
        for event in events {
            Self::apply_event(state, handle, event, toasts, lang);
        }
    }

    fn apply_event(
        state: &mut UiState,
        handle: &WorkerHandle,
        event: Event,
        toasts: &mut Toasts,
        lang: Lang,
    ) {
        match event {
            Event::Saved(Ok(())) => {
                state.cores.busy = false;
                state.instances.busy = false;
                toasts::success(toasts, lang::t("toast.locks_saved", lang));
            },
            Event::Saved(Err(e)) => {
                state.cores.busy = false;
                state.instances.busy = false;
                toasts::error(toasts, format!("{}: {e}", lang::t("toast.save_err", lang)));
            },

            Event::CoresSaved(Ok(())) => {
                state.cores.busy = false;
                toasts::success(toasts, lang::t("toast.cores_saved", lang));
            },
            Event::CoresSaved(Err(e)) => {
                state.cores.busy = false;
                toasts::error(toasts, format!("{}: {e}", lang::t("toast.cores_save_err", lang)));
            },

            Event::InstancesSaved(Ok(())) => {
                state.instances.busy = false;
                toasts::success(toasts, lang::t("toast.instances_saved", lang));
            },
            Event::InstancesSaved(Err(e)) => {
                state.instances.busy = false;
                toasts::error(
                    toasts,
                    format!("{}: {e}", lang::t("toast.instances_save_err", lang)),
                );
            },

            Event::CoresInstalled(CoresInstalledResult {
                successful,
                failed,
            }) => {
                let installed_keys: Vec<String> = state
                    .cores
                    .installed
                    .iter()
                    .map(|(_, li)| format!("{}/{}", li.item.name, li.item.version))
                    .collect();
                state.cores.downloads.complete_installed(&installed_keys);

                if failed.is_empty() {
                    toasts::success(
                        toasts,
                        format!("{}: {successful}", lang::t("toast.cores_installed", lang)),
                    );
                } else {
                    let errors: Vec<String> = failed
                        .iter()
                        .map(|(item, err)| format!("  {} {}: {err}", item.name, item.version))
                        .collect();
                    toasts::error(
                        toasts,
                        format!(
                            "{}: {successful}, {}:\n{}",
                            lang::t("toast.cores_installed", lang),
                            lang::t("toast.cores_install_err", lang),
                            errors.join("\n")
                        ),
                    );
                }

                // Re-validate after reinstall to refresh validation state
                if successful > 0 {
                    handle.try_send(Command::ValidateCores);
                }
            },

            Event::CoresValidated(Ok(reasons)) => {
                state.cores.busy = false;
                if reasons.is_empty() {
                    toasts::success(toasts, lang::t("toast.cores_valid", lang));
                } else {
                    toasts::warning(
                        toasts,
                        format!("{}: {}", lang::t("toast.cores_issues", lang), reasons.len()),
                    );
                }
                state.cores.validation = reasons;
            },
            Event::CoresValidated(Err(e)) => {
                state.cores.busy = false;
                toasts::error(toasts, format!("{}: {e}", lang::t("toast.cores_valid_err", lang)));
            },

            Event::InstancesValidated(Ok(reasons)) => {
                state.instances.busy = false;
                if reasons.is_empty() {
                    toasts::success(toasts, lang::t("toast.instances_valid", lang));
                } else {
                    toasts::warning(
                        toasts,
                        format!(
                            "{}: {}",
                            lang::t("toast.instances_issues", lang),
                            reasons.len()
                        ),
                    );
                }
                state.instances.validation = reasons;
            },
            Event::InstancesValidated(Err(e)) => {
                state.instances.busy = false;
                toasts::error(
                    toasts,
                    format!("{}: {e}", lang::t("toast.instances_valid_err", lang)),
                );
            },

            Event::CoreRemoved {
                hash,
                item,
            } => {
                if let Some(lock_item) = &item {
                    state.cores.installed.retain(|(h, _)| h != &hash);
                    toasts::success(
                        toasts,
                        format!(
                            "{}: {} {}",
                            lang::t("toast.core_removed", lang),
                            lock_item.item.name,
                            lock_item.item.version
                        ),
                    );
                } else {
                    toasts::warning(
                        toasts,
                        format!("{}: {hash}", lang::t("toast.core_not_found", lang)),
                    );
                }
            },

            Event::InstanceRemoved {
                name,
                item,
            } => {
                state.instances.busy_instances.remove(&name);
                if item.is_some() {
                    state.instances.installed.retain(|(n, _)| n != &name);
                    toasts::success(
                        toasts,
                        format!("{}: {name}", lang::t("toast.instance_removed", lang)),
                    );
                } else {
                    toasts::warning(
                        toasts,
                        format!("{}: {name}", lang::t("toast.instance_not_found", lang)),
                    );
                }
            },

            Event::InstanceCreated(Ok(name)) => {
                toasts::success(
                    toasts,
                    format!("{}: {name}", lang::t("toast.instance_created", lang)),
                );
            },
            Event::InstanceCreated(Err(e)) => {
                toasts::error(
                    toasts,
                    format!("{}: {e}", lang::t("toast.instance_create_err", lang)),
                );
            },

            Event::InstanceInfo(Ok(instance)) => {
                state.instances.viewing = Some(instance);
            },
            Event::InstanceInfo(Err(e)) => {
                toasts::error(
                    toasts,
                    format!("{}: {e}", lang::t("toast.instance_info_err", lang)),
                );
            },

            Event::InstanceEdited(Ok(name)) => {
                toasts::success(
                    toasts,
                    format!("{}: {name}", lang::t("toast.instance_updated", lang)),
                );
            },
            Event::InstanceEdited(Err(e)) => {
                toasts::error(
                    toasts,
                    format!("{}: {e}", lang::t("toast.instance_update_err", lang)),
                );
            },

            Event::InstanceLaunched {
                name,
                result: Ok(pid),
            } => {
                state.instances.running_instances.insert(name.clone(), pid);
                if let Some((_n, meta)) =
                    state.instances.installed.iter_mut().find(|(n, _)| n == &name)
                {
                    meta.last_launch = Some(chrono::Utc::now());
                }
                toasts::success(
                    toasts,
                    format!("{}: {name} (PID {pid})", lang::t("toast.instance_launched", lang)),
                );
            },
            Event::InstanceLaunched {
                name,
                result: Err(e),
            } => {
                toasts::error(
                    toasts,
                    format!("{} ({name}): {e}", lang::t("toast.instance_launch_err", lang)),
                );
            },

            Event::InstanceStopped {
                name,
                status,
            } => {
                state.instances.running_instances.remove(&name);
                match status {
                    Some(0) => toasts::info(
                        toasts,
                        format!("{}: {name}", lang::t("toast.instance_stopped", lang)),
                    ),
                    Some(code) => toasts::warning(
                        toasts,
                        format!(
                            "{}: {name} ({} {code})",
                            lang::t("toast.instance_stopped", lang),
                            lang::t("toast.instance_stopped_code", lang)
                        ),
                    ),
                    None => toasts::warning(
                        toasts,
                        format!(
                            "{}: {name} ({})",
                            lang::t("toast.instance_stopped", lang),
                            lang::t("toast.instance_stopped_killed", lang)
                        ),
                    ),
                }
            },

            Event::CoresFetched(Ok(items)) => {
                state.cores.busy = false;
                toasts::info(
                    toasts,
                    format!("{}: {}", lang::t("toast.cores_fetched", lang), items.len()),
                );
                state.cores.available = items;
            },
            Event::CoresFetched(Err(e)) => {
                state.cores.busy = false;
                toasts::error(toasts, format!("{}: {e}", lang::t("toast.fetch_err", lang)));
            },

            Event::CoreFetched(Ok(Some(item))) => {
                toasts::info(
                    toasts,
                    format!(
                        "{}: {} {}",
                        lang::t("toast.core_found", lang),
                        item.name,
                        item.version
                    ),
                );
            },
            Event::CoreFetched(Ok(None)) => {
                toasts::warning(toasts, lang::t("toast.version_not_found", lang));
            },
            Event::CoreFetched(Err(e)) => {
                toasts::error(
                    toasts,
                    format!("{}: {e}", lang::t("toast.fetch_version_err", lang)),
                );
            },

            Event::CoresItems(items) => {
                state.cores.installed = items;
            },
            Event::InstancesItems(items, core_dependents) => {
                state.cores.core_dependents = core_dependents;
                state.instances.installed = items;
            },

            Event::InstanceDirSize {
                name,
                bytes,
            } => {
                if let Some(ref mut panel) = state.instances.instance_panel
                    && panel.name == name
                {
                    panel.dir_size = Some(bytes);
                }
            },

            Event::Error(ref e) => match e {
                ComposerError::CoreInUse {
                    hash: _,
                    dependents,
                } => {
                    let names = dependents.join(", ");
                    toasts::error(
                        toasts,
                        format!("{}: {names}", lang::t("toast.core_in_use", lang)),
                    );
                },
                _ => {
                    toasts::error(
                        toasts,
                        format!("{}: {e}", lang::t("toast.fatal_err", lang)),
                    );
                },
            },
            Event::ShutdownComplete => {
                toasts::info(toasts, lang::t("toast.worker_stopped", lang));
            },
        }
    }

    /// Calls render_ui statically.
    fn call_render_ui(
        &mut self,
        ui: &mut egui::Ui,
    ) {
        let mut args = crate::ui::RenderArgs {
            current_tab: &mut self.current_tab,
            state: &mut self.state,
            handle: &mut self.handle,
        };
        crate::ui::render_ui(ui, &mut args);
    }
}

// ── eframe::App ──────────────────────────────────────────────────────

impl eframe::App for App {
    fn logic(
        &mut self,
        _ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
    }

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        _frame: &mut eframe::Frame,
    ) {
        // Debug hotkeys
        #[cfg(debug_assertions)]
        {
            let ctx = ui.ctx();
            if ctx.input(|i| i.key_pressed(egui::Key::F12)) {
                let v = ctx.debug_on_hover();
                ctx.set_debug_on_hover(!v);
            }
            if ctx.input(|i| i.key_pressed(egui::Key::F11)) {
                ctx.global_style_mut(|s| s.debug.show_widget_hits = !s.debug.show_widget_hits);
            }
            if ctx.input(|i| i.key_pressed(egui::Key::F10)) {
                let id = egui::Id::new("__debug_inspection");
                let open = ctx.data_mut(|d| {
                    let v = d.get_temp::<bool>(id).unwrap_or(false);
                    d.insert_temp(id, !v);
                    !v
                });
                if open {
                    ctx.set_debug_on_hover(false);
                }
            }
            {
                let id = egui::Id::new("__debug_inspection");
                let open = ctx.data(|d| d.get_temp::<bool>(id).unwrap_or(false));
                if open {
                    let mut still_open = true;
                    egui::Window::new("Inspector")
                        .open(&mut still_open)
                        .show(ctx, |ui| ctx.inspection_ui(ui));
                    if !still_open {
                        ctx.data_mut(|d| d.insert_temp(id, false));
                    }
                }
            }
        }

        // Render UI (includes event drain + toasts)
        self.call_render_ui(ui);
    }

    fn on_exit(
        &mut self,
        _gl: Option<&eframe::glow::Context>,
    ) {
        tracing::info!("sending Shutdown to worker");
        self.handle.try_send(Command::Shutdown);
        let _ = self.state.settings.lock.save();
    }
}
