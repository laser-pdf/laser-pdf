use crate::{
    flex::{DrawLayout, MeasureLayout},
    utils::max_optional_size,
    *,
};

use super::none::NoneElement;

/// A container that arranges child elements horizontally with flexible sizing.
///
/// Elements can be sized as self-sized, fixed width, or expanding to fill available space.
/// The `expand` option makes all children the same height by passing the maximum height
/// and break count as `preferred_height` and `preferred_height_break_count`, enabling
/// features like bottom alignment and background fills.
///
/// The `preferred_height_break_count` represents the number of page breaks, with
/// `preferred_height` being the height on the final page. For example, if
/// `preferred_height_break_count = 2` and `preferred_height = 15.0`, it means
/// \"break twice, then use 15mm on the final page\".
///
/// ## Flex System
///
/// - `Flex::SelfSized`: Element uses its natural width
/// - `Flex::Fixed(width)`: Element uses the specified width  
/// - `Flex::Expand(weight)`: Element gets a portion of remaining space based on weight
///
/// Remaining space is calculated as: total_width - sum(self_sized_widths) - sum(fixed_widths)
/// Then distributed proportionally: element_width = remaining_space * (weight / total_weights)
///
/// ## Performance Note
///
/// When `expand: false`, only self-sized elements are measured in the first pass.
/// When `expand: true`, all elements must be measured before drawing to determine
/// the maximum height, which requires an additional measurement pass.
pub struct Row<F: Fn(&mut RowContent)> {
    /// Horizontal spacing between elements in millimeters
    pub gap: f32,
    /// Whether to expand all children to the same height by passing preferred_height
    pub expand: bool,
    /// Whether to collapse when all children have None height/width
    pub collapse: bool,
    /// Closure that gets called for adding the content.
    ///
    /// The closure is basically an internal iterator that produces elements by calling
    /// [RowContent::add].
    ///
    /// This closure will be called at least twice because the non-expanded elements need to be
    /// measured first. Depending on the surrounding context it could be called more than that
    /// (though in real world layouts this effect should be minimal as not all containers need a
    /// measure pass before drawing). Because of this it's beneficial to keep expensive computations
    /// and allocations outside of this closure.
    pub content: F,
}

impl<F: Fn(&mut RowContent)> Row<F> {
    pub fn new(content: F) -> Self {
        Row {
            gap: 0.,
            expand: false,
            collapse: true,
            content,
        }
    }

    pub fn with_gap(self, gap: f32) -> Self {
        Row { gap, ..self }
    }

    pub fn with_expand(self, expand: bool) -> Self {
        Row { expand, ..self }
    }

    pub fn with_collapse(self, collapse: bool) -> Self {
        Row { collapse, ..self }
    }

    pub fn expand(self) -> Self {
        Row {
            expand: true,
            ..self
        }
    }
}

