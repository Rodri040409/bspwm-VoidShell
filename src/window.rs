use crate::config::{AppConfig, ConfigManager};
use crate::constants;
use crate::context::PanelContext;
use crate::history::{HistoryManager, HistoryStore};
use crate::layout::{Direction, InsertPosition, SplitAxis, TileTree};
use crate::preferences::{self, PreferenceCallbacks};
use crate::quick_actions::{
    self, ActionTarget, InternalAction, QuickActionCommand, QuickActionItem, QuickActionSection,
};
use crate::terminal_pane::{PaneCallbacks, PaneSpawnMotion, TerminalPane};
use crate::theme;
use crate::util;
#[cfg(not(windows))]
use adw::prelude::AdwDialogExt;
use gtk::gio;
use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;
pub struct MainWindow;

struct WindowState {
    app: gtk::Application,
    window: gtk::ApplicationWindow,
    layout_surface: gtk::Overlay,
    shared_wallpaper: gtk::Picture,
    shared_wallpaper_tint: gtk::Box,
    layout_host: gtk::Box,
    title_label: gtk::Label,
    subtitle_label: gtk::Label,
    layout: RefCell<TileTree>,
    panes: RefCell<BTreeMap<u64, Rc<TerminalPane>>>,
    focused_pane: Cell<Option<u64>>,
    zoomed_pane: Cell<Option<u64>>,
    focus_history: RefCell<Vec<u64>>,
    closing_panes: RefCell<BTreeSet<u64>>,
    next_pane_id: Cell<u64>,
    next_split_id: Cell<u64>,
    config_manager: ConfigManager,
    history_manager: HistoryManager,
    config: RefCell<AppConfig>,
    history: RefCell<HistoryStore>,
    palette_revealer: gtk::Revealer,
    palette_search: gtk::SearchEntry,
    palette_list: gtk::ListBox,
    palette_items: RefCell<Vec<QuickActionItem>>,
    toast_revealer: gtk::Revealer,
    toast_label: gtk::Label,
    toast_serial: Cell<u64>,
    startup_command: Option<String>,
    working_directory: Option<PathBuf>,
}

impl MainWindow {
    pub fn present(
        app: &gtk::Application,
        startup_command: Option<String>,
        working_directory: Option<PathBuf>,
    ) {
        let state = WindowState::new(app.clone(), startup_command, working_directory);
        state.window.present();
    }
}

impl WindowState {
    fn new(
        app: gtk::Application,
        startup_command: Option<String>,
        working_directory: Option<PathBuf>,
    ) -> Rc<Self> {
        let config_manager = ConfigManager::new();
        let history_manager = HistoryManager::new();
        let config = config_manager.load_or_default();
        let history = history_manager.load_or_default();

        theme::install_or_update(&config);

        let window = gtk::ApplicationWindow::builder()
            .application(&app)
            .default_width(1480)
            .default_height(920)
            .title(constants::APP_NAME)
            .icon_name(constants::APP_ICON)
            .build();

        let title_label = gtk::Label::new(Some(constants::APP_NAME));
        title_label.add_css_class("title");
        title_label.add_css_class("termvoid-title");
        title_label.set_xalign(0.0);

        let subtitle_label = gtk::Label::new(None);
        subtitle_label.add_css_class("subtitle");
        subtitle_label.set_xalign(0.0);
        subtitle_label.set_visible(false);

        let title_stack = gtk::Box::new(gtk::Orientation::Vertical, 2);
        title_stack.append(&title_label);
        title_stack.append(&subtitle_label);

        let header = gtk::HeaderBar::new();
        header.add_css_class("termvoid-headerbar");
        header.set_title_widget(Some(&title_stack));
        window.set_titlebar(Some(&header));

        let palette_button = gtk::Button::from_icon_name("system-search-symbolic");
        palette_button.add_css_class("flat");
        palette_button.add_css_class("header-utility-button");
        header.pack_start(&palette_button);

        let prefs_button = gtk::Button::from_icon_name("preferences-system-symbolic");
        prefs_button.add_css_class("flat");
        prefs_button.add_css_class("header-utility-button");
        header.pack_end(&prefs_button);

        let about_button = gtk::Button::from_icon_name("help-about-symbolic");
        about_button.add_css_class("flat");
        about_button.add_css_class("header-utility-button");
        header.pack_end(&about_button);

        let root_overlay = gtk::Overlay::new();
        root_overlay.add_css_class("app-shell");
        root_overlay.set_hexpand(true);
        root_overlay.set_vexpand(true);

        let window_surface = gtk::Overlay::new();
        window_surface.add_css_class("window-surface");
        window_surface.set_hexpand(true);
        window_surface.set_vexpand(true);
        root_overlay.set_child(Some(&window_surface));

        let shared_wallpaper = gtk::Picture::new();
        shared_wallpaper.add_css_class("shared-wallpaper");
        shared_wallpaper.set_content_fit(gtk::ContentFit::Cover);
        shared_wallpaper.set_can_shrink(true);
        shared_wallpaper.set_hexpand(true);
        shared_wallpaper.set_vexpand(true);
        window_surface.set_child(Some(&shared_wallpaper));

        let shared_wallpaper_tint = gtk::Box::new(gtk::Orientation::Vertical, 0);
        shared_wallpaper_tint.add_css_class("shared-wallpaper-tint");
        shared_wallpaper_tint.set_hexpand(true);
        shared_wallpaper_tint.set_vexpand(true);
        shared_wallpaper_tint.set_can_target(false);
        window_surface.add_overlay(&shared_wallpaper_tint);

        let layout_surface = gtk::Overlay::new();
        layout_surface.add_css_class("layout-surface");
        layout_surface.set_hexpand(true);
        layout_surface.set_vexpand(true);
        window_surface.add_overlay(&layout_surface);

        let layout_host = gtk::Box::new(gtk::Orientation::Vertical, 0);
        layout_host.set_hexpand(true);
        layout_host.set_vexpand(true);
        layout_surface.set_child(Some(&layout_host));

        let palette_revealer = gtk::Revealer::builder()
            .transition_type(if config.enable_animations {
                gtk::RevealerTransitionType::SlideDown
            } else {
                gtk::RevealerTransitionType::None
            })
            .transition_duration((180.0 / config.animation_speed.max(0.2)) as u32)
            .build();
        palette_revealer.set_halign(gtk::Align::Center);
        palette_revealer.set_valign(gtk::Align::Start);
        palette_revealer.set_margin_top(74);
        root_overlay.add_overlay(&palette_revealer);

        let palette_card = gtk::Box::new(gtk::Orientation::Vertical, 10);
        palette_card.add_css_class("palette-card");
        let palette_search = gtk::SearchEntry::new();
        palette_search.set_placeholder_text(Some(
            "Acciones rápidas, ssh, contenedores, proyectos, git...",
        ));
        let palette_scroller = gtk::ScrolledWindow::new();
        palette_scroller.set_min_content_height(340);
        palette_scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        let palette_list = gtk::ListBox::new();
        palette_list.add_css_class("palette-list");
        palette_list.set_selection_mode(gtk::SelectionMode::None);
        palette_scroller.set_child(Some(&palette_list));
        palette_card.append(&palette_search);
        palette_card.append(&palette_scroller);
        palette_revealer.set_child(Some(&palette_card));

        let toast_revealer = gtk::Revealer::builder()
            .transition_type(if config.enable_animations {
                gtk::RevealerTransitionType::SlideUp
            } else {
                gtk::RevealerTransitionType::None
            })
            .transition_duration((180.0 / config.animation_speed.max(0.2)) as u32)
            .build();
        toast_revealer.set_halign(gtk::Align::Center);
        toast_revealer.set_valign(gtk::Align::End);
        toast_revealer.set_margin_bottom(24);
        root_overlay.add_overlay(&toast_revealer);

        let toast_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        toast_box.add_css_class("toast");
        toast_box.add_css_class("frame");
        toast_box.add_css_class("card");
        toast_box.set_margin_start(16);
        toast_box.set_margin_end(16);
        toast_box.set_margin_top(8);
        toast_box.set_margin_bottom(8);

        let toast_label = gtk::Label::new(None);
        toast_label.set_wrap(true);
        toast_label.set_xalign(0.0);
        toast_label.set_max_width_chars(72);
        toast_label.set_margin_start(14);
        toast_label.set_margin_end(14);
        toast_label.set_margin_top(10);
        toast_label.set_margin_bottom(10);
        toast_box.append(&toast_label);
        toast_revealer.set_child(Some(&toast_box));

        window.set_child(Some(&root_overlay));

        let state = Rc::new(Self {
            app,
            window,
            layout_surface,
            shared_wallpaper,
            shared_wallpaper_tint,
            layout_host,
            title_label,
            subtitle_label,
            layout: RefCell::new(TileTree::default()),
            panes: RefCell::new(BTreeMap::new()),
            focused_pane: Cell::new(None),
            zoomed_pane: Cell::new(None),
            focus_history: RefCell::new(Vec::new()),
            closing_panes: RefCell::new(BTreeSet::new()),
            next_pane_id: Cell::new(1),
            next_split_id: Cell::new(1),
            config_manager,
            history_manager,
            config: RefCell::new(config),
            history: RefCell::new(history),
            palette_revealer,
            palette_search,
            palette_list,
            palette_items: RefCell::new(Vec::new()),
            toast_revealer,
            toast_label,
            toast_serial: Cell::new(0),
            startup_command,
            working_directory,
        });

        state.install_actions();
        state.install_ui_handlers(palette_button, prefs_button, about_button);
        state.apply_shared_wallpaper(&state.config.borrow());
        state.create_initial_pane();
        state.refresh_header();
        state
    }

