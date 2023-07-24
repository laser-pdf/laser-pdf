use crate::*;

/**
 * This preemptively triggers a page break if the available height is below the threshold.
 **/
pub fn first_draw_rect_min_height(threshold: f64, element: impl Element) -> impl Element {
    move |width: Option<f64>, draw: Option<DrawContext>| {
        if let Some(ctx) = draw {
            match ctx.next_draw_pos {
                Some(next_draw_pos) if ctx.draw_pos.height_available < threshold => {
                    let new_draw_pos = next_draw_pos(ctx.pdf, 0, [0.; 2]);

                    element.element(
                        width,
                        Some(DrawContext {
                            pdf: ctx.pdf,
                            draw_pos: new_draw_pos,
                            full_height: ctx.full_height,
                            next_draw_pos: Some(&mut |pdf, draw_rect, size| {
                                next_draw_pos(pdf, draw_rect + 1, size)
                            }),
                        }),
                    )
                }
                _ => element.element(width, Some(ctx)),
            }
        } else {
            element.element(width, None)
        }
    }
}
