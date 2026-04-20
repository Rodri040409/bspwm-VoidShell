use crate::constants;
use crate::util;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_ITEMS: usize = 40;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionRecord {
    pub pane_id: u64,
    pub shell: String,
    pub cwd: Option<String>,
    pub context: Vec<String>,
    pub started_at: u64,
    pub ended_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecentPath {
    pub path: String,
    pub last_used: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecentAction {
    pub title: String,
    pub command: String,
    pub last_used: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecentCommand {
    pub title: String,
    pub command: String,
    pub category: String,
    pub last_used: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecentConnection {
    pub label: String,
    pub command: String,
    pub last_used: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct HistoryStore {
    pub recent_sessions: Vec<SessionRecord>,
    pub recent_directories: Vec<RecentPath>,
    pub recent_projects: Vec<RecentPath>,
    pub recent_quick_actions: Vec<RecentAction>,
    pub recent_commands: Vec<RecentCommand>,
    pub recent_connections: Vec<RecentConnection>,
    pub recent_panel_ids: Vec<u64>,
}

#[derive(Debug, Clone)]
pub struct HistoryManager {
    path: PathBuf,
}

impl HistoryManager {
    pub fn new() -> Self {
        let path = ProjectDirs::from(
            constants::CONFIG_QUALIFIER,
            constants::CONFIG_ORGANIZATION,
            constants::CONFIG_APPLICATION,
        )
        .and_then(|dirs| dirs.state_dir().map(|path| path.join("history.json")))
        .unwrap_or_else(|| PathBuf::from(".termvoid-history.json"));

        Self { path }
    }

    pub fn load_or_default(&self) -> HistoryStore {
        self.load().unwrap_or_default()
    }

    pub fn load(&self) -> Option<HistoryStore> {
        let content = fs::read_to_string(&self.path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(&self, history: &HistoryStore) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }

        let serialized =
            serde_json::to_string_pretty(history).map_err(|error| error.to_string())?;
        fs::write(&self.path, serialized).map_err(|error| error.to_string())
    }
}

impl HistoryStore {
    pub fn begin_session(&mut self, pane_id: u64, shell: &str, cwd: Option<&str>) {
        self.recent_panel_ids.retain(|entry| *entry != pane_id);
        self.recent_panel_ids.insert(0, pane_id);
        self.recent_panel_ids.truncate(MAX_ITEMS);

        self.recent_sessions.insert(
            0,
            SessionRecord {
                pane_id,
                shell: shell.to_string(),
                cwd: cwd.map(ToOwned::to_owned),
                context: Vec::new(),
                started_at: util::now_epoch_seconds(),
                ended_at: None,
            },
        );
        self.recent_sessions.truncate(MAX_ITEMS);
    }

    pub fn end_session(&mut self, pane_id: u64, cwd: Option<&str>, context: &[String]) {
        if let Some(session) = self
            .recent_sessions
            .iter_mut()
            .find(|record| record.pane_id == pane_id && record.ended_at.is_none())
        {
            session.ended_at = Some(util::now_epoch_seconds());
            session.cwd = cwd.map(ToOwned::to_owned);
            session.context = context.to_vec();
        }
    }

    pub fn note_directory(&mut self, path: &Path) {
        Self::touch_path(&mut self.recent_directories, path);
    }

    pub fn note_project(&mut self, path: &Path) {
        Self::touch_path(&mut self.recent_projects, path);
    }

    pub fn note_action(&mut self, title: &str, command: &str) {
        let now = util::now_epoch_seconds();
        self.recent_quick_actions
            .retain(|item| item.command != command);
        self.recent_quick_actions.insert(
            0,
            RecentAction {
                title: title.to_string(),
                command: command.to_string(),
                last_used: now,
            },
        );
        self.recent_quick_actions.truncate(MAX_ITEMS);
    }

    pub fn note_command(&mut self, title: &str, command: &str, category: &str) {
        let now = util::now_epoch_seconds();
        self.recent_commands.retain(|item| item.command != command);
        self.recent_commands.insert(
            0,
            RecentCommand {
                title: title.to_string(),
                command: command.to_string(),
                category: category.to_string(),
                last_used: now,
            },
        );
        self.recent_commands.truncate(MAX_ITEMS);
    }

    pub fn note_connection(&mut self, label: &str, command: &str) {
        let now = util::now_epoch_seconds();
        self.recent_connections
            .retain(|item| item.command != command && item.label != label);
        self.recent_connections.insert(
            0,
            RecentConnection {
                label: label.to_string(),
                command: command.to_string(),
                last_used: now,
            },
        );
        self.recent_connections.truncate(MAX_ITEMS);
    }

    fn touch_path(items: &mut Vec<RecentPath>, path: &Path) {
        let path = path.display().to_string();
        let now = util::now_epoch_seconds();
        items.retain(|item| item.path != path);
        items.insert(
            0,
            RecentPath {
                path,
                last_used: now,
            },
        );
        items.truncate(MAX_ITEMS);
    }
}
