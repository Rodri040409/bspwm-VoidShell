use crate::constants;
use directories::ProjectDirs;
use gtk::gdk;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::env;
#[cfg(unix)]
use std::ffi::CStr;
use std::fs;
use std::net::{SocketAddr, UdpSocket};
use std::path::{Path, PathBuf};

thread_local! {
    static WALLPAPER_CACHE: RefCell<BTreeMap<String, gdk::Texture>> = const { RefCell::new(BTreeMap::new()) };
}

pub fn default_shell_path() -> String {
    if let Some(shell) = env::var("SHELL")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        return resolve_executable_path(&shell).unwrap_or(shell);
    }

    #[cfg(windows)]
    {
        if let Some(comspec) = env::var("COMSPEC")
            .ok()
            .filter(|value| !value.trim().is_empty())
        {
            return resolve_executable_path(&comspec).unwrap_or(comspec);
        }

        if let Some(pwsh) = resolve_executable_path("pwsh") {
            return pwsh;
        }

        if let Some(powershell) = resolve_executable_path("powershell") {
            return powershell;
        }

        return "cmd.exe".to_string();
    }

    #[cfg(unix)]
    unsafe {
        let uid = libc::geteuid();
        let pwd = libc::getpwuid(uid);
        if !pwd.is_null() {
            let shell = CStr::from_ptr((*pwd).pw_shell)
                .to_string_lossy()
                .into_owned();
            if !shell.trim().is_empty() {
                return shell;
            }
        }
    }

    #[cfg(not(windows))]
    {
        if let Some(shell) = resolve_executable_path("bash") {
            return shell;
        }

        if let Some(shell) = resolve_executable_path("sh") {
            return shell;
        }

        return "/bin/bash".to_string();
    }

    #[cfg(windows)]
    unreachable!("windows branch returned earlier");
}

pub fn effective_shell_path(configured: &str) -> String {
    let configured = configured.trim();
    if configured.is_empty() {
        return default_shell_path();
    }

    resolve_executable_path(configured).unwrap_or_else(|| configured.to_string())
}

pub fn default_shell_args(shell_path: &str) -> Vec<String> {
    let shell = shell_name(shell_path).to_ascii_lowercase();
    match shell.as_str() {
        "bash" => {
            let mut args = Vec::new();
            if let Some(rcfile) = bash_integration_rcfile() {
                args.push("--rcfile".to_string());
                args.push(rcfile);
            }
            args.push("-i".to_string());
            args
        }
        "zsh" | "sh" | "fish" | "nu" | "nushell" => vec!["-i".to_string()],
        "pwsh" | "pwsh.exe" | "pwsh-preview" | "powershell" | "powershell.exe" => {
            vec!["-NoLogo".to_string()]
        }
        "cmd" | "cmd.exe" => vec!["/K".to_string()],
        _ => Vec::new(),
    }
}

pub fn shell_name(shell_path: &str) -> String {
    Path::new(shell_path)
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "shell".to_string())
}

pub fn hostname() -> String {
    fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            env::var("HOSTNAME")
                .ok()
                .or_else(|| env::var("COMPUTERNAME").ok())
        })
        .or_else(|| command_output("hostname", &[]))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "localhost".to_string())
}

pub fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
        .or_else(|| {
            let drive = env::var_os("HOMEDRIVE")?;
            let path = env::var_os("HOMEPATH")?;
            let mut combined = PathBuf::from(drive);
            combined.push(path);
            Some(combined)
        })
}

pub fn runtime_icon_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    let dev_assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/icons");
    if dev_assets.exists() {
        paths.push(dev_assets);
    }

    if let Some(home) = home_dir() {
        let local_icons = home.join(".local/share/icons");
        if local_icons.exists() {
            paths.push(local_icons);
        }
    }

    if let Ok(exe) = env::current_exe()
        && let Some(bin_dir) = exe.parent()
    {
        let bundled_share = bin_dir.join("../share/icons");
        if bundled_share.exists() {
            paths.push(bundled_share);
        }
    }

    paths
}

pub fn live_banner_state_file() -> PathBuf {
    project_state_file("live-banner.ansi")
}

pub fn write_live_banner(payload: &str) -> Option<PathBuf> {
    let path = live_banner_state_file();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok()?;
    }
    fs::write(&path, payload).ok()?;
    Some(path)
}

