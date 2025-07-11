use crate::{utils::max_optional_size, *};

use self::utils::add_optional_size_with_gap;

pub struct BreakList<C: Fn(BreakListContent) -> Option<()>> {
    pub gap: f32,
    pub content: C,
}

impl<C: Fn(BreakListContent) -> Option<()>> Element for BreakList<C> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        FirstLocationUsage::WillUse
    }

    fn measure(&self, mut ctx: MeasureCtx) -> ElementSize {
        let mut max_width = None;
        let mut x_offset = None;
        let mut y_offset = None;
        let mut line_height = None;

        (self.content)(BreakListContent {
            pass: Pass::Measure {
                breakable: ctx.breakable.as_mut(),
            },
            gap: self.gap,
            width_constraint: ctx.width,
            height_available: ctx.first_height,
            max_width: &mut max_width,
            x_offset: &mut x_offset,
            y_offset: &mut y_offset,
            line_height: &mut line_height,
        });

        ElementSize {
            width: if ctx.width.expand {
                Some(ctx.width.max)
            } else {
                max_optional_size(max_width, x_offset)
            },
            height: match (y_offset, line_height) {
                (None, None) => None,
                (None, Some(x)) | (Some(x), None) => Some(x),
                (Some(y_offset), Some(line_height)) => Some(y_offset + self.gap + line_height),
            },
        }
    }

    fn draw(&self, mut ctx: DrawCtx) -> ElementSize {
        let mut max_width = None;
        let mut x_offset = None;
        let mut y_offset = None;
        let mut line_height = None;

        (self.content)(BreakListContent {
            pass: Pass::Draw {
                pdf: ctx.pdf,
                location: ctx.location,
                breakable: ctx.breakable.as_mut().map(|b| (b, 0)),
            },
            gap: self.gap,
            width_constraint: ctx.width,
            height_available: ctx.first_height,
            max_width: &mut max_width,
            x_offset: &mut x_offset,
            y_offset: &mut y_offset,
            line_height: &mut line_height,
        });

        ElementSize {
            width: if ctx.width.expand {
                Some(ctx.width.max)
            } else {
                max_optional_size(max_width, x_offset)
            },
            height: match (y_offset, line_height) {
                (None, None) => None,
                (None, Some(x)) | (Some(x), None) => Some(x),
                (Some(y_offset), Some(line_height)) => Some(y_offset + self.gap + line_height),
            },
        }
    }
}

pub struct BreakListContent<'a, 'b, 'c> {
    pass: Pass<'a, 'b, 'c>,

    gap: f32,

    width_constraint: WidthConstraint,

    height_available: f32,

    max_width: &'a mut Option<f32>,
    x_offset: &'a mut Option<f32>,
    y_offset: &'a mut Option<f32>,
    line_height: &'a mut Option<f32>,
}

enum Pass<'a, 'b, 'c> {
    FirstLocationUsage {},
    Measure {
        breakable: Option<&'a mut BreakableMeasure<'b>>,
    },
    Draw {
        pdf: &'c mut Pdf,
        breakable: Option<(&'a mut BreakableDraw<'b>, u32)>,
        location: Location,
    },
}

