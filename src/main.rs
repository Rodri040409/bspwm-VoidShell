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

    app.connect_activate(|app| {
        window::MainWindow::present(app);
    });

    app.run()
}
