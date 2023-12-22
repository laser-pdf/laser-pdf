use crate::*;

pub struct AlignBottom<W: Element> {
    element: W,
}

impl<W: Element> Element for AlignBottom<W> {
    fn measure(&self, ctx: MeasureCtx) -> Option<ElementSize> {
        self.element.measure(ctx)
    }

    fn draw(&self, ctx: DrawCtx) -> Option<ElementSize> {
        let widget_size = self.element.measure(width, None);

        let mut location = ctx.location;

        if let Some(next_location) = ctx.next_location {
            if widget_size[1] > location.height_available {
                location = next_location(ctx.pdf, 0, [widget_size[0], 0.]);
            }
        }

        Some(ElementSize {
            width: self.element.draw(
                width,
                Some(DrawCtx {
                    pdf: ctx.pdf,
                    location: Location {
                        pos: [
                            location.pos[0],
                            location.pos[1] - location.height_available + widget_size[1],
                        ],
                        preferred_height: Some(widget_size[1]),
                        ..location
                    },
                    full_height: ctx.full_height,
                    next_location: None,
                }),
            )[0],
            height: Some(location.height_available),
        })
    }
}
