use crate::context::{PanelContext, PanelMode};
use crate::history::HistoryStore;
use crate::layout::Direction;
use crate::theme::PanePalettePreset;
use crate::util;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionTarget {
    CurrentPane,
    NewPane,
}

#[derive(Debug, Clone)]
pub enum QuickActionCommand {
    Shell(String),
    ChangeDirectory(PathBuf),
    OpenFileManager(PathBuf),
    Internal(InternalAction),
}

#[derive(Debug, Clone)]
pub enum InternalAction {
    ShowInfo,
    TogglePaneZoom,
    SwapPane(Direction),
    SetPanePalette(Option<PanePalettePreset>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QuickActionSection {
    Suggested,
    Layout,
    Workspace,
    Git,
    Commands,
    Remote,
    Theme,
}

impl QuickActionSection {
    pub fn label(self) -> &'static str {
        match self {
            Self::Suggested => "Sugerencias",
            Self::Layout => "Ventana y Panel",
            Self::Workspace => "Espacio de trabajo",
            Self::Git => "Git y Proyecto",
            Self::Commands => "Comandos",
            Self::Remote => "Remoto y Contenedores",
            Self::Theme => "Tema y color",
        }
    }

    pub fn icon_name(self) -> &'static str {
        match self {
            Self::Suggested => "starred-symbolic",
            Self::Layout => "view-grid-symbolic",
            Self::Workspace => "folder-symbolic",
            Self::Git => "version-control-symbolic",
            Self::Commands => "utilities-terminal-symbolic",
            Self::Remote => "network-workgroup-symbolic",
            Self::Theme => "applications-graphics-symbolic",
        }
    }

    pub fn css_class(self) -> &'static str {
        match self {
            Self::Suggested => "suggested",
            Self::Layout => "layout",
            Self::Workspace => "workspace",
            Self::Git => "git",
            Self::Commands => "commands",
            Self::Remote => "remote",
            Self::Theme => "theme",
        }
    }
}

#[derive(Debug, Clone)]
pub struct QuickActionItem {
    pub section: QuickActionSection,
    pub title: String,
    pub subtitle: String,
    pub badge: Option<String>,
    pub target: ActionTarget,
    pub command: QuickActionCommand,
}

pub fn collect_actions(
    context: Option<&PanelContext>,
    history: &HistoryStore,
) -> Vec<QuickActionItem> {
    let mut items = built_in_actions();

    if let Some(cwd) = context.and_then(|value| value.cwd.clone()) {
        items.push(QuickActionItem {
            section: QuickActionSection::Workspace,
            title: "Abrir directorio actual en Archivos".to_string(),
            subtitle: cwd.display().to_string(),
            badge: Some("ARCHIVOS".to_string()),
            target: ActionTarget::CurrentPane,
            command: QuickActionCommand::OpenFileManager(cwd),
        });
    }

    items.extend(ssh_host_actions());
    items.extend(container_actions("docker"));
    items.extend(container_actions("podman"));
    items.extend(history_directory_actions(history));
    items.extend(history_project_actions(history));
    items.extend(history_action_actions(history));
    items.extend(history_command_actions(history));
    items.extend(history_connection_actions(history));

    dedupe(items)
}

