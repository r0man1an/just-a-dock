use crate::config::{Config, DockEdge, HideMode, StyleVariant};
use crate::geometry::Geometry;
use crate::theme::ColorScheme;

pub fn generate_css(geometry: &Geometry, config: &Config, scheme: ColorScheme) -> String {
    let (bg, border) = match scheme {
        ColorScheme::Light => ("#E6E6E6", "rgba(0, 0, 0, 0.2)"),
        ColorScheme::Dark => ("#666666", "rgba(255, 255, 255, 0.3)"),
    };
    let dot_color = match scheme {
        ColorScheme::Light => "#1a1a1a",
        ColorScheme::Dark => "#f5f5f5",
    };

    let radius = match config.style {
        StyleVariant::Round => geometry.corner_radius,
        StyleVariant::Straight => 0.0,
    };

    let opacity = config.opacity;
    let border_width = geometry.border_width;
    let slot = geometry.slot_size;
    let icon_render = geometry.icon_render_size;
    let icon_padding = geometry.icon_padding;
    let idle = geometry.idle_dot_diameter;
    let pill = geometry.active_pill_length;
    let outward_margin = geometry.indicator_outward_margin(config.edge);

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

    // .dock-background.collapsed uses 0.05, not 0, opacity because some wlroots compositors skip hit-testing on fully transparent surffaces
    format!(
        "\
window, .background {{
  background-color: transparent;
}}

.dock-background {{
  background-color: alpha({bg}, {opacity});
  border: {border_width:.3}px solid {border};
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
}}

.dock-icon-button image {{
  min-width: {icon_render:.2}px;
  min-height: {icon_render:.2}px;
}}

.dock-icon-button:active {{
  background-color: alpha(#000000, 0.4);
  border-radius: {radius:.2}px;
}}

{indicator_rule}

.dock-indicator {{
  background-color: {dot_color};
  border-radius: 9999px;
  transition: min-width 250ms ease-in-out, min-height 250ms ease-in-out;
}}

.dock-tooltip {{
  background-color: alpha(#000000, 0.75);
  color: #ffffff;
  font-size: 15px;
  font-weight: bold;
  border-radius: 9999px;
  padding: 7px 20px;
}}

.dock-menu-list {{
  background-color: alpha(#000000, 0.92);
  border-radius: 16px;
  padding: 6px;
}}

.dock-menu-item {{
  background: none;
  color: #ffffff;
  font-size: 18px;
  font-weight: bold;
  border-radius: 10px;
  padding: 9px 26px;
  border: none;
  box-shadow: none;
}}

.dock-menu-item:hover {{
  background-color: alpha(#ffffff, 0.15);
}}

.dock-menu-item:active {{
  background-color: alpha(#ffffff, 0.28);
}}

.dock-menu-separator {{
  min-height: 1px;
  background-color: alpha(#ffffff, 0.2);
  margin: 4px 10px;
}}
"
    )
}
