use crate::config::AppConfig;
#[cfg(not(windows))]
use crate::config::CursorStyle;
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
            "red" | "ruby" | "rojo" => Some(Self::Red),
            "green" | "emerald" | "verde" => Some(Self::Green),
            "blue" | "azure" | "azul" => Some(Self::Blue),
            "amber" | "gold" | "yellow" | "ambar" | "ámbar" => Some(Self::Amber),
            "rose" | "pink" | "rosa" => Some(Self::Rose),
            "cyan" | "teal" | "cian" => Some(Self::Cyan),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Red => "Rojo",
            Self::Green => "Verde",
            Self::Blue => "Azul",
            Self::Amber => "Ámbar",
            Self::Rose => "Rosa",
            Self::Cyan => "Cian",
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

#[cfg(not(windows))]
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
        background.set_alpha(0.14);
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
    background.set_alpha(0.12);
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
    surface.set_alpha(0.36);
    let accent = util::parse_rgba(&config.accent_color, "#b98cff");
    let active = util::parse_rgba(&config.active_border_color, "#d2a1ff");
    let foreground = util::parse_rgba(&config.foreground_color, "#f4eeff");
    let mut window_surface = surface.clone();
    window_surface.set_alpha(0.20);
    let mut header_surface = surface.clone();
    header_surface.set_alpha(0.24);
    let mut pane_tint = surface.clone();
    pane_tint.set_alpha(0.42);
    let mut accent_glow = accent.clone();
    accent_glow.set_alpha(0.18);
    let mut active_glow = active.clone();
    active_glow.set_alpha(0.26);
    let mut active_border_glow = active.clone();
    active_border_glow.set_alpha(0.12);
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
  background: rgba(6, 6, 9, 0.98);
}}

.app-shell {{
  background: transparent;
}}

.window-surface {{
  background:
    radial-gradient(circle at 18% 0%, rgba(255, 255, 255, 0.03), transparent 32%),
    radial-gradient(circle at 100% 100%, {accent_glow}, transparent 28%),
    linear-gradient(180deg, rgba(7, 7, 11, 0.58), rgba(7, 8, 12, 0.76));
}}

.window-toolbar-view {{
  background: transparent;
}}

.termvoid-headerbar {{
  margin: 12px 16px 0;
  padding: 4px 8px;
  border-radius: 18px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.065), rgba(255, 255, 255, 0.018)),
    {header_surface};
  box-shadow:
    0 14px 32px rgba(0, 0, 0, 0.24),
    inset 0 1px 0 rgba(255, 255, 255, 0.08);
  color: {foreground};
}}

.termvoid-title {{
  color: {foreground};
  text-shadow: 0 1px 10px rgba(0, 0, 0, 0.18);
}}

.layout-surface {{
  padding: 14px 16px 16px;
  background:
    radial-gradient(circle at 0% 20%, rgba(255, 255, 255, 0.02), transparent 24%),
    radial-gradient(circle at 100% 0%, {active_border_glow}, transparent 26%),
    linear-gradient(180deg, rgba(6, 7, 10, 0.10), rgba(5, 6, 8, 0.22));
}}

.shared-wallpaper {{
  opacity: 1.0;
}}

.shared-wallpaper-tint {{
  background:
    radial-gradient(circle at 24% 0%, rgba(255, 255, 255, 0.08), transparent 30%),
    radial-gradient(circle at 86% 20%, {accent_glow}, transparent 28%),
    linear-gradient(180deg, rgba(9, 10, 14, 0.30), rgba(7, 7, 9, 0.48));
}}

.layout-surface.zoomed {{
  padding: 10px 12px 12px;
}}

.terminal-pane-shell {{
  border-radius: 18px;
  border: {border_width}px solid rgba(255, 255, 255, 0.10);
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.075), rgba(255, 255, 255, 0.018)),
    {surface};
  box-shadow:
    0 22px 52px rgba(0, 0, 0, 0.30),
    0 0 0 1px rgba(255, 255, 255, 0.02),
    inset 0 1px 0 rgba(255, 255, 255, 0.10);
  {shadow}
}}

.terminal-pane-shell.active {{
  border-color: {active};
  box-shadow:
    0 0 0 1px rgba(255, 255, 255, 0.06),
    0 30px 70px rgba(0, 0, 0, 0.40),
    0 0 0 1px {active_border_glow},
    0 0 42px {active_glow};
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
    0 0 44px {accent_glow};
}}

.terminal-pane-shell.pane-born {{
  opacity: 0.0;
  box-shadow:
    0 0 0 rgba(0, 0, 0, 0.0),
    inset 0 1px 0 rgba(255, 255, 255, 0.00);
}}