#[cfg(not(windows))]
fn linux_data_home() -> Option<PathBuf> {
    env::var_os("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|home| home.join(".local/share")))
}

#[cfg(not(windows))]
pub fn install_local_desktop_integration() -> Option<()> {
    let data_home = linux_data_home()?;
    let source_icons_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/icons/hicolor");
    let applications_dir = data_home.join("applications");
    let icons_root = data_home.join("icons");
    let hicolor_root = icons_root.join("hicolor");
    let desktop_target = applications_dir.join(format!("{}.desktop", constants::APP_ID));
    let pixmaps_target = data_home
        .join("pixmaps")
        .join(format!("{}.png", constants::APP_ICON));
    let current_exe = env::current_exe().ok()?;

    if let Some(parent) = desktop_target.parent() {
        fs::create_dir_all(parent).ok()?;
    }
    if let Some(parent) = pixmaps_target.parent() {
        fs::create_dir_all(parent).ok()?;
    }

    for size in ["64x64", "128x128", "256x256", "512x512"] {
        let source = source_icons_root
            .join(size)
            .join("apps")
            .join(format!("{}.png", constants::APP_ICON));
        if !source.exists() {
            continue;
        }

        let target = hicolor_root
            .join(size)
            .join("apps")
            .join(format!("{}.png", constants::APP_ICON));
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).ok()?;
        }
        let _ = fs::copy(&source, &target);

        if size == "256x256" {
            let _ = fs::copy(&source, &pixmaps_target);
        }
    }

    let desktop_content = format!(
        "[Desktop Entry]\nName={name}\nComment=VoidShell tiling terminal with contextual chrome\nExec={exec} %F\nIcon={icon}\nTerminal=false\nType=Application\nCategories=System;TerminalEmulator;GTK;\nKeywords=terminal;shell;console;pty;\nMimeType=x-scheme-handler/terminal;inode/directory;\nStartupNotify=true\nStartupWMClass={wm_class}\nX-ExecArg=--execute\nX-TerminalArgDir=--working-directory\n",
        name = constants::APP_NAME,
        exec = desktop_exec_value(&current_exe),
        icon = constants::APP_ICON,
        wm_class = constants::APP_ID,
    );
    let _ = fs::write(&desktop_target, desktop_content);

    if command_exists("update-desktop-database") {
        let _ = std::process::Command::new("update-desktop-database")
            .arg(&applications_dir)
            .output();
    }

    if command_exists("gtk-update-icon-cache") {
        let _ = std::process::Command::new("gtk-update-icon-cache")
            .args(["-f", "-t"])
            .arg(&hicolor_root)
            .output();
    }

    Some(())
}

#[cfg(windows)]
pub fn install_local_desktop_integration() -> Option<()> {
    None
}

pub fn display_path(path: &Path) -> String {
    if let Some(home) = home_dir() {
        if let Ok(stripped) = path.strip_prefix(&home) {
            if stripped.as_os_str().is_empty() {
                return "~".to_string();
            }
            return format!("~/{}", stripped.display());
        }
    }

    path.display().to_string()
}

pub fn shell_quote(input: &str) -> String {
    if input.is_empty() {
        return "''".to_string();
    }

    let escaped = input.replace('\'', "'\"'\"'");
    format!("'{escaped}'")
}

pub fn supports_python_venv_commands(shell_path: &str) -> bool {
    python_venv_deactivation_command(shell_path).is_some()
}

pub fn python_venv_activation_command(shell_path: &str, venv_root: &Path) -> Option<String> {
    let shell = shell_name(shell_path).to_ascii_lowercase();
    match shell.as_str() {
        "bash" | "zsh" | "sh" => {
            let script = venv_root.join("bin/activate");
            script
                .exists()
                .then(|| format!(". {}", shell_quote(&script.display().to_string())))
        }
        "fish" => {
            let script = venv_root.join("bin/activate.fish");
            script
                .exists()
                .then(|| format!("source {}", shell_quote(&script.display().to_string())))
        }
        "pwsh" | "pwsh.exe" | "pwsh-preview" | "powershell" | "powershell.exe" => {
            let script = venv_root.join("Scripts/Activate.ps1");
            script.exists().then(|| format!("& '{}'", script.display()))
        }
        "cmd" | "cmd.exe" => {
            let script = venv_root.join("Scripts/activate.bat");
            script
                .exists()
                .then(|| format!("call \"{}\"", script.display()))
        }
        _ => None,
    }
}

