mod app;
mod config;
mod configapp;
mod desktop;
mod geometry;
mod icon;
mod layer;
mod model;
mod style;
mod theme;
mod toplevel;

use gtk4::glib;
use gtk4::prelude::*;

use std::os::unix::process::CommandExt;

fn main() -> glib::ExitCode {
    if std::env::var_os("GDK_BACKEND").is_none() {
        unsafe { std::env::set_var("GDK_BACKEND", "wayland") };
    }
    if std::env::var_os("GSK_RENDERER").is_none() {
        unsafe { std::env::set_var("GSK_RENDERER", "cairo") };
    }

    if std::env::args().any(|arg| arg == "--config") {
        let application = gtk4::Application::new(
            Some("com.justadock.Dock.Config"),
            gtk4::gio::ApplicationFlags::empty(),
        );
        application.connect_activate(configapp::build_ui);
        return application.run_with_args::<&str>(&[]);
    }

    let application = gtk4::Application::new(
        Some("com.just-a-dock.Dock"),
        gtk4::gio::ApplicationFlags::empty(),
    );

    glib_unix::unix_signal_add_local(libc::SIGUSR1, || {
        if let Err(err) = restart() {
            eprintln!("Failed to restart jdock: {err}");
        }
        gtk4::glib::ControlFlow::Continue
    });

    application.connect_activate(app::build_ui);
    application.run()
}

fn restart() -> std::io::Result<()> {
    let exe = std::env::current_exe()?;
    let err = std::process::Command::new(exe)
        .args(std::env::args_os().skip(1))
        .exec();
    Err(err)
}
