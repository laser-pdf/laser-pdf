use std::ops::Deref;

use printpdf::types::pdf_layer::GappedTextElement;
use printpdf::*;
use serde::Deserialize;
use serde::Serialize;
use stb_truetype as tt;

use crate::break_text_into_lines::*;
use crate::utils::*;
use crate::*;

/**
 * Calculates the width needed for a given string, font and size (in pt).
 */
pub fn text_width<D: Deref<Target = [u8]>>(
    text: &str,
    size: f64,
    font: &tt::FontInfo<D>,
    character_spacing: f64,
    word_spacing: f64,
) -> f64 {
    use itertools::{Itertools, Position};

    let scale = font.units_per_em() as f64;
    let character_spacing = character_spacing * scale / size;
    let word_spacing = word_spacing * scale / size;
    let total_width = text
        .chars()
        .with_position()
        .filter_map(|(p, ch)| {
            if ch == '\u{00ad}' && !matches!(p, Position::Last | Position::Only) {
                return None;
            }

            Some((ch, font.get_codepoint_h_metrics(ch as u32)))
        })
        .fold(0., |acc, (ch, h_metrics)| {
            acc + h_metrics.advance_width as f64
                + character_spacing
                + if ch == ' ' { word_spacing } else { 0. }
        });
    total_width as f64 * size as f64 / scale
}

pub fn remove_non_trailing_soft_hyphens(text: &str) -> String {
    use itertools::{Itertools, Position};

    text.chars()
        .with_position()
        .filter_map(|(p, c)| {
            if c != '\u{00ad}' || matches!(p, Position::Last | Position::Only) {
                Some(c)
            } else {
                None
            }
        })
        .collect()
}

pub struct Text<'a, D: Deref<Target = [u8]>>(pub &'a str, pub &'a crate::widget::Font<D>, pub f64);

impl<'a, D: Deref<Target = [u8]>> Element for Text<'a, D> {
    fn insufficient_first_height(&self, ctx: InsufficientFirstHeightCtx) -> bool {
        false
    }

    fn measure(&self, ctx: MeasureCtx) -> Option<ElementSize> {
        TextFull {
            text: self.0,
            font: self.1,
            size: self.2,
            color: 0x00_00_00_FF,
            underline: false,
            character_spacing: 0.,
            word_spacing: 0.,
            extra_line_height: 0.,
            align: TextAlign::Left,
        }.measure(ctx)
    }
    

    fn draw(&self, ctx: DrawCtx) -> Option<ElementSize> {
        TextFull {
            text: self.0,
            font: self.1,
            size: self.2,
            color: 0x00_00_00_FF,
            underline: false,
            character_spacing: 0.,
            word_spacing: 0.,
            extra_line_height: 0.,
            align: TextAlign::Left,
        }
        .draw(width, draw)
    }
}

pub struct TextColor<'a, D: Deref<Target = [u8]>>(
    pub &'a str,
    pub &'a crate::widget::Font<D>,
    pub f64,
    pub u32,
);

impl<'a, D: Deref<Target = [u8]>> Element for TextColor<'a, D> {
    fn insufficient_first_height(&self, ctx: InsufficientFirstHeightCtx) -> bool {
        false
    }

    fn measure(&self, ctx: MeasureCtx) -> Option<ElementSize> {
        TextFull {
            text: self.0,
            font: self.1,
            size: self.2,
            color: self.3,
            underline: false,
            character_spacing: 0.,
            word_spacing: 0.,
            extra_line_height: 0.,
            align: TextAlign::Left,
        }.measure(ctx)
    }
    

    fn draw(&self, ctx: DrawCtx) -> Option<ElementSize> {
        TextFull {
            text: self.0,
            font: self.1,
            size: self.2,
            color: self.3,
            underline: false,
            character_spacing: 0.,
            word_spacing: 0.,
            extra_line_height: 0.,
            align: TextAlign::Left,
        }
        .draw(width, draw)
    }
}

