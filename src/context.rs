use crate::util;
#[cfg(not(windows))]
use gtk::gio;
#[cfg(not(windows))]
use gtk::gio::prelude::SettingsExt;
use std::cell::RefCell;
use std::collections::BTreeMap;
#[cfg(unix)]
use std::ffi::CStr;
use std::fs;
use std::net::{IpAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::fd::RawFd;
#[cfg(not(unix))]
type RawFd = i32;

const VPN_CACHE_MAX_AGE_SECONDS: u64 = 2;
const EXPRESSVPNCTL_CANDIDATES: [&str; 2] = ["/opt/expressvpn/bin/expressvpnctl", "expressvpnctl"];
const MAX_PYTHON_PROJECT_ANCESTORS: usize = 8;
const PYTHON_MARKER_FILES: [&str; 6] = [
    "pyproject.toml",
    "requirements.txt",
    "setup.py",
    "setup.cfg",
    "Pipfile",
    "manage.py",
];
const COMMON_PYTHON_SOURCE_DIRS: [&str; 5] = ["src", "app", "scripts", "tests", "tools"];

thread_local! {
    static VPN_CONTEXT_CACHE: RefCell<Option<CachedVpnContext>> = const { RefCell::new(None) };
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NetworkRoute {
    #[default]
    Direct,
    Proxy,
    Vpn,
    VpnProxy,
    Offline,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VpnContext {
    pub provider: Option<String>,
    pub region: Option<String>,
    pub smart_region: Option<String>,
    pub protocol: Option<String>,
    pub state: Option<String>,
    pub interface_name: Option<String>,
    pub connection_name: Option<String>,
    pub assigned_ip: Option<String>,
}

impl VpnContext {
    pub fn is_activeish(&self) -> bool {
        matches!(
            self.state
                .as_deref()
                .map(str::trim)
                .map(str::to_ascii_lowercase)
                .as_deref(),
            Some("connected")
                | Some("activated")
                | Some("connected (externally)")
                | Some("connecting")
                | Some("reconnecting")
                | Some("interrupted")
                | Some("disconnectingtoreconnect")
        ) || self.interface_name.is_some()
            || self.assigned_ip.is_some()
    }

    pub fn is_connected(&self) -> bool {
        matches!(
            self.state
                .as_deref()
                .map(str::trim)
                .map(str::to_ascii_lowercase)
                .as_deref(),
            Some("connected") | Some("activated") | Some("connected (externally)")
        )
    }

    pub fn provider_label(&self) -> Option<String> {
        self.provider
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
    }

    pub fn state_label(&self) -> Option<String> {
        let state = self.state.as_deref()?.trim();
        if state.is_empty() {
            return None;
        }

        Some(match state.to_ascii_lowercase().as_str() {
            "connected" => "Conectado".to_string(),
            "activated" => "Conectado".to_string(),
            "connected (externally)" => "Conectado".to_string(),
            "connecting" => "Conectando".to_string(),
            "reconnecting" => "Reconectando".to_string(),
            "interrupted" => "Interrumpido".to_string(),
            "disconnecting" => "Desconectando".to_string(),
            "disconnectingtoreconnect" => "Reiniciando".to_string(),
            "disconnected" => "Desconectado".to_string(),
            _ => state.to_string(),
        })
    }

    pub fn protocol_label(&self) -> Option<String> {
        let protocol = self.protocol.as_deref()?.trim();
        if protocol.is_empty() {
            return None;
        }

        Some(match protocol.to_ascii_lowercase().as_str() {
            "auto" => "Auto".to_string(),
            "lightwayudp" => "Lightway UDP".to_string(),
            "lightwaytcp" => "Lightway TCP".to_string(),
            "openvpnudp" => "OpenVPN UDP".to_string(),
            "openvpntcp" => "OpenVPN TCP".to_string(),
            "wireguard" => "WireGuard".to_string(),
            _ => humanize_slug(protocol),
        })
    }

    pub fn region_label(&self) -> Option<String> {
        let region = self
            .region
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        match region {
            Some("smart") => self.smart_region_label(),
            Some(region) => Some(humanize_slug(region)),
            None => self.smart_region_label(),
        }
    }

    pub fn smart_region_label(&self) -> Option<String> {
        self.smart_region
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(humanize_slug)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NetworkContext {
    pub route: NetworkRoute,
    pub local_ip: Option<String>,
    pub public_ip: Option<String>,
    pub vpn_ip: Option<String>,
    pub proxy_ip: Option<String>,
    pub proxy_target: Option<String>,
    pub vpn: Option<VpnContext>,
}

impl NetworkContext {
    pub fn detail_line(&self) -> String {
        let mut parts = Vec::new();

        push_network_field(&mut parts, "red", self.route_label());

        match self.route {
            NetworkRoute::Direct | NetworkRoute::Proxy | NetworkRoute::Offline => {}
            NetworkRoute::Vpn | NetworkRoute::VpnProxy => {
                if let Some(provider) = self.vpn.as_ref().and_then(VpnContext::provider_label) {
                    push_network_field(&mut parts, "servicio", provider);
                }

                if let Some(vpn) = &self.vpn {
                    if !vpn.is_connected()
                        && let Some(state) = vpn.state_label()
                    {
                        push_network_field(&mut parts, "estado", state);
                    }

                    if let Some(region) = vpn.region_label() {
                        push_network_field(&mut parts, "region", region);
                    }

                    if let Some(protocol) = vpn.protocol_label() {
                        push_network_field(&mut parts, "protocolo", protocol);
                    }
                }
            }
        }

        if let Some(local_ip) = &self.local_ip {
            push_network_field(&mut parts, "ip local", local_ip);
        }

        if let Some(public_ip) = &self.public_ip {
            push_network_field(&mut parts, "ip publica", public_ip);
        }

        if matches!(self.route, NetworkRoute::Vpn | NetworkRoute::VpnProxy)
            && let Some(vpn_ip) = &self.vpn_ip
        {
            push_network_field(&mut parts, "ip vpn", vpn_ip);
        }

        if matches!(self.route, NetworkRoute::Proxy | NetworkRoute::VpnProxy) {
            let proxy_value = self.proxy_ip.as_ref().or(self.proxy_target.as_ref());
            if let Some(proxy_value) = proxy_value {
                push_network_field(&mut parts, "proxy", proxy_value);
            }
        }

        parts.join(" · ")
    }

    pub fn has_any_signal(&self) -> bool {
        self.local_ip.is_some()
            || self.public_ip.is_some()
            || self.vpn_ip.is_some()
            || self.proxy_ip.is_some()
            || self.proxy_target.is_some()
            || self.vpn.as_ref().is_some_and(VpnContext::is_activeish)
            || !matches!(self.route, NetworkRoute::Offline)
    }

    pub fn active_vpn(&self) -> Option<&VpnContext> {
        self.vpn.as_ref().filter(|vpn| vpn.is_activeish())
    }

    fn route_label(&self) -> &'static str {
        match self.route {
            NetworkRoute::Direct => "directa",
            NetworkRoute::Proxy => "proxy",
            NetworkRoute::Vpn => "vpn",
            NetworkRoute::VpnProxy => "vpn + proxy",
            NetworkRoute::Offline => "sin red",
        }
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
    pub python_project: Option<PythonProjectContext>,
    pub active_python_venv: Option<PathBuf>,
    pub network: NetworkContext,
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

    fn stable_badges(&self) -> Vec<(String, bool)> {
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

    pub fn badges(&self) -> Vec<(String, bool)> {
        let mut badges = self.stable_badges();

        if let Some(project) = &self.python_project {
            let project_active = self.active_python_venv.as_ref() == Some(&project.venv_path);
            let status = if project_active { "ACTIVO" } else { "INACTIVO" };
            badges.push((format!("PY {} {status}", project.venv_name), project_active));
        }

        if let Some(venv) = &self.active_python_venv {
            let project_matches_venv = self
                .python_project
                .as_ref()
                .is_some_and(|project| project.venv_path == *venv);
            if !project_matches_venv {
                badges.push((venv_badge_label(venv), true));
            }
        }

        if let Some(vpn) = self.network.active_vpn() {
            let vpn_label = vpn
                .provider_label()
                .map(|provider| format!("VPN {provider}"))
                .unwrap_or_else(|| "VPN".to_string());
            badges.push((vpn_label, true));

            if let Some(region) = vpn.region_label() {
                badges.push((region, false));
            }
        }

        badges
    }

    pub fn history_context(&self) -> Vec<String> {
        self.stable_badges()
            .into_iter()
            .map(|(label, _)| label)
            .collect()
    }
}

pub fn detect_panel_context(
    shell_pid: Option<i32>,
    pty_fd: Option<RawFd>,
    shell_path: &str,
) -> PanelContext {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pty_fd;
        return detect_panel_context_portable(shell_pid, shell_path);
    }

    #[cfg(target_os = "linux")]
    {
        let hostname = util::hostname();
        let shell = util::shell_name(shell_path);
        let network = detect_network_context(&BTreeMap::new());
        let Some(shell_pid) = shell_pid else {
            return PanelContext {
                hostname,
                shell,
                shell_alive: false,
                network,
                mode: PanelMode::Exited,
                ..PanelContext::default()
            };
        };

        if !Path::new(&format!("/proc/{shell_pid}")).exists() {
            return PanelContext {
                hostname,
                shell,
                shell_alive: false,
                network,
                mode: PanelMode::Exited,
                ..PanelContext::default()
            };
        }

        let foreground_pid = pty_fd.and_then(find_foreground_pid);
        let target_pid = foreground_pid.unwrap_or(shell_pid);
        let env = read_environ(target_pid);
        let cwd = fs::read_link(format!("/proc/{target_pid}/cwd")).ok();
        let active_python_venv = detect_active_python_venv(&env);
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
        let network = detect_network_context(&env);

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
            python_project: None,
            active_python_venv,
            network,
            mode,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonProjectContext {
    pub project_root: PathBuf,
    pub venv_path: PathBuf,
    pub venv_name: String,
}

pub fn detect_python_project(path: &Path) -> Option<PythonProjectContext> {
    let home = util::home_dir();

    path.ancestors()
        .take(MAX_PYTHON_PROJECT_ANCESTORS + 1)
        .take_while(|ancestor| {
            home.as_ref()
                .map(|home| *ancestor == home.as_path() && *ancestor != path)
                .map(|is_home_boundary| !is_home_boundary)
                .unwrap_or(true)
        })
        .find_map(detect_python_project_in_directory)
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

fn detect_python_project_in_directory(directory: &Path) -> Option<PythonProjectContext> {
    if !directory.is_dir() {
        return None;
    }

    let venv_path = detect_python_venv_root(directory)?;
    let normalized_directory = normalize_existing_path(directory);
    let normalized_venv_path = normalize_existing_path(&venv_path);
    if normalized_directory == normalized_venv_path && !directory_has_python_signal(directory) {
        return None;
    }

    Some(PythonProjectContext {
        project_root: directory.to_path_buf(),
        venv_name: python_venv_name(&normalized_venv_path),
        venv_path: normalized_venv_path,
    })
}

fn directory_has_python_signal(directory: &Path) -> bool {
    if PYTHON_MARKER_FILES
        .iter()
        .any(|name| directory.join(name).is_file())
    {
        return true;
    }

    let Ok(entries) = fs::read_dir(directory) else {
        return false;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("py"))
        {
            return true;
        }

        if !file_type.is_dir() {
            continue;
        }

        let name = entry.file_name();
        let name = name.to_string_lossy();
        if is_python_venv_root(&path)
            || matches!(name.as_ref(), ".venv" | "venv" | "env")
            || !COMMON_PYTHON_SOURCE_DIRS.contains(&name.as_ref())
        {
            continue;
        }

        if directory_contains_python_file(&path) {
            return true;
        }
    }

    false
}

fn directory_contains_python_file(directory: &Path) -> bool {
    let Ok(entries) = fs::read_dir(directory) else {
        return false;
    };

    entries.flatten().any(|entry| {
        let path = entry.path();
        entry
            .file_type()
            .map(|kind| kind.is_file())
            .unwrap_or(false)
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("py"))
    })
}

fn detect_python_venv_root(directory: &Path) -> Option<PathBuf> {
    if is_python_venv_root(directory) {
        return Some(directory.to_path_buf());
    }

    for preferred in [".venv", "venv", "env"] {
        let candidate = directory.join(preferred);
        if is_python_venv_root(&candidate) {
            return Some(candidate);
        }
    }

    let mut fallback = None;
    let Ok(entries) = fs::read_dir(directory) else {
        return None;
    };

    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }

        let path = entry.path();
        if !is_python_venv_root(&path) {
            continue;
        }

        let name = entry.file_name();
        let name = name.to_string_lossy();
        if matches!(name.as_ref(), ".venv" | "venv" | "env") {
            return Some(path);
        }

        if fallback.is_none() {
            fallback = Some(path);
        }
    }

    fallback
}

fn is_python_venv_root(path: &Path) -> bool {
    path.is_dir()
        && path.join("pyvenv.cfg").is_file()
        && (path.join("bin/activate").is_file()
            || path.join("bin/activate.fish").is_file()
            || path.join("Scripts/activate").is_file()
            || path.join("Scripts/activate.bat").is_file()
            || path.join("Scripts/Activate.ps1").is_file())
}

fn python_venv_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "venv".to_string())
}

fn detect_active_python_venv(env: &BTreeMap<String, String>) -> Option<PathBuf> {
    env.get("VIRTUAL_ENV")
        .map(PathBuf::from)
        .map(|path| normalize_existing_path(&path))
}

fn normalize_existing_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn venv_badge_label(path: &Path) -> String {
    format!("VENV {} ACTIVO", python_venv_name(path))
}

fn find_foreground_pid(pty_fd: RawFd) -> Option<i32> {
    #[cfg(unix)]
    let process_group = unsafe { libc::tcgetpgrp(pty_fd) };
    #[cfg(not(unix))]
    let process_group = {
        let _ = pty_fd;
        -1
    };

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

fn detect_network_context(env: &BTreeMap<String, String>) -> NetworkContext {
    let interfaces = interface_ipv4_addresses();
    let default_interface = default_route_interface();
    let vpn = detect_vpn_context(&interfaces, default_interface.as_deref());
    let local_ip = primary_local_interface_ip(&interfaces, default_interface.as_deref());
    let vpn_ip = vpn
        .as_ref()
        .and_then(|details| details.assigned_ip.clone())
        .or_else(|| primary_vpn_interface_ip(&interfaces, default_interface.as_deref()));
    let proxy = detect_proxy_details(env);
    let has_vpn = vpn.as_ref().is_some_and(VpnContext::is_activeish)
        || vpn_ip.is_some()
        || default_interface
            .as_deref()
            .is_some_and(is_vpn_interface_name);

    let route = match (
        has_vpn,
        proxy.is_some(),
        local_ip.is_some() || vpn_ip.is_some(),
    ) {
        (true, true, _) => NetworkRoute::VpnProxy,
        (true, false, _) => NetworkRoute::Vpn,
        (false, true, _) => NetworkRoute::Proxy,
        (false, false, true) => NetworkRoute::Direct,
        (false, false, false) => NetworkRoute::Offline,
    };

    NetworkContext {
        route,
        local_ip,
        public_ip: None,
        vpn_ip,
        proxy_ip: proxy.as_ref().and_then(|details| details.ip.clone()),
        proxy_target: proxy.map(|details| details.display),
        vpn,
    }
}

fn detect_proxy_details(env: &BTreeMap<String, String>) -> Option<ProxyDetails> {
    for key in [
        "all_proxy",
        "ALL_PROXY",
        "https_proxy",
        "HTTPS_PROXY",
        "http_proxy",
        "HTTP_PROXY",
        "socks_proxy",
        "SOCKS_PROXY",
    ] {
        let Some(raw) = env.get(key).map(String::as_str) else {
            continue;
        };
        let Some(endpoint) = parse_proxy_endpoint(raw) else {
            continue;
        };

        let ip = resolve_host_ip(&endpoint.host, endpoint.port);
        let display = endpoint
            .port
            .map(|port| format!("{}:{port}", endpoint.host))
            .unwrap_or_else(|| endpoint.host.clone());

        return Some(ProxyDetails { display, ip });
    }

    detect_system_proxy_details()
}

#[cfg(not(windows))]
fn detect_system_proxy_details() -> Option<ProxyDetails> {
    let settings = settings_for_schema("org.gnome.system.proxy")?;
    if settings.string("mode").as_str() == "none" {
        return None;
    }

    for schema_id in [
        "org.gnome.system.proxy.https",
        "org.gnome.system.proxy.http",
        "org.gnome.system.proxy.socks",
    ] {
        let settings = settings_for_schema(schema_id)?;
        let host = settings.string("host");
        let host = host.trim();
        if host.is_empty() {
            continue;
        }

        let port = settings.uint("port");
        let endpoint = ProxyEndpoint {
            host: host.to_string(),
            port: (port > 0).then_some(port as u16),
        };
        let ip = resolve_host_ip(&endpoint.host, endpoint.port);
        let display = endpoint
            .port
            .map(|port| format!("{}:{port}", endpoint.host))
            .unwrap_or_else(|| endpoint.host.clone());

        return Some(ProxyDetails { display, ip });
    }

    None
}

#[cfg(windows)]
fn detect_system_proxy_details() -> Option<ProxyDetails> {
    None
}

#[cfg(not(windows))]
fn settings_for_schema(schema_id: &str) -> Option<gio::Settings> {
    let schema = gio::SettingsSchemaSource::default()?.lookup(schema_id, true)?;
    Some(gio::Settings::new_full(
        &schema,
        None::<&gio::SettingsBackend>,
        None,
    ))
}

fn detect_vpn_context(
    interfaces: &[InterfaceAddress],
    default_interface: Option<&str>,
) -> Option<VpnContext> {
    if let Some(cached) = cached_vpn_context() {
        return cached;
    }

    let detected = detect_expressvpn_context(interfaces, default_interface)
        .or_else(|| detect_nmcli_vpn_context())
        .or_else(|| detect_interface_vpn_context(interfaces, default_interface));
    store_cached_vpn_context(detected.clone());
    detected
}

fn cached_vpn_context() -> Option<Option<VpnContext>> {
    VPN_CONTEXT_CACHE.with(|slot| {
        let cache = slot.borrow();
        let cached = cache.as_ref()?;
        let age = util::now_epoch_seconds().saturating_sub(cached.updated_at);
        (age <= VPN_CACHE_MAX_AGE_SECONDS).then(|| cached.value.clone())
    })
}

fn store_cached_vpn_context(value: Option<VpnContext>) {
    VPN_CONTEXT_CACHE.with(|slot| {
        *slot.borrow_mut() = Some(CachedVpnContext {
            updated_at: util::now_epoch_seconds(),
            value,
        });
    });
}

fn detect_expressvpn_context(
    interfaces: &[InterfaceAddress],
    default_interface: Option<&str>,
) -> Option<VpnContext> {
    let executable = find_expressvpnctl()?;
    let state = util::command_output(&executable, &["get", "connectionstate"])?;
    let region = util::command_output(&executable, &["get", "region"]);
    let smart_region = region
        .as_deref()
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("smart"))
        .then(|| util::command_output(&executable, &["get", "smart"]))
        .flatten();
    let protocol = util::command_output(&executable, &["get", "protocol"]);
    let assigned_ip = util::command_output(&executable, &["get", "vpnip"])
        .filter(|value| !value.eq_ignore_ascii_case("unknown"));
    let interface_name = expressvpn_interface_name(interfaces, default_interface).or_else(|| {
        default_interface
            .filter(|name| is_vpn_interface_name(name))
            .map(str::to_string)
    });

    let details = VpnContext {
        provider: Some("ExpressVPN".to_string()),
        region,
        smart_region,
        protocol,
        state: Some(state),
        interface_name,
        connection_name: None,
        assigned_ip,
    };

    if details.is_activeish() {
        Some(details)
    } else {
        None
    }
}

fn find_expressvpnctl() -> Option<String> {
    EXPRESSVPNCTL_CANDIDATES.iter().find_map(|candidate| {
        let path = Path::new(candidate);
        if path.is_absolute() {
            path.exists().then(|| candidate.to_string())
        } else {
            util::resolve_executable_path(candidate)
        }
    })
}

fn expressvpn_interface_name(
    interfaces: &[InterfaceAddress],
    default_interface: Option<&str>,
) -> Option<String> {
    if let Some(default_interface) = default_interface
        && is_expressvpn_interface_name(default_interface)
    {
        return Some(default_interface.to_string());
    }

    interfaces
        .iter()
        .find(|interface| is_expressvpn_interface_name(&interface.name))
        .map(|interface| interface.name.clone())
}

fn is_expressvpn_interface_name(name: &str) -> bool {
    let normalized = name.to_ascii_lowercase();
    normalized.starts_with("tap")
        || normalized.starts_with("tun")
        || normalized.starts_with("ppp")
        || normalized.contains("expressvpn")
}

#[cfg(not(windows))]
fn detect_nmcli_vpn_context() -> Option<VpnContext> {
    let output = util::command_output(
        "nmcli",
        &[
            "-t",
            "-f",
            "NAME,TYPE,DEVICE,STATE",
            "connection",
            "show",
            "--active",
        ],
    )?;

    output.lines().find_map(|line| {
        let mut parts = line.splitn(4, ':');
        let name = parts.next()?.trim();
        let conn_type = parts.next()?.trim().to_ascii_lowercase();
        let device = parts.next()?.trim();
        let state = parts.next().unwrap_or_default().trim();
        let is_vpn_like = matches!(conn_type.as_str(), "vpn" | "wireguard")
            || is_vpn_interface_name(device)
            || infer_vpn_provider(name).is_some();
        if !is_vpn_like {
            return None;
        }

        let provider = infer_vpn_provider(name).or_else(|| infer_vpn_provider(device));
        let connection_name = (!name.is_empty()).then(|| name.to_string());
        let region = connection_name
            .as_ref()
            .and_then(|value| region_from_connection_name(value, provider.as_deref()));
        Some(VpnContext {
            provider,
            region,
            smart_region: None,
            protocol: matches!(conn_type.as_str(), "wireguard").then(|| "WireGuard".to_string()),
            state: (!state.is_empty()).then(|| state.to_string()),
            interface_name: (!device.is_empty()).then(|| device.to_string()),
            connection_name,
            assigned_ip: None,
        })
    })
}

#[cfg(windows)]
fn detect_nmcli_vpn_context() -> Option<VpnContext> {
    None
}

fn detect_interface_vpn_context(
    interfaces: &[InterfaceAddress],
    default_interface: Option<&str>,
) -> Option<VpnContext> {
    let interface = default_interface
        .filter(|name| is_vpn_interface_name(name))
        .map(str::to_string)
        .or_else(|| {
            interfaces
                .iter()
                .find(|interface| is_vpn_interface_name(&interface.name))
                .map(|interface| interface.name.clone())
        })?;

    let provider = infer_vpn_provider(&interface).or_else(|| {
        if is_expressvpn_service_active() {
            Some("ExpressVPN".to_string())
        } else {
            None
        }
    });
    let assigned_ip = interfaces
        .iter()
        .find(|entry| entry.name == interface)
        .map(|entry| entry.ip.clone());

    Some(VpnContext {
        provider,
        region: None,
        smart_region: None,
        protocol: infer_protocol_from_interface(&interface),
        state: Some("Connected".to_string()),
        interface_name: Some(interface),
        connection_name: None,
        assigned_ip,
    })
}

fn infer_vpn_provider(value: &str) -> Option<String> {
    let normalized = value.to_ascii_lowercase();
    if normalized.contains("expressvpn") {
        return Some("ExpressVPN".to_string());
    }
    if normalized.contains("nord") {
        return Some("NordVPN".to_string());
    }
    if normalized.contains("proton") {
        return Some("Proton VPN".to_string());
    }
    if normalized.contains("mullvad") {
        return Some("Mullvad".to_string());
    }
    if normalized.contains("tailscale") || normalized.starts_with("ts") {
        return Some("Tailscale".to_string());
    }
    if normalized.contains("surfshark") {
        return Some("Surfshark".to_string());
    }
    if normalized.contains("wireguard") || normalized.starts_with("wg") {
        return Some("WireGuard".to_string());
    }
    if normalized.contains("openvpn")
        || normalized.starts_with("tun")
        || normalized.starts_with("tap")
    {
        return Some("OpenVPN".to_string());
    }
    None
}

fn region_from_connection_name(name: &str, provider: Option<&str>) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }

    let without_provider = provider
        .map(|provider| trimmed.replace(provider, ""))
        .unwrap_or_else(|| trimmed.to_string());
    let region = without_provider
        .trim_matches(|ch: char| matches!(ch, '-' | '_' | ' ' | '(' | ')'))
        .trim();
    (!region.is_empty() && !region.eq_ignore_ascii_case(trimmed)).then(|| region.to_string())
}