pub fn python_venv_deactivation_command(shell_path: &str) -> Option<String> {
    let shell = shell_name(shell_path).to_ascii_lowercase();
    match shell.as_str() {
        "bash" | "zsh" | "sh" | "fish" | "pwsh" | "pwsh.exe" | "pwsh-preview" | "powershell"
        | "powershell.exe" | "cmd" | "cmd.exe" => Some("deactivate".to_string()),
        _ => None,
    }
}

fn desktop_exec_value(path: &Path) -> String {
    path.display()
        .to_string()
        .replace('\\', "\\\\")
        .replace(' ', "\\ ")
}

pub fn first_command_token(command: &str) -> Option<String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut chars = trimmed.chars();
    let first = chars.next()?;

    if first == '"' || first == '\'' {
        let token: String = chars.take_while(|ch| *ch != first).collect();
        return (!token.is_empty()).then_some(token);
    }

    let mut token = String::from(first);
    token.extend(chars.take_while(|ch| !ch.is_whitespace()));
    Some(token)
}

pub fn command_exists(program: &str) -> bool {
    resolve_executable_path(program).is_some()
}

pub fn command_line_exists(command: &str) -> bool {
    first_command_token(command)
        .as_deref()
        .is_some_and(command_exists)
}

pub fn resolve_executable_path(program: &str) -> Option<String> {
    let program = program.trim();
    if program.is_empty() {
        return None;
    }

    let candidate = Path::new(program);
    if candidate.components().count() > 1 || candidate.is_absolute() {
        return candidate.exists().then(|| candidate.display().to_string());
    }

    let path_var = env::var_os("PATH")?;
    let extensions = executable_extensions();
    for directory in env::split_paths(&path_var) {
        for extension in &extensions {
            let full_path = if extension.is_empty() {
                directory.join(program)
            } else {
                directory.join(format!("{program}{extension}"))
            };

            if full_path.exists() {
                return Some(full_path.display().to_string());
            }
        }
    }

    None
}

fn executable_extensions() -> Vec<String> {
    #[cfg(windows)]
    {
        return env::var("PATHEXT")
            .ok()
            .map(|value| {
                value
                    .split(';')
                    .filter(|entry| !entry.trim().is_empty())
                    .map(|entry| entry.to_ascii_lowercase())
                    .collect::<Vec<_>>()
            })
            .filter(|entries| !entries.is_empty())
            .unwrap_or_else(|| vec![".exe".to_string(), ".bat".to_string(), ".cmd".to_string()]);
    }

    #[cfg(not(windows))]
    {
        vec![String::new()]
    }
}

pub fn expand_user_path(input: &str) -> PathBuf {
    if let Some(stripped) = input.strip_prefix("~/") {
        if let Some(home) = home_dir() {
            return home.join(stripped);
        }
    }

    PathBuf::from(input)
}

pub fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!stdout.is_empty()).then_some(stdout)
}

pub fn project_state_file(name: &str) -> PathBuf {
    ProjectDirs::from(
        constants::CONFIG_QUALIFIER,
        constants::CONFIG_ORGANIZATION,
        constants::CONFIG_APPLICATION,
    )
    .and_then(|dirs| dirs.state_dir().map(|path| path.join(name)))
    .unwrap_or_else(|| PathBuf::from(format!(".termvoid-{name}")))
}

pub fn cached_wallpaper_texture(path: &str) -> Option<gdk::Texture> {
    let path = path.trim();
    if path.is_empty() {
        return None;
    }

    WALLPAPER_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(texture) = cache.get(path).cloned() {
            return Some(texture);
        }

        let texture = gdk::Texture::from_filename(path).ok()?;
        cache.insert(path.to_string(), texture.clone());
        Some(texture)
    })
}

pub fn primary_local_ip() -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("1.1.1.1:80").ok()?;
    match socket.local_addr().ok()? {
        SocketAddr::V4(address) if !address.ip().is_loopback() => Some(address.ip().to_string()),
        _ => command_output("sh", &["-lc", "hostname -I 2>/dev/null | awk '{print $1}'"]),
    }
}

