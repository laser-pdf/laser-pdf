use crate::*;

/// Note(flo): Instead of starting to draw the content of the widget we could change the widget
/// interface to allow us to ask a widget for its minimum content height. This would of course break
/// the way widgets can be created as closures and might mean more manual work when creating a
/// wrapper widget. So for now this approach wins.
pub fn titled<T: Element, C: Element>(
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
            let mut page = 0;
            let mut title_on_page = 0;
            let mut title_drawn = false;

            let content_size = if let Some(next_draw_pos) = next_draw_pos {
                // let content_draw_pos = if draw_pos.height_available >= title_height {
                //     DrawPos {
                //         pos: [draw_pos.pos[0], draw_pos.pos[1] - gap - title_size[1]],
                //         height_available: draw_pos.height_available - gap - title_size[1],
                //         layer: draw_pos.layer.clone(),
                //     }
                // } else if vanish_if_empty {
                //     DrawPos {
                //         height_available: 0.,
                //         ..draw_pos.clone()
                //     }
                // } else {
                //     draw_pos = next_draw_pos(pdf, [0.; 2]);

                //     DrawPos {
                //         pos: [draw_pos.pos[0], draw_pos.pos[1] - gap - title_size[1]],
                //         height_available: draw_pos.height_available - gap - title_size[1],
                //         layer: draw_pos.layer.clone(),
                //     }
                // };

                let content_draw_pos;
                let first_draw_rect;

                if vanish_if_empty || draw_pos.height_available >= title_height {
                    first_draw_rect = 0;
                    content_draw_pos = DrawPos {
                        pos: [draw_pos.pos[0], draw_pos.pos[1] - gap - title_size[1]],
                        preferred_height: None,
                        height_available: draw_pos.height_available - gap - title_size[1],
                        layer: draw_pos.layer.clone(),
                    };
                } else {
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

                    title_drawn = true;

                    // draw_pos.pos[1] -= title_height;
                    // draw_pos.height_available -= title_height;

                    first_draw_rect = 1;
                    content_draw_pos = DrawPos {
                        pos: [draw_pos.pos[0], draw_pos.pos[1] - gap - title_size[1]],
                        preferred_height: None,
                        height_available: draw_pos.height_available - gap - title_size[1],
                        layer: draw_pos.layer,
                    };
                };

                content.element(
                    width,
                    Some(DrawContext {
                        pdf,
                        draw_pos: content_draw_pos,
                        // draw_pos: DrawPos {
                        //     pos: [draw_pos.pos[0], draw_pos.pos[1] - gap - title_size[1]],
                        //     height_available: if draw_pos.height_available <= title_height {
                        //         // next_draw_pos could be called before starting the render but when
                        //         // combined with vanish_if_empty this could mean an unneeded empty
                        //         // page at the end of the document. So what we do here is let the
                        //         // content trigger a page break if there's any content at all.
                        //         0.
                        //     } else {
                        //         draw_pos.height_available - gap - title_size[1]
                        //     },
                        //     layer: draw_pos.layer.clone(),
                        //     ..draw_pos
                        // },
                        next_draw_pos: Some(&mut |pdf, draw_rect_id, size| {
                            let draw_rect_id = draw_rect_id + first_draw_rect;
                            page = page.max(draw_rect_id + 1);
                            if title_drawn {
                                let size = if title_on_page == draw_rect_id {
                                    [size[0].max(title_size[0]), size[1] + gap + title_size[1]]
                                } else {
                                    size
                                };
                                // title_on_current_page = false;
                                let mut new_draw_pos = next_draw_pos(pdf, draw_rect_id, size);

                                if title_on_page == draw_rect_id + 1 {
                                    new_draw_pos.pos[1] -= title_height;
                                }

                                new_draw_pos
                            } else if size[1] > 0. {
                                // Title is only drawn on the upper page when some of the content is
                                // there too.

                                title.element(
                                    width,
                                    Some(DrawContext {
                                        pdf,
                                        draw_pos: draw_pos.clone(),
                                        next_draw_pos: None,
                                        full_height: 0.,
                                    }),
                                );

                                title_drawn = true;
                                title_on_page = draw_rect_id;

                                next_draw_pos(
                                    pdf,
                                    draw_rect_id,
                                    [size[0].max(title_size[0]), size[1] + title_height],
                                )
                            } else {
                                let mut draw_pos = next_draw_pos(pdf, draw_rect_id, [0.; 2]);

                                title.element(
                                    width,
                                    Some(DrawContext {
                                        pdf,
                                        draw_pos: draw_pos.clone(),
                                        next_draw_pos: None,
                                        full_height: 0.,
                                    }),
                                );

                                title_drawn = true;
                                title_on_page = draw_rect_id + 1;

                                draw_pos.pos[1] -= title_height;
                                draw_pos.height_available -= title_height;

                                draw_pos
                            }
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

            if !title_drawn {
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

            if title_on_page == page {
                [
                    content_size[0].max(title_size[0]),
                    content_size[1] + title_height,
                ]
            } else {
                content_size
            }
        } else {
            let content_size = content.element(width, None);

            [
                content_size[0].max(title_size[0]),
                content_size[1] + title_height,
            ]
        }
    }
}
