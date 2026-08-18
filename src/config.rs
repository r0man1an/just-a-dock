use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StyleVariant {
    Round,
    Straight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DockEdge {
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum HideMode {
    Disabled,
    #[default]
    Maximized,
    Timed,
}

impl HideMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Disabled => "Disabled",
            Self::Maximized => "Maximized",
            Self::Timed => "Timed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub icon_size: u32,
    pub style: StyleVariant,
    pub edge: DockEdge,
    pub theme: ThemePreference,
    pub edge_margin: u32,
    pub opacity: f64,
    pub pinned: Vec<String>,
    pub monitor: Option<String>,
    pub hide_mode: HideMode,
    pub show_trash: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            icon_size: 53,
            style: StyleVariant::Round,
            edge: DockEdge::Bottom,
            theme: ThemePreference::System,
            edge_margin: 10,
            opacity: 0.7,
            pinned: vec![
                "firefox.desktop".into(),
                "org.gnome.Nautilus.desktop".into(),
            ],
            monitor: None,
            hide_mode: HideMode::default(),
            show_trash: true,
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("jdock").join("config.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        match fs::read_to_string(&path) {
            Ok(raw) => match toml::from_str(&raw) {
                Ok(cfg) => cfg,
                Err(err) => {
                    eprintln!("jdock: failed to parse {path:?}: {err}; using defaults");
                    Config::default()
                }
            },
            Err(_) => {
                let cfg = Config::default();
                cfg.write_default(&path);
                cfg
            }
        }
    }

    pub fn save(&self) {
        self.write_default(&Self::config_path());
    }

    fn write_default(&self, path: &PathBuf) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if let Ok(toml_str) = toml::to_string_pretty(self) {
            let _ = fs::write(path, toml_str);
        }
    }
}
