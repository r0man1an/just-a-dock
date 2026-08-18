use std::path::PathBuf;
use std::process::{Command, Stdio};

fn data_home() -> PathBuf {
    if let Some(value) = std::env::var_os("XDG_DATA_HOME") {
        if !value.is_empty() {
            return PathBuf::from(value);
        }
    }
    dirs::data_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn trash_root() -> PathBuf {
    data_home().join("Trash")
}

fn files_dir() -> PathBuf {
    trash_root().join("files")
}

pub fn is_empty() -> bool {
    match std::fs::read_dir(files_dir()) {
        Ok(mut entries) => entries.next().is_none(),
        Err(_) => true,
    }
}

pub fn icon_name() -> &'static str {
    if is_empty() {
        "user-trash"
    } else {
        "user-trash-full"
    }
}

pub fn monitor_dir() -> Option<PathBuf> {
    [files_dir(), trash_root(), data_home()]
        .into_iter()
        .find(|path| path.is_dir())
}

pub fn open() -> std::io::Result<()> {
    spawn_detached("xdg-open", &["trash:///"])
}

pub fn empty() -> std::io::Result<()> {
    spawn_detached("gio", &["trash", "--empty"])
}

fn spawn_detached(program: &str, args: &[&str]) -> std::io::Result<()> {
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}
