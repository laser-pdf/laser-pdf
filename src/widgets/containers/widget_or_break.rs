use crate::*;

/// Draws `a` above `b` except if a page break occurs such that all of `b` is on the new page.
pub struct WidgetOrBreak {
    a: impl Element,
    b: impl Element,
    vanish_if_empty: bool,
}

impl Element for WidgetOrBreak<T> {
    fn measure(&self, ctx: MeasureCtx) -> Option<ElementSize> {
        let a_size = a.draw(width, None);
        let b_size = b.draw(width, None);

        if vanish_if_empty && b_size[1] <= 0. {
            [0.; 2]
        } else {
            Some(ElementSize {
                width: a_size[0].max(b_size[0]),
                height: Some(a_size[1] + b_size[1]),
            })
        }
    }

    fn draw(&self, ctx: DrawCtx) -> Option<ElementSize> {
        let mut location = ctx.location;

        let a_size = a.draw(width, None);

        let size = if let Some(next_location) = draw.next_location {
            let mut draw_a = true;
            let mut a_on_current_page = true;

            let mut first_page = true;

            // let b_location = if vanish_if_empty || location.height_available >= a_size[1] {
            //     // location = next_location(draw.pdf, [0.; 2]);

            //     // Force a break in the widget if it has any content at all.
            //     // DrawPos {
            //     //     layer: location.layer.clone(),
            //     //     pos: location.pos,
            //     //     height_available: 0.,
            //     // }
            //     DrawPos {
            //         layer: location.layer.clone(),
            //         pos: [
            //             location.pos[0],
            //             location.pos[1] - a_size[1],
            //         ],
            //         height_available: location.height_available - a_size[1],
            //     }

            //     // if location.height_available < draw.full_height {
            //     //     DrawPos {
            //     //         layer: location.layer.clone(),
            //     //         pos: [
            //     //             location.pos[0],
            //     //             location.pos[1] - a_size[1],
            //     //         ],
            //     //         height_available: location.height_available - a_size[1],
            //     //     }
            //     // } else {
            //     //     // The new location is clear so no a needed.
            //     //     draw_a = false;
            //     //     a_on_current_page = false;
            //     //     location.clone()
            //     // }
            // } else {
            //     first_page = false;
            //     a_on_current_page = false;
            //     draw_a = false;
            //     next_location(draw.pdf, [0.; 2])
            //     // DrawPos {
            //     //     layer: location.layer.clone(),
            //     //     pos: [
            //     //         location.pos[0],
            //     //         location.pos[1] - a_size[1],
            //     //     ],
            //     //     height_available: location.height_available - a_size[1],
            //     // }
            // };

            let space_for_a = location.height_available >= a_size[1];

            let b_location = Location {
                layer: location.layer.clone(),
                pos: [location.pos[0], location.pos[1] - a_size[1]],
                preferred_height: None,
                height_available: location.height_available - a_size[1],
            };

            let b_size = b.draw(
                width,
                Some(DrawCtx {
                    pdf: draw.pdf,
                    location: b_location,
                    full_height: draw.full_height,
                    breakable: Some(BreakableDraw {
                        get_location: &mut |pdf, draw_rect_id| {
                            if !first_page {
                                next_location(pdf, draw_rect_id, size)
                            } else if size[1] > 0. {
                                first_page = false;
                                a_on_current_page = false;

                                next_location(
                                    pdf,
                                    draw_rect_id,
                                    [size[0].max(a_size[0]), size[1] + a_size[1]],
                                )
                            } else {
                                first_page = false;
                                let mut new_location =
                                    next_location(pdf, draw_rect_id, [size[0], 0.]);

                                if new_location.height_available < draw.full_height {
                                    location = new_location.clone();
                                    new_location.pos[1] -= a_size[1];
                                    new_location.height_available -= a_size[1];
                                } else {
                                    draw_a = false;
                                    a_on_current_page = false;
                                }

                                new_location
                            }
                        },
                        ..break_ctx
                    }),
                }),
            );

            if (vanish_if_empty || !space_for_a) && first_page && b_size[1] <= 0. {
                return [0.; 2];
            }

            if draw_a {
                a.draw(
                    width,
                    Some(DrawCtx {
                        pdf: draw.pdf,
                        location,
                        full_height: 0.,
                        next_location: None,
                    }),
                );
            }

            if a_on_current_page {
                Some(ElementSize {
                    width: a_size[0].max(b_size[0]),
                    height: Some(a_size[1] + b_size[1]),
                })
            } else {
                b_size
            }
        } else {
            let b_size = b.draw(
                width,
                Some(DrawCtx {
                    pdf: draw.pdf,
                    location: Location {
                        pos: [location.pos[0], location.pos[1] - a_size[1]],
                        height_available: location.height_available - a_size[1],
                        ..location.clone()
                    },
                    full_height: 0.,
                    next_location: None,
                }),
            );

            if vanish_if_empty && b_size[1] <= 0. {
                return [0.; 2];
            }

            // I think it's better to always have a last in the pdf than just sometimes.
            a.draw(
                width,
                Some(DrawCtx {
                    pdf: draw.pdf,
                    location,
                    full_height: 0.,
                    next_location: None,
                }),
            );

            Some(ElementSize {
                width: a_size[0].max(b_size[0]),
                height: Some(a_size[1] + b_size[1]),
            })
        };

        size
    }
}
