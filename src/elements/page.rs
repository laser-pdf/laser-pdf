use crate::*;

pub struct Page<P: Element, D: Fn(&mut DecorationElements, usize, usize)> {
    pub primary: P,
    pub border_left: f32,
    pub border_right: f32,
    pub border_top: f32,
    pub border_bottom: f32,
    pub decoration_elements: D,
}

impl<P: Element, D: Fn(&mut DecorationElements, usize, usize)> Element for Page<P, D> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        if ctx.first_height < ctx.full_height {
            FirstLocationUsage::WillSkip
        } else {
            FirstLocationUsage::WillUse
        }
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        if let Some(breakable) = ctx.breakable {
            let mut extra_location_min_height = None;
            let mut break_count = 0;

            let primary_height = self.height(breakable.full_height);

            self.primary.measure(MeasureCtx {
                width: WidthConstraint {
                    max: self.width(ctx.width),
                    expand: true,
                },
                first_height: primary_height,
                breakable: Some(BreakableMeasure {
                    full_height: primary_height,
                    break_count: &mut break_count,
                    extra_location_min_height: &mut extra_location_min_height,
                }),
            });

            if ctx.first_height < breakable.full_height {
                break_count += 1;
            }

            *breakable.break_count = break_count;

            ElementSize {
                width: Some(ctx.width.max),
                height: Some(breakable.full_height),
            }
        } else {
            ElementSize {
                width: Some(ctx.width.max),
                height: Some(ctx.first_height),
            }
        }
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        let primary_width = WidthConstraint {
            max: self.width(ctx.width),
            expand: true,
        };

        let mut breakable = ctx.breakable;

        let height = breakable
            .as_ref()
            .map(|b| b.full_height)
            .unwrap_or(ctx.first_height);

        let primary_height = self.height(height);

        let location;
        let location_offset;

        match breakable {
            Some(ref mut breakable) if ctx.first_height < breakable.full_height => {
                location = (breakable.do_break)(ctx.pdf, 0, None);
                location_offset = 1;
            }
            _ => {
                location = ctx.location;
                location_offset = 0;
            }
        }

        let mut break_count = 0;

        // We put the primary content one layer up so the decoration elements can be used for things
        // like watermarks.
        let primary_location = location.next_layer(ctx.pdf);

        self.primary.draw(DrawCtx {
            pdf: ctx.pdf,
            location: Location {
                pos: (
                    location.pos.0 + self.border_left,
                    location.pos.1 - self.border_top,
                ),
                ..primary_location
            },
            width: primary_width,
            first_height: primary_height,
            preferred_height: None,
            breakable: breakable
                .as_mut()
                .map(|breakable| {
                    |pdf: &mut Pdf, location_idx: u32, _| {
                        break_count = break_count.max(location_idx + 1);
                        let mut location = (breakable.do_break)(
                            pdf,
                            location_idx + location_offset,
                            Some(breakable.full_height),
                        );

                        location = location.next_layer(pdf);
                        location.pos.0 += self.border_left;
                        location.pos.1 -= self.border_top;

                        location
                    }
                })
                .as_mut()
                .map(|get_location| BreakableDraw {
                    full_height: primary_height,
                    preferred_height_break_count: 0,
                    do_break: get_location,
                }),
        });

        if let Some(breakable) = breakable {
            for i in 0..=break_count {
                let location = if i == 0 {
                    location.clone()
                } else {
                    (breakable.do_break)(
                        ctx.pdf,
                        i + location_offset - 1,
                        Some(breakable.full_height),
                    )
                };

                (self.decoration_elements)(
                    &mut DecorationElements {
                        pdf: ctx.pdf,
                        location,
                        width: ctx.width.max,
                        height,
                    },
                    i as usize,
                    (break_count + 1) as usize,
                );
            }
        } else {
            (self.decoration_elements)(
                &mut DecorationElements {
                    pdf: ctx.pdf,
                    location,
                    width: ctx.width.max,
                    height,
                },
                0,
                1,
            );
        }

        ElementSize {
            width: Some(ctx.width.max),
            height: Some(height),
        }
    }
}

impl<P: Element, D: Fn(&mut DecorationElements, usize, usize)> Page<P, D> {
    fn width(&self, width: WidthConstraint) -> f32 {
        width.max - self.border_left - self.border_right
    }

    fn height(&self, full_height: f32) -> f32 {
        full_height - self.border_top - self.border_bottom
    }
}

