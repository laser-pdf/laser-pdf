use crate::{
    utils::{mm_to_pt, u32_to_color_and_alpha},
    *,
};

pub struct StyledBox<'a, E: Element> {
    pub element: &'a E,
    pub padding_left: f64,
    pub padding_right: f64,
    pub padding_top: f64,
    pub padding_bottom: f64,
    pub border_radius: f64,
    pub fill: Option<u32>,
    pub outline: Option<LineStyle>,
}

impl<'a, E: Element> StyledBox<'a, E> {
    pub fn new(element: &'a E) -> Self {
        StyledBox {
            element,
            padding_top: 0.,
            padding_bottom: 0.,
            padding_left: 0.,
            padding_right: 0.,
            border_radius: 0.,
            fill: None,
            outline: None,
        }
    }
}

impl<'a, E: Element> Element for StyledBox<'a, E> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        let common = self.common(ctx.width);
        let first_height = common.height(ctx.first_height);
        let full_height = common.height(ctx.full_height);

        self.element.first_location_usage(FirstLocationUsageCtx {
            width: common.width,
            first_height,
            full_height,
        })
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        let common = self.common(ctx.width);
        let first_height = common.height(ctx.first_height);

        let size = if let Some(breakable) = ctx.breakable {
            let full_height = common.height(breakable.full_height);

            let first_location_usage = self.element.first_location_usage(FirstLocationUsageCtx {
                width: common.width,
                first_height,
                full_height,
            });

            let element_first_height;

            if first_location_usage == FirstLocationUsage::WillSkip {
                *breakable.break_count = 1;
                element_first_height = full_height;
            } else {
                element_first_height = first_height;
            }

            let mut break_count = 0;
            let mut extra_location_min_height = None;

            let size = self.element.measure(MeasureCtx {
                width: common.width,
                first_height: element_first_height,
                breakable: Some(BreakableMeasure {
                    full_height,
                    break_count: &mut break_count,
                    extra_location_min_height: &mut extra_location_min_height,
                }),
            });

            *breakable.break_count += break_count;
            *breakable.extra_location_min_height =
                extra_location_min_height.map(|x| x + self.padding_top + self.padding_bottom);

            size
        } else {
            self.element.measure(MeasureCtx {
                width: common.width,
                first_height,
                breakable: None,
            })
        };

        common.size(size)
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        let common = self.common(ctx.width);
        let first_height = common.height(ctx.first_height);

        let size = if let Some(breakable) = ctx.breakable {
            let full_height = common.height(breakable.full_height);

            let first_location_usage = self.element.first_location_usage(FirstLocationUsageCtx {
                width: common.width,
                first_height,
                full_height,
            });

            let element_first_height;
            let box_first_height;
            let location_offset;
            let location;

            if first_location_usage == FirstLocationUsage::WillSkip {
                location = (breakable.get_location)(ctx.pdf, 0);
                location_offset = 1;
                element_first_height = full_height;
                box_first_height = breakable.full_height;
            } else {
                location = ctx.location;
                location_offset = 0;
                element_first_height = first_height;
                box_first_height = ctx.first_height;
            }

            let mut break_count = 0;

            let element_location = common.location(ctx.pdf, location.clone());
            let size = self.element.draw(DrawCtx {
                pdf: ctx.pdf,
                location: element_location,
                width: common.width,
                first_height: element_first_height,
                preferred_height: if location_offset == 1
                    && breakable.preferred_height_break_count == 0
                {
                    None
                } else {
                    ctx.preferred_height.map(|p| common.height(p))
                },
                breakable: Some(BreakableDraw {
                    full_height,
                    preferred_height_break_count: breakable
                        .preferred_height_break_count
                        .saturating_sub(location_offset),
                    get_location: &mut |pdf, location_idx| {
                        break_count = break_count.max(location_idx + 1);
                        let location =
                            (breakable.get_location)(pdf, location_idx + location_offset);

                        common.location(pdf, location)
                    },
                }),
            });

            if let Some(width) = size.width {
                if break_count > 0 || size.height.is_some() {
                    let width = width + common.left + common.right;

                    self.draw_box(&location, (width, box_first_height));

                    for i in 0..break_count - if size.height.is_none() { 1 } else { 0 } {
                        let location = (breakable.get_location)(ctx.pdf, i + location_offset);
                        self.draw_box(
                            &location,
                            (
                                width,
                                if i == break_count - 1 {
                                    size.height.unwrap()
                                } else {
                                    breakable.full_height
                                },
                            ),
                        );
                    }
                }
            }

            size
        } else {
            let location = common.location(ctx.pdf, ctx.location.clone());

            let size = self.element.draw(DrawCtx {
                pdf: ctx.pdf,
                location,
                preferred_height: ctx.preferred_height.map(|p| common.height(p)),
                width: common.width,
                first_height,
                breakable: None,
            });

            if let ElementSize {
                width: Some(width),
                height: Some(height),
            } = size
            {
                self.draw_box(&ctx.location, (width, height));
            }

            size
        };

        common.size(size)
    }
}

