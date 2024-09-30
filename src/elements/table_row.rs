use crate::{
    flex::{DrawLayout, MeasureLayout},
    utils::{max_optional_size, mm_to_pt, u32_to_color_and_alpha},
    *,
};

/// Currently almost a copy of Row, with the difference that there is no self sized and there's
/// lines instead of gaps. The plan is to eventually replace this with a custom element instead of
/// a gap in Row.
pub struct TableRow<F: Fn(&mut RowContent)> {
    pub line_style: LineStyle,
    pub expand: bool,
    pub content: F,
}

impl<F: Fn(&mut RowContent)> Element for TableRow<F> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        FirstLocationUsage::WillUse
    }

    fn measure(&self, mut ctx: MeasureCtx) -> ElementSize {
        let mut measure_layout = MeasureLayout::new(ctx.width.max, self.line_style.thickness);

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
                gap: self.line_style.thickness,
                breakable: ctx.breakable.as_mut(),
            },
        });

        ElementSize {
            width: if ctx.width.expand {
                Some(ctx.width.max)
            } else {
                width
            },
            height: max_height,
        }
    }

    fn draw(&self, mut ctx: DrawCtx) -> ElementSize {
        let mut measure_layout = MeasureLayout::new(ctx.width.max, self.line_style.thickness);

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
                    width: None, // We'll get that from draw. No point in getting it twice.
                    gap: self.line_style.thickness,
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
        let mut break_count = 0;

        (self.content)(&mut RowContent {
            width: ctx.width,
            first_height: ctx.first_height,
            pass: Pass::Draw {
                layout: &draw_layout,
                max_height: &mut max_height,
                width: &mut width,
                gap: self.line_style.thickness,
                pdf: ctx.pdf,
                location: ctx.location.clone(),
                preferred_height: ctx.preferred_height,
                break_count: &mut break_count,
                breakable: ctx.breakable.as_mut(),
            },
        });

        if let Some(height) = max_height {
            (self.content)(&mut RowContent {
                width: ctx.width,
                first_height: ctx.first_height,
                pass: Pass::DrawLines {
                    layout: &draw_layout,
                    width: None,
                    height,
                    line_style: self.line_style,
                    pdf: ctx.pdf,
                    location: ctx.location,
                    break_count,
                    breakable: ctx.breakable.as_mut(),
                },
            });
        }

        ElementSize {
            width: if ctx.width.expand {
                Some(ctx.width.max)
            } else {
                width
            },
            height: max_height,
        }
    }
}

pub struct RowContent<'a, 'b, 'c> {
    width: WidthConstraint,
    first_height: f64,
    pass: Pass<'a, 'b, 'c>,
}

enum Pass<'a, 'b, 'c> {
    MeasureNonExpanded {
        layout: &'a mut MeasureLayout,
        max_height: Option<&'a mut Option<f64>>,
        breakable: Option<&'a mut BreakableMeasure<'b>>,
    },

    FirstLocationUsage {},

    MeasureExpanded {
        layout: &'a DrawLayout,
        max_height: &'a mut Option<f64>,
        width: Option<&'a mut Option<f64>>,
        gap: f64,
        breakable: Option<&'a mut BreakableMeasure<'b>>,
    },

    Draw {
        layout: &'a DrawLayout,
        max_height: &'a mut Option<f64>,
        width: &'a mut Option<f64>,

        gap: f64,

        pdf: &'c mut Pdf,
        location: Location,

        preferred_height: Option<f64>,
        break_count: &'a mut u32,
        breakable: Option<&'a mut BreakableDraw<'b>>,
    },

    DrawLines {
        layout: &'a DrawLayout,
        height: f64,
        width: Option<f64>,
        break_count: u32,

        line_style: LineStyle,
        pdf: &'c mut Pdf,
        location: Location,
        breakable: Option<&'a mut BreakableDraw<'b>>,
    },
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum Flex {
    Expand(u8),
    Fixed(f64),
}

fn add_height(
    max_height: &mut Option<f64>,
    breakable: Option<&mut BreakableMeasure>,
    size: ElementSize,
    break_count: u32,
    extra_location_min_height: Option<f64>,
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
                            expand: true,
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
                Flex::Fixed(_) => (),
            },

            Pass::Draw {
                layout,
                max_height: &mut ref mut max_height,
                width: &mut ref mut width,
                gap,
                pdf: &mut ref mut pdf,
                ref location,
                preferred_height,
                break_count: &mut ref mut break_count,
                ref mut breakable,
            } => {
                let width_constraint = match flex {
                    Flex::Expand(fraction) => WidthConstraint {
                        max: layout.expand_width(fraction),
                        expand: true,
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

                width_add(width_constraint.max);
            }

            Pass::DrawLines {
                layout,
                height,
                ref mut width,
                line_style,
                pdf: &mut ref mut pdf,
                ref location,
                break_count,
                ref mut breakable,
            } => {
                let element_width = match flex {
                    Flex::Expand(fraction) => layout.expand_width(fraction),
                    Flex::Fixed(width) => width,
                };

                if let Some(width) = width {
                    let draw_line = |location: &Location, height: f64| {
                        let x = location.pos.0 + *width;
                        let y = location.pos.1;

                        location.layer.save_graphics_state();
                        let layer = &location.layer;

                        let (color, _alpha) = u32_to_color_and_alpha(line_style.color);
                        layer.set_outline_color(color);
                        layer.set_outline_thickness(mm_to_pt(line_style.thickness));
                        layer.set_line_cap_style(line_style.cap_style.into());
                        layer.set_line_dash_pattern(
                            if let Some(pattern) = line_style.dash_pattern {
                                pattern.into()
                            } else {
                                printpdf::LineDashPattern::default()
                            },
                        );

                        let line_x = x + line_style.thickness / 2.;

                        location.layer.add_shape(printpdf::Line {
                            points: vec![
                                (printpdf::Point::new(Mm(line_x), Mm(y)), false),
                                (printpdf::Point::new(Mm(line_x), Mm(y - height)), false),
                            ],
                            is_closed: false,
                            has_fill: false,
                            has_stroke: true,
                            is_clipping_path: false,
                        });

                        location.layer.restore_graphics_state();
                    };

                    match breakable {
                        Some(breakable) if break_count > 0 => {
                            draw_line(location, self.first_height);

                            for i in 0..break_count {
                                let location = (breakable.do_break)(
                                    pdf,
                                    i,
                                    Some(if i == 0 {
                                        self.first_height
                                    } else {
                                        breakable.full_height
                                    }),
                                );
                                draw_line(
                                    &location,
                                    if i == break_count - 1 {
                                        height
                                    } else {
                                        breakable.full_height
                                    },
                                );
                            }
                        }
                        _ => {
                            draw_line(location, height);
                        }
                    }

                    *width += line_style.thickness + element_width;
                } else {
                    *width = Some(element_width);
                }
            }

            _ => todo!(),
        }
    }
}
