//! Per-item download progress tracking.
//!
//! Manages a set of [`ProgressBridge`]s keyed by item identity (`name/version`),
//! allowing parallel downloads with independent progress indicators.

use std::collections::HashMap;
use std::sync::Arc;

use clients::item::Item;
use composer::DownloadRequest;
use composer::progress::ProgressBridge;
use eframe::egui;

/// Tracks per-item download progress via [`ProgressBridge`].
///
/// Each active download gets its own bridge. The UI reads `fraction()`
/// on each frame to render a progress ring next to the downloading item.
#[derive(Default)]
pub struct DownloadTracker {
    /// Active bridges keyed by `"name/version"`.
    bridges: HashMap<String, ProgressBridge>,
}

impl DownloadTracker {
    /// Creates a key for an item.
    fn key(item: &Item) -> String {
        format!("{}/{}", item.name, item.version)
    }

    /// Starts tracking a download for `item`.
    ///
    /// Creates a [`ProgressBridge`] with a repaint hook tied to `ctx`,
    /// stores it, and returns a [`DownloadRequest`] with the bridge's sink.
    pub fn start(&mut self, item: &Item, ctx: &egui::Context) -> DownloadRequest {
        let ctx_clone = ctx.clone();
        let hook = Arc::new(move || ctx_clone.request_repaint());
        let bridge = ProgressBridge::new(hook);
        let request = DownloadRequest::with_progress(item.clone(), bridge.sink());
        self.bridges.insert(Self::key(item), bridge);
        request
    }

    /// Returns `true` if this item has an active download.
    pub fn is_active(&self, item: &Item) -> bool {
        self.bridges.contains_key(&Self::key(item))
    }

    /// Returns the download fraction for an item.
    ///
    /// - `None` — total unknown (indeterminate ring)
    /// - `Some(0.0..=1.0)` — determinate progress
    pub fn fraction(&self, item: &Item) -> Option<f32> {
        self.bridges.get(&Self::key(item)).and_then(|b| b.fraction())
    }

    /// Removes a specific item's bridge (download finished or failed).
    #[allow(dead_code)]
    pub fn complete(&mut self, item: &Item) {
        self.bridges.remove(&Self::key(item));
    }

    /// Removes all active bridges (batch install finished).
    #[allow(dead_code)]
    pub fn complete_all(&mut self) {
        self.bridges.clear();
    }

    /// Removes bridges for items whose key (`name/version`) is in the given set.
    ///
    /// Used after `CoresInstalled` to clear only the bridges for items
    /// that were successfully installed, leaving queued downloads intact.
    pub fn complete_installed(&mut self, installed_keys: &[String]) {
        self.bridges.retain(|key, _| !installed_keys.contains(key));
    }

    /// Returns `true` if any download is in progress.
    pub fn has_active(&self) -> bool {
        !self.bridges.is_empty()
    }

    /// Number of active downloads.
    pub fn active_count(&self) -> usize {
        self.bridges.len()
    }
}