.terminal-pane-shell.spawn-from-center {{
  transform: scale(0.962);
}}

.terminal-pane-shell.spawn-from-left {{
  transform: translateX(-72px) scale(0.972);
}}

.terminal-pane-shell.spawn-from-right {{
  transform: translateX(72px) scale(0.972);
}}

.terminal-pane-shell.spawn-from-top {{
  transform: translateY(-58px) scale(0.972);
}}

.terminal-pane-shell.spawn-from-bottom {{
  transform: translateY(58px) scale(0.972);
}}

.terminal-pane-shell.spawn-overshoot-center {{
  transform: scale(1.014);
}}

.terminal-pane-shell.spawn-overshoot-left {{
  transform: translateX(14px) scale(1.006);
}}

.terminal-pane-shell.spawn-overshoot-right {{
  transform: translateX(-14px) scale(1.006);
}}

.terminal-pane-shell.spawn-overshoot-top {{
  transform: translateY(12px) scale(1.006);
}}

.terminal-pane-shell.spawn-overshoot-bottom {{
  transform: translateY(-12px) scale(1.006);
}}

.terminal-pane-shell.close-kick-center {{
  transform: scale(1.014);
}}

.terminal-pane-shell.close-kick-left {{
  transform: translateX(-12px) scale(1.004);
}}

.terminal-pane-shell.close-kick-right {{
  transform: translateX(12px) scale(1.004);
}}

.terminal-pane-shell.close-kick-top {{
  transform: translateY(-10px) scale(1.004);
}}

.terminal-pane-shell.close-kick-bottom {{
  transform: translateY(10px) scale(1.004);
}}

.terminal-pane-shell.pane-closing {{
  opacity: 0.0;
  box-shadow:
    0 10px 28px rgba(0, 0, 0, 0.16),
    inset 0 1px 0 rgba(255, 255, 255, 0.00);
}}

.terminal-pane-shell.pane-closing.close-to-center {{
  transform: scale(0.952);
}}

.terminal-pane-shell.pane-closing.close-to-left {{
  transform: translateX(-86px) scale(0.964);
}}

.terminal-pane-shell.pane-closing.close-to-right {{
  transform: translateX(86px) scale(0.964);
}}

.terminal-pane-shell.pane-closing.close-to-top {{
  transform: translateY(-68px) scale(0.964);
}}

.terminal-pane-shell.pane-closing.close-to-bottom {{
  transform: translateY(68px) scale(0.964);
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
  border-radius: 15px;
}}

.terminal-pane-shell.dense {{
  border-radius: 13px;
}}

.terminal-pane-shell.drag-source {{
  opacity: 0.72;
}}

.terminal-pane-shell.drop-target {{
  box-shadow:
    0 0 0 1px rgba(255, 255, 255, 0.06),
    0 0 40px {accent_glow};
}}

.pane-ambient {{
  background:
    radial-gradient(circle at 12% 0%, rgba(255, 255, 255, 0.08), transparent 24%),
    radial-gradient(circle at 100% 80%, {accent_glow}, transparent 24%),
    linear-gradient(180deg, rgba(10, 10, 14, 0.18), rgba(8, 9, 12, 0.34));
}}

.pane-wallpaper {{
  opacity: 0.42;
}}

.terminal-pane-shell.compact .pane-wallpaper {{
  opacity: 0.30;
}}

.terminal-pane-shell.dense .pane-wallpaper {{
  opacity: 0.18;
}}

.pane-tint {{
  background:
    linear-gradient(180deg, rgba(8, 8, 12, 0.14), rgba(6, 6, 9, 0.26)),
    {pane_tint};
}}

.pane-terminal-wrap {{
  padding: 0;
}}

.pane-terminal {{
  background-color: transparent;
}}

.pane-chrome {{
  padding: 10px 14px 9px;
  margin: 0;
  border-top-left-radius: 17px;
  border-top-right-radius: 17px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.055), rgba(255, 255, 255, 0.016)),
    {window_surface};
  box-shadow: inset 0 -1px 0 rgba(255, 255, 255, 0.04);
  color: {foreground};
}}

.terminal-pane-shell.compact .pane-chrome {{
  padding: 8px 11px 6px;
}}

.terminal-pane-shell.dense .pane-chrome {{
  padding: 6px 9px 5px;
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
  border: 1px solid rgba(255, 255, 255, 0.10);
  background: rgba(255, 255, 255, 0.08);
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
  border-radius: 24px;
  border: 1px solid rgba(255, 255, 255, 0.10);
  background:
    radial-gradient(circle at top right, rgba(255, 255, 255, 0.07), transparent 34%),
    linear-gradient(180deg, rgba(18, 18, 22, 0.985), rgba(12, 12, 16, 0.995));
  box-shadow:
    0 24px 64px rgba(0, 0, 0, 0.44),
    inset 0 1px 0 rgba(255, 255, 255, 0.08);
  padding: 14px;
}}

