use crate::*;

/// Note(flo): Instead of starting to draw the content of the widget we could change the widget
/// interface to allow us to ask a widget for its minimum content height. This would of course break
/// the way widgets can be created as closures and might mean more manual work when creating a
/// wrapper widget. So for now this approach wins.
pub struct Titled<T: Element, C: Element> {
    title: T,
    content: C,
    gap: f64,
    vanish_if_empty: bool,
}

impl<T: Element, C: Element> Element for Titled<T> {
    fn measure(&self, ctx: MeasureCtx) -> Option<ElementSize> {
        let title_size = title.draw(width, None);
        let title_height = title_size[1] + gap;

        let content_size = content.draw(width, None);

        Some(ElementSize {
            width: content_size[0].max(title_size[0]),
            height: Some(content_size[1] + title_height),
        })
    }

    fn draw(&self, ctx: DrawCtx) -> Option<ElementSize> {
        let title_size = title.draw(width, None);
        let title_height = title_size[1] + gap;

        let mut page = 0;
        let mut title_on_page = 0;
        let mut title_drawn = false;

        let content_size = if let Some(next_location) = next_location {
            // let content_location = if location.height_available >= title_height {
            //     DrawPos {
            //         pos: [location.pos[0], location.pos[1] - gap - title_size[1]],
            //         height_available: location.height_available - gap - title_size[1],
            //         layer: location.layer.clone(),
            //     }
            // } else if vanish_if_empty {
            //     DrawPos {
            //         height_available: 0.,
            //         ..location.clone()
            //     }
            // } else {
            //     location = next_location(pdf, [0.; 2]);

            //     DrawPos {
            //         pos: [location.pos[0], location.pos[1] - gap - title_size[1]],
            //         height_available: location.height_available - gap - title_size[1],
            //         layer: location.layer.clone(),
            //     }
            // };

            let content_location;
            let first_draw_rect;

            if vanish_if_empty || location.height_available >= title_height {
                first_draw_rect = 0;
                content_location = Location {
                    pos: [location.pos[0], location.pos[1] - gap - title_size[1]],
                    preferred_height: None,
                    height_available: location.height_available - gap - title_size[1],
                    layer: location.layer.clone(),
                };
            } else {
                let location = next_location(pdf, 0, [0.; 2]);

                title.draw(
                    width,
                    Some(DrawCtx {
                        pdf,
                        location: location.clone(),
                        next_location: None,
                        full_height: 0.,
                    }),
                );

                title_drawn = true;

                // location.pos[1] -= title_height;
                // location.height_available -= title_height;

                first_draw_rect = 1;
                content_location = Location {
                    pos: [location.pos[0], location.pos[1] - gap - title_size[1]],
                    preferred_height: None,
                    height_available: location.height_available - gap - title_size[1],
                    layer: location.layer,
                };
            };

            content.draw(
                width,
                Some(DrawCtx {
                    pdf,
                    location: content_location,
                    // location: DrawPos {
                    //     pos: [location.pos[0], location.pos[1] - gap - title_size[1]],
                    //     height_available: if location.height_available <= title_height {
                    //         // next_location could be called before starting the render but when
                    //         // combined with vanish_if_empty this could mean an unneeded empty
                    //         // page at the end of the document. So what we do here is let the
                    //         // content trigger a page break if there's any content at all.
                    //         0.
                    //     } else {
                    //         location.height_available - gap - title_size[1]
                    //     },
                    //     layer: location.layer.clone(),
                    //     ..location
                    // },
                    breakable: Some(BreakableDraw {
                        get_location: &mut |pdf, draw_rect_id| {
                            let draw_rect_id = draw_rect_id + first_draw_rect;
                            page = page.max(draw_rect_id + 1);
                            if title_drawn {
                                let size = if title_on_page == draw_rect_id {
                                    [size[0].max(title_size[0]), size[1] + gap + title_size[1]]
                                } else {
                                    size
                                };
                                // title_on_current_page = false;
                                let mut new_location = next_location(pdf, draw_rect_id, size);

                                if title_on_page == draw_rect_id + 1 {
                                    new_location.pos[1] -= title_height;
                                }

                                new_location
                            } else if size[1] > 0. {
                                // Title is only drawn on the upper page when some of the content is
                                // there too.

                                title.draw(
                                    width,
                                    Some(DrawCtx {
                                        pdf,
                                        location: location.clone(),
                                        next_location: None,
                                        full_height: 0.,
                                    }),
                                );

                                title_drawn = true;
                                title_on_page = draw_rect_id;

                                next_location(
                                    pdf,
                                    draw_rect_id,
                                    [size[0].max(title_size[0]), size[1] + title_height],
                                )
                            } else {
                                let mut location = next_location(pdf, draw_rect_id, [0.; 2]);

                                title.draw(
                                    width,
                                    Some(DrawCtx {
                                        pdf,
                                        location: location.clone(),
                                        next_location: None,
                                        full_height: 0.,
                                    }),
                                );

                                title_drawn = true;
                                title_on_page = draw_rect_id + 1;

                                location.pos[1] -= title_height;
                                location.height_available -= title_height;

                                location
                            }
                        },
                        ..break_ctx
                    }),
                    full_height,
                }),
            )
        } else {
            content.draw(
                width,
                Some(DrawCtx {
                    pdf,
                    location: Location {
                        pos: [location.pos[0], location.pos[1] - gap - title_size[1]],
                        height_available: location.height_available - gap - title_size[1],
                        layer: location.layer.clone(),
                        ..location
                    },
                    next_location: None,
                    full_height,
                }),
            )
        };

        if !title_drawn {
            if vanish_if_empty && content_size[1] <= 0. {
                return [0.; 2];
            } else {
                title.draw(
                    width,
                    Some(DrawCtx {
                        pdf,
                        location,
                        next_location: None,
                        full_height: 0.,
                    }),
                );
            }
        }

        if title_on_page == page {
            Some(ElementSize {
                width: content_size[0].max(title_size[0]),
                height: Some(content_size[1] + title_height),
            })
        } else {
            content_size
        }
    }
}