fn infer_protocol_from_interface(interface: &str) -> Option<String> {
    let normalized = interface.to_ascii_lowercase();
    if normalized.starts_with("wg") {
        return Some("WireGuard".to_string());
    }
    if normalized.starts_with("tun") || normalized.starts_with("tap") {
        return Some("OpenVPN".to_string());
    }
    if normalized.starts_with("tailscale") || normalized.starts_with("ts") {
        return Some("WireGuard".to_string());
    }
    None
}

fn is_expressvpn_service_active() -> bool {
    Path::new("/etc/systemd/system/expressvpn-service.service").exists()
        || Path::new("/opt/expressvpn/bin/expressvpn-daemon").exists()
}

fn parse_proxy_endpoint(raw: &str) -> Option<ProxyEndpoint> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let without_scheme = trimmed
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(trimmed);
    let authority = without_scheme.split('/').next().unwrap_or(without_scheme);
    let host_port = authority
        .rsplit_once('@')
        .map(|(_, rest)| rest)
        .unwrap_or(authority);
    let host_port = host_port.trim();
    if host_port.is_empty() {
        return None;
    }

    if let Some(rest) = host_port.strip_prefix('[') {
        let end = rest.find(']')?;
        let host = rest[..end].to_string();
        let port = rest[end + 1..]
            .strip_prefix(':')
            .and_then(|value| value.parse::<u16>().ok());
        return Some(ProxyEndpoint { host, port });
    }

    if host_port.matches(':').count() > 1 {
        return Some(ProxyEndpoint {
            host: host_port.to_string(),
            port: None,
        });
    }

    let (host, port) = host_port
        .split_once(':')
        .map(|(host, port)| (host.to_string(), port.parse::<u16>().ok()))
        .unwrap_or_else(|| (host_port.to_string(), None));

    (!host.is_empty()).then_some(ProxyEndpoint { host, port })
}

