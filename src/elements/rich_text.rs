use crate::fonts::Font;
use crate::fonts::GeneralMetrics;
// use crate::text::remove_non_trailing_soft_hyphens;
use crate::text::*;
use crate::utils::*;
use crate::*;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: u32,
}

pub struct RichText<'a, F: Font> {
    pub spans: &'a [Span],
    pub size: f32,
    pub small_size: f32,
    pub extra_line_height: f32,
    pub fonts: FontSet<'a, F>,
}

impl<'a, F: Font> Element for RichText<'a, F> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        let line_height = self.line_height();
        let line_height = line_height + self.extra_line_height;

        if ctx.first_height < line_height {
            FirstLocationUsage::WillSkip
        } else {
            FirstLocationUsage::WillUse
        }
    }

    fn measure(&self, mut ctx: MeasureCtx) -> ElementSize {
        let mut max_width = ctx.width.constrain(0.);

        let line_height = self.line_height();
        let line_height = line_height + self.extra_line_height;

        let mut height_available = ctx.first_height;

        if height_available < line_height {
            if let Some(ref mut breakable) = ctx.breakable {
                *breakable.break_count += 1;
                height_available = breakable.full_height;
            }
        }

        let mut line_count = 1;

        self.line_fragments(ctx.width.max, |frag| {
            let line_width = frag.length;

            max_width = max_width.max(frag.x_offset + line_width);

            if frag.new_line {
                match ctx.breakable {
                    Some(ref mut breakable) if height_available < 2. * line_height => {
                        *breakable.break_count += 1;

                        height_available = breakable.full_height;
                        line_count = 1;
                    }
                    _ => {
                        height_available -= line_height;
                        line_count += 1;
                    }
                }
            }
        });

        ElementSize {
            width: Some(max_width),
            height: Some(line_count as f32 * line_height),
        }
    }

    fn draw(&self, mut ctx: DrawCtx) -> ElementSize {
        let mut max_width = ctx.width.constrain(0.);

        let line_height = self.line_height();
        let line_height = line_height + self.extra_line_height;

        let mut x = ctx.location.pos.0;
        let mut y = ctx.location.pos.1;

        let mut height_available = ctx.first_height;

        let mut draw_rect = 0;

        if height_available < line_height {
            if let Some(ref mut breakable) = ctx.breakable {
                let new_location = (breakable.do_break)(ctx.pdf, draw_rect, None);
                draw_rect = 1;
                x = new_location.pos.0;
                y = new_location.pos.1;
                height_available = breakable.full_height;
                ctx.location.page_idx = new_location.page_idx;
                ctx.location.layer_idx = new_location.layer_idx;
            }
        }

        let mut line_count = 1;

        self.line_fragments(ctx.width.max, |frag| {
            let line_width = frag.length;

            max_width = max_width.max(frag.x_offset + line_width);

            if frag.new_line {
                match ctx.breakable {
                    Some(ref mut breakable) if height_available < 2. * line_height => {
                        let new_location = (breakable.do_break)(
                            ctx.pdf,
                            draw_rect,
                            Some(line_count as f32 * line_height),
                        );
                        draw_rect += 1;

                        x = new_location.pos.0;
                        y = new_location.pos.1;
                        height_available = breakable.full_height;
                        ctx.location.page_idx = new_location.page_idx;
                        ctx.location.layer_idx = new_location.layer_idx;
                        line_count = 1;
                    }
                    _ => {
                        y -= line_height;
                        height_available -= line_height;
                        line_count += 1;
                    }
                }
            }

            let layer = ctx.location.layer(ctx.pdf);
            layer.save_state();

            set_fill_color(layer, frag.span.color);

            let layer = ctx.location.layer(ctx.pdf);

            layer.save_state();
            set_fill_color(layer, frag.span.color);

            layer
                .set_font(frag.font.resource_name(), self.size)
                .set_text_matrix([
                    1.0,
                    0.0,
                    0.0,
                    1.0,
                    mm_to_pt(x + frag.x_offset),
                    mm_to_pt(y - line_height + frag.font_vars.line_height - frag.font_vars.ascent),
                ]);

            draw_line(
                ctx.pdf,
                &ctx.location,
                frag.font,
                &frag.span.text,
                frag.line,
            );

            ctx.location.layer(ctx.pdf).restore_state();
        });

        ElementSize {
            width: Some(max_width),
            height: Some(line_count as f32 * line_height),
        }
    }
}

