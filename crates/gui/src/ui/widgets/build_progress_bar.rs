//! Horizontal build-progress widget.
//!
//! Displays `{current}/{total} {stage_text}` next to a progress bar that
//! reflects the overall build fraction across all stages.

use builder::progress::{BuildProgress, BuildStage, TOTAL_STAGES};
use eframe::egui;

use crate::ui::lang::{self, Lang};

/// Maps a [`BuildStage`] to its i18n key.
fn stage_key(stage: BuildStage) -> &'static str {
    match stage {
        BuildStage::InstallingDeps => "build.stage.deps",
        BuildStage::DownloadSource => "build.stage.source",
        BuildStage::CmakeConfigure => "build.stage.configure",
        BuildStage::CmakeBuild => "build.stage.build",
        BuildStage::CopyArtifacts => "build.stage.copy",
        BuildStage::Done => "build.stage.done",
        BuildStage::Failed => "build.stage.failed",
    }
}

/// A horizontal progress bar widget that visualises a [`BuildProgress`].
///
/// Renders `{current}/{total} {stage_text}` as the bar label, and fills the bar
/// according to the overall fraction across all build stages. The widget
/// requests a continuous repaint while the build is in progress.
pub struct BuildProgressBar<'a> {
    progress: &'a BuildProgress,
    lang: Lang,
}

impl<'a> BuildProgressBar<'a> {
    /// Creates a new [`BuildProgressBar`].
    pub fn new(
        progress: &'a BuildProgress,
        lang: Lang,
    ) -> Self {
        Self {
            progress,
            lang,
        }
    }
}

impl egui::Widget for BuildProgressBar<'_> {
    fn ui(
        self,
        ui: &mut egui::Ui,
    ) -> egui::Response {
        let stage = self.progress.stage();
        let ordinal = (stage as u8).min(TOTAL_STAGES);
        let fraction = self.progress.overall_fraction();

        let text =
            format!("{}/{} {}", ordinal + 1, TOTAL_STAGES, lang::t(stage_key(stage), self.lang),);

        let desired_width = ui.available_width().min(260.0);

        let response = ui.horizontal(|ui| {
            ui.set_max_width(desired_width);
            ui.add(egui::ProgressBar::new(fraction).text(text).desired_width(desired_width));
        });

        // Keep repainting while the build is running.
        if !matches!(stage, BuildStage::Done | BuildStage::Failed) {
            ui.ctx().request_repaint();
        }

        response.response
    }
}
