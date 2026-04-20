use crate::config::{AppConfig, CursorStyle};
use crate::util;
use gtk::gdk;
use std::cell::RefCell;

thread_local! {
    static CSS_PROVIDER: RefCell<Option<gtk::CssProvider>> = const { RefCell::new(None) };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanePalettePreset {
    Red,
    Green,
    Blue,
    Amber,
    Rose,
    Cyan,
}

impl PanePalettePreset {
    pub const ALL: [Self; 6] = [
        Self::Red,
        Self::Green,
        Self::Blue,
        Self::Amber,
        Self::Rose,
        Self::Cyan,
    ];

    pub fn from_name(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "red" | "ruby" => Some(Self::Red),
            "green" | "emerald" => Some(Self::Green),
            "blue" | "azure" => Some(Self::Blue),
            "amber" | "gold" | "yellow" => Some(Self::Amber),
            "rose" | "pink" => Some(Self::Rose),
            "cyan" | "teal" => Some(Self::Cyan),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Red => "Red",
            Self::Green => "Green",
            Self::Blue => "Blue",
            Self::Amber => "Amber",
            Self::Rose => "Rose",
            Self::Cyan => "Cyan",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Green => "green",
            Self::Blue => "blue",
            Self::Amber => "amber",
            Self::Rose => "rose",
            Self::Cyan => "cyan",
        }
    }

    pub fn css_class(self) -> &'static str {
        match self {
            Self::Red => "palette-red",
            Self::Green => "palette-green",
            Self::Blue => "palette-blue",
            Self::Amber => "palette-amber",
            Self::Rose => "palette-rose",
            Self::Cyan => "palette-cyan",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerminalPalette {
    pub foreground: gdk::RGBA,
    pub background: gdk::RGBA,
    pub cursor: gdk::RGBA,
    pub cursor_text: gdk::RGBA,
    pub palette: Vec<gdk::RGBA>,
}

pub fn install_or_update(app_config: &AppConfig) {
    CSS_PROVIDER.with(|slot| {
        let mut slot = slot.borrow_mut();
        let provider = slot.get_or_insert_with(gtk::CssProvider::new);
        provider.load_from_string(&build_css(app_config));

        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    });
}

pub fn font_description(config: &AppConfig) -> gtk::pango::FontDescription {
    let mut description = gtk::pango::FontDescription::from_string(&config.font_family);
    description.set_size(config.font_size * gtk::pango::SCALE);
    description
}

pub fn cursor_shape(style: &CursorStyle) -> vte::CursorShape {
    match style {
        CursorStyle::Block => vte::CursorShape::Block,
        CursorStyle::IBeam => vte::CursorShape::Ibeam,
        CursorStyle::Underline => vte::CursorShape::Underline,
    }
}

pub fn terminal_palette(config: &AppConfig, preset: Option<PanePalettePreset>) -> TerminalPalette {
    if let Some(preset) = preset {
        let preset = preset_theme(preset);
        let foreground = util::parse_rgba(preset.foreground, "#f4eeff");
        let mut background = util::parse_rgba(preset.background, "#151120");
        background.set_alpha(0.22);
        let cursor = util::parse_rgba(preset.accent, "#b98cff");
        let cursor_text = util::parse_rgba(preset.cursor_text, "#120d1d");
        let palette = preset
            .palette
            .iter()
            .map(|value| util::parse_rgba(value, "#ffffff"))
            .collect();

        return TerminalPalette {
            foreground,
            background,
            cursor,
            cursor_text,
            palette,
        };
    }

    let foreground = util::parse_rgba(&config.foreground_color, "#f4eeff");
    let mut background = util::parse_rgba(&config.surface_color, "#151120");
    background.set_alpha(0.18);
    let cursor = util::parse_rgba(&config.accent_color, "#b98cff");
    let cursor_text = util::parse_rgba("#120d1d", "#120d1d");

    let palette = [
        "#14111f", "#ff7ea8", "#98f5c2", "#f3d789", "#a994ff", "#d891ff", "#8be9ff", "#ebe5ff",
        "#5f5777", "#ff9bc0", "#b7ffd7", "#ffe4a8", "#c2b4ff", "#ebb9ff", "#a8f2ff", "#ffffff",
    ]
    .iter()
    .map(|value| util::parse_rgba(value, "#ffffff"))
    .collect();

    TerminalPalette {
        foreground,
        background,
        cursor,
        cursor_text,
        palette,
    }
}

fn build_css(config: &AppConfig) -> String {
    let mut surface = util::parse_rgba(&config.surface_color, "#151120");
    surface.set_alpha(0.72);
    let accent = util::parse_rgba(&config.accent_color, "#b98cff");
    let active = util::parse_rgba(&config.active_border_color, "#d2a1ff");
    let foreground = util::parse_rgba(&config.foreground_color, "#f4eeff");
    let border_width = config.active_border_width.max(1);
    let shadow = if config.enable_animations {
        "transition: border-color 170ms ease, box-shadow 220ms ease, background-color 220ms ease, opacity 220ms ease, transform 220ms cubic-bezier(0.18, 0.9, 0.22, 1.0);"
    } else {
        ""
    };
    let palette_css = PanePalettePreset::ALL
        .iter()
        .map(|preset| build_preset_css(*preset))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"
window {{
  background:
    radial-gradient(circle at top, rgba(130, 96, 214, 0.12), transparent 36%),
    linear-gradient(180deg, rgba(9, 7, 17, 0.99), rgba(16, 12, 25, 1.0));
}}

.app-shell {{
  background: transparent;
}}

.termvoid-headerbar {{
  background:
    linear-gradient(180deg, rgba(18, 13, 29, 0.96), rgba(12, 9, 22, 0.98));
  border-bottom: 1px solid rgba(214, 163, 255, 0.10);
  color: {foreground};
}}

.termvoid-title {{
  color: {foreground};
}}

.layout-surface {{
  padding: 16px;
  background:
    radial-gradient(circle at top right, rgba(173, 114, 255, 0.14), transparent 32%),
    radial-gradient(circle at bottom left, rgba(120, 74, 190, 0.10), transparent 26%),
    linear-gradient(180deg, rgba(10, 8, 19, 0.96), rgba(15, 10, 24, 0.99));
}}

.shared-wallpaper {{
  opacity: 0.96;
}}

.shared-wallpaper-tint {{
  background:
    radial-gradient(circle at 20% 0%, rgba(36, 22, 59, 0.40), transparent 34%),
    linear-gradient(180deg, rgba(9, 6, 18, 0.46), rgba(11, 8, 22, 0.60));
}}

.layout-surface.zoomed {{
  padding: 10px;
}}

.terminal-pane-shell {{
  border-radius: 20px;
  border: {border_width}px solid rgba(224, 188, 255, 0.08);
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.040), rgba(255, 255, 255, 0.012)),
    {surface};
  box-shadow:
    0 16px 38px rgba(0, 0, 0, 0.30),
    inset 0 1px 0 rgba(255, 255, 255, 0.04);
  {shadow}
}}

.terminal-pane-shell.active {{
  border-color: {active};
  box-shadow:
    0 0 0 1px rgba(255, 255, 255, 0.03),
    0 18px 42px rgba(0, 0, 0, 0.34),
    0 0 30px rgba(185, 140, 255, 0.18);
}}

.terminal-pane-shell.mode-editor.active {{
  border-color: rgba(141, 238, 255, 0.92);
}}

.terminal-pane-shell.mode-monitor.active {{
  border-color: rgba(248, 221, 136, 0.92);
}}

.terminal-pane-shell.mode-remote.active {{
  border-color: rgba(255, 126, 169, 0.92);
}}

.terminal-pane-shell.action-flash {{
  box-shadow:
    0 0 0 1px rgba(255, 255, 255, 0.05),
    0 0 36px rgba(185, 140, 255, 0.28);
}}

.terminal-pane-shell.pane-born {{
  opacity: 0.0;
  transform: translateY(12px) scale(0.985);
  box-shadow:
    0 0 0 rgba(0, 0, 0, 0.0),
    inset 0 1px 0 rgba(255, 255, 255, 0.00);
}}

.terminal-pane-shell.pane-closing {{
  opacity: 0.0;
  transform: translateY(10px) scale(0.975);
  box-shadow:
    0 8px 20px rgba(0, 0, 0, 0.12),
    inset 0 1px 0 rgba(255, 255, 255, 0.00);
}}

.terminal-pane-shell.output-pulse {{
  box-shadow:
    0 0 0 1px rgba(255, 255, 255, 0.04),
    0 0 28px rgba(139, 233, 255, 0.14);
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.045), rgba(255, 255, 255, 0.015)),
    {surface};
}}

