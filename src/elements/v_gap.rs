use crate::*;

/// A vertical gap element that creates empty vertical space.
/// 
/// This element takes up the specified height (or available height if smaller)
/// without rendering any content. Useful for adding spacing between elements.
pub struct VGap(pub f32);

impl Element for VGap {
    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        size(self, ctx.first_height)
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        size(self, ctx.first_height)
    }
}

fn size(v_gap: &VGap, first_height: f32) -> ElementSize {
    ElementSize {
        width: None,
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
            output.assert_size(ElementSize {
                width: None,
                height: Some(if output.first_height == 11. {
                    11.
                } else {
                    28.3
                }),
            });

            if let Some(b) = output.breakable {
                b.assert_break_count(0)
                    .assert_extra_location_min_height(None);
            }
        }
    }
}
