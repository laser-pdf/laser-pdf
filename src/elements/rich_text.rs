use crate::{
    text::{Line, Piece, draw_line, lines_from_pieces},
    utils::{mm_to_pt, pt_to_mm},
    *,
};

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug)]
pub struct Span<'a, F> {
    /// The text content to render
    pub text: &'a str,
    /// Font reference
    pub font: &'a F,
    /// Font size in points
    pub size: f32,
    /// Text color as RGBA (default: black 0x00_00_00_FF)
    pub color: u32,
    /// Whether to underline the text
    pub underline: bool,
    /// Additional spacing between characters
    pub extra_character_spacing: f32,
    /// Additional spacing between words
    pub extra_word_spacing: f32,
    /// Additional line height
    pub extra_line_height: f32,
}

// This is a manual impl because we don't need the `F: Clone` constraint.
impl<'a, F> Clone for Span<'a, F> {
    fn clone(&self) -> Self {
        Self {
            text: self.text,
            font: self.font,
            size: self.size.clone(),
            color: self.color.clone(),
            underline: self.underline.clone(),
            extra_character_spacing: self.extra_character_spacing.clone(),
            extra_word_spacing: self.extra_word_spacing.clone(),
            extra_line_height: self.extra_line_height.clone(),
        }
    }
}

/// An element for displaying text with mixed fonts, sizes, colors, etc.
///
/// Note: Newline characters belong to both the line they end and the next line. So if you have a
/// newline character at the end of a span with a larger font than the next one, the line after the
/// one terminated by the newline will have at least the height of the larger font as well (it could
/// also be more depending on where the fonts baselines are). This behavior also means that if there
/// are no more spans after one terminated by a newline, the empty line at the end will have the
/// height of the font of the span containing the newline.
pub struct RichText<S> {
    pub spans: S,
    pub align: TextAlign,
}

impl<'a, F: Font + 'a, S: Iterator<Item = Span<'a, F>> + Clone> Element for RichText<S> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        let mut lines = self.break_into_lines(ctx.text_pieces_cache, ctx.width.max);
        let Some(first_line) = lines.next() else {
            return FirstLocationUsage::NoneHeight;
        };

        let line_height =
            pt_to_mm(first_line.height_above_baseline + first_line.height_below_baseline);

        if line_height > ctx.first_height {
            FirstLocationUsage::WillSkip
        } else {
            FirstLocationUsage::WillUse
        }
    }

    fn measure(&self, mut ctx: MeasureCtx) -> ElementSize {
        let lines = self.break_into_lines(ctx.text_pieces_cache, ctx.width.max);
        let size = self.layout_lines(lines, Some(&mut ctx));

        ElementSize {
            width: size.map(|s| ctx.width.max(s.0)),
            height: size.map(|s| s.1),
        }
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        // For left alignment we don't need to pre-layout because the
        // x offset is always zero.
        let width = if ctx.width.expand {
            ctx.width.max
        } else if self.align == TextAlign::Left {
            0.
        } else {
            let lines = self.break_into_lines(ctx.text_pieces_cache, ctx.width.max);
            let Some((width, _)) = self.layout_lines(lines, None) else {
                return ElementSize {
                    width: None,
                    height: None,
                };
            };
            width
        };

        let width_constraint = ctx.width;
        let lines = self.break_into_lines(ctx.text_pieces_cache, ctx.width.max);
        let size = self.render_lines(lines, ctx, width);

        ElementSize {
            width: size.map(|s| width_constraint.max(s.0)),
            height: size.map(|s| s.1),
        }
    }
}