.terminal-pane-shell.compact {{
  border-radius: 16px;
}}

.terminal-pane-shell.dense {{
  border-radius: 14px;
}}

.terminal-pane-shell.drag-source {{
  opacity: 0.72;
}}

.terminal-pane-shell.drop-target {{
  box-shadow:
    0 0 0 1px rgba(255, 255, 255, 0.06),
    0 0 36px rgba(185, 140, 255, 0.24);
}}

.pane-ambient {{
  background:
    radial-gradient(circle at 18% 0%, rgba(170, 118, 255, 0.22), transparent 32%),
    radial-gradient(circle at 100% 78%, rgba(255, 126, 169, 0.10), transparent 25%),
    linear-gradient(180deg, rgba(14, 11, 25, 0.64), rgba(12, 10, 21, 0.82));
}}

.pane-wallpaper {{
  opacity: 0.0;
}}

.terminal-pane-shell.compact .pane-wallpaper {{
  opacity: 0.0;
}}

.terminal-pane-shell.dense .pane-wallpaper {{
  opacity: 0.0;
}}

.pane-tint {{
  background: rgba(9, 6, 18, 0.58);
}}

.pane-terminal-wrap {{
  padding: 0;
}}

.pane-terminal {{
  background-color: transparent;
}}

.pane-chrome {{
  padding: 12px 16px 10px;
  margin: 0;
  border-top-left-radius: 19px;
  border-top-right-radius: 19px;
  border-bottom: 1px solid rgba(218, 175, 255, 0.14);
  background:
    linear-gradient(180deg, rgba(17, 12, 28, 0.96), rgba(15, 11, 24, 0.84));
  box-shadow: inset 0 -1px 0 rgba(255, 255, 255, 0.03);
  color: {foreground};
}}

