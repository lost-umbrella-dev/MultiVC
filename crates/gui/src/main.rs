#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Entry point GUI-приложения MultiVC.
//!
//! Создаёт tokio runtime, загружает [`Composer`], запускает
//! [`ComposerWorker`] в фоновом потоке и открывает окно eframe/egui.

mod app;
mod download_tracker;
mod icons;
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

/// Рендерит LOUM SVG-иконку в `IconData` для окна приложения.
fn loum_icon() -> Option<eframe::egui::IconData> {
    const ICON_SIZE: u32 = 64;

    let opt = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(icons::LOUM.as_bytes(), &opt).ok()?;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(ICON_SIZE, ICON_SIZE)?;
    let svg_size = tree.size();
    let transform = resvg::tiny_skia::Transform::from_scale(
        ICON_SIZE as f32 / svg_size.width(),
        ICON_SIZE as f32 / svg_size.height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // tiny_skia хранит premultiplied RGBA → нужен straight RGBA для IconData
    let mut rgba = pixmap.take();
    for px in rgba.chunks_exact_mut(4) {
        let a = px[3] as f32 / 255.0;
        if a > 0.0 {
            px[0] = (px[0] as f32 / a).min(255.0) as u8;
            px[1] = (px[1] as f32 / a).min(255.0) as u8;
            px[2] = (px[2] as f32 / a).min(255.0) as u8;
        }
    }

    Some(eframe::egui::IconData {
        rgba,
        width: ICON_SIZE,
        height: ICON_SIZE,
    })
}

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
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("MultiVC")
        .with_inner_size([900.0, 600.0]);
    if let Some(icon) = loum_icon() {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
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
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(App::new(handle, rt_for_app)))
        }),
    );

    // После закрытия окна (App::on_exit уже отправил Command::Shutdown)
    // ждём завершения worker thread.
    tracing::info!("GUI закрыт, ожидаем завершения worker'а");
    let _ = worker_thread.join();

    result
}
