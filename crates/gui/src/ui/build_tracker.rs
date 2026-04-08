//! Per-item build progress tracking.
//!
//! Manages a set of [`BuildProgress`]s keyed by version string,
//! allowing parallel builds with independent progress indicators.

use std::collections::HashMap;
use std::sync::Arc;

use builder::progress::BuildProgress;
use clients::item::Item;
use eframe::egui;

/// Tracks per-item build progress via [`BuildProgress`].
///
/// Each active build gets its own progress handle. The UI reads `overall_fraction()`
/// and `stage()` on each frame to render progress next to the building item.
#[derive(Default)]
pub struct BuildTracker {
    /// Active build progresses keyed by version string.
    trackers: HashMap<String, BuildProgress>,
}

#[allow(dead_code)]
impl BuildTracker {
    /// Creates a key for an item (version string).
    fn key(item: &Item) -> String {
        item.version.to_string()
    }

    /// Starts tracking a build for `item`.
    ///
    /// Creates a [`BuildProgress`] with a repaint hook tied to `ctx`,
    /// stores it, and returns a clone to pass to the build task.
    pub fn start(
        &mut self,
        item: &Item,
        ctx: &egui::Context,
    ) -> BuildProgress {
        let ctx_clone = ctx.clone();
        let hook = Arc::new(move || ctx_clone.request_repaint());
        let progress = BuildProgress::new(hook);
        self.trackers.insert(Self::key(item), progress.clone());
        progress
    }

    /// Returns `true` if this item has an active build.
    pub fn is_active(
        &self,
        item: &Item,
    ) -> bool {
        self.trackers.contains_key(&Self::key(item))
    }

    /// Returns the [`BuildProgress`] for an item, if active.
    pub fn progress(
        &self,
        item: &Item,
    ) -> Option<&BuildProgress> {
        self.trackers.get(&Self::key(item))
    }

    /// Returns `true` if any build is in progress.
    pub fn has_active(&self) -> bool {
        !self.trackers.is_empty()
    }

    /// Number of active builds.
    pub fn active_count(&self) -> usize {
        self.trackers.len()
    }

    /// Removes trackers for completed build versions.
    pub fn complete_built(
        &mut self,
        versions: &[String],
    ) {
        for v in versions {
            self.trackers.remove(v);
        }
    }
}
