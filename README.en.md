# MultiVC

[![CI](https://github.com/lost-umbrella-dev/MultiVC/actions/workflows/ci.yml/badge.svg)](https://github.com/lost-umbrella-dev/MultiVC/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

> Launcher for VoxelCore — a cross-platform application for managing installation, validation, and execution of VoxelCore versions.

[Русский](README.md)

## About

MultiVC is a version manager for VoxelCore. The project provides two separate applications:

- **GUI** — graphical interface built with egui/eframe for managing cores and instances through a windowed application.
- **CLI** — command-line interface built with clap for automation and server-side usage.

Both applications are built and distributed independently.

Key features:

- Install and update VoxelCore versions from GitHub Releases
- Validate integrity of downloaded files (SHA-256)
- Manage multiple instances with separate configurations
- Parallel downloads with progress reporting

## Supported Platforms

| Tier | Platform | Status                                            |
| ---- | -------- | ------------------------------------------------- |
| 1    | Windows  | Fully supported                                   |
| 2    | Linux    | Supported                                         |
| 3    | macOS    | Limited support (VoxelCore engine limitations)     |

## Installation

Pre-built binaries are available on the [GitHub Releases](https://github.com/lost-umbrella-dev/MultiVC/releases) page.

### GUI

```bash
gh release download --repo lost-umbrella-dev/MultiVC --pattern '*gui*'
```

### CLI

```bash
gh release download --repo lost-umbrella-dev/MultiVC --pattern '*cli*'
```

List available commands:

```bash
multivc --help
```

## Building from Source

Requirements:

- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)

```bash
git clone https://github.com/lost-umbrella-dev/MultiVC.git
cd MultiVC
```

Build GUI:

```bash
cargo build -p gui --release
```

Build CLI:

```bash
cargo build -p cli --release
```

Compiled binaries are located in `target/release/`.

## License

This project is dual-licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).
