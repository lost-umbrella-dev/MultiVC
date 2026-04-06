mod progress_ring;

pub use progress_ring::ProgressRing;

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
