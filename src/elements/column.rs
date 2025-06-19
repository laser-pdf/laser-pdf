use crate::*;

use self::utils::add_optional_size_with_gap;

/// A container that arranges child elements vertically with optional gaps.
///
/// Elements are laid out from top to bottom with configurable spacing between them.
/// Supports page breaking and collapsing behavior for empty elements.
///
/// ## Gap Behavior
///
/// Gaps are only applied between elements that have actual height. Elements with
/// `None` height (collapsed elements) don't get gaps before or after them.
///
/// ## Collapse Behavior
///
/// When `collapse: true` (default):
/// - If all children have `None` height, the column returns `None` height
/// - If all children have `None` width, the column returns `None` width
/// - The gaps around a child with a `None` height are collapsed into one.
///
/// When `collapse: false`:
/// - Empty columns return `Some(0.0)` for height/width instead of `None`
/// - Useful when you need a column to always occupy space even when empty
/// - A child with a `None` height will still have a gap on either side
///
/// ## Page Breaking
///
/// In breakable contexts, when a child element causes a page break, the column's
/// accumulated height is reset and continues on the new page.
pub struct Column<C: Fn(ColumnContent) -> Option<()>> {
    /// Closure that gets called for adding the content.
    ///
    /// The closure is basically an internal iterator that produces elements by calling
    /// [ColumnContent::add]. For short circuiting with the `?` operator [ColumnContent::add] and
    /// this closure return an [Option].
    ///
    /// If the column is in a context that measures it before drawing (such as `BreakWhole`), this
    /// function will be called twice. In more complicated nested layouts it could be called more
    /// than that (though in real world layouts this effect should be minimal as not all containers
    /// need a measure pass before drawing). Because of this it's beneficial to keep expensive
    /// computations and allocation outside of this closure.
    pub content: C,
    /// Vertical spacing between elements in millimeters
    pub gap: f32,
    /// Whether to collapse to None size when all children are collapsed.
    /// When false, empty columns return Some(0.0) instead of None.
    pub collapse: bool,
}

impl<C: Fn(ColumnContent) -> Option<()>> Column<C> {
    pub fn new(content: C) -> Self {
        Column {
            content,
            gap: 0.,
            collapse: true,
        }
    }

    pub fn with_gap(self, gap: f32) -> Self {
        Column { gap, ..self }
    }

    pub fn with_collapse(self, collapse: bool) -> Self {
        Column { collapse, ..self }
    }
}

impl<C: Fn(ColumnContent) -> Option<()>> Element for Column<C> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        let mut ret = FirstLocationUsage::NoneHeight;

        (self.content)(ColumnContent {
            pass: Pass::InsufficientFirstHeight { ctx, ret: &mut ret },
            gap: self.gap,
        });

        if !self.collapse && ret == FirstLocationUsage::NoneHeight {
            ret = FirstLocationUsage::WillUse;
        }

        ret
    }

    fn measure(&self, mut ctx: MeasureCtx) -> ElementSize {
        let mut width = None;
        let mut height = None;
        let mut break_count = 0;

        (self.content)(ColumnContent {
            pass: Pass::Measure {
                width_constraint: ctx.width,
                breakable: ctx.breakable.as_mut().map(|b| BreakableMeasure {
                    break_count: &mut break_count,
                    extra_location_min_height: b.extra_location_min_height,
                    ..*b
                }),
                height_available: ctx.first_height,
                width: &mut width,
                height: &mut height,
            },
            gap: self.gap,
        });

        if let Some(breakable) = ctx.breakable {
            *breakable.break_count = break_count;
        }

        if !self.collapse {
            if height.is_none() && break_count == 0 {
                height = Some(0.);
            }

            if width.is_none() {
                width = Some(0.);
            }
        }

        ElementSize { width, height }
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        let mut width = None;
        let mut height = None;
        let mut location_offset = 0;

        (self.content)(ColumnContent {
            pass: Pass::Draw {
                pdf: ctx.pdf,
                location: ctx.location,
                location_offset: &mut location_offset,
                width_constraint: ctx.width,
                breakable: ctx.breakable,
                height_available: ctx.first_height,
                width: &mut width,
                height: &mut height,
            },
            gap: self.gap,
        });

        if !self.collapse {
            if height.is_none() && location_offset == 0 {
                height = Some(0.);
            }

            if width.is_none() {
                width = Some(0.);
            }
        }

        ElementSize { width, height }
    }
}

