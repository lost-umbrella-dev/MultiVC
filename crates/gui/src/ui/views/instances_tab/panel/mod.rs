//! Instance panel (tabbed floating window).

mod info_tab;
mod log_tab;
mod settings_tab;

use eframe::egui;
use egui_toast::Toasts;

use clients::hash::Hash;
use composer::item::LockItem;
use composer::worker::WorkerHandle;

use crate::ui::lang::{self, Lang};
use crate::ui::state::{InstancePanelTab, InstancesTabState};

pub(super) fn render_instance_panel(
    ui: &mut egui::Ui,
    state: &mut InstancesTabState,
    handle: &WorkerHandle,
    installed_cores: &[(Hash, LockItem)],
    toasts_out: &mut Toasts,
    lang: Lang,
) {
    let Some(ref panel) = state.instance_panel else {
        return;
    };

    let mut open = true;
    let instance_name = panel.name.clone();
    let current_tab = panel.tab;
    let is_running = state.running_instances.contains_key(&instance_name);
    let pid = state.running_instances.get(&instance_name).copied();

    let mut new_tab = current_tab;

    egui::Window::new(&instance_name)
        .collapsible(true)
        .resizable(true)
        .default_size([700.0, 500.0])
        .default_pos([100.0, 80.0])
        .open(&mut open)
        .show(ui.ctx(), |ui| {
            // ── Tab bar ─────────────────────────────────────────
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut new_tab,
                    InstancePanelTab::Info,
                    lang::t("panel.info", lang),
                );
                ui.selectable_value(
                    &mut new_tab,
                    InstancePanelTab::Log,
                    lang::t("panel.log", lang),
                );
                ui.selectable_value(
                    &mut new_tab,
                    InstancePanelTab::Settings,
                    lang::t("panel.settings", lang),
                );
            });
            ui.separator();

            match new_tab {
                InstancePanelTab::Info => {
                    info_tab::render(
                        ui,
                        state,
                        handle,
                        installed_cores,
                        &instance_name,
                        is_running,
                        pid,
                        lang,
                    );
                },
                InstancePanelTab::Log => {
                    log_tab::render(ui, state, &instance_name, is_running, pid, toasts_out, lang);
                },
                InstancePanelTab::Settings => {
                    settings_tab::render(ui, state, handle, &instance_name, toasts_out, lang);
                },
            }
        });

    // Update tab selection
    if let Some(ref mut panel) = state.instance_panel {
        panel.tab = new_tab;
    }

    if !open {
        state.instance_panel = None;
    }
}
