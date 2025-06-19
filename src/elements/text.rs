use fonts::GeneralMetrics;
use text::{Line, Lines, draw_line, lines};
use utils::{mm_to_pt, pt_to_mm, set_fill_color};

use crate::{fonts::Font, *};

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

/// A text element that renders text content with various styling options.
pub struct Text<'a, F: Font> {
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
    /// Text alignment
    pub align: TextAlign,
}

struct FontMetrics {
    ascent: f32,
    line_height: f32,
}

impl<'a, F: Font> Text<'a, F> {
    pub fn basic(text: &'a str, font: &'a F, size: f32) -> Self {
        Text {
            text,
            font,
            size,
            color: 0x00_00_00_FF,
            underline: false,
            extra_character_spacing: 0.,
            extra_word_spacing: 0.,
            extra_line_height: 0.,
            align: TextAlign::Left,
        }
    }

    fn compute_font_metrics(&self) -> FontMetrics {
        let GeneralMetrics {
            ascent,
            line_height,
        } = self.font.general_metrics();

        let units_per_em = self.font.units_per_em() as f32;

        FontMetrics {
            ascent: pt_to_mm(ascent as f32 * self.size / units_per_em),
            line_height: pt_to_mm(line_height as f32 * self.size / units_per_em)
                + self.extra_line_height,
        }
    }

    #[inline(always)]
    fn render_lines<'b, L: Iterator<Item = Line<F::Shaped<'a>>>>(
        &self,
        text: &str,
        lines: L,
        mut ctx: DrawCtx,
        ascent: f32,
        line_height: f32,
        width: f32,
    ) -> (f32, f32) {
        let mut max_width = width;
        let mut last_line_full_width = 0;

        let mut x = ctx.location.pos.0;
        let mut y = ctx.location.pos.1 - ascent;

        let mut height_available = ctx.first_height;

        let mut line_count = 0;
        let mut draw_rect = 0;

        let start = |pdf: &mut Pdf, location: &Location| {
            let layer = location.layer(pdf);
            layer
                .save_state()
                .set_font(self.font.resource_name(), self.size);
            set_fill_color(layer, self.color);
            layer.begin_text();
        };

        let end = |pdf: &mut Pdf, location: &Location| {
            location.layer(pdf).end_text().restore_state();
        };

        start(ctx.pdf, &ctx.location);

        for line in lines {
            let line_width =
                pt_to_mm(line.width as f32 / self.font.units_per_em() as f32 * self.size);
            max_width = max_width.max(line_width);

            last_line_full_width = line.width + line.trailing_whitespace_width;

            if height_available < line_height {
                if let Some(ref mut breakable) = ctx.breakable {
                    end(ctx.pdf, &ctx.location);

                    let new_location = (breakable.do_break)(
                        ctx.pdf,
                        draw_rect,
                        if line_count == 0 {
                            None
                        } else {
                            Some(line_count as f32 * line_height)
                        },
                    );
                    draw_rect += 1;
                    x = new_location.pos.0;
                    y = new_location.pos.1 - ascent;
                    height_available = breakable.full_height;
                    ctx.location.page_idx = new_location.page_idx;
                    ctx.location.layer_idx = new_location.layer_idx;
                    line_count = 0;

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

            layer.set_text_matrix([1.0, 0.0, 0.0, 1.0, mm_to_pt(x), mm_to_pt(y)]);

            draw_line(ctx.pdf, &ctx.location, self.font, text, line);

            y -= line_height;
            height_available -= line_height;
            line_count += 1;
        }

        end(ctx.pdf, &ctx.location);

        (
            max_width.max(pt_to_mm(
                last_line_full_width as f32 / self.font.units_per_em() as f32 * self.size,
            )),
            line_count as f32 * line_height,
        )
    }

    #[inline(always)]
    fn layout_lines<'b, L: Iterator<Item = Line<F::Shaped<'a>>>>(
        &self,
        lines: L,
        line_height: f32,
        measure_ctx: Option<&mut MeasureCtx>,
    ) -> (f32, f32) {
        let mut max_width = 0;
        let mut last_line_full_width = 0;
        let mut line_count = 0;

        // This function is a bit hacky because it's both used for measure and for determining the
        // max line width in unconstrained-width contexts.
        let mut height_available = if let Some(&mut MeasureCtx { first_height, .. }) = measure_ctx {
            first_height
        } else {
            f32::INFINITY
        };

        for line in lines {
            if let Some(&mut MeasureCtx {
                breakable: Some(ref mut breakable),
                ..
            }) = measure_ctx
            {
                if height_available < line_height {
                    *breakable.break_count += 1;
                    height_available = breakable.full_height;
                    line_count = 0;
                }
            }

            max_width = max_width.max(line.width);
            last_line_full_width = line.width + line.trailing_whitespace_width;

            height_available -= line_height;
            line_count += 1;
        }

        (
            pt_to_mm(
                max_width.max(last_line_full_width) as f32 / self.font.units_per_em() as f32
                    * self.size,
            ),
            line_count as f32 * line_height,
        )
    }

    fn break_into_lines<R>(
        &'a self,
        width: f32,
        f: impl for<'b> FnOnce(Lines<'a, 'b, F>) -> R,
    ) -> R {
        lines(
            self.font,
            (self.extra_character_spacing / self.size * self.font.units_per_em() as f32) as i32,
            (self.extra_word_spacing / self.size * self.font.units_per_em() as f32) as i32,
            // The ceil is to prevent rounding errors from causing problems in cases where the
            // element gets measured and then the measured width gets used for draw, such as in
            // HAlign.
            (mm_to_pt(width) / self.size * self.font.units_per_em() as f32).ceil() as u32,
            self.text,
            f,
        )
    }
}

impl<'a, F: Font> Element for Text<'a, F> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        let FontMetrics {
            ascent: _,
            line_height,
        } = self.compute_font_metrics();

        if line_height > ctx.first_height {
            FirstLocationUsage::WillSkip
        } else {
            FirstLocationUsage::WillUse
        }
    }