fn resolve_host_ip(host: &str, port: Option<u16>) -> Option<String> {
    if host.parse::<IpAddr>().is_ok() {
        return Some(host.to_string());
    }

    let port = port.unwrap_or(80);
    (host, port)
        .to_socket_addrs()
        .ok()?
        .find_map(|address| match address.ip() {
            IpAddr::V4(ip) => Some(ip.to_string()),
            IpAddr::V6(_) => None,
        })
        .or_else(|| {
            (host, port)
                .to_socket_addrs()
                .ok()?
                .next()
                .map(|address| address.ip().to_string())
        })
}

fn primary_local_interface_ip(
    addresses: &[InterfaceAddress],
    default_interface: Option<&str>,
) -> Option<String> {
    if let Some(default_interface) = default_interface
        && let Some(address) = addresses.iter().find(|address| {
            address.name == default_interface
                && !is_vpn_interface_name(&address.name)
                && !is_ignored_local_interface(&address.name)
        })
    {
        return Some(address.ip.clone());
    }

    addresses
        .iter()
        .find(|address| {
            !is_vpn_interface_name(&address.name) && !is_ignored_local_interface(&address.name)
        })
        .or_else(|| {
            addresses
                .iter()
                .find(|address| !is_vpn_interface_name(&address.name))
        })
        .map(|address| address.ip.clone())
}

