//! Главный модуль GUI-приложения на egui.
//!
//! [`App`] — минимальный оркестратор: маршрутизирует [`Event`]-ы от worker'а
//! в [`UiState`], делегирует отрисовку [`gui_ui::render_ui`].
//!
//! В debug-сборке загружает `gui_ui.dll` динамически для hot-reload.
//! В release — вызывает `gui_ui::render_ui` статически.

use eframe::egui;
use egui_toast::Toasts;

use composer::error::ComposerError;
use composer::message::{Command, CoresInstalledResult, Event};
use composer::worker::WorkerHandle;

use gui_ui::Tab;
use gui_ui::state::UiState;
use gui_ui::toasts;

// ── Hot-reload support (debug only) ─────────────────────────────────

/// Dynamically loaded render function (debug builds).
#[cfg(debug_assertions)]
struct HotLib {
    _lib: libloading::Library,
    render_fn: libloading::Symbol<'static, unsafe fn(&mut egui::Ui, &mut gui_ui::RenderArgs)>,
    loaded_modified: Option<std::time::SystemTime>,
}

#[cfg(debug_assertions)]
impl HotLib {
    fn dll_path() -> std::path::PathBuf {
        // cargo puts cdylib at target/debug/gui_ui.dll (Windows)
        let mut path = std::env::current_exe().unwrap();
        path.pop(); // remove exe name
        // Go up from target/debug/gui.exe to target/debug/
        #[cfg(target_os = "windows")]
        path.push("gui_ui.dll");
        #[cfg(target_os = "linux")]
        path.push("libgui_ui.so");
        #[cfg(target_os = "macos")]
        path.push("libgui_ui.dylib");
        path
    }

    fn load() -> Option<Self> {
        let dll_path = Self::dll_path();
        if !dll_path.exists() {
            tracing::warn!("Hot-reload DLL not found at {}, using static link", dll_path.display());
            return None;
        }

        // Copy DLL to a temp file to avoid lock issues on Windows
        let tmp_path = dll_path.with_extension("hot.dll");
        if let Err(e) = std::fs::copy(&dll_path, &tmp_path) {
            tracing::warn!("Failed to copy DLL for hot-reload: {e}");
            return None;
        }

        let modified = std::fs::metadata(&dll_path).ok().and_then(|m| m.modified().ok());

        unsafe {
            match libloading::Library::new(&tmp_path) {
                Ok(lib) => {
                    // SAFETY: render_ui has the same ABI because it's compiled with the same rustc
                    // in the same workspace. We transmute the lifetime to 'static because the
                    // library lives as long as this struct.
                    let render_fn: libloading::Symbol<unsafe fn(&mut egui::Ui, &mut gui_ui::RenderArgs)> =
                        match lib.get::<unsafe fn(&mut egui::Ui, &mut gui_ui::RenderArgs)>(b"render_ui") {
                            Ok(sym) => std::mem::transmute::<
                                libloading::Symbol<'_, unsafe fn(&mut egui::Ui, &mut gui_ui::RenderArgs)>,
                                libloading::Symbol<'_, unsafe fn(&mut egui::Ui, &mut gui_ui::RenderArgs)>,
                            >(sym),
                            Err(e) => {
                                tracing::error!("Failed to find render_ui in DLL: {e}");
                                return None;
                            },
                        };
                    tracing::info!("Hot-reload: loaded {}", dll_path.display());
                    Some(Self {
                        _lib: lib,
                        render_fn,
                        loaded_modified: modified,
                    })
                },
                Err(e) => {
                    tracing::error!("Failed to load DLL: {e}");
                    None
                },
            }
        }
    }

    fn needs_reload(&self) -> bool {
        let dll_path = Self::dll_path();
        let current_modified = std::fs::metadata(&dll_path).ok().and_then(|m| m.modified().ok());
        match (self.loaded_modified, current_modified) {
            (Some(old), Some(new)) => new > old,
            _ => false,
        }
    }
}

