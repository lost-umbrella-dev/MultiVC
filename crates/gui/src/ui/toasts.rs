//! Toast notification helpers.
//!
//! Wraps [`egui_toast::Toasts`] with convenient methods for
//! success / error / info / warning notifications.

use eframe::egui::{Align2, Direction, Order};
use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};

/// Default toast duration in seconds.
const TOAST_DURATION_SEC: f64 = 3.0;

/// Default toast options.
fn default_options() -> ToastOptions {
    ToastOptions::default()
        .duration_in_seconds(TOAST_DURATION_SEC)
        .show_progress(true)
        .show_icon(true)
}

/// Creates a new [`Toasts`] instance with the app's default configuration.
///
/// Must be called each frame (immediate-mode pattern — `Toasts` stores
/// its state in `egui`'s per-frame memory via `Id`).
pub fn create_toasts() -> Toasts {
    Toasts::new()
        .anchor(Align2::LEFT_TOP, (-10.0, 30.0))
        .direction(Direction::TopDown)
        .order(Order::Tooltip)
}

/// Adds a success toast.
pub fn success(
    toasts: &mut Toasts,
    text: impl Into<String>,
) {
    toasts.add(Toast {
        text: text.into().into(),
        kind: ToastKind::Success,
        options: default_options(),
        ..Default::default()
    });
}

/// Adds an error toast.
pub fn error(
    toasts: &mut Toasts,
    text: impl Into<String>,
) {
    toasts.add(Toast {
        text: text.into().into(),
        kind: ToastKind::Error,
        options: default_options(),
        ..Default::default()
    });
}

/// Adds an info toast.
pub fn info(
    toasts: &mut Toasts,
    text: impl Into<String>,
) {
    toasts.add(Toast {
        text: text.into().into(),
        kind: ToastKind::Info,
        options: default_options(),
        ..Default::default()
    });
}

/// Adds a warning toast.
pub fn warning(
    toasts: &mut Toasts,
    text: impl Into<String>,
) {
    toasts.add(Toast {
        text: text.into().into(),
        kind: ToastKind::Warning,
        options: default_options(),
        ..Default::default()
    });
}