pub fn query_actions(query: &str, context: Option<&PanelContext>) -> Vec<QuickActionItem> {
    let query = query.trim();
    if query.is_empty() {
        return Vec::new();
    }

    let mut items = Vec::new();
    let badge = infer_badge(query, context);

    let expanded_path = util::expand_user_path(query);
    if expanded_path.is_dir() {
        let path_string = expanded_path.display().to_string();
        items.push(QuickActionItem {
            section: QuickActionSection::Suggested,
            title: format!("cd {}", util::compact_label(&path_string)),
            subtitle: "Cambiar el directorio de trabajo del panel activo".to_string(),
            badge: Some("DIR".to_string()),
            target: ActionTarget::CurrentPane,
            command: QuickActionCommand::ChangeDirectory(expanded_path.clone()),
        });
        items.push(QuickActionItem {
            section: QuickActionSection::Suggested,
            title: format!("Abrir {}", util::compact_label(&path_string)),
            subtitle: "Mostrar este directorio en el gestor de archivos".to_string(),
            badge: Some("ARCHIVOS".to_string()),
            target: ActionTarget::CurrentPane,
            command: QuickActionCommand::OpenFileManager(expanded_path),
        });
    }

    items.extend(internal_query_actions(query));

    let title = freeform_title(query);
    let (_, preferred_target) = classify_shell_command(query, context);
    if matches!(preferred_target, ActionTarget::CurrentPane) {
        items.push(QuickActionItem {
            section: QuickActionSection::Suggested,
            title: format!("Ejecutar {title}"),
            subtitle: "Ejecutar en el panel activo".to_string(),
            badge: badge.clone(),
            target: ActionTarget::CurrentPane,
            command: QuickActionCommand::Shell(query.to_string()),
        });
    }
    items.push(QuickActionItem {
        section: QuickActionSection::Suggested,
        title: format!("Ejecutar {title} en un panel nuevo"),
        subtitle: "Dividir y ejecutar en un panel nuevo".to_string(),
        badge,
        target: ActionTarget::NewPane,
        command: QuickActionCommand::Shell(query.to_string()),
    });

    dedupe_items(items)
}

pub fn detected_command_entry(context: &PanelContext) -> Option<(String, String, String)> {
    let command = context.foreground_command.as_ref()?.trim();
    let process = context.foreground_process.as_deref()?;

    if command.is_empty() || process.eq_ignore_ascii_case(&context.shell) {
        return None;
    }

    let title = format!("Ejecutar {}", prettify_command(process));
    let category = infer_badge(command, Some(context)).unwrap_or_else(|| "COMANDO".to_string());
    Some((title, command.to_string(), category))
}

pub fn infer_badge(query: &str, context: Option<&PanelContext>) -> Option<String> {
    let token = util::first_command_token(query)?;
    let token = util::shell_name(&token).to_ascii_lowercase();

    let badge = match token.as_str() {
        "nvim" | "vim" | "hx" | "helix" | "nano" | "micro" | "emacs" | "less" => "EDITOR",
        "htop" | "btop" | "top" | "watch" | "iftop" | "nvtop" => "MONITOR",
        "ssh" | "mosh" => "SSH",
        "docker" | "podman" | "distrobox" | "toolbox" => "CONTENEDOR",
        "git" | "lazygit" | "gitui" | "gh" => "GIT",
        "cargo" | "make" | "just" | "npm" | "pnpm" | "yarn" | "go" => "COMPILAR",
        _ => match context.map(|value| &value.mode) {
            Some(PanelMode::Editor) => "EDITOR",
            Some(PanelMode::Monitor) => "MONITOR",
            Some(PanelMode::Container) => "CONTENEDOR",
            Some(PanelMode::Remote) => "REMOTO",
            _ => "COMANDO",
        },
    };

    Some(badge.to_string())
}

pub fn dedupe_items(items: Vec<QuickActionItem>) -> Vec<QuickActionItem> {
    dedupe(items)
}

