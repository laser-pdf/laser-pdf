use super::*;

/// A predictable element for testing containers. It's a bit simpler than actual text in that it
/// doesn't vary it's height based on input width. It just either returns the width from the
/// constraint or [Self::width] if unconstrained.
pub struct FakeText {
    pub lines: u32,
    pub line_height: f32,
    pub width: f32,
}

struct Layout {
    first_lines: u32,
    full_height_lines: u32,
    lines: u32,
    breaks: u32,
}

impl FakeText {
    fn lines_and_breaks(&self, first_height: f32, full_height: f32) -> Layout {
        let first_lines = (first_height / self.line_height).floor() as u32;

        if self.lines <= first_lines {
            Layout {
                first_lines: self.lines,
                full_height_lines: 0,
                lines: self.lines,
                breaks: 0,
            }
        } else {
            let remaining_lines = self.lines - first_lines;
            let lines_per_page = ((full_height / self.line_height).floor() as u32).max(1);
            let full_pages = remaining_lines / lines_per_page;
            let last_page_lines = remaining_lines % lines_per_page;

            Layout {
                first_lines,
                full_height_lines: lines_per_page,
                lines: if last_page_lines == 0 {
                    lines_per_page
                } else {
                    last_page_lines
                },
                breaks: full_pages + if last_page_lines == 0 { 0 } else { 1 },
            }
        }
    }
}

impl Element for FakeText {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        if ctx.first_height < self.line_height {
            FirstLocationUsage::WillSkip
        } else {
            FirstLocationUsage::WillUse
        }
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        let lines = if let Some(breakable) = ctx.breakable {
            let layout = self.lines_and_breaks(ctx.first_height, breakable.full_height);

            *breakable.break_count = layout.breaks;
            layout.lines
        } else {
            self.lines
        };

        ElementSize {
            width: Some(ctx.width.constrain(self.width)),
            height: Some(lines as f32 * self.line_height),
        }
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        let lines = if let Some(breakable) = ctx.breakable {
            let layout = self.lines_and_breaks(ctx.first_height, breakable.full_height);

            for i in 0..layout.breaks {
                (breakable.do_break)(
                    ctx.pdf,
                    i,
                    Some(if i == 0 {
                        self.line_height * layout.first_lines as f32
                    } else {
                        self.line_height * layout.full_height_lines as f32
                    }),
                );
            }

            layout.lines
        } else {
            self.lines
        };

        ElementSize {
            width: Some(ctx.width.constrain(self.width)),
            height: Some(lines as f32 * self.line_height),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fake_text() {
        let element = FakeText {
            line_height: 1.,
            lines: 11,
            width: 5.,
        };

        for mut output in (ElementTestParams {
            first_height: 1.999,
            full_height: 3.3,
            ..Default::default()
        })
        .run(&element)
        {
            if let Some(ref mut b) = output.breakable {
                b.assert_break_count(if output.first_height == 1.999 { 4 } else { 3 });
            }

            output.assert_size(ElementSize {
                width: Some(if output.width.expand {
                    output.width.max
                } else {
                    5.
                }),

                height: Some(if output.breakable.is_some() {
                    if output.first_height == 1.999 {
                        1.
                    } else {
                        2.
                    }
                } else {
                    11.
                }),
            });
        }
    }
}
