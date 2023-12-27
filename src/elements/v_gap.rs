use crate::*;

pub struct VGap(pub f64);

impl Element for VGap {
    fn measure(&self, ctx: MeasureCtx) -> Option<ElementSize> {
        Some(size(self, ctx.width, ctx.first_height))
    }

    fn draw(&self, ctx: DrawCtx) -> Option<ElementSize> {
        Some(size(self, ctx.width, ctx.first_height))
    }
}

fn size(v_gap: &VGap, width: Option<f64>, first_height: f64) -> ElementSize {
    ElementSize {
        width: width.unwrap_or(0.0),
        height: Some(v_gap.0.min(first_height)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    #[test]
    fn test_line() {
        for output in (ElementTestParams {
            first_height: 11.,
            ..Default::default()
        })
        .run(&VGap(28.3))
        {
            output.assert_size(Some(ElementSize {
                width: output.width.unwrap_or(0.),
                height: Some(if output.first_height == 11. {
                    11.
                } else {
                    28.3
                }),
            }));

            if let Some(b) = output.breakable {
                b.assert_break_count(0).assert_extra_location_min_height(0.);
            }
        }
    }
}
