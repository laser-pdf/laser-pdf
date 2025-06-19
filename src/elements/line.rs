use crate::{utils::*, *};

/// A horizontal line element with configurable styling.
/// 
/// The line spans the full available width when drawn in an expanding context.
/// Line thickness, color, dash patterns, and cap styles are configurable.
pub struct Line {
    /// Line styling including thickness, color, dash pattern, and cap style
    pub style: LineStyle,
}

impl Line {
    pub fn new(thickness: f32) -> Self {
        Line {
            style: LineStyle {
                thickness,
                color: 0x00_00_00_FF,
                dash_pattern: None,
                cap_style: LineCapStyle::Butt,
            },
        }
    }
}

impl Element for Line {
    fn measure(&self, mut ctx: MeasureCtx) -> ElementSize {
        ctx.break_if_appropriate_for_min_height(self.style.thickness);

        size(self, ctx.width)
    }

    fn draw(&self, mut ctx: DrawCtx) -> ElementSize {
        ctx.break_if_appropriate_for_min_height(self.style.thickness);

        if ctx.width.expand {
            let (color, _alpha) = u32_to_color_and_alpha(self.style.color);
            let style = self.style;

            let layer = ctx.location.layer(ctx.pdf);

            layer
                .save_state()
                .set_line_width(mm_to_pt(style.thickness))
                .set_stroke_rgb(color[0], color[1], color[2])
                .set_line_cap(style.cap_style.into());

            if let Some(pattern) = style.dash_pattern {
                layer.set_dash_pattern(pattern.dashes.map(f32::from), pattern.offset as f32);
            }

            let line_y = ctx.location.pos.1 - self.style.thickness / 2.0;

            layer
                .move_to(mm_to_pt(ctx.location.pos.0), mm_to_pt(line_y))
                .line_to(
                    mm_to_pt(ctx.location.pos.0 + ctx.width.max),
                    mm_to_pt(line_y),
                )
                .stroke()
                .restore_state();
        }

        size(self, ctx.width)
    }
}

fn size(line: &Line, width: WidthConstraint) -> ElementSize {
    ElementSize {
        width: Some(width.constrain(0.)),
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
            output.assert_size(ElementSize {
                width: Some(output.width.constrain(0.)),
                height: Some(1.),
            });

            if let Some(b) = output.breakable {
                if output.first_height == 0.2 {
                    b.assert_break_count(1);
                } else {
                    b.assert_break_count(0);
                }

                b.assert_extra_location_min_height(None);
            }
        }
    }
}
