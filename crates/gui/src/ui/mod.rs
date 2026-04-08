//! UI rendering: views, widgets, icons, i18n, toasts.

pub mod download_tracker;
pub mod icons;
pub mod lang;
pub mod settings;
pub mod state;
pub mod toasts;
pub mod views;
pub mod widgets;

use eframe::egui;

use composer::worker::WorkerHandle;

use crate::App;
use state::{InstanceForm, UiState};

// ── Navigation ──────────────────────────────────────────────────────

/// Вкладки навигации.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Cores,
    Instances,
}

// ── Utilities ───────────────────────────────────────────────────────

/// Форматирует размер в байтах в человекочитаемый вид.
pub fn format_size(bytes: u64) -> String {
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

// ── Hot-reloadable render entry point ───────────────────────────────

/// Arguments passed from the host to the render function.
pub struct RenderArgs<'a> {
    pub current_tab: &'a mut Tab,
    pub state: &'a mut UiState,
    pub handle: &'a mut WorkerHandle,
}

/// Main UI render function — called by the host each frame.
///
/// Renders navigation, active tab, settings modal, and toasts.
pub fn render_ui(
    ui: &mut egui::Ui,
    args: &mut RenderArgs,
) {
    let mut toasts_instance = toasts::create_toasts();

    // Drain worker events first — updates state before render
    App::drain_and_apply_events(args.state, args.handle, &mut toasts_instance);

    let ctx = ui.ctx().clone();

    // Bottom panel — navigation bar
    egui::Panel::bottom("nav_panel").resizable(false).default_size(30.0).show_inside(ui, |ui| {
        views::sidebar::render(
            ui,
            args.current_tab,
            &args.state.cores.downloads,
            &mut args.state.settings,
        );
    });

    // Central panel — active tab
    egui::CentralPanel::default().show_inside(ui, |ui| match *args.current_tab {
        Tab::Cores => {
            let lang = args.state.settings.lock.language;
            let action = views::cores_tab::render(
                ui,
                &ctx,
                &mut args.state.cores,
                args.handle,
                &mut toasts_instance,
                lang,
            );

            if let Some(core_idx) = action.switch_to_instances_with_core {
                *args.current_tab = Tab::Instances;
                args.state.instances.create_form = Some(InstanceForm {
                    selected_core_idx: Some(core_idx),
                    ..Default::default()
                });
            }
        },
        Tab::Instances => {
            let lang = args.state.settings.lock.language;
            views::instances_tab::render(
                ui,
                &mut args.state.instances,
                args.handle,
                &args.state.cores.installed,
                &args.state.paths.instances_dir,
                &mut toasts_instance,
                lang,
            );
        },
    });

    // Settings modal
    views::settings_modal::render(
        ui,
        &mut args.state.settings,
        args.state.cores.installed.len(),
        args.state.instances.installed.len(),
        &args.state.paths,
        &mut toasts_instance,
    );

    // Toasts (must be last — renders overlay)
    toasts_instance.show(ui);
}
