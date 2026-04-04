/*
  Как используется

 **UI-сторона** создаёт bridge с хуком перерисовки и читает прогресс на каждом кадре:

 // egui
 let ctx = ctx.clone();
 let hook: RepaintHook = Arc::new(move || ctx.request_repaint());
 let bridge = ProgressBridge::new(hook);

 // на каждом кадре — lock-free чтение
 let fraction = bridge.fraction(); // Option<f32>
 ui.add(egui::ProgressBar::new(fraction.unwrap_or(0.0)));

 // ratatui
 let tx = repaint_tx.clone();
 let hook: RepaintHook = Arc::new(move || { tx.send(Event::Repaint).ok(); });
 let bridge = ProgressBridge::new(hook);

 **Background-сторона** получает `Box<dyn ProgressSink>` и не знает ни про какой UI:

 let request = DownloadRequest::with_progress(item, bridge.sink());
 lock.download(&client, vec![request]).await?;
*/
//!  UI Thread                              Background Thread
//! ─────────                              ─────────────────
//! bridge.fraction()                      bridge.sink() → Box<dyn ProgressSink>
//!     │                                       │
//!     │  Relaxed atomic read                  │  .update(DownloadProgress)
//!     ▼                                       ▼
//! ┌──────────────────────────────────────────────┐
//! │            ProgressBridgeInner               │
//! │  downloaded: AtomicU64                       │
//! │  total:      AtomicU64                       │
//! │  repaint:    Arc<dyn Fn() + Send + Sync>  ←──── будит UI
//! └──────────────────────────────────────────────┘

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use clients::prelude::{DownloadProgress, ProgressSink};

/// Максимально абстрактный хук для перерисовки UI.
///
/// - egui: `Arc::new(move || ctx.request_repaint())`
/// - ratatui: `Arc::new(move || tx.send(Event::Repaint).ok(); )`
/// - noop: `Arc::new(|| ())`
pub type RepaintHook = Arc<dyn Fn() + Send + Sync>;

/// Framework-agnostic мост между [`ProgressSink`] (writer-сторона в download pipeline)
/// и UI (reader-сторона для отрисовки прогресс-баров).
///
/// Внутри — атомики, поэтому чтение из UI-потока и запись из background-потока
/// не блокируют друг друга.
///
/// # Пример
///
/// ```ignore
/// // UI-сторона: создаём bridge с хуком перерисовки
/// let hook: RepaintHook = Arc::new(move || ctx.request_repaint());
/// let bridge = ProgressBridge::new(hook);
///
/// // Передаём sink в download pipeline
/// let request = DownloadRequest::with_progress(item, bridge.sink());
///
/// // В update() читаем прогресс
/// let progress = bridge.snapshot();
/// if let Some(fraction) = progress.fraction() {
///     ui.add(egui::ProgressBar::new(fraction));
/// }
/// ```
#[derive(Clone)]
pub struct ProgressBridge {
    inner: Arc<ProgressBridgeInner>,
}

struct ProgressBridgeInner {
    downloaded: AtomicU64,
    total: AtomicU64,
    repaint: RepaintHook,
}

// ── Construction ──────────────────────────────────────────────────────

impl ProgressBridge {
    /// Создаёт новый bridge с хуком перерисовки.
    pub fn new(repaint: RepaintHook) -> Self {
        Self {
            inner: Arc::new(ProgressBridgeInner {
                downloaded: AtomicU64::new(0),
                total: AtomicU64::new(0),
                repaint,
            }),
        }
    }

    /// Создаёт bridge без хука (noop). Подходит для headless / тестов.
    pub fn noop() -> Self {
        Self::new(Arc::new(|| ()))
    }
}

// ── Writer side (background thread) ──────────────────────────────────

/// Обёртка, которую можно передать как `Box<dyn ProgressSink>`.
///
/// Отделена от [`ProgressBridge`] чтобы UI-сторона не могла случайно
/// реализовать `ProgressSink` и вызвать `update` напрямую.
struct BridgeSink {
    inner: Arc<ProgressBridgeInner>,
}

impl ProgressSink for BridgeSink {
    fn update(&self, progress: DownloadProgress) {
        self.inner.downloaded.store(progress.downloaded, Ordering::Relaxed);

        if let Some(total) = progress.total {
            self.inner.total.store(total, Ordering::Relaxed);
        }

        (self.inner.repaint)();
    }
}

impl ProgressBridge {
    /// Создаёт `Box<dyn ProgressSink>` для передачи в [`DownloadRequest::with_progress`].
    ///
    /// Можно вызывать несколько раз — все sink'и пишут в один и тот же bridge.
    pub fn sink(&self) -> Box<dyn ProgressSink> {
        Box::new(BridgeSink {
            inner: Arc::clone(&self.inner),
        })
    }
}

// ── Reader side (UI thread) ──────────────────────────────────────────

impl ProgressBridge {
    /// Текущий снимок прогресса. Lock-free, безопасно вызывать на каждом кадре.
    pub fn snapshot(&self) -> DownloadProgress {
        let downloaded = self.inner.downloaded.load(Ordering::Relaxed);
        let total = self.inner.total.load(Ordering::Relaxed);

        DownloadProgress {
            downloaded,
            total: if total == 0 { None } else { Some(total) },
        }
    }

    /// Скачано байт.
    pub fn downloaded(&self) -> u64 {
        self.inner.downloaded.load(Ordering::Relaxed)
    }

    /// Общий размер (если известен).
    pub fn total(&self) -> Option<u64> {
        let v = self.inner.total.load(Ordering::Relaxed);
        if v == 0 { None } else { Some(v) }
    }

    /// Доля выполнения `0.0..=1.0`, или `None` если total неизвестен.
    pub fn fraction(&self) -> Option<f32> {
        self.snapshot().fraction()
    }

    /// Сброс счётчиков. Вызывается перед повторным использованием bridge.
    pub fn reset(&self) {
        self.inner.downloaded.store(0, Ordering::Relaxed);
        self.inner.total.store(0, Ordering::Relaxed);
    }
}
