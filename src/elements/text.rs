use printpdf::types::pdf_layer::GappedTextElement;

use crate::{
    fonts::{Font, GeneralMetrics},
    text::{break_text_into_lines, remove_non_trailing_soft_hyphens, text_width},
    utils::{mm_to_pt, pt_to_mm, u32_to_color_and_alpha},
    *,
};

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

pub struct Text<'a, F: Font> {
    pub text: &'a str,
    pub font: &'a F,
    pub size: f64,
    pub color: u32,
    pub underline: bool,
    pub extra_character_spacing: f64,
    pub extra_word_spacing: f64,
    pub extra_line_height: f64,
    pub align: TextAlign,
}

struct FontMetrics {
    ascent: f64,
    line_height: f64,
}

impl<'a, F: Font> Text<'a, F> {
    pub fn basic(text: &'a str, font: &'a F, size: f64) -> Self {
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

        let units_per_em = self.font.units_per_em() as f64;

        FontMetrics {
            ascent: pt_to_mm(ascent * self.size / units_per_em),
            line_height: pt_to_mm(line_height * self.size / units_per_em) + self.extra_line_height,
        }
    }

    #[inline(always)]
    fn render_lines<'b, L: Iterator<Item = &'b str>>(
        &self,
        lines: L,
        mut ctx: DrawCtx,
        ascent: f64,
        line_height: f64,
        width: f64,
    ) -> (f64, f64) {
        let mut max_width = width;

        let mut x = ctx.location.pos.0;
        let mut y = ctx.location.pos.1 - ascent;

        let mut height_available = ctx.first_height;

        let pdf_font = &self.font.indirect_font_ref();

        let mut line_count = 0;
        let mut draw_rect = 0;

        for line in lines {
            let line: &str = &remove_non_trailing_soft_hyphens(line);

            let line_width = pt_to_mm(text_width(
                line,
                self.size,
                self.font,
                self.extra_character_spacing,
                self.extra_word_spacing,
            ));
            max_width = max_width.max(line_width);

            if height_available < line_height {
                if let Some(ref mut breakable) = ctx.breakable {
                    let new_location = (breakable.do_break)(
                        ctx.pdf,
                        draw_rect,
                        if line_count == 0 {
                            None
                        } else {
                            Some(line_count as f64 * line_height)
                        },
                    );
                    draw_rect += 1;
                    x = new_location.pos.0;
                    y = new_location.pos.1 - ascent;
                    height_available = breakable.full_height;
                    ctx.location.layer = new_location.layer;
                    line_count = 0;
                }
            }

            ctx.location.layer.save_graphics_state();
            ctx.location
                .layer
                .set_fill_color(u32_to_color_and_alpha(self.color).0);

            if self.extra_character_spacing != 0. {
                ctx.location
                    .layer
                    .set_character_spacing(self.extra_character_spacing);
            }

            let x_offset = match self.align {
                TextAlign::Left => 0.,
                TextAlign::Center => (width - line_width) / 2.,
                TextAlign::Right => width - line_width,
            };

            let x = x + x_offset;

            if self.extra_word_spacing != 0. {
                ctx.location.layer.begin_text_section();
                ctx.location.layer.set_font(pdf_font, self.size);
                ctx.location.layer.set_text_cursor(Mm(x), Mm(y));

                let word_spacing = self.extra_word_spacing * 1000. / self.size;

                ctx.location.layer.write_gapped_text(
                    line.split_inclusive(" ").flat_map(|s| {
                        std::iter::once(GappedTextElement::Text(s)).chain(if s.ends_with(' ') {
                            Some(GappedTextElement::Gap(word_spacing))
                        } else {
                            None
                        })
                    }),
                    pdf_font,
                );
                ctx.location.layer.end_text_section();
            } else {
                ctx.location
                    .layer
                    .use_text(line, self.size, Mm(x), Mm(y), pdf_font);
            }

            if self.underline {
                crate::utils::line(&ctx.location.layer, [x, y - 1.0], line_width, pt_to_mm(2.0));
            }
            ctx.location.layer.restore_graphics_state();
            y -= line_height;
            height_available -= line_height;
            line_count += 1;
        }

        (max_width, line_count as f64 * line_height)
    }

