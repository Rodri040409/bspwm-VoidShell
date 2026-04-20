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
    Commands,
    Remote,
    Theme,
}

impl QuickActionSection {
    pub fn label(self) -> &'static str {
        match self {
            Self::Suggested => "Sugerencias",
            Self::Layout => "Ventana y Panel",
            Self::Workspace => "Workspace",
            Self::Commands => "Comandos",
            Self::Remote => "Remoto y Contenedores",
            Self::Theme => "Tema y Color",
        }
    }

    pub fn icon_name(self) -> &'static str {
        match self {
            Self::Suggested => "starred-symbolic",
            Self::Layout => "view-grid-symbolic",
            Self::Workspace => "folder-symbolic",
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
            Self::Commands => "commands",
            Self::Remote => "remote",
            Self::Theme => "theme",
        }
    }
}

#[derive(Debug, Clone)]
pub struct QuickActionItem {
    pub id: String,
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
            id: "files-current".to_string(),
            section: QuickActionSection::Workspace,
            title: "Open current directory in Files".to_string(),
            subtitle: cwd.display().to_string(),
            badge: Some("FILES".to_string()),
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
            id: format!("query-cd-{path_string}"),
            section: QuickActionSection::Suggested,
            title: format!("cd {}", util::compact_label(&path_string)),
            subtitle: "Change the current pane working directory".to_string(),
            badge: Some("DIR".to_string()),
            target: ActionTarget::CurrentPane,
            command: QuickActionCommand::ChangeDirectory(expanded_path.clone()),
        });
        items.push(QuickActionItem {
            id: format!("query-files-{path_string}"),
            section: QuickActionSection::Suggested,
            title: format!("Open {}", util::compact_label(&path_string)),
            subtitle: "Reveal this directory in the file manager".to_string(),
            badge: Some("FILES".to_string()),
            target: ActionTarget::CurrentPane,
            command: QuickActionCommand::OpenFileManager(expanded_path),
        });
    }

    items.extend(internal_query_actions(query));

    let title = freeform_title(query);
    items.push(QuickActionItem {
        id: format!("query-run-current-{query}"),
        section: QuickActionSection::Suggested,
        title: format!("Run {title}"),
        subtitle: "Execute in the active pane".to_string(),
        badge: badge.clone(),
        target: ActionTarget::CurrentPane,
        command: QuickActionCommand::Shell(query.to_string()),
    });
    items.push(QuickActionItem {
        id: format!("query-run-new-{query}"),
        section: QuickActionSection::Suggested,
        title: format!("Run {title} in new pane"),
        subtitle: "Split and execute in a fresh pane".to_string(),
        badge,
        target: ActionTarget::NewPane,
        command: QuickActionCommand::Shell(query.to_string()),
    });

    dedupe(items)
}

pub fn detected_command_entry(context: &PanelContext) -> Option<(String, String, String)> {
    let command = context.foreground_command.as_ref()?.trim();
    let process = context.foreground_process.as_deref()?;

    if command.is_empty() || process.eq_ignore_ascii_case(&context.shell) {
        return None;
    }

    let title = format!("Run {}", prettify_command(process));
    let category = infer_badge(command, Some(context)).unwrap_or_else(|| "COMMAND".to_string());
    Some((title, command.to_string(), category))
}

pub fn infer_badge(query: &str, context: Option<&PanelContext>) -> Option<String> {
    let token = util::first_command_token(query)?;
    let token = util::shell_name(&token).to_ascii_lowercase();

    let badge = match token.as_str() {
        "nvim" | "vim" | "hx" | "helix" | "nano" | "micro" | "emacs" | "less" => "EDITOR",
        "htop" | "btop" | "top" | "watch" | "iftop" | "nvtop" => "MONITOR",
        "ssh" | "mosh" => "SSH",
        "docker" | "podman" | "distrobox" | "toolbox" => "CONTAINER",
        "git" | "lazygit" => "GIT",
        "cargo" | "make" | "just" | "npm" | "pnpm" | "yarn" | "go" => "BUILD",
        _ => match context.map(|value| &value.mode) {
            Some(PanelMode::Editor) => "EDITOR",
            Some(PanelMode::Monitor) => "MONITOR",
            Some(PanelMode::Container) => "CONTAINER",
            Some(PanelMode::Remote) => "REMOTE",
            _ => "COMMAND",
        },
    };

    Some(badge.to_string())
}

