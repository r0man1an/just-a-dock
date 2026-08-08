use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use calloop::channel::Sender as CalloopSender;
use gtk4::prelude::*;
use gtk4::{glib, Orientation};
use gtk4_layer_shell::{Layer, LayerShell};

use crate::config::{Config, DockEdge, HideMode, StyleVariant, ThemePreference};
use crate::desktop::{DesktopAction, DesktopEntry, DesktopEntryStore};
use crate::geometry::Geometry;
use crate::icon;
use crate::layer;
use crate::model::{DockItem, DockModel};
use crate::style;
use crate::theme::{self, ColorScheme};
use crate::toplevel::{self, Command, ToplevelEvent};

const PEEK_PX: i32 = 6;
const DODGE_HIDE_DELAY: Duration = Duration::from_millis(350); // grace period before a dodging dock collapses - NOTE: test different values down the line
const COLLAPSE_FADE_DELAY: Duration = Duration::from_millis(200);
const DODGE_STARTUP_DELAY: Duration = Duration::from_millis(500);

struct AppInner {
    config: Config,
    desktop: DesktopEntryStore,
    model: DockModel,
    scheme: ColorScheme,
    geometry: Geometry,
    displayed_keys: Vec<String>,
    indicators: HashMap<String, gtk4::Box>,
    css_provider: gtk4::CssProvider,
    window: gtk4::ApplicationWindow,
    content: gtk4::Box,
    hide_timer: Option<glib::SourceId>,
    dodge: bool,
    hidden: bool,
    started: bool,
    tooltip_window: gtk4::ApplicationWindow,
    tooltip_label: gtk4::Label,
    menu_window: gtk4::ApplicationWindow,
    menu_box: gtk4::Box,
    display: gtk4::gdk::Display,
    cmd_tx: CalloopSender<Command>,
    _theme_proxy: Option<gtk4::gio::DBusProxy>,
}

