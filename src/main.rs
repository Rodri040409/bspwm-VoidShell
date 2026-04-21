mod banner;
mod config;
mod constants;
mod context;
mod history;
mod layout;
#[cfg(not(windows))]
mod preferences;
#[cfg(windows)]
#[path = "preferences_windows.rs"]
mod preferences;
mod quick_actions;
mod system_info;
mod theme;
mod util;
mod window;

#[cfg(not(windows))]
mod terminal_pane;
#[cfg(windows)]
#[path = "terminal_pane_windows.rs"]
mod terminal_pane;

use gtk::gio;
use gtk::prelude::*;
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Default)]
struct LaunchOptions {
    startup_command: Option<String>,
    working_directory: Option<PathBuf>,
}

fn normalize_launch_path(value: OsString) -> Option<PathBuf> {
    let candidate = PathBuf::from(&value);
    if candidate.is_dir() {
        return Some(candidate);
    }
    if candidate.is_file() {
        return candidate.parent().map(PathBuf::from);
    }

    let text = value.to_string_lossy();
    if text.contains("://") {
        let file = gtk::gio::File::for_uri(&text);
        if let Some(path) = file.path() {
            if path.is_dir() {
                return Some(path);
            }
            if path.is_file() {
                return path.parent().map(PathBuf::from);
            }
        }
    }

    None
}

fn command_from_args(args: impl Iterator<Item = OsString>) -> Option<String> {
    let parts: Vec<String> = args
        .map(|arg| util::shell_quote(&arg.to_string_lossy()))
        .collect();
    (!parts.is_empty()).then(|| parts.join(" "))
}

fn parse_launch_options_from(args: impl Iterator<Item = OsString>) -> LaunchOptions {
    let mut options = LaunchOptions::default();
    let mut args = args;

    while let Some(arg) = args.next() {
        let text = arg.to_string_lossy();

        if let Some(value) = text.strip_prefix("--working-directory=") {
            options.working_directory = normalize_launch_path(OsString::from(value));
            continue;
        }
        if text == "--working-directory" {
            if let Some(value) = args.next() {
                options.working_directory = normalize_launch_path(value);
            }
            continue;
        }
        if let Some(value) = text.strip_prefix("--execute=") {
            options.startup_command = Some(value.to_string());
            break;
        }
        if text == "--execute" || text == "-e" {
            options.startup_command = command_from_args(args);
            break;
        }
        if text == "--" {
            options.startup_command = command_from_args(args);
            break;
        }
        if text.starts_with('-') {
            continue;
        }
        if options.working_directory.is_none() {
            options.working_directory = normalize_launch_path(arg);
        }
    }

    options
}

#[cfg(windows)]
fn configure_windows_runtime_prefix() {
    use std::env;
    use std::path::Path;

    fn has_runtime_layout(prefix: &Path) -> bool {
        prefix.join("bin").is_dir()
            || prefix.join("share").is_dir()
            || prefix
                .join("share")
                .join("glib-2.0")
                .join("schemas")
                .exists()
            || prefix
                .join("lib")
                .join("gdk-pixbuf-2.0")
                .join("2.10.0")
                .join("loaders")
                .exists()
    }

    let Ok(exe_path) = env::current_exe() else {
        return;
    };
    let Some(bin_dir) = exe_path.parent() else {
        return;
    };

    let mut candidates = vec![bin_dir.to_path_buf()];
    if let Some(parent) = bin_dir.parent() {
        candidates.insert(0, parent.to_path_buf());
    }

    let Some(prefix) = candidates.into_iter().find(|candidate| has_runtime_layout(candidate))
    else {
        return;
    };

    let share_dir = prefix.join("share");
    let schema_dir = share_dir.join("glib-2.0").join("schemas");
    let runtime_bin_dir = prefix.join("bin");
    let pixbuf_dir = prefix
        .join("lib")
        .join("gdk-pixbuf-2.0")
        .join("2.10.0")
        .join("loaders");

    let mut path_entries = Vec::new();
    if runtime_bin_dir.is_dir() {
        path_entries.push(runtime_bin_dir);
    }
    if let Some(existing) = env::var_os("PATH") {
        path_entries.extend(env::split_paths(&existing));
    }

    let mut xdg_data_dirs = Vec::new();
    if share_dir.is_dir() {
        xdg_data_dirs.push(share_dir.clone());
    }
    if let Some(existing) = env::var_os("XDG_DATA_DIRS") {
        xdg_data_dirs.extend(env::split_paths(&existing));
    }

    unsafe {
        env::set_var("GTK_EXE_PREFIX", &prefix);
        env::set_var("GTK_DATA_PREFIX", &prefix);
        if schema_dir.exists() {
            env::set_var("GSETTINGS_SCHEMA_DIR", schema_dir);
        }
        if pixbuf_dir.exists() {
            env::set_var("GDK_PIXBUF_MODULEDIR", pixbuf_dir);
        }
        if let Ok(joined) = env::join_paths(path_entries) {
            env::set_var("PATH", joined);
        }
        if let Ok(joined) = env::join_paths(xdg_data_dirs) {
            env::set_var("XDG_DATA_DIRS", joined);
        }
    }
}

fn main() -> gtk::glib::ExitCode {
    #[cfg(windows)]
    configure_windows_runtime_prefix();
    #[cfg(not(windows))]
    let _ = adw::init();

    let app = gtk::Application::builder()
        .application_id(constants::APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    app.connect_startup(|_| {
        #[cfg(not(windows))]
        let _ = util::install_local_desktop_integration();
        if let Some(display) = gtk::gdk::Display::default() {
            let icon_theme = gtk::IconTheme::for_display(&display);
            for path in util::runtime_icon_search_paths() {
                icon_theme.add_search_path(path);
            }
        }
        gtk::Window::set_default_icon_name(constants::APP_ICON);
    });

    app.connect_activate(|app| {
        if app.active_window().is_none() {
            window::MainWindow::present(app, None, None);
        }
    });

    app.connect_command_line(|app, command_line| {
        let mut launch_options =
            parse_launch_options_from(command_line.arguments().into_iter().skip(1));
        if launch_options.working_directory.is_none() {
            launch_options.working_directory = command_line.cwd();
        }

        window::MainWindow::present(
            app,
            launch_options.startup_command.clone(),
            launch_options.working_directory.clone(),
        );

        gtk::glib::ExitCode::SUCCESS
    });

    app.run()
}
