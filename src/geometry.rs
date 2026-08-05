use crate::config::{Config, DockEdge};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
    pub icon_size: f64,
    pub slot_size: f64,
    pub icon_render_size: f64,
    pub icon_padding: f64,
    pub thickness: f64,
    pub corner_radius: f64,
    pub border_width: f64,
    pub edge_margin: f64,
    pub idle_dot_diameter: f64,
    pub active_pill_length: f64,
}

impl Geometry {
    pub fn compute(config: &Config, item_count: usize, available_length: i32, scale_factor: i32) -> Self {
        let icon_size = config.icon_size as f64;
        let mut slot_size = icon_size * 1.1;

        let usable = (available_length as f64 - config.edge_margin as f64).max(1.0);
        let needed = item_count as f64 * slot_size;
        if item_count > 0 && needed >= usable {
            slot_size = usable / item_count as f64;
        }

        let icon_render_size = slot_size * 0.8;
        let icon_padding = slot_size * 0.1;
        let thickness = slot_size;
        let scale = scale_factor.max(1) as f64;

        Geometry {
            icon_size,
            slot_size,
            icon_render_size,
            icon_padding,
            thickness,
            corner_radius: thickness * 0.3,
            border_width: 1.0 / scale,
            edge_margin: config.edge_margin as f64,
            idle_dot_diameter: thickness * 0.06,
            active_pill_length: thickness * 0.5,
        }
    }

    pub fn indicator_is_horizontal(edge: DockEdge) -> bool {
        matches!(edge, DockEdge::Bottom)
    }

    pub fn indicator_outward_margin(&self, edge: DockEdge) -> f64 {
        match edge {
            DockEdge::Bottom | DockEdge::Right => {
                (self.icon_padding + 1.0 - self.idle_dot_diameter).max(0.0)
            }
            DockEdge::Left => 2.0,
        }
    }
}