pub struct TextVertical<'a, D: Deref<Target = [u8]>> {
    pub text: &'a str,
    pub font: &'a crate::widget::Font<D>,
    pub size: u16,
    pub color: [f64; 3],
    pub underline: bool,
}

impl<'a, D: Deref<Target = [u8]>> Element for TextVertical<'a, D> {
    fn draw(&self, _width: Option<f64>, draw: Option<DrawCtx>) -> [f64; 2] {
        // some font related variables
        let v_metrics = self.font.font.get_v_metrics();
        let size = self.size as f64;
        let units_per_em = self.font.font.units_per_em() as f64;
        let ascent = pt_to_mm(v_metrics.ascent as f64 * size / units_per_em);
        let descent = pt_to_mm(v_metrics.descent as f64 * size / units_per_em);
        let line_gap = pt_to_mm(v_metrics.line_gap as f64 * size / units_per_em);
        let line_height = ascent - descent + line_gap;

        fn render_lines<'a, L: Iterator<Item = &'a str>, D: Deref<Target = [u8]>>(
            lines: L,
            size: u16,
            font: &'a crate::widget::Font<D>,
            color: [f64; 3],
            draw: Option<DrawCtx>,
            ascent: f64,
            line_height: f64,
            underline: bool,
            max_width: &mut f64,
        ) -> f64 {
            let line_count = if let Some(context) = draw {
                let x = context.location.pos[0];
                let mut y = context.location.pos[1];

                let mut height_available = context.location.height_available;

                let pdf_font = &font.font_ref;

                let mut line_count = 0;

                for line in lines {
                    *max_width =
                        max_width.max(pt_to_mm(text_width(line, size as f64, &font.font, 0., 0.)));

                    // if height_available < line_height {
                    //     if let Some(ref mut next_location) = context.next_location {
                    //         let new_location = next_location(
                    //             context.pdf,
                    //             [*max_width, line_count as f64 * line_height],
                    //         );
                    //         x = new_location.pos[0];
                    //         y = new_location.pos[1];
                    //         height_available = new_location.height_available;
                    //         context.location.layer = new_location.layer;
                    //         line_count = 0;
                    //     }
                    // }

                    context.location.layer.save_graphics_state();
                    context
                        .location
                        .layer
                        .set_fill_color(printpdf::Color::Rgb(Rgb::new(
                            color[0], color[1], color[2], None,
                        )));
                    context.location.layer.begin_text_section();
                    context.location.layer.set_font(pdf_font, size as f64);
                    context.location.layer.set_ctm(CurTransMat::Translate(
                        Mm(x + ascent),
                        Mm(y - pt_to_mm(text_width(line, size as f64, &font.font, 0., 0.))),
                    ));
                    context.location.layer.set_ctm(CurTransMat::Rotate(90.));
                    context.location.layer.write_text(line, pdf_font);
                    context.location.layer.end_text_section();

                    if underline {
                        crate::utils::line(
                            &context.location.layer,
                            [x, y - 1.0],
                            pt_to_mm(text_width(line, size as f64, &font.font, 0., 0.)),
                            pt_to_mm(2.0),
                        );
                    }
                    context.location.layer.restore_graphics_state();
                    y -= line_height;
                    height_available -= line_height;
                    line_count += 1;
                }

                line_count
            } else {
                let mut line_count = 0;
                for line in lines {
                    *max_width =
                        max_width.max(pt_to_mm(text_width(line, size as f64, &font.font, 0., 0.)));
                    line_count += 1;
                }
                line_count
                // if let Some(&mut ref mut max_width) = max_width {
                // } else {
                //     lines.count()
                // }
            };

            // This can be used if you don't want the line_gap to be added after the last line
            // [
            //     width,
            //     (line_count as f64 * (ascent - descent)) +
            //         // the gap is to be repeated one time less than the line count unless theres no lines
            //         (if line_count > 0 { line_count - 1 } else { line_count } as f64 * line_gap),
            // ]

            line_count as f64 * line_height
        }

        let mut width = 0.0;
        let lines = self.text.split('\n');

        let height = render_lines(
            lines,
            self.size,
            self.font,
            self.color,
            draw,
            ascent,
            line_height,
            self.underline,
            &mut width,
        );

        Some(ElementSize {
            width: height,
            height: Some(width),
        })
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

pub struct TextFull<'a, D: Deref<Target = [u8]>> {
    pub text: &'a str,
    pub font: &'a crate::widget::Font<D>,
    pub size: f64,
    // pub color: [f64; 3],
    pub color: u32,
    pub underline: bool,
    pub character_spacing: f64,
    pub word_spacing: f64,
    pub extra_line_height: f64,
    pub align: TextAlign,
}

impl<'a, D: Deref<Target = [u8]>> Element for TextFull<'a, D> {
    fn insufficient_first_height(&self, ctx: InsufficientFirstHeightCtx) -> bool {
        false
    }

