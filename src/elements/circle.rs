use printpdf::{utils::calculate_points_for_circle, Line};

use crate::{utils::*, *};

pub struct Circle {
    pub radius: f64,
    pub fill: Option<u32>,
    pub outline: Option<(f64, u32)>,
}

impl Element for Circle {
    fn measure(&self, mut ctx: MeasureCtx) -> Option<ElementSize> {
        let outline_thickness = outline_thickness(self);
        ctx.break_if_appropriate_for_min_height(self.radius * 2. + outline_thickness);

        Some(size(self))
    }

    fn draw(&self, mut ctx: DrawCtx) -> Option<ElementSize> {
        let outline_thickness = outline_thickness(self);
        ctx.break_if_appropriate_for_min_height(self.radius * 2. + outline_thickness);

        let extra_outline_offset = outline_thickness / 2.0;

        let points = calculate_points_for_circle(
            Mm(self.radius),
            Mm(ctx.location.pos.0 + self.radius + extra_outline_offset),
            Mm(ctx.location.pos.1 - self.radius - extra_outline_offset),
        );

        ctx.location.layer.save_graphics_state();

        if let Some(color) = self.fill {
            let (color, alpha) = u32_to_color_and_alpha(color);
            ctx.location.layer.set_fill_color(color);
            ctx.location.layer.set_fill_alpha(alpha);
        }

        if let Some((thickness, color)) = self.outline {
            // No outline alpha?
            let (color, _alpha) = u32_to_color_and_alpha(color);
            ctx.location.layer.set_outline_color(color);
            ctx.location
                .layer
                .set_outline_thickness(mm_to_pt(thickness));
        }

        ctx.location.layer.add_shape(Line {
            points,
            is_closed: true,
            has_fill: self.fill.is_some(),
            has_stroke: self.outline.is_some(),
            is_clipping_path: false,
        });

        ctx.location.layer.restore_graphics_state();

        Some(size(self))
    }
}

fn outline_thickness(circle: &Circle) -> f64 {
    circle.outline.map(|o| o.0).unwrap_or(0.0)
}

fn size(circle: &Circle) -> ElementSize {
    let outline_thickness = outline_thickness(circle);

    let size = circle.radius * 2. + outline_thickness;

    ElementSize {
        width: size,
        height: Some(size),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    #[test]
    fn test_circle() {
        for output in (ElementTestParams {
            first_height: 11.,
            ..Default::default()
        })
        .run(&Circle {
            radius: 5.5,
            fill: None,
            outline: Some((1., 0)),
        }) {
            output.assert_size(Some(ElementSize {
                width: 12.,
                height: Some(12.),
            }));

            if let Some(b) = output.breakable {
                if output.first_height == 11. {
                    b.assert_break_count(1);
                } else {
                    b.assert_break_count(0);
                }

                b.assert_extra_location_min_height(0.);
            }
        }
    }
}
