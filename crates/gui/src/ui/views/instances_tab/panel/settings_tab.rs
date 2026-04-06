//! Settings tab for the instance panel.

use eframe::egui;
use egui_toast::Toasts;

use composer::lock::instance::Instance;
use composer::lock::instances::InstancesItem;
use composer::message::Command;
use composer::worker::WorkerHandle;

use crate::ui::icons;
use crate::ui::lang::{self, Lang};
use crate::ui::state::InstancesTabState;
use crate::ui::toasts;
use crate::ui::widgets::{image_from_base64, start_image_pick};

pub(super) fn render(
    ui: &mut egui::Ui,
    state: &mut InstancesTabState,
    handle: &WorkerHandle,
    name: &str,
    toasts_out: &mut Toasts,
    lang: Lang,
) {
    let meta = state.installed.iter().find(|(n, _)| n == name).map(|(_, m)| m.clone());
    let Some(meta) = meta else {
        ui.label("Instance not found");
        return;
    };

    let panel = state.instance_panel.as_mut().unwrap();

    // Initialize edit state on first open
    if panel.edit_description.is_none() {
        panel.edit_description = Some(
            state.viewing.as_ref().and_then(|inst| inst.description.clone()).unwrap_or_default(),
        );
    }
    if panel.edit_icon.is_none() {
        panel.edit_icon = Some(meta.icon.clone());
    }
    if panel.edit_banner.is_none() {
        panel.edit_banner = Some(meta.banner.clone());
    }

    // Poll in-flight image tasks
    if let Some(ref task) = panel.icon_pick
        && let Some(result) = task.poll()
    {
        match result {
            Ok(b64) => panel.edit_icon = Some(b64),
            Err(e) => tracing::error!("Icon pick failed: {e}"),
        }
        panel.icon_pick = None;
    }
    if let Some(ref task) = panel.banner_pick
        && let Some(result) = task.poll()
    {
        match result {
            Ok(b64) => panel.edit_banner = Some(b64),
            Err(e) => tracing::error!("Banner pick failed: {e}"),
        }
        panel.banner_pick = None;
    }

    let edit_desc = panel.edit_description.as_mut().unwrap();
    let edit_icon = panel.edit_icon.clone().unwrap_or_default();
    let edit_banner = panel.edit_banner.clone().unwrap_or_default();

    // Check if anything has changed from the original
    let orig_desc =
        state.viewing.as_ref().and_then(|inst| inst.description.clone()).unwrap_or_default();
    let changed = *edit_desc != orig_desc || edit_icon != meta.icon || edit_banner != meta.banner;

    // Description editor
    ui.horizontal(|ui| {
        ui.label(lang::t("info.description", lang));
        ui.text_edit_singleline(edit_desc);
    });

    ui.separator();

    // Icon picker
    ui.horizontal(|ui| {
        image_from_base64(
            ui,
            &format!("instance/{name}/icon/settings"),
            &edit_icon,
            egui::vec2(48.0, 48.0),
            icons::DEFAULT_ICON_SVG,
        );
        if panel.icon_pick.is_some() {
            ui.spinner();
        } else if ui
            .button(lang::t("isettings.change_icon", lang))
            .on_hover_text(lang::t("tip.formats", lang))
            .clicked()
        {
            panel.icon_pick = start_image_pick(ui.ctx());
        }
    });

    ui.add_space(4.0);

    // Banner picker
    image_from_base64(
        ui,
        &format!("instance/{name}/banner/settings"),
        &edit_banner,
        egui::vec2(ui.available_width().min(400.0), 80.0),
        icons::DEFAULT_BANNER_SVG,
    );
    if panel.banner_pick.is_some() {
        ui.spinner();
    } else if ui
        .button(lang::t("isettings.change_banner", lang))
        .on_hover_text(lang::t("tip.formats", lang))
        .clicked()
    {
        panel.banner_pick = start_image_pick(ui.ctx());
    }

    ui.separator();

    // Save button
    if ui.add_enabled(changed, egui::Button::new(lang::t("isettings.save", lang))).clicked() {
        let description = panel.edit_description.clone().unwrap_or_default();
        let icon = panel.edit_icon.clone().unwrap_or_default();
        let banner = panel.edit_banner.clone().unwrap_or_default();

        if let Some(inst) = &state.viewing {
            let config = Instance {
                description: if description.is_empty() {
                    None
                } else {
                    Some(description)
                },
                core_version: inst.core_version.clone(),
                dependencies: inst.dependencies.clone(),
            };
            let new_meta = InstancesItem {
                icon,
                banner,
                last_launch: meta.last_launch,
                created_at: meta.created_at,
            };
            handle.try_send(Command::EditInstance {
                name: name.to_owned(),
                config,
                meta: new_meta.clone(),
            });
            // Update local cache
            if let Some((_, m)) = state.installed.iter_mut().find(|(n, _)| n == name) {
                *m = new_meta;
            }
            toasts::success(toasts_out, lang::t("toast.instances_saved", lang));
        }
    }
}
