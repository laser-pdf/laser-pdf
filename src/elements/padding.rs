use crate::*;

pub struct Padding<'a, E: Element> {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub element: &'a E,
}

impl<'a, E: Element> Padding<'a, E> {
    pub fn left(left: f32, element: &'a E) -> Self {
        Padding {
            left,
            right: 0.,
            top: 0.,
            bottom: 0.,
            element,
        }
    }

    pub fn right(right: f32, element: &'a E) -> Self {
        Padding {
            left: 0.,
            right,
            top: 0.,
            bottom: 0.,
            element,
        }
    }

    pub fn top(top: f32, element: &'a E) -> Self {
        Padding {
            left: 0.,
            right: 0.,
            top,
            bottom: 0.,
            element,
        }
    }

    pub fn bottom(bottom: f32, element: &'a E) -> Self {
        Padding {
            left: 0.,
            right: 0.,
            top: 0.,
            bottom,
            element,
        }
    }
}

impl<'a, E: Element> Element for Padding<'a, E> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        self.element.first_location_usage(FirstLocationUsageCtx {
            width: self.width(ctx.width),
            first_height: self.height(ctx.first_height),
            full_height: self.height(ctx.full_height),
        })
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        let mut break_count = 0;
        let mut extra_location_min_height = None;

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

            // TODO: Subtract padding
            *b.extra_location_min_height = extra_location_min_height;
        }

        self.size(size)
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
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

            preferred_height: ctx.preferred_height.map(|p| self.height(p)),

            width,
            first_height: self.height(ctx.first_height),
            breakable: None,
        };

        let size = if let Some(breakable) = ctx.breakable {
            self.element.draw(DrawCtx {
                breakable: Some(BreakableDraw {
                    full_height: self.height(breakable.full_height),
                    preferred_height_break_count: breakable.preferred_height_break_count,
                    do_break: &mut |pdf, location_idx, height| {
                        let mut location = (breakable.do_break)(
                            pdf,
                            location_idx,
                            height.map(|h| h + self.top + self.bottom),
                        );

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

impl<'a, E: Element> Padding<'a, E> {
    fn width(&self, constraint: WidthConstraint) -> WidthConstraint {
        WidthConstraint {
            max: constraint.max - self.left - self.right,
            expand: constraint.expand,
        }
    }

    fn height(&self, input: f32) -> f32 {
        input - self.top - self.bottom
    }

    fn size(&self, size: ElementSize) -> ElementSize {
        ElementSize {
            width: size.width.map(|w| w + self.left + self.right),
            height: size.height.map(|h| h + self.top + self.bottom),
        }
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
                element: &proxy,
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
            output.assert_size(ElementSize {
                width: Some(output.width.constrain(35.)),
                height: Some(if output.breakable.is_none() {
                    10. + 29.
                } else if output.first_height == 30.1 {
                    1. + 29.
                } else {
                    2. + 29.
                }),
            });

            if let Some(b) = output.breakable {
                b.assert_break_count(if output.first_height == 30.1 { 5 } else { 4 })
                    .assert_extra_location_min_height(None);
            }
        }
    }
}
