//! CLI для MultiVC — лаунчер VoxelCore.
//!
//! Clap-based интерфейс командной строки, работающий напрямую с [`Composer`]
//! без каналов Command/Event (они нужны только GUI/TUI).

use clap::{Parser, Subcommand};
use clients::{
    Clients,
    github::{GitHubGetOptions, GitHubListOptions, GithubClient},
    hash::Hash,
    version::Version,
};
use composer::{
    Composer, DownloadRequest,
    item::{LockItem, LockMap},
    lock::ValidateReason,
};

// ── CLI определения ──────────────────────────────────────────────────

/// CLI лаунчер для VoxelCore
#[derive(Parser)]
#[command(
    name = "multivc",
    version,
    author = "TOwInOK",
    about = "MultiVC — CLI лаунчер для VoxelCore",
    long_about = "MultiVC — CLI лаунчер для VoxelCore.\n\n\
        Управляет установкой, обновлением и удалением ядер VoxelCore.\n\
        Поддерживает автоматическую загрузку с GitHub, валидацию целостности\n\
        и хранение состояния в lock-файлах.",
    after_help = "Примеры:\n  \
        multivc install 0.31.1       Установить ядро v0.31.1\n  \
        multivc ls                   Список установленных ядер\n  \
        multivc fetch                Все доступные версии с GitHub\n  \
        multivc rm 0.31.1            Удалить ядро по версии\n  \
        multivc check                Проверить целостность\n\n\
        Репозиторий: https://github.com/lost-umbrella-dev/MultiVC"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Доступные подкоманды
#[derive(Subcommand)]
enum Commands {
    /// Установить ядро указанной версии
    #[command(visible_aliases = ["i", "add", "get"])]
    Install {
        /// Версия ядра для установки (например "0.24.0" или "v0.24.0")
        version: Version,
    },

    /// Показать список установленных ядер
    #[command(visible_aliases = ["ls", "l"])]
    List,

    /// Получить список доступных версий с GitHub
    #[command(visible_aliases = ["f", "search"])]
    Fetch {
        /// Фильтр по версии (можно указать несколько: --version 1.0.0 --version 2.0.0)
        #[arg(long = "version")]
        versions: Vec<Version>,
    },

    /// Проверить целостность установленных ядер и инстансов
    #[command(visible_aliases = ["check", "verify", "v"])]
    Validate,

    /// Удалить установленное ядро (по версии или префиксу хэша)
    #[command(visible_aliases = ["rm", "r", "uninstall", "delete"])]
    Remove {
        /// Версия (например "0.31.1") или начало хэша (например "sha256:ab" или "ab3f")
        query: String,
    },
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Создаёт GitHub-клиент для репозитория VoxelCore.
fn create_github_client() -> Result<GithubClient, Box<dyn std::error::Error>> {
    Ok(GithubClient::new("MihailRis".to_owned(), "VoxelCore".to_owned())?)
}

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

/// Выводит причины непрохождения валидации.
fn print_validate_reasons(label: &str, reasons: &[ValidateReason]) {
    if reasons.is_empty() {
        return;
    }
    println!("{label}:");
    for reason in reasons {
        match reason {
            ValidateReason::HashNotMatcher(hash, lock_item) => {
                println!(
                    "  \u{2717} Хэш не совпадает: «{}» v{} (ожидался {hash})",
                    lock_item.item.name, lock_item.item.version,
                );
            },
            ValidateReason::NotFound(hash, lock_item) => {
                println!(
                    "  \u{2717} Файл не найден: «{}» v{} ({hash})",
                    lock_item.item.name, lock_item.item.version,
                );
            },
        }
    }
}

/// Создаёт `Clients` и загружает `Composer` с диска.
async fn load_composer() -> Result<Composer, Box<dyn std::error::Error>> {
    let clients = Clients::new(create_github_client()?);
    Ok(Composer::load(clients).await?)
}

/// Ищет ядра по запросу: версия или префикс хэша.
///
/// 1. Если `query` парсится как [`Version`] — ищет по версии (с дефолтным `v` префиксом).
/// 2. Иначе — ищет по префиксу строкового представления хэша (`sha256:abc...`)
///    или по префиксу hex-части (`abc...`).
fn find_cores_by_query(cores: &LockMap, query: &str) -> Vec<(Hash, LockItem)> {
    // 1. Пробуем распарсить как версию
    if let Ok(version) = query.parse::<Version>() {
        let prefixed = version.clone().with_default_prefix().to_string();
        let literal = version.to_string();
        let matches: Vec<_> = cores
            .iter()
            .filter(|entry| {
                let v = &entry.value().item.version;
                *v == prefixed || *v == literal
            })
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();
        if !matches.is_empty() {
            return matches;
        }
    }

    // 2. Поиск по префиксу хэша (полному "sha256:ab..." или только hex "ab...")
    cores
        .iter()
        .filter(|entry| {
            let hash_str = entry.key().to_string();
            hash_str.starts_with(query) || hash_str.split_once(':').is_some_and(|(_, hex)| hex.starts_with(query))
        })
        .map(|entry| (entry.key().clone(), entry.value().clone()))
        .collect()
}

// ── Entry point ──────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Инициализация логирования; уровень задаётся через RUST_LOG
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        // ── install ──────────────────────────────────────────────
        Commands::Install { version } => {
            // Ищем ядро нужной версии через GitHub API
            let version = version.with_default_prefix();
            let query_client = create_github_client()?;
            let item = query_client
                .get(GitHubGetOptions {
                    version: version.clone(),
                })
                .await?
                .ok_or_else(|| format!("Ядро версии «{version}» не найдено на GitHub"))?;

            println!("Найдено: {} {} ({})", item.name, item.version, format_size(item.size),);

            // Создаём Composer и запускаем установку (без прогресс-бара)
            let composer = load_composer().await?;
            let request = DownloadRequest::new(item);
            let result = composer.install_cores(vec![request]).await?;

            // Сохраняем lock-файл — даже при частичном успехе
            composer.save_cores().await?;
            tracing::debug!("lock-файл сохранён");

            if let Some(errors) = result {
                if !errors.is_empty() {
                    eprintln!("Ошибки при установке:");
                    for (failed_item, err) in &errors {
                        eprintln!("  \u{2717} «{}»: {err}", failed_item.name);
                    }
                    std::process::exit(1);
                }
            }

            println!("\u{2713} Установка завершена успешно");
        },

