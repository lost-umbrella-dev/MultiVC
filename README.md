# MultiVC

[![CI](https://github.com/lost-umbrella-dev/MultiVC/actions/workflows/ci.yml/badge.svg)](https://github.com/lost-umbrella-dev/MultiVC/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

> Лаунчер для VoxelCore — кроссплатформенное приложение для управления установкой, валидацией и запуском версий VoxelCore.

[English](README.en.md)

## О проекте

MultiVC — менеджер версий VoxelCore. Проект предоставляет два отдельных приложения:

- **GUI** — графический интерфейс на базе egui/eframe для управления ядрами и инстансами через оконное приложение.
- **CLI** — интерфейс командной строки на базе clap для автоматизации и серверного использования.

Оба приложения собираются и распространяются независимо друг от друга.

Основные возможности:

- Установка и обновление версий VoxelCore из GitHub Releases
- Валидация целостности загруженных файлов (SHA-256)
- Управление несколькими инстансами с отдельными конфигурациями
- Параллельная загрузка с отображением прогресса

## Поддерживаемые платформы

| Tier | Платформа | Статус                                                    |
| ---- | --------- | --------------------------------------------------------- |
| 1    | Windows   | Полная поддержка                                          |
| 2    | Linux     | Поддерживается                                            |
| 3    | macOS     | Ограниченная поддержка (ограничения на стороне VoxelCore) |

## Установка

Готовые бинарники доступны на странице [GitHub Releases](https://github.com/lost-umbrella-dev/MultiVC/releases).

### GUI

```bash
gh release download --repo lost-umbrella-dev/MultiVC --pattern '*gui*'
```

### CLI

```bash
gh release download --repo lost-umbrella-dev/MultiVC --pattern '*cli*'
```

Обзор доступных команд:

```bash
multivc --help
```

## Сборка из исходников

Требования:

- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)

```bash
git clone https://github.com/lost-umbrella-dev/MultiVC.git
cd MultiVC
```

Сборка GUI:

```bash
cargo build -p gui --release
```

Сборка CLI:

```bash
cargo build -p cli --release
```

Собранные бинарники находятся в `target/release/`.

## Лицензия

Проект распространяется под двойной лицензией: [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).
