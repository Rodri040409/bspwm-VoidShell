mod banner;
mod config;
mod constants;
mod context;
mod history;
mod layout;
mod preferences;
mod quick_actions;
mod system_info;
mod terminal_pane;
mod theme;
mod util;
mod window;

use adw::prelude::*;

fn main() -> gtk::glib::ExitCode {
    let app = adw::Application::builder()
        .application_id(constants::APP_ID)
        .build();

    app.connect_startup(|_| {
        if let Some(display) = gtk::gdk::Display::default() {
            let icon_theme = gtk::IconTheme::for_display(&display);
            for path in util::runtime_icon_search_paths() {
                icon_theme.add_search_path(path);
            }
        }
    });

    app.connect_activate(|app| {
        window::MainWindow::present(app);
    });

    app.run()
}