impl<'a, F: Font + 'a, S: Iterator<Item = Span<'a, F>> + Clone> RichText<S> {
    #[inline(always)]
    fn render_lines<'c, L: Iterator<Item = Line<'c, F, impl Iterator<Item = (&'c F, &'c Piece)>>>>(
        &self,
        lines: L,
        mut ctx: DrawCtx,
        width: f32,
    ) -> Option<(f32, f32)>
    where
        F: 'c,
    {
        let mut max_width = width;
        let mut last_line_full_width = 0.;

        let mut x = ctx.location.pos.0;

        // This in points because there's no reason to work with mm here.
        let mut y = mm_to_pt(ctx.location.pos.1);

        let mut height_available = ctx.first_height;

        let mut line_count = 0;
        let mut draw_rect = 0;

        let mut height = 0.;

        let start = |pdf: &mut Pdf, location: &Location| {
            let layer = location.layer(pdf);
            layer.save_state();
            layer.begin_text();
        };

        let end = |pdf: &mut Pdf, location: &Location| {
            location.layer(pdf).end_text().restore_state();
        };

        start(ctx.pdf, &ctx.location);

        for line in lines {
            let line_height = pt_to_mm(line.height_above_baseline + line.height_below_baseline);
            let height_above_baseline = line.height_above_baseline;
            let height_below_baseline = line.height_below_baseline;

            let line_width = pt_to_mm(line.width);
            max_width = max_width.max(line_width);

            last_line_full_width = line.width + line.trailing_whitespace_width;

            if height_available < line_height {
                if let Some(ref mut breakable) = ctx.breakable {
                    end(ctx.pdf, &ctx.location);

                    let new_location = (breakable.do_break)(
                        ctx.pdf,
                        draw_rect,
                        if line_count == 0 { None } else { Some(height) },
                    );
                    draw_rect += 1;
                    x = new_location.pos.0;
                    y = mm_to_pt(new_location.pos.1);
                    height_available = breakable.full_height;
                    ctx.location.page_idx = new_location.page_idx;
                    ctx.location.layer_idx = new_location.layer_idx;
                    line_count = 0;
                    height = 0.;

                    start(ctx.pdf, &ctx.location);
                }
            }

            let layer = ctx.location.layer(ctx.pdf);

            let x_offset = match self.align {
                TextAlign::Left => 0.,
                TextAlign::Center => (width - line_width) / 2.,
                TextAlign::Right => width - line_width,
            };

            let x = x + x_offset;

            y -= height_above_baseline;

            layer.set_text_matrix([1.0, 0.0, 0.0, 1.0, mm_to_pt(x), y]);

            draw_line(ctx.pdf, &ctx.location, line);

            y -= height_below_baseline;
            height_available -= line_height;
            line_count += 1;
            height += line_height;
        }

        end(ctx.pdf, &ctx.location);

        (line_count > 0).then_some((max_width.max(pt_to_mm(last_line_full_width)), height))
    }

    #[inline(always)]
    fn layout_lines<'c, L: Iterator<Item = Line<'c, F, impl Iterator<Item = (&'c F, &'c Piece)>>>>(
        &self,
        lines: L,
        measure_ctx: Option<&mut MeasureCtx>,
    ) -> Option<(f32, f32)>
    where
        F: 'c,
    {
        let mut max_width: f32 = 0.;
        let mut last_line_full_width: f32 = 0.;
        let mut height = 0.;

        // This function is a bit hacky because it's both used for measure and for determining the
        // max line width in unconstrained-width contexts.
        let mut height_available = if let Some(&mut MeasureCtx { first_height, .. }) = measure_ctx {
            first_height
        } else {
            f32::INFINITY
        };

        let mut line_count = 0;

        for line in lines {
            let line_height = pt_to_mm(line.height_above_baseline + line.height_below_baseline);

            if let Some(&mut MeasureCtx {
                breakable: Some(ref mut breakable),
                ..
            }) = measure_ctx
            {
                if height_available < line_height {
                    *breakable.break_count += 1;
                    height_available = breakable.full_height;
                    height = 0.;
                    line_count = 0;
                }
            }

            max_width = max_width.max(line.width);
            last_line_full_width = line.width + line.trailing_whitespace_width;

            height_available -= line_height;
            height += line_height;
            line_count += 1;
        }

        (line_count > 0).then_some((pt_to_mm(max_width.max(last_line_full_width)), height))
    }

    fn break_into_lines<'b>(
        &'b self,
        text_pieces_cache: &'b TextPiecesCache,
        width: f32,
    ) -> impl Iterator<Item = Line<'b, F, impl Iterator<Item = (&'b F, &'b Piece)>>>
    where
        'a: 'b,
    {
        let pieces = self.spans.clone().flat_map(|span| {
            let pieces = text_pieces_cache.pieces(
                span.text,
                span.font,
                span.size,
                span.color,
                span.extra_character_spacing,
                span.extra_word_spacing,
                mm_to_pt(span.extra_line_height),
            );

            pieces.into_iter().map(move |p| (span.font, p))
        });

        // The `next_up` mitigates a problem when we get passed the width we returned from
        // measuring. In some cases it would then put one more piece onto the next line. This likely
        // doesn't fix the problem in all cases. TODO
        let lines = lines_from_pieces(pieces, mm_to_pt(width).next_up());

        lines
    }
}

#[cfg(test)]
mod tests {
    use elements::column::{Column, ColumnContent};
    use fonts::{builtin::BuiltinFont, truetype::TruetypeFont};
    use insta::*;

    use crate::{elements::ref_element::RefElement, test_utils::binary_snapshots::*};

    use super::*;

