//! Главный модуль GUI-приложения на egui.
//!
//! [`App`] — минимальный оркестратор: маршрутизирует [`Event`]-ы от worker'а
//! в [`UiState`], делегирует отрисовку модулям [`views`], а нотификации
//! показывает через [`egui_toast`].

use eframe::egui;
use egui_toast::Toasts;

use composer::error::ComposerError;
use composer::message::{Command, CoresInstalledResult, Event};
use composer::worker::WorkerHandle;

use crate::state::{InstanceForm, UiState};
use crate::toasts;
use crate::views;

// ── Навигация ────────────────────────────────────────────────────────

/// Вкладки левой панели навигации.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Cores,
    Instances,
}

// ── App ──────────────────────────────────────────────────────────────

/// Главная структура GUI-приложения.
///
/// Хранит [`WorkerHandle`] для отправки команд / получения событий,
/// tokio runtime (для возможных ad-hoc async вызовов) и всё UI-состояние.
pub struct App {
    /// Хэндл к фоновому [`ComposerWorker`](composer::worker::ComposerWorker).
    pub handle: WorkerHandle,

    /// Tokio runtime — нужен для `block_on` при инициализации
    /// и как владелец spawned worker task.
    #[allow(dead_code)]
    runtime: tokio::runtime::Runtime,

    /// Текущая вкладка навигации.
    pub current_tab: Tab,

    /// Кэшированное UI-состояние.
    pub state: UiState,
}

impl App {
    /// Создаёт новое GUI-приложение.
    ///
    /// `handle` — канал к фоновому worker'у.
    /// `runtime` — tokio runtime, в котором крутится worker.
    pub fn new(handle: WorkerHandle, runtime: tokio::runtime::Runtime) -> Self {
        // Запрашиваем начальный снимок данных из lock-файлов
        handle.try_send(Command::GetCoresItems);
        handle.try_send(Command::GetInstancesItems);

        Self {
            handle,
            runtime,
            current_tab: Tab::default(),
            state: UiState::default(),
        }
    }

    // ── Event handling ───────────────────────────────────────────

    /// Обрабатывает все накопившиеся [`Event`]-ы от worker'а.
    ///
    /// Вызывается в начале каждого кадра `update()`.
    /// Возвращает toast-уведомления для отображения.
    fn drain_and_apply_events(&mut self, toasts: &mut Toasts) {
        let events = self.handle.drain_events();
        for event in events {
            self.apply_event(event, toasts);
        }
    }

    /// Применяет один [`Event`] к UI-состоянию и генерирует toast.
    fn apply_event(&mut self, event: Event, toasts: &mut Toasts) {
        match event {
            // ── Persistence ──────────────────────────────────────
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

            // ── Install ──────────────────────────────────────────
            Event::CoresInstalled(CoresInstalledResult { successful, failed }) => {
                // Remove only bridges for items now in installed list (not all — other downloads may be queued)
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
                        .map(|(item, err)| format!("  {} v{}: {err}", item.name, item.version))
                        .collect();
                    toasts::error(
                        toasts,
                        format!("Installed: {successful}, errors:\n{}", errors.join("\n")),
                    );
                }
            },

            // ── Validation ───────────────────────────────────────
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

            // ── Remove core ─────────────────────────────────────
            Event::CoreRemoved { hash, item } => {
                if let Some(lock_item) = &item {
                    self.state.cores.installed.retain(|(h, _)| h != &hash);
                    toasts::success(
                        toasts,
                        format!("Core removed: {} v{}", lock_item.item.name, lock_item.item.version),
                    );
                } else {
                    toasts::warning(toasts, format!("Core not found: {hash}"));
                }
            },

            // ── Remove instance ──────────────────────────────────
            Event::InstanceRemoved { name, item } => {
                self.state.instances.busy_instances.remove(&name);
                if item.is_some() {
                    self.state.instances.installed.retain(|(n, _)| n != &name);
                    toasts::success(toasts, format!("Instance removed: {name}"));
                } else {
                    toasts::warning(toasts, format!("Instance not found: {name}"));
                }
            },

            // ── Create instance ──────────────────────────────────
            Event::InstanceCreated(Ok(name)) => {
                toasts::success(toasts, format!("Instance created: {name}"));
            },
            Event::InstanceCreated(Err(e)) => {
                toasts::error(toasts, format!("Instance creation error: {e}"));
            },

            // ── Instance info ────────────────────────────────────
            Event::InstanceInfo(Ok(instance)) => {
                self.state.instances.viewing = Some(instance);
            },
            Event::InstanceInfo(Err(e)) => {
                toasts::error(toasts, format!("Instance info error: {e}"));
            },

            // ── Edit instance ────────────────────────────────────
            Event::InstanceEdited(Ok(name)) => {
                toasts::success(toasts, format!("Instance updated: {name}"));
            },
            Event::InstanceEdited(Err(e)) => {
                toasts::error(toasts, format!("Instance update error: {e}"));
            },

            // ── Launch instance ──────────────────────────────────
            Event::InstanceLaunched { name, result: Ok(pid) } => {
                self.state.instances.running_instances.insert(name.clone(), pid);
                toasts::success(toasts, format!("Launched: {name} (PID {pid})"));
            },
            Event::InstanceLaunched { name, result: Err(e) } => {
                toasts::error(toasts, format!("Launch failed ({name}): {e}"));
            },

            // ── Instance stopped ─────────────────────────────────
            Event::InstanceStopped { name, status } => {
                self.state.instances.running_instances.remove(&name);
                match status {
                    Some(0) => toasts::info(toasts, format!("Instance stopped: {name}")),
                    Some(code) => toasts::warning(toasts, format!("Instance stopped: {name} (exit code {code})")),
                    None => toasts::warning(toasts, format!("Instance stopped: {name} (killed)")),
                }
            },

            // ── Fetch ────────────────────────────────────────────
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
                toasts::info(toasts, format!("Found: {} v{}", item.name, item.version));
            },
            Event::CoreFetched(Ok(None)) => {
                toasts::warning(toasts, "Version not found");
            },
            Event::CoreFetched(Err(e)) => {
                toasts::error(toasts, format!("Fetch version error: {e}"));
            },