// ── App ──────────────────────────────────────────────────────────────

/// Главная структура GUI-приложения.
pub struct App {
    pub handle: WorkerHandle,

    #[allow(dead_code)]
    runtime: tokio::runtime::Runtime,

    pub current_tab: Tab,
    pub state: UiState,

    /// Hot-reload library (debug builds only).
    #[cfg(debug_assertions)]
    hot_lib: Option<HotLib>,
}

impl App {
    pub fn new(handle: WorkerHandle, runtime: tokio::runtime::Runtime) -> Self {
        handle.try_send(Command::GetCoresItems);
        handle.try_send(Command::GetInstancesItems);

        Self {
            handle,
            runtime,
            current_tab: Tab::default(),
            state: UiState::default(),
            #[cfg(debug_assertions)]
            hot_lib: HotLib::load(),
        }
    }

    fn drain_and_apply_events(&mut self, toasts: &mut Toasts) {
        let events = self.handle.drain_events();
        for event in events {
            self.apply_event(event, toasts);
        }
    }

    fn apply_event(&mut self, event: Event, toasts: &mut Toasts) {
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

            Event::CoresInstalled(CoresInstalledResult { successful, failed }) => {
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

            Event::CoreRemoved { hash, item } => {
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

            Event::InstanceRemoved { name, item } => {
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

            Event::InstanceLaunched { name, result: Ok(pid) } => {
                self.state.instances.running_instances.insert(name.clone(), pid);
                if let Some((_n, meta)) = self.state.instances.installed.iter_mut().find(|(n, _)| n == &name) {
                    meta.last_launch = Some(chrono::Utc::now());
                }
                toasts::success(toasts, format!("Launched: {name} (PID {pid})"));
            },
            Event::InstanceLaunched { name, result: Err(e) } => {
                toasts::error(toasts, format!("Launch failed ({name}): {e}"));
            },

            Event::InstanceStopped { name, status } => {
                self.state.instances.running_instances.remove(&name);
                match status {
                    Some(0) => toasts::info(toasts, format!("Instance stopped: {name}")),
                    Some(code) => toasts::warning(toasts, format!("Instance stopped: {name} (exit code {code})")),
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

            Event::InstanceDirSize { name, bytes } => {
                if let Some(ref mut panel) = self.state.instances.instance_panel
                    && panel.name == name
                {
                    panel.dir_size = Some(bytes);
                }
            },

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

    /// Calls render_ui — dynamically via DLL in debug, statically in release.
    fn call_render_ui(&mut self, ui: &mut egui::Ui) {
        let mut args = gui_ui::RenderArgs {
            current_tab: &mut self.current_tab,
            state: &mut self.state,
            handle: &self.handle,
        };

        #[cfg(debug_assertions)]
        {
            if let Some(ref hot) = self.hot_lib {
                unsafe { (hot.render_fn)(ui, &mut args) };
                return;
            }
        }

        // Fallback: static call (always used in release)
        gui_ui::render_ui(ui, &mut args);
    }
}

// ── eframe::App ──────────────────────────────────────────────────────

impl eframe::App for App {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {}

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Debug hotkeys + hot-reload check
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

            // Check for DLL changes every frame (cheap: just stat the file)
            if let Some(ref hot) = self.hot_lib
                && hot.needs_reload() {
                    tracing::info!("Hot-reload: DLL changed, reloading...");
                    self.hot_lib = None; // drop old library first
                    self.hot_lib = HotLib::load();
                }
        }

        // Drain events
        let mut toasts_instance = gui_ui::toasts::create_toasts();
        self.drain_and_apply_events(&mut toasts_instance);

        // Render UI (hot or static)
        self.call_render_ui(ui);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        tracing::info!("sending Shutdown to worker");
        self.handle.try_send(Command::Shutdown);
        let _ = self.state.settings.lock.save();
    }
}
