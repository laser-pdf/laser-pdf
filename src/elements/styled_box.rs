use crate::{
    utils::{mm_to_pt, u32_to_color_and_alpha},
    *,
};

pub struct StyledBox<'a, E: Element> {
    pub element: &'a E,
    pub padding_left: f32,
    pub padding_right: f32,
    pub padding_top: f32,
    pub padding_bottom: f32,
    pub border_radius: f32,
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

struct Common {
    top: f32,
    bottom: f32,
    left: f32,
    right: f32,

    inner_width_constraint: WidthConstraint,
    width: Option<f32>,
}

impl Common {
    fn location(&self, pdf: &mut Pdf, location: &Location) -> Location {
        Location {
            pos: (location.pos.0 + self.left, location.pos.1 - self.top),
            ..location.next_layer(pdf)
        }
    }

    fn height(&self, input: f32) -> f32 {
        input - self.top - self.bottom
    }
}

impl<'a, E: Element> StyledBox<'a, E> {
    fn common(&self, width: WidthConstraint) -> Common {
        let extra_outline_offset = self.outline.map(|o| o.thickness).unwrap_or(0.0);

        let top = self.padding_top + extra_outline_offset;
        let bottom = self.padding_bottom + extra_outline_offset;
        let left = self.padding_left + extra_outline_offset;
        let right = self.padding_right + extra_outline_offset;

        let inner_width_constraint = WidthConstraint {
            max: width.max - left - right,
            expand: width.expand,
        };

        let width = width.expand.then_some(inner_width_constraint.max);

        Common {
            top,
            bottom,
            left,
            right,
            inner_width_constraint,
            width,
        }
    }

    fn size(&self, common: &Common, size: ElementSize) -> ElementSize {
        ElementSize {
            width: common
                .width
                .or(size.width)
                .map(|w| w + common.left + common.right),
            height: size.height.map(|h| h + common.top + common.bottom),
        }
    }

    fn draw_box(&self, pdf: &mut Pdf, location: &Location, size: (f32, f32)) {
        use kurbo::{PathEl, RoundedRect, Shape};

        let size = (
            size.0 + self.padding_left + self.padding_right,
            size.1 + self.padding_top + self.padding_bottom,
        );

        let thickness = self.outline.map(|o| o.thickness).unwrap_or(0.);
        let half_thickness = thickness / 2.;

        let fill_alpha = self
            .fill
            .map(|c| u32_to_color_and_alpha(c).1)
            .filter(|&a| a != 1.);

        let outline_alpha = self
            .outline
            .map(|o| u32_to_color_and_alpha(o.color).1)
            .filter(|&a| a != 1.);

        location.layer(pdf).save_state();

        if fill_alpha.is_some() || outline_alpha.is_some() {
            let ext_graphics_ref = pdf.alloc();

            let mut ext_graphics = pdf.pdf.ext_graphics(ext_graphics_ref);
            fill_alpha.inspect(|&a| {
                ext_graphics.non_stroking_alpha(a);
            });
            outline_alpha.inspect(|&a| {
                ext_graphics.stroking_alpha(a);
            });

            let resource_id = pdf.pages[location.page_idx].add_ext_g_state(ext_graphics_ref);
            drop(ext_graphics);
            location
                .layer(pdf)
                .set_parameters(Name(format!("{}", resource_id).as_bytes()));
        }

        let layer = location.layer(pdf);

        if let Some(color) = self.fill {
            let (color, _) = u32_to_color_and_alpha(color);

            layer.set_fill_rgb(color[0], color[1], color[2]);
        }

        if let Some(line_style) = self.outline {
            let (color, _) = u32_to_color_and_alpha(line_style.color);

            layer
                .set_line_width(mm_to_pt(thickness as f32))
                .set_stroke_rgb(color[0], color[1], color[2])
                .set_line_cap(line_style.cap_style.into());

            if let Some(pattern) = line_style.dash_pattern {
                layer.set_dash_pattern(pattern.dashes.map(f32::from), pattern.offset as f32);
            }
        }

        let shape = RoundedRect::new(
            mm_to_pt(location.pos.0 + half_thickness) as f64,
            mm_to_pt(location.pos.1 - half_thickness) as f64,
            mm_to_pt(location.pos.0 + size.0 + thickness + half_thickness) as f64,
            mm_to_pt(location.pos.1 - size.1 - thickness - half_thickness) as f64,
            mm_to_pt(self.border_radius) as f64,
        );

        let els = shape.path_elements(0.1);

        let mut closed = false;

        for el in els {
            use PathEl::*;

            match el {
                MoveTo(point) => {
                    layer.move_to(point.x as f32, point.y as f32);
                }
                LineTo(point) => {
                    layer.line_to(point.x as f32, point.y as f32);
                }
                QuadTo(a, b) => {
                    layer.cubic_to_initial(a.x as f32, a.y as f32, b.x as f32, b.y as f32);
                }
                CurveTo(a, b, c) => {
                    layer.cubic_to(
                        a.x as f32, a.y as f32, b.x as f32, b.y as f32, c.x as f32, c.y as f32,
                    );
                }
                ClosePath => closed = true,
            };
        }

        match (self.outline.is_some(), self.fill.is_some(), closed) {
            (true, true, true) => layer.close_fill_nonzero_and_stroke(),
            (true, true, false) => layer.fill_nonzero(),
            (true, false, true) => layer.close_and_stroke(),
            (true, false, false) => layer.stroke(),
            (false, true, _) => layer.fill_nonzero(),
            _ => layer.end_path(),
        };

        layer.restore_state();
    }
}