fn primary_vpn_interface_ip(
    addresses: &[InterfaceAddress],
    default_interface: Option<&str>,
) -> Option<String> {
    if let Some(default_interface) = default_interface
        && let Some(address) = addresses.iter().find(|address| {
            address.name == default_interface && is_vpn_interface_name(&address.name)
        })
    {
        return Some(address.ip.clone());
    }

    addresses
        .iter()
        .find(|address| is_vpn_interface_name(&address.name))
        .map(|address| address.ip.clone())
}

fn is_vpn_interface_name(name: &str) -> bool {
    let normalized = name.to_ascii_lowercase();
    [
        "tun",
        "tap",
        "wg",
        "ppp",
        "tailscale",
        "ts",
        "zt",
        "utun",
        "vpn",
        "ipsec",
    ]
    .iter()
    .any(|prefix| normalized.starts_with(prefix))
}

fn is_ignored_local_interface(name: &str) -> bool {
    let normalized = name.to_ascii_lowercase();
    [
        "docker", "br-", "virbr", "veth", "cni", "podman", "flannel", "kube", "zt",
    ]
    .iter()
    .any(|prefix| normalized.starts_with(prefix))
}

#[cfg(target_os = "linux")]
fn default_route_interface() -> Option<String> {
    let content = fs::read_to_string("/proc/net/route").ok()?;
    content.lines().skip(1).find_map(|line| {
        let fields: Vec<&str> = line.split_whitespace().collect();
        (fields.get(1) == Some(&"00000000"))
            .then(|| fields.first().map(|value| value.to_string()))?
    })
}