impl<F: Fn(&mut RowContent)> Element for Row<F> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        FirstLocationUsage::WillUse
    }

    fn measure(&self, mut ctx: MeasureCtx) -> ElementSize {
        let mut measure_layout = MeasureLayout::new(ctx.width.max, self.gap);

        let mut max_height = None;

        (self.content)(&mut RowContent {
            width: ctx.width,
            first_height: ctx.first_height,
            pass: Pass::MeasureNonExpanded {
                layout: &mut measure_layout,
                max_height: Some(&mut max_height),
                breakable: ctx.breakable.as_mut(),
            },
        });

        let mut width = measure_layout.no_expand_width();

        let draw_layout = measure_layout.build();

        (self.content)(&mut RowContent {
            width: ctx.width,
            first_height: ctx.first_height,
            pass: Pass::MeasureExpanded {
                layout: &draw_layout,
                max_height: &mut max_height,
                width: if ctx.width.expand {
                    None
                } else {
                    Some(&mut width)
                },
                width_expand: ctx.width.expand,
                gap: self.gap,
                breakable: ctx.breakable.as_mut(),
            },
        });

        if !self.collapse {
            if width.is_none() {
                width = Some(0.);
            }

            if max_height.is_none() {
                max_height = Some(0.);
            }
        }

        ElementSize {
            width: if ctx.width.expand {
                Some(width.map(|w| w.max(ctx.width.max)).unwrap_or(ctx.width.max))
            } else {
                width
            },
            height: max_height,
        }
    }

    fn draw(&self, mut ctx: DrawCtx) -> ElementSize {
        let mut measure_layout = MeasureLayout::new(ctx.width.max, self.gap);

        let mut max_height = None;

        let mut break_count = 0;
        let mut extra_location_min_height = None;

        (self.content)(&mut RowContent {
            width: ctx.width,
            first_height: ctx.first_height,
            pass: Pass::MeasureNonExpanded {
                layout: &mut measure_layout,
                max_height: if self.expand {
                    Some(&mut max_height)
                } else {
                    None
                },
                breakable: ctx
                    .breakable
                    .as_ref()
                    .map(|b| BreakableMeasure {
                        full_height: b.full_height,

                        // in the non-expand case these will just be ignored
                        break_count: &mut break_count,
                        extra_location_min_height: &mut extra_location_min_height,
                    })
                    .as_mut(),
            },
        });

        let draw_layout = measure_layout.build();

        // If we want to expand all of the children to the same size we need an additional pass here
        // to figure out the maximum height & break count of all of the children. This is part of
        // the reason why expanding isn't just what Row always does.
        if self.expand {
            (self.content)(&mut RowContent {
                width: ctx.width,
                first_height: ctx.first_height,
                pass: Pass::MeasureExpanded {
                    layout: &draw_layout,
                    max_height: &mut max_height,
                    width_expand: ctx.width.expand,
                    width: None, // We'll get that from draw. No point in getting it twice.
                    gap: self.gap,
                    breakable: ctx
                        .breakable
                        .as_ref()
                        .map(|b| BreakableMeasure {
                            full_height: b.full_height,

                            // in the non-expand case these will just be ignored
                            break_count: &mut break_count,
                            extra_location_min_height: &mut extra_location_min_height,
                        })
                        .as_mut(),
                },
            });

            if let Some(ref mut b) = ctx.breakable {
                match break_count.cmp(&b.preferred_height_break_count) {
                    std::cmp::Ordering::Less => (),
                    std::cmp::Ordering::Equal => {
                        ctx.preferred_height = max_optional_size(ctx.preferred_height, max_height);
                    }
                    std::cmp::Ordering::Greater => {
                        b.preferred_height_break_count = break_count;
                        ctx.preferred_height = max_height;
                    }
                }
            } else {
                ctx.preferred_height = max_optional_size(ctx.preferred_height, max_height);
            }
        }

        let mut width = None;

        (self.content)(&mut RowContent {
            width: ctx.width,
            first_height: ctx.first_height,
            pass: Pass::Draw {
                layout: &draw_layout,
                max_height: &mut max_height,
                width: &mut width,
                width_expand: ctx.width.expand,
                gap: self.gap,
                pdf: ctx.pdf,
                location: ctx.location,
                preferred_height: ctx.preferred_height,
                break_count: 0,
                breakable: ctx.breakable.as_mut(),
            },
        });

        if !self.collapse {
            if width.is_none() {
                width = Some(0.);
            }

            if max_height.is_none() {
                max_height = Some(0.);
            }
        }

        ElementSize {
            width: if ctx.width.expand {
                Some(width.map(|w| w.max(ctx.width.max)).unwrap_or(ctx.width.max))
            } else {
                width
            },
            height: max_height,
        }
    }
}

pub struct RowContent<'a, 'b, 'c> {
    width: WidthConstraint,
    first_height: f32,
    pass: Pass<'a, 'b, 'c>,
}