.palette-list {{
  background: rgba(16, 16, 20, 0.98);
}}

.palette-card searchentry {{
  border-radius: 16px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(28, 28, 34, 0.99);
  color: {foreground};
}}

.palette-row {{
  border-radius: 16px;
  margin: 3px 0;
  border: 1px solid rgba(255, 255, 255, 0.06);
  background: rgba(30, 29, 36, 0.99);
  transition: background-color 140ms ease, border-color 140ms ease, transform 140ms ease;
}}

.palette-row:hover {{
  background: rgba(41, 39, 48, 0.995);
  border-color: rgba(255, 255, 255, 0.10);
}}

.palette-row:active {{
  transform: scale(0.995);
}}

.palette-row-prefix {{
  min-width: 32px;
  min-height: 32px;
  border-radius: 12px;
  margin: 2px 2px 2px 0;
  border: 1px solid rgba(255, 255, 255, 0.04);
  background: rgba(255, 255, 255, 0.04);
  color: {foreground};
}}

.palette-row-prefix.section-suggested {{
  background: rgba(139, 233, 255, 0.10);
  color: rgba(162, 238, 255, 0.98);
}}

.palette-row-prefix.section-layout {{
  background: rgba(185, 140, 255, 0.10);
  color: rgba(217, 169, 255, 0.98);
}}

.palette-row-prefix.section-workspace {{
  background: rgba(152, 245, 194, 0.10);
  color: rgba(178, 255, 214, 0.98);
}}

.palette-row-prefix.section-git {{
  background: rgba(255, 167, 107, 0.12);
  color: rgba(255, 205, 167, 0.99);
}}

.palette-row-prefix.section-commands {{
  background: rgba(255, 215, 134, 0.10);
  color: rgba(255, 230, 171, 0.98);
}}

.palette-row-prefix.section-remote {{
  background: rgba(255, 126, 169, 0.10);
  color: rgba(255, 159, 191, 0.98);
}}

.palette-row-prefix.section-theme {{
  background: rgba(145, 210, 255, 0.10);
  color: rgba(173, 223, 255, 0.98);
}}

.palette-row-prefix-icon {{
  margin: auto;
}}

.palette-target-badge {{
  padding: 3px 8px;
  border-radius: 999px;
  border: 1px solid rgba(139, 233, 255, 0.18);
  background: rgba(139, 233, 255, 0.08);
  color: rgba(173, 241, 255, 0.98);
  font-size: 7.2pt;
  font-weight: 800;
}}

.palette-section-row {{
  margin: 10px 0 2px;
  padding: 2px 4px;
  background: transparent;
}}

.palette-section-icon {{
  color: rgba(214, 186, 255, 0.88);
}}

.palette-section-label {{
  color: rgba(229, 216, 255, 0.88);
  font-size: 9pt;
  font-weight: 800;
  letter-spacing: 0.03em;
}}

.palette-section-count {{
  color: rgba(180, 165, 208, 0.78);
  font-size: 8pt;
  font-weight: 700;
}}

.palette-empty-row {{
  margin: 6px 0;
  padding: 12px 14px;
  border-radius: 16px;
  border: 1px dashed rgba(255, 255, 255, 0.10);
  background: rgba(24, 24, 28, 0.99);
}}

.palette-empty-title {{
  color: rgba(241, 235, 255, 0.96);
  font-weight: 800;
}}

.palette-empty-subtitle {{
  color: rgba(188, 175, 212, 0.76);
}}

.tile-paned > separator {{
  background: rgba(255, 255, 255, 0.12);
  min-width: 4px;
  min-height: 4px;
  border-radius: 999px;
}}

.header-utility-button {{
  margin: 4px;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.05);
  color: {foreground};
}}

button.header-utility-button:hover {{
  background: rgba(255, 255, 255, 0.10);
}}

{palette_css}
"#,
        surface = util::rgba_to_css(&surface),
        window_surface = util::rgba_to_css(&window_surface),
        header_surface = util::rgba_to_css(&header_surface),
        pane_tint = util::rgba_to_css(&pane_tint),
        active = util::rgba_to_css(&active),
        active_glow = util::rgba_to_css(&active_glow),
        active_border_glow = util::rgba_to_css(&active_border_glow),
        foreground = util::rgba_to_css(&foreground),
        accent = util::rgba_to_css(&accent),
        accent_glow = util::rgba_to_css(&accent_glow),
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
