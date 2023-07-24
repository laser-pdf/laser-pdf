use crate::*;

pub fn align_bottom<W: Element>(element: W) -> impl Element {
    move |width: Option<f64>, draw: Option<DrawContext>| {
        if let Some(ctx) = draw {
            let widget_size = element.element(width, None);

            let mut draw_pos = ctx.draw_pos;

            if let Some(next_draw_pos) = ctx.next_draw_pos {
                if widget_size[1] > draw_pos.height_available {
                    draw_pos = next_draw_pos(ctx.pdf, 0, [widget_size[0], 0.]);
                }
            }

            [
                element.element(
                    width,
                    Some(DrawContext {
                        pdf: ctx.pdf,
                        draw_pos: DrawPos {
                            pos: [
                                draw_pos.pos[0],
                                draw_pos.pos[1] - draw_pos.height_available + widget_size[1],
                            ],
                            preferred_height: Some(widget_size[1]),
                            ..draw_pos
                        },
                        full_height: ctx.full_height,
                        next_draw_pos: None,
                    }),
                )[0],
                draw_pos.height_available,
            ]
        } else {
            element.element(width, draw)
        }
    }
}
