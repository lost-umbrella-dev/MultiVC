# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

MultiVC is a Rust workspace launcher for VoxelCore — a cross-platform desktop app managing installation, validation, and execution of VoxelCore versions. It has both a GUI (egui/eframe) and a CLI (clap) frontend.

## Commands

```bash
# Build
cargo build --workspace
cargo build --release

# Test (always use these flags)
cargo test --workspace --all-features --all-targets --locked

# Run a single test
cargo test -p composer --test lock test_name

# Lint/Format
cargo clippy --workspace --all-features --all-targets --locked -- -D warnings
cargo fmt --all
cargo fmt --all -- --check   # CI check

# Docs
cargo doc --workspace --all-features --no-deps

# Run GUI
cargo run -p gui

# Run CLI
cargo run -p cli -- --help

# Hot-reload development (GUI)
# Terminal 1: run the app
cargo run -p gui
# Terminal 2: watch for UI changes and recompile dylib
cargo watch -w crates/gui-ui -x 'build -p gui-ui'
```

## Workspace Structure

```
crates/
  clients/   — HTTP clients, GitHub API, download + hash validation, core types (Item, Hash, Version)
  composer/  — Core business logic: Lock trait, installation pipeline, worker/channel protocol
  cli/       — clap CLI; calls Composer directly (no channels)
  gui/       — egui/eframe host binary; event handling, eframe::App, startup
  gui-ui/    — GUI rendering library (dylib); views, widgets, icons, i18n, toasts, state
docs/
  ARCHITECTURE.md  — detailed crate-level architecture reference
.zed/
  tasks.json — Zed editor task definitions (build, lint, test, run, hot-reload)
```

## Architecture

**Dependency order:** `cli`/`gui` → `gui-ui` → `composer` → `clients` → GitHub API/filesystem

### gui / gui-ui Split
`gui-ui` is a library crate (`rlib` + `dylib`) containing all UI rendering: views, widgets, icons, i18n, toasts, state types. `gui` is the host binary containing the eframe::App, event handling, and startup code. This split enables hot-reload during development — `gui-ui` is recompiled as a dylib and loaded on the fly without restarting the app. In release builds, everything is statically linked.

### Lock Trait (composer)
`CoresLock` and `InstancesLock` both implement the `Lock` trait, which handles TOML persistence (`load`/`save`/`remove`/`validate_dir`). Each lock file lives at `{folder}/lock.toml`. `LockMap` is a `DashMap<Hash, LockItem>` used for concurrent access.

### Command/Event Protocol (composer ↔ gui/tui)
GUI sends `Command` variants via `mpsc` to `ComposerWorker`; worker responds with `Event` variants. CLI bypasses this and calls `Composer` methods directly.

### Non-Send Worker (gui)
The download pipeline uses `dyn DynDigest` which is `!Send`. The `ComposerWorker` therefore runs on a `tokio::task::LocalSet` in a dedicated thread spawned from `main.rs`.

### Platform-Specific Executables
`crates/composer/src/downloads/core/executable.rs` uses `#[cfg(target_os = "...")]` constants — no runtime branching. Windows gets ZIP extraction; Linux/macOS get direct binaries.

### Progress Reporting
`ProgressBridge` wraps `Arc<AtomicU64>` so the download background task writes bytes without contention, and the GUI reads fraction lock-free on every frame.

### Instance Panel (gui-ui)
Each instance has a tabbed floating panel (Info / Log / Settings). The Info tab shows icon, banner, description, core version, PID, created_at, disk size, dependencies, path. The Log tab has level filters ([I]/[W]/[E]). The Settings tab allows editing description, icon, and banner with native file picker and auto WebP conversion.

### i18n (gui-ui)
`lang.rs` provides `t(key, lang) -> &'static str` for EN/RU translations. Language is persisted in `settings.toml`.

## Key Conventions

- **Error handling:** `thiserror` throughout; rich error enums in each crate (`ClientError`, `ComposerError`)
- **Async:** `tokio` runtime; `futures_util::stream::buffer_unordered` for parallel downloads (8 concurrent)
- **Tests:** integration tests under `crates/{crate}/tests/`; use `CWD_LOCK` mutex for tests that mutate `std::env::current_dir()`; use `tempfile::TempDir` for isolation; init tracing via `init_test_tracing()` from `tests/common/mod.rs`
- **Release profile:** `opt-level = "z"`, `lto = "thin"`, `panic = "abort"`, `strip = "symbols"` — keep binary size minimal
- **TOML formatting:** `.taplo.toml` enforces 120-column width; run `taplo fmt` if editing TOML files
- **Debug hotkeys (debug builds only):** F12 = debug_on_hover, F11 = show_widget_hits, F10 = Inspector window