struct Common {
    top: f64,
    bottom: f64,
    left: f64,
    right: f64,

    width: WidthConstraint,
}

impl Common {
    fn location(&self, pdf: &mut Pdf, location: Location) -> Location {
        Location {
            pos: (location.pos.0 + self.left, location.pos.1 - self.top),
            layer: location.next_layer(pdf),
        }
    }

    fn size(&self, size: ElementSize) -> ElementSize {
        ElementSize {
            width: size.width.map(|w| w + self.left + self.right),
            height: size.height.map(|h| h + self.top + self.bottom),
        }
    }

    fn height(&self, input: f64) -> f64 {
        input - self.top - self.bottom
    }
}

impl<'a, E: Element> StyledBox<'a, E> {
    fn common(&self, width: WidthConstraint) -> Common {
        let outline_thickness = self.outline.map(|o| o.thickness).unwrap_or(0.0);
        let extra_outline_offset = outline_thickness / 2.0;

        let top = self.padding_top + extra_outline_offset;
        let bottom = self.padding_bottom + extra_outline_offset;
        let left = self.padding_left + extra_outline_offset;
        let right = self.padding_right + extra_outline_offset;

        let width = WidthConstraint {
            max: width.max - left - right,
            expand: width.expand,
        };

        Common {
            top,
            bottom,
            left,
            right,
            width,
        }
    }

    fn draw_box(&self, location: &Location, size: (f64, f64)) {
        use kurbo::{PathEl, RoundedRect, Shape};
        use lopdf::content::Operation;
        use printpdf::LineDashPattern;

        let size = (
            size.0 + self.padding_left + self.padding_right,
            size.1 + self.padding_top + self.padding_bottom,
        );

        let shape = RoundedRect::new(
            mm_to_pt(location.pos.0),
            mm_to_pt(location.pos.1),
            mm_to_pt(location.pos.0 + size.0),
            mm_to_pt(location.pos.1 - size.1),
            mm_to_pt(self.border_radius),
        );

        let layer = &location.layer;

        layer.save_graphics_state();

        if let Some(color) = self.fill {
            let (color, alpha) = u32_to_color_and_alpha(color);
            layer.set_fill_color(color);
            layer.set_fill_alpha(alpha);
        }

        if let Some(line_style) = self.outline {
            // No outline alpha?
            let (color, _alpha) = u32_to_color_and_alpha(line_style.color);
            layer.set_outline_color(color);
            layer.set_outline_thickness(mm_to_pt(line_style.thickness));
            layer.set_line_cap_style(line_style.cap_style.into());
            layer.set_line_dash_pattern(if let Some(pattern) = line_style.dash_pattern {
                pattern.into()
            } else {
                LineDashPattern::default()
            });
        }

        let els = shape.path_elements(0.1);

        let mut closed = false;

        for el in els {
            use PathEl::*;

            match el {
                MoveTo(point) => {
                    layer.add_op(Operation::new("m", vec![point.x.into(), point.y.into()]))
                }
                LineTo(point) => {
                    layer.add_op(Operation::new("l", vec![point.x.into(), point.y.into()]))
                }
                QuadTo(a, b) => layer.add_op(
                    // i dunno
                    Operation::new("v", vec![a.x.into(), a.y.into(), b.x.into(), b.y.into()]),
                ),
                CurveTo(a, b, c) => layer.add_op(Operation::new(
                    "c",
                    vec![
                        a.x.into(),
                        a.y.into(),
                        b.x.into(),
                        b.y.into(),
                        c.x.into(),
                        c.y.into(),
                    ],
                )),
                ClosePath => closed = true,
            };
        }

        match (self.outline.is_some(), self.fill.is_some(), closed) {
            (true, true, true) => layer.add_op(Operation::new("b", Vec::new())),
            (true, true, false) => layer.add_op(Operation::new("f", Vec::new())),
            (true, false, true) => layer.add_op(Operation::new("s", Vec::new())),
            (true, false, false) => layer.add_op(Operation::new("S", Vec::new())),
            (false, true, _) => layer.add_op(Operation::new("f", Vec::new())),
            _ => layer.add_op(Operation::new("n", Vec::new())),
        }

        location.layer.restore_graphics_state();
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
            max: 7.,
            expand: false,
        };
        let first_height = 30.;
        let pos = (2., 10.);

