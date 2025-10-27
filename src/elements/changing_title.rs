use crate::*;

use self::utils::{add_optional_size, max_optional_size};

pub struct ChangingTitle<F: Element, R: Element, C: Element> {
    pub first_title: F,
    pub remaining_title: R,
    pub content: C,
    pub gap: f32,
    pub collapse: bool,
}

struct CommonBreakable {
    full_height: f32,
    pre_break: bool,
    remaining_title_size: ElementSize,
    total_remaining_title_height: f32,
    content_first_location_usage: Option<FirstLocationUsage>,
}

struct Common {
    first_height: f32,
    first_title_size: ElementSize,
    total_first_title_height: f32,
    breakable: Option<CommonBreakable>,
}

impl<F: Element, R: Element, C: Element> ChangingTitle<F, R, C> {
    fn common(
        &self,
        text_pieces_cache: &TextPiecesCache,
        width: WidthConstraint,
        first_height: f32,
        full_height: Option<f32>,
    ) -> Common {
        let bottom_first_height = full_height.unwrap_or(first_height);

        let first_title_size = self.first_title.measure(MeasureCtx {
            text_pieces_cache,
            width,
            first_height: bottom_first_height,
            breakable: None,
        });

        let total_first_title_height = first_title_size.height.map(|h| h + self.gap).unwrap_or(0.);

        let mut first_height = first_height - total_first_title_height;

        let breakable = full_height.map(|full_height| {
            let remaining_title_size = self.remaining_title.measure(MeasureCtx {
                text_pieces_cache,
                width,
                first_height: full_height,
                breakable: None,
            });
            let total_remaining_title_height = remaining_title_size
                .height
                .map(|h| h + self.gap)
                .unwrap_or(0.);

            let full_height = full_height - total_remaining_title_height;

            let mut content_first_location_usage = None;

            let pre_break = first_height < full_height
                && !self.collapse
                && (first_title_size.height > Some(first_height)
                    || *content_first_location_usage.insert(self.content.first_location_usage(
                        FirstLocationUsageCtx {
                            text_pieces_cache,
                            width,
                            first_height,
                            full_height,
                        },
                    )) == FirstLocationUsage::WillSkip);

            if pre_break {
                first_height = full_height;
            } else {
                // first_height is not allowed to be more than full_height
                first_height = first_height.min(full_height);
            }

            CommonBreakable {
                full_height,
                pre_break,
                remaining_title_size,
                total_remaining_title_height,
                content_first_location_usage,
            }
        });

        Common {
            first_height,
            first_title_size,
            total_first_title_height,
            breakable,
        }
    }

    fn height(&self, title_height: Option<f32>, height: Option<f32>) -> Option<f32> {
        height
            .map(|h| h + self.gap)
            .or((!self.collapse).then_some(0.))
            .and_then(|h| add_optional_size(Some(h), title_height))
    }

    fn size(&self, common: &Common, break_count: u32, content_size: ElementSize) -> ElementSize {
        let first_width = max_optional_size(content_size.width, common.first_title_size.width);

        if break_count == 0 {
            ElementSize {
                width: first_width,
                height: self.height(common.first_title_size.height, content_size.height),
            }
        } else {
            let breakable = common.breakable.as_ref().unwrap();

            ElementSize {
                width: max_optional_size(first_width, breakable.remaining_title_size.width),
                height: self.height(breakable.remaining_title_size.height, content_size.height),
            }
        }
    }
}

