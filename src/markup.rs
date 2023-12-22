use std::ops::Deref;

use printpdf::*;

use crate::break_text_into_lines::*;
use crate::text::remove_non_trailing_soft_hyphens;
use crate::utils::*;
use crate::{text::text_width, *};

use serde::{Deserialize, Serialize};

// #[derive(Clone, Debug, Deserialize)]
// pub enum Span {
//     Text(String),
//     Link {
//         text: String,
//         target: String,
//         title: String,
//     },
//     Emphasis(Vec<Span>),
//     Strong(Vec<Span>),
//     Break,
// }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: u32,
}

pub struct Spans<'a, D: Deref<Target = [u8]>> {
    pub spans: &'a [Span],
    pub size: f64,
    pub small_size: f64,
    pub extra_line_height: f64,
    pub regular: &'a crate::widget::Font<D>,
    pub bold: &'a crate::widget::Font<D>,
    pub italic: &'a crate::widget::Font<D>,
    pub bold_italic: &'a crate::widget::Font<D>,
}

pub struct LineFragment<'a, D: Deref<Target = [u8]>> {
    text_full: &'a str,
    length_full: f64,

    text_trimmed: &'a str,
    length_trimmed: f64,

    font: &'a crate::widget::Font<D>,
    size: f64,
    bold: bool,
    italic: bool,
    underline: bool,
    color: u32,
    ascent: f64,
    new_line: bool,
    x_offset: f64,
}

impl<'a, D: Deref<Target = [u8]>> Copy for LineFragment<'a, D> {}

impl<'a, D: Deref<Target = [u8]>> Clone for LineFragment<'a, D> {
    fn clone(&self) -> Self {
        *self
    }
}

#[derive(Copy, Clone)]
pub struct LineFragmentTrimmed<'a, D: Deref<Target = [u8]>> {
    text: &'a str,
    length: f64,

    font: &'a crate::widget::Font<D>,
    size: f64,
    bold: bool,
    italic: bool,
    underline: bool,
    color: u32,
    ascent: f64,
    new_line: bool,
    x_offset: f64,
}

impl<'a, D: Deref<Target = [u8]>> Spans<'a, D> {
    fn pieces(
        &'a self,
        width: Option<f64>,
    ) -> (impl Iterator<Item = LineFragment<'a, D>> + 'a, f64) {
        #[derive(Copy, Clone)]
        struct FontVars {
            ascent: f64,
            line_height: f64,
        }

        fn font_vars<D: Deref<Target = [u8]>>(
            font: &crate::widget::Font<D>,
            size: f64,
        ) -> FontVars {
            // some font related variables
            let v_metrics = font.font.get_v_metrics();
            let units_per_em = font.font.units_per_em() as f64;
            let ascent = pt_to_mm(v_metrics.ascent as f64 * size / units_per_em);
            let descent = pt_to_mm(v_metrics.descent as f64 * size / units_per_em);
            let line_gap = pt_to_mm(v_metrics.line_gap as f64 * size / units_per_em);
            let line_height = ascent - descent + line_gap;

            FontVars {
                ascent,
                line_height,
            }
        }

        fn mk_gen<'a, D: Deref<Target = [u8]>>(
            text: &'a str,
            font: &'a crate::widget::Font<D>,
            size: f64,
        ) -> LineGenerator<'a, impl Fn(&str) -> f64 + 'a> {
            let text_width = move |t: &str| text_width(t, size, &font.font, 0., 0.);
            LineGenerator::new(text, text_width)
        }

