use crate::*;

pub struct Column<C: Fn(ColumnContent) -> Option<()>> {
    pub content: C,
    pub gap: f64,
}

impl<C: Fn(ColumnContent) -> Option<()>> Element for Column<C> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        let mut ret = FirstLocationUsage::NoneHeight;

        (self.content)(ColumnContent {
            pass: Pass::InsufficientFirstHeight { ctx, ret: &mut ret },
            gap: self.gap,
        });

        ret
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        let mut width = None;
        let mut height = None;

        (self.content)(ColumnContent {
            pass: Pass::Measure {
                width_constraint: ctx.width,
                breakable: ctx.breakable,
                height_available: ctx.first_height,
                width: &mut width,
                height: &mut height,
            },
            gap: self.gap,
        });

        ElementSize { width, height }
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        let mut width = None;
        let mut height = None;

        (self.content)(ColumnContent {
            pass: Pass::Draw {
                pdf: ctx.pdf,
                location: ctx.location,
                location_offset: 0,
                width_constraint: ctx.width,
                breakable: ctx.breakable,
                height_available: ctx.first_height,
                width: &mut width,
                height: &mut height,
            },
            gap: self.gap,
        });

        ElementSize { width, height }
    }
}

pub struct ColumnContent<'a, 'b, 'r> {
    pass: Pass<'a, 'b, 'r>,
    gap: f64,
}

enum Pass<'a, 'b, 'r> {
    InsufficientFirstHeight {
        ctx: FirstLocationUsageCtx,
        ret: &'r mut FirstLocationUsage,
    },
    Measure {
        width_constraint: WidthConstraint,
        breakable: Option<BreakableMeasure<'a>>,

        /// this is initially first_height and when breaking we set it to full height
        height_available: f64,
        width: &'r mut Option<f64>,
        height: &'r mut Option<f64>,
    },
    Draw {
        pdf: &'a mut Pdf,
        location: Location,
        location_offset: u32,
        width_constraint: WidthConstraint,
        breakable: Option<BreakableDraw<'b>>,

        /// this is initially first_height and when breaking we set it to full height
        height_available: f64,
        width: &'r mut Option<f64>,
        height: &'r mut Option<f64>,
    },
}

