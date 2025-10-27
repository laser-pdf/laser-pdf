use crate::*;

/// Shrinks the element to fit within the given `first_height`, as long as that is >= `min_height`.
/// In a breakable context: if `first_height` is less than `min_height` a pre-break happens first,
/// in which case the element will be shrunk to fit the `full_height`. In an unbreakable context it
/// will simply overflow such that the element is never scaled smaller than the `min_height`.
pub struct ShrinkToFit<E: Element> {
    pub element: E,
    pub min_height: f32,
}

struct Layout {
    pre_break: bool,
    scale_factor: f32,
    size: ElementSize,
    scaled_size: ElementSize,
    height: f32,
}

impl<E: Element> ShrinkToFit<E> {
    fn layout(
        &self,
        text_pieces_cache: &TextPiecesCache,
        width: WidthConstraint,
        first_height: f32,
        full_height: Option<f32>,
    ) -> Layout {
        let pre_break;

        let available_height = if first_height >= self.min_height {
            pre_break = false;

            first_height
        } else {
            pre_break = full_height.is_some();

            // We prefer overflowing if min_height is not available. If available_height were to
            // become negative it would lead to the element being flipped.
            full_height.unwrap_or(first_height).max(self.min_height)
        };

        let size = self.element.measure(MeasureCtx {
            text_pieces_cache,
            width,
            first_height: available_height,
            breakable: None,
        });

        let height = size
            .height
            .map(|h| {
                if h <= available_height {
                    available_height
                } else {
                    h
                }
            })
            .unwrap_or(available_height);

        let scale_factor = size
            .height
            .map(|h| {
                if h <= available_height {
                    1.
                } else {
                    available_height / h
                }
            })
            .unwrap_or(1.);

        let scaled_size = ElementSize {
            width: size.width.map(|w| w * scale_factor),
            height: size.height.map(|h| h * scale_factor),
        };

        Layout {
            pre_break,
            scale_factor,
            height,
            size,
            scaled_size,
        }
    }
}

impl<E: Element> Element for ShrinkToFit<E> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        let layout = self.layout(
            ctx.text_pieces_cache,
            ctx.width,
            ctx.first_height,
            Some(ctx.full_height),
        );

        if layout.pre_break {
            FirstLocationUsage::WillSkip
        } else if layout.size.height.is_some() {
            FirstLocationUsage::WillUse
        } else {
            FirstLocationUsage::NoneHeight
        }
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        let layout = self.layout(
            ctx.text_pieces_cache,
            ctx.width,
            ctx.first_height,
            ctx.breakable.as_ref().map(|b| b.full_height),
        );

        if layout.pre_break {
            *ctx.breakable.unwrap().break_count = 1;
        }

        layout.scaled_size
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        let layout = self.layout(
            ctx.text_pieces_cache,
            ctx.width,
            ctx.first_height,
            ctx.breakable.as_ref().map(|b| b.full_height),
        );

        let location;

        if layout.pre_break {
            let breakable = ctx.breakable.unwrap();

            location = (breakable.do_break)(ctx.pdf, 0, None);
        } else {
            location = ctx.location;
        }

        location
            .layer(ctx.pdf)
            .save_state()
            .transform(utils::scale(layout.scale_factor));

        self.element.draw(DrawCtx {
            pdf: ctx.pdf,
            text_pieces_cache: ctx.text_pieces_cache,
            location: Location {
                pos: (
                    location.pos.0 / layout.scale_factor,
                    location.pos.1 / layout.scale_factor,
                ),
                scale_factor: location.scale_factor * layout.scale_factor,
                ..location.clone()
            },
            width: ctx.width,
            first_height: layout.height,
            preferred_height: None,
            breakable: None,
        });

        location.layer(ctx.pdf).restore_state();

        layout.scaled_size
    }
}

#[cfg(test)]
mod tests {
    use elements::{align_location_bottom::AlignLocationBottom, styled_box::StyledBox};
    use insta::assert_binary_snapshot;

    use super::*;
    use crate::{
        elements::text::Text, fonts::builtin::BuiltinFont, test_utils::binary_snapshots::*,
    };

