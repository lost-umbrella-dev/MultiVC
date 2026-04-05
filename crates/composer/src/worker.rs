//! Background worker: связывает [`Command`]/[`Event`] протокол с [`Composer`].
//!
//! Используется **только** GUI и TUI фронтендами.
//! CLI вызывает методы [`Composer`] напрямую — каналы ему не нужны.
//!
//! ```text
//! UI Thread                           Background Thread
//! ─────────                           ─────────────────
//! WorkerHandle                        ComposerWorker
//!   .commands.send(Command) ────────► .commands.recv()
//!                                          │
//!                                     Composer.method()
//!                                          │
//!   .events.recv() ◄──────────────── .events.send(Event)
//! ```

use tokio::sync::mpsc;

use crate::Composer;
use crate::lock::Lock;
use crate::message::{Command, CoresInstalledResult, Event, InstancesSnapshot};

// ── Handles ──────────────────────────────────────────────────────────

/// Хэндлы для UI-стороны.
///
/// Хранятся в GUI/TUI приложении.
/// `commands` — отправка команд в background.
/// `events` — получение результатов из background.
pub struct WorkerHandle {
    pub commands: mpsc::Sender<Command>,
    pub events: mpsc::Receiver<Event>,
}

impl WorkerHandle {
    /// Отправить команду, не блокируя UI.
    ///
    /// Возвращает `false` если worker уже завершился (канал закрыт).
    pub fn try_send(&self, cmd: Command) -> bool {
        self.commands.try_send(cmd).is_ok()
    }

    /// Забрать все накопившиеся события (non-blocking drain).
    ///
    /// Идиоматичный паттерн для immediate-mode UI:
    /// вызывается на каждом кадре / тике.
    pub fn drain_events(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        while let Ok(event) = self.events.try_recv() {
            events.push(event);
        }
        events
    }
}

// ── Worker ───────────────────────────────────────────────────────────

/// Background runner: крутится в `tokio::spawn`, потребляет [`Command`],
/// вызывает методы [`Composer`] и отправляет [`Event`] обратно в UI.
pub struct ComposerWorker {
    composer: Composer,
    commands: mpsc::Receiver<Command>,
    events: mpsc::Sender<Event>,
}

impl ComposerWorker {
    /// Размер буфера каналов по умолчанию.
    pub const DEFAULT_BUFFER: usize = 8;

    /// Создаёт worker + [`WorkerHandle`] для UI-стороны.
    ///
    /// `buffer` — размер буфера обоих mpsc каналов.
    pub fn new(composer: Composer, buffer: usize) -> (Self, WorkerHandle) {
        let (cmd_tx, cmd_rx) = mpsc::channel(buffer);
        let (evt_tx, evt_rx) = mpsc::channel(buffer);

        let worker = Self {
            composer,
            commands: cmd_rx,
            events: evt_tx,
        };

        let handle = WorkerHandle {
            commands: cmd_tx,
            events: evt_rx,
        };

        (worker, handle)
    }

    /// Создаёт worker + handle с буфером по умолчанию ([`Self::DEFAULT_BUFFER`]).
    pub fn with_defaults(composer: Composer) -> (Self, WorkerHandle) {
        Self::new(composer, Self::DEFAULT_BUFFER)
    }

    /// Запускает event-loop worker'а.
    ///
    /// Вызывается внутри `tokio::spawn`:
    /// ```ignore
    /// let (worker, handle) = ComposerWorker::with_defaults(composer);
    /// tokio::spawn(worker.run());
    /// ```
    ///
    /// Завершается когда:
    /// - получена команда [`Command::Shutdown`]
    /// - UI закрыл свою сторону канала (все `Sender<Command>` дропнуты)
    pub async fn run(mut self) {
        tracing::info!("composer worker started");

        while let Some(cmd) = self.commands.recv().await {
            let is_shutdown = matches!(cmd, Command::Shutdown);

            let event = self.handle(cmd).await;

            // Если UI отключился — выходим молча
            if self.events.send(event).await.is_err() {
                tracing::warn!("event channel closed, worker stopping");
                break;
            }

            if is_shutdown {
                tracing::info!("shutdown requested, worker stopping");
                break;
            }
        }

        tracing::info!("composer worker stopped");
    }