        // ── list ─────────────────────────────────────────────────
        Commands::List => {
            let composer = load_composer().await?;
            let cores = composer.cores_items();

            if cores.is_empty() {
                println!("Установленных ядер нет.");
                return Ok(());
            }

            println!("{:<40} {:<20} {:<12} {}", "HASH", "NAME", "VERSION", "TIMESTAMP",);
            println!("{}", "\u{2500}".repeat(92));

            for entry in cores.iter() {
                let hash = entry.key();
                let lock_item = entry.value();

                // Обрезаем хэш чтобы таблица не разъезжалась
                let hash_str = hash.to_string();
                let hash_display = if hash_str.len() > 37 {
                    format!("{}..", &hash_str[..37])
                } else {
                    hash_str
                };

                println!(
                    "{:<40} {:<20} {:<12} {}",
                    hash_display, lock_item.item.name, lock_item.item.version, lock_item.timestamp,
                );
            }

            println!("\nВсего: {}", cores.len());
        },

        // ── fetch ────────────────────────────────────────────────
        Commands::Fetch { versions } => {
            let client = create_github_client()?;
            let versions = versions.into_iter().map(Version::with_default_prefix).collect();
            let items = client
                .list(GitHubListOptions {
                    search_version: versions,
                })
                .await?;

            if items.is_empty() {
                println!("Доступных версий не найдено.");
                return Ok(());
            }

            println!("{:<30} {:<15} {:>10}", "NAME", "VERSION", "SIZE");
            println!("{}", "\u{2500}".repeat(58));

            for item in &items {
                println!("{:<30} {:<15} {:>10}", item.name, item.version, format_size(item.size),);
            }

            println!("\nВсего: {}", items.len());
        },

        // ── validate ─────────────────────────────────────────────
        Commands::Validate => {
            let composer = load_composer().await?;

            let core_issues = composer.validate_cores().await?;
            let instance_issues = composer.validate_instances().await?;

            print_validate_reasons("Проблемы с ядрами", &core_issues);
            print_validate_reasons("Проблемы с инстансами", &instance_issues);

            if core_issues.is_empty() && instance_issues.is_empty() {
                println!("\u{2713} Всё валидно");
            } else {
                let total = core_issues.len() + instance_issues.len();
                eprintln!("Найдено проблем: {total}");
                std::process::exit(1);
            }
        },

        // ── remove ───────────────────────────────────────────────
        Commands::Remove { query } => {
            let composer = load_composer().await?;
            let matches = find_cores_by_query(composer.cores_items(), &query);

            match matches.len() {
                0 => {
                    eprintln!("Ядро не найдено по запросу «{query}»");
                    std::process::exit(1);
                },
                1 => {
                    let (hash, lock_item) = &matches[0];
                    println!("Удаляю: «{}» {} ({hash})", lock_item.item.name, lock_item.item.version,);
                    composer.remove_core(hash).await?;
                    composer.save_cores().await?;
                    println!("\u{2713} Ядро удалено");
                },
                n => {
                    eprintln!("Найдено {n} совпадений, уточните запрос:");
                    for (hash, lock_item) in &matches {
                        let hash_str = hash.to_string();
                        let hash_short = if hash_str.len() > 20 {
                            format!("{}..", &hash_str[..20])
                        } else {
                            hash_str
                        };
                        eprintln!("  {} {} ({})", lock_item.item.name, lock_item.item.version, hash_short,);
                    }
                    std::process::exit(1);
                },
            }
        },
    }

    Ok(())
}
