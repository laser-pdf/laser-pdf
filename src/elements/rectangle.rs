use printpdf::{utils::calculate_points_for_rect, Line};

use crate::{utils::*, *};

pub struct Rectangle {
    pub size: (f64, f64),
    pub fill: Option<u32>,
    pub outline: Option<(f64, u32)>,
}

impl Element for Rectangle {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        let outline_thickness = outline_thickness(self);
        if ctx.break_appropriate_for_min_height(self.size.1 + outline_thickness) {
            FirstLocationUsage::WillSkip
        } else {
            FirstLocationUsage::WillUse
        }
    }

    fn measure(&self, mut ctx: MeasureCtx) -> ElementSize {
        let outline_thickness = outline_thickness(self);
        ctx.break_if_appropriate_for_min_height(self.size.1 + outline_thickness);

        size(self)
    }

    fn draw(&self, mut ctx: DrawCtx) -> ElementSize {
        let outline_thickness = outline_thickness(self);
        ctx.break_if_appropriate_for_min_height(self.size.1 + outline_thickness);

        let extra_outline_offset = outline_thickness / 2.0;

        let points = calculate_points_for_rect(
            Mm(self.size.0),
            Mm(self.size.1),
            Mm(ctx.location.pos.0 + self.size.0 / 2.0 + extra_outline_offset),
            Mm(ctx.location.pos.1 - self.size.1 / 2.0 - extra_outline_offset),
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

        size(self)
    }
}

fn outline_thickness(rectangle: &Rectangle) -> f64 {
    rectangle.outline.map(|o| o.0).unwrap_or(0.0)
}

fn size(rectangle: &Rectangle) -> ElementSize {
    let outline_thickness = outline_thickness(rectangle);

    ElementSize {
        width: Some(rectangle.size.0 + outline_thickness),
        height: Some(rectangle.size.1 + outline_thickness),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    #[test]
    fn test_rectangle() {
        for output in (ElementTestParams {
            first_height: 12.,
            ..Default::default()
        })
        .run(&Rectangle {
            size: (11., 12.),
            fill: None,
            outline: Some((1., 0)),
        }) {
            output.assert_size(ElementSize {
                width: Some(12.),
                height: Some(13.),
            });

            if let Some(b) = output.breakable {
                if output.first_height == 12. {
                    b.assert_break_count(1);
                } else {
                    b.assert_break_count(0);
                }

                b.assert_extra_location_min_height(None);
            }
        }
    }
}