struct LineFragment<'a, F: Font> {
    font: &'a F,
    font_vars: &'a FontVars,

    span: &'a Span,
    line: Line<F::Shaped<'a>>,

    /// Whether the fragment goes on a new line. So the line breaking has to happen before the
    /// fragment, not after.
    new_line: bool,

    length: f32,

    x_offset: f32,
}

impl<'a, F: Font> RichText<'a, F> {
    // Currently has to be an internal iterator because of LineGenerator, which works with a
    // callback. We'll probably have to change this at some point if we want to support justified
    // text. But for that we'll also need to solve the problem of the unicode segmentation iterator
    // not being cloneable.
    fn line_fragments(&self, width: f32, mut f: impl for<'b> FnMut(LineFragment<'b, F>)) {
        let regular_vars = font_vars(self.fonts.regular, self.size as f32);
        let bold_vars = font_vars(self.fonts.bold, self.size as f32);
        let italic_vars = font_vars(self.fonts.italic, self.size as f32);
        let bold_italic_vars = font_vars(self.fonts.bold_italic, self.size as f32);

        #[derive(PartialEq, Eq, Debug)]
        enum LineState {
            FirstLine,
            InLine,
            LineDone,
        }

        use LineState::*;

        let mut line_state = FirstLine;

        let mut x_offset_pt = 0.;

        for span in self.spans {
            let (font, font_vars): (&F, FontVars) = match (span.bold, span.italic) {
                (false, false) => (self.fonts.regular, regular_vars),
                (false, true) => (self.fonts.italic, italic_vars),
                (true, false) => (self.fonts.bold, bold_vars),
                (true, true) => (self.fonts.bold_italic, bold_italic_vars),
            };

            let mut last_line_empty = true;

            LineGenerator::new(font, &span.text, |mut generator| {
                while let Some(line) = generator.next(
                    ((mm_to_pt(width)
                        - (line_state == InLine).then_some(x_offset_pt).unwrap_or(0.))
                        / self.size
                        * font.units_per_em() as f32) as u32,
                    line_state == InLine,
                ) {
                    if line_state != InLine {
                        x_offset_pt = 0.;
                    }

                    let length =
                        pt_to_mm(line.width as f32 / font.units_per_em() as f32 * self.size);

                    let width = line.width;
                    let trailing_whitespace_width = line.trailing_whitespace_width;

                    last_line_empty = last_line_empty && line.empty;

                    // We need empty parts at the beginning of lines, otherwise trailing newlines
                    // on spans don't work. The reason we filter out empty fragments at all is so
                    // that we don't need add trailing whitespace to the width.
                    if !line.empty || line_state != InLine {
                        f(LineFragment {
                            font,
                            font_vars: &font_vars,
                            span,
                            line,
                            new_line: line_state == LineDone,
                            x_offset: pt_to_mm(x_offset_pt),
                            length,
                        });
                    }

                    x_offset_pt += (width + trailing_whitespace_width) as f32
                        / font.units_per_em() as f32
                        * self.size;
                    line_state = LineDone;
                }
            });

            line_state = if last_line_empty { FirstLine } else { InLine };
        }
    }

    fn line_height(&self) -> f32 {
        let regular_vars = font_vars(self.fonts.regular, self.size as f32);
        let bold_vars = font_vars(self.fonts.bold, self.size as f32);
        let italic_vars = font_vars(self.fonts.italic, self.size as f32);
        let bold_italic_vars = font_vars(self.fonts.bold_italic, self.size as f32);

        regular_vars
            .line_height
            .max(bold_vars.line_height)
            .max(italic_vars.line_height)
            .max(bold_italic_vars.line_height)
    }
}

#[derive(Copy, Clone)]
struct FontVars {
    ascent: f32,
    line_height: f32,
}

