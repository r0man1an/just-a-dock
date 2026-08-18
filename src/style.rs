use crate::config::{Config, DockEdge, HideMode, StyleVariant};
use crate::geometry::Geometry;
use crate::theme::ColorScheme;

struct Palette {
    fill_hex: &'static str,
    rim: &'static str,
    dot: &'static str,
    hover_highlight: &'static str,
    press_highlight: &'static str,
    tooltip_bg: &'static str,
    tooltip_fg: &'static str,
    tooltip_border: &'static str,
    menu_bg: &'static str,
    menu_fg: &'static str,
    menu_border: &'static str,
    menu_separator: &'static str,
    menu_accent: &'static str,
    menu_accent_active: &'static str,
}

pub fn generate_css(geometry: &Geometry, config: &Config, scheme: ColorScheme) -> String {
    let p = match scheme {

        ColorScheme::Light => Palette {
            fill_hex: "#ececec",
            rim: "alpha(#000000, 0.12)",
            dot: "alpha(#000000, 0.60)",
            hover_highlight: "alpha(#000000, 0.06)",
            press_highlight: "alpha(#000000, 0.12)",
            tooltip_bg: "alpha(#ffffff, 0.95)",
            tooltip_fg: "#1a1a1a",
            tooltip_border: "alpha(#000000, 0.10)",
            menu_bg: "alpha(#f5f5f5, 0.96)",
            menu_fg: "#1a1a1a",
            menu_border: "alpha(#000000, 0.10)",
            menu_separator: "alpha(#000000, 0.12)",
            menu_accent: "#007aff",
            menu_accent_active: "alpha(#007aff, 0.85)",
        },
        ColorScheme::Dark => Palette {
            fill_hex: "#191919",
            rim: "alpha(#a0a0a0, 0.40)",
            dot: "alpha(#f0f0f0, 0.90)",
            hover_highlight: "alpha(#ffffff, 0.09)",
            press_highlight: "alpha(#ffffff, 0.16)",
            tooltip_bg: "alpha(#2a2a2a, 0.94)",
            tooltip_fg: "#ffffff",
            tooltip_border: "alpha(#ffffff, 0.12)",
            menu_bg: "alpha(#1e1e1e, 0.94)",
            menu_fg: "#ffffff",
            menu_border: "alpha(#ffffff, 0.10)",
            menu_separator: "alpha(#ffffff, 0.12)",
            menu_accent: "#0a84ff",
            menu_accent_active: "alpha(#0a84ff, 0.85)",
        },
    };

    let radius = match config.style {
        StyleVariant::Round => geometry.corner_radius.max(18.0),
        StyleVariant::Straight => 0.0,
    };

    let opacity = config.opacity;
    let rim_width = geometry.border_width.max(1.0);
    let slot = geometry.slot_size;
    let icon_render = geometry.icon_render_size;
    let icon_padding = geometry.icon_padding;
    let idle = geometry.idle_dot_diameter;
    let pill = geometry.active_pill_length;
    let outward_margin = geometry.indicator_outward_margin(config.edge);
    let fill = format!("alpha({}, {:.3})", p.fill_hex, opacity.clamp(0.0, 1.0));

    let rim = p.rim;
    let dot = p.dot;
    let hover_highlight = p.hover_highlight;
    let press_highlight = p.press_highlight;
    let tooltip_bg = p.tooltip_bg;
    let tooltip_fg = p.tooltip_fg;
    let tooltip_border = p.tooltip_border;
    let menu_bg = p.menu_bg;
    let menu_fg = p.menu_fg;
    let menu_border = p.menu_border;
    let menu_separator = p.menu_separator;
    let menu_accent = p.menu_accent;
    let menu_accent_active = p.menu_accent_active;

    let dock_gap = match config.hide_mode {
        HideMode::Timed | HideMode::Maximized => match config.style {
            StyleVariant::Round => config.edge_margin as f64,
            StyleVariant::Straight => 0.0,
        },
        HideMode::Disabled => 0.0,
    };
    let dock_gap_prop = match config.edge {
        DockEdge::Bottom => "margin-bottom",
        DockEdge::Left => "margin-start",
        DockEdge::Right => "margin-end",
    };

    let indicator_rule = if Geometry::indicator_is_horizontal(config.edge) {
        format!(
            "\
.dock-indicator {{
  min-width: {idle:.2}px;
  min-height: {idle:.2}px;
  margin-bottom: {outward_margin:.2}px;
}}
.dock-indicator.active {{
  min-width: {pill:.2}px;
}}"
        )
    } else {
        let margin_prop = match config.edge {
            DockEdge::Left => "margin-start",
            _ => "margin-end",
        };
        format!(
            "\
.dock-indicator {{
  min-width: {idle:.2}px;
  min-height: {idle:.2}px;
  {margin_prop}: {outward_margin:.2}px;
}}
.dock-indicator.active {{
  min-height: {pill:.2}px;
}}"
        )
    };

    format!(
        "\
window, .background {{
  background-color: transparent;
}}

.dock-background {{
  background-color: {fill};
  border: {rim_width:.3}px solid {rim};
  border-radius: {radius:.2}px;
  {dock_gap_prop}: {dock_gap:.2}px;
  transition: background-color 200ms linear, border-color 200ms linear, opacity 200ms linear;
}}

.dock-background.collapsed {{
  opacity: 0.05;
}}

.dock-cell {{
  min-width: {slot:.2}px;
  min-height: {slot:.2}px;
}}

.dock-icon-button {{
  background: none;
  border: none;
  box-shadow: none;
  padding: {icon_padding:.2}px;
  min-width: {slot:.2}px;
  min-height: {slot:.2}px;
  border-radius: {radius:.2}px;
  transition: background-color 150ms ease-in-out;
}}

.dock-icon-button image {{
  min-width: {icon_render:.2}px;
  min-height: {icon_render:.2}px;
}}

.dock-icon-button:hover {{
  background-color: {hover_highlight};
  border-radius: {radius:.2}px;
}}

.dock-icon-button:active {{
  background-color: {press_highlight};
  border-radius: {radius:.2}px;
}}

{indicator_rule}

.dock-indicator {{
  background-color: {dot};
  border-radius: 9999px;
  transition: min-width 250ms ease-in-out, min-height 250ms ease-in-out;
}}

.dock-tooltip {{
  background-color: {tooltip_bg};
  color: {tooltip_fg};
  font-size: 14px;
  font-weight: normal;
  border: 1px solid {tooltip_border};
  border-radius: 8px;
  padding: 6px 14px;
}}

.dock-menu-list {{
  background-color: {menu_bg};
  border: 1px solid {menu_border};
  border-radius: 12px;
  padding: 6px;
}}

.dock-menu-item {{
  background: none;
  color: {menu_fg};
  font-size: 15px;
  font-weight: normal;
  border-radius: 8px;
  padding: 8px 20px;
  border: none;
  box-shadow: none;
}}

.dock-menu-item:hover {{
  background-color: {menu_accent};
  color: #ffffff;
}}

.dock-menu-item:active {{
  background-color: {menu_accent_active};
  color: #ffffff;
}}

.dock-menu-separator {{
  min-height: 1px;
  background-color: {menu_separator};
  margin: 4px 10px;
}}
"
    )
}
