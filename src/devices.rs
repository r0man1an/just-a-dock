use std::process::{Command, Stdio};

use gtk4::gio;
use gtk4::gio::prelude::*;
use gtk4::glib;

pub struct Device {
    pub id: String,
    pub name: String,
    pub icon: gio::Icon,
    pub volume: gio::Volume,
    pub mounted: bool,
}

pub fn monitor() -> gio::VolumeMonitor {
    gio::VolumeMonitor::get()
}

pub fn list(monitor: &gio::VolumeMonitor) -> Vec<Device> {
    let mut out = Vec::new();
    for volume in monitor.volumes() {
        if !is_removable(&volume) {
            continue;
        }
        let name = volume.name().to_string();
        let id = volume
            .identifier("uuid")
            .or_else(|| volume.identifier("unix-device"))
            .map(|s| s.to_string())
            .unwrap_or_else(|| name.clone());
        let mounted = volume.get_mount().is_some();
        out.push(Device {
            id,
            name,
            icon: volume.icon(),
            volume,
            mounted,
        });
    }
    out
}

fn is_removable(volume: &gio::Volume) -> bool {
    if volume.can_eject() {
        return true;
    }
    if let Some(drive) = volume.drive() {
        return drive.is_removable() || drive.can_eject();
    }
    volume
        .get_mount()
        .map(|mount| mount.can_unmount() || mount.can_eject())
        .unwrap_or(false)
}

pub fn ejectable(volume: &gio::Volume) -> bool {
    volume.can_eject()
        || volume
            .get_mount()
            .map(|mount| mount.can_eject() || mount.can_unmount())
            .unwrap_or(false)
}

pub fn eject_label(volume: &gio::Volume) -> &'static str {
    let can_eject = volume.can_eject()
        || volume.get_mount().map(|mount| mount.can_eject()).unwrap_or(false);
    if can_eject {
        "Eject"
    } else {
        "Unmount"
    }
}

pub fn activate(volume: &gio::Volume) {
    if let Some(mount) = volume.get_mount() {
        open_mount(&mount);
        return;
    }
    let vol = volume.clone();
    let op = gio::MountOperation::new();
    volume.mount(
        gio::MountMountFlags::NONE,
        Some(&op),
        gio::Cancellable::NONE,
        move |result| match result {
            Ok(()) => {
                if let Some(mount) = vol.get_mount() {
                    open_mount(&mount);
                }
            }
            Err(err) => eprintln!("jdock: failed to mount {}: {err}", vol.name()),
        },
    );
}

pub fn eject(volume: &gio::Volume) {
    let name = volume.name().to_string();
    let op = gio::MountOperation::new();

    if volume.can_eject() {
        volume.eject_with_operation(
            gio::MountUnmountFlags::NONE,
            Some(&op),
            gio::Cancellable::NONE,
            move |result| log_result("eject", &name, result),
        );
        return;
    }

    if let Some(mount) = volume.get_mount() {
        if mount.can_eject() {
            mount.eject_with_operation(
                gio::MountUnmountFlags::NONE,
                Some(&op),
                gio::Cancellable::NONE,
                move |result| log_result("eject", &name, result),
            );
        } else if mount.can_unmount() {
            mount.unmount_with_operation(
                gio::MountUnmountFlags::NONE,
                Some(&op),
                gio::Cancellable::NONE,
                move |result| log_result("unmount", &name, result),
            );
        }
    }
}

fn open_mount(mount: &gio::Mount) {
    let uri = mount.root().uri();
    if let Err(err) = open_uri(uri.as_str()) {
        eprintln!("jdock: failed to open {uri}: {err}");
    }
}

fn open_uri(uri: &str) -> std::io::Result<()> {
    Command::new("xdg-open")
        .arg(uri)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

fn log_result(action: &str, name: &str, result: Result<(), glib::Error>) {
    if let Err(err) = result {
        eprintln!("jdock: failed to {action} {name}: {err}");
    }
}
