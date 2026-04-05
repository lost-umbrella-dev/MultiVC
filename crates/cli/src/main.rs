//! CLI для MultiVC — лаунчер VoxelCore.
//!
//! Clap-based интерфейс командной строки, работающий напрямую с [`Composer`]
//! без каналов Command/Event (они нужны только GUI/TUI).

use std::str::FromStr;

use clap::{Parser, Subcommand};
use clients::{
    Clients,
    github::{GitHubGetOptions, GitHubListOptions, GithubClient},
    hash::Hash,
};
use composer::{Composer, DownloadRequest, lock::ValidateReason};

// ── CLI определения ──────────────────────────────────────────────────

/// CLI лаунчер для VoxelCore
#[derive(Parser)]
#[command(name = "multivc", version, about = "CLI лаунчер для VoxelCore")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Доступные подкоманды
#[derive(Subcommand)]
enum Commands {
    /// Установить ядро указанной версии
    Install {
        /// Версия ядра для установки (например "1.2.3")
        version: String,
    },

    /// Показать список установленных ядер
    List,

    /// Получить список доступных версий с GitHub
    Fetch {
        /// Фильтр по версии (можно указать несколько: --version 1.0 --version 2.0)
        #[arg(long = "version")]
        versions: Vec<String>,
    },

    /// Проверить целостность установленных ядер и инстансов
    Validate,

    /// Удалить ядро по хэшу
    Remove {
        /// Хэш ядра (формат: sha256:abcdef... или sha512:abcdef...)
        hash: String,
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
            let query_client = create_github_client()?;
            let item = query_client
                .get(GitHubGetOptions {
                    version: version.clone(),
                })
                .await?
                .ok_or_else(|| format!("Ядро версии «{version}» не найдено на GitHub"))?;

            println!("Найдено: {} v{} ({})", item.name, item.version, format_size(item.size),);

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
        Commands::Remove { hash } => {
            let _hash = Hash::from_str(&hash).map_err(|e| format!("Некорректный формат хэша «{hash}»: {e}"))?;

            // TODO: добавить Composer::remove_core(&self, hash: &Hash)
            //
            // Предполагаемая логика:
            //   1. composer.remove_core(&_hash).await?;
            //   2. composer.save_cores().await?;
            //
            // Поля cores/instances в Composer имеют видимость pub(crate),
            // поэтому нужен публичный метод remove_core() на стороне composer.
            unimplemented!("TODO: add Composer::remove_core() — поля Lock pub(crate)");
        },
    }

    Ok(())
}
