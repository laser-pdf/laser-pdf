use crate::fonts::Font;
use crate::fonts::GeneralMetrics;
use crate::text::remove_non_trailing_soft_hyphens;
use crate::text::*;
use crate::utils::*;
use crate::{text::text_width, *};

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
    pub size: f64,
    pub small_size: f64,
    pub extra_line_height: f64,
    pub fonts: FontSet<'a, F>,
}

pub struct LineFragment<'a, F: Font> {
    text_full: &'a str,
    length_full: f64,

    text_trimmed: &'a str,
    length_trimmed: f64,

    font: &'a F,
    size: f64,
    bold: bool,
    underline: bool,
    color: u32,
    ascent: f64,
    new_line: bool,
    x_offset: f64,
}

impl<'a, F: Font> Copy for LineFragment<'a, F> {}

impl<'a, F: Font> Clone for LineFragment<'a, F> {
    fn clone(&self) -> Self {
        *self
    }
}

#[derive(Copy, Clone)]
pub struct LineFragmentTrimmed<'a, F: Font> {
    text: &'a str,
    length: f64,

    font: &'a F,
    size: f64,

    // needed for underline thickness
    bold: bool,

    underline: bool,
    color: u32,
    ascent: f64,
    new_line: bool,
    x_offset: f64,
}

impl<'a, F: Font> RichText<'a, F> {
    fn pieces(
        &'a self,
        width: Option<f64>,
    ) -> (impl Iterator<Item = LineFragment<'a, F>> + 'a, f64) {
        #[derive(Copy, Clone)]
        struct FontVars {
            ascent: f64,
            line_height: f64,
        }

        fn font_vars<F: Font>(font: &F, size: f64) -> FontVars {
            let GeneralMetrics {
                ascent,
                line_height,
            } = font.general_metrics();

            let units_per_em = font.units_per_em() as f64;

            FontVars {
                ascent: pt_to_mm(ascent * size / units_per_em),
                line_height: pt_to_mm(line_height * size / units_per_em),
            }
        }

        fn mk_gen<'a, F: Font>(
            text: &'a str,
            font: &'a F,
            size: f64,
        ) -> LineGenerator<'a, impl Fn(&str) -> f64 + 'a> {
            let text_width = move |t: &str| text_width(t, size, font, 0., 0.);
            LineGenerator::new(text, text_width)
        }

        let regular_vars = font_vars(self.fonts.regular, self.size as f64);
        let bold_vars = font_vars(self.fonts.bold, self.size as f64);
        let italic_vars = font_vars(self.fonts.italic, self.size as f64);
        let bold_italic_vars = font_vars(self.fonts.bold_italic, self.size as f64);

        let line_height = regular_vars
            .line_height
            .max(bold_vars.line_height)
            .max(italic_vars.line_height)
            .max(bold_italic_vars.line_height);

        let mut spans = self.spans.iter();
        let mut generator = None;

        #[derive(PartialEq, Eq)]
        enum LineState {
            FirstLine,
            InLine,
            LineDone,
        }

        use LineState::*;

        let mut line_state = FirstLine;

        let mut x_offset = 0.;