        let regular_vars = font_vars(self.regular, self.size as f64);
        let bold_vars = font_vars(self.bold, self.size as f64);
        let italic_vars = font_vars(self.italic, self.size as f64);
        let bold_italic_vars = font_vars(self.bold_italic, self.size as f64);

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
                                    let (font, font_vars): (&crate::widget::Font<D>, FontVars) =
                                        match (span.bold, span.italic) {
                                            (false, false) => (self.regular, regular_vars),
                                            (false, true) => (self.italic, italic_vars),
                                            (true, false) => (self.bold, bold_vars),
                                            (true, true) => (self.bold_italic, bold_italic_vars),
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
                        Some((ref mut gen, font, font_vars, bold, italic, underline, color)) => {
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
                                    pt_to_mm(text_width(trimmed, self.size, &font.font, 0., 0.));
                                let length_full = length_trimmed
                                    + pt_to_mm(text_width(
                                        &next[trimmed.len()..],
                                        self.size,
                                        &font.font,
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
                                    italic,
                                    underline,
                                    color,
                                    ascent: font_vars.ascent,
                                    // line_height,
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
    ) -> (impl Iterator<Item = LineFragmentTrimmed<'a, D>> + 'a, f64) {
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
                        italic: last_frag.italic,
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

impl<'a, D: Deref<Target = [u8]>> Element for Spans<'a, D> {
    fn draw(&self, width: Option<f64>, draw: Option<DrawCtx>) -> [f64; 2] {
        let mut max_width = if let Some(width) = width { width } else { 0. };

        let (iter, line_height) = self.pieces_trimmed(width);
        let line_height = line_height + self.extra_line_height;

        let line_count = if let Some(mut context) = draw {
            let mut x = context.location.pos[0];
            let mut y = context.location.pos[1]; // - ascent;

            let mut height_available = context.location.height_available;

            let mut draw_rect = 0;

            if height_available < line_height {
                if let Some(ref mut next_location) = context.next_location {
                    let new_location = next_location(context.pdf, 0, [max_width, 0.]);
                    draw_rect = 1;
                    x = new_location.pos[0];
                    y = new_location.pos[1];
                    height_available = new_location.height_available;
                    context.location.layer = new_location.layer;
                }
            }

            let mut line_count = 1;

            // let max_width: Option<&mut f64> = if width.is_none() {
            //     Some(&mut max_width)
            // } else { None };

            for frag in iter {
                let pdf_font = &frag.font.font_ref;

                let line_width = frag.length;

                // if let Some(&mut ref mut max_width) = max_width {
                // }
                max_width = max_width.max(frag.x_offset + line_width);

                if frag.new_line {
                    match context.next_location {
                        Some(ref mut next_location) if height_available < 2. * line_height => {
                            let new_location = next_location(
                                context.pdf,
                                draw_rect,
                                [max_width, line_count as f64 * line_height],
                            );
                            draw_rect += 1;

                            x = new_location.pos[0];
                            y = new_location.pos[1]; // - ascent;
                            height_available = new_location.height_available;
                            context.location.layer = new_location.layer;
                            line_count = 1;
                        }
                        _ => {
                            y -= line_height;
                            height_available -= line_height;
                            line_count += 1;
                        }
                    }
                }

                context.location.layer.save_graphics_state();
                context
                    .location
                    .layer
                    .set_fill_color(u32_to_color_and_alpha(frag.color).0);
                context.location.layer.use_text(
                    &remove_non_trailing_soft_hyphens(frag.text),
                    frag.size,
                    Mm(x + frag.x_offset),
                    Mm(y - frag.ascent),
                    pdf_font,
                );

                // This isn't quite correct currently. The truetype format has underline postion and
                // thickness information in the `post` table. This information is however not
                // exposed in the `stb_truetype` crate. To get this information we'll have to switch
                // to another crate, such as `ttf-parser`, which exposes the `post` table and the
                // underline information. For now we'll just use some hard-coded values that look
                // mostly right.
                if frag.underline {
                    context
                        .location
                        .layer
                        .set_outline_color(u32_to_color_and_alpha(frag.color).0);
                    crate::utils::line(
                        &context.location.layer,
                        [x + frag.x_offset, y - frag.ascent - 1.0],
                        pt_to_mm(text_width(frag.text, frag.size, &frag.font.font, 0., 0.)),
                        pt_to_mm(if frag.bold { 1.0 } else { 0.5 }),
                    );
                }
                context.location.layer.restore_graphics_state();
            }

            line_count
        } else {
            let mut line_count = 1;

            // let max_width: Option<&mut f64> = if width.is_none() {
            //     Some(&mut max_width)
            // } else { None };

            for frag in iter {
                let line_width = frag.length;

                max_width = max_width.max(frag.x_offset + line_width);
                // if let Some(&mut ref mut max_width) = max_width {
                // }

                if frag.new_line {
                    line_count += 1;
                }
            }

            line_count
        };

        Some(ElementSize {
            width: max_width,
            height: Some(line_count as f64 * line_height),
        })
    }

    // fn widget(&self, width: Option<f64>, draw: Option<DrawCtx>) -> [f64; 2] {
    //     #[derive(Copy, Clone)]
    //     struct FontVars {
    //         ascent: f64,
    //         line_height: f64,
    //     }

    //     fn font_vars(font: &crate::widget::Font, size: f64) -> FontVars {
    //         // some font related variables
    //         let v_metrics = font.font.get_v_metrics();
    //         let size = size as f64;
    //         let units_per_em = font.font.units_per_em() as f64;
    //         let ascent = pt_to_mm(v_metrics.ascent as f64 * size / units_per_em);
    //         let descent = pt_to_mm(v_metrics.descent as f64 * size / units_per_em);
    //         let line_gap = pt_to_mm(v_metrics.line_gap as f64 * size / units_per_em);
    //         let line_height = ascent - descent + line_gap;

    //         FontVars {
    //             ascent,
    //             line_height,
    //         }
    //     }

    //     let size = self.size as f64;
    //     let regular_vars = font_vars(self.regular, size);
    //     let bold_vars = font_vars(self.bold, size);
    //     let italic_vars = font_vars(self.italic, size);
    //     let bold_italic_vars = font_vars(self.bold_italic, size);

    //     let line_height = regular_vars.line_height
    //         .max(bold_vars.line_height)
    //         .max(italic_vars.line_height)
    //         .max(bold_italic_vars.line_height);

    //     let mut max_width = if let Some(width) = width { width } else { 0. };

    //     let line_count = if let Some(mut context) = draw {
    //         let mut x = context.location.pos[0];
    //         let mut y = context.location.pos[1];// - ascent;

    //         let mut height_available = context.location.height_available;

    //         let mut line_count = 0;

    //         let max_width: Option<&mut f64> = if let Some(_) = width { Some(&mut max_width) }
    //         else { None };

    //         let mut x_offset = 0.;

    //         #[derive(Copy, Clone)]
    //         struct UnfinishedLine<'a> {
    //             line: &'a str,
    //             font: &'a crate::widget::Font<'a>,
    //             font_vars: FontVars,
    //             underline: bool,
    //             color: u32,
    //             x_offset: f64,
    //         }

    //         let mut unfinished_line: Option<UnfinishedLine> = None;

    //         for span in self.spans {
    //             let (font, font_vars) = match (span.bold, span.italic) {
    //                 (false, false) => (self.regular, regular_vars),
    //                 (false, true) => (self.italic, italic_vars),
    //                 (true, false) => (self.bold, bold_vars),
    //                 (true, true) => (self.bold_italic, bold_italic_vars),
    //             };

    //             let pdf_font = &font.font_ref;

    //             let mut lines = LineGenerator {
    //                 text: &span.text,
    //                 text_width: |text| text_width(text, self.size, &font.font),
    //                 line_start: 0,
    //             };

    //             while let Some(line) =
    //                 if let Some(width) = width {
    //                     lines.next(
    //                         mm_to_pt(width - x_offset),
    //                         unfinished_line.is_some(),
    //                     )
    //                 } else { lines.next_unconstrained() }
    //             {
    //                 if let Some(ul) = unfinished_line {
    //                     let ul_line = if line.len() == 0 {
    //                         ul.line.trim_end()
    //                     } else { ul.line };

    //                     context.location.layer.save_graphics_state();
    //                     context.location.layer.set_fill_color(u32_to_color_and_alpha(ul.color).0);
    //                     context.location.layer.use_text(
    //                         ul_line,
    //                         size as i64,
    //                         Mm(x + ul.x_offset),
    //                         Mm(y - ul.font_vars.ascent),
    //                         &ul.font.font_ref,
    //                     );
    //                     if ul.underline {
    //                         crate::utils::line(
    //                             &context.location.layer,
    //                             [x + ul.x_offset, y - ul.font_vars.ascent - 1.0],
    //                             pt_to_mm(text_width(ul_line, self.size, &ul.font.font)),
    //                             pt_to_mm(2.0),
    //                         );
    //                     }
    //                     context.location.layer.restore_graphics_state();

    //                     unfinished_line = None;
    //                 } else {
    //                     if height_available < line_height {
    //                         if let Some(ref mut next_location) = context.next_location {
    //                             let new_location = next_location(context.pdf);
    //                             x = new_location.pos[0];
    //                             y = new_location.pos[1];// - ascent;
    //                             height_available = new_location.height_available;
    //                             context.location.layer = new_location.layer;
    //                             line_count = 0;
    //                         }
    //                     }
    //                 }

    //                 if lines.done() {
    //                     let line_width = pt_to_mm(text_width(line, self.size, &font.font));
    //                     unfinished_line = Some(UnfinishedLine {
    //                         line,
    //                         font,
    //                         font_vars,
    //                         underline: span.underline,
    //                         color: span.color,
    //                         x_offset,
    //                     });
    //                     x_offset += line_width;
    //                 } else {
    //                     let line = line.trim_end();

    //                     let line_width = pt_to_mm(text_width(line, self.size, &font.font));

    //                     if let Some(&mut ref mut max_width) = max_width {
    //                         *max_width = max_width.max(x_offset + line_width);
    //                     }

    //                     context.location.layer.save_graphics_state();
    //                     context.location.layer.set_fill_color(u32_to_color_and_alpha(span.color).0);
    //                     context.location.layer.use_text(
    //                         line,
    //                         size as i64,
    //                         Mm(x + x_offset),
    //                         Mm(y - font_vars.ascent),
    //                         pdf_font,
    //                     );
    //                     if span.underline {
    //                         crate::utils::line(
    //                             &context.location.layer,
    //                             [x + x_offset, y - font_vars.ascent - 1.0],
    //                             pt_to_mm(text_width(line, self.size, &font.font)),
    //                             pt_to_mm(2.0),
    //                         );
    //                     }
    //                     context.location.layer.restore_graphics_state();

    //                     y -= line_height;
    //                     height_available -= line_height;
    //                     line_count += 1;
    //                     x_offset = 0.;
    //                 }
    //             }

    //             // x_offset = last_offset;
    //         }

    //         if let Some(ul) = unfinished_line {
    //             let ul_line = ul.line.trim_end();

    //             if let Some(&mut ref mut max_width) = max_width {
    //                 *max_width = max_width.max(x_offset);
    //             }

    //             context.location.layer.save_graphics_state();
    //             context.location.layer.set_fill_color(u32_to_color_and_alpha(ul.color).0);
    //             context.location.layer.use_text(
    //                 ul_line,
    //                 size as i64,
    //                 Mm(x + ul.x_offset),
    //                 Mm(y - ul.font_vars.ascent),
    //                 &ul.font.font_ref,
    //             );
    //             if ul.underline {
    //                 crate::utils::line(
    //                     &context.location.layer,
    //                     [x + ul.x_offset, y - ul.font_vars.ascent - 1.0],
    //                     pt_to_mm(text_width(ul_line, self.size, &ul.font.font)),
    //                     pt_to_mm(2.0),
    //                 );
    //             }
    //             context.location.layer.restore_graphics_state();
    //         }

    //         line_count
    //     } else {
    //         1
    //     };

    //     [max_width, line_count as f64 * line_height]

    //     // if let Some(width) = width {
    //     //     let lines = break_text_into_lines(
    //     //         self.text,
    //     //         mm_to_pt(width),
    //     //         |text| text_width(text, self.size, &self.font.font)
    //     //     );

    //     //     let height = render_lines(
    //     //         lines,
    //     //         self.size,
    //     //         self.font,
    //     //         self.color,
    //     //         draw,
    //     //         ascent,
    //     //         line_height,
    //     //         self.underline,
    //     //         None,
    //     //     );

    //     //     [width, height]
    //     // } else {
    //     //     let mut width = 0.0;
    //     //     let lines = self.text.split('\n');

    //     //     let height = render_lines(
    //     //         lines,
    //     //         self.size,
    //     //         self.font,
    //     //         self.color,
    //     //         draw,
    //     //         ascent,
    //     //         line_height,
    //     //         self.underline,
    //     //         Some(&mut width),
    //     //     );

    //     //     [width, height]
    //     // }
    // }
}