fn built_in_actions() -> Vec<QuickActionItem> {
    let mut items = Vec::new();

    items.push(QuickActionItem {
        id: "internal-info".to_string(),
        section: QuickActionSection::Layout,
        title: "Show system info banner".to_string(),
        subtitle: "Print the ASCII banner and current system summary in this pane.".to_string(),
        badge: Some("INFO".to_string()),
        target: ActionTarget::CurrentPane,
        command: QuickActionCommand::Internal(InternalAction::ShowInfo),
    });

    items.push(QuickActionItem {
        id: "internal-zoom".to_string(),
        section: QuickActionSection::Layout,
        title: "Toggle pane fullscreen".to_string(),
        subtitle: "Zoom the focused pane in place without fullscreening the window.".to_string(),
        badge: Some("LAYOUT".to_string()),
        target: ActionTarget::CurrentPane,
        command: QuickActionCommand::Internal(InternalAction::TogglePaneZoom),
    });

    if let Some(editor) = first_available_command(&["nvim .", "vim .", "hx ."]) {
        items.push(QuickActionItem {
            id: "editor-here".to_string(),
            section: QuickActionSection::Commands,
            title: format!("{} here", prettify_command(&editor)),
            subtitle: "Open an editor in the active working directory".to_string(),
            badge: Some("EDITOR".to_string()),
            target: ActionTarget::CurrentPane,
            command: QuickActionCommand::Shell(editor),
        });
    }

    if let Some(monitor) = first_available_command(&["htop", "btop", "top"]) {
        items.push(QuickActionItem {
            id: "monitor-here".to_string(),
            section: QuickActionSection::Commands,
            title: format!("{} here", prettify_command(&monitor)),
            subtitle: "Run a system monitor in the current pane".to_string(),
            badge: Some("MONITOR".to_string()),
            target: ActionTarget::CurrentPane,
            command: QuickActionCommand::Shell(monitor.clone()),
        });
        items.push(QuickActionItem {
            id: "monitor-new".to_string(),
            section: QuickActionSection::Commands,
            title: format!("{} in new pane", prettify_command(&monitor)),
            subtitle: "Open a fresh monitoring pane".to_string(),
            badge: Some("MONITOR".to_string()),
            target: ActionTarget::NewPane,
            command: QuickActionCommand::Shell(monitor),
        });
    }

    if util::command_exists("git") {
        items.push(QuickActionItem {
            id: "git-status".to_string(),
            section: QuickActionSection::Commands,
            title: "git status".to_string(),
            subtitle: "Inspect the current repository state".to_string(),
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
            id: format!("ssh-{host}"),
            section: QuickActionSection::Remote,
            title: format!("SSH {host}"),
            subtitle: "Launch a remote shell in a new pane".to_string(),
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
            id: format!("{runtime}-{name}"),
            section: QuickActionSection::Remote,
            title: format!("{runtime} shell {name}"),
            subtitle: format!("Attach /bin/sh in {name}"),
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
                id: format!("dir-{}", entry.path),
                section: QuickActionSection::Workspace,
                title: format!("Open {}", util::compact_label(&entry.path)),
                subtitle: "Jump to a recent directory".to_string(),
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
                id: format!("project-{}", entry.path),
                section: QuickActionSection::Workspace,
                title: format!("Project {}", util::compact_label(&entry.path)),
                subtitle: "Open a recent Git project".to_string(),
                badge: Some("PROJECT".to_string()),
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
        .map(|entry| QuickActionItem {
            id: format!("action-{}", entry.command),
            section: QuickActionSection::Commands,
            title: format!("Repeat {}", entry.title),
            subtitle: entry.command.clone(),
            badge: infer_badge(&entry.command, None).or(Some("RECENT".to_string())),
            target: ActionTarget::CurrentPane,
            command: QuickActionCommand::Shell(entry.command.clone()),
        })
        .collect()
}

fn history_command_actions(history: &HistoryStore) -> Vec<QuickActionItem> {
    history
        .recent_commands
        .iter()
        .take(8)
        .map(|entry| QuickActionItem {
            id: format!("command-{}", entry.command),
            section: QuickActionSection::Commands,
            title: entry.title.clone(),
            subtitle: entry.command.clone(),
            badge: Some(entry.category.clone()),
            target: ActionTarget::CurrentPane,
            command: QuickActionCommand::Shell(entry.command.clone()),
        })
        .collect()
}

fn history_connection_actions(history: &HistoryStore) -> Vec<QuickActionItem> {
    history
        .recent_connections
        .iter()
        .take(4)
        .map(|entry| QuickActionItem {
            id: format!("connection-{}", entry.command),
            section: QuickActionSection::Remote,
            title: entry.label.clone(),
            subtitle: entry.command.clone(),
            badge: Some("CONNECTION".to_string()),
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
        if seen.insert(item.id.clone()) {
            deduped.push(item);
        }
    }

    deduped
}

fn internal_query_actions(query: &str) -> Vec<QuickActionItem> {
    let Some(command) = normalize_internal_query(query) else {
        return Vec::new();
    };

    if matches!(command, "info" | "banner" | "fetch") {
        return vec![QuickActionItem {
            id: "query-internal-info".to_string(),
            section: QuickActionSection::Suggested,
            title: "Show system info banner".to_string(),
            subtitle: "Print the current ASCII system info in the active pane.".to_string(),
            badge: Some("INFO".to_string()),
            target: ActionTarget::CurrentPane,
            command: QuickActionCommand::Internal(InternalAction::ShowInfo),
        }];
    }

    if matches!(command, "zoom" | "fullscreen" | "focus") {
        return vec![QuickActionItem {
            id: "query-internal-zoom".to_string(),
            section: QuickActionSection::Suggested,
            title: "Toggle pane fullscreen".to_string(),
            subtitle: "Zoom or restore the focused pane.".to_string(),
            badge: Some("LAYOUT".to_string()),
            target: ActionTarget::CurrentPane,
            command: QuickActionCommand::Internal(InternalAction::TogglePaneZoom),
        }];
    }

    if let Some(direction) = parse_directional_command(command, &["swap", "move"]) {
        return vec![QuickActionItem {
            id: format!("query-internal-swap-{direction:?}"),
            section: QuickActionSection::Suggested,
            title: format!("Swap pane {}", direction_label(direction)),
            subtitle: "Exchange the active pane with its closest neighbor.".to_string(),
            badge: Some("LAYOUT".to_string()),
            target: ActionTarget::CurrentPane,
            command: QuickActionCommand::Internal(InternalAction::SwapPane(direction)),
        }];
    }

    let theme_name = command
        .strip_prefix("theme ")
        .or_else(|| command.strip_prefix("palette "));
    if let Some(theme_name) = theme_name.map(str::trim) {
        if matches!(theme_name, "default" | "reset" | "base") {
            return vec![QuickActionItem {
                id: "query-theme-reset".to_string(),
                section: QuickActionSection::Suggested,
                title: "Reset pane palette".to_string(),
                subtitle: "Restore the pane to the global terminal palette.".to_string(),
                badge: Some("THEME".to_string()),
                target: ActionTarget::CurrentPane,
                command: QuickActionCommand::Internal(InternalAction::SetPanePalette(None)),
            }];
        }

        if let Some(preset) = PanePalettePreset::from_name(theme_name) {
            return vec![QuickActionItem {
                id: format!("query-theme-{}", preset.slug()),
                section: QuickActionSection::Suggested,
                title: format!("Set pane palette to {}", preset.label()),
                subtitle: "Apply a per-pane color preset.".to_string(),
                badge: Some("THEME".to_string()),
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
        id: "theme-reset".to_string(),
        section: QuickActionSection::Theme,
        title: "Reset pane palette".to_string(),
        subtitle: "Return the active pane to the default palette.".to_string(),
        badge: Some("THEME".to_string()),
        target: ActionTarget::CurrentPane,
        command: QuickActionCommand::Internal(InternalAction::SetPanePalette(None)),
    });

    for preset in PanePalettePreset::ALL {
        items.push(QuickActionItem {
            id: format!("theme-{}", preset.slug()),
            section: QuickActionSection::Theme,
            title: format!("Pane palette: {}", preset.label()),
            subtitle: "Apply a per-pane palette preset.".to_string(),
            badge: Some("THEME".to_string()),
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
        "left" | "west" => Some(Direction::Left),
        "right" | "east" => Some(Direction::Right),
        "up" | "north" => Some(Direction::Up),
        "down" | "south" => Some(Direction::Down),
        _ => None,
    }
}

fn direction_label(direction: Direction) -> &'static str {
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