    #[test]
    fn test_basic() {
        let bytes = test_element_bytes(
            TestElementParams {
                first_height: 10.,
                ..TestElementParams::breakable()
            },
            |mut callback| {
                let font = BuiltinFont::courier(callback.pdf());
                let text = Text::basic("TEST", &font, 100.);
                let text = text
                    .debug(1)
                    .show_max_width()
                    .show_last_location_max_height();

                let shrink_to_fit = ShrinkToFit {
                    element: text,
                    min_height: 9.,
                };
                let shrink_to_fit = &shrink_to_fit
                    .debug(0)
                    .show_max_width()
                    .show_last_location_max_height();

                callback.call(shrink_to_fit);
            },
        );
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_unbreakable_negative_first_height() {
        let bytes = test_element_bytes(
            TestElementParams {
                first_height: -10.,
                ..TestElementParams::unbreakable()
            },
            |mut callback| {
                let font = BuiltinFont::courier(callback.pdf());
                let text = Text::basic("TEST", &font, 100.);
                let text = text
                    .debug(1)
                    .show_max_width()
                    .show_last_location_max_height();

                let shrink_to_fit = ShrinkToFit {
                    element: text,
                    min_height: 9.,
                };
                let shrink_to_fit = &shrink_to_fit
                    .debug(0)
                    .show_max_width()
                    .show_last_location_max_height();

                callback.call(shrink_to_fit);
            },
        );
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_pre_break() {
        let bytes = test_element_bytes(
            TestElementParams {
                first_height: 5.,
                ..TestElementParams::breakable()
            },
            |mut callback| {
                let font = BuiltinFont::courier(callback.pdf());
                let text = Text::basic("T E S T", &font, 1024.);
                let text = text
                    .debug(1)
                    .show_max_width()
                    .show_last_location_max_height();

                let shrink_to_fit = ShrinkToFit {
                    element: text,
                    min_height: 10.,
                };
                let shrink_to_fit = &shrink_to_fit
                    .debug(0)
                    .show_max_width()
                    .show_last_location_max_height();

                callback.call(shrink_to_fit);
            },
        );
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_align_location_bottom() {
        let bytes = test_element_bytes(
            TestElementParams {
                first_height: 20.,
                ..TestElementParams::breakable()
            },
            |mut callback| {
                let font = BuiltinFont::courier(callback.pdf());
                let text = Text::basic("Test", &font, 20.);
                let text = text
                    .debug(1)
                    .show_max_width()
                    .show_last_location_max_height();

                let bottom = AlignLocationBottom(text);
                let bottom = bottom.debug(2);

                let shrink_to_fit = ShrinkToFit {
                    element: bottom,
                    min_height: 10.,
                };
                let shrink_to_fit = &shrink_to_fit
                    .debug(0)
                    .show_max_width()
                    .show_last_location_max_height();

                callback.call(shrink_to_fit);
            },
        );
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_layers() {
        let bytes = test_element_bytes(
            TestElementParams {
                first_height: 20.,
                ..TestElementParams::breakable()
            },
            |mut callback| {
                let font = BuiltinFont::courier(callback.pdf());
                let text = Text::basic("Test", &font, 100.);
                let text = text
                    .debug(1)
                    .show_max_width()
                    .show_last_location_max_height();

                let wrapper = StyledBox {
                    outline: Some(LineStyle {
                        thickness: 12.,
                        color: 0x00_00_00_FF,
                        dash_pattern: None,
                        cap_style: LineCapStyle::Round,
                    }),
                    ..StyledBox::new(text)
                };
                let wrapper = wrapper.debug(2);

                let shrink_to_fit = ShrinkToFit {
                    element: wrapper,
                    min_height: 10.,
                };
                let shrink_to_fit = &shrink_to_fit
                    .debug(0)
                    .show_max_width()
                    .show_last_location_max_height();

                callback.call(shrink_to_fit);
            },
        );
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_nested_layers() {
        let bytes = test_element_bytes(
            TestElementParams {
                first_height: 30.,
                ..TestElementParams::breakable()
            },
            |mut callback| {
                let font = BuiltinFont::courier(callback.pdf());
                let text = Text::basic("Test", &font, 100.);
                let text = text
                    .debug(1)
                    .show_max_width()
                    .show_last_location_max_height();

                let wrapper = StyledBox {
                    outline: Some(LineStyle {
                        thickness: 10.,
                        color: 0x00_00_00_FF,
                        dash_pattern: None,
                        cap_style: LineCapStyle::Round,
                    }),
                    ..StyledBox::new(text)
                };
                let wrapper = wrapper.debug(2);
                let shrink_to_fit = ShrinkToFit {
                    element: wrapper,
                    min_height: 10.,
                };

                let wrapper_1 = StyledBox {
                    outline: Some(LineStyle {
                        thickness: 10.,
                        color: 0xAA_00_00_FF,
                        dash_pattern: None,
                        cap_style: LineCapStyle::Round,
                    }),
                    ..StyledBox::new(shrink_to_fit)
                };
                let wrapper_1 = wrapper_1.debug(3);

                let shrink_to_fit_1 = ShrinkToFit {
                    element: wrapper_1,
                    min_height: 10.,
                };
                let shrink_to_fit = &shrink_to_fit_1
                    .debug(0)
                    .show_max_width()
                    .show_last_location_max_height();

                callback.call(shrink_to_fit);
            },
        );
        assert_binary_snapshot!(".pdf", bytes);
    }
}