pub struct ColumnContent<'a, 'b, 'r> {
    pass: Pass<'a, 'b, 'r>,
    gap: f32,
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
        height_available: f32,
        width: &'r mut Option<f32>,
        height: &'r mut Option<f32>,
    },
    Draw {
        pdf: &'a mut Pdf,
        location: Location,
        location_offset: &'r mut u32,
        width_constraint: WidthConstraint,
        breakable: Option<BreakableDraw<'b>>,

        /// this is initially first_height and when breaking we set it to full height
        height_available: f32,
        width: &'r mut Option<f32>,
        height: &'r mut Option<f32>,
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
                    let mut extra_location_min_height = None;

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
                location_offset: &mut ref mut location_offset,
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
                    preferred_height: None,
                    breakable: None,
                };

                let size = if let Some(b) = breakable {
                    let mut break_count = 0;

                    let size = element.draw(DrawCtx {
                        breakable: Some(BreakableDraw {
                            full_height: b.full_height,
                            preferred_height_break_count: 0,
                            do_break: &mut |pdf, location_idx, location_height| {
                                *height_available = b.full_height;

                                let location_height = if location_idx == 0 {
                                    add_optional_size_with_gap(location_height, *height, self.gap)
                                } else {
                                    location_height
                                };

                                let new_location = (b.do_break)(
                                    pdf,
                                    location_idx + *location_offset,
                                    location_height,
                                );

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
            collapse: true,
            content: |_| Some(()),
        };

        for output in ElementTestParams::default().run(&element) {
            output.assert_size(ElementSize {
                width: None,
                height: None,
            });

            if let Some(b) = output.breakable {
                b.assert_break_count(0)
                    .assert_extra_location_min_height(None);
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
                        build_element::Pass::FirstLocationUsage { full_height } => {
                            vec![Pass::FirstLocationUsage {
                                width: build_ctx.width,
                                first_height: build_ctx.first_height,
                                full_height,
                            }]
                        }
                        build_element::Pass::Measure { full_height } => vec![Pass::Measure {
                            width: build_ctx.width,
                            first_height: build_ctx.first_height,
                            full_height,
                        }],
                        build_element::Pass::Draw { ref breakable, .. } => vec![Pass::Draw {
                            width: build_ctx.width,
                            first_height: build_ctx.first_height,
                            preferred_height: None,

                            page: 0,
                            layer: 0,
                            pos: (3., 12.),

                            breakable: breakable.as_ref().map(|b| BreakableDraw {
                                full_height: b.full_height,
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
                collapse: true,
                content: |content| {
                    content.add(&none_0)?.add(&none_1)?.add(&none_2)?;

                    None
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
                b.assert_break_count(0)
                    .assert_extra_location_min_height(None);
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
                    build_element::Pass::FirstLocationUsage { full_height } => {
                        vec![Pass::FirstLocationUsage {
                            width: build_ctx.width,
                            first_height: build_ctx.first_height,
                            full_height,
                        }]
                    }
                    build_element::Pass::Measure { full_height } => vec![Pass::Measure {
                        width: build_ctx.width,
                        first_height: build_ctx.first_height,
                        full_height,
                    }],
                    build_element::Pass::Draw { ref breakable, .. } => vec![Pass::Draw {
                        width: build_ctx.width,
                        first_height: build_ctx.first_height,
                        preferred_height: None,

                        page: 0,
                        layer: 0,
                        pos: (3., 12.),

                        breakable: breakable.as_ref().map(|b| BreakableDraw {
                            full_height: b.full_height,
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
                    build_element::Pass::FirstLocationUsage { full_height } => {
                        vec![Pass::FirstLocationUsage {
                            width: build_ctx.width,
                            first_height: build_ctx.first_height,
                            full_height,
                        }]
                    }
                    build_element::Pass::Measure { full_height } => vec![Pass::Measure {
                        width: build_ctx.width,
                        first_height: build_ctx.first_height,
                        full_height,
                    }],
                    build_element::Pass::Draw { ref breakable, .. } => vec![Pass::Draw {
                        width: build_ctx.width,
                        first_height: build_ctx.first_height,
                        preferred_height: None,

                        page: 0,
                        layer: 0,
                        pos: (3., 12.),

                        breakable: breakable.as_ref().map(|b| BreakableDraw {
                            full_height: b.full_height,
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
                let first_height = match (build_ctx.is_breakable(), less_first_height) {
                    (false, false) => 10. - 16. - 1.,
                    (false, true) => 4. - 16. - 1.,
                    (true, false) => 3.,
                    (true, true) => 7.,
                };

                AssertPasses::new(
                    ForceBreak,
                    match build_ctx.pass {
                        build_element::Pass::FirstLocationUsage { .. } => vec![],
                        build_element::Pass::Measure { full_height } => vec![Pass::Measure {
                            width: build_ctx.width,
                            first_height,
                            full_height,
                        }],
                        build_element::Pass::Draw { ref breakable, .. } => {
                            vec![if let Some(breakable) = breakable {
                                Pass::Draw {
                                    width: build_ctx.width,
                                    first_height,
                                    preferred_height: None,

                                    page: if less_first_height { 2 } else { 1 },
                                    layer: 0,
                                    pos: if less_first_height {
                                        (3., 12. - 3.)
                                    } else {
                                        (3., 12. - 7.)
                                    },

                                    breakable: Some(BreakableDraw {
                                        full_height: breakable.full_height,
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
                                    preferred_height: None,

                                    page: 0,
                                    layer: 0,
                                    pos: (3., 12. - 16. - 1.),

                                    breakable: None,
                                }
                            }]
                        }
                    },
                )
            };

            let child_3 = {
                let first_height = match (build_ctx.is_breakable(), less_first_height) {
                    (false, false) => 10. - 16. - 1.,
                    (false, true) => 4. - 16. - 1.,
                    (true, _) => 10.,
                };

                AssertPasses::new(
                    FranticJumper {
                        jumps: vec![
                            (0, Some(0.)),
                            (5, Some(1.5)),
                            (3, Some(1.5)),
                            (3, Some(1.5)),
                        ],
                        size: ElementSize {
                            width: Some(5.5),
                            height: Some(1.5),
                        },
                    },
                    match build_ctx.pass {
                        build_element::Pass::FirstLocationUsage { .. } => vec![],
                        build_element::Pass::Measure { full_height } => vec![Pass::Measure {
                            width: build_ctx.width,
                            first_height,
                            full_height,
                        }],
                        build_element::Pass::Draw { ref breakable, .. } => {
                            vec![if let Some(breakable) = breakable {
                                let start_page = if less_first_height { 3 } else { 2 };

                                Pass::Draw {
                                    width: build_ctx.width,
                                    first_height,
                                    preferred_height: None,
                                    page: start_page,
                                    layer: 0,
                                    pos: (3., 12.),
                                    breakable: Some(BreakableDraw {
                                        full_height: breakable.full_height,
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
                                    preferred_height: None,

                                    page: 0,
                                    layer: 0,
                                    pos: (3., 12. - 16. - 1.),

                                    breakable: None,
                                }
                            }]
                        }
                    },
                )
            };

            let child_4 = {
                let first_height = match (build_ctx.is_breakable(), less_first_height) {
                    (false, false) => 10. - 16. - 1. - 1.5 - 1.,
                    (false, true) => 4. - 16. - 1. - 1.5 - 1.,
                    (true, _) => 10. - 1.5 - 1.,
                };

                AssertPasses::new(
                    NoneElement,
                    match build_ctx.pass {
                        build_element::Pass::FirstLocationUsage { .. } => vec![],
                        build_element::Pass::Measure { full_height } => vec![Pass::Measure {
                            width: build_ctx.width,
                            first_height,
                            full_height,
                        }],
                        build_element::Pass::Draw { ref breakable, .. } => {
                            vec![if let Some(breakable) = breakable {
                                let start_page = if less_first_height { 3 } else { 2 } + 6;

                                Pass::Draw {
                                    width: build_ctx.width,
                                    first_height,
                                    preferred_height: None,
                                    page: start_page,
                                    layer: 0,
                                    pos: (3., 12. - 1.5 - 1.),
                                    breakable: Some(BreakableDraw {
                                        full_height: breakable.full_height,
                                        preferred_height_break_count: 0,
                                        breaks: vec![],
                                    }),
                                }
                            } else {
                                Pass::Draw {
                                    width: build_ctx.width,
                                    first_height,
                                    preferred_height: None,

                                    page: 0,
                                    layer: 0,
                                    pos: (3., 12. - 16. - 1. - 1.5 - 1.),

                                    breakable: None,
                                }
                            }]
                        }
                    },
                )
            };

            let element = Column {
                gap: 1.,
                collapse: false,
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
                    .assert_extra_location_min_height(None);
            }
        }
    }
}