fn built_in_actions() -> Vec<QuickActionItem> {
    let mut items = Vec::new();

    items.push(QuickActionItem {
        section: QuickActionSection::Layout,
        title: "Mostrar banner de información del sistema".to_string(),
        subtitle: "Imprimir el banner ASCII y el resumen actual del sistema en este panel."
            .to_string(),
        badge: Some("INFO".to_string()),
        target: ActionTarget::CurrentPane,
        command: QuickActionCommand::Internal(InternalAction::ShowInfo),
    });

    items.push(QuickActionItem {
        section: QuickActionSection::Layout,
        title: "Alternar zoom del panel".to_string(),
        subtitle:
            "Acercar o restaurar el panel enfocado sin poner la ventana en pantalla completa."
                .to_string(),
        badge: Some("PANEL".to_string()),
        target: ActionTarget::CurrentPane,
        command: QuickActionCommand::Internal(InternalAction::TogglePaneZoom),
    });

    if let Some(editor) = first_available_command(&["nvim .", "vim .", "hx ."]) {
        items.push(QuickActionItem {
            section: QuickActionSection::Commands,
            title: format!("{} aquí", prettify_command(&editor)),
            subtitle: "Abrir un editor en el directorio de trabajo actual".to_string(),
            badge: Some("EDITOR".to_string()),
            target: ActionTarget::CurrentPane,
            command: QuickActionCommand::Shell(editor),
        });
    }

    if let Some(monitor) = first_available_command(&["htop", "btop", "top"]) {
        items.push(QuickActionItem {
            section: QuickActionSection::Commands,
            title: format!("{} aquí", prettify_command(&monitor)),
            subtitle: "Ejecutar un monitor del sistema en el panel actual".to_string(),
            badge: Some("MONITOR".to_string()),
            target: ActionTarget::CurrentPane,
            command: QuickActionCommand::Shell(monitor.clone()),
        });
        items.push(QuickActionItem {
            section: QuickActionSection::Commands,
            title: format!("{} en un panel nuevo", prettify_command(&monitor)),
            subtitle: "Abrir un panel nuevo de monitoreo".to_string(),
            badge: Some("MONITOR".to_string()),
            target: ActionTarget::NewPane,
            command: QuickActionCommand::Shell(monitor),
        });
    }

    if util::command_exists("git") {
        items.push(QuickActionItem {
            section: QuickActionSection::Git,
            title: "git status".to_string(),
            subtitle: "Inspeccionar el estado actual del repositorio".to_string(),
            badge: Some("GIT".to_string()),
            target: ActionTarget::CurrentPane,
            command: QuickActionCommand::Shell("git status".to_string()),
        });
    }

    items.extend(palette_preset_actions());

    items
}

fn ssh_host_actions() -> Vec<QuickActionItem> {
    let Some(home) = util::home_dir() else {
        return Vec::new();
    };
    let config_path = home.join(".ssh/config");
    let Ok(content) = fs::read_to_string(config_path) else {
        return Vec::new();
    };

    content
        .lines()
        .filter_map(|line| line.trim().strip_prefix("Host "))
        .flat_map(|hosts| hosts.split_whitespace())
        .filter(|host| !host.contains('*') && !host.contains('?'))
        .map(|host| QuickActionItem {
            section: QuickActionSection::Remote,
            title: format!("SSH {host}"),
            subtitle: "Abrir una shell remota en un panel nuevo".to_string(),
            badge: Some("SSH".to_string()),
            target: ActionTarget::NewPane,
            command: QuickActionCommand::Shell(format!("ssh {}", util::shell_quote(host))),
        })
        .collect()
}

fn container_actions(runtime: &str) -> Vec<QuickActionItem> {
    let Some(names) = util::command_output(runtime, &["ps", "--format", "{{.Names}}"]) else {
        return Vec::new();
    };

    names
        .lines()
        .filter(|name| !name.trim().is_empty())
        .map(|name| QuickActionItem {
            section: QuickActionSection::Remote,
            title: format!("{runtime} shell {name}"),
            subtitle: format!("Adjuntar /bin/sh en {name}"),
            badge: Some(runtime.to_ascii_uppercase()),
            target: ActionTarget::NewPane,
            command: QuickActionCommand::Shell(format!(
                "{runtime} exec -it {} /bin/sh",
                util::shell_quote(name)
            )),
        })
        .collect()
}

