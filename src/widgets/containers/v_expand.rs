use crate::*;

pub fn v_expand<W: Element>(element: W) -> impl Element {
    move |width: Option<f64>, draw: Option<DrawContext>| {
        let widget_size = element.element(width, None);

        if let Some(ctx) = draw {
            let preferred_height = ctx
                .draw_pos
                .preferred_height
                .unwrap_or(0.)
                .max(widget_size[1]);

            dbg!(ctx.draw_pos.preferred_height, preferred_height);

            let [width, height] = element.element(
                width,
                Some(DrawContext {
                    pdf: ctx.pdf,
                    draw_pos: DrawPos {
                        pos: [ctx.draw_pos.pos[0], ctx.draw_pos.pos[1]],
                        preferred_height: Some(preferred_height),
                        ..ctx.draw_pos
                    },
                    full_height: preferred_height,
                    next_draw_pos: None,
                }),
            );

            [width, height.max(preferred_height)]
        } else {
            widget_size
        }
    }
}
