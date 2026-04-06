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

        Self {
            handle,
            runtime,
            current_tab: Tab::default(),
            state: UiState::default(),
        }
    }

    fn drain_and_apply_events(
        &mut self,
        toasts: &mut Toasts,
    ) {
        let events = self.handle.drain_events();
        for event in events {
            self.apply_event(event, toasts);
        }
    }

    fn apply_event(
        &mut self,
        event: Event,
        toasts: &mut Toasts,
    ) {
        match event {
            Event::Saved(Ok(())) => {
                self.state.cores.busy = false;
                self.state.instances.busy = false;
                toasts::success(toasts, "All lock files saved");
            },
            Event::Saved(Err(e)) => {
                self.state.cores.busy = false;
                self.state.instances.busy = false;
                toasts::error(toasts, format!("Save error: {e}"));
            },

            Event::CoresSaved(Ok(())) => {
                self.state.cores.busy = false;
                toasts::success(toasts, "Cores lock saved");
            },
            Event::CoresSaved(Err(e)) => {
                self.state.cores.busy = false;
                toasts::error(toasts, format!("Cores save error: {e}"));
            },

            Event::InstancesSaved(Ok(())) => {
                self.state.instances.busy = false;
                toasts::success(toasts, "Instances lock saved");
            },
            Event::InstancesSaved(Err(e)) => {
                self.state.instances.busy = false;
                toasts::error(toasts, format!("Instances save error: {e}"));
            },

            Event::CoresInstalled(CoresInstalledResult {
                successful,
                failed,
            }) => {
                let installed_keys: Vec<String> = self
                    .state
                    .cores
                    .installed
                    .iter()
                    .map(|(_, li)| format!("{}/{}", li.item.name, li.item.version))
                    .collect();
                self.state.cores.downloads.complete_installed(&installed_keys);

                if failed.is_empty() {
                    toasts::success(toasts, format!("Installed cores: {successful}"));
                } else {
                    let errors: Vec<String> = failed
                        .iter()
                        .map(|(item, err)| format!("  {} {}: {err}", item.name, item.version))
                        .collect();
                    toasts::error(
                        toasts,
                        format!("Installed: {successful}, errors:\n{}", errors.join("\n")),
                    );
                }
            },

            Event::CoresValidated(Ok(reasons)) => {
                self.state.cores.busy = false;
                if reasons.is_empty() {
                    toasts::success(toasts, "Cores: all valid");
                } else {
                    toasts::warning(toasts, format!("Cores: {} issue(s) found", reasons.len()));
                }
                self.state.cores.validation = reasons;
            },
            Event::CoresValidated(Err(e)) => {
                self.state.cores.busy = false;
                toasts::error(toasts, format!("Cores validation error: {e}"));
            },

            Event::InstancesValidated(Ok(reasons)) => {
                self.state.instances.busy = false;
                if reasons.is_empty() {
                    toasts::success(toasts, "Instances: all valid");
                } else {
                    toasts::warning(toasts, format!("Instances: {} issue(s) found", reasons.len()));
                }
                self.state.instances.validation = reasons;
            },
            Event::InstancesValidated(Err(e)) => {
                self.state.instances.busy = false;
                toasts::error(toasts, format!("Instances validation error: {e}"));
            },

            Event::CoreRemoved {
                hash,
                item,
            } => {
                if let Some(lock_item) = &item {
                    self.state.cores.installed.retain(|(h, _)| h != &hash);
                    toasts::success(
                        toasts,
                        format!("Core removed: {} {}", lock_item.item.name, lock_item.item.version),
                    );
                } else {
                    toasts::warning(toasts, format!("Core not found: {hash}"));
                }
            },

            Event::InstanceRemoved {
                name,
                item,
            } => {
                self.state.instances.busy_instances.remove(&name);
                if item.is_some() {
                    self.state.instances.installed.retain(|(n, _)| n != &name);
                    toasts::success(toasts, format!("Instance removed: {name}"));
                } else {
                    toasts::warning(toasts, format!("Instance not found: {name}"));
                }
            },

            Event::InstanceCreated(Ok(name)) => {
                toasts::success(toasts, format!("Instance created: {name}"));
            },
            Event::InstanceCreated(Err(e)) => {
                toasts::error(toasts, format!("Instance creation error: {e}"));
            },

            Event::InstanceInfo(Ok(instance)) => {
                self.state.instances.viewing = Some(instance);
            },
            Event::InstanceInfo(Err(e)) => {
                toasts::error(toasts, format!("Instance info error: {e}"));
            },

            Event::InstanceEdited(Ok(name)) => {
                toasts::success(toasts, format!("Instance updated: {name}"));
            },
            Event::InstanceEdited(Err(e)) => {
                toasts::error(toasts, format!("Instance update error: {e}"));
            },

            Event::InstanceLaunched {
                name,
                result: Ok(pid),
            } => {
                self.state.instances.running_instances.insert(name.clone(), pid);
                if let Some((_n, meta)) =
                    self.state.instances.installed.iter_mut().find(|(n, _)| n == &name)
                {
                    meta.last_launch = Some(chrono::Utc::now());
                }
                toasts::success(toasts, format!("Launched: {name} (PID {pid})"));
            },
            Event::InstanceLaunched {
                name,
                result: Err(e),
            } => {
                toasts::error(toasts, format!("Launch failed ({name}): {e}"));
            },

            Event::InstanceStopped {
                name,
                status,
            } => {
                self.state.instances.running_instances.remove(&name);
                match status {
                    Some(0) => toasts::info(toasts, format!("Instance stopped: {name}")),
                    Some(code) => toasts::warning(
                        toasts,
                        format!("Instance stopped: {name} (exit code {code})"),
                    ),
                    None => toasts::warning(toasts, format!("Instance stopped: {name} (killed)")),
                }
            },

            Event::CoresFetched(Ok(items)) => {
                self.state.cores.busy = false;
                toasts::info(toasts, format!("Available versions: {}", items.len()));
                self.state.cores.available = items;
            },
            Event::CoresFetched(Err(e)) => {
                self.state.cores.busy = false;
                toasts::error(toasts, format!("Fetch error: {e}"));
            },

            Event::CoreFetched(Ok(Some(item))) => {
                toasts::info(toasts, format!("Found: {} {}", item.name, item.version));
            },
            Event::CoreFetched(Ok(None)) => {
                toasts::warning(toasts, "Version not found");
            },
            Event::CoreFetched(Err(e)) => {
                toasts::error(toasts, format!("Fetch version error: {e}"));
            },

            Event::CoresItems(items) => {
                self.state.cores.installed = items;
            },
            Event::InstancesItems(items, core_dependents) => {
                self.state.cores.core_dependents = core_dependents;
                self.state.instances.installed = items;
            },

            Event::InstanceDirSize {
                name,
                bytes,
            } => {
                if let Some(ref mut panel) = self.state.instances.instance_panel
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
                    toasts::error(toasts, format!("Cannot delete: used by instance(s): {names}"));
                },
                _ => {
                    toasts::error(toasts, format!("Fatal error: {e}"));
                },
            },
            Event::ShutdownComplete => {
                toasts::info(toasts, "Background worker stopped");
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
            handle: &self.handle,
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

        // Drain events
        let mut toasts_instance = crate::ui::toasts::create_toasts();
        self.drain_and_apply_events(&mut toasts_instance);

        // Render UI
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
