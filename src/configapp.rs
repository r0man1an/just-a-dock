use gtk4::prelude::*;
use gtk4::Orientation;

use crate::config::{Config, HideMode, ThemePreference};
use crate::theme::{self, ColorScheme};

const ICON_SIZE_OPTIONS: [u32; 5] = [40, 48, 53, 64, 72];

const OPACITY_OPTIONS: [(&str, f64); 5] = [
    ("None (opaque)", 1.0),
    ("Low", 0.9),
    ("Medium", 0.7),
    ("High", 0.45),
    ("Very high", 0.22),
];

fn apply_theme_preference(preference: ThemePreference) {
    let prefer_dark = match preference {
        ThemePreference::Light => false,
        ThemePreference::Dark => true,
        ThemePreference::System => theme::init().0 == ColorScheme::Dark,
    };
    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(prefer_dark);
    }
}

fn setting_row(label: &str, control: &impl IsA<gtk4::Widget>) -> gtk4::Box {
    let row = gtk4::Box::new(Orientation::Horizontal, 12);
    let text = gtk4::Label::new(Some(label));
    text.set_halign(gtk4::Align::Start);
    text.set_xalign(0.0);
    text.set_hexpand(true);
    row.append(&text);
    control.set_halign(gtk4::Align::End);
    control.set_valign(gtk4::Align::Center);
    row.append(control);
    row
}

fn settings_page() -> gtk4::Box {
    let page = gtk4::Box::new(Orientation::Vertical, 12);
    page.set_margin_top(18);
    page.set_margin_bottom(18);
    page.set_margin_start(18);
    page.set_margin_end(18);
    page
}