pub fn build_ui(app: &gtk4::Application) {
    if !gtk4_layer_shell::is_supported() {
        eprintln!(
            "jdock: this compositor doesn't support wlr-layer-shell; \
             this dock only works on wlroots-based Wayland compositors (niri, wayfire, labwc, ...)"
        );
        std::process::exit(1);
    }

    let config = Config::load();
    let desktop = DesktopEntryStore::scan();

    eprintln!("jdock: {} hiding mode", config.hide_mode.label());

    let window = gtk4::ApplicationWindow::builder()
        .application(app)
        .decorated(false)
        .resizable(false)
        .build();

    layer::init(&window, &config);

    let display = gtk4::prelude::WidgetExt::display(&window);
    if let Some(name) = &config.monitor {
        if let Some(monitor) = find_monitor(&display, name) {
            window.set_monitor(Some(&monitor));
        }
    }

    let orientation = match config.edge {
        DockEdge::Bottom => Orientation::Horizontal,
        DockEdge::Left | DockEdge::Right => Orientation::Vertical,
    };
    let content = gtk4::Box::new(orientation, 0);
    content.add_css_class("dock-background");
    window.set_child(Some(&content));

    let css_provider = gtk4::CssProvider::new();
    gtk4::style_context_add_provider_for_display(
        &display,
        &css_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let tooltip_label = gtk4::Label::new(None);
    tooltip_label.add_css_class("dock-tooltip");

    let tooltip_window = gtk4::ApplicationWindow::builder()
        .application(app)
        .decorated(false)
        .resizable(false)
        .build();
    tooltip_window.set_child(Some(&tooltip_label));
    tooltip_window.init_layer_shell();
    tooltip_window.set_layer(Layer::Overlay);
    tooltip_window.set_namespace(Some("jdock-tooltip"));
    tooltip_window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);
    tooltip_window.set_exclusive_zone(0);
    tooltip_window.set_anchor(layer::to_layer_edge(config.edge), true);
    tooltip_window.set_anchor(layer::tooltip_primary_edge(config.edge), true);


    if let Some(name) = &config.monitor {
        if let Some(monitor) = find_monitor(&display, name) {
            tooltip_window.set_monitor(Some(&monitor));
        }
    }
    tooltip_window.present();
    tooltip_window.set_visible(false);

    let menu_box = gtk4::Box::new(Orientation::Vertical, 2);
    menu_box.add_css_class("dock-menu-list");

    let menu_window = gtk4::ApplicationWindow::builder()
        .application(app)
        .decorated(false)
        .resizable(false)
        .build();
    menu_window.set_child(Some(&menu_box));
    menu_window.init_layer_shell();
    menu_window.set_layer(Layer::Overlay);
    menu_window.set_namespace(Some("jdock-menu"));
    menu_window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::OnDemand);
    menu_window.set_exclusive_zone(0);
    menu_window.set_anchor(layer::to_layer_edge(config.edge), true);
    menu_window.set_anchor(layer::tooltip_primary_edge(config.edge), true);

    if let Some(name) = &config.monitor {
        if let Some(monitor) = find_monitor(&display, name) {
            menu_window.set_monitor(Some(&monitor));
        }
    }
    let menu_key_controller = gtk4::EventControllerKey::new();
    menu_window.add_controller(menu_key_controller.clone());
    menu_window.present();
    menu_window.set_visible(false);

    let (events_tx, events_rx) = async_channel::unbounded::<ToplevelEvent>();
    let cmd_tx = toplevel::spawn(events_tx);

    let (initial_scheme, theme_proxy) = match config.theme {
        ThemePreference::System => theme::init(),
        ThemePreference::Light => (ColorScheme::Light, None),
        ThemePreference::Dark => (ColorScheme::Dark, None),
    };

    let inner = Rc::new(RefCell::new(AppInner {
        config,
        desktop,
        model: DockModel::new(),
        scheme: initial_scheme,
        geometry: Geometry::compute(&Config::default(), 0, 1, 1),
        displayed_keys: Vec::new(),
        indicators: HashMap::new(),
        css_provider,
        window: window.clone(),
        content,
        hide_timer: None,
        dodge: false,
        hidden: false,
        started: false,
        tooltip_window,
        tooltip_label,
        menu_window,
        menu_box,
        display,
        cmd_tx,
        _theme_proxy: theme_proxy.clone(),
    }));

    sync(&inner);

    inner.borrow().menu_window.connect_notify_local(Some("is-active"), {
        let inner = inner.clone();
        move |window, _| {
            if !window.is_active() {
                dismiss_menu(&inner);
            }
        }
    });
    menu_key_controller.connect_key_pressed({
        let inner = inner.clone();
        move |_, keyval, _, _| {
            if keyval == gtk4::gdk::Key::Escape {
                dismiss_menu(&inner);
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        }
    });

    if inner.borrow().config.hide_mode != HideMode::Disabled {
        let dock_motion = gtk4::EventControllerMotion::new();
        dock_motion.connect_enter({
            let inner = inner.clone();
            move |_, _, _| {
                let Ok(mut guard) = inner.try_borrow_mut() else { return };
                if let Some(id) = guard.hide_timer.take() {
                    id.remove();
                }
                if guard.hidden {
                    drop(guard);
                    show_now(&inner);
                }
            }
        });
        dock_motion.connect_leave({
            let inner = inner.clone();
            move |_| schedule_hide(&inner)
        });
        window.add_controller(dock_motion);
    }

    if let Some(proxy) = &theme_proxy {
        theme::subscribe(proxy, {
            let inner = inner.clone();
            move |scheme| set_scheme(&inner, scheme)
        });
    }

    glib::spawn_future_local({
        let inner = inner.clone();
        async move {
            while let Ok(event) = events_rx.recv().await {
                {
                    let mut guard = inner.borrow_mut();
                    let state = &mut *guard;
                    match event {
                        ToplevelEvent::Updated(info) => {
                            if state.config.hide_mode == HideMode::Maximized {
                                eprintln!(
                                    "jdock: toplevel {} app_id={} activated={} maximized={} fullscreen={}",
                                    info.id, info.app_id, info.activated, info.maximized, info.fullscreen
                                );
                            }
                            state.model.upsert_window(info, &state.desktop);
                        }
                        ToplevelEvent::Closed(id) => {
                            state.model.remove_window(id);
                        }
                    }
                }
                sync(&inner);
                update_dodge(&inner);
            }
        }
    });

    window.present();

    if inner.borrow().config.hide_mode != HideMode::Disabled {
        let inner = inner.clone();
        glib::timeout_add_local_once(DODGE_STARTUP_DELAY, move || {
            let mut guard = inner.borrow_mut();
            guard.started = true;
            let hide_mode = guard.config.hide_mode;
            let dodge = guard.dodge;
            drop(guard);
            if hide_mode == HideMode::Timed || (hide_mode == HideMode::Maximized && dodge) {
                schedule_hide(&inner);
            }
        });
    }
}

