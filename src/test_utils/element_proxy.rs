use crate::*;

#[non_exhaustive]
pub struct ElementProxy<'a, E: Element> {
    pub element: E,
    pub before_draw: &'a dyn Fn(&mut DrawCtx),
    pub after_break: &'a dyn Fn(u32, &Location, WidthConstraint, f64),
}

impl<'a, E: Element> ElementProxy<'a, E> {
    pub fn new(element: E) -> Self {
        ElementProxy {
            element,
            before_draw: &|_| {},
            after_break: &|_, _, _, _| {},
        }
    }
}

impl<'a, E: Element> Element for ElementProxy<'a, E> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        self.element.first_location_usage(ctx)
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        self.element.measure(ctx)
    }

    fn draw(&self, mut ctx: DrawCtx) -> ElementSize {
        (self.before_draw)(&mut ctx);

        if let Some(breakable) = ctx.breakable {
            self.element.draw(DrawCtx {
                breakable: Some(BreakableDraw {
                    get_location: &mut |pdf, location_idx| {
                        let location = (breakable.get_location)(pdf, location_idx);

                        (self.after_break)(location_idx, &location, ctx.width, ctx.first_height);

                        location
                    },
                    ..breakable
                }),
                ..ctx
            })
        } else {
            self.element.draw(ctx)
        }
    }
}