    fn install_ui_handlers(
        self: &Rc<Self>,
        palette_button: gtk::Button,
        prefs_button: gtk::Button,
        about_button: gtk::Button,
    ) {
        let weak = Rc::downgrade(self);
        palette_button.connect_clicked(move |_| {
            if let Some(state) = weak.upgrade() {
                state.toggle_palette();
            }
        });

        let weak = Rc::downgrade(self);
        prefs_button.connect_clicked(move |_| {
            if let Some(state) = weak.upgrade() {
                state.open_preferences();
            }
        });

        let weak = Rc::downgrade(self);
        about_button.connect_clicked(move |_| {
            if let Some(state) = weak.upgrade() {
                state.open_about();
            }
        });

        let weak = Rc::downgrade(self);
        self.palette_search.connect_search_changed(move |_| {
            if let Some(state) = weak.upgrade() {
                state.rebuild_palette_rows();
            }
        });

        let weak = Rc::downgrade(self);
        self.palette_search.connect_stop_search(move |_| {
            if let Some(state) = weak.upgrade() {
                state.close_palette();
            }
        });

        let weak = Rc::downgrade(self);
        self.window.connect_close_request(move |_| {
            if let Some(state) = weak.upgrade() {
                let _ = state.history_manager.save(&state.history.borrow());
            }
            gtk::glib::Propagation::Proceed
        });
    }