enum Pass<'a, 'b, 'c> {
    MeasureNonExpanded {
        layout: &'a mut MeasureLayout,
        max_height: Option<&'a mut Option<f32>>,
        breakable: Option<&'a mut BreakableMeasure<'b>>,
    },

    FirstLocationUsage {},

    MeasureExpanded {
        layout: &'a DrawLayout,
        max_height: &'a mut Option<f32>,
        width: Option<&'a mut Option<f32>>,
        width_expand: bool,
        gap: f32,
        breakable: Option<&'a mut BreakableMeasure<'b>>,
    },

    Draw {
        layout: &'a DrawLayout,
        max_height: &'a mut Option<f32>,
        width: &'a mut Option<f32>,
        width_expand: bool,

        gap: f32,

        pdf: &'c mut Pdf,
        location: Location,

        preferred_height: Option<f32>,
        break_count: u32,
        breakable: Option<&'a mut BreakableDraw<'b>>,
    },
}

/// Flex behavior determining how row elements are sized horizontally.
#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum Flex {
    /// Expand to fill available space proportionally based on weight.
    /// Remaining space after self-sized and fixed elements is distributed
    /// proportionally: weight / sum_of_all_weights.
    Expand(u8),
    /// Use the element's natural width
    SelfSized,
    /// Use a fixed width in millimeters
    Fixed(f32),
}

fn add_height(
    max_height: &mut Option<f32>,
    breakable: Option<&mut BreakableMeasure>,
    size: ElementSize,
    break_count: u32,
    extra_location_min_height: Option<f32>,
) {
    if let Some(b) = breakable {
        *b.extra_location_min_height =
            max_optional_size(extra_location_min_height, *b.extra_location_min_height);

        match break_count.cmp(b.break_count) {
            std::cmp::Ordering::Less => (),
            std::cmp::Ordering::Equal => {
                *max_height = max_optional_size(*max_height, size.height);
            }
            std::cmp::Ordering::Greater => {
                *b.break_count = break_count;
                *max_height = size.height;
            }
        }
    } else {
        *max_height = max_optional_size(*max_height, size.height);
    }
}

impl<'a, 'b, 'c> RowContent<'a, 'b, 'c> {
    /// Add a flexible gap that expands to fill space.
    ///
    /// This is equivalent to adding a `NoneElement` with `Flex::Expand(gap)`.
    /// Useful for pushing elements apart or centering them with flexible spacing.
    pub fn flex_gap(&mut self, gap: u8) {
        self.add(&NoneElement, Flex::Expand(gap));
    }

