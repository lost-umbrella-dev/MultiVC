<p align="center">
  <img src="docs/assets/logo.svg" width="128" alt="MultiVC logo">
</p>

<h1 align="center">MultiVC</h1>

<p align="center">
  Nix-like version manager for <a href="https://github.com/MihailRis/voxelcore">VoxelCore</a>
</p>

<p align="center">
  <a href="https://github.com/lost-umbrella-dev/MultiVC/actions/workflows/ci.yml"><img src="https://github.com/lost-umbrella-dev/MultiVC/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg" alt="License: MIT OR Apache-2.0"></a>
</p>

<p align="center">
  <a href="README.md">Русский</a>
</p>

---

MultiVC manages [VoxelCore](https://github.com/MihailRis/voxelcore) engine versions as dependencies: each version is installed once, and instances (game worlds) reference the version they need by hash. Available as **GUI** (egui) and **CLI** (clap).

## Screenshots

| Cores | Instances |
|:-----:|:---------:|
| ![Cores tab](docs/assets/gui-cores.webp) | ![Instances tab](docs/assets/gui-instances.webp) |

<p align="center">
  <img src="docs/assets/cli-demo.gif" alt="CLI demo" width="600">
</p>

## Installation

Pre-built binaries on the [Releases](https://github.com/lost-umbrella-dev/MultiVC/releases) page. Archives include `settings.toml` next to the executable — portable mode is active right after unpacking.

## Quick Start (CLI)

```bash
multivc install 0.31.1       # Install a core
multivc ls                   # List installed cores
multivc fetch                # Available versions from GitHub

multivc new my_world 0.31.1  # Create an instance
multivc instances            # List instances
multivc launch my_world      # Launch the game

multivc check                # Validate integrity
multivc rm 0.31.1            # Remove a core
```

## Features

- Install and update VoxelCore versions from GitHub Releases
- Instance management — separate configs, dependencies, launch
- File integrity validation (SHA-256)
- Parallel downloads with progress reporting
- Launch and stop instances from GUI and CLI
- i18n — Russian and English interface (GUI)

## How It Works

Each engine version is stored in its own directory by hash. An instance references a core — you cannot delete a core while at least one instance depends on it.

By default, data is stored in user directories:

| Type   | Windows                   | Linux                      | macOS                                    |
|--------|---------------------------|----------------------------|------------------------------------------|
| Config | `%APPDATA%\MultiVC\`      | `~/.config/MultiVC/`       | `~/Library/Application Support/MultiVC/` |
| Data   | `%LOCALAPPDATA%\MultiVC\` | `~/.local/share/MultiVC/`  | `~/Library/Application Support/MultiVC/` |

**Portable mode** — if `settings.toml` is placed next to the executable, everything is stored there. Force it: `--portable`.

```
<data dir>/
├── cores/                # installed engine versions
│   ├── lock.toml         # registry: hash → version, timestamp
│   └── sha256:a1b2c3…/   # directory for a specific version
│       ├── core.exe      # executable (Windows)
│       └── res/          # engine resources
└── instances/            # game worlds
    ├── lock.toml         # registry: name → metadata
    └── my_world/         # instance directory
        └── instance.toml # config: core, description, dependencies
```

## Platforms

| Tier | Platform | Status |
|------|----------|--------|
| 1    | Windows  | Fully supported |
| 2    | Linux    | Supported |
| 3    | macOS    | Limited (VoxelCore engine limitations) |

## Building from Source

Requirements: [Rust](https://www.rust-lang.org/tools/install) (stable)

```bash
git clone https://github.com/lost-umbrella-dev/MultiVC.git
cd MultiVC
cargo build -p gui --release   # GUI
cargo build -p cli --release   # CLI
```

Binaries in `target/release/`.

## License

[MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE)
