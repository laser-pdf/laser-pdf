use crate::{utils::*, *};

pub struct Line {
    style: LineStyle,
}

impl Element for Line {
    fn measure(&self, mut ctx: MeasureCtx) -> Option<ElementSize> {
        ctx.break_if_appropriate_for_min_height(self.style.thickness);

        Some(size(self, ctx.width))
    }

    fn draw(&self, mut ctx: DrawCtx) -> Option<ElementSize> {
        ctx.break_if_appropriate_for_min_height(self.style.thickness);

        if let Some(width) = ctx.width {
            ctx.location.layer.save_graphics_state();

            let (color, _alpha) = u32_to_color_and_alpha(self.style.color);
            ctx.location.layer.set_outline_color(color);
            ctx.location
                .layer
                .set_outline_thickness(mm_to_pt(self.style.thickness));
            ctx.location
                .layer
                .set_line_cap_style(self.style.cap_style.into());
            ctx.location.layer.set_line_dash_pattern(
                if let Some(pattern) = self.style.dash_pattern {
                    pattern.into()
                } else {
                    printpdf::LineDashPattern::default()
                },
            );

            let line_y = ctx.location.pos.1 - self.style.thickness / 2.0;

            ctx.location.layer.add_shape(printpdf::Line {
                points: vec![
                    (Point::new(Mm(ctx.location.pos.0), Mm(line_y)), false),
                    (
                        Point::new(Mm(ctx.location.pos.0 + width), Mm(line_y)),
                        false,
                    ),
                ],
                is_closed: false,
                has_fill: false,
                has_stroke: true,
                is_clipping_path: false,
            });

            ctx.location.layer.restore_graphics_state();
        }

        Some(size(self, ctx.width))
    }
}

fn size(line: &Line, width: Option<f64>) -> ElementSize {
    ElementSize {
        width: width.unwrap_or(0.),
        height: Some(line.style.thickness),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    #[test]
    fn test_line() {
        for output in (ElementTestParams {
            first_height: 0.2,
            ..Default::default()
        })
        .run(&Line {
            style: LineStyle {
                thickness: 1.,
                color: 0,
                dash_pattern: None,
                cap_style: LineCapStyle::Butt,
            },
        }) {
            output.assert_size(Some(ElementSize {
                width: output.width.unwrap_or(0.),
                height: Some(1.),
            }));

            if let Some(b) = output.breakable {
                if output.first_height == 0.2 {
                    b.assert_break_count(1);
                } else {
                    b.assert_break_count(0);
                }

                b.assert_extra_location_min_height(0.);
            }
        }
    }
}