pub fn cached_public_ip(max_age_seconds: u64) -> Option<String> {
    let path = project_state_file("public-ip.txt");
    let stale = read_public_ip_cache(&path);

    if let Some((timestamp, value)) = stale.as_ref() {
        let now = now_epoch_seconds();
        if now.saturating_sub(*timestamp) <= max_age_seconds {
            return Some(value.clone());
        }
    }

    let fresh = fetch_public_ip();
    if let Some(value) = fresh {
        write_public_ip_cache(&path, &value);
        return Some(value);
    }

    stale.map(|(_, value)| value)
}

pub fn readline_inputrc(shell_path: &str) -> Option<String> {
    let shell = shell_name(shell_path).to_ascii_lowercase();
    if !matches!(shell.as_str(), "bash" | "sh" | "rbash") {
        return None;
    }

    let path = project_state_file("readline.inputrc");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok()?;
    }

    let mut content = String::new();
    if let Some(base) = env::var("INPUTRC")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            home_dir()
                .map(|home| home.join(".inputrc"))
                .filter(|path| path.exists())
                .map(|path| path.display().to_string())
        })
    {
        content.push_str(&format!("$include {base}\n\n"));
    }

    content.push_str(
        "set show-all-if-ambiguous on\n\
         set mark-symlinked-directories on\n\
         set completion-ignore-case on\n\
         set menu-complete-display-prefix on\n\
         \"\\t\": complete\n\
         \"\\e[Z\": complete\n\
         \"\\e/\": dynamic-complete-history\n\
         \"\\e[A\": history-search-backward\n\
         \"\\e[B\": history-search-forward\n",
    );

    fs::write(&path, content).ok()?;
    Some(path.display().to_string())
}

pub fn read_os_release_value(key: &str) -> Option<String> {
    let content = fs::read_to_string("/etc/os-release").ok()?;
    for line in content.lines() {
        let prefix = format!("{key}=");
        if let Some(value) = line.strip_prefix(&prefix) {
            return Some(value.trim_matches('"').to_string());
        }
    }
    None
}

pub fn platform_display_name() -> Option<String> {
    read_os_release_value("PRETTY_NAME")
        .or_else(|| command_output("sw_vers", &["-productName"]))
        .or_else(|| {
            command_output(
                "powershell",
                &[
                    "-NoProfile",
                    "-Command",
                    "(Get-CimInstance Win32_OperatingSystem).Caption",
                ],
            )
        })
        .or_else(|| Some(std::env::consts::OS.to_string()))
}

pub fn kernel_release() -> Option<String> {
    fs::read_to_string("/proc/sys/kernel/osrelease")
        .ok()
        .map(|value| value.trim().to_string())
        .or_else(|| command_output("uname", &["-r"]))
        .or_else(|| command_output("cmd", &["/C", "ver"]))
        .filter(|value| !value.is_empty())
}

pub fn cpu_description() -> Option<String> {
    read_first_matching_line("/proc/cpuinfo", "model name")
        .or_else(|| command_output("sysctl", &["-n", "machdep.cpu.brand_string"]))
        .or_else(|| {
            command_output(
                "powershell",
                &[
                    "-NoProfile",
                    "-Command",
                    "(Get-CimInstance Win32_Processor | Select-Object -First 1 -ExpandProperty Name)",
                ],
            )
        })
}

pub fn read_first_matching_line(path: &str, prefix: &str) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    content.lines().find_map(|line| {
        line.split_once(':')
            .and_then(|(name, value)| (name.trim() == prefix).then(|| value.trim().to_string()))
    })
}

pub fn mem_total_gib() -> Option<String> {
    let line = read_first_matching_line("/proc/meminfo", "MemTotal")?;
    let kib = line.split_whitespace().next()?.parse::<f64>().ok()?;
    Some(format!("{:.1} GiB", kib / 1024.0 / 1024.0))
}