impl<'a, 'b, 'c> BreakListContent<'a, 'b, 'c> {
    pub fn add<E: Element>(mut self, element: &E) -> Option<Self> {
        let width_constraint = WidthConstraint {
            max: self.width_constraint.max,
            expand: false,
        };

        let full_height = match self.pass {
            Pass::FirstLocationUsage { .. } => todo!(),
            Pass::Measure { ref breakable } => breakable.as_ref().map(|b| b.full_height),
            Pass::Draw { ref breakable, .. } => breakable.as_ref().map(|b| b.0.full_height),
        };

        let element_size = element.measure(MeasureCtx {
            width: width_constraint,

            // In the unbreakable case this will be more height than is actually available except
            // for the first row. The issue is that what row we're in can depend on the width of the
            // element. And we want to avoid measuring twice. This basically means that elements
            // that expand to first_height are not supported in a BreakList that is in an
            // unbreakable context. This seems like an acceptable tradeoff. It might actually make
            // sense to just not have first_height in unbreakable contexts.
            first_height: full_height.unwrap_or(self.height_available),

            breakable: None,
        });

        // line breaking
        if let (Some(x_offset), Some(width)) = (&mut *self.x_offset, element_size.width) {
            if *x_offset + self.gap + width > self.width_constraint.max {
                *self.max_width = max_optional_size(*self.max_width, Some(*x_offset));
                *self.x_offset = None;

                *self.y_offset = match (*self.y_offset, *self.line_height) {
                    (None, None) => None,
                    (None, Some(x)) | (Some(x), None) => Some(x),
                    (Some(y_offset), Some(line_height)) => Some(y_offset + self.gap + line_height),
                };

                *self.line_height = None;
            }
        }

        let break_needed =
            if let (Some(full_height), Some(height)) = (full_height, element_size.height) {
                let y_offset = self.y_offset.map(|y| y + self.gap).unwrap_or(0.);

                y_offset + height > self.height_available
                    && (y_offset > 0. || full_height > self.height_available)
            } else {
                false
            };

        match self.pass {
            Pass::Measure { ref mut breakable } => {
                if break_needed {
                    *self.x_offset = None;
                    *self.y_offset = None;
                    let breakable = breakable.as_deref_mut().unwrap();
                    *breakable.break_count += 1;
                    self.height_available = breakable.full_height;
                }
            }
            Pass::Draw {
                pdf: &mut ref mut pdf,
                ref mut breakable,
                ref mut location,
            } => {
                if break_needed {
                    let &mut (&mut ref mut breakable, ref mut location_idx) =
                        breakable.as_mut().unwrap();
                    *location = (breakable.do_break)(
                        pdf,
                        *location_idx,
                        add_optional_size_with_gap(*self.y_offset, *self.line_height, self.gap),
                    );
                    *self.x_offset = None;
                    *self.y_offset = None;
                    self.height_available = breakable.full_height;
                    *location_idx += 1;
                }

                let x_offset = if let &mut Some(x_offset) = self.x_offset {
                    x_offset + self.gap
                } else {
                    0.
                };
                let y_offset = self.y_offset.map(|y| y + self.gap).unwrap_or(0.);

                element.draw(DrawCtx {
                    pdf,
                    location: Location {
                        pos: (location.pos.0 + x_offset, location.pos.1 - y_offset),
                        ..*location
                    },

                    // should we only give it the remaining width here?
                    // the thing is that we've already measured with that width constraint so it
                    // should only use as much as it did in measure.
                    width: width_constraint,
                    first_height: element_size.height.unwrap_or(0.),
                    preferred_height: None,
                    breakable: None,
                });
            }
            _ => todo!(),
        }

        // at this point all breaking has been done so we should be able to just add the size
        if let Pass::Measure { .. } | Pass::Draw { .. } = self.pass {
            *self.x_offset = match (*self.x_offset, element_size.width) {
                (None, None) => None,
                (None, Some(x)) | (Some(x), None) => Some(x),
                (Some(x_offset), Some(width)) => Some(x_offset + self.gap + width),
            };

            *self.line_height = max_optional_size(*self.line_height, element_size.height);
        }

        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        elements::{none::NoneElement, rectangle::Rectangle},
        test_utils::{
            assert_passes::{AssertPasses, Pass},
            build_element::BuildElementCtx,
            *,
        },
    };

    #[test]
    fn test_empty() {
        let element = BreakList {
            gap: 12.,
            content: |_content| None,
        };

        for output in ElementTestParams::default().run(&element) {
            output.assert_size(ElementSize {
                width: if output.width.expand {
                    Some(output.width.max)
                } else {
                    None
                },
                height: None,
            });

            if let Some(b) = output.breakable {
                b.assert_break_count(0);
                b.assert_extra_location_min_height(None);
            }
        }
    }

