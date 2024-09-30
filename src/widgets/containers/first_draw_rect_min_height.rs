use crate::*;

/**
 * This preemptively triggers a page break if the available height is below the threshold.
 **/
pub struct FirstDrawRectMinHeight {
    threshold: f64,
    element: impl Element,
}

impl Element for FirstDrawRectMinHeight<T> {
    fn measure(&self, ctx: MeasureCtx) -> Option<ElementSize> {
        element.draw(width, None)
    }

    fn draw(&self, ctx: DrawCtx) -> Option<ElementSize> {
        match ctx.next_location {
            Some(next_location) if ctx.location.height_available < threshold => {
                let new_location = next_location(ctx.pdf, 0, [0.; 2]);

                element.draw(
                    width,
                    Some(DrawCtx {
                        pdf: ctx.pdf,
                        location: new_location,
                        full_height: ctx.full_height,
                        breakable: Some(BreakableDraw {
                            get_location: &mut |pdf, draw_rect, size| {
                                next_location(pdf, draw_rect + 1, size)
                            },
                            ..break_ctx
                        }),
                    }),
                )
            }
            _ => element.draw(width, Some(ctx)),
        }
    }
}
