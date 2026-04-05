//! Главный модуль GUI-приложения на egui.
//!
//! [`App`] хранит [`WorkerHandle`] для связи с background-потоком
//! и UI-состояние, обновляемое через [`Event`]-ы от `[`ComposerWorker`]`.

use std::sync::Arc;

use eframe::egui;

use clients::hash::Hash;
use clients::item::Item;
use composer::item::LockItem;
use composer::lock::ValidateReason;
use composer::lock::instances::{InstanceValidateReason, InstancesItem};
use composer::message::{Command, CoresInstalledResult, Event};
use composer::progress::{ProgressBridge, RepaintHook};
use composer::worker::WorkerHandle;

use clients::github::GitHubListOptions;

// ── Навигация ────────────────────────────────────────────────────────

/// Вкладки левой панели навигации.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Tab {
    #[default]
    Cores,
    Instances,
}

// ── UI State ─────────────────────────────────────────────────────────

/// Кэшированное состояние UI, обновляемое из [`Event`]-ов.
#[derive(Default)]
struct UiState {
    /// Установленные ядра (кэш из lock).
    cores: Vec<(Hash, LockItem)>,

    /// Установленные инстансы (кэш из lock).
    instances: Vec<(String, InstancesItem)>,

    /// Доступные на GitHub версии ядер.
    available_cores: Vec<Item>,

    /// Результат последней валидации ядер.
    core_validation: Vec<ValidateReason>,

    /// Результат последней валидации инстансов.
    instance_validation: Vec<InstanceValidateReason>,

    /// Последняя ошибка для отображения.
    last_error: Option<String>,

    /// Последнее информационное сообщение (toast-стиль).
    last_info: Option<String>,

    /// Идёт ли сейчас какая-то фоновая операция.
    busy: bool,
}

// ── App ──────────────────────────────────────────────────────────────

/// Главная структура GUI-приложения.
///
/// Хранит [`WorkerHandle`] для отправки команд / получения событий,
/// tokio runtime (для возможных ad-hoc async вызовов) и всё UI-состояние.
pub struct App {
    /// Хэндл к фоновому [`ComposerWorker`](composer::worker::ComposerWorker).
    handle: WorkerHandle,

    /// Tokio runtime — нужен для `block_on` при инициализации
    /// и как владелец spawned worker task.
    #[allow(dead_code)]
    runtime: tokio::runtime::Runtime,

    /// Текущая вкладка навигации.
    current_tab: Tab,

    /// Кэшированное UI-состояние.
    state: UiState,