    fn install_actions(self: &Rc<Self>) {
        self.install_simple_action("new-panel", |state| state.new_panel());
        self.install_simple_action("split-horizontal", |state| {
            state.split_focused(SplitAxis::Horizontal);
        });
        self.install_simple_action("split-vertical", |state| {
            state.split_focused(SplitAxis::Vertical);
        });
        self.install_simple_action("close-pane", |state| state.close_focused());
        self.install_simple_action("focus-left", |state| state.focus_direction(Direction::Left));
        self.install_simple_action("focus-right", |state| {
            state.focus_direction(Direction::Right)
        });
        self.install_simple_action("focus-up", |state| state.focus_direction(Direction::Up));
        self.install_simple_action("focus-down", |state| state.focus_direction(Direction::Down));
        self.install_simple_action("resize-left", |state| state.resize_focused(Direction::Left));
        self.install_simple_action("resize-right", |state| {
            state.resize_focused(Direction::Right)
        });
        self.install_simple_action("resize-up", |state| state.resize_focused(Direction::Up));
        self.install_simple_action("resize-down", |state| state.resize_focused(Direction::Down));
        self.install_simple_action("copy", |state| state.copy_from_focused());
        self.install_simple_action("cut", |state| state.cut_from_focused());
        self.install_simple_action("paste", |state| state.paste_into_focused());
        self.install_simple_action("preferences", |state| state.open_preferences());
        self.install_simple_action("reload-config", |state| state.reload_config());
        self.install_simple_action("palette", |state| state.toggle_palette());
        self.install_simple_action("fullscreen", |state| state.toggle_fullscreen());
        self.install_simple_action("zoom-pane", |state| state.toggle_pane_zoom());
        self.install_simple_action("show-info", |state| state.show_info_banner());
        self.install_simple_action("swap-left", |state| state.swap_focused(Direction::Left));
        self.install_simple_action("swap-right", |state| state.swap_focused(Direction::Right));
        self.install_simple_action("swap-up", |state| state.swap_focused(Direction::Up));
        self.install_simple_action("swap-down", |state| state.swap_focused(Direction::Down));

        for index in 1..=9 {
            self.install_simple_action(&format!("focus-slot-{index}"), move |state| {
                state.focus_nth(index);
            });
        }

        self.app.set_accels_for_action("win.new-panel", &["<Alt>t"]);
        self.app
            .set_accels_for_action("win.split-horizontal", &["<Alt>h"]);
        self.app
            .set_accels_for_action("win.split-vertical", &["<Alt>v"]);
        self.app
            .set_accels_for_action("win.close-pane", &["<Alt>q"]);
        self.app
            .set_accels_for_action("win.focus-left", &["<Alt>Left"]);
        self.app
            .set_accels_for_action("win.focus-right", &["<Alt>Right"]);
        self.app.set_accels_for_action("win.focus-up", &["<Alt>Up"]);
        self.app
            .set_accels_for_action("win.focus-down", &["<Alt>Down"]);
        self.app
            .set_accels_for_action("win.resize-left", &["<Alt><Shift>Left"]);
        self.app
            .set_accels_for_action("win.resize-right", &["<Alt><Shift>Right"]);
        self.app
            .set_accels_for_action("win.resize-up", &["<Alt><Shift>Up"]);
        self.app
            .set_accels_for_action("win.resize-down", &["<Alt><Shift>Down"]);
        self.app
            .set_accels_for_action("win.copy", &["<Alt>c", "<Primary><Shift>c"]);
        self.app
            .set_accels_for_action("win.cut", &["<Alt>x", "<Primary><Shift>x"]);
        self.app
            .set_accels_for_action("win.paste", &["<Alt>p", "<Primary><Shift>v"]);
        self.app
            .set_accels_for_action("win.fullscreen", &["<Alt>Return", "F11"]);
        self.app
            .set_accels_for_action("win.zoom-pane", &["<Alt><Shift>Return"]);
        self.app.set_accels_for_action("win.show-info", &["<Alt>i"]);
        self.app
            .set_accels_for_action("win.swap-left", &["<Primary><Alt>Left"]);
        self.app
            .set_accels_for_action("win.swap-right", &["<Primary><Alt>Right"]);
        self.app
            .set_accels_for_action("win.swap-up", &["<Primary><Alt>Up"]);
        self.app
            .set_accels_for_action("win.swap-down", &["<Primary><Alt>Down"]);
        self.app
            .set_accels_for_action("win.preferences", &["<Alt>comma"]);
        self.app
            .set_accels_for_action("win.reload-config", &["<Alt>r"]);
        self.app.set_accels_for_action(
            "win.palette",
            &["<Alt>f", "<Alt>space", constants::ALT_SPACE_FALLBACK],
        );

        for index in 1..=9 {
            self.app.set_accels_for_action(
                &format!("win.focus-slot-{index}"),
                &[&format!("<Alt>{index}")],
            );
        }
    }

    fn install_simple_action<F>(self: &Rc<Self>, name: &str, handler: F)
    where
        F: Fn(&Rc<Self>) + 'static,
    {
        let action = gio::SimpleAction::new(name, None);
        let state = self.clone();
        action.connect_activate(move |_, _| {
            handler(&state);
        });
        self.window.add_action(&action);
    }

    fn create_initial_pane(self: &Rc<Self>) {
        let pane_id = self.allocate_pane_id();
        let shell_path = self.resolved_shell_path();
        let cwd = self
            .working_directory
            .clone()
            .or_else(|| std::env::current_dir().ok())
            .or_else(util::home_dir);
        let pane = TerminalPane::new(
            pane_id,
            shell_path.clone(),
            cwd.clone(),
            self.config.borrow().show_startup_banner,
            PaneSpawnMotion::Center,
            &self.config.borrow(),
            self.pane_callbacks(),
        );

        self.layout.borrow_mut().set_root_leaf(pane_id);
        self.panes.borrow_mut().insert(pane_id, pane.clone());
        self.history.borrow_mut().begin_session(
            pane_id,
            &util::shell_name(&shell_path),
            cwd.as_deref().and_then(|path| path.to_str()),
        );
        if let Some(command) = &self.startup_command {
            pane.run_command(command);
        }
        self.persist_history();
        self.set_focused_pane(pane_id);
        self.rebuild_layout();
    }

    fn pane_callbacks(self: &Rc<Self>) -> PaneCallbacks {
        let weak = Rc::downgrade(self);
        PaneCallbacks {
            on_focus: Rc::new(move |pane_id| {
                if let Some(state) = weak.upgrade() {
                    state.set_focused_pane(pane_id);
                }
            }),
            on_context_changed: {
                let weak = Rc::downgrade(self);
                Rc::new(move |pane_id, context| {
                    if let Some(state) = weak.upgrade() {
                        state.on_context_changed(pane_id, context);
                    }
                })
            },
            on_exit: {
                let weak = Rc::downgrade(self);
                Rc::new(move |pane_id, _status| {
                    if let Some(state) = weak.upgrade() {
                        state.close_pane(pane_id);
                    }
                })
            },
            on_notification: {
                let weak = Rc::downgrade(self);
                Rc::new(move |message| {
                    if let Some(state) = weak.upgrade() {
                        state.show_toast(&message);
                    }
                })
            },
            on_swap_request: {
                let weak = Rc::downgrade(self);
                Rc::new(move |source_id, target_id| {
                    if let Some(state) = weak.upgrade() {
                        state.swap_panes(source_id, target_id);
                    }
                })
            },
            on_toggle_zoom: {
                let weak = Rc::downgrade(self);
                Rc::new(move |pane_id| {
                    if let Some(state) = weak.upgrade() {
                        state.toggle_specific_pane_zoom(pane_id);
                    }
                })
            },
        }
    }

    fn on_context_changed(&self, _pane_id: u64, context: PanelContext) {
        let mut history = self.history.borrow_mut();
        if let Some(cwd) = &context.cwd {
            history.note_directory(cwd);
            if context.git_branch.is_some() {
                history.note_project(cwd);
            }
        }
        if let Some((title, command, category)) = quick_actions::detected_command_entry(&context) {
            history.note_command(&title, &command, &category);
        }
        if let Some(target) = context.ssh_target.as_deref() {
            history.note_connection(&format!("SSH {target}"), &format!("ssh {target}"));
        }
        drop(history);
        self.persist_history();
        self.refresh_header();
    }