#[cfg(not(target_os = "linux"))]
fn default_route_interface() -> Option<String> {
    None
}

#[cfg(unix)]
fn interface_ipv4_addresses() -> Vec<InterfaceAddress> {
    let mut list = Vec::new();
    let mut head = std::ptr::null_mut();

    unsafe {
        if libc::getifaddrs(&mut head) != 0 {
            return list;
        }

        let mut current = head;
        while !current.is_null() {
            let entry = &*current;
            let addr = entry.ifa_addr;
            if !addr.is_null() && i32::from((*addr).sa_family) == libc::AF_INET {
                let flags = entry.ifa_flags as i32;
                let is_up = flags & libc::IFF_UP != 0;
                let is_loopback = flags & libc::IFF_LOOPBACK != 0;
                if is_up && !is_loopback {
                    let name = CStr::from_ptr(entry.ifa_name)
                        .to_string_lossy()
                        .into_owned();
                    let socket = *(addr as *const libc::sockaddr_in);
                    let ip = std::net::Ipv4Addr::from(u32::from_be(socket.sin_addr.s_addr));
                    list.push(InterfaceAddress {
                        name,
                        ip: ip.to_string(),
                    });
                }
            }
            current = entry.ifa_next;
        }

        libc::freeifaddrs(head);
    }

    list.sort_by(|left, right| left.name.cmp(&right.name).then(left.ip.cmp(&right.ip)));
    list.dedup_by(|left, right| left.name == right.name && left.ip == right.ip);
    list
}

