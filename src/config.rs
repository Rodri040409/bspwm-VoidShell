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
pub enum CursorStyle {
    #[serde(rename = "bloque", alias = "block")]
    Block,
    #[serde(rename = "barra", alias = "i-beam")]
    IBeam,
    #[serde(rename = "subrayado", alias = "underline")]
    Underline,
}

impl Default for CursorStyle {
    fn default() -> Self {
        Self::Block
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BannerInfoLayout {
    #[serde(rename = "derecha", alias = "right")]
    Right,
    #[serde(rename = "debajo", alias = "below", alias = "abajo")]
    Below,
}

impl Default for BannerInfoLayout {
    fn default() -> Self {
        Self::Right
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    #[serde(rename = "ruta_shell", alias = "shell_path")]
    pub shell_path: String,
    #[serde(rename = "ruta_fondo", alias = "wallpaper_path")]
    pub wallpaper_path: Option<String>,
    #[serde(rename = "opacidad_overlay", alias = "overlay_opacity")]
    pub overlay_opacity: f64,
    #[serde(rename = "familia_fuente", alias = "font_family")]
    pub font_family: String,
    #[serde(rename = "tamano_fuente", alias = "font_size")]
    pub font_size: i32,
    #[serde(rename = "estilo_cursor", alias = "cursor_style")]
    pub cursor_style: CursorStyle,
    #[serde(rename = "mostrar_banner_inicio", alias = "show_startup_banner")]
    pub show_startup_banner: bool,
    #[serde(
        rename = "mostrar_banner_en_paneles_nuevos",
        alias = "show_banner_on_new_panes"
    )]
    pub show_banner_on_new_panes: bool,
    #[serde(rename = "posicion_info_banner", alias = "banner_info_layout")]
    pub banner_info_layout: BannerInfoLayout,
    #[serde(rename = "color_acento", alias = "accent_color")]
    pub accent_color: String,
    #[serde(rename = "color_superficie", alias = "surface_color")]
    pub surface_color: String,
    #[serde(rename = "color_primer_plano", alias = "foreground_color")]
    pub foreground_color: String,
    #[serde(rename = "color_borde_activo", alias = "active_border_color")]
    pub active_border_color: String,
    #[serde(rename = "padding_panel", alias = "panel_padding")]
    pub panel_padding: i32,
    #[serde(rename = "grosor_borde_activo", alias = "active_border_width")]
    pub active_border_width: i32,
    #[serde(rename = "lineas_scrollback", alias = "scrollback_lines")]
    pub scrollback_lines: i64,
    #[serde(rename = "activar_animaciones", alias = "enable_animations")]
    pub enable_animations: bool,
    #[serde(rename = "velocidad_animacion", alias = "animation_speed")]
    pub animation_speed: f64,
    #[serde(rename = "mostrar_barra_contextual", alias = "show_context_bar")]
    pub show_context_bar: bool,
    #[serde(rename = "activar_acciones_rapidas", alias = "enable_quick_actions")]
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
            show_banner_on_new_panes: true,
            banner_info_layout: BannerInfoLayout::Right,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializa_configuracion_con_claves_en_espanol() {
        let serialized =
            toml::to_string_pretty(&AppConfig::default()).expect("debe serializar la config");

        assert!(serialized.contains("ruta_shell"));
        assert!(serialized.contains("mostrar_banner_inicio = true"));
        assert!(serialized.contains("posicion_info_banner = \"derecha\""));
        assert!(serialized.contains("estilo_cursor = \"bloque\""));
        assert!(!serialized.contains("shell_path"));
        assert!(!serialized.contains("banner_info_layout"));
    }

    #[test]
    fn deserializa_claves_legadas_en_ingles() {
        let config: AppConfig = toml::from_str(
            r#"
shell_path = "/bin/zsh"
cursor_style = "underline"
show_startup_banner = false
show_banner_on_new_panes = false
banner_info_layout = "below"
enable_quick_actions = false
"#,
        )
        .expect("debe leer la config antigua");

        assert_eq!(config.shell_path, "/bin/zsh");
        assert_eq!(config.cursor_style, CursorStyle::Underline);
        assert!(!config.show_startup_banner);
        assert!(!config.show_banner_on_new_panes);
        assert_eq!(config.banner_info_layout, BannerInfoLayout::Below);
        assert!(!config.enable_quick_actions);
    }
}