fn font_vars<F: Font>(font: &F, size: f32) -> FontVars {
    let GeneralMetrics {
        ascent,
        line_height,
    } = font.general_metrics();

    let units_per_em = font.units_per_em() as f32;

    FontVars {
        ascent: pt_to_mm(ascent as f32 * size / units_per_em),
        line_height: pt_to_mm(line_height as f32 * size / units_per_em),
    }
}

#[cfg(test)]
mod tests {
    use elements::column::{Column, ColumnContent};
    use fonts::{ShapedGlyph, truetype::TruetypeFont};
    use insta::*;

    use crate::test_utils::binary_snapshots::*;

    use super::*;

    #[derive(Debug)]
    struct FakeFont;

    #[derive(Clone, Debug)]
    struct FakeShaped<'a> {
        // last: usize,
        inner: std::str::CharIndices<'a>,
    }

    impl<'a> Iterator for FakeShaped<'a> {
        type Item = ShapedGlyph;

        fn next(&mut self) -> Option<Self::Item> {
            if let Some((i, c)) = self.inner.next() {
                Some(ShapedGlyph {
                    unsafe_to_break: false,
                    glyph_id: c as u32,
                    text_range: i..i + c.len_utf8(),
                    x_advance: if matches!(c, '\u{00ad}') { 0 } else { 1 },
                    x_offset: 0,
                    y_offset: 0,
                    y_advance: 0,
                })
            } else {
                None
            }
        }
    }

    impl Font for FakeFont {
        type Shaped<'a>
            = FakeShaped<'a>
        where
            Self: 'a;

        fn shape<'a>(&'a self, text: &'a str) -> Self::Shaped<'a> {
            FakeShaped {
                inner: text.char_indices(),
            }
        }

        fn encode(&self, _: &mut crate::Pdf, _: u32, _: &str) -> crate::fonts::EncodedGlyph {
            unimplemented!()
        }

        fn resource_name(&self) -> pdf_writer::Name {
            unimplemented!()
        }

        fn general_metrics(&self) -> GeneralMetrics {
            GeneralMetrics {
                ascent: 0,
                line_height: 1,
            }
        }

        fn units_per_em(&self) -> u16 {
            1
        }
    }

    #[test]
    fn test_line_fragments() {
        let font = FakeFont;

        let element = RichText {
            spans: &[
                Span {
                    text: "Where are ".to_string(),
                    bold: false,
                    italic: false,
                    underline: false,
                    color: 0x00_00_00_FF,
                },
                Span {
                    text: "they".to_string(),
                    bold: true,
                    italic: false,
                    underline: false,
                    color: 0x00_00_FF_FF,
                },
                Span {
                    text: "\n".to_string(),
                    bold: false,
                    italic: false,
                    underline: false,
                    color: 0x00_00_FF_FF,
                },
                Span {
                    text: "at?".to_string(),
                    bold: false,
                    italic: false,
                    underline: false,
                    color: 0xFF_00_00_FF,
                },
            ],
            size: 1.,
            small_size: 1.,
            extra_line_height: 0.,
            fonts: FontSet {
                regular: &font,
                bold: &font,
                italic: &font,
                bold_italic: &font,
            },
        };

        #[derive(Debug)]
        #[allow(dead_code)]
        struct Frag {
            new_line: bool,
            text: String,
            x_offset: f32,
            span: usize,
            empty: bool,
        }

        let collect = |width: f32| -> Vec<Frag> {
            let mut results = Vec::new();

            element.line_fragments(width, |mut fragment| {
                let empty = fragment.line.empty;
                let text = if let Some(first) = fragment.line.next() {
                    let last = fragment.line.last();

                    &fragment.span.text[first.text_range.start
                        ..last.map_or(first.text_range.end, |l| l.text_range.end)]
                } else {
                    ""
                };

                results.push(Frag {
                    text: text.to_string(),
                    new_line: fragment.new_line,
                    x_offset: mm_to_pt(fragment.x_offset),
                    span: element
                        .spans
                        .iter()
                        .enumerate()
                        .find_map(|(i, span)| std::ptr::eq(span, fragment.span).then_some(i))
                        .unwrap(),
                    empty,
                });
            });

            results
        };

        assert_debug_snapshot!(collect(0.), @r#"
        [
            Frag {
                new_line: false,
                text: "Where ",
                x_offset: 0.0,
                span: 0,
                empty: false,
            },
            Frag {
                new_line: true,
                text: "are ",
                x_offset: 0.0,
                span: 0,
                empty: false,
            },
            Frag {
                new_line: true,
                text: "they",
                x_offset: 0.0,
                span: 1,
                empty: false,
            },
            Frag {
                new_line: true,
                text: "",
                x_offset: 0.0,
                span: 2,
                empty: true,
            },
            Frag {
                new_line: false,
                text: "at?",
                x_offset: 0.0,
                span: 3,
                empty: false,
            },
        ]
        "#);
        assert_debug_snapshot!(collect(3.), @r#"
        [
            Frag {
                new_line: false,
                text: "Where ",
                x_offset: 0.0,
                span: 0,
                empty: false,
            },
            Frag {
                new_line: true,
                text: "are ",
                x_offset: 0.0,
                span: 0,
                empty: false,
            },
            Frag {
                new_line: false,
                text: "they",
                x_offset: 4.0,
                span: 1,
                empty: false,
            },
            Frag {
                new_line: true,
                text: "",
                x_offset: 0.0,
                span: 2,
                empty: true,
            },
            Frag {
                new_line: false,
                text: "at?",
                x_offset: 0.0,
                span: 3,
                empty: false,
            },
        ]
        "#);
        assert_debug_snapshot!(collect(13.), @r#"
        [
            Frag {
                new_line: false,
                text: "Where are ",
                x_offset: 0.0,
                span: 0,
                empty: false,
            },
            Frag {
                new_line: false,
                text: "they",
                x_offset: 10.0,
                span: 1,
                empty: false,
            },
            Frag {
                new_line: true,
                text: "",
                x_offset: 0.0,
                span: 2,
                empty: true,
            },
            Frag {
                new_line: false,
                text: "at?",
                x_offset: 0.0,
                span: 3,
                empty: false,
            },
        ]
        "#);
    }

    #[test]
    fn test_truetype() {
        let bytes = test_element_bytes(TestElementParams::breakable(), |mut callback| {
            let regular =
                TruetypeFont::new(callback.pdf(), include_bytes!("../fonts/Kenney Future.ttf"));
            let bold =
                TruetypeFont::new(callback.pdf(), include_bytes!("../fonts/Kenney Bold.ttf"));

            let rich_text = RichText {
                spans: &[
                    Span {
                        text: "Where are ".to_string(),
                        bold: false,
                        italic: false,
                        underline: false,
                        color: 0x00_00_00_FF,
                    },
                    Span {
                        text: "they".to_string(),
                        bold: true,
                        italic: false,
                        underline: false,
                        color: 0x00_00_FF_FF,
                    },
                    Span {
                        text: "\n".to_string(),
                        bold: true,
                        italic: false,
                        underline: false,
                        color: 0x00_00_FF_FF,
                    },
                    Span {
                        text: "at?".to_string(),
                        bold: false,
                        italic: false,
                        underline: false,
                        color: 0xFF_00_00_FF,
                    },
                ],
                size: 12.,
                small_size: 8.,
                extra_line_height: 0.,
                fonts: FontSet {
                    regular: &regular,
                    bold: &bold,
                    italic: &regular,
                    bold_italic: &bold,
                },
            };

            let list = Column {
                gap: 16.,
                collapse: false,
                content: |content: ColumnContent| {
                    content
                        .add(&rich_text.debug(0))?
                        .add(&Padding::right(140., &rich_text.debug(1).show_max_width()))?
                        .add(&Padding::right(160., &rich_text.debug(2).show_max_width()))?
                        .add(&Padding::right(180., &rich_text.debug(3).show_max_width()))?
                        .add(&Padding::right(194., &rich_text.debug(4).show_max_width()))?;
                    None
                },
            };

            callback.call(&list);
        });
        assert_binary_snapshot!(".pdf", bytes);
    }
}