    fn measure(&self, ctx: MeasureCtx) -> Option<ElementSize> {
        // some font related variables
        let v_metrics = self.font.font.get_v_metrics();
        let size = self.size as f64;
        let units_per_em = self.font.font.units_per_em() as f64;
        let ascent = pt_to_mm(v_metrics.ascent as f64 * size / units_per_em);
        let descent = pt_to_mm(v_metrics.descent as f64 * size / units_per_em);
        let line_gap = pt_to_mm(v_metrics.line_gap as f64 * size / units_per_em);
        let line_height = ascent - descent + line_gap + self.extra_line_height;

        #[inline(always)]
        fn render_lines<'a, L: Iterator<Item = &'a str>, D: Deref<Target = [u8]>>(
            lines: L,
            size: f64,
            font: &'a crate::widget::Font<D>,
            color: u32,
            mut context: DrawCtx,
            ascent: f64,
            line_height: f64,
            underline: bool,
            character_spacing: f64,
            word_spacing: f64,
            width: f64,
            align: TextAlign,
        ) -> [f64; 2] {
            let mut max_width = width;

            let mut x = context.location.pos[0];
            let mut y = context.location.pos[1] - ascent;

            let mut height_available = context.location.height_available;

            let pdf_font = &font.font_ref;

            let mut line_count = 0;
            let mut draw_rect = 0;

            for line in lines {
                let line: &str = &remove_non_trailing_soft_hyphens(line);

                let line_width = pt_to_mm(text_width(
                    line,
                    size,
                    &font.font,
                    character_spacing,
                    word_spacing,
                ));
                max_width = max_width.max(line_width);

                if height_available < line_height {
                    if let Some(ref mut next_location) = context.next_location {
                        let new_location = next_location(
                            context.pdf,
                            draw_rect,
                            [max_width, line_count as f64 * line_height],
                        );
                        draw_rect += 1;
                        x = new_location.pos[0];
                        y = new_location.pos[1] - ascent;
                        height_available = new_location.height_available;
                        context.location.layer = new_location.layer;
                        line_count = 0;
                    }
                }

                context.location.layer.save_graphics_state();
                context
                    .location
                    .layer
                    .set_fill_color(u32_to_color_and_alpha(color).0);

                if character_spacing != 0. {
                    context
                        .location
                        .layer
                        .set_character_spacing(character_spacing);
                }

                let x_offset = match align {
                    TextAlign::Left => 0.,
                    TextAlign::Center => (width - line_width) / 2.,
                    TextAlign::Right => width - line_width,
                };

                let x = x + x_offset;

                if word_spacing != 0. {
                    // context.location.layer.set_word_spacing(word_spacing);
                    context.location.layer.begin_text_section();
                    context.location.layer.set_font(pdf_font, size);
                    context.location.layer.set_text_cursor(Mm(x), Mm(y));

                    let word_spacing = word_spacing * 1000. / size;

                    context.location.layer.write_gapped_text(
                        line.split_inclusive(" ").flat_map(|s| {
                            std::iter::once(GappedTextElement::Text(s)).chain(if s.ends_with(' ') {
                                Some(GappedTextElement::Gap(word_spacing))
                            } else {
                                None
                            })
                        }),
                        pdf_font,
                    );
                    context.location.layer.end_text_section();
                    // context.draadd_op(Operation::new(
                    //     "TJ",
                    //     vec![Object::Array(w_pos.layer.use_
                    //     )],
                    // ));
                } else {
                    context
                        .location
                        .layer
                        .use_text(line, size, Mm(x), Mm(y), pdf_font);
                }

                if underline {
                    crate::utils::line(
                        &context.location.layer,
                        [x, y - 1.0],
                        line_width,
                        pt_to_mm(2.0),
                    );
                }
                context.location.layer.restore_graphics_state();
                y -= line_height;
                height_available -= line_height;
                line_count += 1;
            }

            Some(ElementSize {
                width: max_width,
                height: Some(line_count as f64 * line_height),
            })
        }

        fn layout_lines<'a, L: Iterator<Item = &'a str>, D: Deref<Target = [u8]>>(
            lines: L,
            size: f64,
            font: &'a crate::widget::Font<D>,
            line_height: f64,
            character_spacing: f64,
            word_spacing: f64,
        ) -> [f64; 2] {
            let mut max_width: f64 = 0.;
            let mut line_count = 0;
            for line in lines {
                max_width = max_width.max(pt_to_mm(text_width(
                    line,
                    size,
                    &font.font,
                    character_spacing,
                    word_spacing,
                )));
                line_count += 1;
            }
            Some(ElementSize {
                width: max_width,
                height: Some(line_count as f64 * line_height),
            })
        }

        if let Some(width) = width {
            let lines = break_text_into_lines(self.text, mm_to_pt(width), |text| {
                text_width(
                    text,
                    self.size,
                    &self.font.font,
                    self.character_spacing,
                    self.word_spacing,
                )
            });

            let height = layout_lines(
                lines,
                self.size,
                self.font,
                line_height,
                self.character_spacing,
                self.word_spacing,
            )[1];

            Some(ElementSize {
                width: width,
                height: Some(height),
            })
        } else {
            let lines = self.text.split('\n');

            layout_lines(
                lines,
                self.size,
                self.font,
                line_height,
                self.character_spacing,
                self.word_spacing,
            )
        }
    }

