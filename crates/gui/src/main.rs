//! Entry point GUI-приложения MultiVC.
//!
//! Создаёт tokio runtime, загружает [`Composer`], запускает
//! [`ComposerWorker`] в фоновом потоке и открывает окно eframe/egui.

mod app;
mod download_tracker;
mod state;
mod toasts;
mod views;
mod widgets;

use app::App;
use clients::{Clients, github::GithubClient};
use composer::Composer;
use composer::worker::ComposerWorker;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt};
use tracing_tree::HierarchicalLayer;

fn main() -> eframe::Result<()> {
    // ── Tracing ──────────────────────────────────────────────────
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("warn,clients=debug,composer=debug,gui=debug"));

    let layer = HierarchicalLayer::new(2)
        .with_ansi(true)
        .with_targets(true)
        .with_bracketed_fields(true)
        .with_thread_names(false)
        .with_indent_lines(true)
        .with_filter(filter);

    let subscriber = tracing_subscriber::registry().with(layer);

    let _guard = tracing::subscriber::set_default(subscriber);

    tracing::info!("запуск MultiVC GUI");

    // ── Tokio runtime ────────────────────────────────────────────
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("не удалось создать tokio runtime");

    // ── Composer ─────────────────────────────────────────────────
    let composer = runtime.block_on(async {
        let github =
            GithubClient::new("MihailRis".to_owned(), "VoxelCore".to_owned()).expect("не удалось создать GithubClient");
        let clients = Clients::new(github);

        // Пытаемся загрузить lock-файлы; при первом запуске создадутся пустые
        Composer::load(clients).await.expect("не удалось загрузить Composer")
    });

    // ── Worker ───────────────────────────────────────────────────
    let (worker, handle) = ComposerWorker::with_defaults(composer);

    // NOTE: `worker.run()` future содержит `dyn DynDigest` (не Send),
    // поэтому мы запускаем его через `spawn_local` на выделенном
    // LocalSet, а не через обычный `runtime.spawn()`.
    //
    // LocalSet тоже не Send, поэтому создаём его внутри потока.
    let rt_handle = runtime.handle().clone();
    let worker_thread = std::thread::Builder::new()
        .name("composer-worker".into())
        .spawn(move || {
            let local = tokio::task::LocalSet::new();
            local.spawn_local(worker.run());
            rt_handle.block_on(local);
        })
        .expect("не удалось запустить worker thread");

    // ── eframe ───────────────────────────────────────────────────
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("MultiVC")
            .with_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    // Передаём runtime внутрь App — он нужен для RepaintHook и
    // возможных ad-hoc async вызовов.
    let rt_for_app = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("не удалось создать вспомогательный tokio runtime");

    let result = eframe::run_native(
        "MultiVC",
        native_options,
        Box::new(move |_cc| Ok(Box::new(App::new(handle, rt_for_app)))),
    );

    // После закрытия окна (App::on_exit уже отправил Command::Shutdown)
    // ждём завершения worker thread.
    tracing::info!("GUI закрыт, ожидаем завершения worker'а");
    let _ = worker_thread.join();

    result
}
