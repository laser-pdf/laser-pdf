use crate::{
    utils::{add_optional_size_with_gap, max_optional_size},
    *,
};

pub struct RepeatAfterBreak<'a, T: Element, C: Element> {
    pub title: &'a T,
    pub content: &'a C,
    pub gap: f64,
    pub collapse_on_empty_content: bool,
}

impl<'a, T: Element, C: Element> Element for RepeatAfterBreak<'a, T, C> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        let title_size = self.title.measure(MeasureCtx {
            width: ctx.width,
            first_height: ctx.full_height,
            breakable: None,
        });

        let collapse = self.collapse_on_empty_content || title_size.height.is_none();

        if !collapse && ctx.first_height == ctx.full_height {
            return FirstLocationUsage::WillUse;
        }

        let y_offset = self.y_offset(title_size);
        let first_location_usage = self.content.first_location_usage(FirstLocationUsageCtx {
            width: ctx.width,
            first_height: ctx.first_height - y_offset,
            full_height: ctx.full_height,
        });

        if collapse && first_location_usage == FirstLocationUsage::NoneHeight {
            FirstLocationUsage::NoneHeight
        } else if ctx.first_height < ctx.full_height
            && (y_offset > ctx.first_height || first_location_usage == FirstLocationUsage::WillSkip)
        {
            FirstLocationUsage::WillSkip
        } else {
            FirstLocationUsage::WillUse
        }
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        let title_size = self.title.measure(MeasureCtx {
            width: ctx.width,
            first_height: ctx
                .breakable
                .as_ref()
                .map(|b| b.full_height)
                .unwrap_or(ctx.first_height),
            breakable: None,
        });
        let y_offset = self.y_offset(title_size);

        let mut break_count = 0;

        let content_size;

        if let Some(breakable) = ctx.breakable {
            let first_height;
            let full_height = breakable.full_height - y_offset;

            if ctx.first_height < breakable.full_height
                && (y_offset > ctx.first_height || {
                    let first_location_usage =
                        self.content.first_location_usage(FirstLocationUsageCtx {
                            width: ctx.width,
                            first_height: ctx.first_height - y_offset,
                            full_height: breakable.full_height,
                        });

                    first_location_usage == FirstLocationUsage::WillSkip
                })
            {
                first_height = full_height;
                *breakable.break_count = 1;
            } else {
                first_height = ctx.first_height - y_offset;
            }

            content_size = self.content.measure(MeasureCtx {
                width: ctx.width,
                first_height,
                breakable: Some(BreakableMeasure {
                    full_height,
                    break_count: &mut break_count,
                    extra_location_min_height: breakable.extra_location_min_height,
                }),
            });

            *breakable.break_count += break_count;
        } else {
            content_size = self.content.measure(MeasureCtx {
                width: ctx.width,
                first_height: ctx.first_height - y_offset,
                breakable: None,
            });
        };

        self.size(
            title_size,
            content_size,
            self.collapse(break_count, content_size),
        )
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        let title_first_height = ctx
            .breakable
            .as_ref()
            .map(|b| b.full_height)
            .unwrap_or(ctx.first_height);
        let title_size = self.title.measure(MeasureCtx {
            width: ctx.width,
            first_height: title_first_height,
            breakable: None,
        });
        let y_offset = self.y_offset(title_size);

        let content_size;
        let location;
        let mut last_location_idx = 0;

        if let Some(breakable) = ctx.breakable {
            let first_height;
            let location_offset;
            let full_height = breakable.full_height - y_offset;

            if ctx.first_height < breakable.full_height
                && (y_offset > ctx.first_height || {
                    let first_location_usage =
                        self.content.first_location_usage(FirstLocationUsageCtx {
                            width: ctx.width,
                            first_height: ctx.first_height - y_offset,
                            full_height: breakable.full_height,
                        });

                    first_location_usage == FirstLocationUsage::WillSkip
                })
            {
                first_height = full_height;
                location = (breakable.do_break)(ctx.pdf, 0, None);
                location_offset = 1;
            } else {
                first_height = ctx.first_height - y_offset;
                location = ctx.location;
                location_offset = 0;
            }

            content_size = self.content.draw(DrawCtx {
                pdf: ctx.pdf,
                location: Location {
                    layer: location.layer.clone(),
                    pos: (location.pos.0, location.pos.1 - y_offset),
                    ..location
                },
                width: ctx.width,
                first_height,
                preferred_height: None,
                breakable: Some(BreakableDraw {
                    full_height,
                    preferred_height_break_count: 0,

                    do_break: &mut |pdf, location_idx, height| {
                        let mut new_location = (breakable.do_break)(
                            pdf,
                            location_idx + location_offset,
                            add_optional_size_with_gap(height, title_size.height, self.gap),
                        );

                        if last_location_idx <= location_idx {
                            for i in last_location_idx + 1..=location_idx {
                                let location =
                                    (breakable.do_break)(pdf, i + location_offset - 1, None);

                                self.title.draw(DrawCtx {
                                    pdf,
                                    location,
                                    width: ctx.width,
                                    first_height: title_first_height,
                                    preferred_height: None,
                                    breakable: None,
                                });
                            }

                            self.title.draw(DrawCtx {
                                pdf,
                                location: new_location.clone(),
                                width: ctx.width,
                                first_height: title_first_height,
                                preferred_height: None,
                                breakable: None,
                            });

                            last_location_idx = location_idx + 1;
                        }

                        new_location.pos.1 -= y_offset;
                        new_location
                    },
                }),
            });
        } else {
            location = ctx.location;
            content_size = self.content.draw(DrawCtx {
                pdf: ctx.pdf,
                location: Location {
                    layer: location.layer.clone(),
                    pos: (location.pos.0, location.pos.1 - y_offset),
                    ..location
                },
                width: ctx.width,
                first_height: ctx.first_height - y_offset,
                preferred_height: None,
                breakable: None,
            });
        };

        let collapse = self.collapse(last_location_idx, content_size);

        // if there were any breaks the title was drawn there
        if !collapse {
            self.title.draw(DrawCtx {
                pdf: ctx.pdf,
                location: location.clone(),
                width: ctx.width,
                first_height: title_first_height,
                preferred_height: None,
                breakable: None,
            });
        }

        self.size(title_size, content_size, collapse)
    }
}

