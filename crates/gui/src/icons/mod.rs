pub static LOUM: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="icon icon-tabler icons-tabler-outline icon-tabler-umbrella"><path stroke="none" d="M0 0h24v24H0z" fill="none"/><path d="M4 12a8 8 0 0 1 16 0l-16 0" /><path d="M12 12v6a2 2 0 0 0 4 0" /></svg>"#;
pub static GITHUB: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="icon icon-tabler icons-tabler-outline icon-tabler-brand-github"><path stroke="none" d="M0 0h24v24H0z" fill="none" /><path d="M9 19c-4.3 1.4 -4.3 -2.5 -6 -3m12 5v-3.5c0 -1 .1 -1.4 -.5 -2c2.8 -.3 5.5 -1.4 5.5 -6a4.6 4.6 0 0 0 -1.3 -3.2a4.2 4.2 0 0 0 -.1 -3.2s-1.1 -.3 -3.5 1.3a12.3 12.3 0 0 0 -6.2 0c-2.4 -1.6 -3.5 -1.3 -3.5 -1.3a4.2 4.2 0 0 0 -.1 3.2a4.6 4.6 0 0 0 -1.3 3.2c0 4.6 2.7 5.7 5.5 6c-.6 .6 -.6 1.2 -.5 2v3.5" /></svg>"#;

pub const ICON_DELETE: &str = "\u{1F5D1}"; // 🗑
pub const ICON_LAUNCH: &str = "\u{25B6}"; // ▶
pub const ICON_STOP: &str = "\u{23F9}"; // ⏹
pub const ICON_FOLDER: &str = "\u{1F4C2}"; // 📂
pub const ICON_LOG: &str = "\u{1F4C4}"; // 📄
pub const ICON_VALIDATE: &str = "\u{2714}"; // ✔
pub const ICON_REFRESH: &str = "\u{21BB}"; // ↻
pub const ICON_DOWNLOAD: &str = "\u{2B07}"; // ⬇
pub const ICON_SETTINGS: &str = "\u{2699}"; // ⚙

pub const ICON_TAB_CORES: &str = "\u{2699}"; // ⚙
pub const ICON_TAB_INSTANCES: &str = "\u{1F4E6}"; // 📦
pub const ICON_INFO: &str = "\u{2139}"; // ℹ
pub const LABEL_OPEN_FILE: &str = "\u{1F4C2} Open file"; // 📂 Open file

/// Default icon SVG fallback (simple colored square with cube).
pub static DEFAULT_ICON_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64"><rect width="64" height="64" rx="8" fill="#3b3b4f"/><path d="M32 16l14 8v16l-14 8-14-8V24z" fill="none" stroke="#8888aa" stroke-width="2"/><path d="M32 16l14 8-14 8-14-8z" fill="#8888aa" opacity="0.3"/><line x1="32" y1="32" x2="32" y2="48" stroke="#8888aa" stroke-width="2"/></svg>"##;

/// Default banner SVG fallback (subtle gradient bar).
pub static DEFAULT_BANNER_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="600" height="200" viewBox="0 0 600 200"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0%" stop-color="#2a2a3d"/><stop offset="100%" stop-color="#3b3b4f"/></linearGradient></defs><rect width="600" height="200" rx="8" fill="url(#g)"/><text x="300" y="108" text-anchor="middle" font-family="sans-serif" font-size="24" fill="#666680">Instance</text></svg>"##;