    fn new_panel(self: &Rc<Self>) {
        let Some(current_id) = self
            .focused_pane
            .get()
            .or_else(|| self.layout.borrow().first_leaf())
        else {
            return;
        };

        self.zoomed_pane.set(None);
        let axis = self.smart_split_axis(current_id);
        let position = self.smart_insert_position(current_id);
        self.spawn_split(current_id, axis, position);
    }

    fn split_focused(self: &Rc<Self>, axis: SplitAxis) {
        let Some(current_id) = self
            .focused_pane
            .get()
            .or_else(|| self.layout.borrow().first_leaf())
        else {
            return;
        };

        self.zoomed_pane.set(None);
        self.spawn_split(current_id, axis, InsertPosition::After);
    }

    fn close_focused(self: &Rc<Self>) {
        let Some(pane_id) = self.focused_pane.get() else {
            return;
        };

        self.close_pane(pane_id);
    }

    fn close_pane(self: &Rc<Self>, pane_id: u64) {
        if !self.panes.borrow().contains_key(&pane_id)
            || self.closing_panes.borrow().contains(&pane_id)
        {
            return;
        }

        if self.layout.borrow().leaf_count() <= 1 {
            self.end_session_for_pane(pane_id);
            self.window.close();
            return;
        }

        if self.zoomed_pane.get() == Some(pane_id) {
            self.zoomed_pane.set(None);
        }

        let close_motion = self
            .layout
            .borrow()
            .leaf_edge_direction(pane_id)
            .map(pane_motion_from_direction)
            .unwrap_or(PaneSpawnMotion::Center);

        let fallback = if self.focused_pane.get() == Some(pane_id) {
            self.close_focus_candidate(pane_id)
                .or_else(|| self.previous_focus_candidate(pane_id))
                .or_else(|| {
                    self.layout
                        .borrow()
                        .leaf_ids()
                        .into_iter()
                        .find(|id| *id != pane_id)
                })
        } else {
            self.focused_pane.get()
        };

        if let Some(focus_id) = fallback {
            self.set_focused_pane(focus_id);
            if let Some(pane) = self.panes.borrow().get(&focus_id) {
                pane.focus_terminal();
            }
        } else {
            self.focused_pane.set(None);
        }

        if self.config.borrow().enable_animations {
            if let Some(pane) = self.panes.borrow().get(&pane_id).cloned() {
                self.closing_panes.borrow_mut().insert(pane_id);
                pane.begin_close_animation(&self.config.borrow(), close_motion);
                let delay = pane.close_animation_duration_ms(&self.config.borrow()) + 24;
                let weak = Rc::downgrade(self);
                gtk::glib::timeout_add_local_once(Duration::from_millis(delay), move || {
                    if let Some(state) = weak.upgrade() {
                        state.finish_close_pane(pane_id);
                    }
                });
                return;
            }
        }

        self.finish_close_pane(pane_id);
    }

    fn rebuild_layout(self: &Rc<Self>) {
        for pane in self.panes.borrow().values() {
            pane.detach_from_parent();
        }

        while let Some(child) = self.layout_host.first_child() {
            self.layout_host.remove(&child);
        }

        let ratio_update = {
            let weak = Rc::downgrade(self);
            Rc::new(move |split_id: u64, ratio: f32| {
                if let Some(state) = weak.upgrade() {
                    state
                        .layout
                        .borrow_mut()
                        .update_split_ratio(split_id, ratio);
                }
            })
        };

        if self.zoomed_pane.get().is_some() {
            self.layout_surface.add_css_class("zoomed");
        } else {
            self.layout_surface.remove_css_class("zoomed");
        }

        let widget = if let Some(zoomed) = self.zoomed_pane.get() {
            self.panes.borrow().get(&zoomed).map(|pane| pane.widget())
        } else {
            self.layout
                .borrow()
                .build_widget(&self.panes.borrow(), ratio_update)
        };

        if let Some(widget) = widget {
            widget.set_hexpand(true);
            widget.set_vexpand(true);
            self.layout_host.append(&widget);
        }

        if self
            .focused_pane
            .get()
            .is_some_and(|id| !self.panes.borrow().contains_key(&id))
        {
            self.focused_pane.set(self.layout.borrow().first_leaf());
        }

        self.sync_focus_state();
        self.refresh_pane_density();
        self.refresh_header();

        if let Some(pane) = self.focused_pane_ref() {
            let pane = pane.clone();
            gtk::glib::idle_add_local_once(move || {
                pane.focus_terminal();
            });
        }
    }

    fn sync_focus_state(&self) {
        for (id, pane) in self.panes.borrow().iter() {
            pane.set_active(Some(*id) == self.focused_pane.get());
        }
    }

    fn refresh_header(&self) {
        if let Some(pane) = self.focused_pane_ref() {
            let context = pane.context();
            self.title_label.set_text(constants::APP_NAME);
            let prefix = if self.zoomed_pane.get() == self.focused_pane.get() {
                "Zoom · "
            } else {
                ""
            };
            self.subtitle_label.set_text(&format!(
                "{prefix}{} · {}",
                context.header_title(),
                context.header_subtitle()
            ));
            self.subtitle_label.set_visible(true);
        } else {
            self.title_label.set_text(constants::APP_NAME);
            self.subtitle_label.set_text("Sin panel activo");
            self.subtitle_label.set_visible(true);
        }
    }

    fn set_focused_pane(self: &Rc<Self>, pane_id: u64) {
        if !self.panes.borrow().contains_key(&pane_id) {
            return;
        }

        if self
            .zoomed_pane
            .get()
            .is_some_and(|zoomed| zoomed != pane_id)
        {
            self.zoomed_pane.set(None);
            self.rebuild_layout();
        }

        self.focused_pane.set(Some(pane_id));
        self.remember_focus(pane_id);
        self.sync_focus_state();
        self.refresh_header();
    }

    fn focused_pane_ref(&self) -> Option<Rc<TerminalPane>> {
        self.focused_pane
            .get()
            .and_then(|id| self.panes.borrow().get(&id).cloned())
    }

    fn focus_direction(self: &Rc<Self>, direction: Direction) {
        if self.zoomed_pane.get().is_some() {
            return;
        }
        let Some(current_id) = self.focused_pane.get() else {
            return;
        };
        if let Some((pane_id, _)) = self.nearest_pane_in_direction(current_id, direction) {
            self.set_focused_pane(pane_id);
            if let Some(pane) = self.panes.borrow().get(&pane_id) {
                pane.focus_terminal();
            }
        }
    }