impl<'a, 'b, 'r> ColumnContent<'a, 'b, 'r> {
    pub fn add<E: Element>(mut self, element: &E) -> Option<Self> {
        match self.pass {
            Pass::InsufficientFirstHeight {
                ref mut ctx,
                ret: &mut ref mut ret,
            } => {
                let first_location_usage =
                    element.first_location_usage(FirstLocationUsageCtx { ..*ctx });

                if first_location_usage == FirstLocationUsage::NoneHeight {
                    Some(self)
                } else {
                    *ret = first_location_usage;
                    None
                }
            }
            Pass::Measure {
                width_constraint,
                ref mut breakable,
                ref mut height_available,
                width: &mut ref mut width,
                height: &mut ref mut height,
            } => {
                // The gap is applied here, but will only be actually applied to the height and
                // position for subsequent elements if this element ends up having a height.
                let measure_ctx = MeasureCtx {
                    width: width_constraint,
                    first_height: *height_available
                        - height.unwrap_or(0.)
                        - if height.is_some() { self.gap } else { 0. },
                    breakable: None,
                };

                let size;

                if let Some(b) = breakable {
                    let mut break_count = 0;

                    // We ignore this because we also don't pass on preferred height.
                    let mut extra_location_min_height = 0.;

                    size = element.measure(MeasureCtx {
                        breakable: Some(BreakableMeasure {
                            full_height: b.full_height,
                            break_count: &mut break_count,
                            extra_location_min_height: &mut extra_location_min_height,
                        }),
                        ..measure_ctx
                    });

                    if break_count > 0 {
                        *height_available = b.full_height;
                        *height = None;
                        *b.break_count += break_count;
                    }
                } else {
                    size = element.measure(measure_ctx);
                }

                if let Some(h) = size.height {
                    if let Some(height) = height {
                        *height += self.gap;
                        *height += h;
                    } else {
                        *height = Some(h);
                    }
                }

                if let Some(w) = size.width {
                    if let Some(width) = width {
                        *width = width.max(w);
                    } else {
                        *width = Some(w);
                    }
                }

                Some(self)
            }
            Pass::Draw {
                pdf: &mut ref mut pdf,
                ref mut location,
                ref mut location_offset,
                width_constraint,
                ref mut breakable,
                ref mut height_available,
                width: &mut ref mut width,
                height: &mut ref mut height,
            } => {
                // The gap is applied here, but will only be actually applied to the height and
                // position for subsequent elements if this element ends up having a height.
                let draw_ctx = DrawCtx {
                    pdf,
                    location: Location {
                        layer: location.layer.clone(),
                        pos: if height.is_some() {
                            (location.pos.0, location.pos.1 - self.gap)
                        } else {
                            location.pos
                        },
                        ..*location
                    },
                    width: width_constraint,
                    first_height: *height_available
                        - height.unwrap_or(0.)
                        - if height.is_some() { self.gap } else { 0. },
                    preferred_height: 0.,
                    breakable: None,
                };

                let size = if let Some(b) = breakable {
                    let mut break_count = 0;

                    let size = element.draw(DrawCtx {
                        breakable: Some(BreakableDraw {
                            full_height: b.full_height,
                            preferred_height_break_count: 0,
                            get_location: &mut |pdf, location_idx| {
                                *height_available = b.full_height;

                                let new_location =
                                    (b.get_location)(pdf, location_idx + *location_offset);

                                if location_idx + 1 > break_count {
                                    break_count = location_idx + 1;
                                    *location = new_location.clone();
                                }

                                new_location
                            },
                        }),
                        ..draw_ctx
                    });

                    if break_count > 0 {
                        *location_offset += break_count;
                        *height_available = b.full_height;
                        *height = None;
                    }

                    size
                } else {
                    element.draw(draw_ctx)
                };

                if let Some(h) = size.height {
                    if let Some(height) = height {
                        location.pos.1 -= self.gap;
                        *height += self.gap;

                        *height += h;
                    } else {
                        *height = Some(h);
                    }

                    location.pos.1 -= h;
                }

                if let Some(w) = size.width {
                    if let Some(width) = width {
                        *width = width.max(w);
                    } else {
                        *width = Some(w);
                    }
                }

                Some(self)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{elements::force_break::ForceBreak, elements::none::NoneElement, test_utils::*};

    #[test]
    fn test_column_empty() {
        let element = Column {
            gap: 100.,
            content: |_| Some(()),
        };

        for output in ElementTestParams::default().run(&element) {
            output.assert_size(ElementSize {
                width: None,
                height: None,
            });

            if let Some(b) = output.breakable {
                b.assert_break_count(0).assert_extra_location_min_height(0.);
            }
        }
    }

    #[test]
    fn test_column_with_multiple_nones() {
        use assert_passes::*;

        let element = BuildElement(|build_ctx, callback| {
            // we need to build this multiple times because AssertPasses keeps internal state
            let build = || {
                AssertPasses::new(
                    NoneElement,
                    match build_ctx.pass {
                        build_element::Pass::FirstLocationUsage => vec![Pass::FirstLocationUsage {
                            width: build_ctx.width,
                            first_height: build_ctx.first_height,
                            full_height: build_ctx.full_height.unwrap(),
                        }],
                        build_element::Pass::Measure => vec![Pass::Measure {
                            width: build_ctx.width,
                            first_height: build_ctx.first_height,
                            full_height: build_ctx.full_height,
                        }],
                        build_element::Pass::Draw => vec![Pass::Draw {
                            width: build_ctx.width,
                            first_height: build_ctx.first_height,
                            preferred_height: 0.,

                            page: 0,
                            layer: 0,
                            pos: (3., 12.),

                            breakable: build_ctx.full_height.map(|full_height| BreakableDraw {
                                full_height,
                                preferred_height_break_count: 0,
                                breaks: Vec::new(),
                            }),
                        }],
                    },
                )
            };

            let none_0 = build();
            let none_1 = build();
            let none_2 = build();

            let element = Column {
                gap: 1.,
                content: |content| {
                    content.add(&none_0)?.add(&none_1)?.add(&none_2)?;

                    Some(())
                },
            };

            callback.call(element)
        });

        for output in (ElementTestParams {
            first_height: 4.,
            full_height: 10.,
            width: 6.,
            pos: (3., 12.),
            ..Default::default()
        })
        .run(&element)
        {
            output.assert_size(ElementSize {
                width: None,
                height: None,
            });

            if let Some(b) = output.breakable {
                b.assert_break_count(0).assert_extra_location_min_height(0.);
            }
        }
    }

    #[test]
    fn test_column() {
        use assert_passes::*;

        let element = BuildElement(|build_ctx, callback| {
            let less_first_height = build_ctx.first_height == 4.;

            let child_0 = AssertPasses::new(
                NoneElement,
                match build_ctx.pass {
                    build_element::Pass::FirstLocationUsage => vec![Pass::FirstLocationUsage {
                        width: build_ctx.width,
                        first_height: build_ctx.first_height,
                        full_height: build_ctx.full_height.unwrap(),
                    }],
                    build_element::Pass::Measure => vec![Pass::Measure {
                        width: build_ctx.width,
                        first_height: build_ctx.first_height,
                        full_height: build_ctx.full_height,
                    }],
                    build_element::Pass::Draw => vec![Pass::Draw {
                        width: build_ctx.width,
                        first_height: build_ctx.first_height,
                        preferred_height: 0.,

                        page: 0,
                        layer: 0,
                        pos: (3., 12.),

                        breakable: build_ctx.full_height.map(|full_height| BreakableDraw {
                            full_height,
                            preferred_height_break_count: 0,
                            breaks: Vec::new(),
                        }),
                    }],
                },
            );

            let child_1 = AssertPasses::new(
                FakeText {
                    lines: 8,
                    line_height: 2.,
                    width: 5.,
                },
                match build_ctx.pass {
                    build_element::Pass::FirstLocationUsage => vec![Pass::FirstLocationUsage {
                        width: build_ctx.width,
                        first_height: build_ctx.first_height,
                        full_height: build_ctx.full_height.unwrap(),
                    }],
                    build_element::Pass::Measure => vec![Pass::Measure {
                        width: build_ctx.width,
                        first_height: build_ctx.first_height,
                        full_height: build_ctx.full_height,
                    }],
                    build_element::Pass::Draw => vec![Pass::Draw {
                        width: build_ctx.width,
                        first_height: build_ctx.first_height,
                        preferred_height: 0.,

                        page: 0,
                        layer: 0,
                        pos: (3., 12.),

                        breakable: build_ctx.full_height.map(|full_height| BreakableDraw {
                            full_height,
                            preferred_height_break_count: 0,
                            breaks: if less_first_height {
                                vec![
                                    Break {
                                        page: 1,
                                        layer: 0,
                                        pos: (3., 12.),
                                    },
                                    Break {
                                        page: 2,
                                        layer: 0,
                                        pos: (3., 12.),
                                    },
                                ]
                            } else {
                                vec![Break {
                                    page: 1,
                                    layer: 0,
                                    pos: (3., 12.),
                                }]
                            },
                        }),
                    }],
                },
            );

            let child_2 = {
                let first_height = match (build_ctx.full_height.is_some(), less_first_height) {
                    (false, false) => 10. - 16. - 1.,
                    (false, true) => 4. - 16. - 1.,
                    (true, false) => 3.,
                    (true, true) => 7.,
                };

                AssertPasses::new(
                    ForceBreak,
                    vec![match build_ctx.pass {
                        build_element::Pass::FirstLocationUsage => Pass::FirstLocationUsage {
                            width: build_ctx.width,

                            // if the pass is FirstLocationUsage full_height is always Some
                            first_height,
                            full_height: build_ctx.full_height.unwrap(),
                        },
                        build_element::Pass::Measure => Pass::Measure {
                            width: build_ctx.width,
                            first_height,
                            full_height: build_ctx.full_height,
                        },
                        build_element::Pass::Draw => {
                            if let Some(full_height) = build_ctx.full_height {
                                Pass::Draw {
                                    width: build_ctx.width,
                                    first_height,
                                    preferred_height: 0.,

                                    page: if less_first_height { 2 } else { 1 },
                                    layer: 0,
                                    pos: if less_first_height {
                                        (3., 12. - 3.)
                                    } else {
                                        (3., 12. - 7.)
                                    },

                                    breakable: Some(BreakableDraw {
                                        full_height,
                                        preferred_height_break_count: 0,
                                        breaks: if less_first_height {
                                            vec![Break {
                                                page: 3,
                                                layer: 0,
                                                pos: (3., 12.),
                                            }]
                                        } else {
                                            vec![Break {
                                                page: 2,
                                                layer: 0,
                                                pos: (3., 12.),
                                            }]
                                        },
                                    }),
                                }
                            } else {
                                Pass::Draw {
                                    width: build_ctx.width,
                                    first_height,
                                    preferred_height: 0.,

                                    page: 0,
                                    layer: 0,
                                    pos: (3., 12. - 16. - 1.),

                                    breakable: None,
                                }
                            }
                        }
                    }],
                )
            };

            let child_3 = {
                let first_height = match (build_ctx.full_height.is_some(), less_first_height) {
                    (false, false) => 10. - 16. - 1.,
                    (false, true) => 4. - 16. - 1.,
                    (true, _) => 10.,
                };

                AssertPasses::new(
                    FranticJumper {
                        jumps: vec![0, 5, 3, 3],
                        size: ElementSize {
                            width: Some(5.5),
                            height: Some(1.5),
                        },
                    },
                    vec![match build_ctx.pass {
                        build_element::Pass::FirstLocationUsage => Pass::FirstLocationUsage {
                            width: build_ctx.width,
                            first_height,
                            full_height: 10.,
                        },
                        build_element::Pass::Measure => Pass::Measure {
                            width: build_ctx.width,
                            first_height,
                            full_height: build_ctx.full_height,
                        },
                        build_element::Pass::Draw => {
                            if let Some(full_height) = build_ctx.full_height {
                                let start_page = if less_first_height { 3 } else { 2 };

                                Pass::Draw {
                                    width: build_ctx.width,
                                    first_height,
                                    preferred_height: 0.,
                                    page: start_page,
                                    layer: 0,
                                    pos: (3., 12.),
                                    breakable: Some(BreakableDraw {
                                        full_height,
                                        preferred_height_break_count: 0,
                                        breaks: vec![
                                            Break {
                                                page: start_page + 1,
                                                layer: 0,
                                                pos: (3., 12.),
                                            },
                                            Break {
                                                page: start_page + 6,
                                                layer: 0,
                                                pos: (3., 12.),
                                            },
                                            Break {
                                                page: start_page + 4,
                                                layer: 0,
                                                pos: (3., 12.),
                                            },
                                            Break {
                                                page: start_page + 4,
                                                layer: 0,
                                                pos: (3., 12.),
                                            },
                                        ],
                                    }),
                                }
                            } else {
                                Pass::Draw {
                                    width: build_ctx.width,
                                    first_height,
                                    preferred_height: 0.,

                                    page: 0,
                                    layer: 0,
                                    pos: (3., 12. - 16. - 1.),

                                    breakable: None,
                                }
                            }
                        }
                    }],
                )
            };

            let child_4 = {
                let first_height = match (build_ctx.full_height.is_some(), less_first_height) {
                    (false, false) => 10. - 16. - 1. - 1.5 - 1.,
                    (false, true) => 4. - 16. - 1. - 1.5 - 1.,
                    (true, _) => 10. - 1.5 - 1.,
                };

                AssertPasses::new(
                    NoneElement,
                    vec![match build_ctx.pass {
                        build_element::Pass::FirstLocationUsage => Pass::FirstLocationUsage {
                            width: build_ctx.width,
                            first_height,
                            full_height: 10.,
                        },
                        build_element::Pass::Measure => Pass::Measure {
                            width: build_ctx.width,
                            first_height,
                            full_height: build_ctx.full_height,
                        },
                        build_element::Pass::Draw => {
                            if let Some(full_height) = build_ctx.full_height {
                                let start_page = if less_first_height { 3 } else { 2 } + 6;

                                Pass::Draw {
                                    width: build_ctx.width,
                                    first_height,
                                    preferred_height: 0.,
                                    page: start_page,
                                    layer: 0,
                                    pos: (3., 12. - 1.5 - 1.),
                                    breakable: Some(BreakableDraw {
                                        full_height,
                                        preferred_height_break_count: 0,
                                        breaks: vec![],
                                    }),
                                }
                            } else {
                                Pass::Draw {
                                    width: build_ctx.width,
                                    first_height,
                                    preferred_height: 0.,

                                    page: 0,
                                    layer: 0,
                                    pos: (3., 12. - 16. - 1. - 1.5 - 1.),

                                    breakable: None,
                                }
                            }
                        }
                    }],
                )
            };

            let element = Column {
                gap: 1.,
                content: |content| {
                    content
                        .add(&child_0)?
                        .add(&child_1)?
                        .add(&child_2)?
                        .add(&child_3)?
                        .add(&child_4)?;

                    Some(())
                },
            };

            callback.call(element)
        });

        for output in (ElementTestParams {
            first_height: 4.,
            full_height: 10.,
            width: 6.,
            pos: (3., 12.),
            ..Default::default()
        })
        .run(&element)
        {
            output.assert_size(ElementSize {
                width: Some(output.width.constrain(5.5)),
                height: Some(if output.breakable.is_some() {
                    1.5
                } else {
                    16. + 1. + 1.5
                }),
            });

            if let Some(b) = output.breakable {
                b.assert_break_count(if output.first_height == 4. { 9 } else { 8 })
                    .assert_extra_location_min_height(0.);
            }
        }
    }
}
