use crate::{
    text::{Line, Lines, Piece, draw_line, lines_from_pieces, pieces},
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

pub struct RichText<S> {
    pub spans: S,
    pub align: TextAlign,
}

impl<'a, F: Font + 'a, S: Iterator<Item = Span<'a, F>> + Clone> Element for RichText<S> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        self.break_into_lines(ctx.width.max, |mut lines| {
            // There's always at least one line.
            let first_line = lines.next().unwrap();

            let line_height =
                pt_to_mm(first_line.height_above_baseline + first_line.height_below_baseline);

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

impl<'a, F: Font + 'a, S: Iterator<Item = Span<'a, F>> + Clone> RichText<S> {
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
        // TODO: caching
        let pieces = self
            .spans
            .clone()
            .map(|span| {
                pieces(
                    span.font,
                    span.extra_character_spacing,
                    span.extra_word_spacing,
                    mm_to_pt(span.extra_line_height),
                    span.text,
                    span.size,
                    span.color,
                    |pieces| pieces.collect::<Vec<_>>(),
                )
            })
            .flatten()
            .collect::<Vec<_>>();

        // The `next_up` mitigates a problem when we get passed the width we returned from
        // measuring. In some cases it would then put one more piece onto the next line. This likely
        // doesn't fix the problem in all cases. TODO
        let lines = lines_from_pieces(&pieces, mm_to_pt(width).next_up());

        f(lines)
    }
}
