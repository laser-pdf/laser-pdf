use crate::*;

pub struct MinFirstHeight<'a, E: Element> {
    pub element: &'a E,
    pub min_first_height: f64,
}

impl<'a, E: Element> Element for MinFirstHeight<'a, E> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        use FirstLocationUsage::*;

        if self.pre_break(ctx.first_height, ctx.full_height) {
            match self.element.first_location_usage(FirstLocationUsageCtx {
                width: ctx.width,
                first_height: ctx.full_height,
                full_height: ctx.full_height,
            }) {
                NoneHeight => NoneHeight, // collapse
                _ => WillSkip,
            }
        } else {
            self.element.first_location_usage(ctx)
        }
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        let location_offset;
        let mut size;
        let mut break_count = 0;

        if let Some(breakable) = ctx.breakable {
            let first_height;

            if self.pre_break(ctx.first_height, breakable.full_height) {
                first_height = breakable.full_height;
                location_offset = 1;
            } else {
                first_height = ctx.first_height;
                location_offset = 0;
            }

            size = self.element.measure(MeasureCtx {
                width: ctx.width,
                first_height,
                breakable: Some(BreakableMeasure {
                    full_height: breakable.full_height,
                    break_count: &mut break_count,
                    extra_location_min_height: breakable.extra_location_min_height,
                }),
            });

            // collapse
            if size.height.is_none() && break_count == 0 {
                return size;
            }

            *breakable.break_count = break_count + location_offset;
        } else {
            location_offset = 0;
            size = self.element.measure(ctx);
        }

        if let Some(ref mut height) = size.height {
            if location_offset == 0 && break_count == 0 {
                *height = height.max(self.min_first_height);
            }
        }

        size
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        let location_offset;
        let mut size;
        let mut break_count = 0;

        if let Some(breakable) = ctx.breakable {
            let location;
            let first_height;
            let preferred_height;

            if self.pre_break(ctx.first_height, breakable.full_height)
                // needed for collapse:
                && self.element.first_location_usage(FirstLocationUsageCtx {
                    width: ctx.width,
                    first_height: breakable.full_height,
                    full_height: breakable.full_height,
                }) != FirstLocationUsage::NoneHeight
            {
                location = (breakable.get_location)(ctx.pdf, 0);
                location_offset = 1;
                first_height = breakable.full_height;
                preferred_height = if breakable.preferred_height_break_count == 0 {
                    None
                } else {
                    ctx.preferred_height
                };
            } else {
                location = ctx.location;
                location_offset = 0;
                first_height = ctx.first_height;
                preferred_height = ctx.preferred_height;
            }

            size = self.element.draw(DrawCtx {
                pdf: ctx.pdf,
                location,
                width: ctx.width,
                first_height,
                preferred_height,
                breakable: Some(BreakableDraw {
                    full_height: breakable.full_height,
                    preferred_height_break_count: breakable
                        .preferred_height_break_count
                        .saturating_sub(location_offset),

                    get_location: &mut |pdf, location_idx| {
                        break_count = break_count.max(location_idx + 1);
                        (breakable.get_location)(pdf, location_idx + location_offset)
                    },
                }),
            });
        } else {
            location_offset = 0;
            size = self.element.draw(ctx);
        }

        if let Some(ref mut height) = size.height {
            if location_offset == 0 && break_count == 0 {
                *height = height.max(self.min_first_height);
            }
        }

        size
    }
}

impl<'a, E: Element> MinFirstHeight<'a, E> {
    fn pre_break(&self, first_height: f64, full_height: f64) -> bool {
        first_height < full_height && first_height < self.min_first_height
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        elements::none::NoneElement,
        test_utils::{record_passes::RecordPasses, *},
    };
    use insta::assert_debug_snapshot;

    #[test]
    fn test_unbreakable() {
        let output = test_element(
            TestElementParams {
                width: WidthConstraint {
                    max: 12.,
                    expand: false,
                },
                first_height: 12.,
                breakable: None,
                pos: (7., 20.0),
                ..Default::default()
            },
            |assert, callback| {
                let content = RecordPasses::new(FakeText {
                    lines: 1,
                    line_height: 5.,
                    width: 3.,
                });

                let element = MinFirstHeight {
                    element: &content,
                    min_first_height: 10.,
                };

                let ret = callback.call(element);

                if assert {
                    assert_debug_snapshot!(content.into_passes());
                }

                ret
            },
        );

        assert_debug_snapshot!(output);
    }

    #[test]
    fn test_breakable() {
        let output = test_element(
            TestElementParams {
                width: WidthConstraint {
                    max: 12.,
                    expand: false,
                },
                first_height: 12.,
                breakable: Some(TestElementParamsBreakable {
                    preferred_height_break_count: 0,
                    full_height: 15.,
                }),
                pos: (7., 20.0),
                ..Default::default()
            },
            |assert, callback| {
                let content = RecordPasses::new(FakeText {
                    lines: 1,
                    line_height: 5.,
                    width: 3.,
                });

                let element = MinFirstHeight {
                    element: &content,
                    min_first_height: 10.,
                };

                let ret = callback.call(element);

                if assert {
                    assert_debug_snapshot!(content.into_passes());
                }

                ret
            },
        );

        assert_debug_snapshot!(output);
    }

    #[test]
    fn test_pre_break() {
        let output = test_element(
            TestElementParams {
                width: WidthConstraint {
                    max: 12.,
                    expand: false,
                },
                first_height: 9.,
                breakable: Some(TestElementParamsBreakable {
                    preferred_height_break_count: 3,
                    full_height: 15.,
                }),
                pos: (7., 20.0),
                preferred_height: Some(4.),
                ..Default::default()
            },
            |assert, callback| {
                let content = RecordPasses::new(FakeText {
                    lines: 4,
                    line_height: 5.,
                    width: 3.,
                });

                let element = MinFirstHeight {
                    element: &content,
                    min_first_height: 10.,
                };

                let ret = callback.call(element);

                if assert {
                    assert_debug_snapshot!(content.into_passes());
                }

                ret
            },
        );

        assert_debug_snapshot!(output);
    }

    #[test]
    fn test_collapse() {
        let output = test_element(
            TestElementParams {
                width: WidthConstraint {
                    max: 12.,
                    expand: false,
                },
                first_height: 9.,
                breakable: Some(TestElementParamsBreakable {
                    preferred_height_break_count: 3,
                    full_height: 15.,
                }),
                pos: (7., 20.0),
                preferred_height: Some(4.),
                ..Default::default()
            },
            |assert, callback| {
                let content = RecordPasses::new(NoneElement);

                let element = MinFirstHeight {
                    element: &content,
                    min_first_height: 10.,
                };

                let ret = callback.call(element);

                if assert {
                    assert_debug_snapshot!(content.into_passes());
                }

                ret
            },
        );

        assert_debug_snapshot!(output);
    }
}