    fn measure(&self, mut ctx: MeasureCtx) -> ElementSize {
        let FontMetrics { line_height, .. } = self.compute_font_metrics();

        let size = self.break_into_lines(ctx.width.max, |lines| {
            self.layout_lines(lines, line_height, Some(&mut ctx))
        });

        ElementSize {
            width: Some(ctx.width.constrain(size.0)),
            height: Some(size.1),
        }
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        let FontMetrics {
            ascent,
            line_height,
        } = self.compute_font_metrics();

        // For left alignment we don't need to pre-layout because the
        // x offset is always zero.
        let width = if ctx.width.expand {
            ctx.width.max
        } else if self.align == TextAlign::Left {
            0.
        } else {
            // TODO: Figure out a way to avoid shaping twice here.
            self.break_into_lines(ctx.width.max, |lines| {
                self.layout_lines(lines, line_height, None).0
            })
        };

        let width_constraint = ctx.width;
        let size = self.break_into_lines(ctx.width.max, |lines| {
            self.render_lines(self.text, lines, ctx, ascent, line_height, width)
        });

        ElementSize {
            width: Some(width_constraint.constrain(size.0)),
            height: Some(size.1),
        }
    }
}

#[cfg(test)]
mod tests {
    use elements::column::Column;
    use fonts::truetype::TruetypeFont;
    use insta::*;

    use crate::test_utils::binary_snapshots::*;
    use crate::{
        DrawCtx, ElementSize,
        fonts::builtin::BuiltinFont,
        test_utils::{ElementProxy, ElementTestParams},
    };

    use super::*;

    const FONT: &[u8] = include_bytes!("../fonts/Kenney Bold.ttf");

