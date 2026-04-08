use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[repr(u8)]
pub enum BuildStage {
    InstallingDeps = 0,
    DownloadSource = 1,
    CmakeConfigure = 2,
    CmakeBuild = 3,
    CopyArtifacts = 4,
    Done = 5,
    Failed = 6,
}

pub const TOTAL_STAGES: u8 = 5; // stages 0..4

impl BuildStage {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::InstallingDeps,
            1 => Self::DownloadSource,
            2 => Self::CmakeConfigure,
            3 => Self::CmakeBuild,
            4 => Self::CopyArtifacts,
            5 => Self::Done,
            _ => Self::Failed,
        }
    }
}

pub type RepaintHook = Arc<dyn Fn() + Send + Sync>;

pub struct BuildProgress {
    inner: Arc<BuildProgressInner>,
}

struct BuildProgressInner {
    stage: AtomicU8,
    /// Stage fraction stored as f64 bits (0.0..=1.0)
    stage_fraction: AtomicU64,
    repaint: RepaintHook,
}

impl BuildProgress {
    pub fn new(repaint: RepaintHook) -> Self {
        Self {
            inner: Arc::new(BuildProgressInner {
                stage: AtomicU8::new(0),
                stage_fraction: AtomicU64::new(0.0f64.to_bits()),
                repaint,
            }),
        }
    }

    /// Create a no-op progress (for CLI or tests).
    pub fn noop() -> Self {
        Self::new(Arc::new(|| {}))
    }

    // Writer side (background thread)
    pub fn set_stage(
        &self,
        stage: BuildStage,
    ) {
        self.inner.stage.store(stage as u8, Ordering::Relaxed);
        self.inner.stage_fraction.store(0.0f64.to_bits(), Ordering::Relaxed);
        (self.inner.repaint)();
    }

    pub fn set_fraction(
        &self,
        fraction: f32,
    ) {
        let clamped = fraction.clamp(0.0, 1.0) as f64;
        self.inner.stage_fraction.store(clamped.to_bits(), Ordering::Relaxed);
        (self.inner.repaint)();
    }

    // Reader side (UI thread)
    pub fn stage(&self) -> BuildStage {
        BuildStage::from_u8(self.inner.stage.load(Ordering::Relaxed))
    }

    pub fn stage_fraction(&self) -> f32 {
        f64::from_bits(self.inner.stage_fraction.load(Ordering::Relaxed)) as f32
    }

    /// Overall fraction across all stages: (stage_ordinal + stage_fraction) / TOTAL_STAGES
    pub fn overall_fraction(&self) -> f32 {
        let stage = self.inner.stage.load(Ordering::Relaxed).min(TOTAL_STAGES);
        let frac = f64::from_bits(self.inner.stage_fraction.load(Ordering::Relaxed)) as f32;
        (stage as f32 + frac) / TOTAL_STAGES as f32
    }
}

impl Clone for BuildProgress {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_transitions() {
        let p = BuildProgress::noop();
        assert_eq!(p.stage(), BuildStage::InstallingDeps);

        p.set_stage(BuildStage::CmakeBuild);
        assert_eq!(p.stage(), BuildStage::CmakeBuild);

        // Fraction resets on stage change
        p.set_fraction(0.5);
        assert!((p.stage_fraction() - 0.5).abs() < 0.01);

        p.set_stage(BuildStage::CopyArtifacts);
        assert!((p.stage_fraction() - 0.0).abs() < 0.01);
    }

    #[test]
    fn overall_fraction() {
        let p = BuildProgress::noop();
        // Stage 0, fraction 0 -> 0/5 = 0.0
        assert!((p.overall_fraction() - 0.0).abs() < 0.01);

        // Stage 2, fraction 0.5 -> 2.5/5 = 0.5
        p.set_stage(BuildStage::CmakeConfigure);
        p.set_fraction(0.5);
        assert!((p.overall_fraction() - 0.5).abs() < 0.01);

        // Stage 4, fraction 1.0 -> 5/5 = 1.0
        p.set_stage(BuildStage::CopyArtifacts);
        p.set_fraction(1.0);
        assert!((p.overall_fraction() - 1.0).abs() < 0.01);
    }

    #[test]
    fn from_u8_roundtrip() {
        for stage in [
            BuildStage::InstallingDeps,
            BuildStage::DownloadSource,
            BuildStage::CmakeConfigure,
            BuildStage::CmakeBuild,
            BuildStage::CopyArtifacts,
            BuildStage::Done,
            BuildStage::Failed,
        ] {
            assert_eq!(BuildStage::from_u8(stage as u8), stage);
        }
        // Unknown values map to Failed
        assert_eq!(BuildStage::from_u8(255), BuildStage::Failed);
    }

    #[test]
    fn fraction_clamping() {
        let p = BuildProgress::noop();
        p.set_fraction(1.5);
        assert!((p.stage_fraction() - 1.0).abs() < 0.01);

        p.set_fraction(-0.5);
        assert!((p.stage_fraction() - 0.0).abs() < 0.01);
    }
}