    fn focus_nth(self: &Rc<Self>, index: usize) {
        if let Some(pane_id) = self.layout.borrow().leaf_ids().get(index - 1).copied() {
            self.set_focused_pane(pane_id);
            if let Some(pane) = self.panes.borrow().get(&pane_id) {
                pane.focus_terminal();
            }
        }
    }

    fn resize_focused(self: &Rc<Self>, direction: Direction) {
        if self.zoomed_pane.get().is_some() {
            return;
        }
        let Some(pane_id) = self.focused_pane.get() else {
            return;
        };

        if self
            .layout
            .borrow_mut()
            .resize_leaf(pane_id, direction, constants::DEFAULT_RESIZE_STEP)
        {
            self.rebuild_layout();
        }
    }

    fn copy_from_focused(&self) {
        if let Some(pane) = self.focused_pane_ref() {
            let _ = pane.copy_selection_to_clipboard();
        }
    }

    fn cut_from_focused(&self) {
        if let Some(pane) = self.focused_pane_ref() {
            let _ = pane.cut_selection_to_clipboard();
        }
    }

    fn paste_into_focused(&self) {
        if let Some(pane) = self.focused_pane_ref() {
            pane.paste_from_clipboard();
        }
    }

    fn toggle_fullscreen(&self) {
        if self.window.is_fullscreen() {
            self.window.unfullscreen();
        } else {
            self.window.fullscreen();
        }
    }

    fn toggle_pane_zoom(self: &Rc<Self>) {
        let Some(pane_id) = self.focused_pane.get() else {
            return;
        };
        self.toggle_specific_pane_zoom(pane_id);
    }

    fn toggle_specific_pane_zoom(self: &Rc<Self>, pane_id: u64) {
        if !self.panes.borrow().contains_key(&pane_id) {
            return;
        }

        if self.zoomed_pane.get() == Some(pane_id) {
            self.zoomed_pane.set(None);
            self.show_toast("Panel restaurado");
        } else {
            self.zoomed_pane.set(Some(pane_id));
            self.set_focused_pane(pane_id);
            self.show_toast("Panel ampliado");
        }
        self.rebuild_layout();
    }

    fn show_info_banner(&self) {
        if let Some(pane) = self.focused_pane_ref() {
            pane.show_banner_info();
            self.show_toast("Información del sistema renderizada en el panel activo");
        }
    }

    fn open_preferences(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        #[cfg(not(windows))]
        let dialog = preferences::build_dialog(
            &self.window,
            &self.config.borrow(),
            PreferenceCallbacks {
                on_config_changed: Rc::new(move |config| {
                    if let Some(state) = weak.upgrade() {
                        state.update_config(config);
                    }
                }),
                on_reload_from_disk: {
                    let weak = Rc::downgrade(self);
                    Rc::new(move || {
                        if let Some(state) = weak.upgrade() {
                            state.reload_config();
                        }
                    })
                },
            },
        );
        #[cfg(not(windows))]
        dialog.present(Some(&self.window));

        #[cfg(windows)]
        {
            let dialog = preferences::build_dialog(
                &self.window,
                &self.config.borrow(),
                PreferenceCallbacks {
                    on_config_changed: Rc::new(move |config| {
                        if let Some(state) = weak.upgrade() {
                            state.update_config(config);
                        }
                    }),
                    on_reload_from_disk: {
                        let weak = Rc::downgrade(self);
                        Rc::new(move || {
                            if let Some(state) = weak.upgrade() {
                                state.reload_config();
                            }
                        })
                    },
                },
            );
            dialog.present();
        }
    }

    fn open_about(&self) {
        let dialog = gtk::AboutDialog::builder()
            .program_name(constants::APP_NAME)
            .logo_icon_name(constants::APP_ICON)
            .version(constants::APP_VERSION)
            .website("https://github.com/Rodri040409/bspwm-VoidShell")
            .comments("VoidShell es una terminal con paneles en mosaico, chrome contextual y arranque por directorio o comando. En Linux usa VTE; en Windows mantiene un backend nativo mientras madura la capa multiplataforma.")
            .authors(vec!["voidscripter"])
            .modal(true)
            .transient_for(&self.window)
            .build();
        dialog.present();
    }

    fn update_config(&self, config: AppConfig) {
        *self.config.borrow_mut() = config.clone();
        theme::install_or_update(&config);
        self.apply_shared_wallpaper(&config);
        self.palette_revealer
            .set_transition_duration((180.0 / config.animation_speed.max(0.2)) as u32);
        self.palette_revealer
            .set_transition_type(if config.enable_animations {
                gtk::RevealerTransitionType::SlideDown
            } else {
                gtk::RevealerTransitionType::None
            });
        self.toast_revealer
            .set_transition_duration((180.0 / config.animation_speed.max(0.2)) as u32);
        self.toast_revealer
            .set_transition_type(if config.enable_animations {
                gtk::RevealerTransitionType::SlideUp
            } else {
                gtk::RevealerTransitionType::None
            });
        for pane in self.panes.borrow().values() {
            pane.apply_config(&config);
        }
        self.refresh_pane_density();
        if let Err(error) = self.config_manager.save(&config) {
            self.show_toast(&format!("No se pudieron guardar las preferencias: {error}"));
        }
    }

    fn reload_config(&self) {
        let config = self.config_manager.load_or_default();
        self.update_config(config);
        self.show_toast("Configuración recargada");
    }

    fn end_session_for_pane(&self, pane_id: u64) {
        let Some(pane) = self.panes.borrow().get(&pane_id).cloned() else {
            return;
        };

        let context = pane.context();
        self.history.borrow_mut().end_session(
            pane_id,
            context.cwd.as_deref().and_then(|path| path.to_str()),
            &context.history_context(),
        );
        self.persist_history();
    }

    fn toggle_palette(self: &Rc<Self>) {
        if self.palette_revealer.reveals_child() {
            self.close_palette();
        } else {
            self.open_palette();
        }
    }

    fn open_palette(self: &Rc<Self>) {
        if !self.config.borrow().enable_quick_actions {
            self.show_toast("Las acciones rápidas están desactivadas en preferencias");
            return;
        }

        let context = self.focused_pane_ref().map(|pane| pane.context());
        let items = quick_actions::collect_actions(context.as_ref(), &self.history.borrow());
        *self.palette_items.borrow_mut() = items;
        self.palette_search.set_text("");
        self.rebuild_palette_rows();
        self.palette_revealer.set_reveal_child(true);
        self.palette_search.grab_focus();
    }