    /// Мост прогресса для текущей загрузки (если идёт).
    progress_bridge: Option<ProgressBridge>,
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
            progress_bridge: None,
        }
    }

    /// Создаёт [`RepaintHook`] привязанный к текущему egui контексту.
    ///
    /// Вызов хука из любого потока безопасно запросит
    /// перерисовку UI-кадра.
    fn repaint_hook(ctx: &egui::Context) -> RepaintHook {
        let ctx = ctx.clone();
        Arc::new(move || ctx.request_repaint())
    }

    // ── Event handling ───────────────────────────────────────────

    /// Обрабатывает все накопившиеся [`Event`]-ы от worker'а.
    ///
    /// Вызывается в начале каждого кадра `update()`.
    fn drain_and_apply_events(&mut self) {
        let events = self.handle.drain_events();
        for event in events {
            self.apply_event(event);
        }
    }

    /// Применяет один [`Event`] к UI-состоянию.
    fn apply_event(&mut self, event: Event) {
        // Большинство событий означает конец фоновой операции
        self.state.busy = false;

        match event {
            // ── Persistence ──────────────────────────────────────
            Event::Saved(Ok(())) => {
                self.state.last_info = Some("Все lock-файлы сохранены".into());
            },
            Event::Saved(Err(e)) => {
                self.state.last_error = Some(format!("Ошибка сохранения: {e}"));
            },

            Event::CoresSaved(Ok(())) => {
                self.state.last_info = Some("Lock ядер сохранён".into());
            },
            Event::CoresSaved(Err(e)) => {
                self.state.last_error = Some(format!("Ошибка сохранения ядер: {e}"));
            },

            Event::InstancesSaved(Ok(())) => {
                self.state.last_info = Some("Lock инстансов сохранён".into());
            },
            Event::InstancesSaved(Err(e)) => {
                self.state.last_error = Some(format!("Ошибка сохранения инстансов: {e}"));
            },

            // ── Install ──────────────────────────────────────────
            Event::CoresInstalled(CoresInstalledResult { successful, failed }) => {
                self.progress_bridge = None;
                if failed.is_empty() {
                    self.state.last_info = Some(format!("Установлено ядер: {successful}"));
                } else {
                    let errors: Vec<String> = failed
                        .iter()
                        .map(|(item, err)| format!("  {} v{}: {err}", item.name, item.version))
                        .collect();
                    self.state.last_error = Some(format!("Установлено: {successful}, ошибки:\n{}", errors.join("\n")));
                }
            },

            // ── Validation ───────────────────────────────────────
            Event::CoresValidated(Ok(reasons)) => {
                if reasons.is_empty() {
                    self.state.last_info = Some("Ядра: всё валидно ✓".into());
                } else {
                    self.state.last_info = Some(format!("Ядра: найдено проблем: {}", reasons.len()));
                }
                self.state.core_validation = reasons;
            },
            Event::CoresValidated(Err(e)) => {
                self.state.last_error = Some(format!("Ошибка валидации ядер: {e}"));
            },

            Event::InstancesValidated(Ok(reasons)) => {
                if reasons.is_empty() {
                    self.state.last_info = Some("Инстансы: всё валидно ✓".into());
                } else {
                    self.state.last_info = Some(format!("Инстансы: найдено проблем: {}", reasons.len()));
                }
                self.state.instance_validation = reasons;
            },
            Event::InstancesValidated(Err(e)) => {
                self.state.last_error = Some(format!("Ошибка валидации инстансов: {e}"));
            },

            // ── Remove ───────────────────────────────────────────
            Event::CoreRemoved { hash, item } => {
                if let Some(lock_item) = &item {
                    self.state.cores.retain(|(h, _)| h != &hash);
                    self.state.last_info = Some(format!(
                        "Ядро удалено: {} v{}",
                        lock_item.item.name, lock_item.item.version
                    ));
                } else {
                    self.state.last_error = Some(format!("Ядро не найдено: {hash}"));
                }
            },

            Event::InstanceRemoved { name, item } => {
                if item.is_some() {
                    self.state.instances.retain(|(n, _)| n != &name);
                    self.state.last_info = Some(format!("Инстанс удалён: {name}"));
                } else {
                    self.state.last_error = Some(format!("Инстанс не найден: {name}"));
                }
            },

            Event::InstanceCreated(Ok(name)) => {
                self.state.last_info = Some(format!("Инстанс создан: {name}"));
            },
            Event::InstanceCreated(Err(e)) => {
                self.state.last_error = Some(format!("Ошибка создания инстанса: {e}"));
            },

            Event::InstanceInfo(_) => {
                // Handled by specific UI flows that request instance info
            },

            Event::InstanceEdited(Ok(name)) => {
                self.state.last_info = Some(format!("Инстанс обновлён: {name}"));
            },
            Event::InstanceEdited(Err(e)) => {
                self.state.last_error = Some(format!("Ошибка обновления инстанса: {e}"));
            },

            // ── Fetch ────────────────────────────────────────────
            Event::CoresFetched(Ok(items)) => {
                self.state.last_info = Some(format!("Доступных версий: {}", items.len()));
                self.state.available_cores = items;
            },
            Event::CoresFetched(Err(e)) => {
                self.state.last_error = Some(format!("Ошибка получения списка: {e}"));
            },

            Event::CoreFetched(Ok(Some(item))) => {
                self.state.last_info = Some(format!("Найдено: {} v{}", item.name, item.version));
            },
            Event::CoreFetched(Ok(None)) => {
                self.state.last_info = Some("Версия не найдена".into());
            },
            Event::CoreFetched(Err(e)) => {
                self.state.last_error = Some(format!("Ошибка получения версии: {e}"));
            },

            // ── Items snapshot ────────────────────────────────────
            Event::CoresItems(items) => {
                self.state.cores = items;
            },
            Event::InstancesItems(items) => {
                self.state.instances = items;
            },

            // ── Errors / Lifecycle ───────────────────────────────
            Event::Error(e) => {
                self.state.last_error = Some(format!("Фатальная ошибка: {e}"));
            },
            Event::ShutdownComplete => {
                self.state.last_info = Some("Background worker завершён".into());
            },
        }
    }

    // ── Render helpers ───────────────────────────────────────────

    /// Рисует левую панель навигации.
    fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.heading("MultiVC");
        ui.separator();

        ui.selectable_value(&mut self.current_tab, Tab::Cores, "\u{2699} Ядра");
        ui.selectable_value(&mut self.current_tab, Tab::Instances, "\u{1F4E6} Инстансы");

        ui.separator();

        // Статус
        if self.state.busy {
            ui.spinner();
            ui.label("Выполняется...");
        }

        // Прогресс-бар текущей загрузки
        if let Some(bridge) = &self.progress_bridge
            && let Some(fraction) = bridge.fraction()
        {
            ui.add(egui::ProgressBar::new(fraction).show_percentage());
        }

        ui.separator();

        // Ошибка
        if let Some(err) = &self.state.last_error {
            ui.colored_label(egui::Color32::RED, err);
            if ui.small_button("Скрыть").clicked() {
                self.state.last_error = None;
            }
        }

        // Информация
        if let Some(info) = &self.state.last_info {
            ui.colored_label(egui::Color32::GREEN, info);
            if ui.small_button("Скрыть").clicked() {
                self.state.last_info = None;
            }
        }
    }

    /// Рисует вкладку «Ядра».
    fn render_cores_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Установленные ядра");

        // Кнопки действий
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!self.state.busy, egui::Button::new("Обновить список с GitHub"))
                .clicked()
            {
                self.state.busy = true;
                self.handle.try_send(Command::FetchCoresList {
                    search_version: GitHubListOptions { search_version: vec![] },
                });
            }

            if ui
                .add_enabled(!self.state.busy, egui::Button::new("Валидация"))
                .clicked()
            {
                self.state.busy = true;
                self.handle.try_send(Command::ValidateCores);
            }

            if ui
                .add_enabled(!self.state.busy, egui::Button::new("Сохранить lock"))
                .clicked()
            {
                self.state.busy = true;
                self.handle.try_send(Command::SaveCores);
            }
        });

        ui.separator();

        // Таблица установленных ядер
        if self.state.cores.is_empty() {
            ui.label("Нет установленных ядер.");
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("cores_grid")
                    .num_columns(4)
                    .striped(true)
                    .show(ui, |ui| {
                        // Заголовки
                        ui.strong("Имя");
                        ui.strong("Версия");
                        ui.strong("Хэш");
                        ui.strong("Действия");
                        ui.end_row();

                        // Данные — собираем хэши для удаления отдельно,
                        // чтобы не мутировать `self` внутри итерации.
                        let mut to_remove: Option<Hash> = None;

                        for (hash, lock_item) in &self.state.cores {
                            ui.label(&lock_item.item.name);
                            ui.label(&lock_item.item.version);

                            let hash_str = hash.to_string();
                            let short = if hash_str.len() > 20 {
                                format!("{}…", &hash_str[..20])
                            } else {
                                hash_str
                            };
                            ui.label(short).on_hover_text(hash.to_string());

                            if ui
                                .add_enabled(!self.state.busy, egui::Button::new("\u{1F5D1}"))
                                .on_hover_text("Удалить")
                                .clicked()
                            {
                                to_remove = Some(hash.clone());
                            }
                            ui.end_row();
                        }

                        if let Some(hash) = to_remove {
                            self.state.busy = true;
                            self.handle.try_send(Command::RemoveCore { hash });
                        }
                    });
            });
        }

        // Результаты валидации
        if !self.state.core_validation.is_empty() {
            ui.separator();
            ui.heading("Результаты валидации");
            for reason in &self.state.core_validation {
                match reason {
                    ValidateReason::HashNotMatcher(hash, item) => {
                        ui.colored_label(
                            egui::Color32::YELLOW,
                            format!("Хэш не совпадает: {} v{} ({})", item.item.name, item.item.version, hash),
                        );
                    },
                    ValidateReason::NotFound(hash, item) => {
                        ui.colored_label(
                            egui::Color32::RED,
                            format!("Не найден: {} v{} ({})", item.item.name, item.item.version, hash),
                        );
                    },
                }
            }
        }

        // Доступные на GitHub версии
        if !self.state.available_cores.is_empty() {
            ui.separator();
            ui.heading("Доступные версии");

            egui::ScrollArea::vertical()
                .id_salt("available_cores_scroll")
                .max_height(200.0)
                .show(ui, |ui| {
                    egui::Grid::new("available_cores_grid")
                        .num_columns(4)
                        .striped(true)
                        .show(ui, |ui| {
                            ui.strong("Имя");
                            ui.strong("Версия");
                            ui.strong("Размер");
                            ui.strong("Действия");
                            ui.end_row();

                            // Собираем версии для установки / переустановки
                            let mut to_install: Option<Item> = None;

                            // Собираем имена установленных ядер для быстрого поиска
                            let installed_names: std::collections::HashSet<&str> =
                                self.state.cores.iter().map(|(_, li)| li.item.name.as_str()).collect();

                            for item in &self.state.available_cores {
                                ui.label(&item.name);
                                ui.label(&item.version);
                                ui.label(format_size(item.size));

                                let is_installed = installed_names.contains(item.name.as_str());

                                ui.horizontal(|ui| {
                                    if is_installed {
                                        // Ядро уже установлено — disabled кнопка + кнопка переустановки
                                        ui.add_enabled(false, egui::Button::new("\u{2714} Скачано"));

                                        if ui
                                            .add_enabled(!self.state.busy, egui::Button::new("\u{21BB}").small())
                                            .on_hover_text("Переустановить")
                                            .clicked()
                                        {
                                            to_install = Some(item.clone());
                                        }
                                    } else if ui
                                        .add_enabled(!self.state.busy, egui::Button::new("Установить"))
                                        .clicked()
                                    {
                                        to_install = Some(item.clone());
                                    }
                                });
                                ui.end_row();
                            }

                            if let Some(item) = to_install {
                                self.state.busy = true;

                                // Создаём ProgressBridge с хуком перерисовки
                                let bridge = ProgressBridge::new(Self::repaint_hook(ctx));
                                let request = composer::DownloadRequest::with_progress(item, bridge.sink());
                                self.progress_bridge = Some(bridge);

                                self.handle.try_send(Command::InstallCores {
                                    requests: vec![request],
                                });
                            }
                        });
                });
        }
    }

    /// Рисует вкладку «Инстансы».
    fn render_instances_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Инстансы");

        ui.horizontal(|ui| {
            if ui
                .add_enabled(!self.state.busy, egui::Button::new("Валидация"))
                .clicked()
            {
                self.state.busy = true;
                self.handle.try_send(Command::ValidateInstances);
            }

            if ui
                .add_enabled(!self.state.busy, egui::Button::new("Сохранить lock"))
                .clicked()
            {
                self.state.busy = true;
                self.handle.try_send(Command::SaveInstances);
            }
        });

        ui.separator();

        if self.state.instances.is_empty() {
            ui.label("Нет инстансов.");
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("instances_grid")
                    .num_columns(3)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Имя");
                        ui.strong("Иконка");
                        ui.strong("Действия");
                        ui.end_row();

                        let mut to_remove: Option<String> = None;

                        for (name, _meta) in &self.state.instances {
                            ui.label(name);
                            ui.label("(icon)");

                            if ui
                                .add_enabled(!self.state.busy, egui::Button::new("\u{1F5D1}"))
                                .on_hover_text("Удалить")
                                .clicked()
                            {
                                to_remove = Some(name.clone());
                            }
                            ui.end_row();
                        }

                        if let Some(name) = to_remove {
                            self.state.busy = true;
                            self.handle.try_send(Command::RemoveInstance { name });
                        }
                    });
            });
        }

        // Результаты валидации инстансов
        if !self.state.instance_validation.is_empty() {
            ui.separator();
            ui.heading("Результаты валидации");
            for reason in &self.state.instance_validation {
                match reason {
                    InstanceValidateReason::NotFound(name, _meta) => {
                        ui.colored_label(egui::Color32::RED, format!("Папка не найдена: {name}"));
                    },
                }
            }
        }
    }
}

// ── eframe::App ──────────────────────────────────────────────────────

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Получаем и применяем все события от worker'а
        self.drain_and_apply_events();

        // 2. Левая панель навигации
        egui::SidePanel::left("nav_panel")
            .resizable(false)
            .default_width(200.0)
            .show(ctx, |ui| {
                self.render_sidebar(ui);
            });

        // 3. Центральная панель — контент вкладки
        egui::CentralPanel::default().show(ctx, |ui| match self.current_tab {
            Tab::Cores => self.render_cores_tab(ui, ctx),
            Tab::Instances => self.render_instances_tab(ui),
        });
    }

    /// При закрытии окна — отправляем Shutdown worker'у.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        tracing::info!("отправляем Shutdown worker'у");
        self.handle.try_send(Command::Shutdown);
    }
}

// ── Утилиты ──────────────────────────────────────────────────────────

/// Форматирует размер в байтах в человекочитаемый вид.
fn format_size(bytes: u64) -> String {
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