fn history_directory_actions(history: &HistoryStore) -> Vec<QuickActionItem> {
    history
        .recent_directories
        .iter()
        .take(6)
        .map(|entry| {
            let path = PathBuf::from(&entry.path);
            QuickActionItem {
                section: QuickActionSection::Workspace,
                title: format!("Abrir {}", util::compact_label(&entry.path)),
                subtitle: "Ir a un directorio reciente".to_string(),
                badge: Some("DIR".to_string()),
                target: ActionTarget::CurrentPane,
                command: QuickActionCommand::ChangeDirectory(path),
            }
        })
        .collect()
}

fn history_project_actions(history: &HistoryStore) -> Vec<QuickActionItem> {
    history
        .recent_projects
        .iter()
        .take(6)
        .map(|entry| {
            let path = PathBuf::from(&entry.path);
            QuickActionItem {
                section: QuickActionSection::Workspace,
                title: format!("Proyecto {}", util::compact_label(&entry.path)),
                subtitle: "Abrir un proyecto Git reciente".to_string(),
                badge: Some("PROYECTO".to_string()),
                target: ActionTarget::CurrentPane,
                command: QuickActionCommand::ChangeDirectory(path),
            }
        })
        .collect()
}

fn history_action_actions(history: &HistoryStore) -> Vec<QuickActionItem> {
    history
        .recent_quick_actions
        .iter()
        .take(6)
        .map(|entry| {
            let (section, target) = classify_shell_command(&entry.command, None);
            QuickActionItem {
                section,
                title: format!("Repetir {}", entry.title),
                subtitle: entry.command.clone(),
                badge: infer_badge(&entry.command, None).or(Some("RECIENTE".to_string())),
                target,
                command: QuickActionCommand::Shell(entry.command.clone()),
            }
        })
        .collect()
}

fn history_command_actions(history: &HistoryStore) -> Vec<QuickActionItem> {
    history
        .recent_commands
        .iter()
        .take(8)
        .map(|entry| {
            let (section, target) = classify_shell_command(&entry.command, None);
            QuickActionItem {
                section,
                title: entry.title.clone(),
                subtitle: entry.command.clone(),
                badge: Some(entry.category.clone()),
                target,
                command: QuickActionCommand::Shell(entry.command.clone()),
            }
        })
        .collect()
}

fn history_connection_actions(history: &HistoryStore) -> Vec<QuickActionItem> {
    history
        .recent_connections
        .iter()
        .take(4)
        .map(|entry| QuickActionItem {
            section: QuickActionSection::Remote,
            title: entry.label.clone(),
            subtitle: entry.command.clone(),
            badge: infer_badge(&entry.command, None).or(Some("CONEXIÓN".to_string())),
            target: ActionTarget::NewPane,
            command: QuickActionCommand::Shell(entry.command.clone()),
        })
        .collect()
}

fn first_available_command(commands: &[&str]) -> Option<String> {
    commands
        .iter()
        .copied()
        .find(|command| util::command_line_exists(command))
        .map(str::to_string)
}

fn freeform_title(query: &str) -> String {
    format!("`{}`", util::compact_label(query))
}

fn prettify_command(command: &str) -> String {
    util::first_command_token(command)
        .map(|token| util::shell_name(&token))
        .unwrap_or_else(|| command.to_string())
}

fn dedupe(items: Vec<QuickActionItem>) -> Vec<QuickActionItem> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();

    for item in items {
        if seen.insert(action_identity(&item)) {
            deduped.push(item);
        }
    }

    deduped
}

fn classify_shell_command(
    command: &str,
    context: Option<&PanelContext>,
) -> (QuickActionSection, ActionTarget) {
    let token = util::first_command_token(command)
        .map(|token| util::shell_name(&token).to_ascii_lowercase())
        .unwrap_or_default();

    match token.as_str() {
        "git" | "lazygit" | "gitui" | "gh" => (QuickActionSection::Git, ActionTarget::CurrentPane),
        "ssh" | "mosh" | "scp" | "sftp" | "docker" | "podman" | "distrobox" | "toolbox" => {
            (QuickActionSection::Remote, ActionTarget::NewPane)
        }
        _ => match context.map(|value| &value.mode) {
            Some(PanelMode::Remote) | Some(PanelMode::Container) => {
                (QuickActionSection::Remote, ActionTarget::NewPane)
            }
            _ => (QuickActionSection::Commands, ActionTarget::CurrentPane),
        },
    }
}

