use crate::*;

pub fn pin_below<C: Element, B: Element>(content: C, pinned_element: B, gap: f64) -> impl Element {
    move |width: Option<f64>, draw: Option<DrawContext>| {
        let pinned_size = pinned_element.element(width, None);
        let pinned_height = pinned_size[1] + gap;

        if let Some(ctx) = draw {
            let mut draw_pos = ctx.draw_pos;

            let content_size = if let Some(next_draw_pos) = ctx.next_draw_pos {
                content.element(
                    width,
                    Some(DrawContext {
                        pdf: ctx.pdf,
                        draw_pos: DrawPos {
                            layer: draw_pos.layer.clone(),
                            pos: draw_pos.pos,
                            preferred_height: None,
                            height_available: draw_pos.height_available - pinned_height,
                        },
                        full_height: (ctx.full_height - pinned_height).max(0.),
                        next_draw_pos: Some(&mut |pdf, draw_rect_id, size| {
                            draw_pos = next_draw_pos(
                                pdf,
                                draw_rect_id,
                                [size[0].max(pinned_size[0]), size[1] + pinned_height],
                            );
                            DrawPos {
                                height_available: draw_pos.height_available - pinned_height,
                                ..draw_pos.clone()
                            }
                        }),
                    }),
                )
            } else {
                content.element(
                    width,
                    Some(DrawContext {
                        pdf: ctx.pdf,
                        draw_pos: DrawPos {
                            layer: draw_pos.layer.clone(),
                            height_available: draw_pos.height_available - pinned_height,
                            preferred_height: None,
                            ..draw_pos
                        },
                        full_height: 0.,
                        next_draw_pos: None,
                    }),
                )
            };

            pinned_element.element(
                width,
                Some(DrawContext {
                    pdf: ctx.pdf,
                    draw_pos: DrawPos {
                        layer: draw_pos.layer.clone(),
                        pos: [draw_pos.pos[0], draw_pos.pos[1] - content_size[1] - gap],
                        preferred_height: None,
                        height_available: pinned_size[1],
                    },
                    full_height: 0.,
                    next_draw_pos: None,
                }),
            );

            [
                content_size[0].max(pinned_size[0]),
                content_size[1] + pinned_height,
            ]
        } else {
            let content_size = content.element(width, None);

            [
                content_size[0].max(pinned_size[0]),
                content_size[1] + pinned_height,
            ]
        }
    }
}
