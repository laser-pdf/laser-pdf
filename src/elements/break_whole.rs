use crate::*;

pub struct BreakWhole<'a, E: Element>(pub &'a E);

impl<'a, E: Element> Element for BreakWhole<'a, E> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        let layout = self.layout(
            ctx.text_pieces_cache,
            ctx.width,
            ctx.first_height,
            ctx.full_height,
        );

        match layout {
            Layout::NoPreBreak => self.0.first_location_usage(ctx),
            Layout::Other {
                pre_break, size, ..
            } => {
                if pre_break {
                    FirstLocationUsage::WillSkip
                } else if size.height.is_none() {
                    FirstLocationUsage::NoneHeight
                } else {
                    // This is correct because if the element wants to skip it would have to break
                    // and in that case pre_break would be true.
                    FirstLocationUsage::WillUse
                }
            }
        }
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        if let Some(breakable) = ctx.breakable {
            let layout = self.layout(
                ctx.text_pieces_cache,
                ctx.width,
                ctx.first_height,
                breakable.full_height,
            );

            match layout {
                Layout::NoPreBreak => self.0.measure(MeasureCtx {
                    breakable: Some(breakable),
                    ..ctx
                }),
                Layout::Other {
                    pre_break,
                    break_count,
                    size,
                } => {
                    *breakable.break_count = break_count;

                    if pre_break {
                        *breakable.break_count += 1;
                    }

                    size
                }
            }
        } else {
            self.0.measure(ctx)
        }
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        if let Some(breakable) = ctx.breakable {
            let layout = self.layout(
                ctx.text_pieces_cache,
                ctx.width,
                ctx.first_height,
                breakable.full_height,
            );

            if let Layout::Other {
                pre_break: true, ..
            } = layout
            {
                let location = (breakable.do_break)(ctx.pdf, 0, None);

                self.0.draw(DrawCtx {
                    pdf: ctx.pdf,
                    text_pieces_cache: ctx.text_pieces_cache,
                    width: ctx.width,
                    location,
                    first_height: breakable.full_height,
                    preferred_height: None,
                    breakable: Some(BreakableDraw {
                        full_height: breakable.full_height,
                        preferred_height_break_count: 0,
                        do_break: &mut |pdf, location_idx, height| {
                            (breakable.do_break)(pdf, location_idx + 1, height)
                        },
                    }),
                })
            } else {
                self.0.draw(DrawCtx {
                    breakable: Some(BreakableDraw {
                        preferred_height_break_count: 0,
                        ..breakable
                    }),
                    preferred_height: None,
                    ..ctx
                })
            }
        } else {
            self.0.draw(DrawCtx {
                preferred_height: None,
                ..ctx
            })
        }
    }
}

enum Layout {
    /// This is a bit awkward, but in the case where first_height equals full height we don't want
    /// to do an unnecessary measure so we can't return the element size and break count.
    NoPreBreak,
    Other {
        pre_break: bool,
        break_count: u32,
        size: ElementSize,
    },
}