    #[test]
    fn test_none() {
        for configuration in (ElementTestParams {
            first_height: 1.,
            width: 1.,
            full_height: 2.,
            ..Default::default()
        })
        .configurations()
        {
            let element = BuildElement(|BuildElementCtx { pass, .. }, callback| {
                let width = WidthConstraint {
                    max: 1.,
                    expand: false,
                };

                let measure_pass = Pass::Measure {
                    width,
                    first_height: if configuration.breakable || !configuration.use_first_height {
                        2.
                    } else {
                        1.
                    },
                    full_height: None,
                };

                let draw_pass = Pass::Draw {
                    width,
                    first_height: 0.,
                    breakable: None,
                    preferred_height: None,
                    page: 0,
                    layer: 0,
                    pos: configuration.params.pos,
                };

                let child = AssertPasses::new(
                    NoneElement,
                    match pass {
                        build_element::Pass::FirstLocationUsage { .. } => todo!(),
                        build_element::Pass::Measure { .. } => vec![measure_pass],
                        build_element::Pass::Draw { .. } => vec![measure_pass, draw_pass],
                    },
                );

                let element = BreakList {
                    gap: 12.,
                    content: |content| {
                        content.add(&child);

                        None
                    },
                };

                callback.call(element)
            });

            let output = configuration.run(&element);

            output.assert_size(ElementSize {
                width: if output.width.expand {
                    Some(output.width.max)
                } else {
                    None
                },
                height: None,
            });

            if let Some(b) = output.breakable {
                b.assert_break_count(0);
                b.assert_extra_location_min_height(None);
            }
        }
    }

