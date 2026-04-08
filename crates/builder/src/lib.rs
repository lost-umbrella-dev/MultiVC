pub mod cmake;
pub mod deps;
pub mod error;
pub mod pipeline;
pub mod platform;
pub mod progress;

pub use error::BuilderError;
pub use pipeline::{BuildRequest, BuildResult, build_core};
pub use platform::{LinuxDistro, Platform, detect};
pub use progress::{BuildProgress, BuildStage, RepaintHook, TOTAL_STAGES};
