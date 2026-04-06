mod progress_ring;

pub use progress_ring::ProgressRing;

use base64::Engine as _;
use eframe::egui;

/// Uniform min size for icon-only buttons (e.g. "\u{1F5D1}", "+", "\u{25B6}").
const ICON_BUTTON_SIZE: f32 = 24.0;

/// Creates a uniformly-sized icon button.
pub fn icon_button(icon: impl Into<egui::WidgetText>) -> egui::Button<'static> {
    egui::Button::new(icon).min_size(egui::vec2(ICON_BUTTON_SIZE, ICON_BUTTON_SIZE))
}

/// Returns a [`egui::Frame`] with faint striped background when `striped` is true.
pub fn striped_frame(striped: bool, ui: &egui::Ui) -> egui::Frame {
    if striped {
        egui::Frame::NONE.fill(ui.visuals().faint_bg_color)
    } else {
        egui::Frame::NONE
    }
}

/// Renders a form row with a label and a single-line text input.
pub fn form_row(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.text_edit_singleline(value);
    });
}

/// Renders a section toolbar: heading on the left, action buttons on the right.
pub fn tab_toolbar(ui: &mut egui::Ui, title: &str, buttons: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.heading(title);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), buttons);
    });
}

/// Shows a centered confirmation dialog.
///
/// Returns `Some(true)` if confirmed, `Some(false)` if cancelled or closed, `None` if still open.
pub fn confirm_dialog(ctx: &egui::Context, title: &str, message: &str) -> Option<bool> {
    let mut result = None;
    let mut open = true;

    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .open(&mut open)
        .show(ctx, |ui| {
            ui.label(message);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Yes, delete").clicked() {
                    result = Some(true);
                }
                if ui.button("Cancel").clicked() {
                    result = Some(false);
                }
            });
        });

    if !open {
        return Some(false);
    }
    result
}

/// Opens `path` in the system file manager using the `open` crate.
/// Shows a toast error/warning if the path does not exist or the open fails.
pub fn open_folder(path: impl AsRef<std::path::Path>, toasts: &mut egui_toast::Toasts) {
    let path = path.as_ref();
    if path.exists() {
        if let Err(e) = open::that(path) {
            crate::toasts::error(toasts, format!("Failed to open folder: {e}"));
        }
    } else {
        crate::toasts::warning(toasts, format!("Folder does not exist: {}", path.display()));
    }
}

/// Renders a clickable folder-open button (icon only).
/// Calls `open_folder()` on click.
/// Returns `true` if the button was clicked.
#[allow(dead_code)]
pub fn open_folder_button(
    ui: &mut egui::Ui,
    path: impl AsRef<std::path::Path>,
    hover_text: &str,
    toasts: &mut egui_toast::Toasts,
) -> bool {
    let clicked = ui
        .add(icon_button(crate::icons::ICON_FOLDER))
        .on_hover_text(hover_text)
        .clicked();

    if clicked {
        open_folder(path, toasts);
    }

    clicked
}

// ── Image widgets ───────────────────────────────────────────────────

/// Renders an image from a base64-encoded string.
///
/// If `base64_str` is empty, renders the `fallback_svg` instead.
/// `id` must be unique per image instance (e.g. `"instance/myname/icon"`).
/// A content hash is appended to the URI to invalidate egui's texture cache on change.
pub fn image_from_base64(
    ui: &mut egui::Ui,
    id: &str,
    base64_str: &str,
    size: egui::Vec2,
    fallback_svg: &str,
) -> egui::Response {
    use std::hash::{Hash, Hasher};

    if base64_str.is_empty() {
        let uri = format!("bytes://{id}/fallback");
        let image = egui::Image::from_bytes(uri, fallback_svg.as_bytes().to_vec()).fit_to_exact_size(size);
        ui.add(image)
    } else {
        // Hash the content so the URI changes when the image data changes,
        // forcing egui to reload the texture instead of serving stale cache.
        let mut hasher = std::hash::DefaultHasher::new();
        base64_str.hash(&mut hasher);
        let content_hash = hasher.finish();
        let uri = format!("bytes://{id}/{content_hash:x}");

        match base64::engine::general_purpose::STANDARD.decode(base64_str) {
            Ok(bytes) => {
                let image = egui::Image::from_bytes(uri, bytes).fit_to_exact_size(size);
                ui.add(image)
            },
            Err(_) => {
                let fallback_uri = format!("bytes://{id}/fallback");
                let image =
                    egui::Image::from_bytes(fallback_uri, fallback_svg.as_bytes().to_vec()).fit_to_exact_size(size);
                ui.add(image)
            },
        }
    }
}

/// An in-flight image pick + convert task.
///
/// Created by [`start_image_pick`]. Poll with [`ImagePickTask::poll`] each frame.
pub struct ImagePickTask {
    rx: std::sync::mpsc::Receiver<Result<String, String>>,
}

impl ImagePickTask {
    /// Non-blocking poll. Returns `Some(Ok(base64))` on success,
    /// `Some(Err(msg))` on failure, `None` while still processing.
    pub fn poll(&self) -> Option<Result<String, String>> {
        self.rx.try_recv().ok()
    }
}

/// Opens a native file dialog, then converts the selected image to base64
/// in a background thread. Returns `None` if the user cancelled the dialog,
/// or `Some(task)` to poll for the result.
///
/// The file dialog itself runs synchronously (it's a native OS modal),
/// but the image decoding + WebP conversion runs in a background thread.
pub fn start_image_pick(ctx: &egui::Context) -> Option<ImagePickTask> {
    let path = rfd::FileDialog::new()
        .add_filter("Images", &["png", "jpg", "jpeg", "webp", "svg"])
        .pick_file()?;

    let (tx, rx) = std::sync::mpsc::channel();
    let ctx = ctx.clone();

    std::thread::spawn(move || {
        let result = convert_image_to_base64(&path);
        let _ = tx.send(result);
        ctx.request_repaint();
    });

    Some(ImagePickTask { rx })
}

fn convert_image_to_base64(path: &std::path::Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read file: {e}"))?;

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

    match ext.as_str() {
        "svg" | "webp" => Ok(base64::engine::general_purpose::STANDARD.encode(&bytes)),
        "png" | "jpg" | "jpeg" => {
            let img = image::load_from_memory(&bytes).map_err(|e| format!("Failed to decode image: {e}"))?;
            let mut webp_buf = std::io::Cursor::new(Vec::new());
            img.write_to(&mut webp_buf, image::ImageFormat::WebP)
                .map_err(|e| format!("Failed to encode WebP: {e}"))?;
            Ok(base64::engine::general_purpose::STANDARD.encode(webp_buf.into_inner()))
        },
        other => Err(format!("Unsupported format: {other}")),
    }
}

/// Legacy sync version — kept for simple use sites but prefer [`start_image_pick`].
#[allow(dead_code)]
pub fn pick_image_as_base64() -> Option<String> {
    let path = rfd::FileDialog::new()
        .add_filter("Images", &["png", "jpg", "jpeg", "webp", "svg"])
        .pick_file()?;

    convert_image_to_base64(&path).ok()
}
