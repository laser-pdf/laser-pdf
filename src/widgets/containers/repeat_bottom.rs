use crate::*;

pub struct RepeatBottom<C: Element, B: Element> {
    content: C,
    bottom: B,
    gap: f64,
    // vanish_if_empty: bool,
}

impl<C: Element, B: Element> Element for RepeatBottom<T> {
    fn measure(&self, ctx: MeasureCtx) -> Option<ElementSize> {
        let bottom_size = bottom.draw(width, None);
        let bottom_height = bottom_size[1] + gap;

        let content_size = content.draw(width, None);

        Some(ElementSize {
            width: content_size[0].max(bottom_size[0]),
            height: Some(content_size[1] + bottom_height),
        })
    }

    fn draw(&self, ctx: DrawCtx) -> Option<ElementSize> {
        let bottom_size = bottom.draw(width, None);
        let bottom_height = bottom_size[1] + gap;

        let mut location = ctx.location;

        let content_size = if let Some(next_location) = ctx.next_location {
            content.draw(
                width,
                Some(DrawCtx {
                    pdf: ctx.pdf,
                    location: Location {
                        layer: location.layer.clone(),
                        pos: location.pos,
                        preferred_height: None,
                        height_available: location.height_available - bottom_height,
                    },
                    full_height: (ctx.full_height - bottom_height).max(0.),
                    breakable: Some(BreakableDraw {
                        get_location: &mut |pdf, draw_rect_id| {
                            bottom.draw(
                                width,
                                Some(DrawCtx {
                                    pdf,
                                    location: Location {
                                        layer: location.layer.clone(),
                                        pos: [location.pos[0], location.pos[1] - size[1] - gap],
                                        preferred_height: None,
                                        height_available: bottom_size[1],
                                    },
                                    full_height: 0.,
                                    next_location: None,
                                }),
                            );
                            location = next_location(
                                pdf,
                                draw_rect_id,
                                [size[0].max(bottom_size[0]), size[1] + bottom_height],
                            );
                            location.clone()
                        },
                        ..break_ctx
                    }),
                }),
            )
        } else {
            content.draw(
                width,
                Some(DrawCtx {
                    pdf: ctx.pdf,
                    location: Location {
                        layer: location.layer.clone(),
                        height_available: location.height_available - bottom_height,
                        preferred_height: None,
                        ..location
                    },
                    full_height: 0.,
                    next_location: None,
                }),
            )
        };

        bottom.draw(
            width,
            Some(DrawCtx {
                pdf: ctx.pdf,
                location: Location {
                    layer: location.layer.clone(),
                    pos: [location.pos[0], location.pos[1] - content_size[1] - gap],
                    preferred_height: None,
                    height_available: bottom_size[1],
                },
                full_height: 0.,
                next_location: None,
            }),
        );

        Some(ElementSize {
            width: content_size[0].max(bottom_height),
            height: Some(content_size[1] + bottom_size[1]),
        })
    }
}