pub fn build_ui(app: &gtk4::Application) {
    let config = Config::load();
    let default_icon_size = Config::default().icon_size;
    apply_theme_preference(config.theme);

    let window = gtk4::ApplicationWindow::builder()
        .application(app)
        .title("JustADock Settings")
        .default_width(360)
        .default_height(340)
        .resizable(false)
        .build();

    let theme_dropdown = gtk4::DropDown::from_strings(&["System", "Light", "Dark"]);
    theme_dropdown.set_selected(match config.theme {
        ThemePreference::System => 0,
        ThemePreference::Light => 1,
        ThemePreference::Dark => 2,
    });
    theme_dropdown.connect_selected_notify(|dropdown| {
        let preference = match dropdown.selected() {
            1 => ThemePreference::Light,
            2 => ThemePreference::Dark,
            _ => ThemePreference::System,
        };
        apply_theme_preference(preference);
    });

    let size_labels: Vec<String> = ICON_SIZE_OPTIONS
        .iter()
        .map(|size| match *size == default_icon_size {
            true => format!("{size}px (default)"),
            false => format!("{size}px"),
        })
        .collect();
    let size_label_refs: Vec<&str> = size_labels.iter().map(String::as_str).collect();
    let size_dropdown = gtk4::DropDown::from_strings(&size_label_refs);
    let size_index = ICON_SIZE_OPTIONS
        .iter()
        .position(|size| *size == config.icon_size)
        .unwrap_or(0);
    size_dropdown.set_selected(size_index as u32);

    let default_opacity = Config::default().opacity;
    let opacity_labels: Vec<String> = OPACITY_OPTIONS
        .iter()
        .map(|(name, value)| match (*value - default_opacity).abs() < f64::EPSILON {
            true => format!("{name} (default)"),
            false => (*name).to_string(),
        })
        .collect();
    let opacity_label_refs: Vec<&str> = opacity_labels.iter().map(String::as_str).collect();
    let opacity_dropdown = gtk4::DropDown::from_strings(&opacity_label_refs);
    let opacity_index = OPACITY_OPTIONS
        .iter()
        .enumerate()
        .min_by(|(_, (_, a)), (_, (_, b))| {
            (*a - config.opacity)
                .abs()
                .total_cmp(&(*b - config.opacity).abs())
        })
        .map(|(index, _)| index)
        .unwrap_or(0);
    opacity_dropdown.set_selected(opacity_index as u32);

    let hide_mode_dropdown = gtk4::DropDown::from_strings(
        &[HideMode::Disabled, HideMode::Maximized, HideMode::Timed].map(HideMode::label),
    );
    hide_mode_dropdown.set_selected(config.hide_mode as u32);

    let trash_switch = gtk4::Switch::new();
    trash_switch.set_active(config.show_trash);

    let devices_switch = gtk4::Switch::new();
    devices_switch.set_active(config.show_devices);

    let style_page = settings_page();
    style_page.append(&setting_row("Theme", &theme_dropdown));
    style_page.append(&setting_row("Dock size", &size_dropdown));
    style_page.append(&setting_row("Transparency", &opacity_dropdown));

    let behaviour_page = settings_page();
    behaviour_page.append(&setting_row("Hide mode", &hide_mode_dropdown));
    behaviour_page.append(&setting_row("Show trash icon", &trash_switch));
    behaviour_page.append(&setting_row("Show removable devices", &devices_switch));

    let stack = gtk4::Stack::new();
    stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
    stack.add_titled(&style_page, Some("style"), "Style");
    stack.add_titled(&behaviour_page, Some("behaviour"), "Behaviour");

    let switcher = gtk4::StackSwitcher::new();
    switcher.set_stack(Some(&stack));
    switcher.set_halign(gtk4::Align::Center);

    let root = gtk4::Box::new(Orientation::Vertical, 0);

    let header = gtk4::Box::new(Orientation::Vertical, 0);
    header.set_margin_top(14);
    header.set_margin_bottom(6);
    header.append(&switcher);
    root.append(&header);

    root.append(&stack);

    let spacer = gtk4::Box::new(Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    root.append(&spacer);

    let button_row = gtk4::Box::new(Orientation::Horizontal, 8);
    button_row.set_halign(gtk4::Align::End);
    button_row.set_margin_bottom(16);
    button_row.set_margin_end(18);
    button_row.set_margin_start(18);
    button_row.set_margin_top(4);
    let cancel_button = gtk4::Button::with_label("Cancel");
    let save_button = gtk4::Button::with_label("Save");
    save_button.add_css_class("suggested-action");
    button_row.append(&cancel_button);
    button_row.append(&save_button);
    root.append(&button_row);

    window.set_child(Some(&root));

    cancel_button.connect_clicked({
        let window = window.clone();
        move |_| window.close()
    });

    save_button.connect_clicked({
        let window = window.clone();
        let theme_dropdown = theme_dropdown.clone();
        let size_dropdown = size_dropdown.clone();
        let opacity_dropdown = opacity_dropdown.clone();
        let hide_mode_dropdown = hide_mode_dropdown.clone();
        let trash_switch = trash_switch.clone();
        let devices_switch = devices_switch.clone();
        move |_| {
            let mut config = Config::load();
            config.theme = match theme_dropdown.selected() {
                1 => ThemePreference::Light,
                2 => ThemePreference::Dark,
                _ => ThemePreference::System,
            };
            config.icon_size = ICON_SIZE_OPTIONS
                .get(size_dropdown.selected() as usize)
                .copied()
                .unwrap_or(default_icon_size);
            config.opacity = OPACITY_OPTIONS
                .get(opacity_dropdown.selected() as usize)
                .map(|(_, value)| *value)
                .unwrap_or(default_opacity);
            config.hide_mode = match hide_mode_dropdown.selected() {
                1 => HideMode::Maximized,
                2 => HideMode::Timed,
                _ => HideMode::Disabled,
            };
            config.show_trash = trash_switch.is_active();
            config.show_devices = devices_switch.is_active();
            config.save();
            restart_dock();
            window.close();
        }
    });

    window.present();
}

fn restart_dock() {
    let Ok(current_exe) = std::env::current_exe() else {
        return;
    };
    let my_pid = std::process::id();

    if let Ok(entries) = std::fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let Some(pid) = entry.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) else {
                continue;
            };
            if pid == my_pid {
                continue;
            }
            let Ok(exe) = std::fs::read_link(entry.path().join("exe")) else {
                continue;
            };
            if exe != current_exe {
                continue;
            }
            let Ok(cmdline) = std::fs::read(entry.path().join("cmdline")) else {
                continue;
            };
            let is_config_window = cmdline.split(|&b| b == 0).any(|arg| arg == b"--config");
            if is_config_window {
                continue;
            }
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
        }
    }

    std::thread::sleep(std::time::Duration::from_millis(150));
    let _ = std::process::Command::new(current_exe).spawn();
}
