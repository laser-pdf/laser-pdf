use crate::*;

// TODO: rename this to something like repeating_title
pub struct RepeatAfterBreak<T: Element, C: Element> {
    title: T,
    content: C,
    gap: f64,
    vanish_if_empty: bool,
}

impl<T: Element, C: Element> Element for RepeatAfterBreak<T> {
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

        let mut first_page = true;

        let content_size = if let Some(next_location) = next_location {
            let location_offset;
            let content_location = if vanish_if_empty || location.height_available >= title_height {
                location_offset = 0;
                Location {
                    pos: [location.pos[0], location.pos[1] - gap - title_size[1]],
                    preferred_height: None,
                    height_available: location.height_available - gap - title_size[1],
                    layer: location.layer.clone(),
                }
            } else {
                location_offset = 1;
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

                first_page = false;

                Location {
                    pos: [location.pos[0], location.pos[1] - gap - title_size[1]],
                    preferred_height: None,
                    height_available: location.height_available - gap - title_size[1],
                    layer: location.layer,
                }
            };

            let mut page = location_offset;

            content.draw(
                width,
                Some(DrawCtx {
                    pdf,
                    location: content_location,
                    breakable: Some(BreakableDraw {
                        get_location: &mut |pdf, draw_rect_id| {
                            if first_page && size[1] > 0. {
                                title.draw(
                                    width,
                                    Some(DrawCtx {
                                        pdf,
                                        location: location.clone(),
                                        next_location: None,
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

                            while page <= location_offset + draw_rect_id {
                                let location = next_location(pdf, page, size);
                                title.draw(
                                    width,
                                    Some(DrawCtx {
                                        pdf,
                                        location: location.clone(),
                                        next_location: None,
                                        full_height: 0.,
                                    }),
                                );
                                page += 1;
                            }

                            let mut new_location =
                                next_location(pdf, location_offset + draw_rect_id, size);

                            page = page.max(location_offset + draw_rect_id + 1);

                            // title.element(
                            //     width,
                            //     Some(DrawCtx {
                            //         pdf,
                            //         location: new_location.clone(),
                            //         next_location: None,
                            //         full_height: 0.,
                            //     }),
                            // );

                            new_location.pos[1] -= title_height;
                            new_location.height_available -= title_height;

                            new_location
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

        if first_page {
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

        Some(ElementSize {
            width: content_size[0].max(title_size[0]),
            height: Some(content_size[1] + title_height),
        })
    }
}
