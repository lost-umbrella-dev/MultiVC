# Архитектура MultiVC

## Обзор

**MultiVC** — лаунчер для [VoxelCore](https://github.com/MihailRis/VoxelCore), построенный как Cargo workspace.

| Крейт      | Роль                                  | Статус        |
|------------|---------------------------------------|---------------|
| `clients`  | HTTP-клиенты, загрузки, типы          | Реализован    |
| `composer` | Бизнес-логика, lock-файлы, пайплайны  | Реализован    |
| `gui`      | Графический интерфейс (egui)          | Реализован    |
| `cli`      | Командная строка (clap)               | Реализован    |

---

## Крейты

### `clients`

HTTP-клиенты, download pipeline и общие типы. **Не зависит** от других крейтов.

- **`Clients`** — набор клиентов (`pub core: GithubClient`).
- **`GithubClient`** — GitHub API: `list()`, `get()`. Репо: `"MihailRis"` / `"VoxelCore"`.
- **`Item`** — `{ name, version, url, zipball_url, hash, size, dependencies, supported_engine }`.
- **`Hash`** — `SHA256(String) | SHA512(String)`, Display как `"sha256:..."`, FromStr.
- **`Version`** — семантическая версия с опциональным `v` префиксом, Ord, Display, FromStr.
- **`DownloadProgress`** / **`ProgressSink`** — трейт прогресса загрузки.
- **`download()`** — скачивание с валидацией хэша и прогрессом.

### `composer`

Бизнес-логика. **Зависит от `clients`.**

#### `Composer` — фасад

```rust
pub struct Composer {
    pub(crate) clients: Clients,
    pub(crate) instances: InstancesLock,
    pub(crate) cores: CoresLock,
}
```

API: `new()`, `load()`, `save()`, `save_cores()`, `save_instances()`, `cores_items()`, `instances_items()`, `validate_cores()`, `validate_instances()`, `install_cores()`, `remove_core()`, `create_instance()`, `get_instance()`, `edit_instance()`, `remove_instance()`, `launch_instance()`, `launch_instance_cmd()`, `build_launch_args()`, `instances_using_core()`, `core_dependents_map()`, `cores_with_dependents()`, `instances_with_details()`.

#### `Lock` trait

Обобщённая логика для lock-файлов: `load()`, `save()`, `remove()`, `validate_dir()`.

- **`CoresLock`** реализует `Lock`. Ключ — `Hash`. `cores/lock.toml`, папка `cores/`.
- **`InstancesLock`** **не** реализует `Lock` (ключ — имя, не хэш). Своя реализация `load`/`save`/`remove`/`validate_dir`.
- **`Instance`** — конфигурация инстанса (`instance.toml`): `{ description, core_version: Hash, dependencies }`.
- **`InstancesItem`** — UI-метаданные в lock: `{ icon, banner, last_launch, created_at }`.

#### Типы данных

- `LockItem` = `{ item: Item, timestamp }`. `LockMap` = `DashMap<Hash, LockItem>`.
- `InstancesMap` = `DashMap<String, InstancesItem>`.
- `ValidateReason` — `HashNotMatcher | NotFound` (для cores).
- `InstanceValidateReason` — `NotFound` (для instances).

#### `Command` / `Event` протокол

Протокол для канала UI <-> Background. CLI не использует.

**Commands:** Save, SaveCores, SaveInstances, InstallCores, ValidateCores, ValidateInstances, RemoveCore, CreateInstance, GetInstance, EditInstance, RemoveInstance, GetCoresItems, GetInstancesItems, FetchCoresList, FetchCore, LaunchInstance, StopInstance, GetInstanceDirSize, Shutdown.

**Events:** Saved, CoresSaved, InstancesSaved, CoresInstalled, CoresValidated, InstancesValidated, CoreRemoved, InstanceCreated, InstanceInfo, InstanceEdited, InstanceRemoved, CoresFetched, CoreFetched, CoresItems, InstancesItems, InstanceLaunched, InstanceStopped, InstanceDirSize, Error, ShutdownComplete.

#### `ComposerWorker`

Background runner для GUI/TUI. Хранит `running: HashMap<String, Child>` для запущенных процессов. Поллит `try_wait()` каждые 500ms через `tokio::select!`. Автосохраняет lock после мутирующих операций. Автоматически отправляет snapshots после мутаций.

#### `ProgressBridge`

Lock-free мост прогресса (`Arc<AtomicU64>`). `RepaintHook = Arc<dyn Fn() + Send + Sync>` — абстракция хука перерисовки (egui/ratatui/noop).

#### Download pipeline (`downloads/core/`)

- **`executable.rs`** — `CANONICAL_NAME`: `"core.exe"` (Win) / `"core.AppImage"` (Linux) / `"core.dmg"` (macOS). `rename()` — единая точка переименования. `needs_extraction()` — `true` только на Windows.
- **`pipeline.rs`** — `download_and_prepare()`: Windows: zip -> extract -> rename -> hash -> commit. Linux/macOS: download + zipball res/ -> rename -> hash -> commit.
- **`install.rs`** — `Composer::install_cores()`: параллельно через `buffer_unordered(8)`.

#### Ошибки

`ComposerError`: Io, Serialise, Desirialise, Client, Validation, Archive, CoreExecutableNotFound, ZipballUrlMissing, ResNotFound, CoreInUse, InstanceAlreadyExists, InstanceNotFound, LaunchExeNotFound.

### `gui` (egui)

Единый крейт: host binary + UI rendering (views, widgets, icons, i18n, toasts, state).

**`main.rs`** — entry point: tracing, tokio runtime, `Composer::load()`, `ComposerWorker` в `LocalSet` + `spawn_local` (обход `!Send`), `eframe::run_native()`.

**`app.rs`** — `App`: хранит `WorkerHandle`, runtime, `Tab`, `UiState`. `drain_and_apply_events()` на каждом кадре. `on_exit()` -> `Command::Shutdown` + сохранение настроек.

**`ui/`** — модули: `state` (UiState, CoresTabState, InstancesTabState, InstancePanelState, SettingsState), `views/` (sidebar, cores_tab, instances_tab с panel, settings_modal), `widgets/` (progress_ring, image picker), `icons`, `lang` (EN/RU i18n), `toasts`, `download_tracker`, `settings`.

### `cli` (clap)

Бинарь `multivc`. Прямые вызовы `Composer` без каналов.

Подкоманды: `install`, `list` (ls), `fetch`, `validate` (check), `remove` (rm), `list-instances` (ps), `create-instance` (new), `remove-instance` (rmi), `launch` (run/start).

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
       → composer.install_cores() / .launch_instance() / ...
       → composer.save()
       → exit
```

---

## Граф зависимостей

```text
┌───────┐              ┌───────┐
│  gui  │              │  cli  │
└───┬───┘              └───┬───┘
    │                      │
    └──────────┬───────────┘
               ▼
         ┌──────────┐
         │ composer  │
         └─────┬────┘
               ▼
         ┌──────────┐
         │ clients   │
         └──────────┘
```
