use crate::*;

pub struct AlignLocationBottom<E: Element>(pub E);

impl<E: Element> Element for AlignLocationBottom<E> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        let layout = self.layout(
            ctx.text_pieces_cache,
            ctx.width,
            ctx.first_height,
            Some(ctx.full_height),
            0,
        );

        if layout.breaks > 0 {
            FirstLocationUsage::WillSkip
        } else if layout.size.height.is_some() {
            FirstLocationUsage::WillUse
        } else {
            FirstLocationUsage::NoneHeight
        }
    }

    fn measure(&self, mut ctx: MeasureCtx) -> ElementSize {
        let layout = self.layout(
            ctx.text_pieces_cache,
            ctx.width,
            ctx.first_height,
            ctx.breakable.as_ref().map(|b| b.full_height),
            0,
        );

        let height = if layout.breaks > 0 {
            let breakable = ctx.breakable.as_mut().unwrap();

            *breakable.break_count = layout.breaks;

            Some(breakable.full_height)
        } else {
            layout.size.height.map(|_| ctx.first_height)
        };

        if let Some(breakable) = ctx.breakable {
            *breakable.extra_location_min_height = Some(breakable.full_height);
        }

        ElementSize {
            width: layout.size.width,
            height,
        }
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        let layout = self.layout(
            ctx.text_pieces_cache,
            ctx.width,
            ctx.first_height,
            ctx.breakable.as_ref().map(|b| b.full_height),
            ctx.breakable
                .as_ref()
                .map(|b| b.preferred_height_break_count)
                .unwrap_or(0),
        );

        let height_available;
        let height;
        let mut location;

        if layout.breaks > 0 {
            let breakable = ctx.breakable.unwrap();

            location = (breakable.do_break)(ctx.pdf, layout.breaks - 1, None);
            height_available = breakable.full_height;

            height = Some(breakable.full_height);
        } else {
            location = ctx.location;
            height_available = ctx.first_height;
            height = layout.size.height.map(|_| ctx.first_height);
        }

        location.pos.1 -= layout.y_offset;

        self.0.draw(DrawCtx {
            pdf: ctx.pdf,
            text_pieces_cache: ctx.text_pieces_cache,
            location,
            width: ctx.width,
            first_height: height_available,
            preferred_height: None,
            breakable: None,
        });

        ElementSize {
            width: layout.size.width,
            height,
        }
    }
}

#[derive(Debug)]
struct Layout {
    breaks: u32,
    y_offset: f32,
    size: ElementSize,
}

impl<E: Element> AlignLocationBottom<E> {
    fn layout(
        &self,
        text_pieces_cache: &TextPiecesCache,
        width: WidthConstraint,
        first_height: f32,
        full_height: Option<f32>,
        preferred_breaks: u32,
    ) -> Layout {
        let height_available = full_height.unwrap_or(first_height);

        let size = self.0.measure(MeasureCtx {
            text_pieces_cache,
            width,
            first_height: height_available,
            breakable: None,
        });

        let breaks;
        let location_height;

        if let (Some(height), Some(full_height)) = (size.height, full_height) {
            breaks = if preferred_breaks == 0 && height > first_height {
                1
            } else {
                preferred_breaks
            };

            location_height = if breaks > 0 {
                full_height
            } else {
                first_height
            };
        } else {
            breaks = 0;
            location_height = first_height;
        };

        let y_offset = if let Some(height) = size.height {
            location_height - height
        } else {
            0.
        };

        Layout {
            breaks,
            y_offset,
            size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        elements::ref_element::RefElement,
        test_utils::{record_passes::RecordPasses, *},
    };
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

                let element = AlignLocationBottom(RefElement(&content));

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
                breakable: Some(TestElementParamsBreakable {
                    full_height: 25.,
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

                let element = AlignLocationBottom(RefElement(&content));

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
                breakable: Some(TestElementParamsBreakable {
                    full_height: 26.,
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

                let element = AlignLocationBottom(RefElement(&content));

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

                let element = AlignLocationBottom(RefElement(&content));

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