    #[test]
    fn test_truetype() {
        let bytes = test_element_bytes(TestElementParams::breakable(), |mut callback| {
            let regular =
                TruetypeFont::new(callback.pdf(), include_bytes!("../fonts/Kenney Future.ttf"));
            let bold =
                TruetypeFont::new(callback.pdf(), include_bytes!("../fonts/Kenney Bold.ttf"));

            let rich_text = RichText {
                spans: [
                    Span {
                        text: "Where are ",
                        font: &regular,
                        size: 12.,
                        underline: false,
                        color: 0x00_00_00_FF,
                        extra_character_spacing: 0.,
                        extra_word_spacing: 0.,
                        extra_line_height: 0.,
                    },
                    Span {
                        text: "they",
                        font: &bold,
                        size: 12.,
                        underline: false,
                        color: 0x00_00_FF_FF,
                        extra_character_spacing: 0.,
                        extra_word_spacing: 0.,
                        extra_line_height: 0.,
                    },
                    Span {
                        text: "\n",
                        font: &bold,
                        size: 12.,
                        underline: false,
                        color: 0x00_00_FF_FF,
                        extra_character_spacing: 0.,
                        extra_word_spacing: 0.,
                        extra_line_height: 0.,
                    },
                    Span {
                        text: "at?",
                        font: &regular,
                        size: 12.,
                        underline: false,
                        color: 0xFF_00_00_FF,
                        extra_character_spacing: 0.,
                        extra_word_spacing: 0.,
                        extra_line_height: 0.,
                    },
                ]
                .into_iter(),
                align: TextAlign::Left,
            };

            let list = Column {
                gap: 16.,
                collapse: false,
                content: |content: ColumnContent| {
                    content
                        .add(&RefElement(&rich_text).debug(0))?
                        .add(&Padding::right(
                            140.,
                            RefElement(&rich_text).debug(1).show_max_width(),
                        ))?
                        .add(&Padding::right(
                            160.,
                            RefElement(&rich_text).debug(2).show_max_width(),
                        ))?
                        .add(&Padding::right(
                            180.,
                            RefElement(&rich_text).debug(3).show_max_width(),
                        ))?
                        .add(&Padding::right(
                            194.,
                            RefElement(&rich_text).debug(4).show_max_width(),
                        ))?;
                    None
                },
            };

            callback.call(&list);
        });
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_truetype_trailing_whitespace() {
        let mut params = TestElementParams::breakable();
        params.width.expand = false;

        let bytes = test_element_bytes(params, |mut callback| {
            let regular =
                TruetypeFont::new(callback.pdf(), include_bytes!("../fonts/Kenney Future.ttf"));
            let bold =
                TruetypeFont::new(callback.pdf(), include_bytes!("../fonts/Kenney Bold.ttf"));

            let rich_text = RichText {
                spans: [
                    Span {
                        text: "Where are ",
                        font: &regular,
                        size: 12.,
                        underline: false,
                        color: 0x00_00_00_FF,
                        extra_character_spacing: 0.,
                        extra_word_spacing: 0.,
                        extra_line_height: 0.,
                    },
                    Span {
                        text: "they ",
                        font: &bold,
                        size: 12.,
                        underline: false,
                        color: 0x00_FF_00_FF,
                        extra_character_spacing: 0.,
                        extra_word_spacing: 0.,
                        extra_line_height: 0.,
                    },
                    Span {
                        text: "at?        ",
                        font: &regular,
                        size: 12.,
                        underline: false,
                        color: 0xFF_00_00_FF,
                        extra_character_spacing: 0.,
                        extra_word_spacing: 0.,
                        extra_line_height: 0.,
                    },
                ]
                .into_iter(),
                align: TextAlign::Left,
            };

            let list = Column {
                gap: 16.,
                collapse: false,
                content: |content: ColumnContent| {
                    content
                        .add(&RefElement(&rich_text).debug(0))?
                        .add(&Padding::right(
                            145.,
                            RefElement(&rich_text).debug(1).show_max_width(),
                        ))?
                        .add(&Padding::right(
                            160.,
                            RefElement(&rich_text).debug(2).show_max_width(),
                        ))?
                        .add(&Padding::right(
                            180.,
                            RefElement(&rich_text).debug(3).show_max_width(),
                        ))?
                        .add(&Padding::right(
                            194.,
                            RefElement(&rich_text).debug(4).show_max_width(),
                        ))?;
                    None
                },
            };

            callback.call(&list);
        });
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_truetype_small() {
        let bytes = test_element_bytes(
            TestElementParams::breakable().no_expand(),
            |mut callback| {
                let regular =
                    TruetypeFont::new(callback.pdf(), include_bytes!("../fonts/Kenney Future.ttf"));
                let bold =
                    TruetypeFont::new(callback.pdf(), include_bytes!("../fonts/Kenney Bold.ttf"));

                let rich_text = RichText {
                    spans: [
                        Span {
                            text: "Where are ",
                            font: &regular,
                            size: 12.,
                            underline: false,
                            color: 0x00_00_00_FF,
                            extra_character_spacing: 0.,
                            extra_word_spacing: 0.,
                            extra_line_height: 0.,
                        },
                        Span {
                            text: "they ",
                            font: &bold,
                            size: 4.,
                            underline: false,
                            color: 0x00_00_FF_FF,
                            extra_character_spacing: 0.,
                            extra_word_spacing: 0.,
                            extra_line_height: 0.,
                        },
                        Span {
                            text: "they",
                            font: &regular,
                            size: 4.,
                            underline: false,
                            color: 0x00_FF_FF_FF,
                            extra_character_spacing: 0.,
                            extra_word_spacing: 0.,
                            extra_line_height: 0.,
                        },
                        Span {
                            text: " at?",
                            font: &regular,
                            size: 12.,
                            underline: false,
                            color: 0xFF_FF_00_FF,
                            extra_character_spacing: 0.,
                            extra_word_spacing: 0.,
                            extra_line_height: 0.,
                        },
                    ]
                    .into_iter(),
                    align: TextAlign::Left,
                };

                let list = Column {
                    gap: 16.,
                    collapse: false,
                    content: |content: ColumnContent| {
                        content
                            .add(&RefElement(&rich_text).debug(0).show_max_width())?
                            .add(&Padding::right(
                                140.,
                                RefElement(&rich_text).debug(1).show_max_width(),
                            ))?
                            .add(&Padding::right(
                                155.,
                                RefElement(&rich_text).debug(2).show_max_width(),
                            ))?
                            .add(&Padding::right(
                                180.,
                                RefElement(&rich_text).debug(3).show_max_width(),
                            ))?
                            .add(&Padding::right(
                                194.,
                                RefElement(&rich_text).debug(4).show_max_width(),
                            ))?;
                        None
                    },
                };

                callback.call(&list);
            },
        );
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_small() {
        let bytes = test_element_bytes(
            TestElementParams::breakable().no_expand(),
            |mut callback| {
                let regular = BuiltinFont::helvetica(callback.pdf());
                let bold = BuiltinFont::helvetica_bold(callback.pdf());

                let rich_text = RichText {
                    spans: [
                        Span {
                            text: "Where are ",
                            font: &regular,
                            underline: false,
                            color: 0x00_00_00_FF,
                            size: 12.,
                            extra_character_spacing: 0.,
                            extra_word_spacing: 0.,
                            extra_line_height: 0.,
                        },
                        Span {
                            text: "they ",
                            font: &bold,
                            underline: false,
                            color: 0x00_00_FF_FF,
                            size: 4.,
                            extra_character_spacing: 0.,
                            extra_word_spacing: 0.,
                            extra_line_height: 0.,
                        },
                        Span {
                            text: "they",
                            font: &regular,
                            underline: false,
                            color: 0x00_FF_FF_FF,
                            size: 4.,
                            extra_character_spacing: 0.,
                            extra_word_spacing: 0.,
                            extra_line_height: 0.,
                        },
                        Span {
                            text: " at?",
                            font: &regular,
                            underline: false,
                            color: 0xFF_FF_00_FF,
                            size: 12.,
                            extra_character_spacing: 0.,
                            extra_word_spacing: 0.,
                            extra_line_height: 0.,
                        },
                    ]
                    .into_iter(),
                    align: TextAlign::Left,
                };

                let list = Column {
                    gap: 16.,
                    collapse: false,
                    content: |content: ColumnContent| {
                        content
                            .add(&RefElement(&rich_text).debug(0).show_max_width())?
                            .add(&Padding::right(
                                140.,
                                RefElement(&rich_text).debug(1).show_max_width(),
                            ))?
                            .add(&Padding::right(
                                155.,
                                RefElement(&rich_text).debug(2).show_max_width(),
                            ))?
                            .add(&Padding::right(
                                180.,
                                RefElement(&rich_text).debug(3).show_max_width(),
                            ))?
                            .add(&Padding::right(
                                194.,
                                RefElement(&rich_text).debug(4).show_max_width(),
                            ))?;
                        None
                    },
                };

                callback.call(&list);
            },
        );
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_no_rich_text_content() {
        let bytes = test_element_bytes(
            TestElementParams::breakable().no_expand(),
            |mut callback| {
                BuiltinFont::helvetica(callback.pdf());

                let spans: [Span<BuiltinFont>; 0] = [];

                let rich_text = RichText {
                    spans: spans.into_iter(),
                    align: TextAlign::Left,
                };

                let list = Column {
                    gap: 16.,
                    collapse: true,
                    content: |content: ColumnContent| {
                        content
                            .add(&RefElement(&rich_text).debug(0).show_max_width())?
                            .add(&Padding::top(
                                120.,
                                RefElement(&rich_text).debug(1).show_max_width(),
                            ))?;
                        None
                    },
                };

                callback.call(&list);
            },
        );
        assert_binary_snapshot!(".pdf", bytes);
    }
}
