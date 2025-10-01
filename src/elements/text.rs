use text::{Line, Lines, Piece, draw_line, lines};
use utils::{mm_to_pt, pt_to_mm};

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

    #[inline(always)]
    fn render_lines<'c, L: Iterator<Item = Line<'c, F, impl Iterator<Item = &'c Piece<'c, F>>>>>(
        &self,
        lines: L,
        mut ctx: DrawCtx,
        width: f32,
    ) -> (f32, f32)
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

        (max_width.max(pt_to_mm(last_line_full_width)), height)
    }

    #[inline(always)]
    fn layout_lines<'c, L: Iterator<Item = Line<'c, F, impl Iterator<Item = &'c Piece<'c, F>>>>>(
        &self,
        lines: L,
        measure_ctx: Option<&mut MeasureCtx>,
    ) -> (f32, f32)
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

        for line in lines {
            let line_height = line.height_above_baseline + line.height_below_baseline;

            if let Some(&mut MeasureCtx {
                breakable: Some(ref mut breakable),
                ..
            }) = measure_ctx
            {
                if height_available < line_height {
                    *breakable.break_count += 1;
                    height_available = breakable.full_height;
                    height = 0.;
                }
            }

            max_width = max_width.max(line.width);
            last_line_full_width = line.width + line.trailing_whitespace_width;

            height_available -= line_height;
            height += line_height;
        }

        (pt_to_mm(max_width.max(last_line_full_width)), height)
    }

    fn break_into_lines<R>(
        &self,
        width: f32,
        f: impl for<'b> FnOnce(Lines<'b, F, std::slice::Iter<'b, Piece<'b, F>>>) -> R,
    ) -> R {
        lines(
            self.font,
            self.extra_character_spacing,
            self.extra_word_spacing,
            mm_to_pt(width),
            self.text,
            self.size,
            self.color,
            f,
        )
    }
}

impl<'a, F: Font> Element for Text<'a, F> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        self.break_into_lines(ctx.width.max, |mut lines| {
            // There's always at least one line.
            let first_line = lines.next().unwrap();

            let line_height = first_line.height_above_baseline + first_line.height_below_baseline;

            if line_height > ctx.first_height {
                FirstLocationUsage::WillSkip
            } else {
                FirstLocationUsage::WillUse
            }
        })
    }

    fn measure(&self, mut ctx: MeasureCtx) -> ElementSize {
        let size = self.break_into_lines(ctx.width.max, |lines| {
            self.layout_lines(lines, Some(&mut ctx))
        });

        ElementSize {
            width: Some(ctx.width.constrain(size.0)),
            height: Some(size.1),
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
            // TODO: Figure out a way to avoid shaping twice here.
            self.break_into_lines(ctx.width.max, |lines| self.layout_lines(lines, None).0)
        };

        let width_constraint = ctx.width;
        let size =
            self.break_into_lines(ctx.width.max, |lines| self.render_lines(lines, ctx, width));

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

    use crate::fonts::builtin::BuiltinFont;
    use crate::test_utils::binary_snapshots::*;

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
}
