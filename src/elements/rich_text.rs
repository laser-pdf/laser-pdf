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
    pub size: f32,
    pub small_size: f32,
    pub extra_line_height: f32,
    pub fonts: FontSet<'a, F>,
}

pub struct LineFragment<'a, F: Font> {
    text_full: &'a str,
    length_full: f32,

    text_trimmed: &'a str,
    length_trimmed: f32,

    font: &'a F,
    size: f32,
    bold: bool,
    underline: bool,
    color: u32,
    ascent: f32,
    new_line: bool,
    x_offset: f32,
}

// These are manually implemented because the derive macro would otherwise put a Copy bound on F.
impl<'a, F: Font> Copy for LineFragment<'a, F> {}

impl<'a, F: Font> Clone for LineFragment<'a, F> {
    fn clone(&self) -> Self {
        *self
    }
}

#[derive(Copy, Clone)]
pub struct LineFragmentTrimmed<'a, F: Font> {
    text: &'a str,
    length: f32,

    font: &'a F,
    size: f32,

    // needed for underline thickness
    bold: bool,

    underline: bool,
    color: u32,
    ascent: f32,
    new_line: bool,
    x_offset: f32,
}

impl<'a, F: Font> RichText<'a, F> {
    fn pieces(&'a self, width: f32) -> (impl Iterator<Item = LineFragment<'a, F>> + 'a, f32) {
        #[derive(Copy, Clone)]
        struct FontVars {
            ascent: f32,
            line_height: f32,
        }

        fn font_vars<F: Font>(font: &F, size: f32) -> FontVars {
            let GeneralMetrics {
                ascent,
                line_height,
            } = font.general_metrics(size);

            // let units_per_em = font.units_per_em() as f32;

            FontVars {
                ascent: pt_to_mm(ascent),
                line_height: pt_to_mm(line_height), // ascent: pt_to_mm(ascent * size / units_per_em),
                                                    // line_height: pt_to_mm(line_height * size / units_per_em),
            }
        }

        fn mk_gen<'a, F: Font>(text: &'a str, font: &'a F, size: f32) -> () {
            //LineGenerator<'a, impl Fn(&str) -> f32 + 'a> {
            todo!()
            // let text_width = move |t: &str| font.line_width(t, size, 0., 0.);
            // LineGenerator::new(text, text_width)
        }

        let regular_vars = font_vars(self.fonts.regular, self.size as f32);
        let bold_vars = font_vars(self.fonts.bold, self.size as f32);
        let italic_vars = font_vars(self.fonts.italic, self.size as f32);
        let bold_italic_vars = font_vars(self.fonts.bold_italic, self.size as f32);

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
                            let next = if let FirstLine | LineDone = line_state {
                                gen.next(width, false)
                            } else {
                                gen.next((width - x_offset).max(0.), true)
                            };

                            if let Some(next) = next {
                                let new_line = line_state == LineDone;
                                line_state = LineDone;

                                let trimmed = next.trim_end();
                                let length_trimmed = font.line_width(trimmed, self.size, 0., 0.);
                                let length_full = length_trimmed
                                    + font.line_width(&next[trimmed.len()..], self.size, 0., 0.);

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
        width: f32,
    ) -> (impl Iterator<Item = LineFragmentTrimmed<'a, F>> + 'a, f32) {
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
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        let (_, line_height) = self.pieces_trimmed(ctx.width.max);
        let line_height = line_height + self.extra_line_height;

        if ctx.first_height < line_height {
            FirstLocationUsage::WillSkip
        } else {
            FirstLocationUsage::WillUse
        }
    }

    fn measure(&self, mut ctx: MeasureCtx) -> ElementSize {
        let mut max_width = ctx.width.constrain(0.);

        let (iter, line_height) = self.pieces_trimmed(ctx.width.max);
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

        ElementSize {
            width: Some(max_width),
            height: Some(line_count as f32 * line_height),
        }
    }