impl<F: Element, R: Element, C: Element> Element for ChangingTitle<F, R, C> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        let common = self.common(
            ctx.text_pieces_cache,
            ctx.width,
            ctx.first_height,
            Some(ctx.full_height),
        );
        let breakable = common.breakable.unwrap();

        if breakable.pre_break {
            return FirstLocationUsage::WillSkip;
        }

        let first_location_usage = breakable.content_first_location_usage.unwrap_or_else(|| {
            self.content.first_location_usage(FirstLocationUsageCtx {
                text_pieces_cache: ctx.text_pieces_cache,
                width: ctx.width,
                first_height: common.first_height,
                full_height: breakable.full_height,
            })
        });

        if first_location_usage == FirstLocationUsage::NoneHeight && !self.collapse {
            if common.first_title_size.height.is_none() {
                FirstLocationUsage::NoneHeight
            } else {
                FirstLocationUsage::WillUse
            }
        } else {
            first_location_usage
        }
    }

    fn measure(&self, mut ctx: MeasureCtx) -> ElementSize {
        let common = self.common(
            ctx.text_pieces_cache,
            ctx.width,
            ctx.first_height,
            ctx.breakable.as_ref().map(|b| b.full_height),
        );

        let mut break_count = 0;
        let mut extra_location_min_height = None;

        let size = self.content.measure(MeasureCtx {
            text_pieces_cache: ctx.text_pieces_cache,
            width: ctx.width,
            first_height: common.first_height,
            breakable: ctx.breakable.as_mut().zip(common.breakable.as_ref()).map(
                |(_, breakable)| BreakableMeasure {
                    full_height: breakable.full_height,
                    break_count: &mut break_count,
                    extra_location_min_height: &mut extra_location_min_height,
                },
            ),
        });

        if let Some((breakable, common_breakable)) = ctx.breakable.zip(common.breakable.as_ref()) {
            *breakable.break_count = break_count + u32::from(common_breakable.pre_break);
            *breakable.extra_location_min_height = extra_location_min_height
                .map(|x| x + common_breakable.total_remaining_title_height);
        }

        self.size(&common, break_count, size)
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        let common = self.common(
            ctx.text_pieces_cache,
            ctx.width,
            ctx.first_height,
            ctx.breakable.as_ref().map(|b| b.full_height),
        );

        let mut current_location = ctx.location.clone();
        let mut break_count = 0;

        let size = if let Some((breakable, common_breakable)) =
            ctx.breakable.zip(common.breakable.as_ref())
        {
            let (location, location_offset) = if common_breakable.pre_break {
                current_location = (breakable.do_break)(ctx.pdf, 0, None);
                (current_location.clone(), 1)
            } else {
                (ctx.location.clone(), 0)
            };

            self.content.draw(DrawCtx {
                pdf: ctx.pdf,
                text_pieces_cache: ctx.text_pieces_cache,
                location: Location {
                    pos: (
                        location.pos.0,
                        location.pos.1 - common.total_first_title_height,
                    ),
                    ..location
                },
                width: ctx.width,
                first_height: common.first_height,
                preferred_height: ctx.preferred_height.map(|p| {
                    p - if breakable.preferred_height_break_count > 0 {
                        common.total_first_title_height
                    } else {
                        common_breakable.total_remaining_title_height
                    }
                }),
                breakable: Some(BreakableDraw {
                    full_height: common_breakable.full_height,
                    preferred_height_break_count: breakable.preferred_height_break_count,
                    do_break: &mut |pdf, location_idx, height| {
                        let outer_height = self.height(
                            if location_idx == 0 {
                                common.first_title_size.height
                            } else {
                                common_breakable.remaining_title_size.height
                            },
                            height,
                        );

                        let location = if location_idx >= break_count {
                            if let Some(first_height) =
                                common.first_title_size.height.filter(|_| break_count == 0)
                            {
                                self.first_title.draw(DrawCtx {
                                    pdf,
                                    text_pieces_cache: ctx.text_pieces_cache,
                                    location: ctx.location.clone(),
                                    width: ctx.width,
                                    first_height,
                                    preferred_height: None,
                                    breakable: None,
                                });
                            }

                            if let Some(title_height) =
                                common_breakable.remaining_title_size.height.filter(|_| {
                                    (height.is_some() || !self.collapse) && location_idx > 0
                                })
                            {
                                let first_location_idx = if self.collapse {
                                    location_idx
                                } else {
                                    break_count.max(1)
                                };

                                // here i is the location we want to draw on, not the location we break
                                // break from
                                for i in first_location_idx..=location_idx {
                                    let title_location = if i == break_count {
                                        current_location.clone()
                                    } else {
                                        (breakable.do_break)(
                                            pdf,
                                            location_offset + i - 1,
                                            // this works because skipped locations have an implied
                                            // height of None
                                            (!self.collapse).then_some(title_height),
                                        )
                                    };

                                    self.remaining_title.draw(DrawCtx {
                                        pdf,
                                        text_pieces_cache: ctx.text_pieces_cache,
                                        location: title_location,
                                        width: ctx.width,
                                        first_height: title_height,
                                        preferred_height: None,
                                        breakable: None,
                                    });
                                }
                            }

                            break_count = location_idx + 1;

                            current_location = (breakable.do_break)(
                                pdf,
                                location_offset + location_idx,
                                outer_height,
                            );

                            current_location.clone()
                        } else {
                            (breakable.do_break)(pdf, location_offset + location_idx, outer_height)
                        };

                        Location {
                            pos: (
                                location.pos.0,
                                location.pos.1 - common_breakable.total_remaining_title_height,
                            ),
                            ..location
                        }
                    },
                }),
            })
        } else {
            self.content.draw(DrawCtx {
                pdf: ctx.pdf,
                text_pieces_cache: ctx.text_pieces_cache,
                location: Location {
                    pos: (
                        ctx.location.pos.0,
                        ctx.location.pos.1 - common.total_first_title_height,
                    ),
                    ..ctx.location
                },
                width: ctx.width,
                first_height: common.first_height,
                preferred_height: ctx
                    .preferred_height
                    .map(|p| p - common.total_first_title_height),
                breakable: None,
            })
        };

        if let Some(title_height) = (if break_count == 0 {
            common.first_title_size.height
        } else {
            common
                .breakable
                .as_ref()
                .unwrap()
                .remaining_title_size
                .height
        })
        .filter(|_| size.height.is_some() || !self.collapse)
        {
            let draw_ctx = DrawCtx {
                pdf: ctx.pdf,
                text_pieces_cache: ctx.text_pieces_cache,
                location: current_location,
                width: ctx.width,
                first_height: title_height,
                preferred_height: None,
                breakable: None,
            };

            if break_count == 0 {
                self.first_title.draw(draw_ctx);
            } else {
                self.remaining_title.draw(draw_ctx);
            }
        }

        self.size(&common, break_count, size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        elements::{none::NoneElement, text::Text, titled::Titled},
        fonts::builtin::BuiltinFont,
        test_utils::{FranticJumper, binary_snapshots::*},
    };
    use insta::*;

    #[test]
    fn test() {
        let bytes = test_element_bytes(TestElementParams::breakable(), |mut callback| {
            let font = BuiltinFont::courier(callback.pdf());

            let first = Text::basic("first", &font, 12.);
            let first = first.debug(1);

            let remaining = Text::basic("remaining\nremaining", &font, 12.);
            let remaining = remaining.debug(2);

            let content = Text::basic(LOREM_IPSUM, &font, 32.);
            let content = content.debug(3);

            callback.call(
                &ChangingTitle {
                    first_title: first,
                    remaining_title: remaining,
                    content,
                    gap: 5.,
                    collapse: true,
                }
                .debug(0),
            );
        });
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_first_height_not_greater_than_full_height() {
        let bytes = test_element_bytes(
            TestElementParams {
                first_height: TestElementParams::DEFAULT_FULL_HEIGHT,
                ..TestElementParams::breakable()
            },
            |mut callback| {
                let font = BuiltinFont::courier(callback.pdf());

                let first = Text::basic("first", &font, 12.);
                let first = first.debug(1);

                let remaining = Text::basic("remaining\nremaining", &font, 12.);
                let remaining = remaining.debug(2);

                let content = Text::basic(LOREM_IPSUM, &font, 48.);
                let content = content.debug(3);

                callback.call(
                    &ChangingTitle {
                        first_title: first,
                        remaining_title: remaining,
                        content,
                        gap: 5.,
                        collapse: true,
                    }
                    .debug(0),
                );
            },
        );
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_collapse() {
        let bytes = test_element_bytes(TestElementParams::unbreakable(), |mut callback| {
            let font = BuiltinFont::courier(callback.pdf());

            let first = Text::basic("first", &font, 12.);
            let first = first.debug(1);

            let remaining = Text::basic("remaining\nremaining", &font, 12.);
            let remaining = remaining.debug(2);

            let content = NoneElement;
            let content = content.debug(3);

            callback.call(
                &ChangingTitle {
                    first_title: first,
                    remaining_title: remaining,
                    content,
                    gap: 5.,
                    collapse: true,
                }
                .debug(0),
            );
        });
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_no_collapse() {
        let bytes = test_element_bytes(TestElementParams::unbreakable(), |mut callback| {
            let font = BuiltinFont::courier(callback.pdf());

            let first = Text::basic("first", &font, 12.);
            let first = first.debug(1);

            let remaining = Text::basic("remaining\nremaining", &font, 12.);
            let remaining = remaining.debug(2);

            let content = NoneElement;
            let content = content.debug(3);

            callback.call(
                &ChangingTitle {
                    first_title: first,
                    remaining_title: remaining,
                    content,
                    gap: 5.,
                    collapse: false,
                }
                .debug(0),
            );
        });
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_multipage_collapse() {
        let bytes = test_element_bytes(TestElementParams::breakable(), |mut callback| {
            let font = BuiltinFont::courier(callback.pdf());

            let first = Text::basic("first", &font, 12.);
            let first = first.debug(1);

            let remaining = Text::basic("remaining\nremaining", &font, 12.);
            let remaining = remaining.debug(2);

            let content = FranticJumper {
                jumps: vec![(1, None), (1, None), (3, Some(32.)), (4, None)],
                size: ElementSize {
                    width: Some(44.),
                    height: None,
                },
            };
            let content = content.debug(3);

            callback.call(
                &ChangingTitle {
                    first_title: first,
                    remaining_title: remaining,
                    content,
                    gap: 5.,
                    collapse: true,
                }
                .debug(0),
            );
        });
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_titled() {
        let bytes = test_element_bytes(
            TestElementParams {
                first_height: 27.,
                width: WidthConstraint {
                    max: TestElementParams::DEFAULT_MAX_WIDTH,
                    expand: false,
                },
                ..TestElementParams::breakable()
            },
            |mut callback| {
                let font = BuiltinFont::courier(callback.pdf());

                let title = Text::basic("title", &font, 12.);
                let title = title.debug(1);

                let first = Text::basic("first", &font, 12.);
                let first = first.debug(3);

                let remaining = Text::basic("remaining\nremaining", &font, 12.);
                let remaining = remaining.debug(4);

                let content = Text::basic(LOREM_IPSUM, &font, 32.);
                let content = content.debug(5);

                let changing_title = ChangingTitle {
                    first_title: first,
                    remaining_title: remaining,
                    content,
                    gap: 5.,
                    collapse: true,
                };
                let changing_title = changing_title.debug(2);

                callback.call(
                    &Titled {
                        title,
                        content: changing_title,
                        gap: 2.,
                        collapse_on_empty_content: true,
                    }
                    .debug(0)
                    .show_max_width()
                    .show_last_location_max_height(),
                );
            },
        );
        assert_binary_snapshot!(".pdf", bytes);
    }
}