fn action_identity(item: &QuickActionItem) -> String {
    let target = match item.target {
        ActionTarget::CurrentPane => "current",
        ActionTarget::NewPane => "new",
    };

    match &item.command {
        QuickActionCommand::Shell(command) => {
            format!("shell:{target}:{}", command.trim())
        }
        QuickActionCommand::ChangeDirectory(path) => {
            format!("cd:{}", path.display())
        }
        QuickActionCommand::OpenFileManager(path) => {
            format!("files:{}", path.display())
        }
        QuickActionCommand::Internal(action) => match action {
            InternalAction::ShowInfo => "internal:show-info".to_string(),
            InternalAction::TogglePaneZoom => "internal:toggle-zoom".to_string(),
            InternalAction::SwapPane(direction) => {
                format!("internal:swap:{}", direction_slug(*direction))
            }
            InternalAction::SetPanePalette(Some(preset)) => {
                format!("internal:theme:{}", preset.slug())
            }
            InternalAction::SetPanePalette(None) => "internal:theme:reset".to_string(),
        },
    }
}

fn internal_query_actions(query: &str) -> Vec<QuickActionItem> {
    let Some(command) = normalize_internal_query(query) else {
        return Vec::new();
    };

    if matches!(command, "info" | "banner" | "fetch") {
        return vec![QuickActionItem {
            section: QuickActionSection::Suggested,
            title: "Mostrar banner de información del sistema".to_string(),
            subtitle: "Imprimir la información ASCII actual del sistema en el panel activo."
                .to_string(),
            badge: Some("INFO".to_string()),
            target: ActionTarget::CurrentPane,
            command: QuickActionCommand::Internal(InternalAction::ShowInfo),
        }];
    }

    if matches!(command, "zoom" | "fullscreen" | "focus") {
        return vec![QuickActionItem {
            section: QuickActionSection::Suggested,
            title: "Alternar zoom del panel".to_string(),
            subtitle: "Acercar o restaurar el panel enfocado.".to_string(),
            badge: Some("PANEL".to_string()),
            target: ActionTarget::CurrentPane,
            command: QuickActionCommand::Internal(InternalAction::TogglePaneZoom),
        }];
    }

    if let Some(direction) =
        parse_directional_command(command, &["swap", "move", "intercambiar", "mover"])
    {
        return vec![QuickActionItem {
            section: QuickActionSection::Suggested,
            title: format!("Intercambiar panel {}", direction_label(direction)),
            subtitle: "Intercambiar el panel activo con su vecino más cercano.".to_string(),
            badge: Some("PANEL".to_string()),
            target: ActionTarget::CurrentPane,
            command: QuickActionCommand::Internal(InternalAction::SwapPane(direction)),
        }];
    }

    let theme_name = command
        .strip_prefix("theme ")
        .or_else(|| command.strip_prefix("tema "))
        .or_else(|| command.strip_prefix("palette "))
        .or_else(|| command.strip_prefix("paleta "));
    if let Some(theme_name) = theme_name.map(str::trim) {
        if matches!(
            theme_name,
            "default" | "reset" | "base" | "predeterminado" | "restablecer"
        ) {
            return vec![QuickActionItem {
                section: QuickActionSection::Suggested,
                title: "Restablecer paleta del panel".to_string(),
                subtitle: "Restaurar el panel a la paleta global de la terminal.".to_string(),
                badge: Some("TEMA".to_string()),
                target: ActionTarget::CurrentPane,
                command: QuickActionCommand::Internal(InternalAction::SetPanePalette(None)),
            }];
        }

        if let Some(preset) = PanePalettePreset::from_name(theme_name) {
            return vec![QuickActionItem {
                section: QuickActionSection::Suggested,
                title: format!("Cambiar paleta del panel a {}", preset.label()),
                subtitle: "Aplicar un preset de color por panel.".to_string(),
                badge: Some("TEMA".to_string()),
                target: ActionTarget::CurrentPane,
                command: QuickActionCommand::Internal(InternalAction::SetPanePalette(Some(preset))),
            }];
        }
    }

    Vec::new()
}