fn find_monitor(display: &gtk4::gdk::Display, connector: &str) -> Option<gtk4::gdk::Monitor> {
    let monitors = display.monitors();
    for i in 0..monitors.n_items() {
        let obj = monitors.item(i)?;
        let monitor = obj.downcast::<gtk4::gdk::Monitor>().ok()?;
        if monitor.connector().as_deref() == Some(connector) {
            return Some(monitor);
        }
    }
    None
}

fn set_scheme(inner: &Rc<RefCell<AppInner>>, scheme: ColorScheme) {
    let mut guard = inner.borrow_mut();
    guard.scheme = scheme;
    let css = style::generate_css(&guard.geometry, &guard.config, guard.scheme);
    guard.css_provider.load_from_data(&css);
}

fn sync(inner: &Rc<RefCell<AppInner>>) {
    let mut guard = inner.borrow_mut();
    let items = guard.model.build_items(&guard.config.pinned, &guard.desktop);
    let new_keys: Vec<String> = items.iter().map(|i| i.key.clone()).collect();

    let available_length = available_length(&guard);
    let scale_factor = primary_scale_factor(&guard);
    guard.geometry = Geometry::compute(&guard.config, items.len(), available_length, scale_factor);

    let css = style::generate_css(&guard.geometry, &guard.config, guard.scheme);
    guard.css_provider.load_from_data(&css);

    if new_keys != guard.displayed_keys {
        rebuild_row(&mut guard, &items, inner);
        guard.displayed_keys = new_keys;
    } else {
        for item in &items {
            update_indicator(&guard, item);
        }
    }
}

fn available_length(guard: &AppInner) -> i32 {
    let monitor = guard
        .config
        .monitor
        .as_ref()
        .and_then(|name| find_monitor(&guard.display, name))
        .or_else(|| {
            let monitors = guard.display.monitors();
            monitors
                .item(0)
                .and_then(|o| o.downcast::<gtk4::gdk::Monitor>().ok())
        });

    let Some(monitor) = monitor else {
        return 1920;
    };
    let geom = monitor.geometry();
    match guard.config.edge {
        DockEdge::Bottom => geom.width(),
        DockEdge::Left | DockEdge::Right => geom.height(),
    }
}

fn primary_scale_factor(guard: &AppInner) -> i32 {
    let monitors = guard.display.monitors();
    monitors
        .item(0)
        .and_then(|o| o.downcast::<gtk4::gdk::Monitor>().ok())
        .map(|m| m.scale_factor())
        .unwrap_or(1)
}

fn position_above_dock(
    guard: &AppInner,
    window: &gtk4::ApplicationWindow,
    measure_widget: &impl IsA<gtk4::Widget>,
    anchor: &impl IsA<gtk4::Widget>,
) {
    let primary_edge = layer::tooltip_primary_edge(guard.config.edge);
    let opposite_edge = match primary_edge {
        gtk4_layer_shell::Edge::Left => gtk4_layer_shell::Edge::Right,
        gtk4_layer_shell::Edge::Right => gtk4_layer_shell::Edge::Left,
        gtk4_layer_shell::Edge::Top => gtk4_layer_shell::Edge::Bottom,
        gtk4_layer_shell::Edge::Bottom => gtk4_layer_shell::Edge::Top,
        unknown_edge => {
            eprintln!("jdock warning: unhandled layer shell edge: {:?}", unknown_edge);
            gtk4_layer_shell::Edge::Top
        }
    };
    window.set_anchor(primary_edge, false);
    window.set_margin(primary_edge, 0);
    window.set_anchor(opposite_edge, false);
    window.set_margin(opposite_edge, 0);

    let gap = guard.geometry.thickness * 0.41 * 0.75;
    let cross_margin =
        (guard.config.edge_margin as f64 + guard.geometry.thickness + gap).round() as i32;
    window.set_margin(layer::to_layer_edge(guard.config.edge), cross_margin);

    let orientation = if guard.config.edge == DockEdge::Bottom {
        gtk4::Orientation::Horizontal
    } else {
        gtk4::Orientation::Vertical
    };

    measure_widget.set_margin_start(0);
    measure_widget.set_margin_end(0);
    measure_widget.set_margin_top(0);
    measure_widget.set_margin_bottom(0);

    let (content_size, anchor_pos, anchor_size) = if orientation == gtk4::Orientation::Horizontal {
        let (x, _) = anchor.translate_coordinates(&guard.content, 0.0, 0.0).unwrap_or((0.0, 0.0));
        (guard.content.width() as f64, x, anchor.width() as f64)
    } else {
        let (_, y) = anchor.translate_coordinates(&guard.content, 0.0, 0.0).unwrap_or((0.0, 0.0));
        (guard.content.height() as f64, y, anchor.height() as f64)
    };

    let dock_center = content_size / 2.0;
    let icon_center = anchor_pos + anchor_size / 2.0;
    let offset = icon_center - dock_center;

    // GTK centers the total size (widget + margin), so we double the offset to shift the visual center.
    let margin_pos = if offset > 0.0 { (offset * 2.0).round() as i32 } else { 0 };
    let margin_neg = if offset <= 0.0 { (offset.abs() * 2.0).round() as i32 } else { 0 };

    if orientation == gtk4::Orientation::Horizontal {
        measure_widget.set_halign(gtk4::Align::Center);
        measure_widget.set_margin_start(margin_pos);
        measure_widget.set_margin_end(margin_neg);
    } else {
        measure_widget.set_valign(gtk4::Align::Center);
        measure_widget.set_margin_top(margin_pos);
        measure_widget.set_margin_bottom(margin_neg);
    }
}