    #[test]
    fn test_passes() {
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
                let width = WidthConstraint {
                    max: 10.,
                    expand: false,
                };

                let first_height = if configuration.breakable || !configuration.use_first_height {
                    10.
                } else {
                    5.
                };

                let child_0 = {
                    let measure_pass = Pass::Measure {
                        width,
                        first_height,
                        full_height: None,
                    };

                    let draw_pass = Pass::Draw {
                        width,
                        first_height: 0.,
                        breakable: None,
                        preferred_height: None,
                        page: 0,
                        layer: 0,
                        pos: (1., 10.),
                    };

                    AssertPasses::new(
                        NoneElement,
                        match pass {
                            build_element::Pass::FirstLocationUsage { .. } => todo!(),
                            build_element::Pass::Measure { .. } => vec![measure_pass],
                            build_element::Pass::Draw { .. } => vec![measure_pass, draw_pass],
                        },
                    )
                };

                let child_1 = {
                    let measure_pass = Pass::Measure {
                        width,
                        first_height,
                        full_height: None,
                    };

                    let breaks = configuration.use_first_height && configuration.breakable;

                    let draw_pass = Pass::Draw {
                        width,
                        first_height: 6.,
                        breakable: None,
                        preferred_height: None,
                        page: if breaks { 1 } else { 0 },
                        layer: 0,
                        pos: (1., 10.),
                    };

                    AssertPasses::new(
                        Rectangle {
                            size: (1., 6.),
                            fill: None,
                            outline: None,
                        },
                        match pass {
                            build_element::Pass::FirstLocationUsage { .. } => todo!(),
                            build_element::Pass::Measure { .. } => vec![measure_pass],
                            build_element::Pass::Draw { .. } => vec![measure_pass, draw_pass],
                        },
                    )
                };

                let on_page_1 = configuration.use_first_height && configuration.breakable;

                let child_2 = {
                    let measure_pass = Pass::Measure {
                        width,
                        first_height,
                        full_height: None,
                    };

                    let draw_pass = Pass::Draw {
                        width,
                        first_height: 4.,
                        breakable: None,
                        preferred_height: None,
                        page: if on_page_1 { 1 } else { 0 },
                        layer: 0,
                        pos: (1. + 1. + gap, 10.),
                    };

                    AssertPasses::new(
                        Rectangle {
                            size: (7., 4.),
                            fill: None,
                            outline: None,
                        },
                        match pass {
                            build_element::Pass::FirstLocationUsage { .. } => todo!(),
                            build_element::Pass::Measure { .. } => vec![measure_pass],
                            build_element::Pass::Draw { .. } => vec![measure_pass, draw_pass],
                        },
                    )
                };

                let child_3 = {
                    let measure_pass = Pass::Measure {
                        width,
                        first_height,
                        full_height: None,
                    };

                    let draw_pass = Pass::Draw {
                        width,
                        first_height: 4.,
                        breakable: None,
                        preferred_height: None,
                        page: if on_page_1 {
                            2
                        } else if configuration.breakable {
                            1
                        } else {
                            0
                        },
                        layer: 0,
                        pos: (
                            1.,
                            if configuration.breakable {
                                10.
                            } else {
                                10. - 6. - gap
                            },
                        ),
                    };

                    AssertPasses::new(
                        Rectangle {
                            size: (1., 4.),
                            fill: None,
                            outline: None,
                        },
                        match pass {
                            build_element::Pass::FirstLocationUsage { .. } => todo!(),
                            build_element::Pass::Measure { .. } => vec![measure_pass],
                            build_element::Pass::Draw { .. } => vec![measure_pass, draw_pass],
                        },
                    )
                };

                let element = BreakList {
                    gap,
                    content: |content| {
                        content
                            .add(&child_0)?
                            .add(&child_1)?
                            .add(&child_2)?
                            .add(&child_3)?;

                        None
                    },
                };

                callback.call(element)
            });

            let output = configuration.run(&element);

            output.assert_size(ElementSize {
                width: if output.width.expand {
                    Some(output.width.max)
                } else {
                    Some(1. + gap + 7.)
                },
                height: Some(if configuration.breakable {
                    4.
                } else {
                    6. + gap + 4.
                }),
            });

            if let Some(b) = output.breakable {
                b.assert_break_count(if configuration.use_first_height { 2 } else { 1 });
                b.assert_extra_location_min_height(None);
            }
        }
    }

    #[test]
    fn no_unhelpful_breaks() {
        // If an element overflows the height, but breaking would not help because the next location
        // / page is not larger then no breaking should happen.

        {
            let element = BreakList {
                gap: 1.,
                content: |content| {
                    content
                        .add(&Rectangle {
                            size: (1., 9.),
                            fill: None,
                            outline: None,
                        })?
                        .add(&Rectangle {
                            size: (1., 9.),
                            fill: None,
                            outline: None,
                        })?
                        .add(&Rectangle {
                            size: (1., 9.),
                            fill: None,
                            outline: None,
                        })?;

                    None
                },
            };

            let output = test_measure_draw_compatibility(
                &element,
                WidthConstraint {
                    max: 3.,
                    expand: true,
                },
                8.,
                Some(8.),
                (1., 2.),
                (0., 0.),
            );

            output.assert_size(ElementSize {
                width: Some(3.),
                height: Some(9.),
            });
            output.breakable.unwrap().assert_break_count(1);
        }

        {
            // If there's no gap and a row with a height of zero it still is the full height. For
            // non-zero gaps we could just look at whether the y offset is None (if a
            // NoneElement was all there is in the previous row for example) and if it isn't just
            // assume we don't have the full height because the first height can't be more than the
            // full_heigth. But for a zero gap that optimization doesn't work.
            let element = BreakList {
                gap: 0.,
                content: |content| {
                    content
                        .add(&Rectangle {
                            size: (1., 9.),
                            fill: None,
                            outline: None,
                        })?
                        .add(&Rectangle {
                            size: (1.5, 0.),
                            fill: None,
                            outline: None,
                        })?
                        .add(&Rectangle {
                            size: (1., 9.),
                            fill: None,
                            outline: None,
                        })?;

                    None
                },
            };

            let output = test_measure_draw_compatibility(
                &element,
                WidthConstraint {
                    max: 2.,
                    expand: false,
                },
                8.,
                Some(8.),
                (1., 2.),
                (0., 0.),
            );

            output.assert_size(ElementSize {
                width: Some(1.5),
                height: Some(9.),
            });
            output.breakable.unwrap().assert_break_count(1);
        }
    }
}
