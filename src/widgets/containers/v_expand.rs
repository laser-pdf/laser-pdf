use crate::*;

pub struct VExpand<W: Element> {
    element: W,
}

impl<W: Element> Element for VExpand<T> {
    fn measure(&self, ctx: MeasureCtx) -> Option<ElementSize> {
        let widget_size = element.draw(width, None);

        widget_size
    }

    fn draw(&self, ctx: DrawCtx) -> Option<ElementSize> {
        let widget_size = element.draw(width, None);

        let preferred_height = ctx
            .location
            .preferred_height
            .unwrap_or(0.)
            .max(widget_size[1]);

        let [width, height] = element.draw(
            width,
            Some(DrawCtx {
                pdf: ctx.pdf,
                location: Location {
                    pos: [ctx.location.pos[0], ctx.location.pos[1]],
                    preferred_height: Some(preferred_height),
                    ..ctx.location
                },
                full_height: preferred_height,
                next_location: None,
            }),
        );

        Some(ElementSize {
            width: width,
            height: Some(height.max(preferred_height)),
        })
    }
}