    fn draw(&self, ctx: DrawCtx) -> Option<ElementSize> {
        // some font related variables
        let v_metrics = self.font.font.get_v_metrics();
        let size = self.size as f64;
        let units_per_em = self.font.font.units_per_em() as f64;
        let ascent = pt_to_mm(v_metrics.ascent as f64 * size / units_per_em);
        let descent = pt_to_mm(v_metrics.descent as f64 * size / units_per_em);
        let line_gap = pt_to_mm(v_metrics.line_gap as f64 * size / units_per_em);
        let line_height = ascent - descent + line_gap + self.extra_line_height;

        #[inline(always)]
        fn render_lines<'a, L: Iterator<Item = &'a str>, D: Deref<Target = [u8]>>(
            lines: L,
            size: f64,
            font: &'a crate::widget::Font<D>,
            color: u32,
            mut context: DrawCtx,
            ascent: f64,
            line_height: f64,
            underline: bool,
            character_spacing: f64,
            word_spacing: f64,
            width: f64,
            align: TextAlign,
        ) -> [f64; 2] {
            let mut max_width = width;

            let mut x = context.location.pos[0];
            let mut y = context.location.pos[1] - ascent;

            let mut height_available = context.location.height_available;

            let pdf_font = &font.font_ref;

            let mut line_count = 0;
            let mut draw_rect = 0;

            for line in lines {
                let line: &str = &remove_non_trailing_soft_hyphens(line);

                let line_width = pt_to_mm(text_width(
                    line,
                    size,
                    &font.font,
                    character_spacing,
                    word_spacing,
                ));
                max_width = max_width.max(line_width);

                if height_available < line_height {
                    if let Some(ref mut next_location) = context.next_location {
                        let new_location = next_location(
                            context.pdf,
                            draw_rect,
                            [max_width, line_count as f64 * line_height],
                        );
                        draw_rect += 1;
                        x = new_location.pos[0];
                        y = new_location.pos[1] - ascent;
                        height_available = new_location.height_available;
                        context.location.layer = new_location.layer;
                        line_count = 0;
                    }
                }

                context.location.layer.save_graphics_state();
                context
                    .location
                    .layer
                    .set_fill_color(u32_to_color_and_alpha(color).0);

                if character_spacing != 0. {
                    context
                        .location
                        .layer
                        .set_character_spacing(character_spacing);
                }

                let x_offset = match align {
                    TextAlign::Left => 0.,
                    TextAlign::Center => (width - line_width) / 2.,
                    TextAlign::Right => width - line_width,
                };

                let x = x + x_offset;

                if word_spacing != 0. {
                    // context.location.layer.set_word_spacing(word_spacing);
                    context.location.layer.begin_text_section();
                    context.location.layer.set_font(pdf_font, size);
                    context.location.layer.set_text_cursor(Mm(x), Mm(y));

                    let word_spacing = word_spacing * 1000. / size;

                    context.location.layer.write_gapped_text(
                        line.split_inclusive(" ").flat_map(|s| {
                            std::iter::once(GappedTextElement::Text(s)).chain(if s.ends_with(' ') {
                                Some(GappedTextElement::Gap(word_spacing))
                            } else {
                                None
                            })
                        }),
                        pdf_font,
                    );
                    context.location.layer.end_text_section();
                    // context.draadd_op(Operation::new(
                    //     "TJ",
                    //     vec![Object::Array(w_pos.layer.use_
                    //     )],
                    // ));
                } else {
                    context
                        .location
                        .layer
                        .use_text(line, size, Mm(x), Mm(y), pdf_font);
                }

                if underline {
                    crate::utils::line(
                        &context.location.layer,
                        [x, y - 1.0],
                        line_width,
                        pt_to_mm(2.0),
                    );
                }
                context.location.layer.restore_graphics_state();
                y -= line_height;
                height_available -= line_height;
                line_count += 1;
            }

            Some(ElementSize {
                width: max_width,
                height: Some(line_count as f64 * line_height),
            })
        }

        fn layout_lines<'a, L: Iterator<Item = &'a str>, D: Deref<Target = [u8]>>(
            lines: L,
            size: f64,
            font: &'a crate::widget::Font<D>,
            line_height: f64,
            character_spacing: f64,
            word_spacing: f64,
        ) -> [f64; 2] {
            let mut max_width: f64 = 0.;
            let mut line_count = 0;
            for line in lines {
                max_width = max_width.max(pt_to_mm(text_width(
                    line,
                    size,
                    &font.font,
                    character_spacing,
                    word_spacing,
                )));
                line_count += 1;
            }
            Some(ElementSize {
                width: max_width,
                height: Some(line_count as f64 * line_height),
            })
        }

        if let Some(width) = width {
            let lines = break_text_into_lines(self.text, mm_to_pt(width), |text| {
                text_width(
                    text,
                    self.size,
                    &self.font.font,
                    self.character_spacing,
                    self.word_spacing,
                )
            });

            let height = render_lines(
                lines,
                self.size,
                self.font,
                self.color,
                context,
                ascent,
                line_height,
                self.underline,
                self.character_spacing,
                self.word_spacing,
                width,
                self.align,
            )[1];

            Some(ElementSize {
                width: width,
                height: Some(height),
            })
        } else {
            let lines = self.text.split('\n');

            // For left alignment we don't need to pre-layout because the
            // x offset is always zero.
            let width = if self.align == TextAlign::Left {
                0.
            } else {
                layout_lines(
                    lines.clone(),
                    self.size,
                    self.font,
                    line_height,
                    self.character_spacing,
                    self.word_spacing,
                )[0]
            };

            render_lines(
                lines,
                self.size,
                self.font,
                self.color,
                draw,
                ascent,
                line_height,
                self.underline,
                self.character_spacing,
                self.word_spacing,
                width,
                self.align,
            )
        }
    }
}