pub fn mem_total_gib_portable() -> Option<String> {
    mem_total_gib()
        .or_else(|| {
            command_output("sysctl", &["-n", "hw.memsize"]).and_then(|value| {
                value.trim()
                    .parse::<f64>()
                    .ok()
                    .map(|bytes| format!("{:.1} GiB", bytes / 1024.0 / 1024.0 / 1024.0))
            })
        })
        .or_else(|| {
            command_output(
                "powershell",
                &[
                    "-NoProfile",
                    "-Command",
                    "[math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB, 1)",
                ],
            )
            .map(|value| format!("{value} GiB"))
        })
}

pub fn now_epoch_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

pub fn envv(shell_path: &str) -> Vec<String> {
    let mut vars = BTreeMap::new();

    for (key, value) in std::env::vars_os() {
        vars.insert(
            key.to_string_lossy().into_owned(),
            value.to_string_lossy().into_owned(),
        );
    }

    vars.insert("TERM".to_string(), "xterm-256color".to_string());
    vars.insert("COLORTERM".to_string(), "truecolor".to_string());
    vars.insert("TERM_PROGRAM".to_string(), constants::APP_NAME.to_string());
    vars.insert(
        "TERM_PROGRAM_VERSION".to_string(),
        constants::APP_VERSION.to_string(),
    );
    vars.insert(
        "VOIDSHELL_BANNER_FILE".to_string(),
        live_banner_state_file().display().to_string(),
    );
    if let Some(inputrc) = readline_inputrc(shell_path) {
        vars.insert("INPUTRC".to_string(), inputrc);
    }

    vars.into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect()
}

pub fn parse_rgba(input: &str, fallback: &str) -> gdk::RGBA {
    gdk::RGBA::parse(input)
        .ok()
        .or_else(|| gdk::RGBA::parse(fallback).ok())
        .unwrap_or_else(|| gdk::RGBA::new(0.1, 0.1, 0.1, 1.0))
}

pub fn rgba_to_css(input: &gdk::RGBA) -> String {
    format!(
        "rgba({:.0}, {:.0}, {:.0}, {:.3})",
        input.red() * 255.0,
        input.green() * 255.0,
        input.blue() * 255.0,
        input.alpha()
    )
}

pub fn compact_label(input: &str) -> Cow<'_, str> {
    if input.chars().count() > 34 {
        Cow::Owned(format!("{}…", input.chars().take(33).collect::<String>()))
    } else {
        Cow::Borrowed(input)
    }
}

fn fetch_public_ip() -> Option<String> {
    if command_exists("curl") {
        return command_output(
            "curl",
            &["-4fsS", "--max-time", "0.4", "https://api.ipify.org"],
        );
    }

    if command_exists("wget") {
        return command_output("wget", &["-4qO-", "--timeout=1", "https://api.ipify.org"]);
    }

    None
}

fn read_public_ip_cache(path: &Path) -> Option<(u64, String)> {
    let content = fs::read_to_string(path).ok()?;
    let mut lines = content.lines();
    let timestamp = lines.next()?.trim().parse::<u64>().ok()?;
    let value = lines.next()?.trim().to_string();
    (!value.is_empty()).then_some((timestamp, value))
}

fn write_public_ip_cache(path: &Path, value: &str) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let payload = format!("{}\n{}\n", now_epoch_seconds(), value.trim());
    let _ = fs::write(path, payload);
}

fn bash_integration_rcfile() -> Option<String> {
    let path = project_state_file("bashrc");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok()?;
    }

    let mut content = String::new();
    content.push_str(
        "if [ -f /etc/bashrc ]; then\n  . /etc/bashrc\nfi\n\
         if [ -f \"$HOME/.bashrc\" ]; then\n  . \"$HOME/.bashrc\"\nfi\n\n",
    );
    content.push_str(VOIDSHELL_BASH_INTEGRATION);
    content.push_str(
        "\n__voidshell_show_banner() {\n\
         \t[ -r \"${VOIDSHELL_BANNER_FILE:-}\" ] || return 0\n\
         \tcommand cat -- \"$VOIDSHELL_BANNER_FILE\"\n\
         }\n\
         bind -x '\"\\C-g\":__voidshell_show_banner'\n",
    );
    fs::write(&path, content).ok()?;
    Some(path.display().to_string())
}

