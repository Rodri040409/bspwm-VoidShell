use crate::banner;
use crate::config::{AppConfig, BannerInfoLayout};
use crate::constants;
use crate::context::{self, PanelContext, PanelMode};
use crate::theme;
use crate::util;
use gtk::glib;
use gtk::prelude::*;
use std::cell::{Cell, RefCell};
#[cfg(unix)]
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;
use vte::prelude::*;

#[derive(Clone)]
pub struct PaneCallbacks {
    pub on_focus: Rc<dyn Fn(u64)>,
    pub on_context_changed: Rc<dyn Fn(u64, PanelContext)>,
    pub on_exit: Rc<dyn Fn(u64, i32)>,
    pub on_notification: Rc<dyn Fn(String)>,
    pub on_swap_request: Rc<dyn Fn(u64, u64)>,
    pub on_toggle_zoom: Rc<dyn Fn(u64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneSpawnMotion {
    Center,
    FromLeft,
    FromRight,
    FromTop,
    FromBottom,
}

pub struct TerminalPane {
    id: u64,
    revealer: gtk::Revealer,
    shell_box: gtk::Box,
    terminal_wrap: gtk::Box,
    terminal: vte::Terminal,
    background: gtk::Picture,
    ambient: gtk::Box,
    tint: gtk::Box,
    chrome: gtk::Box,
    title_label: gtk::Label,
    subtitle_label: gtk::Label,
    badge_box: gtk::Box,
    callbacks: PaneCallbacks,
    shell_path: String,
    child_pid: Cell<Option<glib::Pid>>,
    pending_commands: RefCell<Vec<String>>,
    context: RefCell<PanelContext>,
    context_tick: Cell<u64>,
    output_pulse_pending: Cell<bool>,
    is_active: Cell<bool>,
    compact_mode: Cell<bool>,
    dense_mode: Cell<bool>,
    wallpaper_hint: Cell<bool>,
    wallpaper_available: Cell<bool>,
    show_context_bar: Cell<bool>,
    banner_layout: Cell<BannerInfoLayout>,
    palette_preset: Cell<Option<theme::PanePalettePreset>>,
}

impl TerminalPane {
    pub fn new(
        id: u64,
        shell_path: String,
        working_directory: Option<PathBuf>,
        show_banner: bool,
        spawn_motion: PaneSpawnMotion,
        config: &AppConfig,
        callbacks: PaneCallbacks,
    ) -> Rc<Self> {
        let revealer = gtk::Revealer::builder()
            .transition_type(if config.enable_animations {
                gtk::RevealerTransitionType::Crossfade
            } else {
                gtk::RevealerTransitionType::None
            })
            .transition_duration((160.0 / config.animation_speed.max(0.2)) as u32)
            .build();

        let shell_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        shell_box.add_css_class("terminal-pane-shell");
        shell_box.set_hexpand(true);
        shell_box.set_vexpand(true);

        let overlay = gtk::Overlay::new();
        overlay.set_hexpand(true);
        overlay.set_vexpand(true);
        shell_box.append(&overlay);

        let background = gtk::Picture::new();
        background.set_can_shrink(true);
        background.set_content_fit(gtk::ContentFit::Cover);
        background.set_hexpand(true);
        background.set_vexpand(true);
        background.add_css_class("pane-wallpaper");
        overlay.set_child(Some(&background));

        let ambient = gtk::Box::new(gtk::Orientation::Vertical, 0);
        ambient.add_css_class("pane-ambient");
        ambient.set_hexpand(true);
        ambient.set_vexpand(true);
        ambient.set_can_target(false);
        overlay.add_overlay(&ambient);

        let tint = gtk::Box::new(gtk::Orientation::Vertical, 0);
        tint.add_css_class("pane-tint");
        tint.set_hexpand(true);
        tint.set_vexpand(true);
        tint.set_can_target(false);
        overlay.add_overlay(&tint);

        let terminal_wrap = gtk::Box::new(gtk::Orientation::Vertical, 0);
        terminal_wrap.add_css_class("pane-terminal-wrap");
        terminal_wrap.set_hexpand(true);
        terminal_wrap.set_vexpand(true);
        overlay.add_overlay(&terminal_wrap);

        let terminal = vte::Terminal::new();
        terminal.add_css_class("pane-terminal");
        terminal.set_hexpand(true);
        terminal.set_vexpand(true);
        terminal.set_scroll_on_output(false);
        terminal.set_scroll_on_keystroke(true);
        terminal.set_mouse_autohide(true);
        terminal.set_allow_hyperlink(true);
        terminal.set_audible_bell(false);
        terminal.set_bold_is_bright(true);
        terminal.set_enable_shaping(true);
        terminal_wrap.append(&terminal);

        let chrome = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        chrome.add_css_class("pane-chrome");
        chrome.set_halign(gtk::Align::Fill);
        chrome.set_hexpand(true);
        chrome.set_can_target(true);
        shell_box.prepend(&chrome);

        let title_stack = gtk::Box::new(gtk::Orientation::Vertical, 2);
        title_stack.set_hexpand(true);
        title_stack.set_can_target(false);
        let title_label = gtk::Label::new(Some("shell"));
        title_label.add_css_class("pane-title");
        title_label.set_xalign(0.0);
        title_label.set_hexpand(true);
        title_label.set_single_line_mode(true);
        title_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        title_label.set_can_target(false);
        let subtitle_label = gtk::Label::new(Some(""));
        subtitle_label.add_css_class("pane-subtitle");
        subtitle_label.set_xalign(0.0);
        subtitle_label.set_visible(false);
        subtitle_label.set_can_target(false);
        title_stack.append(&title_label);
        chrome.append(&title_stack);

        let badge_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        badge_box.set_valign(gtk::Align::Center);
        badge_box.set_visible(false);
        badge_box.set_can_target(false);
        chrome.append(&badge_box);

        revealer.set_child(Some(&shell_box));

        let pane = Rc::new(Self {
            id,
            revealer,
            shell_box,
            terminal_wrap,
            terminal,
            background,
            ambient,
            tint,
            chrome,
            title_label,
            subtitle_label,
            badge_box,
            callbacks,
            shell_path,
            child_pid: Cell::new(None),
            pending_commands: RefCell::new(Vec::new()),
            context: RefCell::new(PanelContext::default()),
            context_tick: Cell::new(0),
            output_pulse_pending: Cell::new(false),
            is_active: Cell::new(false),
            compact_mode: Cell::new(false),
            dense_mode: Cell::new(false),
            wallpaper_hint: Cell::new(true),
            wallpaper_available: Cell::new(false),
            show_context_bar: Cell::new(true),
            banner_layout: Cell::new(BannerInfoLayout::Right),
            palette_preset: Cell::new(None),
        });

        pane.apply_config(config);
        pane.install_focus_handlers();
        pane.install_drag_handlers();
        pane.install_runtime_handlers();
        pane.spawn_shell(working_directory, show_banner);
        pane.animate_open(config, spawn_motion);
        pane.schedule_context_refresh();

        let reveal = pane.revealer.clone();
        gtk::glib::idle_add_local_once(move || {
            reveal.set_reveal_child(true);
        });

        pane
    }

    pub fn widget(&self) -> gtk::Widget {
        self.revealer.clone().upcast()
    }

    pub fn terminal(&self) -> &vte::Terminal {
        &self.terminal
    }

    pub fn context(&self) -> PanelContext {
        self.context.borrow().clone()
    }

    pub fn current_directory(&self) -> Option<PathBuf> {
        self.context.borrow().cwd.clone()
    }

    pub fn detach_from_parent(&self) {
        let Some(parent) = self.revealer.parent() else {
            return;
        };

        if let Ok(paned) = parent.clone().downcast::<gtk::Paned>() {
            let revealer_widget: gtk::Widget = self.revealer.clone().upcast();
            let revealer_ptr = revealer_widget.as_ptr();

            if paned
                .start_child()
                .as_ref()
                .is_some_and(|child| child.as_ptr() == revealer_ptr)
            {
                paned.set_start_child(Option::<&gtk::Widget>::None);
            }

            if paned
                .end_child()
                .as_ref()
                .is_some_and(|child| child.as_ptr() == revealer_ptr)
            {
                paned.set_end_child(Option::<&gtk::Widget>::None);
            }

            return;
        }

        if let Ok(container) = parent.downcast::<gtk::Box>() {
            container.remove(&self.revealer);
        }
    }

    pub fn focus_terminal(&self) {
        self.terminal.grab_focus();
    }

    pub fn show_banner_info(&self) {
        self.render_banner(true, true);
        self.focus_terminal();
    }

    pub fn begin_close_animation(&self, config: &AppConfig, motion: PaneSpawnMotion) {
        self.clear_spawn_motion_classes();
        self.clear_close_motion_classes();

        let kick_class = motion.close_kick_css_class();
        let exit_class = motion.close_css_class();
        self.shell_box.add_css_class(kick_class);

        let shell_box = self.shell_box.clone();
        let kick_delay = ((44.0 / config.animation_speed.max(0.2)) as u64).max(1);
        gtk::glib::timeout_add_local_once(Duration::from_millis(kick_delay), move || {
            shell_box.remove_css_class(kick_class);
            shell_box.add_css_class("pane-closing");
            shell_box.add_css_class(exit_class);
        });
    }

    pub fn close_animation_duration_ms(&self, config: &AppConfig) -> u64 {
        ((300.0 / config.animation_speed.max(0.2)) as u64).max(1)
    }

    pub fn set_active(&self, active: bool) {
        self.is_active.set(active);
        if active {
            self.shell_box.add_css_class("active");
        } else {
            self.shell_box.remove_css_class("active");
        }
        self.refresh_density_visuals();
    }

    pub fn run_command(&self, command: &str) {
        let mut payload = command.to_string();
        if !payload.ends_with('\n') {
            payload.push('\n');
        }

        if self.child_pid.get().is_some() {
            self.terminal.feed_child(payload.as_bytes());
        } else {
            self.pending_commands.borrow_mut().push(payload);
        }

        self.flash_action();
    }

    pub fn apply_config(&self, config: &AppConfig) {
        theme::install_or_update(config);

        self.terminal_wrap.set_margin_top(config.panel_padding);
        self.terminal_wrap.set_margin_bottom(config.panel_padding);
        self.terminal_wrap.set_margin_start(config.panel_padding);
        self.terminal_wrap.set_margin_end(config.panel_padding);

        self.show_context_bar.set(config.show_context_bar);
        self.banner_layout.set(config.banner_info_layout);
        self.chrome.set_visible(config.show_context_bar);
        self.tint
            .set_opacity(config.overlay_opacity.clamp(0.0, 0.95));
        self.revealer
            .set_transition_duration((160.0 / config.animation_speed.max(0.2)) as u32);
        self.revealer
            .set_transition_type(if config.enable_animations {
                gtk::RevealerTransitionType::Crossfade
            } else {
                gtk::RevealerTransitionType::None
            });

        self.wallpaper_available.set(false);
        self.background
            .set_paintable(Option::<&gtk::gdk::Texture>::None);
        if let Some(path) = config
            .wallpaper_path
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            && let Some(texture) = util::cached_wallpaper_texture(path)
        {
            self.background.set_paintable(Some(&texture));
            self.wallpaper_available.set(true);
        }

        let font = theme::font_description(config);
        self.terminal.set_font(Some(&font));
        self.terminal.set_scrollback_lines(config.scrollback_lines);
        self.terminal
            .set_cursor_shape(theme::cursor_shape(&config.cursor_style));

        self.apply_palette_classes();
        let palette = theme::terminal_palette(config, self.palette_preset.get());
        let palette_refs: Vec<&gtk::gdk::RGBA> = palette.palette.iter().collect();
        self.terminal.set_colors(
            Some(&palette.foreground),
            Some(&palette.background),
            &palette_refs,
        );
        self.terminal.set_color_cursor(Some(&palette.cursor));
        self.terminal
            .set_color_cursor_foreground(Some(&palette.cursor_text));
        self.terminal.set_color_highlight(Some(&palette.cursor));
        self.terminal
            .set_color_highlight_foreground(Some(&palette.cursor_text));
        self.refresh_density_visuals();
    }

    pub fn set_density(&self, compact: bool, dense: bool, wallpaper_visible: bool) {
        self.compact_mode.set(compact);
        self.dense_mode.set(dense);
        self.wallpaper_hint.set(wallpaper_visible);
        self.refresh_density_visuals();
    }

    pub fn set_palette_preset(&self, preset: Option<theme::PanePalettePreset>, config: &AppConfig) {
        self.palette_preset.set(preset);
        self.apply_config(config);
    }

    fn install_focus_handlers(self: &Rc<Self>) {
        let click = gtk::GestureClick::new();
        let focus_callback = self.callbacks.on_focus.clone();
        let pane_id = self.id;
        let terminal = self.terminal.clone();
        click.connect_pressed(move |_, _, _, _| {
            terminal.grab_focus();
            focus_callback(pane_id);
        });
        self.shell_box.add_controller(click);

        let focus_callback = self.callbacks.on_focus.clone();
        let pane_id = self.id;
        self.terminal.connect_has_focus_notify(move |terminal| {
            if terminal.has_focus() {
                focus_callback(pane_id);
            }
        });
    }

    fn install_drag_handlers(self: &Rc<Self>) {
        let drag_source = gtk::DragSource::new();
        drag_source.set_actions(gtk::gdk::DragAction::MOVE);
        let pane_id = self.id;
        let focus_callback = self.callbacks.on_focus.clone();
        drag_source.connect_prepare(move |_, _, _| {
            focus_callback(pane_id);
            Some(gtk::gdk::ContentProvider::for_value(&pane_id.to_value()))
        });

        let shell_box = self.shell_box.clone();
        drag_source.connect_drag_begin(move |_, _| {
            shell_box.add_css_class("drag-source");
        });

        let shell_box = self.shell_box.clone();
        drag_source.connect_drag_end(move |_, _, _| {
            shell_box.remove_css_class("drag-source");
        });
        self.chrome.add_controller(drag_source);

        let drop_target = gtk::DropTarget::new(u64::static_type(), gtk::gdk::DragAction::MOVE);
        let shell_box = self.shell_box.clone();
        drop_target.connect_enter(move |_, _, _| {
            shell_box.add_css_class("drop-target");
            gtk::gdk::DragAction::MOVE
        });

        let shell_box = self.shell_box.clone();
        drop_target.connect_leave(move |_| {
            shell_box.remove_css_class("drop-target");
        });

        let pane_id = self.id;
        let swap_callback = self.callbacks.on_swap_request.clone();
        let shell_box = self.shell_box.clone();
        drop_target.connect_drop(move |_, value, _, _| {
            shell_box.remove_css_class("drop-target");
            let Ok(source_id) = value.get::<u64>() else {
                return false;
            };
            if source_id != pane_id {
                swap_callback(source_id, pane_id);
            }
            true
        });
        self.shell_box.add_controller(drop_target);

        let click = gtk::GestureClick::new();
        let pane_id = self.id;
        let zoom_callback = self.callbacks.on_toggle_zoom.clone();
        click.connect_pressed(move |_, n_press, _, _| {
            if n_press == 2 {
                zoom_callback(pane_id);
            }
        });
        self.chrome.add_controller(click);
    }

    fn install_runtime_handlers(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.terminal
            .connect_notify_local(Some("current-directory-uri"), move |_, _| {
                if let Some(pane) = weak.upgrade() {
                    pane.refresh_context();
                }
            });

        let weak = Rc::downgrade(self);
        self.terminal.connect_contents_changed(move |_| {
            if let Some(pane) = weak.upgrade() {
                pane.pulse_output();
            }
        });

        let weak = Rc::downgrade(self);
        self.terminal.connect_child_exited(move |_, status| {
            if let Some(pane) = weak.upgrade() {
                pane.shell_box.remove_css_class("mode-editor");
                pane.shell_box.remove_css_class("mode-monitor");
                pane.shell_box.remove_css_class("mode-remote");
                pane.child_pid.set(None);
                pane.refresh_context();
                if status != 0 {
                    (pane.callbacks.on_notification)(format!(
                        "Pane {} exited with status {}",
                        pane.id, status
                    ));
                }
                (pane.callbacks.on_exit)(pane.id, status);
            }
        });
    }

    fn schedule_context_refresh(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        let delay = self.context_refresh_delay_ms();
        gtk::glib::timeout_add_local_once(Duration::from_millis(delay), move || {
            if let Some(pane) = weak.upgrade() {
                pane.refresh_context();
                pane.schedule_context_refresh();
            }
        });
    }

    fn spawn_shell(self: &Rc<Self>, working_directory: Option<PathBuf>, show_banner: bool) {
        let env_strings = util::envv(&self.shell_path);
        let env_refs: Vec<&str> = env_strings.iter().map(String::as_str).collect();
        let shell_args = util::default_shell_args(&self.shell_path);
        let mut argv_strings = vec![self.shell_path.clone()];
        argv_strings.extend(shell_args);
        let argv_refs: Vec<&str> = argv_strings.iter().map(String::as_str).collect();
        let working_directory_string = working_directory
            .as_ref()
            .map(|path| path.display().to_string());

        let weak = Rc::downgrade(self);
        self.terminal.spawn_async(
            vte::PtyFlags::DEFAULT,
            working_directory_string.as_deref(),
            &argv_refs,
            &env_refs,
            gtk::glib::SpawnFlags::DEFAULT,
            || {},
            -1,
            None::<&gtk::gio::Cancellable>,
            move |result| {
                if let Some(pane) = weak.upgrade() {
                    match result {
                        Ok(pid) => {
                            pane.child_pid.set(Some(pid));
                            if show_banner {
                                pane.render_banner(false, false);
                            }
                            let mut commands = pane.pending_commands.borrow_mut();
                            for command in commands.drain(..) {
                                pane.terminal.feed_child(command.as_bytes());
                            }
                            pane.refresh_context();
                        }
                        Err(error) => {
                            (pane.callbacks.on_notification)(format!(
                                "Failed to spawn pane {}: {error}",
                                pane.id
                            ));
                        }
                    }
                }
            },
        );
    }

    fn refresh_context(&self) {
        #[cfg(unix)]
        let pty_fd = self.terminal.pty().map(|pty| pty.fd().as_raw_fd());
        #[cfg(not(unix))]
        let pty_fd = Option::<i32>::None;
        let mut next = context::detect_panel_context(
            self.child_pid.get().map(|pid| pid.0),
            pty_fd,
            &self.shell_path,
        );

        let tick = self.context_tick.get() + 1;
        self.context_tick.set(tick);
        let previous = self.context.borrow().clone();
        let should_refresh_git =
            previous.cwd != next.cwd || (self.is_active.get() && tick % 4 == 0) || tick % 12 == 0;
        next.git_branch = if should_refresh_git {
            next.cwd.as_deref().and_then(context::detect_git_branch)
        } else {
            previous.git_branch.clone()
        };

        self.render_context(&next);

        if previous != next {
            *self.context.borrow_mut() = next.clone();
            (self.callbacks.on_context_changed)(self.id, next);
        }
    }

    fn render_context(&self, context: &PanelContext) {
        self.title_label.set_text(&context.header_title());
        self.subtitle_label.set_text(&context.header_subtitle());
        self.chrome
            .set_tooltip_text(Some(&context.header_subtitle()));
        self.title_label
            .set_tooltip_text(Some(&context.header_title()));

        while let Some(child) = self.badge_box.first_child() {
            self.badge_box.remove(&child);
        }

        for (label, accent) in context.badges() {
            let badge = gtk::Label::new(Some(&label));
            badge.add_css_class("context-badge");
            if accent {
                badge.add_css_class("accent");
            }
            self.badge_box.append(&badge);
        }
        self.badge_box
            .set_visible(self.badge_box.first_child().is_some());

        self.shell_box.remove_css_class("mode-editor");
        self.shell_box.remove_css_class("mode-monitor");
        self.shell_box.remove_css_class("mode-remote");

        match context.mode {
            PanelMode::Editor => self.shell_box.add_css_class("mode-editor"),
            PanelMode::Monitor => self.shell_box.add_css_class("mode-monitor"),
            PanelMode::Remote => self.shell_box.add_css_class("mode-remote"),
            PanelMode::Shell | PanelMode::Container | PanelMode::Exited => {}
        }

        self.refresh_density_visuals();
    }

    fn flash_action(&self) {
        self.shell_box.add_css_class("action-flash");
        let shell_box = self.shell_box.clone();
        gtk::glib::timeout_add_local_once(Duration::from_millis(380), move || {
            shell_box.remove_css_class("action-flash");
        });
    }

    fn animate_open(&self, config: &AppConfig, motion: PaneSpawnMotion) {
        self.shell_box.add_css_class("pane-born");
        self.clear_spawn_motion_classes();
        self.clear_close_motion_classes();
        self.shell_box.remove_css_class("pane-closing");

        let start_class = motion.start_css_class();
        let overshoot_class = motion.overshoot_css_class();
        self.shell_box.add_css_class(start_class);

        let shell_box = self.shell_box.clone();
        let settle_delay = ((92.0 / config.animation_speed.max(0.2)) as u64).max(1);
        gtk::glib::timeout_add_local_once(Duration::from_millis(settle_delay), move || {
            shell_box.remove_css_class("pane-born");
            shell_box.remove_css_class(start_class);
            shell_box.add_css_class(overshoot_class);
        });

        let shell_box = self.shell_box.clone();
        let duration = ((340.0 / config.animation_speed.max(0.2)) as u64).max(1);
        gtk::glib::timeout_add_local_once(Duration::from_millis(duration), move || {
            shell_box.remove_css_class("pane-born");
            shell_box.remove_css_class(start_class);
            shell_box.remove_css_class(overshoot_class);
        });
    }

    fn pulse_output(&self) {
        if self.dense_mode.get() && !self.is_active.get() {
            return;
        }

        if self.output_pulse_pending.replace(true) {
            return;
        }

        self.shell_box.add_css_class("output-pulse");
        let shell_box = self.shell_box.clone();
        let pending = self.output_pulse_pending.clone();
        gtk::glib::timeout_add_local_once(Duration::from_millis(160), move || {
            shell_box.remove_css_class("output-pulse");
            pending.set(false);
        });
    }

    fn apply_palette_classes(&self) {
        for preset in theme::PanePalettePreset::ALL {
            self.shell_box.remove_css_class(preset.css_class());
        }

        if let Some(preset) = self.palette_preset.get() {
            self.shell_box.add_css_class(preset.css_class());
        }
    }

    fn refresh_density_visuals(&self) {
        let compact = self.compact_mode.get();
        let dense = self.dense_mode.get();

        set_css_class(&self.shell_box, "compact", compact);
        set_css_class(&self.shell_box, "dense", dense);

        let show_chrome = self.show_context_bar.get();
        self.chrome.set_visible(show_chrome);
        self.subtitle_label
            .set_visible(show_chrome && !compact && !self.subtitle_label.text().is_empty());
        self.badge_box
            .set_visible(show_chrome && !dense && self.badge_box.first_child().is_some());
        self.ambient.set_visible(true);
        self.background.set_visible(
            self.wallpaper_available.get()
                && self.wallpaper_hint.get()
                && (!dense || self.is_active.get()),
        );
        self.terminal.set_enable_shaping(!dense);
    }

    fn render_banner(&self, focus_terminal: bool, flash: bool) {
        let columns = self.terminal.column_count().max(1) as usize;
        let rendered = banner::startup_payload_for_columns(
            &self.shell_path,
            Some(columns),
            self.banner_layout.get(),
        );

        let shell = util::shell_name(&self.shell_path).to_ascii_lowercase();
        let rendered_via_shell = self.child_pid.get().is_some()
            && matches!(shell.as_str(), "bash" | "rbash")
            && util::write_live_banner(&rendered).is_some();

        if rendered_via_shell {
            self.terminal.feed_child(&[0x07]);
        } else {
            let mut payload = String::from("\r\n");
            payload.push_str(&rendered.replace('\n', "\r\n"));
            payload.push_str("\r\n");
            self.terminal.feed(payload.as_bytes());
            if self.child_pid.get().is_some() {
                self.terminal.feed_child(b"\n");
            }
        }

        if focus_terminal {
            self.focus_terminal();
        }
        if flash {
            self.flash_action();
        }
    }

    fn clear_spawn_motion_classes(&self) {
        for class_name in [
            "spawn-from-center",
            "spawn-from-left",
            "spawn-from-right",
            "spawn-from-top",
            "spawn-from-bottom",
            "spawn-overshoot-center",
            "spawn-overshoot-left",
            "spawn-overshoot-right",
            "spawn-overshoot-top",
            "spawn-overshoot-bottom",
        ] {
            self.shell_box.remove_css_class(class_name);
        }
    }

    fn clear_close_motion_classes(&self) {
        for class_name in [
            "close-kick-center",
            "close-kick-left",
            "close-kick-right",
            "close-kick-top",
            "close-kick-bottom",
            "close-to-center",
            "close-to-left",
            "close-to-right",
            "close-to-top",
            "close-to-bottom",
        ] {
            self.shell_box.remove_css_class(class_name);
        }
    }

    fn context_refresh_delay_ms(&self) -> u64 {
        if self.child_pid.get().is_none() {
            return constants::CONTEXT_REFRESH_MS * 4;
        }

        if self.is_active.get() {
            constants::CONTEXT_REFRESH_MS
        } else {
            constants::CONTEXT_REFRESH_MS * 4
        }
    }
}

impl PaneSpawnMotion {
    fn start_css_class(self) -> &'static str {
        match self {
            Self::Center => "spawn-from-center",
            Self::FromLeft => "spawn-from-left",
            Self::FromRight => "spawn-from-right",
            Self::FromTop => "spawn-from-top",
            Self::FromBottom => "spawn-from-bottom",
        }
    }

    fn overshoot_css_class(self) -> &'static str {
        match self {
            Self::Center => "spawn-overshoot-center",
            Self::FromLeft => "spawn-overshoot-left",
            Self::FromRight => "spawn-overshoot-right",
            Self::FromTop => "spawn-overshoot-top",
            Self::FromBottom => "spawn-overshoot-bottom",
        }
    }

    fn close_kick_css_class(self) -> &'static str {
        match self {
            Self::Center => "close-kick-center",
            Self::FromLeft => "close-kick-right",
            Self::FromRight => "close-kick-left",
            Self::FromTop => "close-kick-bottom",
            Self::FromBottom => "close-kick-top",
        }
    }

    fn close_css_class(self) -> &'static str {
        match self {
            Self::Center => "close-to-center",
            Self::FromLeft => "close-to-left",
            Self::FromRight => "close-to-right",
            Self::FromTop => "close-to-top",
            Self::FromBottom => "close-to-bottom",
        }
    }
}

fn set_css_class(widget: &impl IsA<gtk::Widget>, class_name: &str, enabled: bool) {
    if enabled {
        widget.add_css_class(class_name);
    } else {
        widget.remove_css_class(class_name);
    }
}