    pub fn add<E: Element>(&mut self, element: &E, flex: Flex) {
        match self.pass {
            Pass::MeasureNonExpanded {
                layout: &mut ref mut layout,
                ref mut max_height,
                ref mut breakable,
            } => match flex {
                Flex::Expand(fraction) => {
                    layout.add_expand(fraction);
                }
                Flex::SelfSized => {
                    let mut break_count = 0;
                    let mut extra_location_min_height = None;

                    let size = element.measure(MeasureCtx {
                        width: WidthConstraint {
                            expand: false,
                            ..self.width
                        },
                        first_height: self.first_height,
                        breakable: breakable.as_deref_mut().map(|b| BreakableMeasure {
                            full_height: b.full_height,
                            break_count: &mut break_count,
                            extra_location_min_height: &mut extra_location_min_height,
                        }),
                    });

                    if let Some(max_height) = max_height {
                        // if max_height is None we're not interested in height or breaks
                        add_height(
                            max_height,
                            breakable.as_deref_mut(),
                            size,
                            break_count,
                            extra_location_min_height,
                        );
                    }

                    // elements with no width are collapsed
                    if let Some(w) = size.width {
                        layout.add_fixed(w);
                    }
                }
                Flex::Fixed(width) => {
                    layout.add_fixed(width);

                    if let Some(max_height) = max_height {
                        let mut break_count = 0;
                        let mut extra_location_min_height = None;

                        let size = element.measure(MeasureCtx {
                            width: WidthConstraint {
                                max: width,
                                expand: true,
                            },
                            first_height: self.first_height,
                            breakable: breakable.as_mut().map(|b| BreakableMeasure {
                                full_height: b.full_height,
                                break_count: &mut break_count,
                                extra_location_min_height: &mut extra_location_min_height,
                            }),
                        });

                        add_height(
                            max_height,
                            breakable.as_deref_mut(),
                            size,
                            break_count,
                            extra_location_min_height,
                        );
                    }
                }
            },

            Pass::MeasureExpanded {
                layout,
                max_height: &mut ref mut max_height,
                ref mut width,
                width_expand,
                gap,
                ref mut breakable,
            } => match flex {
                Flex::Expand(fraction) => {
                    let element_width = layout.expand_width(fraction);

                    let mut break_count = 0;
                    let mut extra_location_min_height = None;

                    let size = element.measure(MeasureCtx {
                        width: WidthConstraint {
                            max: element_width,
                            expand: width_expand,
                        },
                        first_height: self.first_height,
                        breakable: breakable.as_deref_mut().map(|b| BreakableMeasure {
                            full_height: b.full_height,
                            break_count: &mut break_count,
                            extra_location_min_height: &mut extra_location_min_height,
                        }),
                    });

                    add_height(
                        max_height,
                        breakable.as_deref_mut(),
                        size,
                        break_count,
                        extra_location_min_height,
                    );

                    if let &mut Some(&mut ref mut width) = width {
                        if let Some(w) = size.width {
                            if let Some(width) = width {
                                *width += gap + w;
                            } else {
                                *width = Some(w);
                            }
                        }
                    }
                }
                Flex::SelfSized => (),
                Flex::Fixed(_) => (),
            },

            Pass::Draw {
                layout,
                max_height: &mut ref mut max_height,
                width: &mut ref mut width,
                width_expand,
                gap,
                pdf: &mut ref mut pdf,
                ref location,
                preferred_height,
                ref mut break_count,
                ref mut breakable,
            } => {
                let width_constraint = match flex {
                    Flex::Expand(fraction) => WidthConstraint {
                        max: layout.expand_width(fraction),
                        expand: width_expand,
                    },
                    Flex::SelfSized => WidthConstraint {
                        max: self.width.max,
                        expand: false,
                    },
                    Flex::Fixed(width) => WidthConstraint {
                        max: width,
                        expand: true,
                    },
                };

                let mut element_break_count = 0;

                let x_offset = if let &mut Some(width) = width {
                    width + gap
                } else {
                    0.
                };

                let size = element.draw(DrawCtx {
                    pdf,
                    location: Location {
                        pos: (location.pos.0 + x_offset, location.pos.1),
                        ..location.clone()
                    },

                    width: width_constraint,
                    first_height: self.first_height,
                    preferred_height,

                    // some trickery to get rust to make a temporary option that owns the closure
                    breakable: breakable
                        .as_deref_mut()
                        .map(|b| {
                            (
                                b.full_height,
                                b.preferred_height_break_count,
                                |pdf: &mut Pdf, location_idx: u32, _| {
                                    element_break_count = element_break_count.max(location_idx + 1);

                                    let mut new_location = (b.do_break)(
                                        pdf,
                                        location_idx,
                                        Some(if location_idx == 0 {
                                            self.first_height
                                        } else {
                                            b.full_height
                                        }),
                                    );
                                    new_location.pos.0 += x_offset;
                                    new_location
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
                });

                if breakable.is_some() {
                    match element_break_count.cmp(break_count) {
                        std::cmp::Ordering::Less => (),
                        std::cmp::Ordering::Equal => {
                            *max_height = max_optional_size(*max_height, size.height);
                        }
                        std::cmp::Ordering::Greater => {
                            *break_count = element_break_count;
                            *max_height = size.height;
                        }
                    }
                } else {
                    *max_height = max_optional_size(*max_height, size.height);
                }

                let mut width_add = |w| {
                    if let Some(width) = width {
                        *width += gap + w;
                    } else {
                        *width = Some(w);
                    }
                };

                match (flex, width_expand) {
                    (Flex::Expand(_), true) | (Flex::Fixed(_), _) => {
                        width_add(width_constraint.max);
                    }
                    (Flex::Expand(_), false) | (Flex::SelfSized, _) => {
                        if let Some(w) = size.width {
                            width_add(w);
                        }
                    }
                }
            }

            _ => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        elements::{force_break::ForceBreak, none::NoneElement, rectangle::Rectangle},
        test_utils::{build_element::BuildElementCtx, *},
    };

    #[test]
    fn test_empty_row() {
        let element = Row {
            gap: 12.,
            expand: true,
            collapse: true,
            content: |_content| {},
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

        let element = Row {
            gap: 12.,
            expand: false,
            collapse: true,
            content: |_content| {},
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
    fn test_row_expand() {
        test_row(true);
    }

    #[test]
    fn test_row_no_expand() {
        test_row(false);
    }

    fn test_row(expand: bool) {
        use assert_passes::*;

        let gap = 1.;

        let element = BuildElement(
            |BuildElementCtx {
                 width,
                 first_height,
                 mut pass,
             },
             callback| {
                let less_first_height = first_height == 6.;

                // This way we don't have to duplicate the logic for this in every child. They
                // should all get the same preferred height.
                if let build_element::Pass::Draw {
                    preferred_height,
                    breakable,
                } = &mut pass
                {
                    if preferred_height.is_some() || expand {
                        if let Some(breakable) = breakable {
                            let (height, break_count) =
                                if less_first_height { (6., 2) } else { (12., 1) };

                            *preferred_height = Some(height);
                            breakable.preferred_height_break_count = break_count;
                        } else {
                            *preferred_height = Some(24.);
                        }
                    }
                }

                let child_0 = AssertPasses::new(
                    NoneElement,
                    match pass {
                        build_element::Pass::FirstLocationUsage { .. } => todo!(),
                        build_element::Pass::Measure { full_height } => vec![Pass::Measure {
                            width: WidthConstraint {
                                max: width.max,
                                expand: false,
                            },
                            first_height,
                            full_height,
                        }],
                        build_element::Pass::Draw {
                            ref breakable,
                            preferred_height,
                        } => vec![
                            Pass::Measure {
                                width: WidthConstraint {
                                    max: width.max,
                                    expand: false,
                                },
                                first_height: first_height,
                                full_height: breakable.as_ref().map(|b| b.full_height),
                            },
                            Pass::Draw {
                                width: WidthConstraint {
                                    max: width.max,
                                    expand: false,
                                },
                                first_height: first_height,
                                preferred_height,

                                page: 0,
                                layer: 0,
                                pos: (12., 14.),

                                breakable: breakable.as_ref().map(|b| BreakableDraw {
                                    full_height: b.full_height,
                                    preferred_height_break_count: b.preferred_height_break_count,
                                    breaks: Vec::new(),
                                }),
                            },
                        ],
                    },
                );

                let child_1 = {
                    let width = WidthConstraint {
                        max: 6.,
                        expand: width.expand,
                    };

                    AssertPasses::new(
                        Rectangle {
                            size: (5., 5.),
                            fill: None,
                            outline: None,
                        },
                        match pass {
                            build_element::Pass::FirstLocationUsage { .. } => todo!(),
                            build_element::Pass::Measure { full_height } => vec![Pass::Measure {
                                width,
                                first_height,
                                full_height,
                            }],
                            build_element::Pass::Draw {
                                ref breakable,
                                preferred_height,
                            } => {
                                let mut r = Vec::new();

                                if expand {
                                    r.push(Pass::Measure {
                                        width,
                                        first_height,
                                        full_height: breakable.as_ref().map(|b| b.full_height),
                                    });
                                }

                                r.push(Pass::Draw {
                                    width,
                                    first_height,
                                    preferred_height,

                                    page: 0,
                                    layer: 0,
                                    pos: (12., 14.),

                                    breakable: breakable.as_ref().map(|b| BreakableDraw {
                                        full_height: b.full_height,
                                        preferred_height_break_count: b
                                            .preferred_height_break_count,
                                        breaks: Vec::new(),
                                    }),
                                });

                                r
                            }
                        },
                    )
                };

                let child_1_width = if width.expand { 6. } else { 5. } + gap;

                let child_2 = {
                    let x = 12. + child_1_width;

                    AssertPasses::new(
                        FakeText {
                            lines: 12,
                            line_height: 2.,
                            width: 500.,
                        },
                        match pass {
                            build_element::Pass::FirstLocationUsage { .. } => todo!(),
                            build_element::Pass::Measure { full_height } => vec![Pass::Measure {
                                width: WidthConstraint {
                                    max: 3.,
                                    expand: true,
                                },
                                first_height,
                                full_height,
                            }],
                            build_element::Pass::Draw {
                                ref breakable,
                                preferred_height,
                            } => {
                                let mut r = Vec::new();

                                if expand {
                                    r.push(Pass::Measure {
                                        width: WidthConstraint {
                                            max: 3.,
                                            expand: true,
                                        },
                                        first_height,
                                        full_height: breakable.as_ref().map(|b| b.full_height),
                                    });
                                }

                                r.push(Pass::Draw {
                                    width: WidthConstraint {
                                        max: 3.,
                                        expand: true,
                                    },
                                    first_height,
                                    preferred_height,

                                    page: 0,
                                    layer: 0,
                                    pos: (x, 14.),

                                    breakable: breakable.as_ref().map(|b| BreakableDraw {
                                        full_height: b.full_height,
                                        preferred_height_break_count: b
                                            .preferred_height_break_count,
                                        breaks: if less_first_height {
                                            vec![
                                                Break {
                                                    page: 1,
                                                    layer: 0,
                                                    pos: (x, 14.),
                                                },
                                                Break {
                                                    page: 2,
                                                    layer: 0,
                                                    pos: (x, 14.),
                                                },
                                            ]
                                        } else {
                                            vec![Break {
                                                page: 1,
                                                layer: 0,
                                                pos: (x, 14.),
                                            }]
                                        },
                                    }),
                                });

                                r
                            }
                        },
                    )
                };

                let child_2_width = 3. + gap;

                let child_3 = {
                    let x = 12. + child_1_width + child_2_width;
                    let width = WidthConstraint {
                        max: 13.,
                        expand: width.expand,
                    };

                    AssertPasses::new(
                        ForceBreak,
                        match pass {
                            build_element::Pass::FirstLocationUsage { .. } => todo!(),
                            build_element::Pass::Measure { full_height } => vec![Pass::Measure {
                                width,
                                first_height,
                                full_height,
                            }],
                            build_element::Pass::Draw {
                                ref breakable,
                                preferred_height,
                            } => {
                                let mut r = Vec::new();

                                if expand {
                                    r.push(Pass::Measure {
                                        width,
                                        first_height,
                                        full_height: breakable.as_ref().map(|b| b.full_height),
                                    });
                                }

                                r.push(Pass::Draw {
                                    width,
                                    first_height,
                                    preferred_height,

                                    page: 0,
                                    layer: 0,
                                    pos: (x, 14.),

                                    breakable: breakable.as_ref().map(|b| BreakableDraw {
                                        full_height: b.full_height,
                                        preferred_height_break_count: b
                                            .preferred_height_break_count,
                                        breaks: vec![Break {
                                            page: 1,
                                            layer: 0,
                                            pos: (x, 14.),
                                        }],
                                    }),
                                });

                                r
                            }
                        },
                    )
                };

                let element = Row {
                    gap,
                    expand,
                    collapse: false,
                    content: |content| {
                        content.add(&child_0, Flex::SelfSized);
                        content.add(&child_1, Flex::Expand(1));
                        content.add(&child_2, Flex::Fixed(3.));
                        content.add(&child_3, Flex::Expand(2));
                    },
                };

                callback.call(element)
            },
        );

        for output in (ElementTestParams {
            width: 24.,
            first_height: 6.,
            full_height: 12.,
            pos: (12., 14.),
            ..Default::default()
        })
        .run(&element)
        {
            let (height, breaks) = if output.breakable.is_none() {
                (24., 0)
            } else if output.first_height == 6. {
                (6., 2)
            } else {
                (12., 1)
            };

            output.assert_size(ElementSize {
                width: if output.width.expand {
                    Some(20. + gap + 3.)
                } else {
                    Some(5. + gap + 3.)
                },
                height: Some(height),
            });

            if let Some(b) = output.breakable {
                b.assert_break_count(breaks);

                // TODO
                b.assert_extra_location_min_height(None);
            }
        }
    }
}
