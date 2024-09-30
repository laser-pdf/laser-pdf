use crate::{utils::max_optional_size, *};

pub struct Stack<C: Fn(&mut StackContent)> {
    pub content: C,
    pub expand: bool,
}

impl<C: Fn(&mut StackContent)> Element for Stack<C> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        let mut ret = FirstLocationUsage::NoneHeight;

        let mut content = StackContent(Pass::FirstLocationUsage { ctx, ret: &mut ret });

        (self.content)(&mut content);

        ret
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        let mut size = ElementSize {
            width: None,
            height: None,
        };

        let mut content = StackContent(Pass::Measure {
            ctx,
            size: &mut size,
        });

        (self.content)(&mut content);

        size
    }

    fn draw(&self, mut ctx: DrawCtx) -> ElementSize {
        let mut size = ElementSize {
            width: None,
            height: None,
        };

        if self.expand {
            let mut break_count = 0;
            let mut extra_location_min_height = None;
            let mut size = ElementSize {
                width: None,
                height: None,
            };

            let mut content = StackContent(Pass::Measure {
                ctx: MeasureCtx {
                    width: ctx.width,
                    first_height: ctx.first_height,
                    breakable: ctx.breakable.as_ref().map(|b| BreakableMeasure {
                        full_height: b.full_height,
                        break_count: &mut break_count,
                        extra_location_min_height: &mut extra_location_min_height,
                    }),
                },
                size: &mut size,
            });

            (self.content)(&mut content);

            if let Some(ref mut breakable) = ctx.breakable {
                match break_count.cmp(&breakable.preferred_height_break_count) {
                    std::cmp::Ordering::Less => (),
                    std::cmp::Ordering::Equal => {
                        ctx.preferred_height = max_optional_size(ctx.preferred_height, size.height);
                    }
                    std::cmp::Ordering::Greater => {
                        breakable.preferred_height_break_count = break_count;
                        ctx.preferred_height = size.height;
                    }
                }
            } else {
                ctx.preferred_height = max_optional_size(ctx.preferred_height, size.height);
            }
        } else {
            ctx.preferred_height = None;

            if let Some(ref mut breakable) = ctx.breakable {
                breakable.preferred_height_break_count = 0;
            }
        }

        let mut content = StackContent(Pass::Draw {
            ctx,
            size: &mut size,
            max_break_count: 0,
        });

        (self.content)(&mut content);

        size
    }
}

pub struct StackContent<'pdf, 'a, 'r>(Pass<'pdf, 'a, 'r>);

enum Pass<'pdf, 'a, 'r> {
    FirstLocationUsage {
        ctx: FirstLocationUsageCtx,
        ret: &'r mut FirstLocationUsage,
    },
    Measure {
        ctx: MeasureCtx<'a>,
        size: &'r mut ElementSize,
    },
    Draw {
        ctx: DrawCtx<'pdf, 'a>,
        size: &'r mut ElementSize,
        max_break_count: u32,
    },
}

