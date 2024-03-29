use printpdf::utils::{calculate_points_for_circle, calculate_points_for_rect};
use printpdf::*;

use crate::utils::*;
use crate::*;

pub struct Rectangle {
    pub size: [f64; 2],
    pub fill: Option<u32>,
    pub outline: Option<(f64, u32)>,
}

impl Element for Rectangle {
    fn element(&self, _width: Option<f64>, draw: Option<DrawContext>) -> [f64; 2] {
        let outline_thickness = self.outline.map(|o| o.0).unwrap_or(0.0);

        if let Some(context) = draw {
            let extra_outline_offset = outline_thickness / 2.0;

            let points = calculate_points_for_rect(
                Mm(self.size[0]),
                Mm(self.size[1]),
                Mm(context.draw_pos.pos[0] + self.size[0] / 2.0 + extra_outline_offset),
                Mm(context.draw_pos.pos[1] - self.size[1] / 2.0 - extra_outline_offset),
            );

            context.draw_pos.layer.save_graphics_state();

            if let Some(color) = self.fill {
                let (color, alpha) = u32_to_color_and_alpha(color);
                context.draw_pos.layer.set_fill_color(color);
                context.draw_pos.layer.set_fill_alpha(alpha);
            }

            if let Some((thickness, color)) = self.outline {
                // No outline alpha?
                let (color, _alpha) = u32_to_color_and_alpha(color);
                context.draw_pos.layer.set_outline_color(color);
                context
                    .draw_pos
                    .layer
                    .set_outline_thickness(mm_to_pt(thickness));
            }

            context.draw_pos.layer.add_shape(Line {
                points,
                is_closed: true,
                has_fill: self.fill.is_some(),
                has_stroke: self.outline.is_some(),
                is_clipping_path: false,
            });

            context.draw_pos.layer.restore_graphics_state();
        }

        [
            self.size[0] + outline_thickness,
            self.size[1] + outline_thickness,
        ]
    }
}

pub struct Circle {
    pub radius: f64,
    pub fill: Option<u32>,
    pub outline: Option<(f64, u32)>,
}

impl Element for Circle {
    fn element(&self, _width: Option<f64>, draw: Option<DrawContext>) -> [f64; 2] {
        let outline_thickness = self.outline.map(|o| o.0).unwrap_or(0.0);

        if let Some(context) = draw {
            let extra_outline_offset = outline_thickness / 2.0;

            let points = calculate_points_for_circle(
                Mm(self.radius),
                Mm(context.draw_pos.pos[0] + self.radius + extra_outline_offset),
                Mm(context.draw_pos.pos[1] - self.radius - extra_outline_offset),
            );

            context.draw_pos.layer.save_graphics_state();

            if let Some(color) = self.fill {
                let (color, alpha) = u32_to_color_and_alpha(color);
                context.draw_pos.layer.set_fill_color(color);
                context.draw_pos.layer.set_fill_alpha(alpha);
            }

            if let Some((thickness, color)) = self.outline {
                // No outline alpha?
                let (color, _alpha) = u32_to_color_and_alpha(color);
                context.draw_pos.layer.set_outline_color(color);
                context
                    .draw_pos
                    .layer
                    .set_outline_thickness(mm_to_pt(thickness));
            }

            context.draw_pos.layer.add_shape(Line {
                points,
                is_closed: true,
                has_fill: self.fill.is_some(),
                has_stroke: self.outline.is_some(),
                is_clipping_path: false,
            });

            context.draw_pos.layer.restore_graphics_state();
        }

        [self.radius * 2.0 + outline_thickness; 2]
    }
}