.terminal-pane-shell.compact .pane-chrome {{
  padding: 9px 12px 7px;
}}

.terminal-pane-shell.dense .pane-chrome {{
  padding: 7px 10px 5px;
}}

.pane-title {{
  font-weight: 800;
  font-size: 10pt;
}}

.pane-subtitle {{
  color: rgba(232, 221, 255, 0.70);
  font-size: 9pt;
}}

.context-badge {{
  padding: 3px 7px;
  border-radius: 999px;
  border: 1px solid rgba(224, 188, 255, 0.12);
  background: rgba(255, 255, 255, 0.06);
  color: {foreground};
  font-size: 7.5pt;
  font-weight: 700;
}}

.context-badge.accent {{
  background: {accent};
  color: rgba(18, 10, 29, 0.98);
}}

.palette-card {{
  min-width: 560px;
  border-radius: 22px;
  border: 1px solid rgba(224, 188, 255, 0.12);
  background:
    linear-gradient(180deg, rgba(18, 13, 30, 0.98), rgba(11, 8, 20, 0.99));
  box-shadow: 0 22px 58px rgba(0, 0, 0, 0.52);
  padding: 14px;
}}

.palette-list {{
  background: transparent;
}}

.palette-row {{
  border-radius: 14px;
  margin: 2px 0;
}}

.tile-paned > separator {{
  background: rgba(216, 169, 255, 0.18);
  min-width: 5px;
  min-height: 5px;
  border-radius: 999px;
}}

.header-utility-button {{
  margin: 4px;
  border-radius: 999px;
  border: 1px solid rgba(224, 188, 255, 0.08);
  background: rgba(255, 255, 255, 0.03);
  color: {foreground};
}}

button.header-utility-button:hover {{
  background: rgba(185, 140, 255, 0.12);
}}

{palette_css}
"#,
        surface = util::rgba_to_css(&surface),
        active = util::rgba_to_css(&active),
        foreground = util::rgba_to_css(&foreground),
        accent = util::rgba_to_css(&accent),
        palette_css = palette_css,
    )
}

fn build_preset_css(preset: PanePalettePreset) -> String {
    let preset = preset_theme(preset);
    let accent = util::rgba_to_css(&util::parse_rgba(preset.accent, "#ffffff"));
    let mut accent_glow = util::parse_rgba(preset.accent, "#ffffff");
    accent_glow.set_alpha(0.34);
    let border = util::rgba_to_css(&util::parse_rgba(preset.border, "#ffffff"));
    let foreground = util::rgba_to_css(&util::parse_rgba(preset.foreground, "#ffffff"));
    let cursor_text = util::rgba_to_css(&util::parse_rgba(preset.cursor_text, "#0f0f0f"));

    format!(
        r#"
.terminal-pane-shell.{class_name}.active {{
  border-color: {border};
  box-shadow:
    0 0 0 1px rgba(255, 255, 255, 0.03),
    0 18px 42px rgba(0, 0, 0, 0.34),
    0 0 34px {accent_glow};
}}

.terminal-pane-shell.{class_name}.drop-target {{
  border-color: {border};
}}

.terminal-pane-shell.{class_name} .pane-title {{
  color: {accent};
}}

.terminal-pane-shell.{class_name} .context-badge.accent {{
  background: {accent};
  color: {cursor_text};
}}

.terminal-pane-shell.{class_name} .pane-chrome {{
  color: {foreground};
}}
"#,
        class_name = preset.class_name,
        accent = accent,
        accent_glow = util::rgba_to_css(&accent_glow),
        border = border,
        foreground = foreground,
        cursor_text = cursor_text,
    )
}

