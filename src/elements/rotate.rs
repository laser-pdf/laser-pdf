use printpdf::CurTransMat;

use crate::*;

pub enum Rotation {
    QuarterLeft,
    QuarterRight,
}

pub struct Rotate<'a, E: Element> {
    pub element: &'a E,
    pub rotation: Rotation,
}

impl<'a, E: Element> Element for Rotate<'a, E> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        let element_width_constraint = WidthConstraint {
            max: ctx.full_height,
            expand: false,
        };

        let size = self.element.measure(MeasureCtx {
            width: element_width_constraint,
            first_height: ctx.width.max,
            breakable: None,
        });

        if size.width.is_none() {
            FirstLocationUsage::NoneHeight
        } else if ctx.first_height < ctx.full_height && size.width > Some(ctx.first_height) {
            FirstLocationUsage::WillSkip
        } else {
            FirstLocationUsage::WillUse
        }
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        let element_width_constraint = WidthConstraint {
            max: ctx
                .breakable
                .as_ref()
                .map(|b| b.full_height)
                .unwrap_or(ctx.first_height),
            expand: false,
        };

        let size = self.element.measure(MeasureCtx {
            width: element_width_constraint,
            first_height: ctx.width.max,
            breakable: None,
        });

        match ctx.breakable {
            Some(breakable)
                if ctx.first_height < breakable.full_height
                    && size.width > Some(ctx.first_height) =>
            {
                *breakable.break_count = 1;
            }
            _ => (),
        }

        ElementSize {
            width: size.height,
            height: size.width,
        }
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        let element_width_constraint = WidthConstraint {
            max: ctx
                .breakable
                .as_ref()
                .map(|b| b.full_height)
                .unwrap_or(ctx.first_height),
            expand: false,
        };

        let size = self.element.measure(MeasureCtx {
            width: element_width_constraint,
            first_height: ctx.width.max,
            breakable: None,
        });

        let location;

        match ctx.breakable {
            Some(breakable)
                if ctx.first_height < breakable.full_height
                    && size.width > Some(ctx.first_height) =>
            {
                location = (breakable.do_break)(ctx.pdf, 0, None);
            }
            _ => location = ctx.location,
        }

        let layer = location.layer;

        if let (Some(width), Some(height)) = (size.width, size.height) {
            layer.save_graphics_state();

            let (x, y, rotation) = match self.rotation {
                Rotation::QuarterLeft => (location.pos.0, location.pos.1 - width, 90.),
                Rotation::QuarterRight => (location.pos.0 + height, location.pos.1, 270.),
            };

            layer.set_ctm(CurTransMat::Translate(Mm(x), Mm(y)));
            layer.set_ctm(CurTransMat::Rotate(rotation));

            // TODO: Make layers work inside here. Maybe this could be done when we migrate to
            // pdfwriter.

            self.element.draw(DrawCtx {
                pdf: ctx.pdf,
                location: Location {
                    layer: layer.clone(),
                    pos: (0., 0.),
                    ..location
                },
                width: element_width_constraint,
                first_height: ctx.width.max,
                preferred_height: None,
                breakable: None,
            });

            layer.restore_graphics_state();
        }

        ElementSize {
            width: size.height,
            height: size.width,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        elements::none::NoneElement,
        test_utils::{record_passes::RecordPasses, *},
    };
    use insta::*;

    #[test]
    fn test_basic() {
        let output = test_element(
            TestElementParams {
                width: WidthConstraint {
                    max: 16.,
                    expand: true,
                },
                first_height: 21.,
                breakable: Some(TestElementParamsBreakable {
                    preferred_height_break_count: 0,
                    full_height: 500.,
                }),
                pos: (11., 29.0),
                ..Default::default()
            },
            |assert, callback| {
                let content = RecordPasses::new(FakeText {
                    lines: 3,
                    line_height: 5.,
                    width: 18.,
                });

                let element = Rotate {
                    element: &content,
                    rotation: Rotation::QuarterRight,
                };

                let ret = callback.call(element);

                if assert {
                    assert_debug_snapshot!(content.into_passes());
                }

                ret
            },
        );

        assert_debug_snapshot!(output);
    }

    #[test]
    fn test_pre_break() {
        let output = test_element(
            TestElementParams {
                width: WidthConstraint {
                    max: 16.,
                    expand: true,
                },
                first_height: 21.,
                breakable: Some(TestElementParamsBreakable {
                    preferred_height_break_count: 0,
                    full_height: 500.,
                }),
                pos: (11., 29.0),
                ..Default::default()
            },
            |assert, callback| {
                let content = RecordPasses::new(FakeText {
                    lines: 3,
                    line_height: 5.,
                    width: 100.,
                });

                let element = Rotate {
                    element: &content,
                    rotation: Rotation::QuarterLeft,
                };

                let ret = callback.call(element);

                if assert {
                    assert_debug_snapshot!(content.into_passes());
                }

                ret
            },
        );

        assert_debug_snapshot!(output);
    }

    #[test]
    fn test_none() {
        let output = test_element(
            TestElementParams {
                width: WidthConstraint {
                    max: 16.,
                    expand: true,
                },
                first_height: 21.,
                breakable: Some(TestElementParamsBreakable {
                    preferred_height_break_count: 0,
                    full_height: 500.,
                }),
                pos: (11., 29.0),
                ..Default::default()
            },
            |assert, callback| {
                let content = RecordPasses::new(NoneElement);

                let element = Rotate {
                    element: &content,
                    rotation: Rotation::QuarterRight,
                };

                let ret = callback.call(element);

                if assert {
                    assert_debug_snapshot!(content.into_passes());
                }

                ret
            },
        );

        assert_debug_snapshot!(output);
    }
}
