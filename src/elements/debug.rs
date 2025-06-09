use utils::mm_to_pt;

use crate::*;

pub struct Debug<E: Element> {
    pub element: E,
    pub color: u8,
    pub show_max_width: bool,
    pub show_last_location_max_height: bool,
}

impl<E: Element> Debug<E> {
    pub fn show_max_width(self) -> Self {
        Self {
            show_max_width: true,
            ..self
        }
    }

    pub fn show_last_location_max_height(self) -> Self {
        Self {
            show_last_location_max_height: true,
            ..self
        }
    }
}

impl<E: Element> Element for Debug<E> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        self.element.first_location_usage(ctx)
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        self.element.measure(ctx)
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        let size;
        let mut last_location = ctx.location.clone();

        let color = calculate_color(self.color);
        let max_width = ctx.width.max;
        let first_height = ctx.first_height;
        let full_height = ctx.breakable.as_ref().map(|b| b.full_height);

        let mut break_heights = Vec::new();

        if let Some(breakable) = ctx.breakable {
            size = self.element.draw(DrawCtx {
                pdf: ctx.pdf,
                breakable: Some(BreakableDraw {
                    do_break: &mut |pdf, location_idx, height| {
                        let break_count = break_heights.len() as u32;

                        if location_idx >= break_count {
                            break_heights.reserve((location_idx - break_count + 1) as usize);

                            break_heights.extend(
                                std::iter::repeat(None).take((location_idx - break_count) as usize),
                            );

                            break_heights.push(height);
                            last_location = (breakable.do_break)(pdf, location_idx, height);
                            last_location.clone()
                        } else {
                            let previous = break_heights[location_idx as usize];

                            // TODO: A visual indication would probably be better here than a panic.
                            assert_eq!(previous, height);

                            (breakable.do_break)(pdf, location_idx, height)
                        }
                    },
                    ..breakable
                }),
                location: ctx.location.clone(),
                ..ctx
            });

            if size.width.is_some() || self.show_max_width {
                for (i, &height) in break_heights.iter().enumerate() {
                    let full_height;
                    let location;

                    if i == 0 {
                        full_height = ctx.first_height;
                        location = ctx.location.clone();
                    } else {
                        full_height = breakable.full_height;
                        location =
                            (breakable.do_break)(ctx.pdf, i as u32 - 1, break_heights[i - 1]);
                    }

                    let dashed_size = (
                        if self.show_max_width {
                            max_width
                        } else {
                            size.width.unwrap()
                        },
                        full_height,
                    );

                    let dashed = match size.width.zip(height) {
                        Some(solid_size) => {
                            draw_box(ctx.pdf, location.clone(), solid_size, color, false);
                            solid_size != dashed_size
                        }
                        _ => true,
                    };

                    if dashed {
                        draw_box(ctx.pdf, location, dashed_size, color, dashed);
                    }
                }
            }
        } else {
            size = self.element.draw(DrawCtx {
                pdf: ctx.pdf,
                ..ctx
            });
        }

        let dashed_size = (
            if self.show_max_width {
                Some(max_width)
            } else {
                size.width
            },
            if self.show_last_location_max_height {
                Some(if break_heights.len() == 0 {
                    first_height
                } else {
                    full_height.unwrap()
                })
            } else {
                size.height
            },
        );

        let dashed = if let (Some(width), Some(height)) = (size.width, size.height) {
            draw_box(
                ctx.pdf,
                last_location.clone(),
                (width, height),
                color,
                false,
            );
            dashed_size != (Some(width), Some(height))
        } else {
            true
        };

        if let Some((width, height)) = dashed.then_some(dashed_size.0.zip(dashed_size.1)).flatten()
        {
            draw_box(ctx.pdf, last_location, (width, height), color, true);
        }

        size
    }
}

fn hue_to_rgb(hue: u8) -> [u8; 3] {
    let x = 6u8.saturating_mul(43 - 43u8.abs_diff(hue % 85));

    match hue / 43 {
        0 => [255, x, 0],
        1 => [x, 255, 0],
        2 => [0, 255, x],
        3 => [0, x, 255],
        4 => [x, 0, 255],
        5 => [255, 0, x],
        _ => unreachable!(),
    }
}

fn calculate_color(input: u8) -> [f32; 3] {
    hue_to_rgb(input.reverse_bits()).map(|c| c as f32 / 255.)
}

fn draw_box(pdf: &mut Pdf, location: Location, size: (f32, f32), color: [f32; 3], dashed: bool) {
    let layer = location.layer(pdf);

    layer
        .save_state()
        .set_line_width(0.)
        .set_stroke_rgb(color[0], color[1], color[2]);

    if dashed {
        layer.set_dash_pattern([2., 2.], 0.);
    }

    layer
        .rect(
            mm_to_pt(location.pos.0),
            mm_to_pt(location.pos.1 - size.1),
            mm_to_pt(size.0),
            mm_to_pt(size.1),
        )
        .stroke()
        .restore_state();
}