const VOIDSHELL_BASH_INTEGRATION: &str = r#"
# VoidShell interactive helpers
bind '"\t": complete'
bind '"\e[Z": complete'
bind '"\e/": dynamic-complete-history'
bind '"\e[A": history-search-backward'
bind '"\e[B": history-search-forward'

shopt -s direxpand 2>/dev/null

complete -A directory cd pushd popd 2>/dev/null || complete -o dirnames cd pushd popd

__voidshell_confirm_sudo() {
  local cmd="$1"
  shift
  printf '\n[VoidShell] "%s' "$cmd" >&2
  if [ "$#" -gt 0 ]; then
    printf ' %s' "$*" >&2
  fi
  printf '" normalmente requiere sudo. Ejecutarlo con sudo? [y/N] ' >&2

  local answer
  IFS= read -r answer
  case "$answer" in
    y|Y|yes|YES)
      history -s "sudo $cmd $*"
      sudo "$cmd" "$@"
      ;;
    *)
      command "$cmd" "$@"
      ;;
  esac
}

__voidshell_maybe_sudo() {
  local cmd="$1"
  shift

  if [ "${EUID:-0}" -eq 0 ] || [ -n "${SUDO_USER:-}" ]; then
    command "$cmd" "$@"
    return $?
  fi

  case "$cmd" in
    systemctl)
      for arg in "$@"; do
        case "$arg" in
          --user|--version|--help)
            command "$cmd" "$@"
            return $?
            ;;
        esac
      done
      __voidshell_confirm_sudo "$cmd" "$@"
      ;;
    dnf|yum|apt|apt-get|pacman|zypper|rpm|mount|umount|firewall-cmd|ufw|iptables|ip6tables|modprobe|dracut|mkinitcpio|shutdown|reboot|poweroff)
      __voidshell_confirm_sudo "$cmd" "$@"
      ;;
    *)
      command "$cmd" "$@"
      ;;
  esac
}

dnf() { __voidshell_maybe_sudo dnf "$@"; }
yum() { __voidshell_maybe_sudo yum "$@"; }
apt() { __voidshell_maybe_sudo apt "$@"; }
apt-get() { __voidshell_maybe_sudo apt-get "$@"; }
pacman() { __voidshell_maybe_sudo pacman "$@"; }
zypper() { __voidshell_maybe_sudo zypper "$@"; }
rpm() { __voidshell_maybe_sudo rpm "$@"; }
systemctl() { __voidshell_maybe_sudo systemctl "$@"; }
mount() { __voidshell_maybe_sudo mount "$@"; }
umount() { __voidshell_maybe_sudo umount "$@"; }
firewall-cmd() { __voidshell_maybe_sudo firewall-cmd "$@"; }
ufw() { __voidshell_maybe_sudo ufw "$@"; }
iptables() { __voidshell_maybe_sudo iptables "$@"; }
ip6tables() { __voidshell_maybe_sudo ip6tables "$@"; }
modprobe() { __voidshell_maybe_sudo modprobe "$@"; }
dracut() { __voidshell_maybe_sudo dracut "$@"; }
mkinitcpio() { __voidshell_maybe_sudo mkinitcpio "$@"; }
shutdown() { __voidshell_maybe_sudo shutdown "$@"; }
reboot() { __voidshell_maybe_sudo reboot "$@"; }
poweroff() { __voidshell_maybe_sudo poweroff "$@"; }
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
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
            "termvoid-util-{label}-{}-{counter}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn genera_comando_de_activacion_para_bash() {
        let root = unique_temp_dir("bash-venv");
        fs::create_dir_all(root.join("bin")).unwrap();
        fs::write(root.join("bin/activate"), "echo active\n").unwrap();

        let command = python_venv_activation_command("bash", &root)
            .expect("debe generar comando de activacion");
        assert_eq!(
            command,
            format!(
                ". {}",
                shell_quote(&root.join("bin/activate").display().to_string())
            )
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn genera_comando_de_desactivacion_para_shells_soportados() {
        assert_eq!(
            python_venv_deactivation_command("bash").as_deref(),
            Some("deactivate")
        );
        assert_eq!(
            python_venv_deactivation_command("zsh").as_deref(),
            Some("deactivate")
        );
        assert_eq!(
            python_venv_deactivation_command("fish").as_deref(),
            Some("deactivate")
        );
    }
}
