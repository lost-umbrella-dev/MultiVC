# Архитектура MultiVC

## Обзор

**MultiVC** — лаунчер для [VoxelCore](https://github.com/MihailRis/VoxelCore), построенный как Cargo workspace из нескольких крейтов.

| Крейт      | Роль                                  | Статус        |
|------------|---------------------------------------|---------------|
| `clients`  | HTTP-клиенты, загрузки, типы          | ✅ Реализован  |
| `composer` | Бизнес-логика, lock-файлы, пайплайны  | ✅ Реализован  |
| `gui`      | Графический интерфейс (egui)          | ✅ Заготовка   |
| `cli`      | Командная строка (clap)               | ✅ Заготовка   |
| `tui`      | Терминальный интерфейс (ratatui)      | 🔜 Планируется |

---

## Структура крейтов

### `clients`

HTTP-клиенты, download pipeline и общие типы данных.
**Не зависит** от других крейтов проекта.

Ключевые типы и модули:

- **`Clients`** — набор клиентов (`pub core: GithubClient`).
- **`GithubClient`** — работа с GitHub API: `list()`, `get()`, `get_all_releases()`.
  - Инициализация: `GithubClient::new(repo_owner, repo)`.
  - Репо VoxelCore: `"MihailRis"`, `"VoxelCore"`.
- **`GitHubListOptions`** — `{ search_version: Vec<String> }`.
- **`GitHubGetOptions`** — `{ version: String }`.
- **`Item`** — `{ name, version, url, hash: Option<Hash>, size, dependencies, supported_engine }`.
- **`Hash`** — `SHA256(String) | SHA512(String)`, Display как `"sha256:..."`, FromStr.
- **`DownloadProgress`** — `{ downloaded: u64, total: Option<u64> }`, метод `fraction()`.
- **`ProgressSink`** — trait для получения прогресса загрузки.
- **`download()`** — скачивание с валидацией хэша и прогрессом.
- **`asset_matches_current_system()`** — фильтрация ассетов по ОС.

### `composer`

Бизнес-логика приложения. **Зависит от `clients`.**

#### `Composer` (`lib.rs`) — фасад

```rust
pub struct Composer {
    pub(crate) clients: Clients,
    pub(crate) instances: InstancesLock,
    pub(crate) cores: CoresLock,
}
```

API:

- `Composer::new(clients)` — пустой (тесты, свежая установка).
- `Composer::load(clients)` — загрузка lock-файлов с диска.
- `save()`, `save_cores()`, `save_instances()` — персистенция.
- `cores_items() -> &LockMap`, `instances_items() -> &LockMap` — доступ к данным.
- `validate_cores()`, `validate_instances()` — проверка директорий на соответствие lock.
- `install_cores(Vec<DownloadRequest>)` — установка ядер (параллельно).

#### `Lock` trait (`lock.rs`)

Обобщённая логика для lock-файлов: `load()`, `save()`, `remove()`, `validate_dir()`.
`save()` автоматически создаёт родительские директории.

Реализации:

- `CoresLock` — `cores/lock.toml`, папка `cores/`.
- `InstancesLock` — `instances/lock.toml`, папка `instances/`.
- `ContentsLock` — планируется.

#### Типы данных (`item.rs`)

- `LockItem` — `{ item: Item, timestamp: DateTime<Utc> }`.
- `LockMap` = `DashMap<Hash, LockItem>` (concurrent, serde).

#### `ValidateReason`

```rust
pub enum ValidateReason {
    HashNotMatcher(Hash, LockItem),
    NotFound(Hash, LockItem),
}
```

#### `Command` / `Event` (`message.rs`)

Протокол для канала UI ↔ Background:

```rust
pub type ItemsSnapshot = Vec<(Hash, LockItem)>;

pub enum Command {
    Save, SaveCores, SaveInstances,
    InstallCores { requests: Vec<DownloadRequest> },
    ValidateCores, ValidateInstances,
    RemoveCore { hash }, RemoveInstance { hash },
    GetCoresItems, GetInstancesItems,
    FetchCoresList { search_version: GitHubListOptions },
    FetchCore { version: String },
    Shutdown,
}

pub enum Event {
    Saved(Result<()>), CoresSaved(Result<()>), InstancesSaved(Result<()>),
    CoresInstalled(CoresInstalledResult),
    CoresValidated(Result<Vec<ValidateReason>>),
    InstancesValidated(Result<Vec<ValidateReason>>),
    CoreRemoved { hash, item: Option<LockItem> },
    InstanceRemoved { hash, item: Option<LockItem> },
    CoresFetched(Result<Vec<Item>>), CoreFetched(Result<Option<Item>>),
    CoresItems(ItemsSnapshot), InstancesItems(ItemsSnapshot),
    Error(ComposerError), ShutdownComplete,
}

pub struct CoresInstalledResult { pub successful: usize, pub failed: Vec<(Item, ComposerError)> }
```

#### `ComposerWorker` (`worker.rs`)

Background runner для GUI/TUI. CLI не использует.

```rust
pub struct ComposerWorker { composer, commands: Receiver<Command>, events: Sender<Event> }
pub struct WorkerHandle { pub commands: Sender<Command>, pub events: Receiver<Event> }
```

- `ComposerWorker::with_defaults(composer) -> (Self, WorkerHandle)`.
- `worker.run()` — event-loop, запускается через `LocalSet` + `spawn_local` (future не Send из-за `dyn DynDigest`).
- Автосохраняет lock после `InstallCores` и `RemoveCore`/`RemoveInstance`.
- Автоматически отправляет `CoresItems`/`InstancesItems` после мутирующих операций.

#### `ProgressBridge` (`progress.rs`)

Lock-free мост прогресса:

- `ProgressBridge::new(RepaintHook)` — с хуком перерисовки.
- `ProgressBridge::noop()` — для CLI/тестов.
- `sink() -> Box<dyn ProgressSink>` — writer-сторона для background.
- `fraction() -> Option<f32>` — reader-сторона для UI (каждый кадр).
- `RepaintHook = Arc<dyn Fn() + Send + Sync>`.

#### `DownloadRequest` (`downloads/mod.rs`)

```rust
pub struct DownloadRequest { pub item: Item, pub progress: Option<Box<dyn ProgressSink>> }
```

`DownloadRequest::new(item)` или `DownloadRequest::with_progress(item, sink)`.

#### Download pipeline (`downloads/core/`)

Платформозависимый pipeline установки ядер:

- **`executable.rs`** — переименование с `#[cfg]`:
  - `CANONICAL_NAME` — `"core.exe"` / `"core.AppImage"` / `"core.dmg"` (const, cfg-gated).
  - `rename(dir, source)` — единая точка входа:
    - **Windows** (`#[cfg(target_os = "windows")]`): рекурсивно ищет `VoxelCore.exe` (а не любой `.exe`!) в распакованном архиве, перемещает в корень как `core.exe`. Остальные exe не трогает.
    - **Linux/macOS** (`#[cfg(not(target_os = "windows"))]`): переименовывает скачанный файл в `core.{ext}`.
  - `needs_extraction() -> bool` — `true` только на Windows.
  - `ext()` / `name()` — расширение и каноническое имя.

- **`pipeline.rs`** — `download_and_prepare()`:
  - Единая `prepare_inner()` с `#[cfg]` блоками внутри.
  - **Windows**: download zip → extract → `rename(dir, _)` → hash dir → commit.
  - **Linux/macOS**: download file → `rename(dir, source)` → hash dir → commit.
  - Общие шаги: staging dir, hash, `commit_extracted_dir`.

- **`install.rs`** — `Composer::install_cores()`: параллельный запуск pipeline через `stream::buffer_unordered(8)`.

#### Ошибки (`error.rs`)

```rust
pub enum ComposerError {
    Io, Serialise, Desirialise, Client, Validation, ValidationSingle,
    Archive { path, source: ArchiveError },
    CoreExecutableNotFound { path },
}
```

### `gui` (egui) ✅

Файлы: `src/main.rs`, `src/app.rs`.

**`main.rs`** — entry point:

- Tracing: `warn,clients=debug,composer=debug,gui=debug` (winit/eframe/egui заглушены).
- Создаёт tokio runtime, загружает `Composer::load()`.
- Создаёт `ComposerWorker::with_defaults()`.
- Запускает `worker.run()` через `LocalSet` + `spawn_local` в отдельном потоке (обход `!Send`).
- Запускает `eframe::run_native()`.
- При закрытии — `Command::Shutdown`, join worker thread.

**`app.rs`** — `App` (~620 строк):

- Хранит `WorkerHandle`, tokio runtime, `UiState`, `Option<ProgressBridge>`.
- `UiState` — кэш: `cores: Vec<(Hash, LockItem)>`, `instances`, `available_cores: Vec<Item>`, `core_validation`, `last_error`, `last_info`, `busy`.
- При создании запрашивает `GetCoresItems` + `GetInstancesItems`.
- `eframe::App::update()`: `drain_and_apply_events()` → render sidebar + tabs.
- Навигация: вкладки «Ядра» / «Инстансы».
- Вкладка «Ядра»: таблица установленных, кнопки Fetch/Validate/Save, доступные версии с GitHub.
- Установленные ядра помечаются в списке доступных: disabled кнопка `✔ Скачано` + кнопка `↻` (переустановка).
- Install с `ProgressBridge` + progress bar.
- `RepaintHook` через `ctx.request_repaint()`.
- `on_exit()` → `Command::Shutdown`.

### `cli` (clap) ✅

Файл: `src/main.rs`. Бинарь: `multivc`.

Прямые вызовы `Composer` без каналов:

- `multivc install <VERSION>` — поиск на GitHub → `install_cores()` → `save_cores()`.
- `multivc list` — таблица из `cores_items()` (hash, name, version, timestamp).
- `multivc fetch [--version X]` — `GithubClient::list()` → таблица (name, version, size).
- `multivc validate` — `validate_cores()` + `validate_instances()`.
- `multivc remove <HASH>` — TODO: нужен `Composer::remove_core()` (поля `pub(crate)`).

### `tui` (ratatui) 🔜

Планируется. Архитектура аналогична GUI: `WorkerHandle` + `Command`/`Event`.

---

## Архитектурные решения

### CLI отдельно от GUI/TUI

CLI — однозадачный (`request → response → exit`), не нуждается в каналах.
GUI/TUI — event-loop с долгоживущим состоянием, каналы необходимы.

### Нет единого trait `Frontend`

Разные API рендеринга, разный уровень детализации, async vs sync.
Общий код — `Composer` + `Command/Event` + `ProgressBridge`. Фронтенды тонкие.

### `worker.run()` не Send

Future содержит `dyn DynDigest` (не Send) в цепочке `install_cores`.
Решение: `LocalSet` + `spawn_local` в отдельном потоке.
В тестах — прямой `worker.run().await` без `tokio::spawn`.

### Платформозависимый pipeline через `#[cfg]`

Весь платформенный код за `#[cfg(target_os = "...")]`. Одна функция `rename()`,
одна `prepare_inner()`. Мёртвого кода в бинарнике нет.

---

## Диаграмма потоков данных

### GUI / TUI

```text
GUI/TUI                              Background
───────                              ──────────
WorkerHandle                         ComposerWorker
  .commands.send(Command) ──────────► .commands.recv()
                                           │
                                      Composer.method()
                                           │
  .events.recv() ◄──────────────────  .events.send(Event)

ProgressBridge:
  bridge.fraction() ◄── AtomicU64 ──► BridgeSink.update()
  (UI reads)            (lock-free)   (background writes)
```

### CLI

```text
main() → Composer::load()
       → composer.install_cores().await
       → composer.save().await
       → println!("✓ done")
```

---

## Граф зависимостей

```text
┌───────┐   ┌───────┐   ┌───────┐
│  gui  │   │  tui  │   │  cli  │
└───┬───┘   └───┬───┘   └───┬───┘
    │           │            │
    └─────┬─────┘            │
          ▼                  ▼
    ┌──────────┐       ┌──────────┐
    │ composer │       │ composer │
    │ (worker) │       │ (direct) │
    └─────┬────┘       └─────┬────┘
          └────────┬─────────┘
                   ▼
             ┌──────────┐
             │ clients  │
             └──────────┘
```

---

## TODO / Roadmap

- [x] Реализовать `ComposerWorker` в `composer/src/worker.rs`
- [x] Создать крейт `cli` с clap (subcommands: install, list, fetch, validate, remove — TODO)
- [x] Реализовать GUI в крейте `gui` с egui (навигация, таблицы, fetch, validate, install с прогрессом, пометка установленных)
- [x] `Command::GetCoresItems` / `Event::CoresItems` — синхронизация UI-кэша
- [x] Платформозависимый pipeline: Windows (zip + VoxelCore.exe) / Linux-macOS (direct file)
- [x] Автосохранение lock после install/remove в worker
- [x] Автосоздание директорий в `Lock::save()`
- [ ] Создать крейт `tui` с ratatui
- [ ] Добавить `Composer::remove_core()` / `remove_instance()` (публичные методы для CLI)
- [ ] Добавить content packs (провайдеры + `ContentsLock`)
- [ ] Прогресс-бар в CLI (indicatif или аналог)