        (
            std::iter::from_fn(move || {
                loop {
                    match generator {
                        None => {
                            if let Some(span) = spans.next() {
                                // this way we make sure the generator has at least one item
                                if span.text.len() > 0 {
                                    let (font, font_vars): (&F, FontVars) =
                                        match (span.bold, span.italic) {
                                            (false, false) => (self.fonts.regular, regular_vars),
                                            (false, true) => (self.fonts.italic, italic_vars),
                                            (true, false) => (self.fonts.bold, bold_vars),
                                            (true, true) => {
                                                (self.fonts.bold_italic, bold_italic_vars)
                                            }
                                        };

                                    generator = Some((
                                        mk_gen(&span.text, font, self.size),
                                        font,
                                        font_vars,
                                        span.bold,
                                        span.italic,
                                        span.underline,
                                        span.color,
                                    ));
                                }
                            } else {
                                break None;
                            }
                        }
                        Some((ref mut gen, font, font_vars, bold, _italic, underline, color)) => {
                            let next = if let Some(width) = width {
                                if let FirstLine | LineDone = line_state {
                                    gen.next(mm_to_pt(width), false)
                                } else {
                                    gen.next(mm_to_pt(width - x_offset).max(0.), true)
                                }
                            } else {
                                gen.next_unconstrained()
                            };

                            if let Some(next) = next {
                                let new_line = line_state == LineDone;
                                line_state = LineDone;

                                let trimmed = next.trim_end();
                                let length_trimmed =
                                    pt_to_mm(text_width(trimmed, self.size, font, 0., 0.));
                                let length_full = length_trimmed
                                    + pt_to_mm(text_width(
                                        &next[trimmed.len()..],
                                        self.size,
                                        font,
                                        0.,
                                        0.,
                                    ));

                                let ret_x_offset = if new_line { 0. } else { x_offset };
                                x_offset = if new_line {
                                    length_full
                                } else {
                                    x_offset + length_full
                                };

                                break Some(LineFragment {
                                    text_full: next,
                                    length_full,

                                    text_trimmed: trimmed,
                                    length_trimmed,

                                    font,
                                    size: self.size,
                                    bold,
                                    underline,
                                    color,
                                    ascent: font_vars.ascent,
                                    new_line,
                                    x_offset: ret_x_offset,
                                });
                            } else {
                                generator = None;
                                line_state = InLine;
                            }
                        }
                    }
                }
            })
            .filter(|i| i.new_line || i.text_trimmed.len() != 0),
            line_height,
        )
    }

    fn pieces_trimmed(
        &'a self,
        width: Option<f64>,
    ) -> (impl Iterator<Item = LineFragmentTrimmed<'a, F>> + 'a, f64) {
        let (mut iter, line_height) = self.pieces(width);

        let mut last = iter.next();

        (
            std::iter::from_fn(move || {
                if let Some(last_frag) = last {
                    last = iter.next();

                    let trim = if let Some(new) = last {
                        new.new_line
                    } else {
                        true
                    };

                    Some(LineFragmentTrimmed {
                        text: if trim {
                            last_frag.text_trimmed
                        } else {
                            last_frag.text_full
                        },
                        length: if trim {
                            last_frag.length_trimmed
                        } else {
                            last_frag.length_full
                        },

                        font: last_frag.font,
                        size: last_frag.size,
                        bold: last_frag.bold,
                        underline: last_frag.underline,
                        color: last_frag.color,
                        ascent: last_frag.ascent,
                        new_line: last_frag.new_line,
                        x_offset: last_frag.x_offset,
                    })
                } else {
                    None
                }
            }),
            line_height,
        )
    }
}

impl<'a, F: Font> Element for RichText<'a, F> {
    fn measure(&self, mut ctx: MeasureCtx) -> Option<ElementSize> {
        let mut max_width = if let Some(width) = ctx.width {
            width
        } else {
            0.
        };

        let (iter, line_height) = self.pieces_trimmed(ctx.width);
        let line_height = line_height + self.extra_line_height;

        let mut height_available = ctx.first_height;

        if height_available < line_height {
            if let Some(ref mut breakable) = ctx.breakable {
                *breakable.break_count += 1;
                height_available = breakable.full_height;
            }
        }

        let mut line_count = 1;

        for frag in iter {
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
        }

        Some(ElementSize {
            width: max_width,
            height: Some(line_count as f64 * line_height),
        })
    }

    fn draw(&self, mut ctx: DrawCtx) -> Option<ElementSize> {
        let mut max_width = if let Some(width) = ctx.width {
            width
        } else {
            0.
        };

        let (iter, line_height) = self.pieces_trimmed(ctx.width);
        let line_height = line_height + self.extra_line_height;

        let mut x = ctx.location.pos.0;
        let mut y = ctx.location.pos.1;

        let mut height_available = ctx.first_height;

        let mut draw_rect = 0;

        if height_available < line_height {
            if let Some(ref mut breakable) = ctx.breakable {
                let new_location = (breakable.get_location)(ctx.pdf, draw_rect);
                draw_rect = 1;
                x = new_location.pos.0;
                y = new_location.pos.1;
                height_available = breakable.full_height;
                ctx.location.layer = new_location.layer;
            }
        }

        let mut line_count = 1;

        for frag in iter {
            let pdf_font = &frag.font.indirect_font_ref();

            let line_width = frag.length;

            max_width = max_width.max(frag.x_offset + line_width);

            if frag.new_line {
                match ctx.breakable {
                    Some(ref mut breakable) if height_available < 2. * line_height => {
                        let new_location = (breakable.get_location)(ctx.pdf, draw_rect);
                        draw_rect += 1;

                        x = new_location.pos.0;
                        y = new_location.pos.1;
                        height_available = breakable.full_height;
                        ctx.location.layer = new_location.layer;
                        line_count = 1;
                    }
                    _ => {
                        y -= line_height;
                        height_available -= line_height;
                        line_count += 1;
                    }
                }
            }

            ctx.location.layer.save_graphics_state();
            ctx.location
                .layer
                .set_fill_color(u32_to_color_and_alpha(frag.color).0);
            ctx.location.layer.use_text(
                &remove_non_trailing_soft_hyphens(frag.text),
                frag.size,
                Mm(x + frag.x_offset),
                Mm(y - frag.ascent),
                pdf_font,
            );

            // This isn't quite correct currently. The truetype format has underline position and
            // thickness information in the `post` table. This information is however not
            // exposed in the `stb_truetype` crate. To get this information we'll have to switch
            // to another crate, such as `ttf-parser`, which exposes the `post` table and the
            // underline information. For now we'll just use some hard-coded values that look
            // mostly right.
            if frag.underline {
                ctx.location
                    .layer
                    .set_outline_color(u32_to_color_and_alpha(frag.color).0);
                crate::utils::line(
                    &ctx.location.layer,
                    [x + frag.x_offset, y - frag.ascent - 1.0],
                    pt_to_mm(text_width(frag.text, frag.size, frag.font, 0., 0.)),
                    pt_to_mm(if frag.bold { 1.0 } else { 0.5 }),
                );
            }
            ctx.location.layer.restore_graphics_state();
        }

        Some(ElementSize {
            width: max_width,
            height: Some(line_count as f64 * line_height),
        })
    }
}

