mod banner;
mod config;
mod constants;
mod context;
mod history;
mod layout;
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

use adw::prelude::*;
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

fn parse_launch_options() -> LaunchOptions {
    let mut options = LaunchOptions::default();
    let mut args = std::env::args_os().skip(1);

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

    let Ok(exe_path) = env::current_exe() else {
        return;
    };
    let Some(bin_dir) = exe_path.parent() else {
        return;
    };
    let Some(prefix) = bin_dir.parent() else {
        return;
    };

    let share_dir = prefix.join("share");
    let schema_dir = share_dir.join("glib-2.0").join("schemas");
    let pixbuf_dir = prefix
        .join("lib")
        .join("gdk-pixbuf-2.0")
        .join("2.10.0")
        .join("loaders");

    let mut xdg_data_dirs = vec![share_dir.clone()];
    if let Some(existing) = env::var_os("XDG_DATA_DIRS") {
        xdg_data_dirs.extend(env::split_paths(&existing));
    }

    unsafe {
        env::set_var("GTK_EXE_PREFIX", prefix);
        env::set_var("GTK_DATA_PREFIX", prefix);
        env::set_var("GSETTINGS_SCHEMA_DIR", schema_dir);
        env::set_var("GDK_PIXBUF_MODULEDIR", pixbuf_dir);
        if let Ok(joined) = env::join_paths(xdg_data_dirs) {
            env::set_var("XDG_DATA_DIRS", joined);
        }
    }
}

fn main() -> gtk::glib::ExitCode {
    #[cfg(windows)]
    configure_windows_runtime_prefix();

    let launch_options = parse_launch_options();
    if let Some(working_directory) = &launch_options.working_directory {
        let _ = std::env::set_current_dir(working_directory);
    }

    let app = adw::Application::builder()
        .application_id(constants::APP_ID)
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

    app.connect_activate(move |app| {
        window::MainWindow::present(app, launch_options.startup_command.clone());
    });

    app.run()
}