impl<'a, E: Element> Element for StyledBox<'a, E> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        let common = self.common(ctx.width);
        let first_height = common.height(ctx.first_height);
        let full_height = common.height(ctx.full_height);

        self.element.first_location_usage(FirstLocationUsageCtx {
            width: common.inner_width_constraint,
            first_height,
            full_height,
        })
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        let common = self.common(ctx.width);
        let first_height = common.height(ctx.first_height);

        let size = if let Some(breakable) = ctx.breakable {
            let full_height = common.height(breakable.full_height);

            let size = self.element.measure(MeasureCtx {
                width: common.inner_width_constraint,
                first_height,
                breakable: Some(BreakableMeasure {
                    full_height,
                    break_count: breakable.break_count,
                    extra_location_min_height: breakable.extra_location_min_height,
                }),
            });

            *breakable.extra_location_min_height = breakable
                .extra_location_min_height
                .map(|x| x + self.padding_top + self.padding_bottom);

            size
        } else {
            self.element.measure(MeasureCtx {
                width: common.inner_width_constraint,
                first_height,
                breakable: None,
            })
        };

        self.size(&common, size)
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        let common = self.common(ctx.width);
        let first_height = common.height(ctx.first_height);

        let size = if let Some(breakable) = ctx.breakable {
            let full_height = common.height(breakable.full_height);

            let mut break_count = 0;

            let width = if ctx.width.expand {
                Some(ctx.width.max - common.left - common.right)
            } else {
                let mut break_count = 0;
                let mut extra_location_min_height = None;

                self.element
                    .measure(MeasureCtx {
                        width: common.inner_width_constraint,
                        first_height,
                        breakable: Some(BreakableMeasure {
                            full_height,
                            break_count: &mut break_count,
                            extra_location_min_height: &mut extra_location_min_height,
                        }),
                    })
                    .width
            };

            let element_location = common.location(ctx.pdf, &ctx.location);
            let mut last_location = ctx.location;
            let size = self.element.draw(DrawCtx {
                pdf: ctx.pdf,
                location: element_location,
                width: common.inner_width_constraint,
                first_height,
                preferred_height: ctx.preferred_height.map(|p| common.height(p)),
                breakable: Some(BreakableDraw {
                    full_height,
                    preferred_height_break_count: breakable.preferred_height_break_count,
                    do_break: &mut |pdf, location_idx, height| {
                        let location = (breakable.do_break)(
                            pdf,
                            location_idx,
                            height.map(|h| h + common.top + common.bottom),
                        );

                        match (width, height) {
                            (Some(width), Some(height)) if location_idx >= break_count => {
                                let location = if location_idx == break_count {
                                    &last_location
                                } else {
                                    &(breakable.do_break)(pdf, location_idx, None)
                                };

                                self.draw_box(pdf, location, (width, height));
                            }
                            _ => (),
                        }

                        break_count = break_count.max(location_idx + 1);

                        let ret = common.location(pdf, &location);
                        last_location = location;
                        ret
                    },
                }),
            });

            if let (Some(width), Some(height)) = (width, size.height) {
                self.draw_box(ctx.pdf, &last_location, (width, height));
            }

            size
        } else {
            let location = common.location(ctx.pdf, &ctx.location);

            let size = self.element.draw(DrawCtx {
                pdf: ctx.pdf,
                location,
                preferred_height: ctx.preferred_height.map(|p| common.height(p)),
                width: common.inner_width_constraint,
                first_height,
                breakable: None,
            });

            if let ElementSize {
                width: Some(width),
                height: Some(height),
            } = size
            {
                self.draw_box(ctx.pdf, &ctx.location, (width, height));
            }

            size
        };

        self.size(&common, size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        elements::{none::NoneElement, rectangle::Rectangle, text::Text},
        fonts::builtin::BuiltinFont,
        test_utils::{
            record_passes::{Break, BreakableDraw, DrawPass, RecordPasses},
            *,
        },
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
                    content.assert_first_location_usage_count(0);
                    content.assert_measure_count(1);
                    content.assert_draw(DrawPass {
                        width: WidthConstraint {
                            max: 4.,
                            expand: false,
                        },
                        first_height: 2.,
                        preferred_height: None,
                        page: 0,
                        layer: 1,
                        pos: (3., 6.),
                        breakable: Some(BreakableDraw {
                            full_height: 11.,
                            preferred_height_break_count: 0,
                            breaks: vec![
                                // we don't actually pre-break anymore
                                Break {
                                    page: 1,
                                    layer: 1,
                                    pos: (3., 15.),
                                },
                                Break {
                                    page: 2,
                                    layer: 1,
                                    pos: (3., 15.),
                                },
                            ],
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

    #[test]
    fn test_x_size() {
        use crate::test_utils::binary_snapshots::*;
        use insta::*;

        let bytes = test_element_bytes(TestElementParams::breakable(), |callback| {
            // let font = BuiltinFont::courier(callback.document());

            // // let first = Text::basic("test", &font, 12.);
            let first = Rectangle {
                size: (12., 12.),
                fill: Some(0x00_00_77_FF),
                outline: Some((2., 0x00_00_00_FF)),
            };
            let first = first.debug(1).show_max_width();

            callback.call(
                &StyledBox {
                    element: &first,
                    padding_left: 1.,
                    padding_right: 2.,
                    padding_top: 3.,
                    padding_bottom: 4.,
                    border_radius: 1.,
                    fill: None,
                    outline: Some(LineStyle {
                        thickness: 1.,
                        color: 0x00_00_00_FF,
                        dash_pattern: None,
                        cap_style: LineCapStyle::Butt,
                    }),
                }
                .debug(0)
                .show_max_width()
                .show_last_location_max_height(),
            );
        });
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_border_sizing() {
        use crate::test_utils::binary_snapshots::*;
        use insta::*;

        let bytes = test_element_bytes(TestElementParams::breakable(), |callback| {
            let first = Rectangle {
                size: (12., 12.),
                fill: Some(0x00_00_77_FF),
                outline: None,
            };
            let first = first.debug(1).show_max_width();

            callback.call(
                &StyledBox {
                    outline: Some(LineStyle {
                        thickness: 32.,
                        color: 0x00_00_00_FF,
                        dash_pattern: None,
                        cap_style: LineCapStyle::Butt,
                    }),
                    ..StyledBox::new(&first)
                }
                .debug(0)
                .show_max_width()
                .show_last_location_max_height(),
            );
        });
        assert_binary_snapshot!(".pdf", bytes);
    }
}