fn update_indicator(guard: &AppInner, item: &DockItem) {
    let Some(indicator) = guard.indicators.get(&item.key) else {
        return;
    };
    indicator.set_visible(!item.windows.is_empty());
    if guard.model.any_activated(&item.windows) {
        indicator.add_css_class("active");
    } else {
        indicator.remove_css_class("active");
    }
}

fn rebuild_row(guard: &mut AppInner, items: &[DockItem], inner: &Rc<RefCell<AppInner>>) {
    guard.tooltip_window.set_visible(false);
    guard.menu_window.set_visible(false);
    while let Some(child) = guard.content.first_child() {
        guard.content.remove(&child);
    }
    guard.indicators.clear();

    let (halign, valign) = match guard.config.edge {
        DockEdge::Bottom => (gtk4::Align::Center, gtk4::Align::End),
        DockEdge::Left => (gtk4::Align::Start, gtk4::Align::Center),
        DockEdge::Right => (gtk4::Align::End, gtk4::Align::Center),
    };

    for item in items.iter() {
        let overlay = gtk4::Overlay::new();
        overlay.add_css_class("dock-cell");

        let image = icon::build_icon_image(
            &guard.display,
            item.icon_name.as_deref(),
            guard.geometry.icon_render_size.round() as i32,
        );
        let button = gtk4::Button::new();
        button.add_css_class("dock-icon-button");
        button.add_css_class("flat");
        button.set_child(Some(&image));

        let tooltip_text = match item.windows.as_slice() {
            [single] => guard.model.window_title(*single).unwrap_or(&item.name),
            _ => &item.name,
        }
        .to_string();

        let motion = gtk4::EventControllerMotion::new();
        motion.connect_enter({
            let inner = inner.clone();
            let tooltip_text = tooltip_text.clone();
            let overlay = overlay.clone();
            move |_, _, _| {
                let Ok(guard) = inner.try_borrow() else { return };
                if guard.menu_window.is_visible() {
                    return;
                }
                guard.tooltip_label.set_text(&tooltip_text);
                position_above_dock(&guard, &guard.tooltip_window, &guard.tooltip_label, &overlay);
                guard.tooltip_window.set_visible(true);
            }
        });
        motion.connect_leave({
            let inner = inner.clone();
            move |_| {
                if let Ok(guard) = inner.try_borrow() {
                    guard.tooltip_window.set_visible(false);
                }
            }
        });
        button.add_controller(motion);
        overlay.set_child(Some(&button));

        let indicator = gtk4::Box::new(Orientation::Horizontal, 0);
        indicator.add_css_class("dock-indicator");
        indicator.set_halign(halign);
        indicator.set_valign(valign);
        indicator.set_visible(!item.windows.is_empty());
        if guard.model.any_activated(&item.windows) {
            indicator.add_css_class("active");
        }
        overlay.add_overlay(&indicator);

        button.connect_clicked({
            let inner = inner.clone();
            let key = item.key.clone();
            let launch_desktop_id = item.launch_desktop_id.clone();
            move |_| on_item_clicked(&inner, &key, launch_desktop_id.as_deref())
        });

        let middle_click = gtk4::GestureClick::new();
        middle_click.set_button(gtk4::gdk::BUTTON_MIDDLE);
        middle_click.connect_pressed({
            let inner = inner.clone();
            let windows = item.windows.clone();
            move |_, _, _, _| {
                if let Some(&id) = windows.first() {
                    let _ = inner.borrow().cmd_tx.send(Command::Close(id));
                }
            }
        });
        button.add_controller(middle_click);

        let right_click = gtk4::GestureClick::new();
        right_click.set_button(gtk4::gdk::BUTTON_SECONDARY);
        right_click.connect_pressed({
            let inner = inner.clone();
            let key = item.key.clone();
            let windows = item.windows.clone();
            let launch_desktop_id = item.launch_desktop_id.clone();
            let overlay = overlay.clone();
            move |_, _, _, _| {
                let guard = inner.borrow();
                guard.tooltip_window.set_visible(false);
                rebuild_menu(&guard, &inner, &key, &windows, launch_desktop_id.as_deref());
                position_above_dock(&guard, &guard.menu_window, &guard.menu_box, &overlay);
                guard.menu_window.set_visible(true);
                guard.menu_window.present();
            }
        });
        button.add_controller(right_click);

        guard.content.append(&overlay);
        guard.indicators.insert(item.key.clone(), indicator);
    }
}