impl<'a, T: Element, C: Element> RepeatAfterBreak<'a, T, C> {
    fn y_offset(&self, title_size: ElementSize) -> f64 {
        title_size.height.map(|h| h + self.gap).unwrap_or(0.)
    }

    fn collapse(&self, break_count: u32, content_size: ElementSize) -> bool {
        self.collapse_on_empty_content && break_count == 0 && content_size.height.is_none()
    }

    fn size(
        &self,
        title_size: ElementSize,
        content_size: ElementSize,
        collapse: bool,
    ) -> ElementSize {
        ElementSize {
            width: if collapse {
                content_size.width
            } else {
                max_optional_size(title_size.width, content_size.width)
            },
            height: if collapse {
                None
            } else {
                add_optional_size_with_gap(title_size.height, content_size.height, self.gap)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        elements::{force_break::ForceBreak, none::NoneElement, rectangle::Rectangle},
        test_utils::{
            build_element::BuildElementCtx,
            record_passes::{Break, DrawPass, RecordPasses},
            *,
        },
    };

    #[test]
    fn test_collapse() {
        for configuration in (ElementTestParams {
            first_height: 5.,
            width: 10.,
            full_height: 10.,
            pos: (1., 10.),
            ..Default::default()
        })
        .configurations()
        {
            let element = RepeatAfterBreak {
                gap: 1.,
                collapse_on_empty_content: true,
                title: &Rectangle {
                    size: (1., 2.),
                    fill: None,
                    outline: None,
                },
                content: &NoneElement,
            };

            let output = configuration.run(&element);
            output.assert_no_breaks().assert_size(ElementSize {
                width: None,
                height: None,
            });
        }
    }

    #[test]
    fn test_pull_down() {
        let gap = 1.;

        for configuration in (ElementTestParams {
            first_height: 5.,
            width: 10.,
            full_height: 10.,
            pos: (1., 10.),
            ..Default::default()
        })
        .configurations()
        {
            let element = BuildElement(|BuildElementCtx { pass, .. }, callback| {
                let title = RecordPasses::new(Rectangle {
                    size: (2.5, 2.),
                    fill: None,
                    outline: None,
                });

                let content = RecordPasses::new(Rectangle {
                    size: (2., 3.),
                    fill: None,
                    outline: None,
                });

                let ret = callback.call(RepeatAfterBreak {
                    gap,
                    title: &title,
                    content: &content,
                    collapse_on_empty_content: false,
                });

                title.assert_measure_count(1);
                title.assert_first_location_usage_count(0);

                content.assert_first_location_usage_count(
                    if configuration.breakable && configuration.use_first_height {
                        1
                    } else {
                        0
                    },
                );

                match pass {
                    build_element::Pass::FirstLocationUsage { .. } => todo!(),
                    build_element::Pass::Measure { .. } => {
                        title.assert_draw_count(0);
                        content.assert_draw_count(0);
                        content.assert_measure_count(1);
                    }
                    build_element::Pass::Draw { .. } => {
                        let width = WidthConstraint {
                            max: 10.,
                            expand: configuration.expand_width,
                        };

                        let first_height = if configuration.use_first_height {
                            5.
                        } else {
                            10.
                        };

                        title.assert_draw(DrawPass {
                            width,
                            first_height: if configuration.breakable {
                                10.
                            } else {
                                first_height
                            },
                            preferred_height: None,
                            page: if configuration.breakable && configuration.use_first_height {
                                1
                            } else {
                                0
                            },
                            layer: 0,
                            pos: (1., 10.),
                            breakable: None,
                        });

                        content.assert_draw(DrawPass {
                            width,
                            first_height: if configuration.breakable {
                                7.
                            } else {
                                first_height - 3.
                            },
                            preferred_height: None,
                            page: if configuration.breakable && configuration.use_first_height {
                                1
                            } else {
                                0
                            },
                            layer: 0,
                            pos: (1., 7.),
                            breakable: if configuration.breakable {
                                Some(record_passes::BreakableDraw {
                                    full_height: 7.,
                                    preferred_height_break_count: 0,
                                    breaks: vec![],
                                })
                            } else {
                                None
                            },
                        });
                        content.assert_measure_count(0);
                    }
                }

                ret
            });

            let output = configuration.run(&element);

            output.assert_size(ElementSize {
                width: Some(2.5),
                height: Some(6.),
            });

            if let Some(b) = output.breakable {
                if configuration.use_first_height {
                    b.assert_break_count(1);
                } else {
                    b.assert_break_count(0);
                }
            }
        }
    }

    #[test]
    fn test_title_overflow() {
        let gap = 1.;

        for configuration in (ElementTestParams {
            first_height: 2.,
            width: 10.,
            full_height: 10.,
            pos: (1., 10.),
            ..Default::default()
        })
        .configurations()
        {
            let element = BuildElement(|BuildElementCtx { pass, .. }, callback| {
                let title = RecordPasses::new(Rectangle {
                    size: (2.5, 3.),
                    fill: None,
                    outline: None,
                });

                let content = RecordPasses::new(ForceBreak);

                let ret = callback.call(RepeatAfterBreak {
                    gap,
                    title: &title,
                    content: &content,
                    collapse_on_empty_content: false,
                });

                title.assert_measure_count(1);
                title.assert_first_location_usage_count(0);

                content.assert_first_location_usage_count(0);

                match pass {
                    build_element::Pass::FirstLocationUsage { .. } => todo!(),
                    build_element::Pass::Measure { .. } => {
                        title.assert_draw_count(0);
                        content.assert_draw_count(0);
                        content.assert_measure_count(1);
                    }
                    build_element::Pass::Draw { .. } => {
                        let width = WidthConstraint {
                            max: 10.,
                            expand: configuration.expand_width,
                        };

                        let first_height = if configuration.use_first_height {
                            2.
                        } else {
                            10.
                        };

                        if configuration.breakable {
                            title.assert_draws(&[
                                DrawPass {
                                    width,
                                    first_height: 10.,
                                    preferred_height: None,
                                    page: if configuration.use_first_height { 2 } else { 1 },
                                    layer: 0,
                                    pos: (1., 10.),
                                    breakable: None,
                                },
                                DrawPass {
                                    width,
                                    first_height: 10.,
                                    preferred_height: None,
                                    page: if configuration.use_first_height { 1 } else { 0 },
                                    layer: 0,
                                    pos: (1., 10.),
                                    breakable: None,
                                },
                            ]);
                        } else {
                            title.assert_draw(DrawPass {
                                width,
                                first_height,
                                preferred_height: None,
                                page: 0,
                                layer: 0,
                                pos: (1., 10.),
                                breakable: None,
                            });
                        }

                        content.assert_draw(DrawPass {
                            width,
                            first_height: if configuration.breakable {
                                6.
                            } else {
                                first_height - 4.
                            },
                            preferred_height: None,
                            page: if configuration.breakable && configuration.use_first_height {
                                1
                            } else {
                                0
                            },
                            layer: 0,
                            pos: (1., 6.),
                            breakable: if configuration.breakable {
                                Some(record_passes::BreakableDraw {
                                    full_height: 6.,
                                    preferred_height_break_count: 0,
                                    breaks: vec![Break {
                                        page: if configuration.use_first_height { 2 } else { 1 },
                                        layer: 0,
                                        pos: (1., 6.),
                                    }],
                                })
                            } else {
                                None
                            },
                        });
                        content.assert_measure_count(0);
                    }
                }

                ret
            });

            let output = configuration.run(&element);

            output.assert_size(ElementSize {
                width: Some(2.5),
                height: Some(3.),
            });

            if let Some(b) = output.breakable {
                if configuration.use_first_height {
                    b.assert_break_count(2);
                } else {
                    b.assert_break_count(1);
                }
            }
        }
    }

    #[test]
    fn test_unhelpful_breaks() {
        let gap = 1.;

        for configuration in (ElementTestParams {
            first_height: 5.,
            width: 10.,
            full_height: 10.,
            pos: (1., 10.),
            ..Default::default()
        })
        .configurations()
        {
            let element = BuildElement(|BuildElementCtx { pass, .. }, callback| {
                let title = RecordPasses::new(Rectangle {
                    size: (2.5, 5.),
                    fill: None,
                    outline: None,
                });

                let content = RecordPasses::new(Rectangle {
                    size: (4., 10.),
                    fill: None,
                    outline: None,
                });

                let ret = callback.call(RepeatAfterBreak {
                    gap,
                    title: &title,
                    content: &content,
                    collapse_on_empty_content: false,
                });

                title.assert_measure_count(1);
                title.assert_first_location_usage_count(0);

                content.assert_first_location_usage_count(0);

                match pass {
                    build_element::Pass::FirstLocationUsage { .. } => todo!(),
                    build_element::Pass::Measure { .. } => {
                        title.assert_draw_count(0);
                        content.assert_draw_count(0);
                        content.assert_measure_count(1);
                    }
                    build_element::Pass::Draw { .. } => {
                        let width = WidthConstraint {
                            max: 10.,
                            expand: configuration.expand_width,
                        };

                        let first_height = if configuration.use_first_height {
                            5.
                        } else {
                            10.
                        };

                        title.assert_draw(DrawPass {
                            width,
                            first_height: if configuration.breakable {
                                10.
                            } else {
                                first_height
                            },
                            preferred_height: None,
                            page: if configuration.breakable && configuration.use_first_height {
                                1
                            } else {
                                0
                            },
                            layer: 0,
                            pos: (1., 10.),
                            breakable: None,
                        });

                        content.assert_draw(DrawPass {
                            width,
                            first_height: if configuration.breakable {
                                4.
                            } else {
                                first_height - 6.
                            },
                            preferred_height: None,

                            // if the first height is equal to the full height a break won't
                            // accomplish but if the first height is less we always break if
                            // first_location_usage is WillSkip because otherwise we'd have to
                            // call first_location_usage twice
                            page: if configuration.breakable && configuration.use_first_height {
                                1
                            } else {
                                0
                            },

                            layer: 0,
                            pos: (1., 4.),
                            breakable: if configuration.breakable {
                                Some(record_passes::BreakableDraw {
                                    full_height: 4.,
                                    preferred_height_break_count: 0,
                                    breaks: vec![],
                                })
                            } else {
                                None
                            },
                        });
                        content.assert_measure_count(0);
                    }
                }

                ret
            });

            let output = configuration.run(&element);

            output.assert_size(ElementSize {
                width: Some(4.),
                height: Some(16.),
            });

            if let Some(b) = output.breakable {
                if configuration.use_first_height {
                    b.assert_break_count(1);
                } else {
                    b.assert_break_count(0);
                }
            }
        }
    }

    #[test]
    fn test_skipped_locations() {
        let gap = 0.;

        let element = BuildElement(|BuildElementCtx { pass, .. }, callback| {
            let title = RecordPasses::new(Rectangle {
                size: (2.5, 5.),
                fill: None,
                outline: None,
            });

            let content = RecordPasses::new(FranticJumper {
                jumps: vec![(0, Some(0.)), (1, Some(11.)), (4, Some(11.))],
                size: ElementSize {
                    width: None,
                    height: Some(11.),
                },
            });

            let ret = callback.call(RepeatAfterBreak {
                gap,
                title: &title,
                content: &content,
                collapse_on_empty_content: false,
            });

            title.assert_measure_count(1);
            title.assert_first_location_usage_count(0);

            content.assert_first_location_usage_count(1);

            match pass {
                build_element::Pass::FirstLocationUsage { .. } => todo!(),
                build_element::Pass::Measure { .. } => {
                    title.assert_draw_count(0);
                    content.assert_draw_count(0);
                    content.assert_measure_count(1);
                }
                build_element::Pass::Draw { .. } => {
                    let width = WidthConstraint {
                        max: 10.,
                        expand: false,
                    };

                    let mut draws = (0..=5)
                        .map(|i| DrawPass {
                            width,
                            first_height: 12.,
                            preferred_height: None,
                            page: i,
                            layer: 0,
                            pos: (1., 20.),
                            breakable: None,
                        })
                        .collect::<Vec<_>>();
                    draws.rotate_left(1);
                    title.assert_draws(&draws);

                    content.assert_draw(DrawPass {
                        width,
                        first_height: 3.,
                        preferred_height: None,

                        page: 0,

                        layer: 0,
                        pos: (1., 15.),
                        breakable: Some(record_passes::BreakableDraw {
                            full_height: 7.,
                            preferred_height_break_count: 0,
                            breaks: [1, 2, 5]
                                .into_iter()
                                .map(|i| record_passes::Break {
                                    page: i,
                                    layer: 0,
                                    pos: (1., 15.),
                                })
                                .collect::<Vec<_>>(),
                        }),
                    });
                    content.assert_measure_count(0);
                }
            }

            ret
        });

        let output = test_measure_draw_compatibility(
            &element,
            WidthConstraint {
                max: 10.,
                expand: false,
            },
            8.,
            Some(12.),
            (1., 20.),
            (400., 400.),
        );

        output.assert_size(ElementSize {
            width: Some(2.5),
            height: Some(16.),
        });
        output.breakable.unwrap().assert_break_count(5);
    }
}
