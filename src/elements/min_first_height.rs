use crate::*;

pub struct MinFirstHeight<E: Element> {
    pub element: E,
    pub min_first_height: f32,
}

impl<E: Element> Element for MinFirstHeight<E> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        use FirstLocationUsage::*;

        let layout = self.layout(ctx.width, ctx.first_height, ctx.full_height);

        if layout.pre_break {
            match self.element.first_location_usage(FirstLocationUsageCtx {
                width: ctx.width,
                first_height: ctx.full_height,
                full_height: ctx.full_height,
            }) {
                NoneHeight => NoneHeight, // collapse
                _ => WillSkip,
            }
        } else {
            match layout.measured {
                Some(measure_output) if measure_output.break_count == 0 => {
                    if measure_output.size.height.is_none() {
                        NoneHeight
                    } else {
                        WillUse
                    }
                }
                _ => self.element.first_location_usage(ctx),
            }
        }
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        if let Some(breakable) = ctx.breakable {
            let location_offset;
            let first_height;

            let layout = self.layout(ctx.width, ctx.first_height, breakable.full_height);

            if layout.pre_break {
                first_height = breakable.full_height;
                location_offset = 1;
            } else {
                first_height = ctx.first_height;
                location_offset = 0;
            }

            let size = if let Some(measure_output) = layout.measured {
                *breakable.break_count = measure_output.break_count;
                *breakable.extra_location_min_height = measure_output.extra_location_min_height;
                measure_output.size
            } else {
                self.element.measure(MeasureCtx {
                    width: ctx.width,
                    first_height,
                    breakable: Some(BreakableMeasure {
                        full_height: breakable.full_height,
                        break_count: breakable.break_count,
                        extra_location_min_height: breakable.extra_location_min_height,
                    }),
                })
            };

            *breakable.break_count += location_offset;
            size
        } else {
            self.element.measure(ctx)
        }
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        if let Some(breakable) = ctx.breakable {
            let location;
            let first_height;
            let preferred_height;
            let location_offset;

            if self
                .layout(ctx.width, ctx.first_height, breakable.full_height)
                .pre_break
            {
                location = (breakable.do_break)(ctx.pdf, 0, None);
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

            self.element.draw(DrawCtx {
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

                    do_break: &mut |pdf, location_idx, height| {
                        (breakable.do_break)(pdf, location_idx + location_offset, height)
                    },
                }),
            })
        } else {
            self.element.draw(ctx)
        }
    }
}

struct MeasureOutput {
    size: ElementSize,
    break_count: u32,
    extra_location_min_height: Option<f32>,
}

struct Layout {
    pre_break: bool,
    measured: Option<MeasureOutput>,
}

impl<E: Element> MinFirstHeight<E> {
    #[inline(always)]
    fn layout(&self, width: WidthConstraint, first_height: f32, full_height: f32) -> Layout {
        let mut measured = None;
        let pre_break = first_height < full_height && first_height < self.min_first_height && {
            let mut break_count = 0;
            let mut extra_location_min_height = None;

            let size = self.element.measure(MeasureCtx {
                width,
                first_height,
                breakable: Some(BreakableMeasure {
                    full_height,
                    break_count: &mut break_count,
                    extra_location_min_height: &mut extra_location_min_height,
                }),
            });

            if break_count > 0 {
                true
            } else {
                measured = Some(MeasureOutput {
                    size,
                    break_count,
                    extra_location_min_height,
                });
                false
            }
        };

        Layout {
            pre_break,
            measured,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        elements::{none::NoneElement, ref_element::RefElement, text::Text},
        fonts::builtin::BuiltinFont,
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
                    element: RefElement(&content),
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
                    element: RefElement(&content),
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
        use crate::test_utils::binary_snapshots::*;
        use insta::*;

        let bytes = test_element_bytes(
            TestElementParams {
                first_height: 9.,
                ..TestElementParams::breakable()
            },
            |mut callback| {
                let font = BuiltinFont::courier(callback.pdf());

                let content = Text::basic(LOREM_IPSUM, &font, 12.);
                let content = content.debug(1).show_max_width();

                callback.call(
                    &MinFirstHeight {
                        element: content,
                        min_first_height: 10.,
                    }
                    .debug(0)
                    .show_max_width()
                    .show_last_location_max_height(),
                );
            },
        );
        assert_binary_snapshot!(".pdf", bytes);
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
                    element: RefElement(&content),
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
