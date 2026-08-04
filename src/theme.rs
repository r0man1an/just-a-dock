use gio::glib;
use gio::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme {
    Light,
    Dark,
}

const PORTAL_BUS_NAME: &str = "org.freedesktop.portal.Desktop";
const PORTAL_OBJECT_PATH: &str = "/org/freedesktop/portal/desktop";
const PORTAL_SETTINGS_IFACE: &str = "org.freedesktop.portal.Settings";
const APPEARANCE_NAMESPACE: &str = "org.freedesktop.appearance";
const COLOR_SCHEME_KEY: &str = "color-scheme";

fn code_to_scheme(code: u32) -> ColorScheme {
    if code == 1 {
        ColorScheme::Dark
    } else {
        ColorScheme::Light
    }
}

fn unwrap_scheme_variant(value: &glib::Variant) -> Option<ColorScheme> {
    let inner = value.as_variant().unwrap_or_else(|| value.clone());
    inner.get::<u32>().map(code_to_scheme)
}

pub fn init() -> (ColorScheme, Option<gio::DBusProxy>) {
    let proxy = match gio::DBusProxy::for_bus_sync(
        gio::BusType::Session,
        gio::DBusProxyFlags::NONE,
        None::<&gio::DBusInterfaceInfo>,
        PORTAL_BUS_NAME,
        PORTAL_OBJECT_PATH,
        PORTAL_SETTINGS_IFACE,
        gio::Cancellable::NONE,
    ) {
        Ok(proxy) => proxy,
        Err(err) => {
            eprintln!("jdock: no settings portal ({err}); defaulting to light theme");
            return (ColorScheme::Light, None);
        }
    };

    let initial = proxy
        .call_sync(
            "Read",
            Some(&glib::Variant::tuple_from_iter([
                APPEARANCE_NAMESPACE.to_variant(),
                COLOR_SCHEME_KEY.to_variant(),
            ])),
            gio::DBusCallFlags::NONE,
            1000,
            gio::Cancellable::NONE,
        )
        .ok()
        .and_then(|reply| unwrap_scheme_variant(&reply.child_value(0)))
        .unwrap_or(ColorScheme::Light);

    (initial, Some(proxy))
}

struct MainThreadOnly<F>(F);
unsafe impl<F> Send for MainThreadOnly<F> {}
unsafe impl<F> Sync for MainThreadOnly<F> {}

impl<F: Fn(ColorScheme)> MainThreadOnly<F> {
    fn call(&self, scheme: ColorScheme) {
        (self.0)(scheme)
    }
}

pub fn subscribe<F>(proxy: &gio::DBusProxy, on_change: F)
where
    F: Fn(ColorScheme) + 'static,
{
    let on_change = MainThreadOnly(on_change);
    proxy.connect_g_signal(move |_proxy, _sender, signal, params| {
        if signal != "SettingChanged" || params.n_children() < 3 {
            return;
        }
        let namespace = params.child_value(0).str().unwrap_or_default().to_string();
        let key = params.child_value(1).str().unwrap_or_default().to_string();
        if namespace != APPEARANCE_NAMESPACE || key != COLOR_SCHEME_KEY {
            return;
        }
        if let Some(scheme) = unwrap_scheme_variant(&params.child_value(2)) {
            on_change.call(scheme);
        }
    });
}
