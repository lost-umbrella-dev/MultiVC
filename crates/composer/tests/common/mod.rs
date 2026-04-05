#![allow(dead_code)]
//! Общие утилиты для тестов `composer`.
//!
//! Содержит хелперы для инициализации tracing, создания тестовых данных
//! и работы с временными директориями.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::Utc;
use clients::hash::Hash;
use clients::item::Item;
use composer::item::{LockItem, LockMap};
use tracing::subscriber::DefaultGuard;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_tree::HierarchicalLayer;

/// Инициализирует tracing для тестов.
///
/// Возвращает `DefaultGuard` — пока он жив, логи пишутся.
/// Уровень берётся из `RUST_LOG`, по умолчанию `debug`.
pub fn init_test_tracing() -> DefaultGuard {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));

    let subscriber = tracing_subscriber::registry().with(filter).with(
        HierarchicalLayer::new(2)
            .with_ansi(true)
            .with_targets(true)
            .with_bracketed_fields(true)
            .with_thread_names(false)
            .with_indent_lines(true),
    );

    tracing::subscriber::set_default(subscriber)
}

/// Создаёт тестовый `Item` с заданным именем и версией.
pub fn make_item(name: &str, version: &str) -> Item {
    Item {
        name: name.to_owned(),
        version: version.to_owned(),
        url: format!("https://example.com/{name}/{version}"),
        hash: None,
        size: 1024,
        dependencies: None,
        supported_engine: None,
    }
}

/// Создаёт тестовый `LockItem` с текущим временем.
pub fn make_lock_item(name: &str, version: &str) -> LockItem {
    LockItem {
        item: make_item(name, version),
        timestamp: Utc::now(),
    }
}

/// Создаёт `Hash::SHA256` из произвольной строки (не настоящий хэш, только для ключей).
pub fn fake_hash(value: &str) -> Hash {
    Hash::SHA256(value.to_owned())
}

/// Создаёт `LockMap` с `n` элементами и возвращает пары `(Hash, LockItem)`.
///
/// Хэши генерируются как `sha256:test_hash_0`, `sha256:test_hash_1`, и т.д.
pub fn make_lock_map(n: usize) -> (LockMap, Vec<(Hash, LockItem)>) {
    let map = LockMap::new();
    let mut pairs = Vec::with_capacity(n);

    for i in 0..n {
        let hash = fake_hash(&format!("test_hash_{i}"));
        let item = make_lock_item(&format!("item_{i}"), &format!("1.0.{i}"));
        map.insert(hash.clone(), item.clone());
        pairs.push((hash, item));
    }

    (map, pairs)
}

/// Создаёт файл с произвольным содержимым внутри директории.
pub fn write_test_file(dir: &Path, name: &str, content: &[u8]) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("не удалось создать родительскую директорию");
    }
    std::fs::write(&path, content).expect("не удалось записать тестовый файл");
}

/// Создаёт простой ZIP-архив в памяти и записывает в указанный путь.
///
/// `files` — список `(имя_в_архиве, содержимое)`.
pub fn create_test_zip(archive_path: &Path, files: &[(&str, &[u8])]) {
    let file = std::fs::File::create(archive_path).expect("не удалось создать файл архива");
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for (name, content) in files {
        zip.start_file(*name, options).expect("не удалось начать файл в архиве");
        std::io::Write::write_all(&mut zip, content).expect("не удалось записать данные в архив");
    }

    zip.finish().expect("не удалось завершить запись архива");
}

/// Счётчик вызовов (для тестирования repaint hook и подобного).
#[derive(Clone)]
pub struct CallCounter {
    count: Arc<AtomicU64>,
}

impl CallCounter {
    pub fn new() -> Self {
        Self {
            count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Возвращает замыкание, которое увеличивает счётчик при вызове.
    pub fn hook(&self) -> Arc<dyn Fn() + Send + Sync> {
        let count = Arc::clone(&self.count);
        Arc::new(move || {
            count.fetch_add(1, Ordering::Relaxed);
        })
    }

    /// Текущее значение счётчика.
    pub fn get(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }
}
