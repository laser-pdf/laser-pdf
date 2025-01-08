use crate::{utils::max_optional_size, *};

pub struct ExpandToPreferredHeight<'a, E: Element>(pub &'a E);

impl<'a, E: Element> Element for ExpandToPreferredHeight<'a, E> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        self.0.first_location_usage(ctx)
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        self.0.measure(ctx)
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        let preferred_height = ctx.preferred_height;
        let preferred_breaks = ctx
            .breakable
            .as_ref()
            .map(|b| b.preferred_height_break_count)
            .unwrap_or(0);

        let size;
        let height;

        if let Some(breakable) = ctx.breakable {
            let mut break_count = 0;

            size = self.0.draw(DrawCtx {
                pdf: ctx.pdf,
                breakable: Some(BreakableDraw {
                    do_break: &mut |pdf, location_idx, _height| {
                        break_count = break_count.max(location_idx + 1);

                        (breakable.do_break)(
                            pdf,
                            location_idx,
                            // we also expand all of the heights
                            if location_idx == 0 {
                                Some(ctx.first_height)
                            } else {
                                Some(breakable.full_height)
                            },
                        )
                    },
                    ..breakable
                }),

                ..ctx
            });

            match break_count.cmp(&preferred_breaks) {
                std::cmp::Ordering::Less => {
                    // We need to go through all of the locations to use the full_height on all of
                    // them.
                    for i in break_count..preferred_breaks {
                        (breakable.do_break)(
                            ctx.pdf,
                            i,
                            if i == 0 {
                                Some(ctx.first_height)
                            } else {
                                Some(breakable.full_height)
                            },
                        );
                    }

                    height = preferred_height;
                }
                std::cmp::Ordering::Equal => {
                    height = max_optional_size(size.height, preferred_height);
                }
                std::cmp::Ordering::Greater => {
                    height = size.height;
                }
            }
        } else {
            size = self.0.draw(ctx);
            height = max_optional_size(size.height, preferred_height);
        }

        ElementSize { height, ..size }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        elements::text::Text, fonts::builtin::BuiltinFont, test_utils::binary_snapshots::*,
    };
    use insta::*;

    #[test]
    fn test_basic() {
        let bytes = test_element_bytes(
            TestElementParams {
                preferred_height: Some(12.),
                breakable: Some(TestElementParamsBreakable {
                    preferred_height_break_count: 7,
                    full_height: TestElementParams::DEFAULT_FULL_HEIGHT,
                }),
                ..TestElementParams::breakable()
            },
            |mut callback| {
                let font = BuiltinFont::courier(callback.pdf());

                let content = Text::basic(LOREM_IPSUM, &font, 32.);
                let content = content.debug(1);

                callback.call(&ExpandToPreferredHeight(&content).debug(0));
            },
        );
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_single_location_content() {
        let bytes = test_element_bytes(
            TestElementParams {
                preferred_height: Some(32.),
                breakable: Some(TestElementParamsBreakable {
                    preferred_height_break_count: 3,
                    full_height: TestElementParams::DEFAULT_FULL_HEIGHT,
                }),
                ..TestElementParams::breakable()
            },
            |mut callback| {
                let font = BuiltinFont::courier(callback.pdf());

                let content = Text::basic(LOREM_IPSUM, &font, 12.);
                let content = content.debug(1);

                callback.call(&ExpandToPreferredHeight(&content).debug(0));
            },
        );
        assert_binary_snapshot!(".pdf", bytes);
    }

    // TODO: Figure out what makes sense here.
    // #[test]
    // fn test_collapse() {
    //     let output = test_element(
    //         TestElementParams {
    //             width: WidthConstraint {
    //                 max: 20.,
    //                 expand: true,
    //             },
    //             first_height: 21.,
    //             preferred_height: Some(12.),
    //             breakable: Some(TestElementParamsBreakable {
    //                 preferred_height_break_count: 7,
    //                 full_height: 500.,
    //             }),
    //             pos: (11., 600.0),
    //             ..Default::default()
    //         },
    //         |assert, callback| {
    //             let content = RecordPasses::new(NoneElement);

    //             let element = ExpandToPreferredHeight(&content);

    //             let ret = callback.call(element);

    //             if assert {
    //                 assert_debug_snapshot!(content.into_passes());
    //             }

    //             ret
    //         },
    //     );

    //     assert_debug_snapshot!(output);
    // }
}