    #[test]
    fn test_multi_page() {
        let bytes = test_element_bytes(TestElementParams::breakable(), |mut callback| {
            let font = BuiltinFont::courier(callback.pdf());

            let content = Text::basic(LOREM_IPSUM, &font, 32.);
            let content = content.debug(0);

            callback.call(&content);
        });
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_truetype() {
        let bytes = test_element_bytes(TestElementParams::breakable(), |mut callback| {
            let font = TruetypeFont::new(callback.pdf(), FONT);

            let content = Text::basic(LOREM_IPSUM, &font, 32.);
            let content = content.debug(0);

            callback.call(&content);
        });
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_truetype_trailing_whitespace() {
        let mut params = TestElementParams::breakable();
        params.width.expand = false;

        let bytes = test_element_bytes(params, |mut callback| {
            let font = TruetypeFont::new(callback.pdf(), FONT);

            let content = Text::basic("Whitespace ", &font, 32.);
            let content = content.debug(0);

            callback.call(&content);
        });
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_truetype_extra_spacing() {
        let mut params = TestElementParams::breakable();
        params.width.expand = false;

        let bytes = test_element_bytes(params, |mut callback| {
            let font = TruetypeFont::new(callback.pdf(), FONT);

            callback.call(&Column {
                gap: 12.,
                collapse: false,
                content: |content| {
                    let normal = Text::basic("Hello, World", &font, 32.);

                    let character_spacing = Text {
                        extra_character_spacing: 16.,
                        ..Text::basic("Hello, World", &font, 32.)
                    };

                    let word_spacing = Text {
                        extra_word_spacing: 16.,
                        ..Text::basic("Hello, World", &font, 32.)
                    };

                    let both = Text {
                        extra_character_spacing: 16.,
                        extra_word_spacing: 16.,
                        ..Text::basic("Hello, World", &font, 32.)
                    };

                    content
                        .add(&normal.debug(0).show_max_width())?
                        .add(&character_spacing.debug(1).show_max_width())?
                        .add(&word_spacing.debug(2).show_max_width())?
                        .add(&both.debug(3).show_max_width())?;

                    None
                },
            });
        });
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_truetype_soft_hyphen() {
        let mut params = TestElementParams::breakable();
        params.width.expand = false;

        let bytes = test_element_bytes(params, |mut callback| {
            let font = TruetypeFont::new(callback.pdf(), FONT);

            callback.call(&Column {
                gap: 12.,
                collapse: false,
                content: |content| {
                    let a = Text::basic("Hello\u{00AD}Wrld", &font, 32.);
                    let b = Text::basic("A Hello\u{00AD}Wrld", &font, 32.);
                    let c = Text::basic("A\u{00A0}Hello\u{00AD}Wrld", &font, 32.);
                    let d = Text::basic("Hello\u{00AD}Wrld\u{00AD}", &font, 32.);

                    content
                        .add(&Padding::right(100., a.debug(0).show_max_width()))?
                        .add(&Padding::right(120., b.debug(0).show_max_width()))?
                        .add(&Padding::right(120., c.debug(0).show_max_width()))?
                        .add(&Padding::right(20., d.debug(0).show_max_width()))?;

                    None
                },
            });
        });
        assert_binary_snapshot!(".pdf", bytes);
    }

    // #[test]
    // fn test_text() {
    //     // A fake document for adding the font to.
    //     let doc = PdfDocument::empty("i contain a font");

    //     let font = BuiltinFont::helvetica(&doc);

    //     let text_element = Text {
    //         ..Text::basic("i am a line\nso am i", &font, 12.)
    //     };

    //     let element = ElementProxy {
    //         before_draw: &|ctx: &mut DrawCtx| {
    //             // These seem to be stored in a map by name and when drawing a font with the same
    //             // needs to exist in the document being drawn on.
    //             ctx.pdf
    //                 .document
    //                 .add_builtin_font(printpdf::BuiltinFont::Helvetica)
    //                 .unwrap();
    //         },
    //         ..ElementProxy::new(text_element)
    //     };

    //     for mut output in (ElementTestParams {
    //         first_height: 4.,
    //         full_height: 5.,
    //         ..Default::default()
    //     })
    //     .run(&element)
    //     {
    //         if let Some(ref mut b) = output.breakable {
    //             b.assert_break_count(if output.first_height == 4. { 2 } else { 1 });
    //         }

    //         output.assert_size(ElementSize {
    //             width: Some(output.width.constrain(19.291312152)),

    //             // Note: I'm not sure this line height is correct. When running the same test with
    //             // Nimbus Sans L, which is supposed to be fully metrically compatible with
    //             // helvetica (at least according to the readme in
    //             // https://git.ghostscript.com/?p=user/tor/urw-base-12.git), the height ends up
    //             // being slightly more. On the x axis it matches exactly though. It's possible that
    //             // the bounding box in afm is not meant to be the equivalent of ascent + descent +
    //             // line gap in ttf. NimbusSans-Regular.afm from the following repo has the same
    //             // numbers https://git.ghostscript.com/?p=urw-core35-fonts.git as the Adobe one
    //             // and when running with NimbusSans-Regular.ttf from that repo the numbers are much
    //             // closer, which is reassuring (that one uses 2048 units per em so that should
    //             // explain the slight difference). The numbers for that one are 19.293924914062497
    //             // and 4.86792298828125.
    //             height: Some(4.893736415999999 * if output.breakable.is_some() { 1. } else { 2. }),
    //         });
    //     }
    // }
}