#[cfg(test)]
mod tests {
    use printpdf::PdfDocument;

    use crate::{
        fonts::builtin::BuiltinFont,
        test_utils::{ElementProxy, ElementTestParams},
    };

    use super::*;

    #[test]
    fn test_rich_text() {
        // A fake document for adding the fonts to.
        let doc = PdfDocument::empty("i contain a font");

        let text_element = RichText {
            spans: &[
                Span {
                    text: "Lorem ip".to_string(),
                    bold: false,
                    italic: false,
                    underline: false,
                    color: 0,
                },
                Span {
                    text: "sum dol ".to_string(),
                    bold: true,
                    italic: true,
                    underline: false,
                    color: 0,
                },
                Span {
                    text: "or sit amet".to_string(),
                    bold: true,
                    italic: true,
                    underline: false,
                    color: 0,
                },
            ],
            size: 12.,
            small_size: 12.,
            extra_line_height: 12.,
            fonts: FontSet {
                regular: &BuiltinFont::courier(&doc),
                bold: &BuiltinFont::courier_bold(&doc),
                italic: &BuiltinFont::courier_oblique(&doc),
                bold_italic: &BuiltinFont::courier_bold_oblique(&doc),
            },
        };

        // Should be broken into lines like this:
        // Lorem
        // ipsum
        // dol or
        // sit
        // amet

        let element = ElementProxy {
            element: text_element,
            before_draw: |ctx: &mut DrawCtx| {
                // These seem to be stored in a map by name and when drawing a font with the same
                // needs to exist in the document being drawn on.
                ctx.pdf
                    .document
                    .add_builtin_font(printpdf::BuiltinFont::Courier)
                    .unwrap();
                ctx.pdf
                    .document
                    .add_builtin_font(printpdf::BuiltinFont::CourierBold)
                    .unwrap();
                ctx.pdf
                    .document
                    .add_builtin_font(printpdf::BuiltinFont::CourierOblique)
                    .unwrap();
                ctx.pdf
                    .document
                    .add_builtin_font(printpdf::BuiltinFont::CourierBoldOblique)
                    .unwrap();
            },
        };

        // Since courier is a monospace font all letters should be the same width which makes our
        // math easier.
        let letter_width = 2.5400016;
        let line_height = 16.466169479999998;

        for mut output in (ElementTestParams {
            first_height: 2.,
            full_height: line_height,
            width: letter_width * 6.,
            ..Default::default()
        })
        .run(&element)
        {
            if let Some(ref mut b) = output.breakable {
                b.assert_break_count(match (output.first_height == 2., output.width.is_some()) {
                    (false, false) => 0,
                    (false, true) => 4,
                    (true, false) => 1,
                    (true, true) => 5,
                });
            }

            output.assert_size(Some(ElementSize {
                width: if let Some(width) = output.width {
                    width
                } else {
                    letter_width * 27.
                },

                height: Some(
                    line_height
                        * if output.breakable.is_some() || output.width.is_none() {
                            1.
                        } else {
                            5.
                        },
                ),
            }));
        }
    }
}
