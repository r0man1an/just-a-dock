use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct DesktopEntry {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub exec: Option<String>,
    pub terminal: bool,
    pub startup_wm_class: Option<String>,
    pub actions: Vec<DesktopAction>,
}

#[derive(Debug, Clone)]
pub struct DesktopAction {
    pub name: String,
    pub exec: Option<String>,
}

pub struct DesktopEntryStore {
    by_id: HashMap<String, DesktopEntry>,
}

impl DesktopEntryStore {
    pub fn scan() -> Self {
        let mut by_id = HashMap::new();
        for dir in application_dirs() {
            scan_dir(&dir, &dir, &mut by_id);
        }
        DesktopEntryStore { by_id }
    }

    pub fn get(&self, id: &str) -> Option<&DesktopEntry> {
        self.by_id.get(id)
    }

    pub fn find_by_app_id(&self, app_id: &str) -> Option<&DesktopEntry> {
        let needle = app_id.to_lowercase();

        if let Some(entry) = self.get(&format!("{app_id}.desktop")) {
            return Some(entry);
        }

        self.by_id
            .values()
            .find(|e| {
                e.startup_wm_class
                    .as_deref()
                    .map(|c| c.to_lowercase() == needle)
                    .unwrap_or(false)
            })
            .or_else(|| {
                self.by_id.values().find(|e| {
                    e.id.trim_end_matches(".desktop").to_lowercase() == needle
                })
            })
            .or_else(|| {
                let last = needle.rsplit('.').next().unwrap_or(&needle);
                self.by_id.values().find(|e| {
                    let stem = e.id.trim_end_matches(".desktop").to_lowercase();
                    stem == last || stem.rsplit('.').next() == Some(last)
                })
            })
    }
}

fn application_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(data_home) = dirs_next_data_home() {
        dirs.push(data_home.join("applications"));
    }

    let data_dirs = std::env::var("XDG_DATA_DIRS")
        .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
    for dir in data_dirs.split(':').filter(|s| !s.is_empty()) {
        dirs.push(PathBuf::from(dir).join("applications"));
    }

    dirs
}

fn dirs_next_data_home() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg));
        }
    }
    dirs::data_dir()
}

fn scan_dir(root: &Path, dir: &Path, out: &mut HashMap<String, DesktopEntry>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir(root, &path, out);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
            continue;
        }
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        let id = rel.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "-");
        if out.contains_key(&id) {
            continue;
        }
        if let Some(parsed) = parse_desktop_file(&path, id.clone()) {
            out.insert(id, parsed);
        }
    }
}

fn parse_desktop_file(path: &Path, id: String) -> Option<DesktopEntry> {
    let raw = fs::read_to_string(path).ok()?;

    let mut sections: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current: Option<String> = None;

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            current = Some(name.to_string());
            continue;
        }
        let Some(section) = current.as_ref() else {
            continue;
        };
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        sections
            .entry(section.clone())
            .or_default()
            .insert(key.trim().to_string(), value.trim().to_string());
    }

    let main = sections.get("Desktop Entry")?;
    if main.get("Type").map(String::as_str) != Some("Application") {
        return None;
    }

    let actions = main
        .get("Actions")
        .map(|list| {
            list.split(';')
                .filter(|action_id| !action_id.is_empty())
                .filter_map(|action_id| {
                    let section = sections.get(&format!("Desktop Action {action_id}"))?;
                    Some(DesktopAction {
                        name: section.get("Name")?.clone(),
                        exec: section.get("Exec").cloned(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Some(DesktopEntry {
        id,
        name: main.get("Name")?.clone(),
        icon: main.get("Icon").cloned(),
        exec: main.get("Exec").cloned(),
        terminal: main
            .get("Terminal")
            .is_some_and(|v| v.eq_ignore_ascii_case("true")),
        startup_wm_class: main.get("StartupWMClass").cloned(),
        actions,
    })
}

pub fn launch(entry: &DesktopEntry) -> std::io::Result<()> {
    run_exec(entry.exec.as_deref(), entry)
}

pub fn launch_action(entry: &DesktopEntry, action: &DesktopAction) -> std::io::Result<()> {
    run_exec(action.exec.as_deref(), entry)
}

fn run_exec(exec: Option<&str>, entry: &DesktopEntry) -> std::io::Result<()> {
    let exec = exec.ok_or_else(|| std::io::Error::other("desktop entry has no Exec line"))?;

    let tokens = shlex::split(exec)
        .ok_or_else(|| std::io::Error::other("could not parse Exec line"))?;

    let args: Vec<String> = tokens
        .into_iter()
        .filter(|t| !matches!(t.as_str(), "%f" | "%F" | "%u" | "%U" | "%d" | "%D" | "%n" | "%N" | "%v" | "%m"))
        .map(|t| match t.as_str() {
            "%i" => entry.icon.clone().map(|i| format!("--icon {i}")).unwrap_or_default(),
            "%c" => entry.name.clone(),
            "%k" => String::new(),
            "%%" => "%".to_string(),
            other => other.to_string(),
        })
        .filter(|t| !t.is_empty())
        .collect();

    if args.is_empty() {
        return Err(std::io::Error::other("Exec line had no command after stripping field codes"));
    }

    let (program, rest): (&str, &[String]) = if entry.terminal {
        let term = std::env::var("TERMINAL").unwrap_or_else(|_| "x-terminal-emulator".to_string());
        let mut cmd = vec![term, "-e".to_string()];
        cmd.extend(args);
        return spawn_detached(&cmd[0], &cmd[1..]);
    } else {
        (&args[0], &args[1..])
    };

    spawn_detached(program, rest)
}

fn spawn_detached(program: &str, args: &[String]) -> std::io::Result<()> {
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}