#[cfg(not(unix))]
fn interface_ipv4_addresses() -> Vec<InterfaceAddress> {
    util::primary_local_ip()
        .map(|ip| {
            vec![InterfaceAddress {
                name: "default".to_string(),
                ip,
            }]
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone)]
struct ProxyEndpoint {
    host: String,
    port: Option<u16>,
}

#[derive(Debug, Clone)]
struct ProxyDetails {
    display: String,
    ip: Option<String>,
}

#[derive(Debug, Clone)]
struct CachedVpnContext {
    updated_at: u64,
    value: Option<VpnContext>,
}

#[derive(Debug, Clone)]
struct InterfaceAddress {
    name: String,
    ip: String,
}

fn humanize_slug(input: &str) -> String {
    let compact = input.trim().replace(['_', '-'], " ");
    let acronyms = ["uk", "us", "usa", "uae", "vpn", "tcp", "udp"];
    compact
        .split_whitespace()
        .map(|part| {
            let lowered = part.to_ascii_lowercase();
            if acronyms.contains(&lowered.as_str()) {
                lowered.to_ascii_uppercase()
            } else if lowered.chars().all(|ch| ch.is_ascii_digit()) {
                lowered
            } else {
                let mut chars = lowered.chars();
                let first = chars.next().unwrap_or_default().to_ascii_uppercase();
                format!("{first}{}", chars.collect::<String>())
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn push_network_field(parts: &mut Vec<String>, label: &str, value: impl AsRef<str>) {
    let value = value.as_ref().trim();
    if value.is_empty() {
        return;
    }

    parts.push(format!("{label}: {value}"));
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
        python_project: None,
        active_python_venv: None,
        network: detect_network_context(&BTreeMap::new()),
        mode: if shell_pid.is_some() {
            PanelMode::Shell
        } else {
            PanelMode::Exited
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let counter = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "termvoid-{label}-{}-{counter}-{nonce}",
            std::process::id()
        ))
    }

    fn build_python_project(root: &Path) {
        fs::create_dir_all(root.join(".venv/bin")).unwrap();
        fs::write(root.join(".venv/pyvenv.cfg"), "home = /usr/bin\n").unwrap();
        fs::write(root.join(".venv/bin/activate"), "export VIRTUAL_ENV=1\n").unwrap();
        fs::write(root.join("main.py"), "print('ok')\n").unwrap();
    }

    #[test]
    fn detecta_proyecto_python_con_venv_en_directorio_actual() {
        let root = unique_temp_dir("python-project-root");
        build_python_project(&root);

        let detected = detect_python_project(&root).expect("debe detectar el proyecto Python");
        assert_eq!(detected.project_root, root);
        assert_eq!(detected.venv_name, ".venv");
        assert!(detected.venv_path.ends_with(".venv"));

        let _ = fs::remove_dir_all(detected.project_root);
    }

    #[test]
    fn detecta_proyecto_python_si_solo_hay_venv_en_la_raiz() {
        let root = unique_temp_dir("python-project-venv-only");
        fs::create_dir_all(root.join(".venv/bin")).unwrap();
        fs::write(root.join(".venv/pyvenv.cfg"), "home = /usr/bin\n").unwrap();
        fs::write(root.join(".venv/bin/activate"), "export VIRTUAL_ENV=1\n").unwrap();

        let detected = detect_python_project(&root).expect("debe detectar el venv del proyecto");
        assert_eq!(detected.project_root, root);
        assert_eq!(detected.venv_name, ".venv");

        let _ = fs::remove_dir_all(detected.project_root);
    }

    #[test]
    fn detecta_proyecto_python_desde_subdirectorio() {
        let root = unique_temp_dir("python-project-nested");
        build_python_project(&root);
        let nested = root.join("src/services");
        fs::create_dir_all(&nested).unwrap();

        let detected = detect_python_project(&nested).expect("debe subir hasta la raiz");
        assert_eq!(detected.project_root, root);
        assert!(detected.venv_path.ends_with(".venv"));

        let _ = fs::remove_dir_all(detected.project_root);
    }

    #[test]
    fn detecta_proyecto_python_real_si_se_define_ruta() {
        let Some(raw_path) = std::env::var_os("TERMVOID_REAL_PYTHON_PROJECT") else {
            return;
        };
        let path = PathBuf::from(raw_path);
        if !path.exists() {
            return;
        }

        let detected = detect_python_project(&path).expect("debe detectar la ruta real");
        assert_eq!(detected.project_root, path);
        assert!(detected.venv_path.ends_with(".venv"));
        assert!(
            detected.venv_path.join("bin/activate").exists()
                || detected.venv_path.join("Scripts/activate").exists()
        );
    }
}