    fn draw(&self, mut ctx: DrawCtx) -> ElementSize {
        let mut max_width = ctx.width.constrain(0.);

        let (iter, line_height) = self.pieces_trimmed(ctx.width.max);
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

        for frag in iter {
            // let pdf_font = &frag.font.indirect_font_ref();

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

            set_fill_color(layer, frag.color);

            // frag.font.render_line(
            //     layer,
            //     &remove_non_trailing_soft_hyphens(frag.text),
            //     frag.size,
            //     0.,
            //     0.,
            //     frag.underline,
            //     x + frag.x_offset,
            //     y - frag.ascent,
            // );

            layer.restore_state();
        }

        ElementSize {
            width: Some(max_width),
            height: Some(line_count as f32 * line_height),
        }
    }
}

#[cfg(test)]
mod tests {
    use insta::*;

    use crate::test_utils::binary_snapshots::*;
    use crate::{
        fonts::builtin::BuiltinFont,
        test_utils::{ElementProxy, ElementTestParams},
    };

    use super::*;

    #[test]
    fn test_basic() {
        let bytes = test_element_bytes(TestElementParams::breakable(), |mut callback| {
            let fonts = FontSet {
                regular: &BuiltinFont::courier(callback.pdf()),
                bold: &BuiltinFont::courier_bold(callback.pdf()),
                italic: &BuiltinFont::courier_oblique(callback.pdf()),
                bold_italic: &BuiltinFont::courier_bold_oblique(callback.pdf()),
            };

            // let content = Text::basic(LOREM_IPSUM, &font, 32.);
            let content = RichText {
                spans: &[
                    Span {
                        text: LOREM_IPSUM[0..5].to_string(),
                        bold: true,
                        italic: false,
                        underline: true,
                        color: 0x00_00_00_FF,
                    },
                    Span {
                        text: LOREM_IPSUM[5..].to_string(),
                        bold: false,
                        italic: false,
                        underline: false,
                        color: 0x00_00_00_FF,
                    },
                ],
                size: 12.,
                small_size: 7.,
                extra_line_height: 0.,
                fonts,
            };
            let content = content
                .debug(0)
                .show_max_width()
                .show_last_location_max_height();

            callback.call(&content);
        });
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_rich_text() {
        return;
        // A fake document for adding the fonts to.
        // let doc = PdfDocument::empty("i contain a font");
        let mut pdf = Pdf::new((0., 0.));

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
                regular: &BuiltinFont::courier(&mut pdf),
                bold: &BuiltinFont::courier_bold(&mut pdf),
                italic: &BuiltinFont::courier_oblique(&mut pdf),
                bold_italic: &BuiltinFont::courier_bold_oblique(&mut pdf),
            },
        };

        // Should be broken into lines like this:
        // Lorem
        // ipsum
        // dol or
        // sit
        // amet

        let element = ElementProxy {
            before_draw: &|ctx: &mut DrawCtx| {
                BuiltinFont::courier(ctx.pdf);
                BuiltinFont::courier_bold(ctx.pdf);
                BuiltinFont::courier_oblique(ctx.pdf);
                BuiltinFont::courier_bold_oblique(ctx.pdf);
            },
            ..ElementProxy::new(text_element)
        };

        // Since courier is a monospace font all letters should be the same width which makes our
        // math easier.
        let letter_width = 2.5400016;
        let line_height = 16.466169479999998;

        for mut output in (ElementTestParams {
            first_height: 2.,
            full_height: line_height,
            width: letter_width * 6.5,
            ..Default::default()
        })
        .run(&element)
        {
            if let Some(ref mut b) = output.breakable {
                b.assert_break_count(if output.first_height == 2. { 5 } else { 4 });
            }

            output.assert_size(ElementSize {
                width: Some(output.width.constrain(letter_width * 6.)),

                height: Some(line_height * if output.breakable.is_some() { 1. } else { 5. }),
            });
        }
    }
}
