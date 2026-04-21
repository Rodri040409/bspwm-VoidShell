use crate::config::{AppConfig, BannerInfoLayout, CursorStyle};
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct PreferenceCallbacks {
    pub on_config_changed: Rc<dyn Fn(AppConfig)>,
    pub on_reload_from_disk: Rc<dyn Fn()>,
}

pub fn build_dialog(
    parent: &gtk::ApplicationWindow,
    current_config: &AppConfig,
    callbacks: PreferenceCallbacks,
) -> gtk::Dialog {
    let shared = Rc::new(RefCell::new(current_config.clone()));
    let notify = {
        let shared = shared.clone();
        let on_change = callbacks.on_config_changed.clone();
        Rc::new(move || on_change(shared.borrow().clone()))
    };

    let dialog = gtk::Dialog::builder()
        .title("Preferencias")
        .transient_for(parent)
        .modal(true)
        .default_width(760)
        .default_height(860)
        .build();
    dialog.add_button("Cerrar", gtk::ResponseType::Close);
    dialog.connect_response(|dialog, _| dialog.close());

    let content = dialog.content_area();
    content.set_spacing(0);

    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hexpand(true);
    scroller.set_vexpand(true);
    scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    content.append(&scroller);

    let root = gtk::Box::new(gtk::Orientation::Vertical, 18);
    root.set_margin_start(18);
    root.set_margin_end(18);
    root.set_margin_top(18);
    root.set_margin_bottom(18);
    scroller.set_child(Some(&root));

    let visuals = build_section("Identidad visual", "Fondo, chrome y tipografía.");
    let runtime = build_section("Ejecución", "Inicio, animaciones y ayudas de productividad.");
    let utility = build_section("Utilidades", "Recarga de configuración desde disco.");
    root.append(&visuals.0);
    root.append(&runtime.0);
    root.append(&utility.0);

    let wallpaper_entry = gtk::Entry::new();
    wallpaper_entry.set_hexpand(true);
    wallpaper_entry.set_text(&shared.borrow().wallpaper_path.clone().unwrap_or_default());
    let browse = gtk::Button::from_icon_name("folder-open-symbolic");
    browse.add_css_class("flat");
    let wallpaper_suffix = build_setting_row(
        &visuals.1,
        "Fondo de pantalla",
        "Ruta del wallpaper compartido entre paneles.",
    );
    wallpaper_suffix.append(&wallpaper_entry);
    wallpaper_suffix.append(&browse);

    let overlay_spin = gtk::SpinButton::with_range(0.0, 0.95, 0.05);
    overlay_spin.set_value(shared.borrow().overlay_opacity);
    build_setting_row(
        &visuals.1,
        "Opacidad del overlay",
        "Valores más altos mejoran la legibilidad sobre el fondo.",
    )
    .append(&overlay_spin);

    let font_dialog = gtk::FontDialog::builder()
        .modal(true)
        .title("Seleccionar fuente de la terminal")
        .build();
    let font_button = gtk::FontDialogButton::new(Some(font_dialog));
    font_button.set_use_font(true);
    font_button.set_use_size(true);
    font_button.set_font_desc(&gtk::pango::FontDescription::from_string(&format!(
        "{} {}",
        shared.borrow().font_family,
        shared.borrow().font_size
    )));
    build_setting_row(
        &visuals.1,
        "Fuente de la terminal",
        "Familia principal para la superficie del terminal.",
    )
    .append(&font_button);

    let font_size_spin = gtk::SpinButton::with_range(8.0, 28.0, 1.0);
    font_size_spin.set_value(shared.borrow().font_size as f64);
    build_setting_row(&visuals.1, "Tamaño de fuente", "").append(&font_size_spin);

    let cursor_dropdown = gtk::DropDown::from_strings(&["Bloque", "Barra", "Subrayado"]);
    cursor_dropdown.set_selected(match shared.borrow().cursor_style {
        CursorStyle::Block => 0,
        CursorStyle::IBeam => 1,
        CursorStyle::Underline => 2,
    });
    build_setting_row(
        &visuals.1,
        "Estilo del cursor",
        "Formas de cursor compatibles con el backend activo.",
    )
    .append(&cursor_dropdown);

    let accent_button = build_color_button(
        &shared.borrow().accent_color,
        "#7dc8ff",
        "Color de acento",
        &visuals.1,
        "Se usa en badges, foco y brillo de acciones.",
    );
    let surface_button = build_color_button(
        &shared.borrow().surface_color,
        "#0d1117",
        "Color de superficie",
        &visuals.1,
        "Tinte base para chrome y fondos.",
    );
    let foreground_button = build_color_button(
        &shared.borrow().foreground_color,
        "#edf3ff",
        "Color de primer plano",
        &visuals.1,
        "Tono base del texto principal.",
    );
    let border_button = build_color_button(
        &shared.borrow().active_border_color,
        "#66bfff",
        "Color del borde activo",
        &visuals.1,
        "Línea de foco para el panel actual.",
    );

    let padding_spin = gtk::SpinButton::with_range(0.0, 40.0, 1.0);
    padding_spin.set_value(shared.borrow().panel_padding as f64);
    build_setting_row(&visuals.1, "Padding del panel", "").append(&padding_spin);

    let border_width_spin = gtk::SpinButton::with_range(1.0, 8.0, 1.0);
    border_width_spin.set_value(shared.borrow().active_border_width as f64);
    build_setting_row(&visuals.1, "Grosor del borde activo", "").append(&border_width_spin);

    let scrollback_spin = gtk::SpinButton::with_range(1000.0, 500000.0, 1000.0);
    scrollback_spin.set_value(shared.borrow().scrollback_lines as f64);
    build_setting_row(&visuals.1, "Tamaño del scrollback", "").append(&scrollback_spin);

    let shell_entry = gtk::Entry::new();
    shell_entry.set_hexpand(true);
    shell_entry.set_text(&shared.borrow().shell_path);
    build_setting_row(
        &runtime.1,
        "Ejecutable del shell",
        "Puedes apuntar a bash, zsh, fish, pwsh, PowerShell, cmd, etc.",
    )
    .append(&shell_entry);

    let banner_switch = gtk::Switch::builder()
        .active(shared.borrow().show_startup_banner)
        .build();
    build_setting_row(
        &runtime.1,
        "Banner de inicio",
        "Imprime la info del sistema en el primer panel.",
    )
    .append(&banner_switch);

    let split_banner_switch = gtk::Switch::builder()
        .active(shared.borrow().show_banner_on_new_panes)
        .build();
    build_setting_row(
        &runtime.1,
        "Banner en paneles nuevos",
        "Renderiza el banner ASCII cada vez que creas otro panel.",
    )
    .append(&split_banner_switch);

    let banner_layout_dropdown =
        gtk::DropDown::from_strings(&["A la derecha del ASCII", "Debajo del ASCII"]);
    banner_layout_dropdown.set_selected(match shared.borrow().banner_info_layout {
        BannerInfoLayout::Right => 0,
        BannerInfoLayout::Below => 1,
    });
    build_setting_row(
        &runtime.1,
        "Posición de la info del banner",
        "Puedes mantener la info a la derecha o moverla abajo.",
    )
    .append(&banner_layout_dropdown);

    let animations_switch = gtk::Switch::builder()
        .active(shared.borrow().enable_animations)
        .build();
    build_setting_row(
        &runtime.1,
        "Animaciones",
        "Activa transiciones de foco, overlay y aparición de paneles.",
    )
    .append(&animations_switch);

    let animation_speed_spin = gtk::SpinButton::with_range(0.2, 2.0, 0.1);
    animation_speed_spin.set_value(shared.borrow().animation_speed);
    build_setting_row(&runtime.1, "Velocidad de animación", "").append(&animation_speed_spin);

    let context_switch = gtk::Switch::builder()
        .active(shared.borrow().show_context_bar)
        .build();
    build_setting_row(
        &runtime.1,
        "Chrome contextual",
        "Muestra por panel la ruta, el host y las badges de modo.",
    )
    .append(&context_switch);

    let quick_actions_switch = gtk::Switch::builder()
        .active(shared.borrow().enable_quick_actions)
        .build();
    build_setting_row(
        &runtime.1,
        "Acciones rápidas",
        "Activa la paleta de comandos y el historial de acciones.",
    )
    .append(&quick_actions_switch);

    let reload_button = gtk::Button::with_label("Recargar desde disco");
    reload_button.add_css_class("suggested-action");
    build_setting_row(
        &utility.1,
        "Recargar configuración",
        "Vuelve a leer el archivo de configuración.",
    )
    .append(&reload_button);

    {
        let shared = shared.clone();
        let notify = notify.clone();
        wallpaper_entry.connect_changed(move |entry| {
            let text = entry.text().to_string();
            shared.borrow_mut().wallpaper_path = (!text.trim().is_empty()).then_some(text);
            notify();
        });
    }

    {
        let parent = parent.clone();
        let wallpaper_entry = wallpaper_entry.clone();
        browse.connect_clicked(move |_| {
            let dialog = gtk::FileDialog::builder()
                .title("Seleccionar fondo de pantalla")
                .modal(true)
                .build();
            gtk::glib::MainContext::default().spawn_local({
                let parent = parent.clone();
                let wallpaper_entry = wallpaper_entry.clone();
                async move {
                    if let Ok(file) = dialog.open_future(Some(&parent)).await {
                        if let Some(path) = file.path() {
                            wallpaper_entry.set_text(&path.display().to_string());
                        }
                    }
                }
            });
        });
    }

    connect_spin_button(
        &overlay_spin,
        shared.clone(),
        notify.clone(),
        |config, value| config.overlay_opacity = value,
    );
    connect_spin_button(
        &font_size_spin,
        shared.clone(),
        notify.clone(),
        |config, value| config.font_size = value.round() as i32,
    );
    connect_spin_button(
        &padding_spin,
        shared.clone(),
        notify.clone(),
        |config, value| config.panel_padding = value.round() as i32,
    );
    connect_spin_button(
        &border_width_spin,
        shared.clone(),
        notify.clone(),
        |config, value| config.active_border_width = value.round() as i32,
    );
    connect_spin_button(
        &scrollback_spin,
        shared.clone(),
        notify.clone(),
        |config, value| config.scrollback_lines = value.round() as i64,
    );
    connect_spin_button(
        &animation_speed_spin,
        shared.clone(),
        notify.clone(),
        |config, value| config.animation_speed = value,
    );

    connect_switch(
        &banner_switch,
        shared.clone(),
        notify.clone(),
        |config, value| config.show_startup_banner = value,
    );
    connect_switch(
        &split_banner_switch,
        shared.clone(),
        notify.clone(),
        |config, value| config.show_banner_on_new_panes = value,
    );
    connect_switch(
        &animations_switch,
        shared.clone(),
        notify.clone(),
        |config, value| config.enable_animations = value,
    );
    connect_switch(
        &context_switch,
        shared.clone(),
        notify.clone(),
        |config, value| config.show_context_bar = value,
    );
    connect_switch(
        &quick_actions_switch,
        shared.clone(),
        notify.clone(),
        |config, value| config.enable_quick_actions = value,
    );

    {
        let shared = shared.clone();
        let notify = notify.clone();
        shell_entry.connect_changed(move |entry| {
            shared.borrow_mut().shell_path = entry.text().to_string();
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

    {
        let shared = shared.clone();
        let notify = notify.clone();
        banner_layout_dropdown.connect_selected_notify(move |dropdown| {
            shared.borrow_mut().banner_info_layout = match dropdown.selected() {
                1 => BannerInfoLayout::Below,
                _ => BannerInfoLayout::Right,
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

fn build_section(title: &str, description: &str) -> (gtk::Frame, gtk::Box) {
    let frame = gtk::Frame::new(Some(title));
    frame.add_css_class("preferences-group");

    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.set_margin_start(14);
    body.set_margin_end(14);
    body.set_margin_top(14);
    body.set_margin_bottom(14);

    if !description.is_empty() {
        let description_label = gtk::Label::new(Some(description));
        description_label.add_css_class("dim-label");
        description_label.set_wrap(true);
        description_label.set_xalign(0.0);
        body.append(&description_label);
    }

    frame.set_child(Some(&body));
    (frame, body)
}

fn build_setting_row(container: &gtk::Box, title: &str, subtitle: &str) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.set_hexpand(true);

    let labels = gtk::Box::new(gtk::Orientation::Vertical, 2);
    labels.set_hexpand(true);

    let title_label = gtk::Label::new(Some(title));
    title_label.set_xalign(0.0);
    title_label.add_css_class("heading");
    labels.append(&title_label);

    if !subtitle.is_empty() {
        let subtitle_label = gtk::Label::new(Some(subtitle));
        subtitle_label.add_css_class("dim-label");
        subtitle_label.set_wrap(true);
        subtitle_label.set_xalign(0.0);
        labels.append(&subtitle_label);
    }

    let suffix = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    suffix.set_valign(gtk::Align::Center);

    row.append(&labels);
    row.append(&suffix);
    container.append(&row);

    suffix
}

fn build_color_button(
    current: &str,
    fallback: &str,
    title: &str,
    container: &gtk::Box,
    subtitle: &str,
) -> gtk::ColorDialogButton {
    let dialog = gtk::ColorDialog::builder().modal(true).title(title).build();
    let button = gtk::ColorDialogButton::new(Some(dialog));
    button.set_rgba(&crate::util::parse_rgba(current, fallback));
    build_setting_row(container, title, subtitle).append(&button);
    button
}

fn connect_spin_button<F: Fn(&mut AppConfig, f64) + 'static>(
    button: &gtk::SpinButton,
    shared: Rc<RefCell<AppConfig>>,
    notify: Rc<dyn Fn()>,
    update: F,
) {
    button.connect_value_changed(move |spin| {
        update(&mut shared.borrow_mut(), spin.value());
        notify();
    });
}

fn connect_switch<F: Fn(&mut AppConfig, bool) + 'static>(
    widget: &gtk::Switch,
    shared: Rc<RefCell<AppConfig>>,
    notify: Rc<dyn Fn()>,
    update: F,
) {
    widget.connect_active_notify(move |switch| {
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