            // ── Items snapshot ────────────────────────────────────
            Event::CoresItems(items) => {
                self.state.cores.installed = items;
            },
            Event::InstancesItems(items, core_dependents) => {
                self.state.cores.core_dependents = core_dependents;
                self.state.instances.installed = items;
            },

            // ── Errors / Lifecycle ───────────────────────────────
            Event::Error(ref e) => match e {
                ComposerError::CoreInUse { hash: _, dependents } => {
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
}

// ── eframe::App ──────────────────────────────────────────────────────

impl eframe::App for App {
    /// Обработка событий без UI — вызывается перед `ui()`,
    /// а также когда окно свёрнуто, но был `request_repaint`.
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drain events here so state is fresh before ui() draws.
        // We can't emit toasts here (no Ui), so we buffer nothing —
        // toasts are emitted inside ui() from the same drain call.
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Toast container — recreated each frame (stores state in egui memory)
        let mut toasts = toasts::create_toasts();

        // 1. Drain events from worker
        self.drain_and_apply_events(&mut toasts);

        let ctx = ui.ctx().clone();

        // 2. Bottom panel — navigation bar
        egui::Panel::bottom("nav_panel")
            .resizable(false)
            .default_size(30.0)
            .show_inside(ui, |ui| {
                views::sidebar::render(ui, &mut self.current_tab, &self.state.cores.downloads);
            });

        // 3. Central panel — active tab
        egui::CentralPanel::default().show_inside(ui, |ui| match self.current_tab {
            Tab::Cores => {
                let action = views::cores_tab::render(ui, &ctx, &mut self.state.cores, &self.handle, &mut toasts);

                // Handle cross-tab action: "+" button → switch to Instances with pre-selected core
                if let Some(core_idx) = action.switch_to_instances_with_core {
                    self.current_tab = Tab::Instances;
                    self.state.instances.create_form = Some(InstanceForm {
                        selected_core_idx: Some(core_idx),
                        ..Default::default()
                    });
                }
            },
            Tab::Instances => {
                views::instances_tab::render(
                    ui,
                    &mut self.state.instances,
                    &self.handle,
                    &self.state.cores.installed,
                    &mut toasts,
                );
            },
        });

        // 4. Draw toasts (must be last — renders overlay)
        toasts.show(ui);
    }

    /// При закрытии окна — отправляем Shutdown worker'у.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        tracing::info!("sending Shutdown to worker");
        self.handle.try_send(Command::Shutdown);
    }
}

// ── Утилиты ──────────────────────────────────────────────────────────

/// Форматирует размер в байтах в человекочитаемый вид.
pub fn format_size(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;

    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}
