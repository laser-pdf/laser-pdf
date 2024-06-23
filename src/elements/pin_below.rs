use crate::*;

use self::utils::{add_optional_size, max_optional_size};

pub struct PinBelow<'a, C: Element, B: Element> {
    pub content: &'a C,
    pub pinned_element: &'a B,
    pub gap: f64,
    pub collapse: bool,
}

struct Common {
    first_height: f64,
    full_height: Option<f64>,
    bottom_size: ElementSize,
    bottom_height: f64,
    pre_break: bool,
    content_first_location_usage: Option<FirstLocationUsage>,
}

impl<'a, C: Element, B: Element> PinBelow<'a, C, B> {
    fn common(
        &self,
        width: WidthConstraint,
        first_height: f64,
        full_height: Option<f64>,
    ) -> Common {
        let bottom_first_height = full_height.unwrap_or(first_height);

        let bottom_size = self.pinned_element.measure(MeasureCtx {
            width,
            first_height: bottom_first_height,
            breakable: None,
        });

        let bottom_height = bottom_size.height.map(|h| h + self.gap).unwrap_or(0.);

        let mut first_height = first_height - bottom_height;

        let full_height = full_height.map(|f| f - bottom_height);

        let mut content_first_location_usage = None;

        let pre_break = full_height.is_some_and(|full_height| {
            first_height < full_height
                && !self.collapse
                && (bottom_size.height > Some(first_height)
                    || *content_first_location_usage.insert(self.content.first_location_usage(
                        FirstLocationUsageCtx {
                            width,
                            first_height,
                            full_height,
                        },
                    )) == FirstLocationUsage::WillSkip)
        });

        if pre_break {
            first_height = full_height.unwrap();
        }

        Common {
            bottom_size,
            bottom_height,
            first_height,
            full_height,
            pre_break,
            content_first_location_usage,
        }
    }

    fn height(&self, common: &Common, height: Option<f64>) -> Option<f64> {
        height
            .map(|h| h + self.gap)
            .or((!self.collapse).then_some(0.))
            .and_then(|h| add_optional_size(Some(h), common.bottom_size.height))
    }

    fn size(&self, common: &Common, content_size: ElementSize) -> ElementSize {
        ElementSize {
            width: max_optional_size(content_size.width, common.bottom_size.width),
            height: self.height(common, content_size.height),
        }
    }
}