pub struct DecorationElements<'a> {
    pdf: &'a mut Pdf,
    location: Location,
    width: f32,
    height: f32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum X {
    Left(f32),
    Right(f32),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Y {
    Top(f32),
    Bottom(f32),
}

impl<'a> DecorationElements<'a> {
    pub fn add(&mut self, element: &impl Element, pos: (X, Y), width: Option<f32>) {
        element.draw(DrawCtx {
            pdf: self.pdf,
            location: Location {
                pos: (
                    match pos.0 {
                        X::Left(left) => self.location.pos.0 + left,
                        X::Right(right) => self.location.pos.0 + self.width - right,
                    },
                    match pos.1 {
                        Y::Top(top) => self.location.pos.1 - top,
                        Y::Bottom(bottom) => self.location.pos.1 - self.height + bottom,
                    },
                ),
                ..self.location
            },
            width: WidthConstraint {
                max: width.unwrap_or_else(|| match pos.0 {
                    X::Left(left) => self.width - left,
                    X::Right(right) => right,
                }),
                expand: width.is_some(),
            },
            first_height: match pos.1 {
                Y::Top(top) => self.height - top,
                Y::Bottom(bottom) => bottom,
            },
            preferred_height: None,
            breakable: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;

    use super::*;
    use crate::{
        elements::ref_element::RefElement,
        test_utils::{record_passes::RecordPasses, *},
    };
    use X::*;
    use Y::*;

    #[test]
    fn test_unbreakable() {
        let output = test_element(
            TestElementParams {
                width: WidthConstraint {
                    max: 10.,
                    expand: false,
                },
                first_height: 20.,
                breakable: None,
                pos: (10., 30.0),
                ..Default::default()
            },
            |assert, callback| {
                let primary = RecordPasses::new(FakeText {
                    lines: 1,
                    line_height: 5.,
                    width: 3.,
                });

                let top_left = RecordPasses::new(FakeText {
                    lines: 1,
                    line_height: 5.,
                    width: 6.,
                });

                let bottom_right = RecordPasses::new(FakeText {
                    lines: 1,
                    line_height: 4.,
                    width: 3.,
                });

                let element = Page {
                    primary: RefElement(&primary),
                    border_left: 2.,
                    border_right: 3.,
                    border_top: 4.,
                    border_bottom: 5.,
                    decoration_elements: |content: &mut DecorationElements, _, _| {
                        content.add(&top_left, (Left(1.), Top(2.)), None);
                        content.add(&bottom_right, (Right(2.), Bottom(5.)), Some(4.));
                    },
                };

                let ret = callback.call(element);

                if assert {
                    assert_debug_snapshot!((
                        primary.into_passes(),
                        top_left.into_passes(),
                        bottom_right.into_passes()
                    ));
                }

                ret
            },
        );

        assert_debug_snapshot!(output);
    }

    #[test]
    fn test_breakable() {
        let output = test_element(
            TestElementParams {
                width: WidthConstraint {
                    max: 10.,
                    expand: false,
                },
                first_height: 19.,
                breakable: Some(TestElementParamsBreakable {
                    preferred_height_break_count: 5,
                    full_height: 20.,
                }),
                pos: (10., 30.0),
                ..Default::default()
            },
            |assert, callback| {
                let primary = RecordPasses::new(FakeText {
                    lines: 3,
                    line_height: 5.,
                    width: 3.,
                });

                let top_right = RecordPasses::new(FakeText {
                    lines: 1,
                    line_height: 5.,
                    width: 6.,
                });

                let bottom_left = RecordPasses::new(FakeText {
                    lines: 1,
                    line_height: 4.,
                    width: 3.,
                });

                let element = Page {
                    primary: RefElement(&primary),
                    border_left: 2.,
                    border_right: 3.,
                    border_top: 4.,
                    border_bottom: 5.,
                    decoration_elements: |content: &mut DecorationElements, _, _| {
                        content.add(&top_right, (Right(2.5), Top(2.)), None);
                        content.add(&bottom_left, (Left(2.), Bottom(5.)), Some(4.));
                    },
                };

                let ret = callback.call(element);

                if assert {
                    assert_debug_snapshot!((
                        primary.into_passes(),
                        top_right.into_passes(),
                        bottom_left.into_passes()
                    ));
                }

                ret
            },
        );

        assert_debug_snapshot!(output);
    }
}