    /// Снимок текущих ядер из lock (для отправки в UI).
    fn cores_snapshot(&self) -> crate::message::ItemsSnapshot {
        self.composer
            .cores_items()
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// Снимок текущих инстансов из lock (для отправки в UI).
    fn instances_snapshot(&self) -> InstancesSnapshot {
        self.composer
            .instances_items()
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// Отправляет событие в UI, игнорируя ошибку закрытого канала.
    async fn send(&self, event: Event) {
        let _ = self.events.send(event).await;
    }

    /// Диспетчеризация одной команды → один Event.
    async fn handle(&mut self, cmd: Command) -> Event {
        match cmd {
            // ── Persistence ──────────────────────────────────────
            Command::Save => {
                let result = self.composer.save().await;
                Event::Saved(result)
            },

            Command::SaveCores => {
                let result = self.composer.save_cores().await;
                Event::CoresSaved(result)
            },

            Command::SaveInstances => {
                let result = self.composer.save_instances().await;
                Event::InstancesSaved(result)
            },

            // ── Install ──────────────────────────────────────────
            Command::InstallCores { requests } => {
                let total = requests.len();
                let result = self.composer.install_cores(requests).await;
                match result {
                    Ok(maybe_failed) => {
                        let failed = maybe_failed.unwrap_or_default();
                        let successful = total.saturating_sub(failed.len());

                        // Автосохраняем lock после установки — даже при частичном успехе,
                        // чтобы не потерять уже установленные элементы.
                        if successful > 0
                            && let Err(e) = self.composer.save_cores().await
                        {
                            tracing::error!(error = %e, "failed to save cores lock after install");
                        }

                        // Отправляем актуальный снимок ядер в UI
                        self.send(Event::CoresItems(self.cores_snapshot())).await;

                        Event::CoresInstalled(CoresInstalledResult { successful, failed })
                    },
                    Err(fatal) => Event::Error(fatal),
                }
            },

            // ── Validation ───────────────────────────────────────
            Command::ValidateCores => {
                let result = self.composer.validate_cores().await;
                Event::CoresValidated(result)
            },

            Command::ValidateInstances => {
                let result = self.composer.validate_instances().await;
                Event::InstancesValidated(result)
            },

            // ── Remove core ──────────────────────────────────────
            Command::RemoveCore { hash } => {
                match self.composer.cores.remove(&hash).await {
                    Ok(item) => {
                        // Автосохраняем lock после удаления
                        if item.is_some() {
                            if let Err(e) = self.composer.save_cores().await {
                                tracing::error!(error = %e, "failed to save cores lock after removal");
                            }
                            // Отправляем актуальный снимок ядер в UI
                            self.send(Event::CoresItems(self.cores_snapshot())).await;
                        }
                        Event::CoreRemoved { hash, item }
                    },
                    Err(e) => Event::Error(e),
                }
            },

            // ── Instances CRUD ───────────────────────────────────
            Command::CreateInstance { name, config, meta } => {
                match self.composer.create_instance(name.clone(), config, meta).await {
                    Ok(()) => {
                        // Отправляем актуальный снимок инстансов в UI
                        self.send(Event::InstancesItems(self.instances_snapshot())).await;
                        Event::InstanceCreated(Ok(name))
                    },
                    Err(e) => Event::InstanceCreated(Err(e)),
                }
            },

            Command::GetInstance { name } => {
                let result = self.composer.get_instance(&name).await;
                Event::InstanceInfo(result)
            },

            Command::EditInstance { name, config, meta } => {
                match self.composer.edit_instance(&name, config, meta).await {
                    Ok(()) => {
                        // Отправляем актуальный снимок инстансов в UI
                        self.send(Event::InstancesItems(self.instances_snapshot())).await;
                        Event::InstanceEdited(Ok(name))
                    },
                    Err(e) => Event::InstanceEdited(Err(e)),
                }
            },

            Command::RemoveInstance { name } => {
                match self.composer.remove_instance(&name).await {
                    Ok(item) => {
                        if item.is_some() {
                            // Автосохраняем lock после удаления
                            if let Err(e) = self.composer.save_instances().await {
                                tracing::error!(error = %e, "failed to save instances lock after removal");
                            }
                            // Отправляем актуальный снимок инстансов в UI
                            self.send(Event::InstancesItems(self.instances_snapshot())).await;
                        }
                        Event::InstanceRemoved { name, item }
                    },
                    Err(e) => Event::Error(e),
                }
            },

            // ── Fetch (list / get) ───────────────────────────────
            Command::FetchCoresList { search_version } => {
                let result = self
                    .composer
                    .clients
                    .core
                    .list(search_version)
                    .await
                    .map_err(Into::into);
                Event::CoresFetched(result)
            },

            Command::FetchCore { version } => {
                let result = self
                    .composer
                    .clients
                    .core
                    .get(clients::github::GitHubGetOptions { version })
                    .await
                    .map_err(Into::into);
                Event::CoreFetched(result)
            },

            // ── Items snapshot ────────────────────────────────────
            Command::GetCoresItems => Event::CoresItems(self.cores_snapshot()),

            Command::GetInstancesItems => Event::InstancesItems(self.instances_snapshot()),

            // ── Lifecycle ────────────────────────────────────────
            Command::Shutdown => Event::ShutdownComplete,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clients::Clients;
    use clients::github::GithubClient;

    fn test_composer() -> Composer {
        let client =
            GithubClient::new("test-owner".to_owned(), "test-repo".to_owned()).expect("failed to create github client");
        Composer::new(Clients::new(client))
    }

    /// Smoke test: worker корректно обрабатывает Shutdown.
    ///
    /// Используем `tokio::select!` вместо `tokio::spawn`, потому что
    /// future `worker.run()` не является `Send` (из-за `dyn DynDigest`
    /// в цепочке `install_cores`). `select!` запускает ветки на одном
    /// потоке — `Send` не требуется.
    #[tokio::test]
    async fn shutdown_returns_complete() {
        let (worker, mut handle) = ComposerWorker::with_defaults(test_composer());

        // Отправляем Shutdown до запуска run() — команда попадёт в буфер канала.
        handle.commands.send(Command::Shutdown).await.unwrap();

        // Worker обработает одну команду и завершится.
        worker.run().await;

        let event = handle.events.recv().await.expect("expected ShutdownComplete");
        assert!(matches!(event, Event::ShutdownComplete));
    }

    /// Smoke test: drop sender-стороны commands приводит к завершению worker'а.
    #[tokio::test]
    async fn drop_handle_stops_worker() {
        let (worker, handle) = ComposerWorker::with_defaults(test_composer());

        // Дропаем handle — commands sender закрывается,
        // worker.run() увидит `None` из recv() и выйдет.
        drop(handle);

        // Worker должен завершиться без паники.
        worker.run().await;
    }
}
