use crate::constants;
use crate::util;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

fn bundled_wallpaper_path() -> Option<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/wallpapers/Fondo.jpg");
    path.exists().then(|| path.display().to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CursorStyle {
    Block,
    IBeam,
    Underline,
}

impl Default for CursorStyle {
    fn default() -> Self {
        Self::Block
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub shell_path: String,
    pub wallpaper_path: Option<String>,
    pub overlay_opacity: f64,
    pub font_family: String,
    pub font_size: i32,
    pub cursor_style: CursorStyle,
    pub show_startup_banner: bool,
    pub accent_color: String,
    pub surface_color: String,
    pub foreground_color: String,
    pub active_border_color: String,
    pub panel_padding: i32,
    pub active_border_width: i32,
    pub scrollback_lines: i64,
    pub enable_animations: bool,
    pub animation_speed: f64,
    pub show_context_bar: bool,
    pub enable_quick_actions: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            shell_path: util::default_shell_path(),
            wallpaper_path: bundled_wallpaper_path(),
            overlay_opacity: 0.44,
            font_family: "JetBrainsMono Nerd Font, JetBrains Mono, Iosevka Term, Monospace"
                .to_string(),
            font_size: 13,
            cursor_style: CursorStyle::Block,
            show_startup_banner: true,
            accent_color: "#b98cff".to_string(),
            surface_color: "#151120".to_string(),
            foreground_color: "#f4eeff".to_string(),
            active_border_color: "#d2a1ff".to_string(),
            panel_padding: 14,
            active_border_width: 2,
            scrollback_lines: 15000,
            enable_animations: true,
            animation_speed: 1.0,
            show_context_bar: true,
            enable_quick_actions: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigManager {
    path: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Self {
        let path = ProjectDirs::from(
            constants::CONFIG_QUALIFIER,
            constants::CONFIG_ORGANIZATION,
            constants::CONFIG_APPLICATION,
        )
        .map(|dirs| dirs.config_dir().join("config.toml"))
        .unwrap_or_else(|| PathBuf::from(".termvoid.toml"));

        Self { path }
    }

    pub fn load_or_default(&self) -> AppConfig {
        self.load().unwrap_or_default()
    }

    pub fn load(&self) -> Option<AppConfig> {
        let content = fs::read_to_string(&self.path).ok()?;
        toml::from_str(&content).ok()
    }

    pub fn save(&self, config: &AppConfig) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }

        let serialized = toml::to_string_pretty(config).map_err(|error| error.to_string())?;
        fs::write(&self.path, serialized).map_err(|error| error.to_string())
    }
}