    fn close_palette(&self) {
        self.palette_revealer.set_reveal_child(false);
        if let Some(pane) = self.focused_pane_ref() {
            let pane = pane.clone();
            gtk::glib::idle_add_local_once(move || {
                pane.focus_terminal();
            });
        } else {
            self.window.grab_focus();
        }
    }

    fn rebuild_palette_rows(self: &Rc<Self>) {
        while let Some(child) = self.palette_list.first_child() {
            self.palette_list.remove(&child);
        }

        let raw_query = self.palette_search.text().to_string();
        let query = raw_query.to_ascii_lowercase();
        let context = self.focused_pane_ref().map(|pane| pane.context());
        let mut filtered: Vec<QuickActionItem> = self
            .palette_items
            .borrow()
            .iter()
            .filter(|item| {
                query.is_empty()
                    || item.title.to_ascii_lowercase().contains(&query)
                    || item.subtitle.to_ascii_lowercase().contains(&query)
                    || item
                        .badge
                        .as_deref()
                        .is_some_and(|badge| badge.to_ascii_lowercase().contains(&query))
            })
            .cloned()
            .collect();

        if !query.is_empty() {
            let dynamic = quick_actions::query_actions(&raw_query, context.as_ref());
            filtered.extend(dynamic);
            filtered = quick_actions::dedupe_items(filtered);
        }

        if filtered.is_empty() {
            self.palette_list
                .append(&build_palette_empty_row(query.is_empty()));
            return;
        }

        let mut ranked = filtered
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                (
                    item.section,
                    quick_actions::match_score(&item, &raw_query),
                    index,
                    item,
                )
            })
            .collect::<Vec<_>>();

        ranked.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| right.1.cmp(&left.1))
                .then_with(|| left.2.cmp(&right.2))
        });

        let mut section_counts = BTreeMap::new();
        for (section, _, _, _) in &ranked {
            *section_counts.entry(*section).or_insert(0usize) += 1;
        }

        let mut current_section = None;
        for (section, _, _, item) in ranked {
            if current_section != Some(section) {
                self.palette_list.append(&build_palette_section_row(
                    section,
                    *section_counts.get(&section).unwrap_or(&0),
                ));
                current_section = Some(section);
            }

            let row = build_palette_action_row(&item, section);
            let weak = Rc::downgrade(self);
            row.connect_activate(move |_| {
                if let Some(state) = weak.upgrade() {
                    state.execute_quick_action(item.clone());
                }
            });

            self.palette_list.append(&row);
        }
    }

    fn execute_quick_action(self: &Rc<Self>, item: QuickActionItem) {
        match &item.command {
            QuickActionCommand::OpenFileManager(path) => {
                let uri = gio::File::for_path(path).uri();
                if let Err(error) = gio::AppInfo::launch_default_for_uri(
                    uri.as_str(),
                    None::<&gio::AppLaunchContext>,
                ) {
                    self.show_toast(&format!("No se pudo abrir el directorio: {error}"));
                }
            }
            QuickActionCommand::ChangeDirectory(path) => {
                if let Some(pane) = self.focused_pane_ref() {
                    let command = format!(
                        "cd {} && clear",
                        util::shell_quote(&path.display().to_string())
                    );
                    pane.run_command(&command);
                }
            }
            QuickActionCommand::Shell(command) => match item.target {
                ActionTarget::CurrentPane => {
                    if let Some(pane) = self.focused_pane_ref() {
                        pane.run_command(command);
                    }
                }
                ActionTarget::NewPane => {
                    self.new_panel();
                    if let Some(pane) = self.focused_pane_ref() {
                        pane.run_command(command);
                    }
                }
            },
            QuickActionCommand::Internal(action) => {
                self.execute_internal_action(action);
            }
        }

        match &item.command {
            QuickActionCommand::Shell(command) => {
                self.history.borrow_mut().note_action(&item.title, command);
                if command.starts_with("ssh ") {
                    self.history
                        .borrow_mut()
                        .note_connection(&item.title, command);
                }
            }
            QuickActionCommand::ChangeDirectory(path) => {
                self.history.borrow_mut().note_project(path);
            }
            QuickActionCommand::OpenFileManager(_) | QuickActionCommand::Internal(_) => {}
        }

        self.persist_history();
        self.close_palette();
        if !matches!(item.command, QuickActionCommand::Internal(_)) {
            self.show_toast(&format!("Se ejecutó {}", item.title));
        }
    }

    fn show_toast(&self, message: &str) {
        let serial = self.toast_serial.get() + 1;
        self.toast_serial.set(serial);
        self.toast_label.set_text(message);
        self.toast_revealer.set_reveal_child(true);

        let revealer = self.toast_revealer.clone();
        let serial_cell = self.toast_serial.clone();
        gtk::glib::timeout_add_local_once(Duration::from_secs(2), move || {
            if serial_cell.get() == serial {
                revealer.set_reveal_child(false);
            }
        });
    }

    fn apply_shared_wallpaper(&self, config: &AppConfig) {
        if let Some(path) = config
            .wallpaper_path
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            if let Some(texture) = util::cached_wallpaper_texture(path) {
                self.shared_wallpaper.set_paintable(Some(&texture));
                self.shared_wallpaper.set_visible(true);
            } else {
                self.shared_wallpaper
                    .set_paintable(Option::<&gtk::gdk::Texture>::None);
                self.shared_wallpaper.set_visible(false);
            }
        } else {
            self.shared_wallpaper
                .set_paintable(Option::<&gtk::gdk::Texture>::None);
            self.shared_wallpaper.set_visible(false);
        }

        self.shared_wallpaper_tint
            .set_opacity((config.overlay_opacity * 0.38).clamp(0.08, 0.58));
    }

    fn execute_internal_action(self: &Rc<Self>, action: &InternalAction) {
        match action {
            InternalAction::ShowInfo => self.show_info_banner(),
            InternalAction::TogglePaneZoom => self.toggle_pane_zoom(),
            InternalAction::SwapPane(direction) => self.swap_focused(*direction),
            InternalAction::SetPanePalette(preset) => {
                if let Some(pane) = self.focused_pane_ref() {
                    pane.set_palette_preset(*preset, &self.config.borrow());
                    let message = preset
                        .map(|preset| format!("Paleta del panel cambiada a {}", preset.label()))
                        .unwrap_or_else(|| "Paleta del panel restablecida".to_string());
                    self.show_toast(&message);
                }
            }
        }
    }

    fn allocate_pane_id(&self) -> u64 {
        let next = self.next_pane_id.get();
        self.next_pane_id.set(next + 1);
        next
    }

    fn allocate_split_id(&self) -> u64 {
        let next = self.next_split_id.get();
        self.next_split_id.set(next + 1);
        next
    }

    fn nearest_pane_in_direction(
        &self,
        current_id: u64,
        direction: Direction,
    ) -> Option<(u64, f32)> {
        let current_rect = self.pane_rect(current_id)?;
        let center = center_of(&current_rect);
        let mut best: Option<(u64, f32)> = None;

        for (pane_id, pane) in self.panes.borrow().iter() {
            if *pane_id == current_id {
                continue;
            }

            let Some(rect) = pane.widget().compute_bounds(&self.layout_host) else {
                continue;
            };
            let candidate = center_of(&rect);

            let primary = match direction {
                Direction::Left => center.0 - candidate.0,
                Direction::Right => candidate.0 - center.0,
                Direction::Up => center.1 - candidate.1,
                Direction::Down => candidate.1 - center.1,
            };

            if primary <= 0.0 {
                continue;
            }

            let secondary = match direction {
                Direction::Left | Direction::Right => (candidate.1 - center.1).abs(),
                Direction::Up | Direction::Down => (candidate.0 - center.0).abs(),
            };

            let score = primary + secondary * 0.35;
            if best.map(|(_, current)| score < current).unwrap_or(true) {
                best = Some((*pane_id, score));
            }
        }

        best
    }

    fn close_focus_candidate(&self, pane_id: u64) -> Option<u64> {
        let current_rect = self.pane_rect(pane_id)?;
        let center = center_of(&current_rect);
        let mut best: Option<(u64, f32)> = None;

        for (candidate_id, pane) in self.panes.borrow().iter() {
            if *candidate_id == pane_id {
                continue;
            }

            let Some(rect) = pane.widget().compute_bounds(&self.layout_host) else {
                continue;
            };
            let candidate = center_of(&rect);
            let dx = candidate.0 - center.0;
            let dy = candidate.1 - center.1;
            let distance = (dx * dx + dy * dy).sqrt();

            let horizontal_overlap = overlap_span(
                current_rect.x(),
                current_rect.x() + current_rect.width(),
                rect.x(),
                rect.x() + rect.width(),
            );
            let vertical_overlap = overlap_span(
                current_rect.y(),
                current_rect.y() + current_rect.height(),
                rect.y(),
                rect.y() + rect.height(),
            );

            let overlap_bonus = horizontal_overlap.max(vertical_overlap) * 0.22;
            let score = distance - overlap_bonus;
            if best.map(|(_, current)| score < current).unwrap_or(true) {
                best = Some((*candidate_id, score));
            }
        }

        best.map(|(pane_id, _)| pane_id)
    }

    fn swap_focused(self: &Rc<Self>, direction: Direction) {
        if self.zoomed_pane.get().is_some() {
            return;
        }

        let Some(current_id) = self.focused_pane.get() else {
            return;
        };
        let Some((target_id, _)) = self.nearest_pane_in_direction(current_id, direction) else {
            return;
        };
        self.swap_panes(current_id, target_id);
    }

    fn swap_panes(self: &Rc<Self>, first_id: u64, second_id: u64) {
        if !self.layout.borrow_mut().swap_leaves(first_id, second_id) {
            return;
        }

        self.rebuild_layout();
        if let Some(pane) = self.focused_pane_ref() {
            pane.focus_terminal();
        }
        self.show_toast("Paneles reordenados");
    }

    fn spawn_split(self: &Rc<Self>, current_id: u64, axis: SplitAxis, position: InsertPosition) {
        let cwd = self
            .panes
            .borrow()
            .get(&current_id)
            .and_then(|pane| pane.current_directory());
        let shell_path = self.resolved_shell_path();
        let pane_id = self.allocate_pane_id();
        let pane = TerminalPane::new(
            pane_id,
            shell_path.clone(),
            cwd.clone(),
            self.config.borrow().show_banner_on_new_panes,
            pane_spawn_motion(axis, position),
            &self.config.borrow(),
            self.pane_callbacks(),
        );

        self.layout.borrow_mut().split_leaf_with_position(
            current_id,
            pane_id,
            self.allocate_split_id(),
            axis,
            position,
        );
        self.panes.borrow_mut().insert(pane_id, pane);
        self.history.borrow_mut().begin_session(
            pane_id,
            &util::shell_name(&shell_path),
            cwd.as_deref().and_then(|path| path.to_str()),
        );
        self.persist_history();
        self.set_focused_pane(pane_id);
        self.rebuild_layout();
    }

    fn smart_split_axis(&self, pane_id: u64) -> SplitAxis {
        let depth = self.layout.borrow().leaf_depth(pane_id).unwrap_or(0);

        if let Some(rect) = self.pane_rect(pane_id) {
            let width = rect.width();
            let height = rect.height();
            if width > 0.0 && height > 0.0 {
                let ratio = width / height;
                if ratio >= 1.24 {
                    return SplitAxis::Vertical;
                }
                if ratio <= 0.92 {
                    return SplitAxis::Horizontal;
                }
            }
        }

        if depth % 2 == 0 {
            SplitAxis::Vertical
        } else {
            SplitAxis::Horizontal
        }
    }

    fn smart_insert_position(&self, pane_id: u64) -> InsertPosition {
        if self.layout.borrow().leaf_depth(pane_id).unwrap_or(0) % 2 == 0 {
            InsertPosition::After
        } else {
            InsertPosition::Before
        }
    }

    fn refresh_pane_density(&self) {
        let pane_count = self.layout.borrow().leaf_count();
        let zoomed = self.zoomed_pane.get();
        let focused = self.focused_pane.get();

        for (pane_id, pane) in self.panes.borrow().iter() {
            let active = Some(*pane_id) == focused;
            let compact = zoomed.is_none() && pane_count >= 4;
            let dense = zoomed.is_none() && pane_count >= 7;
            let wallpaper_visible = zoomed == Some(*pane_id) || pane_count <= 6 || active;
            pane.set_density(compact, dense, wallpaper_visible);
        }
    }

    fn pane_rect(&self, pane_id: u64) -> Option<gtk::graphene::Rect> {
        self.panes
            .borrow()
            .get(&pane_id)
            .and_then(|pane| pane.widget().compute_bounds(&self.layout_host))
    }

    fn finish_close_pane(self: &Rc<Self>, pane_id: u64) {
        if !self.panes.borrow().contains_key(&pane_id) {
            self.closing_panes.borrow_mut().remove(&pane_id);
            return;
        }

        self.end_session_for_pane(pane_id);

        if let Some(pane) = self.panes.borrow_mut().remove(&pane_id) {
            pane.detach_from_parent();
        }
        self.closing_panes.borrow_mut().remove(&pane_id);
        self.focus_history.borrow_mut().retain(|id| *id != pane_id);
        if self.zoomed_pane.get() == Some(pane_id) {
            self.zoomed_pane.set(None);
        }

        let _ = self.layout.borrow_mut().remove_leaf(pane_id);

        if self.panes.borrow().is_empty() || self.layout.borrow().leaf_count() == 0 {
            self.focused_pane.set(None);
            self.window.close();
            return;
        }

        if self
            .focused_pane
            .get()
            .is_some_and(|id| !self.panes.borrow().contains_key(&id))
        {
            if let Some(focus_id) = self
                .previous_focus_candidate(pane_id)
                .or_else(|| self.layout.borrow().first_leaf())
            {
                self.set_focused_pane(focus_id);
            } else {
                self.focused_pane.set(None);
            }
        }

        self.rebuild_layout();
    }

    fn remember_focus(&self, pane_id: u64) {
        let mut history = self.focus_history.borrow_mut();
        history.retain(|entry| *entry != pane_id);
        history.push(pane_id);
        if history.len() > 64 {
            let excess = history.len() - 64;
            history.drain(0..excess);
        }
    }

    fn previous_focus_candidate(&self, excluding: u64) -> Option<u64> {
        let panes = self.panes.borrow();
        let closing = self.closing_panes.borrow();
        self.focus_history
            .borrow()
            .iter()
            .rev()
            .copied()
            .find(|pane_id| {
                *pane_id != excluding && panes.contains_key(pane_id) && !closing.contains(pane_id)
            })
    }

    fn resolved_shell_path(&self) -> String {
        util::effective_shell_path(&self.config.borrow().shell_path)
    }

    fn persist_history(&self) {
        if let Err(error) = self.history_manager.save(&self.history.borrow()) {
            self.show_toast(&format!("No se pudo guardar el historial: {error}"));
        }
    }
}

