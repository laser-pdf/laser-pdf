use printpdf::utils::calculate_points_for_rect;

use crate::{utils::*, *};

pub struct Rectangle((f64, f64));

impl Element for Rectangle {
    fn measure(&self, mut ctx: MeasureCtx) -> Option<ElementSize> {
        ctx.break_if_appropriate_for_min_height(self.0 .1);

        Some(size(self))
    }

    fn draw(&self, mut ctx: DrawCtx) -> Option<ElementSize> {
        ctx.break_if_appropriate_for_min_height(self.0 .1);

        let points = calculate_points_for_rect(
            Pt(mm_to_pt(self.0 .0)),
            Pt(mm_to_pt(self.0 .1)),
            Pt(mm_to_pt(ctx.location.pos.0 + self.0 .0 / 2.0)),
            Pt(mm_to_pt(ctx.location.pos.1 - self.0 .1 / 2.0)),
        );

        ctx.location.layer.add_shape(printpdf::Line {
            points,
            is_closed: true,
            has_fill: true,
            has_stroke: false,
            is_clipping_path: false,
        });

        Some(size(self))
    }
}

fn size(rectangle: &Rectangle) -> ElementSize {
    ElementSize {
        width: rectangle.0 .0,
        height: Some(rectangle.0 .1),
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
        .run(&Rectangle((12., 13.)))
        {
            output.assert_size(Some(ElementSize {
                width: 12.,
                height: Some(13.),
            }));

            if let Some(b) = output.breakable {
                if output.first_height == 12. {
                    b.assert_break_count(1);
                } else {
                    b.assert_break_count(0);
                }

                b.assert_extra_location_min_height(0.);
            }
        }
    }
}
