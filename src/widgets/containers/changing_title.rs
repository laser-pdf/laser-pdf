use std::cell::OnceCell;

use crate::*;

pub fn changing_title<T: Element, U: Element, C: Element>(
    first_title: T,
    second_title: U,
    content: C,
    gap: f64,
    vanish_if_empty: bool,
) -> impl Element {
    move |width: Option<f64>, draw: Option<DrawContext>| {
        let title_size = first_title.element(width, None);
        let title_height = title_size[1] + gap;

        // this is perhaps a bit silly, but i wanted to try OnceCell
        let second_title_size: OnceCell<([f64; 2], f64)> = OnceCell::new();

        let get_second_title_size = || {
            second_title_size.get_or_init(|| {
                let size = second_title.element(width, None);
                let height = size[1] + gap;

                (size, height)
            })
        };

        if let Some(DrawContext {
            pdf,
            draw_pos,
            next_draw_pos,
            full_height,
        }) = draw
        {
            // in this case this means that the first title hasn't been drawn yet
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

                        first_title.element(
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
                            let mut draw_pos = draw_pos.clone();

                            if first_page {
                                if size[1] <= 0. {
                                    assert!(page == 0);
                                    page += 1;
                                    draw_pos = next_draw_pos(pdf, 0, size);
                                };

                                first_title.element(
                                    width,
                                    Some(DrawContext {
                                        pdf,
                                        draw_pos: draw_pos.clone(),
                                        next_draw_pos: None,
                                        full_height: 0.,
                                    }),
                                );

                                // will be overwritten if this isn't the final page
                                draw_pos.pos[1] -= title_height;
                                draw_pos.height_available -= title_height;
                            }

                            let mut size = [
                                size[0].max(title_size[0]),
                                if size[1] == 0. {
                                    0.
                                } else {
                                    size[1] + title_height
                                },
                            ];

                            first_page = false;

                            while page <= draw_pos_offset + draw_rect_id {
                                let (second_title_size, second_title_height) =
                                    get_second_title_size();

                                size = [
                                    size[0].max(second_title_size[0]),
                                    if size[1] == 0. {
                                        0.
                                    } else {
                                        size[1] + second_title_height
                                    },
                                ];

                                draw_pos = next_draw_pos(pdf, page, size);
                                second_title.element(
                                    width,
                                    Some(DrawContext {
                                        pdf,
                                        draw_pos: draw_pos.clone(),
                                        next_draw_pos: None,
                                        full_height: 0.,
                                    }),
                                );
                                page += 1;
                                draw_pos.pos[1] -= second_title_height;
                                draw_pos.height_available -= second_title_height;
                            }

                            page = page.max(draw_pos_offset + draw_rect_id + 1);

                            draw_pos
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
                    first_title.element(
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