    #[inline(always)]
    fn layout_lines<'b, L: Iterator<Item = &'b str>>(
        &self,
        lines: L,
        line_height: f64,
        measure_ctx: Option<&mut MeasureCtx>,
    ) -> (f64, f64) {
        let mut max_width: f64 = 0.;
        let mut line_count = 0;

        // This function is a bit hacky because it's both used for measure and for determining the
        // max line width in unconstrained-width contexts.
        let mut height_available = if let Some(&mut MeasureCtx { first_height, .. }) = measure_ctx {
            first_height
        } else {
            f64::INFINITY
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

            max_width = max_width.max(pt_to_mm(text_width(
                line,
                self.size,
                self.font,
                self.extra_character_spacing,
                self.extra_word_spacing,
            )));

            height_available -= line_height;
            line_count += 1;
        }

        (max_width, line_count as f64 * line_height)
    }

    fn break_into_lines(&'a self, width: f64) -> impl Iterator<Item = &'a str> + Clone {
        break_text_into_lines(self.text, mm_to_pt(width), move |text| {
            text_width(
                text,
                self.size,
                self.font,
                self.extra_character_spacing,
                self.extra_word_spacing,
            )
        })
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

        let size = self.layout_lines(
            self.break_into_lines(ctx.width.max),
            line_height,
            Some(&mut ctx),
        );

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

        let lines = self.break_into_lines(ctx.width.max);

        // For left alignment we don't need to pre-layout because the
        // x offset is always zero.
        let width = if ctx.width.expand {
            ctx.width.max
        } else if self.align == TextAlign::Left {
            0.
        } else {
            self.layout_lines(lines.clone(), line_height, None).0
        };

        let width_constraint = ctx.width;
        let size = self.render_lines(lines, ctx, ascent, line_height, width);

        ElementSize {
            width: Some(width_constraint.constrain(size.0)),
            height: Some(size.1),
        }
    }
}

#[cfg(test)]
mod tests {
    use insta::*;
    use printpdf::PdfDocument;

    use crate::test_utils::binary_snapshots::*;
    use crate::{
        fonts::builtin::BuiltinFont,
        test_utils::{ElementProxy, ElementTestParams},
        DrawCtx, ElementSize,
    };

    use super::*;

    #[test]
    fn test_multi_page() {
        let bytes = test_element_bytes(TestElementParams::breakable(), |callback| {
            let font = BuiltinFont::courier(callback.document());

            let content = Text::basic(LOREM_IPSUM, &font, 32.);
            let content = content.debug(0);

            callback.call(&content);
        });
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_text() {
        // A fake document for adding the font to.
        let doc = PdfDocument::empty("i contain a font");

        let font = BuiltinFont::helvetica(&doc);

        let text_element = Text {
            ..Text::basic("i am a line\nso am i", &font, 12.)
        };

        let element = ElementProxy {
            before_draw: &|ctx: &mut DrawCtx| {
                // These seem to be stored in a map by name and when drawing a font with the same
                // needs to exist in the document being drawn on.
                ctx.pdf
                    .document
                    .add_builtin_font(printpdf::BuiltinFont::Helvetica)
                    .unwrap();
            },
            ..ElementProxy::new(text_element)
        };

        for mut output in (ElementTestParams {
            first_height: 4.,
            full_height: 5.,
            ..Default::default()
        })
        .run(&element)
        {
            if let Some(ref mut b) = output.breakable {
                b.assert_break_count(if output.first_height == 4. { 2 } else { 1 });
            }

            output.assert_size(ElementSize {
                width: Some(output.width.constrain(19.291312152)),

                // Note: I'm not sure this line height is correct. When running the same test with
                // Nimbus Sans L, which is supposed to be fully metrically compatible with
                // helvetica (at least according to the readme in
                // https://git.ghostscript.com/?p=user/tor/urw-base-12.git), the height ends up
                // being slightly more. On the x axis it matches exactly though. It's possible that
                // the bounding box in afm is not meant to be the equivalent of ascent + descent +
                // line gap in ttf. NimbusSans-Regular.afm from the following repo has the same
                // numbers https://git.ghostscript.com/?p=urw-core35-fonts.git as the Adobe one
                // and when running with NimbusSans-Regular.ttf from that repo the numbers are much
                // closer, which is reassuring (that one uses 2048 units per em so that should
                // explain the slight difference). The numbers for that one are 19.293924914062497
                // and 4.86792298828125.
                height: Some(4.893736415999999 * if output.breakable.is_some() { 1. } else { 2. }),
            });
        }
    }
}
