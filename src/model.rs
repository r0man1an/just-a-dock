use std::collections::HashMap;

use crate::desktop::DesktopEntryStore;
use crate::toplevel::{ToplevelId, ToplevelInfo};

#[derive(Debug, Clone)]
pub struct DockItem {
    pub key: String,
    pub name: String,
    pub icon_name: Option<String>,
    pub launch_desktop_id: Option<String>,
    pub windows: Vec<ToplevelId>,
}

pub struct DockModel {
    windows: HashMap<ToplevelId, ToplevelInfo>,
    window_key: HashMap<ToplevelId, String>,
    open_order: Vec<String>,
}

impl DockModel {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            window_key: HashMap::new(),
            open_order: Vec::new(),
        }
    }

    pub fn upsert_window(&mut self, info: ToplevelInfo, desktop: &DesktopEntryStore) {
        let key = desktop
            .find_by_app_id(&info.app_id)
            .map(|e| e.id.clone())
            .unwrap_or_else(|| info.app_id.to_lowercase());

        if !self.open_order.contains(&key) {
            self.open_order.push(key.clone());
        }
        self.window_key.insert(info.id, key);
        self.windows.insert(info.id, info);
    }

    pub fn remove_window(&mut self, id: ToplevelId) {
        self.windows.remove(&id);
        if let Some(key) = self.window_key.remove(&id) {
            let still_open = self.window_key.values().any(|k| *k == key);
            if !still_open {
                self.open_order.retain(|k| *k != key);
            }
        }
    }

    pub fn build_items(&self, pinned: &[String], desktop: &DesktopEntryStore) -> Vec<DockItem> {
        let mut items = Vec::new();

        for desktop_id in pinned {
            let entry = desktop.get(desktop_id);
            let windows = self.windows_for_key(desktop_id);
            let icon_name = entry
                .and_then(|e| e.icon.clone())
                .or_else(|| Some(desktop_id.trim_end_matches(".desktop").to_string()));
            items.push(DockItem {
                key: desktop_id.clone(),
                name: entry
                    .map(|e| e.name.clone())
                    .unwrap_or_else(|| desktop_id.clone()),
                icon_name,
                launch_desktop_id: entry.map(|e| e.id.clone()),
                windows,
            });
        }

        for key in &self.open_order {
            if pinned.iter().any(|p| p == key) {
                continue;
            }
            let windows = self.windows_for_key(key);
            if windows.is_empty() {
                continue;
            }
            let entry = desktop.get(key);
            let raw_app_id = self.windows.get(&windows[0]).map(|w| w.app_id.clone());
            let fallback_name = raw_app_id.clone().unwrap_or_else(|| key.clone());
            let icon_name = entry.and_then(|e| e.icon.clone()).or(raw_app_id);
            items.push(DockItem {
                key: key.clone(),
                name: entry.map(|e| e.name.clone()).unwrap_or(fallback_name),
                icon_name,
                launch_desktop_id: entry.map(|e| e.id.clone()),
                windows,
            });
        }

        items
    }

    fn windows_for_key(&self, key: &str) -> Vec<ToplevelId> {
        let mut ids: Vec<ToplevelId> = self
            .window_key
            .iter()
            .filter(|(_, k)| k.as_str() == key)
            .map(|(id, _)| *id)
            .collect();
        ids.sort_unstable();
        ids
    }

    pub fn is_activated(&self, id: ToplevelId) -> bool {
        self.windows.get(&id).map(|w| w.activated).unwrap_or(false)
    }

    pub fn any_activated(&self, windows: &[ToplevelId]) -> bool {
        windows.iter().any(|id| self.is_activated(*id))
    }

    pub fn window_title(&self, id: ToplevelId) -> Option<&str> {
        self.windows
            .get(&id)
            .map(|w| w.title.as_str())
            .filter(|t| !t.is_empty())
    }

    pub fn should_dodge(&self, dock_output: Option<&str>) -> bool {
        self.windows.values().any(|w| {
            w.activated && (w.maximized || w.fullscreen) && self.on_dock_output(w, dock_output)
        })
    }

    fn on_dock_output(&self, w: &ToplevelInfo, dock_output: Option<&str>) -> bool {
        match dock_output {
            None => true,
            Some(_) if w.outputs.is_empty() => true,
            Some(name) => w.outputs.iter().any(|o| o.as_str() == name),
        }
    }
}
