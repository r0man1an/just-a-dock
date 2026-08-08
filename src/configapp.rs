use gtk4::prelude::*;
use gtk4::Orientation;

use crate::config::{Config, HideMode, ThemePreference};
use crate::theme::{self, ColorScheme};

const ICON_SIZE_OPTIONS: [u32; 5] = [40, 48, 53, 64, 72];

const OPACITY_OPTIONS: [(&str, f64); 5] = [
    ("None (opaque)", 1.0),
    ("Low", 0.65),
    ("Medium", 0.35),
    ("High", 0.2),
    ("Very high", 0.1),
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

pub fn build_ui(app: &gtk4::Application) {
    let config = Config::load();
    let default_icon_size = Config::default().icon_size;
    apply_theme_preference(config.theme);

    let window = gtk4::ApplicationWindow::builder()
        .application(app)
        .title("JustADock Settings")
        .default_width(280)
        .default_height(430)
        .resizable(false)
        .build();

    let root = gtk4::Box::new(Orientation::Vertical, 12);
    root.set_margin_top(20);
    root.set_margin_bottom(20);
    root.set_margin_start(20);
    root.set_margin_end(20);

    let theme_label = gtk4::Label::new(Some("Theme"));
    theme_label.set_halign(gtk4::Align::Start);
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

    let size_label = gtk4::Label::new(Some("Dock size"));
    size_label.set_halign(gtk4::Align::Start);
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

    let hide_mode_label = gtk4::Label::new(Some("Hide mode"));
    hide_mode_label.set_halign(gtk4::Align::Start);
    let hide_mode_dropdown = gtk4::DropDown::from_strings(&[
        HideMode::Disabled, 
        HideMode::Maximized, 
        HideMode::Timed
    ].map(HideMode::label));
    hide_mode_dropdown.set_selected(config.hide_mode as u32);

    let opacity_label = gtk4::Label::new(Some("Dock transparency"));
    opacity_label.set_halign(gtk4::Align::Start);
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

    root.append(&theme_label);
    root.append(&theme_dropdown);
    root.append(&size_label);
    root.append(&size_dropdown);
    root.append(&hide_mode_label);
    root.append(&hide_mode_dropdown);
    root.append(&opacity_label);
    root.append(&opacity_dropdown);

    let spacer = gtk4::Box::new(Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    root.append(&spacer);

    let button_row = gtk4::Box::new(Orientation::Horizontal, 8);
    button_row.set_halign(gtk4::Align::End);
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
        let hide_mode_dropdown = hide_mode_dropdown.clone();
        let opacity_dropdown = opacity_dropdown.clone();
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
            config.hide_mode = match hide_mode_dropdown.selected() {
                1 => crate::config::HideMode::Maximized,
                2 => crate::config::HideMode::Timed,
                _ => crate::config::HideMode::Disabled,
            };
            config.opacity = OPACITY_OPTIONS
                .get(opacity_dropdown.selected() as usize)
                .map(|(_, value)| *value)
                .unwrap_or(default_opacity);
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
