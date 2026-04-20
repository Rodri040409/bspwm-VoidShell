use crate::util;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::fd::RawFd;
#[cfg(not(unix))]
type RawFd = i32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PanelMode {
    Shell,
    Editor,
    Monitor,
    Remote,
    Container,
    Exited,
}

impl Default for PanelMode {
    fn default() -> Self {
        Self::Shell
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PanelContext {
    pub cwd: Option<PathBuf>,
    pub hostname: String,
    pub shell: String,
    pub shell_alive: bool,
    pub foreground_process: Option<String>,
    pub foreground_command: Option<String>,
    pub in_ssh: bool,
    pub ssh_target: Option<String>,
    pub container_hint: Option<String>,
    pub git_branch: Option<String>,
    pub lab_hint: Option<String>,
    pub mode: PanelMode,
}

impl PanelContext {
    pub fn header_title(&self) -> String {
        self.cwd
            .as_ref()
            .map(|path| util::display_path(path))
            .unwrap_or_else(|| "shell".to_string())
    }

    pub fn header_subtitle(&self) -> String {
        let mut parts = vec![self.hostname.clone(), self.shell.clone()];
        if let Some(process) = &self.foreground_process {
            if process != &self.shell {
                parts.push(process.clone());
            }
        }
        parts.join(" · ")
    }

    pub fn badges(&self) -> Vec<(String, bool)> {
        let mut badges = Vec::new();

        if self.in_ssh {
            badges.push((
                self.ssh_target
                    .as_ref()
                    .map(|target| format!("SSH {target}"))
                    .unwrap_or_else(|| "REMOTE".to_string()),
                true,
            ));
        }

        if let Some(container) = &self.container_hint {
            badges.push((container.to_ascii_uppercase(), false));
        }

        match self.mode {
            PanelMode::Editor => badges.push(("EDITOR".to_string(), false)),
            PanelMode::Monitor => badges.push(("MONITOR".to_string(), false)),
            PanelMode::Container => badges.push(("CONTAINER".to_string(), false)),
            PanelMode::Exited => badges.push(("EXITED".to_string(), false)),
            PanelMode::Shell | PanelMode::Remote => {}
        }

        if let Some(lab) = &self.lab_hint {
            badges.push((lab.to_ascii_uppercase(), false));
        }

        badges
    }

    pub fn history_context(&self) -> Vec<String> {
        self.badges().into_iter().map(|(label, _)| label).collect()
    }
}

pub fn detect_panel_context(
    shell_pid: Option<i32>,
    pty_fd: Option<RawFd>,
    shell_path: &str,
) -> PanelContext {
    #[cfg(not(target_os = "linux"))]
    {
        return detect_panel_context_portable(shell_pid, shell_path);
    }

    #[cfg(target_os = "linux")]
    {
        let hostname = util::hostname();
        let shell = util::shell_name(shell_path);
        let Some(shell_pid) = shell_pid else {
            return PanelContext {
                hostname,
                shell,
                shell_alive: false,
                mode: PanelMode::Exited,
                ..PanelContext::default()
            };
        };

        if !Path::new(&format!("/proc/{shell_pid}")).exists() {
            return PanelContext {
                hostname,
                shell,
                shell_alive: false,
                mode: PanelMode::Exited,
                ..PanelContext::default()
            };
        }

        let foreground_pid = pty_fd.and_then(find_foreground_pid);
        let target_pid = foreground_pid.unwrap_or(shell_pid);
        let env = read_environ(target_pid);
        let cwd = fs::read_link(format!("/proc/{target_pid}/cwd")).ok();
        let foreground_process = read_comm(target_pid);
        let foreground_command = read_cmdline(target_pid);
        let ssh_target = if foreground_process.as_deref() == Some("ssh") {
            foreground_command
                .as_deref()
                .and_then(parse_ssh_target_from_command)
        } else {
            env.get("SSH_CONNECTION").map(|_| hostname.clone())
        };
        let in_ssh = ssh_target.is_some() || env.contains_key("SSH_CONNECTION");

        let container_hint = if env.get("DISTROBOX_ENTER_PATH").is_some()
            || env.get("DISTROBOX_HOST_HOME").is_some()
        {
            Some("distrobox".to_string())
        } else if env.get("TOOLBOX_PATH").is_some() || env.get("TOOLBOX_NAME").is_some() {
            Some("toolbox".to_string())
        } else if env
            .get("container")
            .map(|value| !value.is_empty())
            .unwrap_or(false)
        {
            Some(
                env.get("container")
                    .cloned()
                    .unwrap_or_else(|| "container".to_string()),
            )
        } else {
            None
        };

        let mode = detect_mode(
            foreground_process.as_deref(),
            foreground_command.as_deref(),
            in_ssh,
            container_hint.is_some(),
        );

        let lab_hint = ssh_target
            .as_deref()
            .and_then(|target| detect_lab_context(target))
            .or_else(|| {
                cwd.as_deref()
                    .and_then(|path| detect_lab_context(&path.display().to_string()))
            });

        PanelContext {
            cwd,
            hostname,
            shell,
            shell_alive: true,
            foreground_process,
            foreground_command,
            in_ssh,
            ssh_target,
            container_hint,
            git_branch: None,
            lab_hint,
            mode,
        }
    }
}

pub fn detect_git_branch(path: &Path) -> Option<String> {
    util::command_output(
        "git",
        &[
            "-C",
            &path.display().to_string(),
            "rev-parse",
            "--abbrev-ref",
            "HEAD",
        ],
    )
    .filter(|branch| branch != "HEAD")
}

fn detect_mode(
    process: Option<&str>,
    command: Option<&str>,
    in_ssh: bool,
    in_container: bool,
) -> PanelMode {
    if in_ssh {
        return PanelMode::Remote;
    }

    let normalized = process.unwrap_or_default();
    match normalized {
        "nvim" | "vim" | "nano" | "hx" | "helix" | "kak" | "micro" | "emacs" | "less" => {
            PanelMode::Editor
        }
        "htop" | "btop" | "top" | "watch" | "iftop" | "nvtop" => PanelMode::Monitor,
        "docker" | "podman" => PanelMode::Container,
        _ if in_container => PanelMode::Container,
        _ if command.unwrap_or_default().contains("docker exec") => PanelMode::Container,
        _ => PanelMode::Shell,
    }
}

fn detect_lab_context(value: &str) -> Option<String> {
    let lowered = value.to_ascii_lowercase();
    if lowered.contains("hackthebox") || lowered.contains(".htb") || lowered.contains("/htb") {
        return Some("htb".to_string());
    }
    if lowered.contains("tryhackme") || lowered.contains("thm") {
        return Some("tryhackme".to_string());
    }
    if lowered.contains("vulnlab") {
        return Some("vulnlab".to_string());
    }
    None
}

fn find_foreground_pid(pty_fd: RawFd) -> Option<i32> {
    #[cfg(unix)]
    let process_group = unsafe { libc::tcgetpgrp(pty_fd) };
    #[cfg(not(unix))]
    let process_group = -1;

    if process_group <= 0 {
        return None;
    }

    let mut leader = None;
    for entry in fs::read_dir("/proc").ok()? {
        let Ok(entry) = entry else {
            continue;
        };
        let name = entry.file_name();
        let Ok(pid) = name.to_string_lossy().parse::<i32>() else {
            continue;
        };
        let Some(stat) = read_stat(pid) else {
            continue;
        };
        if stat.process_group == process_group {
            if pid == process_group {
                return Some(pid);
            }
            leader = Some(leader.map_or(pid, |current: i32| current.max(pid)));
        }
    }

    leader
}

fn read_stat(pid: i32) -> Option<ProcStat> {
    let path = format!("/proc/{pid}/stat");
    let content = fs::read_to_string(path).ok()?;
    let after_name = content.rsplit_once(") ")?.1;
    let fields: Vec<&str> = after_name.split_whitespace().collect();
    let process_group = fields.get(2)?.parse::<i32>().ok()?;
    Some(ProcStat { process_group })
}

fn read_comm(pid: i32) -> Option<String> {
    fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn read_cmdline(pid: i32) -> Option<String> {
    let raw = fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    let args: Vec<String> = raw
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .map(|entry| String::from_utf8_lossy(entry).into_owned())
        .collect();
    (!args.is_empty()).then(|| args.join(" "))
}

fn read_environ(pid: i32) -> BTreeMap<String, String> {
    fs::read(format!("/proc/{pid}/environ"))
        .ok()
        .map(|raw| {
            raw.split(|byte| *byte == 0)
                .filter_map(|entry| {
                    let string = String::from_utf8_lossy(entry);
                    string
                        .split_once('=')
                        .map(|(key, value)| (key.to_string(), value.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_ssh_target_from_command(command: &str) -> Option<String> {
    command
        .split_whitespace()
        .rev()
        .find(|segment| !segment.starts_with('-'))
        .map(|segment| segment.to_string())
}

#[derive(Debug, Clone, Copy)]
struct ProcStat {
    process_group: i32,
}

#[cfg(not(target_os = "linux"))]
fn detect_panel_context_portable(shell_pid: Option<i32>, shell_path: &str) -> PanelContext {
    let hostname = util::hostname();
    let shell = util::shell_name(shell_path);

    PanelContext {
        cwd: std::env::current_dir().ok().or_else(util::home_dir),
        hostname,
        shell,
        shell_alive: shell_pid.is_some(),
        foreground_process: None,
        foreground_command: None,
        in_ssh: false,
        ssh_target: None,
        container_hint: None,
        git_branch: None,
        lab_hint: None,
        mode: if shell_pid.is_some() {
            PanelMode::Shell
        } else {
            PanelMode::Exited
        },
    }
}