fn rebuild_menu(
    guard: &AppInner,
    inner: &Rc<RefCell<AppInner>>,
    key: &str,
    windows: &[toplevel::ToplevelId],
    launch_desktop_id: Option<&str>,
) {
    while let Some(child) = guard.menu_box.first_child() {
        guard.menu_box.remove(&child);
    }

    let entry = launch_desktop_id.and_then(|id| guard.desktop.get(id)).cloned();

    if let Some(entry) = entry.filter(|e| !e.actions.is_empty()) {
        for action in entry.actions.clone() {
            let row = menu_row(&action.name);
            row.connect_clicked({
                let inner = inner.clone();
                let entry = entry.clone();
                move |_| run_action(&inner, &entry, &action)
            });
            guard.menu_box.append(&row);
        }
        append_menu_separator(&guard.menu_box);
    }

    let is_pinned = guard.config.pinned.iter().any(|p| p == key);
    let pin_row = menu_row(if is_pinned { "Unpin" } else { "Pin" });
    pin_row.connect_clicked({
        let inner = inner.clone();
        let key = key.to_string();
        move |_| toggle_pin(&inner, &key)
    });
    guard.menu_box.append(&pin_row);

    if !windows.is_empty() {
        let close_row = menu_row("Close");
        close_row.connect_clicked({
            let inner = inner.clone();
            let windows = windows.to_vec();
            move |_| close_item(&inner, &windows)
        });
        guard.menu_box.append(&close_row);
    }
}

fn menu_row(label: &str) -> gtk4::Button {
    let row = gtk4::Button::builder().label(label).build();
    row.add_css_class("flat");
    row.add_css_class("dock-menu-item");
    row
}

fn append_menu_separator(container: &gtk4::Box) {
    let separator = gtk4::Box::new(Orientation::Horizontal, 0);
    separator.add_css_class("dock-menu-separator");
    container.append(&separator);
}

fn toggle_pin(inner: &Rc<RefCell<AppInner>>, key: &str) {
    let mut guard = inner.borrow_mut();
    guard.menu_window.set_visible(false);
    if let Some(pos) = guard.config.pinned.iter().position(|p| p == key) {
        guard.config.pinned.remove(pos);
    } else {
        guard.config.pinned.push(key.to_string());
    }
    guard.config.save();
    drop(guard);
    sync(inner);
}

fn close_item(inner: &Rc<RefCell<AppInner>>, windows: &[toplevel::ToplevelId]) {
    let guard = inner.borrow();
    guard.menu_window.set_visible(false);
    for &id in windows {
        let _ = guard.cmd_tx.send(Command::Close(id));
    }
}

fn run_action(inner: &Rc<RefCell<AppInner>>, entry: &DesktopEntry, action: &DesktopAction) {
    let guard = inner.borrow();
    guard.menu_window.set_visible(false);
    if let Err(err) = crate::desktop::launch_action(entry, action) {
        eprintln!("jdock: failed to launch action \"{}\": {err}", action.name);
    }
}

