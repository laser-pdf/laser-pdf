use crate::*;

pub struct RefElement<'a, E: Element>(pub &'a E);

impl<'a, E: Element> Element for RefElement<'a, E> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        self.0.first_location_usage(ctx)
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        self.0.measure(ctx)
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        self.0.draw(ctx)
    }
}