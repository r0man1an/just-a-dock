mod app;
mod config;
mod configapp;
mod desktop;
mod devices;
mod geometry;
mod icon;
mod layer;
mod model;
mod style;
mod theme;
mod toplevel;
mod trash;

use gtk4::glib;
use gtk4::prelude::*;

fn main() -> glib::ExitCode {
    unsafe {
        libc::signal(libc::SIGCHLD, libc::SIG_IGN);
    }

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
    application.connect_activate(app::build_ui);
    application.run()
}
