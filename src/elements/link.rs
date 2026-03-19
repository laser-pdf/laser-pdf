use crate::{
    utils::{add_link_annotation, mm_to_pt},
    *,
};

/// Wraps an element with a link annotation. If breaking happens, a separate annotation is added for
/// each location that isn't collapsed.
pub struct Link<'a, E: Element> {
    pub element: E,
    pub target: LinkTarget<'a>,
}

impl<'a, E: Element> Element for Link<'a, E> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        self.element.first_location_usage(ctx)
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        self.element.measure(ctx)
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        let size;

        if let Some(breakable) = ctx.breakable {
            let first_location = ctx.location.clone();

            let mut max_location_idx = 0;

            // We allocate here because for creating the annotations we need to know the width,
            // which we only get at the end. Since this is only on draw it shouldn't be a big issue.
            // We choose it over an additional measure pass first, which is definitely a bit of a
            // tradeoff. The expectation is that in most cases where this element is used it won't
            // break. This is an example where an arena allocator might be useful.
            let mut heights = Vec::new();

            size = self.element.draw(DrawCtx {
                pdf: ctx.pdf,
                breakable: Some(BreakableDraw {
                    do_break: &mut |pdf: &mut Pdf, location_idx: u32, height: Option<f32>| {
                        let location = (breakable.do_break)(pdf, location_idx, height);

                        if location_idx >= max_location_idx {
                            heights.resize(heights.len().max(location_idx as usize), None);
                            heights.push(height);

                            max_location_idx = location_idx + 1;
                        }

                        location
                    },
                    ..breakable
                }),
                ..ctx
            });

            if let Some(width) = size.width {
                for (i, height) in heights
                    .iter()
                    .cloned()
                    .chain(std::iter::once(size.height))
                    .enumerate()
                {
                    if let Some(height) = height {
                        let location = if i == 0 {
                            &first_location
                        } else {
                            &(breakable.do_break)(ctx.pdf, i as u32 - 1, heights[i - 1])
                        };
                        let page_idx = location.page_idx;
                        let pos = location.pos;

                        add_link_annotation(
                            ctx.pdf,
                            page_idx,
                            (mm_to_pt(pos.0), mm_to_pt(pos.1)),
                            (mm_to_pt(width), mm_to_pt(height)),
                            self.target,
                        );
                    }
                }
            }
        } else {
            let page_idx = ctx.location.page_idx;
            let pos = ctx.location.pos;

            size = self.element.draw(DrawCtx {
                pdf: ctx.pdf,
                ..ctx
            });

            if let ElementSize {
                width: Some(width),
                height: Some(height),
            } = size
            {
                add_link_annotation(
                    ctx.pdf,
                    page_idx,
                    (mm_to_pt(pos.0), mm_to_pt(pos.1)),
                    (mm_to_pt(width), mm_to_pt(height)),
                    self.target,
                );
            }
        };

        size
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        elements::{column::Column, force_break::ForceBreak, text::Text},
        fonts::builtin::BuiltinFont,
        test_utils::FranticJumper,
    };

    use super::*;

    #[test]
    fn test_basic() {
        use crate::test_utils::binary_snapshots::*;
        use insta::*;

        let bytes = test_element_bytes(TestElementParams::unbreakable(), |mut callback| {
            let font = BuiltinFont::courier(callback.pdf());

            let element = Text::new("test", &font, 11.);
            let element = element.debug(1).show_max_width();

            callback.call(
                &Link {
                    element,
                    target: LinkTarget::Uri("https://github.com/laser-pdf/laser-pdf"),
                }
                .debug(0)
                .show_max_width()
                .show_last_location_max_height(),
            );
        });
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_no_expand() {
        use crate::test_utils::binary_snapshots::*;
        use insta::*;

        let bytes = test_element_bytes(
            TestElementParams::breakable().no_expand(),
            |mut callback| {
                let font = BuiltinFont::courier(callback.pdf());

                let element = Text::new("test", &font, 11.);
                let element = element.debug(1).show_max_width();

                callback.call(
                    &Link {
                        element,
                        target: LinkTarget::Uri("https://github.com/laser-pdf/laser-pdf"),
                    }
                    .debug(0)
                    .show_max_width()
                    .show_last_location_max_height(),
                );
            },
        );
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_breaking() {
        use crate::test_utils::binary_snapshots::*;
        use insta::*;

        let bytes = test_element_bytes(
            TestElementParams::breakable().no_expand(),
            |mut callback| {
                let font = BuiltinFont::courier(callback.pdf());

                let element = Column::new(|content| {
                    content
                        .add(&Text::new("test", &font, 11.))?
                        .add(&ForceBreak)?
                        .add(&Text::new("test", &font, 11.))?
                        .add(&ForceBreak)?
                        .add(&ForceBreak)?
                        .add(&Text::new("test", &font, 11.))?;
                    None
                });
                let element = element.debug(1).show_max_width();

                callback.call(
                    &Link {
                        element,
                        target: LinkTarget::Uri("https://github.com/laser-pdf/laser-pdf"),
                    }
                    .debug(0)
                    .show_max_width()
                    .show_last_location_max_height(),
                );
            },
        );
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_frantic_breaking() {
        use crate::test_utils::binary_snapshots::*;
        use insta::*;

        let bytes = test_element_bytes(TestElementParams::breakable().no_expand(), |callback| {
            let element = FranticJumper {
                jumps: vec![(2, Some(8.)), (0, None)],
                size: ElementSize {
                    width: Some(12.),
                    height: Some(4.),
                },
            };
            let element = element.debug(1).show_max_width();

            callback.call(
                &Link {
                    element,
                    target: LinkTarget::Uri("https://github.com/laser-pdf/laser-pdf"),
                }
                .debug(0)
                .show_max_width()
                .show_last_location_max_height(),
            );
        });
        assert_binary_snapshot!(".pdf", bytes);
    }
}
