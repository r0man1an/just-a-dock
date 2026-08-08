use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use crate::config::{Config, DockEdge, HideMode, StyleVariant};

pub fn init(window: &gtk4::ApplicationWindow, config: &Config) {
    window.init_layer_shell();
    window.set_layer(Layer::Top);
    window.set_namespace(Some("jdock"));
    window.set_keyboard_mode(KeyboardMode::None);

    let edge = to_layer_edge(config.edge);
    window.set_anchor(edge, true);

    let margin = match config.hide_mode {
        HideMode::Maximized | HideMode::Timed => 0,
        HideMode::Disabled => match config.style {
            StyleVariant::Round => config.edge_margin as i32,
            StyleVariant::Straight => 0,
        }
    };
    window.set_margin(edge, margin);

    match config.style {
        StyleVariant::Straight => window.auto_exclusive_zone_enable(),
        StyleVariant::Round => window.set_exclusive_zone(0),
    }
}

pub fn to_layer_edge(edge: DockEdge) -> Edge {
    match edge {
        DockEdge::Bottom => Edge::Bottom,
        DockEdge::Left => Edge::Left,
        DockEdge::Right => Edge::Right,
    }
}

pub fn tooltip_primary_edge(edge: DockEdge) -> Edge {
    match edge {
        DockEdge::Bottom => Edge::Left,
        DockEdge::Left | DockEdge::Right => Edge::Top,
    }
}