fn palette_preset_actions() -> Vec<QuickActionItem> {
    let mut items = Vec::new();

    items.push(QuickActionItem {
        section: QuickActionSection::Theme,
        title: "Restablecer paleta del panel".to_string(),
        subtitle: "Devolver el panel activo a la paleta predeterminada.".to_string(),
        badge: Some("TEMA".to_string()),
        target: ActionTarget::CurrentPane,
        command: QuickActionCommand::Internal(InternalAction::SetPanePalette(None)),
    });

    for preset in PanePalettePreset::ALL {
        items.push(QuickActionItem {
            section: QuickActionSection::Theme,
            title: format!("Paleta del panel: {}", preset.label()),
            subtitle: "Aplicar un preset de color por panel.".to_string(),
            badge: Some("TEMA".to_string()),
            target: ActionTarget::CurrentPane,
            command: QuickActionCommand::Internal(InternalAction::SetPanePalette(Some(preset))),
        });
    }

    items
}

fn normalize_internal_query(query: &str) -> Option<&str> {
    let trimmed = query.trim();
    trimmed
        .strip_prefix(':')
        .or_else(|| trimmed.strip_prefix("voidshell "))
        .or_else(|| trimmed.strip_prefix("tv "))
        .or_else(|| trimmed.strip_prefix("termvoid "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn parse_directional_command(input: &str, prefixes: &[&str]) -> Option<Direction> {
    let mut parts = input.split_whitespace();
    let prefix = parts.next()?;
    if !prefixes.iter().any(|candidate| *candidate == prefix) {
        return None;
    }

    match parts.next()? {
        "left" | "west" | "izquierda" => Some(Direction::Left),
        "right" | "east" | "derecha" => Some(Direction::Right),
        "up" | "north" | "arriba" => Some(Direction::Up),
        "down" | "south" | "abajo" => Some(Direction::Down),
        _ => None,
    }
}

fn direction_label(direction: Direction) -> &'static str {
    match direction {
        Direction::Left => "a la izquierda",
        Direction::Right => "a la derecha",
        Direction::Up => "hacia arriba",
        Direction::Down => "hacia abajo",
    }
}

fn direction_slug(direction: Direction) -> &'static str {
    match direction {
        Direction::Left => "left",
        Direction::Right => "right",
        Direction::Up => "up",
        Direction::Down => "down",
    }
}

pub fn match_score(item: &QuickActionItem, query: &str) -> i32 {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return 0;
    }

    let title = item.title.to_ascii_lowercase();
    let subtitle = item.subtitle.to_ascii_lowercase();
    let badge = item
        .badge
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();

    let mut score = 0;
    if title == query {
        score += 120;
    }
    if title.starts_with(&query) {
        score += 90;
    } else if title.contains(&query) {
        score += 56;
    }
    if subtitle.starts_with(&query) {
        score += 42;
    } else if subtitle.contains(&query) {
        score += 26;
    }
    if badge == query {
        score += 24;
    } else if !badge.is_empty() && badge.contains(&query) {
        score += 12;
    }
    if item.section == QuickActionSection::Suggested {
        score += 18;
    }
    if matches!(item.target, ActionTarget::CurrentPane) {
        score += 3;
    }

    score
}
