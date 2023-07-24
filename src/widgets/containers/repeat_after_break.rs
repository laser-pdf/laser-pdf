use crate::*;

// TODO: rename this to something like repeating_title
pub fn repeat_after_break<T: Element, C: Element>(
    title: T,
    content: C,
    gap: f64,
    vanish_if_empty: bool,
) -> impl Element {
    move |width: Option<f64>, draw: Option<DrawContext>| {
        let title_size = title.element(width, None);
        let title_height = title_size[1] + gap;

        if let Some(DrawContext {
            pdf,
            draw_pos,
            next_draw_pos,
            full_height,
        }) = draw
        {
            let mut first_page = true;

            let content_size = if let Some(next_draw_pos) = next_draw_pos {
                let draw_pos_offset;
                let content_draw_pos =
                    if vanish_if_empty || draw_pos.height_available >= title_height {
                        draw_pos_offset = 0;
                        DrawPos {
                            pos: [draw_pos.pos[0], draw_pos.pos[1] - gap - title_size[1]],
                            preferred_height: None,
                            height_available: draw_pos.height_available - gap - title_size[1],
                            layer: draw_pos.layer.clone(),
                        }
                    } else {
                        draw_pos_offset = 1;
                        let draw_pos = next_draw_pos(pdf, 0, [0.; 2]);

                        title.element(
                            width,
                            Some(DrawContext {
                                pdf,
                                draw_pos: draw_pos.clone(),
                                next_draw_pos: None,
                                full_height: 0.,
                            }),
                        );

                        first_page = false;

                        DrawPos {
                            pos: [draw_pos.pos[0], draw_pos.pos[1] - gap - title_size[1]],
                            preferred_height: None,
                            height_available: draw_pos.height_available - gap - title_size[1],
                            layer: draw_pos.layer,
                        }
                    };

                let mut page = draw_pos_offset;

                content.element(
                    width,
                    Some(DrawContext {
                        pdf,
                        draw_pos: content_draw_pos,
                        next_draw_pos: Some(&mut |pdf, draw_rect_id, size| {
                            if first_page && size[1] > 0. {
                                title.element(
                                    width,
                                    Some(DrawContext {
                                        pdf,
                                        draw_pos: draw_pos.clone(),
                                        next_draw_pos: None,
                                        full_height: 0.,
                                    }),
                                );
                            }

                            let size = [
                                size[0].max(title_size[0]),
                                if size[1] == 0. {
                                    0.
                                } else {
                                    size[1] + title_height
                                },
                            ];

                            first_page = false;

                            while page <= draw_pos_offset + draw_rect_id {
                                let draw_pos = next_draw_pos(pdf, page, size);
                                title.element(
                                    width,
                                    Some(DrawContext {
                                        pdf,
                                        draw_pos: draw_pos.clone(),
                                        next_draw_pos: None,
                                        full_height: 0.,
                                    }),
                                );
                                page += 1;
                            }

                            let mut new_draw_pos =
                                next_draw_pos(pdf, draw_pos_offset + draw_rect_id, size);

                            page = page.max(draw_pos_offset + draw_rect_id + 1);

                            // title.element(
                            //     width,
                            //     Some(DrawContext {
                            //         pdf,
                            //         draw_pos: new_draw_pos.clone(),
                            //         next_draw_pos: None,
                            //         full_height: 0.,
                            //     }),
                            // );

                            new_draw_pos.pos[1] -= title_height;
                            new_draw_pos.height_available -= title_height;

                            new_draw_pos
                        }),
                        full_height,
                    }),
                )
            } else {
                content.element(
                    width,
                    Some(DrawContext {
                        pdf,
                        draw_pos: DrawPos {
                            pos: [draw_pos.pos[0], draw_pos.pos[1] - gap - title_size[1]],
                            height_available: draw_pos.height_available - gap - title_size[1],
                            layer: draw_pos.layer.clone(),
                            ..draw_pos
                        },
                        next_draw_pos: None,
                        full_height,
                    }),
                )
            };

            if first_page {
                if vanish_if_empty && content_size[1] <= 0. {
                    return [0.; 2];
                } else {
                    title.element(
                        width,
                        Some(DrawContext {
                            pdf,
                            draw_pos,
                            next_draw_pos: None,
                            full_height: 0.,
                        }),
                    );
                }
            }

            [
                content_size[0].max(title_size[0]),
                content_size[1] + title_height,
            ]
        } else {
            let content_size = content.element(width, None);

            [
                content_size[0].max(title_size[0]),
                content_size[1] + title_height,
            ]
        }
    }
}
