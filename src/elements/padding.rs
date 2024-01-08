use crate::*;

pub struct Padding<E: Element> {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
    pub element: E,
}

impl<E: Element> Element for Padding<E> {
    fn insufficient_first_height(&self, ctx: InsufficientFirstHeightCtx) -> bool {
        self.element
            .insufficient_first_height(InsufficientFirstHeightCtx {
                width: self.width(ctx.width),
                first_height: self.height(ctx.first_height),
                full_height: self.height(ctx.full_height),
            })
    }

    fn measure(&self, ctx: MeasureCtx) -> Option<ElementSize> {
        let mut break_count = 0;
        let mut extra_location_min_height = 0.;

        let size = self.element.measure(MeasureCtx {
            width: self.width(ctx.width),
            first_height: self.height(ctx.first_height),
            breakable: ctx.breakable.as_ref().map(|b| BreakableMeasure {
                break_count: &mut break_count,
                extra_location_min_height: &mut extra_location_min_height,
                full_height: self.height(b.full_height),
            }),
        });

        if let Some(b) = ctx.breakable {
            *b.break_count = break_count;
            *b.extra_location_min_height = extra_location_min_height;
        }

        self.size(size)
    }

    fn draw(&self, ctx: DrawCtx) -> Option<ElementSize> {
        let width = self.width(ctx.width);

        let draw_ctx = DrawCtx {
            pdf: ctx.pdf,

            location: Location {
                pos: (
                    ctx.location.pos.0 + self.left,
                    ctx.location.pos.1 - self.top,
                ),
                ..ctx.location
            },

            preferred_height: self.height(ctx.preferred_height),

            width,
            first_height: self.height(ctx.first_height),
            breakable: None,
        };

        let size = if let Some(breakable) = ctx.breakable {
            self.element.draw(DrawCtx {
                breakable: Some(BreakableDraw {
                    full_height: self.height(breakable.full_height),
                    preferred_height_break_count: breakable.preferred_height_break_count,
                    get_location: &mut |pdf, location_idx| {
                        let mut location = (breakable.get_location)(pdf, location_idx);

                        location.pos.0 += self.left;
                        location.pos.1 -= self.top;

                        location
                    },
                }),
                ..draw_ctx
            })
        } else {
            self.element.draw(draw_ctx)
        };

        self.size(size)
    }
}

impl<E: Element> Padding<E> {
    fn width(&self, constraint: WidthConstraint) -> WidthConstraint {
        WidthConstraint {
            max: constraint.max - self.left - self.right,
            expand: constraint.expand,
        }
    }

    fn height(&self, input: f64) -> f64 {
        input - self.top - self.bottom
    }

    fn size(&self, size: Option<ElementSize>) -> Option<ElementSize> {
        size.map(|size| ElementSize {
            width: size.width + self.left + self.right,
            height: size.height.map(|height| height + self.top + self.bottom),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    #[test]
    fn test_padding() {
        let element = BuildElement(|build_ctx, callback| {
            let content = FakeText {
                width: 10.,
                line_height: 1.,
                lines: 10,
            };

            let proxy = ElementProxy {
                before_draw: &|ctx: &mut DrawCtx| {
                    assert_eq!(
                        ctx.width,
                        WidthConstraint {
                            max: 40. - 25.,
                            expand: build_ctx.width.expand,
                        }
                    );
                    assert_eq!(ctx.location.pos.0, 24.);
                },
                after_break: &|_location_idx: u32,
                               location: &Location,
                               _width: WidthConstraint,
                               _first_height| {
                    assert_eq!(location.pos.0, 24.);
                },
                ..ElementProxy::new(content)
            };
            callback.call(Padding {
                left: 12.,
                right: 13.,
                top: 14.,
                bottom: 15.,
                element: proxy,
            })
        });

        for output in (ElementTestParams {
            first_height: 30.1,
            full_height: 31.5,
            width: 40.,
            ..Default::default()
        })
        .run(&element)
        {
            output.assert_size(Some(ElementSize {
                width: output.width.constrain(35.),
                height: Some(if output.breakable.is_none() {
                    10. + 29.
                } else if output.first_height == 30.1 {
                    1. + 29.
                } else {
                    2. + 29.
                }),
            }));

            if let Some(b) = output.breakable {
                b.assert_break_count(if output.first_height == 30.1 { 5 } else { 4 })
                    .assert_extra_location_min_height(0.);
            }
        }
    }
}
