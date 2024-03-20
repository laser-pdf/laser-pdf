use crate::*;

pub struct CenterInPreferredHeight<'a, E: Element>(pub &'a E);

impl<'a, E: Element> Element for CenterInPreferredHeight<'a, E> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        let layout = self.layout(ctx.width, ctx.first_height, Some(ctx.full_height));

        if layout.pre_break {
            FirstLocationUsage::WillSkip
        } else if layout.size.height.is_some() {
            FirstLocationUsage::WillUse
        } else {
            FirstLocationUsage::NoneHeight
        }
    }

    fn measure(&self, mut ctx: MeasureCtx) -> ElementSize {
        let layout = self.layout(
            ctx.width,
            ctx.first_height,
            ctx.breakable.as_ref().map(|b| b.full_height),
        );

        if layout.pre_break {
            let breakable = ctx.breakable.as_mut().unwrap();

            *breakable.break_count = 1;
        }

        layout.size
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        let layout = self.layout(
            ctx.width,
            ctx.first_height,
            ctx.breakable.as_ref().map(|b| b.full_height),
        );

        let height_available;
        let mut location;
        let center_height;

        dbg!(
            ctx.first_height,
            ctx.breakable.as_ref().map(|x| x.full_height)
        );

        if layout.size.height.is_none() {
            return layout.size;
        } else if dbg!(layout.pre_break) {
            let breakable = ctx.breakable.unwrap();

            location = (breakable.get_location)(ctx.pdf, 0);
            height_available = breakable.full_height;

            center_height = if breakable.preferred_height_break_count == 1 {
                ctx.preferred_height.unwrap_or(0.)
            } else {
                breakable.full_height
            };
        } else {
            location = ctx.location;
            height_available = ctx.first_height;
            center_height = if ctx
                .breakable
                .map(|b| b.preferred_height_break_count == 0)
                .unwrap_or(true)
            {
                ctx.preferred_height.unwrap_or(0.)
            } else {
                ctx.first_height
            };
        }

        let y_offset = if let Some(height) = layout.size.height {
            (center_height - height).max(0.) / 2.
        } else {
            0.
        };

        location.pos.1 -= y_offset;

        self.0.draw(DrawCtx {
            pdf: ctx.pdf,
            location,
            width: ctx.width,
            first_height: height_available,
            preferred_height: None,
            breakable: None,
        });

        ElementSize {
            width: layout.size.width,
            height: Some(center_height),
        }
    }
}

#[derive(Debug)]
struct Layout {
    pre_break: bool,
    size: ElementSize,
}

impl<'a, E: Element> CenterInPreferredHeight<'a, E> {
    fn layout(
        &self,
        width: WidthConstraint,
        first_height: f64,
        full_height: Option<f64>,
    ) -> Layout {
        let height_available = full_height.unwrap_or(first_height);

        let size = self.0.measure(MeasureCtx {
            width,
            first_height: height_available,
            breakable: None,
        });

        let pre_break;

        if let (Some(height), Some(full_height)) = (size.height, full_height) {
            pre_break = height > first_height && full_height > first_height;
        } else {
            pre_break = false;
        };

        Layout { pre_break, size }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{record_passes::RecordPasses, *};
    use insta::*;

    #[test]
    fn test_unbreakable() {
        let output = test_element(
            TestElementParams {
                width: WidthConstraint {
                    max: 12.,
                    expand: true,
                },
                first_height: 21.,
                preferred_height: Some(20.),
                breakable: None,
                pos: (11., 29.0),
                ..Default::default()
            },
            |assert, callback| {
                let content = RecordPasses::new(FakeText {
                    lines: 3,
                    line_height: 5.,
                    width: 3.,
                });

                let element = CenterInPreferredHeight(&content);

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
    fn test_breakable() {
        let output = test_element(
            TestElementParams {
                width: WidthConstraint {
                    max: 12.,
                    expand: true,
                },
                first_height: 21.,
                preferred_height: Some(19.),
                breakable: Some(TestElementParamsBreakable {
                    full_height: 25.,
                    preferred_height_break_count: 0,
                    ..Default::default()
                }),
                pos: (11., 29.0),
                ..Default::default()
            },
            |assert, callback| {
                let content = RecordPasses::new(FakeText {
                    lines: 3,
                    line_height: 5.,
                    width: 3.,
                });

                let element = CenterInPreferredHeight(&content);

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
                    max: 12.,
                    expand: true,
                },
                first_height: 21.,
                preferred_height: Some(25.),
                breakable: Some(TestElementParamsBreakable {
                    full_height: 26.,
                    preferred_height_break_count: 1,
                    ..Default::default()
                }),
                pos: (11., 29.0),
                ..Default::default()
            },
            |assert, callback| {
                let content = RecordPasses::new(FakeText {
                    lines: 5,
                    line_height: 5.,
                    width: 3.,
                });

                let element = CenterInPreferredHeight(&content);

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
    fn test_preferred_height() {
        let output = test_element(
            TestElementParams {
                width: WidthConstraint {
                    max: 12.,
                    expand: true,
                },
                first_height: 21.,
                breakable: Some(TestElementParamsBreakable {
                    full_height: 26.,
                    preferred_height_break_count: 4,
                }),
                pos: (11., 29.0),
                preferred_height: None,
                ..Default::default()
            },
            |assert, callback| {
                let content = RecordPasses::new(FakeText {
                    lines: 5,
                    line_height: 5.,
                    width: 3.,
                });

                let element = CenterInPreferredHeight(&content);

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