        let output = test_element(
            TestElementParams {
                width,
                first_height,
                breakable: None,
                pos,
                ..Default::default()
            },
            |assert, callback| {
                let content = RecordPasses::new(FakeText {
                    lines: 3,
                    line_height: 5.,
                    width: 3.,
                });

                let element = StyledBox {
                    padding_left: 1.,
                    padding_right: 2.,
                    padding_top: 3.,
                    padding_bottom: 4.,

                    ..StyledBox::new(&content)
                };

                let ret = callback.call(element);

                if assert {
                    content.assert_first_location_usage_count(0);
                    content.assert_measure_count(0);
                    content.assert_draw(DrawPass {
                        width: WidthConstraint {
                            max: 4.,
                            expand: false,
                        },
                        first_height: 23.,
                        preferred_height: None,
                        page: 0,
                        layer: 1,
                        pos: (3., 7.),
                        breakable: None,
                    });
                }

                ret
            },
        );

        output.assert_size(ElementSize {
            width: Some(6.),
            height: Some(22.),
        });
    }

    #[test]
    fn test_pre_break() {
        let width = WidthConstraint {
            max: 7.,
            expand: false,
        };
        let first_height = 9.;
        let full_height = 18.;
        let pos = (2., 18.);

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
                let content = RecordPasses::new(FakeText {
                    lines: 3,
                    line_height: 5.,
                    width: 3.,
                });

                let element = StyledBox {
                    padding_left: 1.,
                    padding_right: 2.,
                    padding_top: 3.,
                    padding_bottom: 4.,

                    ..StyledBox::new(&content)
                };

                let ret = callback.call(element);

                if assert {
                    content.assert_first_location_usage_count(1);
                    content.assert_measure_count(0);
                    content.assert_draw(DrawPass {
                        width: WidthConstraint {
                            max: 4.,
                            expand: false,
                        },
                        first_height: 11.,
                        preferred_height: None,
                        page: 1,
                        layer: 1,
                        pos: (3., 15.),
                        breakable: Some(BreakableDraw {
                            full_height: 11.,
                            preferred_height_break_count: 0,
                            breaks: vec![Break {
                                page: 2,
                                layer: 1,
                                pos: (3., 15.),
                            }],
                        }),
                    });
                }

                ret
            },
        );

        output.assert_size(ElementSize {
            width: Some(6.),
            height: Some(12.),
        });

        output
            .breakable
            .unwrap()
            .assert_first_location_usage(FirstLocationUsage::WillSkip)
            .assert_break_count(2)
            .assert_extra_location_min_height(None);
    }
}