impl<'a, E: Element> BreakWhole<'a, E> {
    fn layout(
        &self,
        text_pieces_cache: &TextPiecesCache,
        width: WidthConstraint,
        first_height: f32,
        full_height: f32,
    ) -> Layout {
        if first_height == full_height {
            return Layout::NoPreBreak;
        }

        let mut break_count = 0;
        let mut extra_location_min_height = None;

        let size = self.0.measure(MeasureCtx {
            text_pieces_cache,
            width,
            first_height: full_height,
            breakable: Some(BreakableMeasure {
                full_height,
                break_count: &mut break_count,
                extra_location_min_height: &mut extra_location_min_height,
            }),
        });

        Layout::Other {
            pre_break: break_count > 0 || size.height.is_some_and(|h| h > first_height),
            break_count,
            size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{
        record_passes::{Break, BreakableDraw, DrawPass, RecordPasses},
        *,
    };

    #[test]
    fn test_unbreakable() {
        let width = WidthConstraint {
            max: 3.,
            expand: false,
        };
        let first_height = 12.;
        let pos = (2., 10.);

        let element = BuildElement(|ctx, callback| {
            let content = RecordPasses::new(FakeText {
                lines: 3,
                line_height: 5.,
                width: 3.,
            });

            let element = BreakWhole(&content);

            let ret = callback.call(element);

            content.assert_first_location_usage_count(0);

            match ctx.pass {
                build_element::Pass::FirstLocationUsage { .. } => unreachable!(),
                build_element::Pass::Measure { .. } => {
                    content.assert_measure_count(1);
                }
                build_element::Pass::Draw { .. } => {
                    content.assert_measure_count(0);
                    content.assert_draw(DrawPass {
                        width,
                        first_height,
                        preferred_height: None,
                        page: 0,
                        layer: 0,
                        pos,
                        breakable: None,
                    });
                }
            }

            ret
        });

        let output =
            test_measure_draw_compatibility(&element, width, first_height, None, pos, (1., 1.));

        output.assert_size(ElementSize {
            width: Some(3.),
            height: Some(15.),
        });
    }

    #[test]
    fn test_no_break() {
        let width = WidthConstraint {
            max: 3.,
            expand: false,
        };
        let first_height = 12.;
        let full_height = 20.;
        let pos = (2., 10.);

        let element = BuildElement(|ctx, callback| {
            let content = RecordPasses::new(FakeText {
                lines: 2,
                line_height: 5.,
                width: 3.,
            });

            let element = BreakWhole(&content);

            let ret = callback.call(element);

            content.assert_measure_count(1);
            content.assert_first_location_usage_count(0);

            match ctx.pass {
                build_element::Pass::FirstLocationUsage { .. } => unreachable!(),
                build_element::Pass::Measure { .. } => {}
                build_element::Pass::Draw { .. } => {
                    content.assert_draw(DrawPass {
                        width,
                        first_height,
                        preferred_height: None,
                        page: 0,
                        layer: 0,
                        pos,
                        breakable: Some(BreakableDraw {
                            full_height,
                            preferred_height_break_count: 0,
                            breaks: vec![],
                        }),
                    });
                }
            }

            ret
        });

        let output = test_measure_draw_compatibility(
            &element,
            width,
            first_height,
            Some(full_height),
            pos,
            (1., 1.),
        );

        output.assert_size(ElementSize {
            width: Some(3.),
            height: Some(10.),
        });
        output.breakable.unwrap().assert_break_count(0);
    }

    #[test]
    fn test_break() {
        let width = WidthConstraint {
            max: 3.,
            expand: false,
        };
        let first_height = 12.;
        let full_height = 20.;
        let pos = (2., 10.);

        let element = BuildElement(|ctx, callback| {
            let content = RecordPasses::new(FakeText {
                lines: 3,
                line_height: 5.,
                width: 3.,
            });

            let element = BreakWhole(&content);

            let ret = callback.call(element);

            content.assert_measure_count(1);
            content.assert_first_location_usage_count(0);

            match ctx.pass {
                build_element::Pass::FirstLocationUsage { .. } => unreachable!(),
                build_element::Pass::Measure { .. } => {}
                build_element::Pass::Draw { .. } => {
                    content.assert_draw(DrawPass {
                        width,
                        first_height: full_height,
                        preferred_height: None,
                        page: 1,
                        layer: 0,
                        pos,
                        breakable: Some(BreakableDraw {
                            full_height,
                            preferred_height_break_count: 0,
                            breaks: vec![],
                        }),
                    });
                }
            }

            ret
        });

        let output = test_measure_draw_compatibility(
            &element,
            width,
            first_height,
            Some(full_height),
            pos,
            (1., 1.),
        );

        output.assert_size(ElementSize {
            width: Some(3.),
            height: Some(15.),
        });
        output.breakable.unwrap().assert_break_count(1);
    }

    #[test]
    fn test_unhelpful_break() {
        let width = WidthConstraint {
            max: 3.,
            expand: false,
        };
        let first_height = 14.;
        let full_height = 14.;
        let pos = (2., 10.);

        let element = BuildElement(|ctx, callback| {
            let content = RecordPasses::new(FakeText {
                lines: 3,
                line_height: 5.,
                width: 3.,
            });

            let element = BreakWhole(&content);

            let ret = callback.call(element);

            content.assert_first_location_usage_count(0);

            match ctx.pass {
                build_element::Pass::FirstLocationUsage { .. } => unreachable!(),
                build_element::Pass::Measure { .. } => {
                    content.assert_measure_count(1);
                }
                build_element::Pass::Draw { .. } => {
                    content.assert_measure_count(0);
                    content.assert_draw(DrawPass {
                        width,
                        first_height: full_height,
                        preferred_height: None,
                        page: 0,
                        layer: 0,
                        pos,
                        breakable: Some(BreakableDraw {
                            full_height,
                            preferred_height_break_count: 0,
                            breaks: vec![Break {
                                page: 1,
                                layer: 0,
                                pos,
                            }],
                        }),
                    });
                }
            }

            ret
        });

        let output = test_measure_draw_compatibility(
            &element,
            width,
            first_height,
            Some(full_height),
            pos,
            (1., 1.),
        );

        output.assert_size(ElementSize {
            width: Some(3.),
            height: Some(5.),
        });
        output.breakable.unwrap().assert_break_count(1);
    }
}