fn pane_spawn_motion(axis: SplitAxis, position: InsertPosition) -> PaneSpawnMotion {
    match (axis, position) {
        (SplitAxis::Vertical, InsertPosition::Before) => PaneSpawnMotion::FromLeft,
        (SplitAxis::Vertical, InsertPosition::After) => PaneSpawnMotion::FromRight,
        (SplitAxis::Horizontal, InsertPosition::Before) => PaneSpawnMotion::FromTop,
        (SplitAxis::Horizontal, InsertPosition::After) => PaneSpawnMotion::FromBottom,
    }
}

fn pane_motion_from_direction(direction: Direction) -> PaneSpawnMotion {
    match direction {
        Direction::Left => PaneSpawnMotion::FromLeft,
        Direction::Right => PaneSpawnMotion::FromRight,
        Direction::Up => PaneSpawnMotion::FromTop,
        Direction::Down => PaneSpawnMotion::FromBottom,
    }
}

fn center_of(rect: &gtk::graphene::Rect) -> (f32, f32) {
    (
        rect.x() + rect.width() / 2.0,
        rect.y() + rect.height() / 2.0,
    )
}

fn overlap_span(start_a: f32, end_a: f32, start_b: f32, end_b: f32) -> f32 {
    (end_a.min(end_b) - start_a.max(start_b)).max(0.0)
}

