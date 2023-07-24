use crate::*;

/// Draws `a` above `b` except if a page break occurs such that all of `b` is on the new page.
pub fn widget_or_break(a: impl Element, b: impl Element, vanish_if_empty: bool) -> impl Element {
    move |width: Option<f64>, draw: Option<DrawContext>| {
        if let Some(draw) = draw {
            let mut draw_pos = draw.draw_pos;

            let a_size = a.element(width, None);

            let size = if let Some(next_draw_pos) = draw.next_draw_pos {
                let mut draw_a = true;
                let mut a_on_current_page = true;

                let mut first_page = true;

                // let b_draw_pos = if vanish_if_empty || draw_pos.height_available >= a_size[1] {
                //     // draw_pos = next_draw_pos(draw.pdf, [0.; 2]);

                //     // Force a break in the widget if it has any content at all.
                //     // DrawPos {
                //     //     layer: draw_pos.layer.clone(),
                //     //     pos: draw_pos.pos,
                //     //     height_available: 0.,
                //     // }
                //     DrawPos {
                //         layer: draw_pos.layer.clone(),
                //         pos: [
                //             draw_pos.pos[0],
                //             draw_pos.pos[1] - a_size[1],
                //         ],
                //         height_available: draw_pos.height_available - a_size[1],
                //     }

                //     // if draw_pos.height_available < draw.full_height {
                //     //     DrawPos {
                //     //         layer: draw_pos.layer.clone(),
                //     //         pos: [
                //     //             draw_pos.pos[0],
                //     //             draw_pos.pos[1] - a_size[1],
                //     //         ],
                //     //         height_available: draw_pos.height_available - a_size[1],
                //     //     }
                //     // } else {
                //     //     // The new draw_pos is clear so no a needed.
                //     //     draw_a = false;
                //     //     a_on_current_page = false;
                //     //     draw_pos.clone()
                //     // }
                // } else {
                //     first_page = false;
                //     a_on_current_page = false;
                //     draw_a = false;
                //     next_draw_pos(draw.pdf, [0.; 2])
                //     // DrawPos {
                //     //     layer: draw_pos.layer.clone(),
                //     //     pos: [
                //     //         draw_pos.pos[0],
                //     //         draw_pos.pos[1] - a_size[1],
                //     //     ],
                //     //     height_available: draw_pos.height_available - a_size[1],
                //     // }
                // };

                let space_for_a = draw_pos.height_available >= a_size[1];

                let b_draw_pos = DrawPos {
                    layer: draw_pos.layer.clone(),
                    pos: [draw_pos.pos[0], draw_pos.pos[1] - a_size[1]],
                    preferred_height: None,
                    height_available: draw_pos.height_available - a_size[1],
                };

                let b_size = b.element(
                    width,
                    Some(DrawContext {
                        pdf: draw.pdf,
                        draw_pos: b_draw_pos,
                        full_height: draw.full_height,
                        next_draw_pos: Some(&mut |pdf, draw_rect_id, size| {
                            if !first_page {
                                next_draw_pos(pdf, draw_rect_id, size)
                            } else if size[1] > 0. {
                                first_page = false;
                                a_on_current_page = false;

                                next_draw_pos(
                                    pdf,
                                    draw_rect_id,
                                    [size[0].max(a_size[0]), size[1] + a_size[1]],
                                )
                            } else {
                                first_page = false;
                                let mut new_draw_pos =
                                    next_draw_pos(pdf, draw_rect_id, [size[0], 0.]);

                                if new_draw_pos.height_available < draw.full_height {
                                    draw_pos = new_draw_pos.clone();
                                    new_draw_pos.pos[1] -= a_size[1];
                                    new_draw_pos.height_available -= a_size[1];
                                } else {
                                    draw_a = false;
                                    a_on_current_page = false;
                                }

                                new_draw_pos
                            }
                        }),
                    }),
                );

                if (vanish_if_empty || !space_for_a) && first_page && b_size[1] <= 0. {
                    return [0.; 2];
                }

                if draw_a {
                    a.element(
                        width,
                        Some(DrawContext {
                            pdf: draw.pdf,
                            draw_pos,
                            full_height: 0.,
                            next_draw_pos: None,
                        }),
                    );
                }

                if a_on_current_page {
                    [a_size[0].max(b_size[0]), a_size[1] + b_size[1]]
                } else {
                    b_size
                }
            } else {
                let b_size = b.element(
                    width,
                    Some(DrawContext {
                        pdf: draw.pdf,
                        draw_pos: DrawPos {
                            pos: [draw_pos.pos[0], draw_pos.pos[1] - a_size[1]],
                            height_available: draw_pos.height_available - a_size[1],
                            ..draw_pos.clone()
                        },
                        full_height: 0.,
                        next_draw_pos: None,
                    }),
                );

                if vanish_if_empty && b_size[1] <= 0. {
                    return [0.; 2];
                }

                // I think it's better to always have a last in the pdf than just sometimes.
                a.element(
                    width,
                    Some(DrawContext {
                        pdf: draw.pdf,
                        draw_pos,
                        full_height: 0.,
                        next_draw_pos: None,
                    }),
                );

                [a_size[0].max(b_size[0]), a_size[1] + b_size[1]]
            };

            size
        } else {
            let a_size = a.element(width, None);
            let b_size = b.element(width, None);

            if vanish_if_empty && b_size[1] <= 0. {
                [0.; 2]
            } else {
                [a_size[0].max(b_size[0]), a_size[1] + b_size[1]]
            }
        }
    }
}