fn on_item_clicked(inner: &Rc<RefCell<AppInner>>, key: &str, launch_desktop_id: Option<&str>) {
    let guard = inner.borrow();
    guard.menu_window.set_visible(false);
    let items = guard.model.build_items(&guard.config.pinned, &guard.desktop);
    let Some(item) = items.iter().find(|i| i.key == key) else {
        return;
    };

    if item.windows.is_empty() {
        if let Some(id) = launch_desktop_id {
            if let Some(entry) = guard.desktop.get(id) {
                if let Err(err) = crate::desktop::launch(entry) {
                    eprintln!("jdock: failed to launch {id}: {err}");
                }
            }
        }
        return;
    }

    if guard.model.any_activated(&item.windows) {
        if item.windows.len() > 1 {
            let next = item
                .windows
                .iter()
                .find(|id| !guard.model.is_activated(**id))
                .copied()
                .unwrap_or(item.windows[0]);
            let _ = guard.cmd_tx.send(Command::Activate(next));
        } else {
            let _ = guard.cmd_tx.send(Command::Minimize(item.windows[0]));
        }
    } else {
        let target = item
            .windows
            .iter()
            .find(|id| !guard.model.is_activated(**id))
            .copied()
            .unwrap_or(item.windows[0]);
        let _ = guard.cmd_tx.send(Command::Activate(target));
    }
}

fn dismiss_menu(inner: &Rc<RefCell<AppInner>>) {
    if let Ok(guard) = inner.try_borrow() {
        guard.menu_window.set_visible(false);
    }
    schedule_hide(inner);
}

fn css_gap(guard: &AppInner) -> i32 {
    match guard.config.style {
        StyleVariant::Round => guard.config.edge_margin as i32,
        StyleVariant::Straight => 0,
    }
}

fn collapsed_margin(guard: &AppInner) -> i32 {
    let extent = guard.geometry.thickness.round() as i32 + css_gap(guard);
    -(extent - PEEK_PX).max(0)
}

fn update_dodge(inner: &Rc<RefCell<AppInner>>) {
    let Ok(mut guard) = inner.try_borrow_mut() else { return };
    if guard.config.hide_mode != HideMode::Maximized {
        return;
    }
    let new_dodge = guard.model.should_dodge();
    if new_dodge == guard.dodge {
        return;
    }
    guard.dodge = new_dodge;
    if !guard.started {
        return;
    }
    if new_dodge {
       // needs further testing
        guard.window.set_layer(Layer::Overlay);
        drop(guard);
        hide_now(inner);
    } else {
        drop(guard);
        show_now(inner);
        if let Ok(guard) = inner.try_borrow() {
            guard.window.set_layer(Layer::Top);
        }
    }
}

fn hide_now(inner: &Rc<RefCell<AppInner>>) {
    let Ok(mut guard) = inner.try_borrow_mut() else { return };
    if let Some(id) = guard.hide_timer.take() {
        id.remove();
    }
    guard.hidden = true;
    guard.content.add_css_class("collapsed");
    drop(guard);
    let inner = inner.clone();
    glib::timeout_add_local_once(COLLAPSE_FADE_DELAY, move || {
        let Ok(guard) = inner.try_borrow() else { return };


        if !guard.hidden {
            return;
        }
        let margin = collapsed_margin(&guard);
        let edge = layer::to_layer_edge(guard.config.edge);
        guard.window.set_margin(edge, margin);
    });
}

fn show_now(inner: &Rc<RefCell<AppInner>>) {
    let Ok(mut guard) = inner.try_borrow_mut() else { return };
    if let Some(id) = guard.hide_timer.take() {
        id.remove();
    }
    guard.hidden = false;
    let edge = layer::to_layer_edge(guard.config.edge);
    guard.window.set_margin(edge, match guard.config.hide_mode {
        HideMode::Timed | HideMode::Maximized => 0,
        HideMode::Disabled => match guard.config.style {
            StyleVariant::Round => guard.config.edge_margin as i32,
            StyleVariant::Straight => 0,
        },
    });
    guard.content.remove_css_class("collapsed");
}

fn schedule_hide(inner: &Rc<RefCell<AppInner>>) {
    let Ok(mut guard) = inner.try_borrow_mut() else { return };
    if !guard.started {
        return;
    }
    if guard.menu_window.is_visible() {
        return;
    }
    
    match guard.config.hide_mode {
        HideMode::Disabled => return,
        HideMode::Maximized if !guard.dodge => return,
        _ => {}
    }

    if let Some(id) = guard.hide_timer.take() {
        id.remove();
    }
    let inner = inner.clone();
    guard.hide_timer = Some(glib::timeout_add_local_once(DODGE_HIDE_DELAY, move || {
        hide_now(&inner);
    }));
}