fn build_palette_section_row(section: QuickActionSection, count: usize) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_activatable(false);
    row.set_selectable(false);
    row.add_css_class("palette-section-row");

    let container = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let icon = gtk::Image::from_icon_name(section.icon_name());
    icon.add_css_class("palette-section-icon");
    let label = gtk::Label::new(Some(section.label()));
    label.add_css_class("palette-section-label");
    label.set_xalign(0.0);
    label.set_hexpand(true);
    let count_label = gtk::Label::new(Some(&count.to_string()));
    count_label.add_css_class("palette-section-count");

    container.append(&icon);
    container.append(&label);
    container.append(&count_label);
    row.set_child(Some(&container));
    row
}

fn build_palette_prefix(section: QuickActionSection) -> gtk::Box {
    let wrapper = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    wrapper.add_css_class("palette-row-prefix");
    wrapper.add_css_class(&format!("section-{}", section.css_class()));
    wrapper.set_valign(gtk::Align::Center);

    let icon = gtk::Image::from_icon_name(section.icon_name());
    icon.add_css_class("palette-row-prefix-icon");
    wrapper.append(&icon);
    wrapper
}

fn build_palette_action_row(
    item: &QuickActionItem,
    section: QuickActionSection,
) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_activatable(true);
    row.add_css_class("palette-row");

    let container = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    container.set_margin_start(12);
    container.set_margin_end(12);
    container.set_margin_top(8);
    container.set_margin_bottom(8);
    container.append(&build_palette_prefix(section));

    let text_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    text_box.set_hexpand(true);

    let title = gtk::Label::new(Some(&item.title));
    title.add_css_class("title-4");
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let subtitle = gtk::Label::new(Some(&item.subtitle));
    subtitle.add_css_class("dim-label");
    subtitle.set_xalign(0.0);
    subtitle.set_wrap(true);

    text_box.append(&title);
    text_box.append(&subtitle);
    container.append(&text_box);

    let suffixes = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    suffixes.set_valign(gtk::Align::Center);

    if let Some(badge) = &item.badge {
        let label = gtk::Label::new(Some(badge));
        label.add_css_class("context-badge");
        suffixes.append(&label);
    }
    if matches!(item.target, ActionTarget::NewPane) {
        let label = gtk::Label::new(Some("NUEVO"));
        label.add_css_class("palette-target-badge");
        suffixes.append(&label);
    }

    container.append(&suffixes);
    row.set_child(Some(&container));
    row
}

fn build_palette_empty_row(empty_query: bool) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_activatable(false);
    row.set_selectable(false);
    row.add_css_class("palette-empty-row");

    let container = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let title = gtk::Label::new(Some(if empty_query {
        "No hay acciones disponibles"
    } else {
        "No hay coincidencias"
    }));
    title.add_css_class("palette-empty-title");
    title.set_xalign(0.0);

    let subtitle = gtk::Label::new(Some(if empty_query {
        "Activa las acciones rápidas o abre un panel con más contexto."
    } else {
        "Prueba otro texto o usa prefijos como `:theme`, `:swap` o una ruta."
    }));
    subtitle.add_css_class("palette-empty-subtitle");
    subtitle.set_xalign(0.0);

    container.append(&title);
    container.append(&subtitle);
    row.set_child(Some(&container));
    row
}