impl<'a, C: Element, B: Element> Element for PinBelow<'a, C, B> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        let common = self.common(ctx.width, ctx.first_height, Some(ctx.full_height));

        if common.pre_break {
            return FirstLocationUsage::WillSkip;
        }

        let first_location_usage = common.content_first_location_usage.unwrap_or_else(|| {
            self.content.first_location_usage(FirstLocationUsageCtx {
                width: ctx.width,
                first_height: common.first_height,
                full_height: common.full_height.unwrap(),
            })
        });

        if first_location_usage == FirstLocationUsage::NoneHeight && !self.collapse {
            if common.bottom_size.height.is_none() {
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
            ctx.width,
            ctx.first_height,
            ctx.breakable.as_ref().map(|b| b.full_height),
        );

        let mut break_count = 0;
        let mut extra_location_min_height = None;

        let size = self.content.measure(MeasureCtx {
            width: ctx.width,
            first_height: common.first_height,
            breakable: ctx.breakable.as_mut().map(|_| BreakableMeasure {
                full_height: common.full_height.unwrap(),
                break_count: &mut break_count,
                extra_location_min_height: &mut extra_location_min_height,
            }),
        });

        if let Some(breakable) = ctx.breakable {
            *breakable.break_count = break_count + u32::from(common.pre_break);
            *breakable.extra_location_min_height =
                extra_location_min_height.map(|x| x + common.bottom_height);
        }

        self.size(&common, size)
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        let common = self.common(
            ctx.width,
            ctx.first_height,
            ctx.breakable.as_ref().map(|b| b.full_height),
        );

        let mut current_location = ctx.location.clone();

        let size = if let Some(breakable) = ctx.breakable {
            let mut break_count = 0;

            let (location, location_offset) = if common.pre_break {
                current_location = (breakable.do_break)(ctx.pdf, 0, None);
                (current_location.clone(), 1)
            } else {
                (ctx.location, 0)
            };

            self.content.draw(DrawCtx {
                pdf: ctx.pdf,
                location,
                width: ctx.width,
                first_height: common.first_height,
                preferred_height: ctx.preferred_height.map(|p| p - common.bottom_height),
                breakable: Some(BreakableDraw {
                    full_height: common.full_height.unwrap(),
                    preferred_height_break_count: breakable.preferred_height_break_count,
                    do_break: &mut |pdf, location_idx, height| {
                        if location_idx >= break_count {
                            break_count = location_idx + 1;

                            current_location =
                                (breakable.do_break)(pdf, location_offset + location_idx, height);

                            current_location.clone()
                        } else {
                            (breakable.do_break)(pdf, location_offset + location_idx, height)
                        }
                    },
                }),
            })
        } else {
            self.content.draw(DrawCtx {
                pdf: ctx.pdf,
                location: ctx.location,
                width: ctx.width,
                first_height: common.first_height,
                preferred_height: ctx.preferred_height.map(|p| p - common.bottom_height),
                breakable: None,
            })
        };

        if let Some((y_offset, bottom_height)) = size
            .height
            .map(|h| h + self.gap)
            .or((!self.collapse).then_some(0.))
            .zip(common.bottom_size.height)
        {
            self.pinned_element.draw(DrawCtx {
                pdf: ctx.pdf,
                location: Location {
                    layer: current_location.layer.clone(),
                    pos: (current_location.pos.0, current_location.pos.1 - y_offset),
                },
                width: ctx.width,
                first_height: bottom_height,
                preferred_height: None,
                breakable: None,
            });
        }

        self.size(&common, size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        elements::{none::NoneElement, text::Text, titled::Titled},
        fonts::builtin::BuiltinFont,
        test_utils::{binary_snapshots::*, FranticJumper},
    };
    use insta::*;

    #[test]
    fn test() {
        let mut write = |file: &mut std::fs::File| {
            test_element_file(
                TestElementParams::breakable(),
                |callback| {
                    let font = BuiltinFont::courier(callback.document());

                    let content = Text::basic(LOREM_IPSUM, &font, 32.);
                    let content = content.debug(1);

                    let bottom = Text::basic("bottom", &font, 12.);
                    let bottom = bottom.debug(2);

                    callback.call(
                        &PinBelow {
                            content: &content,
                            pinned_element: &bottom,
                            gap: 5.,
                            collapse: true,
                        }
                        .debug(0),
                    );
                },
                file,
            );
        };
        assert_binary_snapshot!("pdf", write);
    }

    #[test]
    fn test_collapse() {
        let mut write = |file: &mut std::fs::File| {
            test_element_file(
                TestElementParams::breakable(),
                |callback| {
                    let font = BuiltinFont::courier(callback.document());

                    let content = NoneElement;
                    let content = content.debug(1);

                    let bottom = Text::basic("bottom", &font, 12.);
                    let bottom = bottom.debug(2);

                    callback.call(
                        &PinBelow {
                            content: &content,
                            pinned_element: &bottom,
                            gap: 5.,
                            collapse: true,
                        }
                        .debug(0),
                    );
                },
                file,
            );
        };
        assert_binary_snapshot!("pdf", write);
    }

    #[test]
    fn test_no_collapse() {
        let mut write = |file: &mut std::fs::File| {
            test_element_file(
                TestElementParams::breakable(),
                |callback| {
                    let font = BuiltinFont::courier(callback.document());

                    let content = NoneElement;
                    let content = content.debug(1);

                    let bottom = Text::basic("bottom", &font, 12.);
                    let bottom = bottom.debug(2);

                    callback.call(
                        &PinBelow {
                            content: &content,
                            pinned_element: &bottom,
                            gap: 5.,
                            collapse: false,
                        }
                        .debug(0),
                    );
                },
                file,
            );
        };
        assert_binary_snapshot!("pdf", write);
    }

    #[test]
    fn test_no_collapse_bottom_overflow() {
        let mut write = |file: &mut std::fs::File| {
            test_element_file(
                TestElementParams {
                    first_height: 1.,
                    ..TestElementParams::breakable()
                },
                |callback| {
                    let font = BuiltinFont::courier(callback.document());

                    let content = NoneElement;
                    let content = content.debug(1);

                    let bottom = Text::basic("bottom", &font, 12.);
                    let bottom = bottom.debug(2);

                    callback.call(
                        &PinBelow {
                            content: &content,
                            pinned_element: &bottom,
                            gap: 5.,
                            collapse: false,
                        }
                        .debug(0),
                    );
                },
                file,
            );
        };
        assert_binary_snapshot!("pdf", write);
    }

    #[test]
    fn test_multipage_no_collapse() {
        let mut write = |file: &mut std::fs::File| {
            test_element_file(
                TestElementParams::breakable(),
                |callback| {
                    let font = BuiltinFont::courier(callback.document());

                    let content = FranticJumper {
                        jumps: vec![(0, None), (0, None), (2, Some(32.)), (3, Some(55.))],
                        size: ElementSize {
                            width: Some(12.),
                            height: None,
                        },
                    };
                    let content = content.debug(1);

                    let bottom = Text::basic("bottom", &font, 12.);
                    let bottom = bottom.debug(2);

                    callback.call(
                        &PinBelow {
                            content: &content,
                            pinned_element: &bottom,
                            gap: 10.,
                            collapse: false,
                        }
                        .debug(0),
                    );
                },
                file,
            );
        };
        assert_binary_snapshot!("pdf", write);
    }

    #[test]
    fn test_multipage_collapse() {
        let mut write = |file: &mut std::fs::File| {
            test_element_file(
                TestElementParams::breakable(),
                |callback| {
                    let font = BuiltinFont::courier(callback.document());

                    let content = FranticJumper {
                        jumps: vec![(1, None), (1, None), (3, Some(32.)), (4, None)],
                        size: ElementSize {
                            width: Some(12.),
                            height: None,
                        },
                    };
                    let content = content.debug(1);

                    let bottom = Text::basic("bottom", &font, 12.);
                    let bottom = bottom.debug(2);

                    callback.call(
                        &PinBelow {
                            content: &content,
                            pinned_element: &bottom,
                            gap: 10.,
                            collapse: true,
                        }
                        .debug(0),
                    );
                },
                file,
            );
        };
        assert_binary_snapshot!("pdf", write);
    }

    #[test]
    fn test_titled() {
        let mut write = |file: &mut std::fs::File| {
            test_element_file(
                TestElementParams {
                    first_height: 10.,
                    ..TestElementParams::breakable()
                },
                |callback| {
                    let font = BuiltinFont::courier(callback.document());
                    let title = Text::basic("title", &font, 12.);
                    let title = &title.debug(1);

                    let content = Text::basic("content", &font, 32.);
                    let content = &content.debug(3);

                    let bottom = Text::basic("bottom", &font, 12.);
                    let bottom = &bottom.debug(4);

                    let repeat_bottom = PinBelow {
                        content,
                        pinned_element: bottom,
                        gap: 5.,
                        collapse: true,
                    };
                    let repeat_bottom = &repeat_bottom.debug(2);

                    callback.call(
                        &Titled {
                            title,
                            content: repeat_bottom,
                            gap: 5.,
                            collapse_on_empty_content: true,
                        }
                        .debug(0),
                    );
                },
                file,
            );
        };
        assert_binary_snapshot!("pdf", write);
    }
}