fn preset_theme(preset: PanePalettePreset) -> PresetTheme {
    match preset {
        PanePalettePreset::Red => PresetTheme {
            class_name: "palette-red",
            foreground: "#ffe7ea",
            background: "#1b0b0d",
            accent: "#ff6b7d",
            border: "#ff8fa3",
            cursor_text: "#22090f",
            palette: [
                "#14090b", "#ff6378", "#6fd699", "#f4b860", "#7ea6ff", "#ff8fd1", "#82dfff",
                "#fceef0", "#6c3843", "#ff8e9d", "#9eebbf", "#ffd08b", "#a6bfff", "#ffb3df",
                "#b5ecff", "#ffffff",
            ],
        },
        PanePalettePreset::Green => PresetTheme {
            class_name: "palette-green",
            foreground: "#e9fff2",
            background: "#0a160f",
            accent: "#5fe39d",
            border: "#84f2b4",
            cursor_text: "#05110a",
            palette: [
                "#08110c", "#ff7b83", "#4fe092", "#d6c86b", "#67a8ff", "#a98aff", "#61e0e9",
                "#eefdf3", "#2d5742", "#ff9da3", "#7ff6b0", "#f0e28f", "#95c2ff", "#c9aaff",
                "#95f1f6", "#ffffff",
            ],
        },
        PanePalettePreset::Blue => PresetTheme {
            class_name: "palette-blue",
            foreground: "#edf6ff",
            background: "#09111d",
            accent: "#64b5ff",
            border: "#8ccdff",
            cursor_text: "#09131f",
            palette: [
                "#09101a", "#ff7f8b", "#72e3be", "#e1cc74", "#5fa9ff", "#b194ff", "#5fd5ff",
                "#edf6ff", "#36526f", "#ff9da7", "#9bf0d0", "#f1de99", "#8bc0ff", "#d1bcff",
                "#92e7ff", "#ffffff",
            ],
        },
        PanePalettePreset::Amber => PresetTheme {
            class_name: "palette-amber",
            foreground: "#fff7ea",
            background: "#181109",
            accent: "#ffbf5c",
            border: "#ffd27f",
            cursor_text: "#201406",
            palette: [
                "#130d08", "#ff7b68", "#89d58d", "#f0be52", "#7eafff", "#d89cff", "#79d9ff",
                "#fff8ee", "#6a4b25", "#ff9a86", "#a9ebb0", "#ffd57c", "#abc7ff", "#e7b5ff",
                "#a7ebff", "#ffffff",
            ],
        },
        PanePalettePreset::Rose => PresetTheme {
            class_name: "palette-rose",
            foreground: "#fff0f7",
            background: "#180a13",
            accent: "#ff7fc3",
            border: "#ffa4d6",
            cursor_text: "#220714",
            palette: [
                "#130911", "#ff758d", "#76d4a5", "#efc36e", "#8da6ff", "#ff82c9", "#76ddff",
                "#fff2f8", "#734765", "#ff99ae", "#9de8c0", "#ffd58d", "#b5c2ff", "#ffaddd",
                "#a7ebff", "#ffffff",
            ],
        },
        PanePalettePreset::Cyan => PresetTheme {
            class_name: "palette-cyan",
            foreground: "#ebfeff",
            background: "#081417",
            accent: "#63e4ff",
            border: "#8deeff",
            cursor_text: "#061417",
            palette: [
                "#071215", "#ff7b83", "#68df9f", "#e6c96d", "#6aa8ff", "#b493ff", "#57dcff",
                "#ecfdff", "#30535d", "#ff9fa3", "#94efba", "#f4dc96", "#94c0ff", "#d0baff",
                "#8aedff", "#ffffff",
            ],
        },
    }
}

struct PresetTheme {
    class_name: &'static str,
    foreground: &'static str,
    background: &'static str,
    accent: &'static str,
    border: &'static str,
    cursor_text: &'static str,
    palette: [&'static str; 16],
}
