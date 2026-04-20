use crate::config::{AppConfig, CursorStyle};
use adw::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct PreferenceCallbacks {
    pub on_config_changed: Rc<dyn Fn(AppConfig)>,
    pub on_reload_from_disk: Rc<dyn Fn()>,
}

pub fn build_dialog(
    parent: &adw::ApplicationWindow,
    current_config: &AppConfig,
    callbacks: PreferenceCallbacks,
) -> adw::PreferencesDialog {
    let shared = Rc::new(RefCell::new(current_config.clone()));
    let notify = {
        let shared = shared.clone();
        let on_change = callbacks.on_config_changed.clone();
        Rc::new(move || on_change(shared.borrow().clone()))
    };

    let dialog = adw::PreferencesDialog::new();
    dialog.set_title("Preferences");
    dialog.set_search_enabled(true);

    let appearance = adw::PreferencesPage::builder()
        .name("appearance")
        .title("Appearance")
        .icon_name("preferences-desktop-theme-symbolic")
        .build();

    let visuals = adw::PreferencesGroup::builder()
        .title("Visual Identity")
        .description("Wallpaper, chrome and typography.")
        .build();

    let wallpaper_row = adw::EntryRow::builder()
        .title("Wallpaper")
        .text(shared.borrow().wallpaper_path.clone().unwrap_or_default())
        .build();
    let browse = gtk::Button::from_icon_name("folder-open-symbolic");
    browse.add_css_class("flat");
    wallpaper_row.add_suffix(&browse);
    visuals.add(&wallpaper_row);

    let overlay_row = adw::SpinRow::with_range(0.0, 0.95, 0.05);
    overlay_row.set_title("Dark overlay opacity");
    overlay_row.set_subtitle("Higher values improve readability over wallpaper.");
    overlay_row.set_value(shared.borrow().overlay_opacity);
    visuals.add(&overlay_row);

    let font_dialog = gtk::FontDialog::builder()
        .modal(true)
        .title("Select terminal font")
        .build();
    let font_button = gtk::FontDialogButton::new(Some(font_dialog));
    font_button.set_use_font(true);
    font_button.set_use_size(true);
    font_button.set_font_desc(&gtk::pango::FontDescription::from_string(&format!(
        "{} {}",
        shared.borrow().font_family,
        shared.borrow().font_size
    )));
    let font_row = adw::ActionRow::builder()
        .title("Terminal font")
        .subtitle("Distinctive monospace family for the VTE surface.")
        .build();
    font_row.add_suffix(&font_button);
    visuals.add(&font_row);

    let font_size_row = adw::SpinRow::with_range(8.0, 28.0, 1.0);
    font_size_row.set_title("Font size");
    font_size_row.set_value(shared.borrow().font_size as f64);
    visuals.add(&font_size_row);

    let cursor_row = adw::ActionRow::builder()
        .title("Cursor style")
        .subtitle("VTE-supported cursor shapes.")
        .build();
    let cursor_dropdown = gtk::DropDown::from_strings(&["Block", "I-Beam", "Underline"]);
    cursor_dropdown.set_selected(match shared.borrow().cursor_style {
        CursorStyle::Block => 0,
        CursorStyle::IBeam => 1,
        CursorStyle::Underline => 2,
    });
    cursor_row.add_suffix(&cursor_dropdown);
    cursor_row.set_activatable_widget(Some(&cursor_dropdown));
    visuals.add(&cursor_row);

    let accent_dialog = gtk::ColorDialog::builder()
        .modal(true)
        .title("Accent color")
        .build();
    let accent_button = gtk::ColorDialogButton::new(Some(accent_dialog));
    accent_button.set_rgba(&crate::util::parse_rgba(
        &shared.borrow().accent_color,
        "#7dc8ff",
    ));
    let accent_row = adw::ActionRow::builder()
        .title("Accent color")
        .subtitle("Used in badges, cursor and action glow.")
        .build();
    accent_row.add_suffix(&accent_button);
    visuals.add(&accent_row);

    let surface_dialog = gtk::ColorDialog::builder()
        .modal(true)
        .title("Surface color")
        .build();
    let surface_button = gtk::ColorDialogButton::new(Some(surface_dialog));
    surface_button.set_rgba(&crate::util::parse_rgba(
        &shared.borrow().surface_color,
        "#0d1117",
    ));
    let surface_row = adw::ActionRow::builder()
        .title("Surface color")
        .subtitle("Backdrop tint for chrome and terminal surfaces.")
        .build();
    surface_row.add_suffix(&surface_button);
    visuals.add(&surface_row);

    let foreground_dialog = gtk::ColorDialog::builder()
        .modal(true)
        .title("Foreground color")
        .build();
    let foreground_button = gtk::ColorDialogButton::new(Some(foreground_dialog));
    foreground_button.set_rgba(&crate::util::parse_rgba(
        &shared.borrow().foreground_color,
        "#edf3ff",
    ));
    let foreground_row = adw::ActionRow::builder()
        .title("Foreground color")
        .subtitle("Base terminal foreground tone.")
        .build();
    foreground_row.add_suffix(&foreground_button);
    visuals.add(&foreground_row);

    let border_dialog = gtk::ColorDialog::builder()
        .modal(true)
        .title("Active border color")
        .build();
    let border_button = gtk::ColorDialogButton::new(Some(border_dialog));
    border_button.set_rgba(&crate::util::parse_rgba(
        &shared.borrow().active_border_color,
        "#66bfff",
    ));
    let border_row = adw::ActionRow::builder()
        .title("Active border color")
        .subtitle("Focus line for the current pane.")
        .build();
    border_row.add_suffix(&border_button);
    visuals.add(&border_row);

    let metrics = adw::PreferencesGroup::builder()
        .title("Density")
        .description("Scrollback, padding and layout chrome.")
        .build();

    let padding_row = adw::SpinRow::with_range(0.0, 40.0, 1.0);
    padding_row.set_title("Pane padding");
    padding_row.set_value(shared.borrow().panel_padding as f64);
    metrics.add(&padding_row);

    let border_width_row = adw::SpinRow::with_range(1.0, 8.0, 1.0);
    border_width_row.set_title("Active border width");
    border_width_row.set_value(shared.borrow().active_border_width as f64);
    metrics.add(&border_width_row);

    let scrollback_row = adw::SpinRow::with_range(1000.0, 500000.0, 1000.0);
    scrollback_row.set_title("Scrollback size");
    scrollback_row.set_value(shared.borrow().scrollback_lines as f64);
    metrics.add(&scrollback_row);

    appearance.add(&visuals);
    appearance.add(&metrics);
    dialog.add(&appearance);

    let behavior = adw::PreferencesPage::builder()
        .name("behavior")
        .title("Behavior")
        .icon_name("preferences-system-symbolic")
        .build();

    let runtime = adw::PreferencesGroup::builder()
        .title("Runtime")
        .description("Startup, animation and productivity helpers.")
        .build();

    let shell_row = adw::EntryRow::builder()
        .title("Shell executable")
        .text(shared.borrow().shell_path.clone())
        .build();
    shell_row.set_show_apply_button(true);
    shell_row.set_input_hints(gtk::InputHints::NO_SPELLCHECK);
    shell_row.set_input_purpose(gtk::InputPurpose::Url);
    shell_row.set_tooltip_text(Some(
        "Leave empty to auto-detect. You can point this to bash, zsh, fish, pwsh, PowerShell, cmd, etc.",
    ));
    runtime.add(&shell_row);

    let banner_row = adw::SwitchRow::builder()
        .title("Startup banner")
        .subtitle("Print system info plus the launch banner on the first pane.")
        .active(shared.borrow().show_startup_banner)
        .build();
    runtime.add(&banner_row);

    let animations_row = adw::SwitchRow::builder()
        .title("Animations")
        .subtitle("Enable focus, overlay and pane reveal transitions.")
        .active(shared.borrow().enable_animations)
        .build();
    runtime.add(&animations_row);

    let animation_speed_row = adw::SpinRow::with_range(0.2, 2.0, 0.1);
    animation_speed_row.set_title("Animation speed");
    animation_speed_row.set_value(shared.borrow().animation_speed);
    runtime.add(&animation_speed_row);

    let context_row = adw::SwitchRow::builder()
        .title("Context chrome")
        .subtitle("Show per-pane path, host and mode badges.")
        .active(shared.borrow().show_context_bar)
        .build();
    runtime.add(&context_row);

    let quick_actions_row = adw::SwitchRow::builder()
        .title("Quick actions")
        .subtitle("Enable the command palette and action history.")
        .active(shared.borrow().enable_quick_actions)
        .build();
    runtime.add(&quick_actions_row);

    let reload_row = adw::ActionRow::builder()
        .title("Reload configuration")
        .subtitle("Re-read the config file from disk.")
        .build();
    let reload_button = gtk::Button::with_label("Reload");
    reload_button.add_css_class("suggested-action");
    reload_row.add_suffix(&reload_button);
    let utilities = adw::PreferencesGroup::builder().title("Utilities").build();
    utilities.add(&reload_row);
    behavior.add(&runtime);
    behavior.add(&utilities);
    dialog.add(&behavior);

    {
        let shared = shared.clone();
        let notify = notify.clone();
        wallpaper_row.connect_changed(move |row| {
            let text = row.text().to_string();
            shared.borrow_mut().wallpaper_path = (!text.trim().is_empty()).then_some(text);
            notify();
        });
    }

    {
        let parent = parent.clone();
        let wallpaper_row = wallpaper_row.clone();
        browse.connect_clicked(move |_| {
            let dialog = gtk::FileDialog::builder()
                .title("Select wallpaper")
                .modal(true)
                .build();
            gtk::glib::MainContext::default().spawn_local({
                let parent = parent.clone();
                let wallpaper_row = wallpaper_row.clone();
                async move {
                    if let Ok(file) = dialog.open_future(Some(&parent)).await {
                        if let Some(path) = file.path() {
                            wallpaper_row.set_text(&path.display().to_string());
                        }
                    }
                }
            });
        });
    }

    connect_spin_row(
        &overlay_row,
        shared.clone(),
        notify.clone(),
        |config, value| {
            config.overlay_opacity = value;
        },
    );
    connect_spin_row(
        &font_size_row,
        shared.clone(),
        notify.clone(),
        |config, value| {
            config.font_size = value.round() as i32;
        },
    );
    connect_spin_row(
        &padding_row,
        shared.clone(),
        notify.clone(),
        |config, value| {
            config.panel_padding = value.round() as i32;
        },
    );
    connect_spin_row(
        &border_width_row,
        shared.clone(),
        notify.clone(),
        |config, value| {
            config.active_border_width = value.round() as i32;
        },
    );
    connect_spin_row(
        &scrollback_row,
        shared.clone(),
        notify.clone(),
        |config, value| {
            config.scrollback_lines = value.round() as i64;
        },
    );
    connect_spin_row(
        &animation_speed_row,
        shared.clone(),
        notify.clone(),
        |config, value| {
            config.animation_speed = value;
        },
    );

    connect_switch_row(
        &banner_row,
        shared.clone(),
        notify.clone(),
        |config, value| {
            config.show_startup_banner = value;
        },
    );
    connect_switch_row(
        &animations_row,
        shared.clone(),
        notify.clone(),
        |config, value| {
            config.enable_animations = value;
        },
    );
    connect_switch_row(
        &context_row,
        shared.clone(),
        notify.clone(),
        |config, value| {
            config.show_context_bar = value;
        },
    );
    connect_switch_row(
        &quick_actions_row,
        shared.clone(),
        notify.clone(),
        |config, value| {
            config.enable_quick_actions = value;
        },
    );

    {
        let shared = shared.clone();
        let notify = notify.clone();
        shell_row.connect_apply(move |row| {
            shared.borrow_mut().shell_path = row.text().to_string();
            notify();
        });
    }

    {
        let shared = shared.clone();
        let notify = notify.clone();
        font_button.connect_font_desc_notify(move |button| {
            if let Some(description) = button.font_desc() {
                if let Some(family) = description.family() {
                    shared.borrow_mut().font_family = family.to_string();
                }
                let size = description.size() / gtk::pango::SCALE;
                if size > 0 {
                    shared.borrow_mut().font_size = size;
                }
                notify();
            }
        });
    }

    {
        let shared = shared.clone();
        let notify = notify.clone();
        cursor_dropdown.connect_selected_notify(move |dropdown| {
            shared.borrow_mut().cursor_style = match dropdown.selected() {
                1 => CursorStyle::IBeam,
                2 => CursorStyle::Underline,
                _ => CursorStyle::Block,
            };
            notify();
        });
    }

    connect_color_button(
        &accent_button,
        shared.clone(),
        notify.clone(),
        |config, value| config.accent_color = value,
    );
    connect_color_button(
        &surface_button,
        shared.clone(),
        notify.clone(),
        |config, value| config.surface_color = value,
    );
    connect_color_button(
        &foreground_button,
        shared.clone(),
        notify.clone(),
        |config, value| config.foreground_color = value,
    );
    connect_color_button(
        &border_button,
        shared.clone(),
        notify.clone(),
        |config, value| config.active_border_color = value,
    );

    {
        let callback = callbacks.on_reload_from_disk.clone();
        reload_button.connect_clicked(move |_| callback());
    }

    dialog
}

fn connect_spin_row<F: Fn(&mut AppConfig, f64) + 'static>(
    row: &adw::SpinRow,
    shared: Rc<RefCell<AppConfig>>,
    notify: Rc<dyn Fn()>,
    update: F,
) {
    row.connect_value_notify(move |spin| {
        update(&mut shared.borrow_mut(), spin.value());
        notify();
    });
}

fn connect_switch_row<F: Fn(&mut AppConfig, bool) + 'static>(
    row: &adw::SwitchRow,
    shared: Rc<RefCell<AppConfig>>,
    notify: Rc<dyn Fn()>,
    update: F,
) {
    row.connect_active_notify(move |switch| {
        update(&mut shared.borrow_mut(), switch.is_active());
        notify();
    });
}

fn connect_color_button<F: Fn(&mut AppConfig, String) + 'static>(
    button: &gtk::ColorDialogButton,
    shared: Rc<RefCell<AppConfig>>,
    notify: Rc<dyn Fn()>,
    update: F,
) {
    button.connect_rgba_notify(move |button| {
        update(&mut shared.borrow_mut(), button.rgba().to_string());
        notify();
    });
}
