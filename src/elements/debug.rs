use printpdf::{utils::calculate_points_for_rect, Line, Rgb};

use crate::*;

pub struct Debug<'a, E: Element + ?Sized> {
    pub element: &'a E,
    pub color: u8,
}

impl<'a, E: Element + ?Sized> Element for Debug<'a, E> {
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

        if let Some(breakable) = ctx.breakable {
            let mut break_heights = Vec::new();

            size = self.element.draw(DrawCtx {
                pdf: ctx.pdf,
                breakable: Some(BreakableDraw {
                    do_break: &mut |pdf, location_idx, height| {
                        let break_count = break_heights.len() as u32;

                        // dbg!(self.color, location_idx);

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

            if let Some(width) = size.width {
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

                    let dashed = match height {
                        Some(height) if full_height != height => {
                            draw_box(location.clone(), (width, height), color, false);
                            true
                        }
                        height => height.is_none(),
                    };

                    draw_box(location, (width, full_height), color, dashed);
                }
            }
        } else {
            size = self.element.draw(ctx);
        }

        if let (Some(width), Some(height)) = (size.width, size.height) {
            draw_box(last_location, (width, height), color, false);
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

fn calculate_color(input: u8) -> [f64; 3] {
    hue_to_rgb(input.reverse_bits()).map(|c| c as f64 / 255.)
}

fn draw_box(location: Location, size: (f64, f64), color: [f64; 3], dashed: bool) {
    let points = calculate_points_for_rect(
        Mm(size.0),
        Mm(size.1),
        Mm(location.pos.0 + size.0 / 2.0),
        Mm(location.pos.1 - size.1 / 2.0),
    );

    location.layer.save_graphics_state();

    location.layer.set_outline_thickness(0.);

    if dashed {
        location
            .layer
            .set_line_dash_pattern(printpdf::LineDashPattern::new(
                0,
                Some(2),
                Some(2),
                None,
                None,
                None,
                None,
            ));
    }

    location
        .layer
        .set_outline_color(printpdf::Color::Rgb(Rgb::new(
            color[0], color[1], color[2], None,
        )));

    location.layer.add_shape(Line {
        points,
        is_closed: true,
        has_fill: false,
        has_stroke: true,
        is_clipping_path: false,
    });

    location.layer.restore_graphics_state();
}