impl<'pdf, 'a, 'r> StackContent<'pdf, 'a, 'r> {
    pub fn add(&mut self, element: &impl Element) {
        match self.0 {
            Pass::FirstLocationUsage {
                ref mut ctx,
                ret: &mut ref mut ret,
            } => {
                use FirstLocationUsage::*;

                let first_location_usage =
                    element.first_location_usage(FirstLocationUsageCtx { ..*ctx });

                match ret {
                    WillUse => {}
                    NoneHeight => {
                        *ret = first_location_usage;
                    }
                    WillSkip => {
                        if first_location_usage == WillUse {
                            *ret = WillUse;
                        }
                    }
                }
            }
            Pass::Measure {
                ref mut ctx,
                size: &mut ref mut size,
            } => {
                let mut break_count = 0;
                let mut extra_location_min_height = None;

                let element_size = element.measure(MeasureCtx {
                    breakable: ctx.breakable.as_mut().map(|b| BreakableMeasure {
                        full_height: b.full_height,
                        break_count: &mut break_count,
                        extra_location_min_height: &mut extra_location_min_height,
                    }),
                    ..*ctx
                });

                size.width = max_optional_size(size.width, element_size.width);

                if let Some(ref mut breakable) = ctx.breakable {
                    size.height = match break_count.cmp(breakable.break_count) {
                        std::cmp::Ordering::Less => size.height,
                        std::cmp::Ordering::Equal => {
                            max_optional_size(size.height, element_size.height)
                        }
                        std::cmp::Ordering::Greater => {
                            *breakable.break_count = break_count;
                            element_size.height
                        }
                    }
                } else {
                    size.height = max_optional_size(size.height, element_size.height);
                }
            }
            Pass::Draw {
                ref mut ctx,
                size: &mut ref mut size,
                ref mut max_break_count,
            } => {
                let mut break_count = 0;

                let element_size = element.draw(DrawCtx {
                    pdf: ctx.pdf,
                    location: ctx.location.clone(),
                    breakable: ctx
                        .breakable
                        .as_mut()
                        .map(|b| {
                            (
                                b.full_height,
                                b.preferred_height_break_count,
                                |pdf: &mut Pdf, location_idx: u32, _| {
                                    break_count = break_count.max(location_idx + 1);
                                    (b.do_break)(
                                        pdf,
                                        location_idx,
                                        Some(if location_idx == 0 {
                                            ctx.first_height
                                        } else {
                                            b.full_height
                                        }),
                                    )
                                },
                            )
                        })
                        .as_mut()
                        .map(
                            |&mut (
                                full_height,
                                preferred_height_break_count,
                                ref mut get_location,
                            )| {
                                BreakableDraw {
                                    full_height,
                                    preferred_height_break_count,
                                    do_break: get_location,
                                }
                            },
                        ),
                    ..*ctx
                });

                size.width = max_optional_size(size.width, element_size.width);

                if ctx.breakable.is_some() {
                    size.height = match break_count.cmp(max_break_count) {
                        std::cmp::Ordering::Less => size.height,
                        std::cmp::Ordering::Equal => {
                            max_optional_size(size.height, element_size.height)
                        }
                        std::cmp::Ordering::Greater => {
                            *max_break_count = break_count;
                            element_size.height
                        }
                    }
                } else {
                    size.height = max_optional_size(size.height, element_size.height);
                }
            }
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
    fn test() {
        let width = WidthConstraint {
            max: 3.,
            expand: false,
        };
        let first_height = 12.;
        let full_height = 20.;
        let pos = (2., 10.);

        let output = test_element(
            TestElementParams {
                width,
                first_height,
                breakable: Some(TestElementParamsBreakable {
                    full_height,
                    ..Default::default()
                }),
                pos,
                ..Default::default()
            },
            |assert, callback| {
                let a = RecordPasses::new(FakeText {
                    lines: 3,
                    line_height: 5.,
                    width: 2.5,
                });

                let b = RecordPasses::new(FakeText {
                    lines: 40,
                    line_height: 1.,
                    width: 2.,
                });

                let c = RecordPasses::new(FakeText {
                    lines: 3,
                    line_height: 3.9,
                    width: 2.9,
                });

                let element = Stack {
                    content: |content| {
                        content.add(&a);
                        content.add(&b);
                        content.add(&c);
                    },
                    expand: false,
                };

                let ret = callback.call(element);

                if assert {
                    a.assert_first_location_usage_count(0);
                    b.assert_first_location_usage_count(0);
                    c.assert_first_location_usage_count(0);
                    a.assert_measure_count(0);
                    b.assert_measure_count(0);
                    c.assert_measure_count(0);

                    let draw_pass = DrawPass {
                        width,
                        first_height,
                        preferred_height: None,
                        page: 0,
                        layer: 0,
                        pos: (2., 2.),
                        breakable: None,
                    };

                    a.assert_draw(DrawPass {
                        breakable: Some(BreakableDraw {
                            full_height,
                            preferred_height_break_count: 0,
                            breaks: vec![Break {
                                page: 1,
                                layer: 0,
                                pos,
                            }],
                        }),
                        ..draw_pass
                    });

                    b.assert_draw(DrawPass {
                        breakable: Some(BreakableDraw {
                            full_height,
                            preferred_height_break_count: 0,
                            breaks: vec![
                                Break {
                                    page: 1,
                                    layer: 0,
                                    pos,
                                },
                                Break {
                                    page: 2,
                                    layer: 0,
                                    pos,
                                },
                            ],
                        }),
                        ..draw_pass
                    });

                    c.assert_draw(DrawPass {
                        breakable: Some(BreakableDraw {
                            full_height,
                            preferred_height_break_count: 0,
                            breaks: vec![],
                        }),
                        ..draw_pass
                    });
                }

                ret
            },
        );

        output.assert_size(ElementSize {
            width: Some(2.9),
            height: Some(8.),
        });

        output
            .breakable
            .unwrap()
            .assert_break_count(2)
            .assert_extra_location_min_height(None)
            .assert_first_location_usage(FirstLocationUsage::WillUse);
    }
}
