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
        let first_location_height;
        let location = ctx.location.clone();

        let color = calculate_color(self.color);

        if let Some(breakable) = ctx.breakable {
            let mut break_count = 0;

            size = self.element.draw(DrawCtx {
                pdf: ctx.pdf,
                breakable: Some(BreakableDraw {
                    do_break: &mut |pdf, location_idx, height| {
                        // TODO: draw box here
                        break_count = break_count.max(location_idx + 1);
                        (breakable.do_break)(pdf, location_idx, height)
                    },
                    ..breakable
                }),
                ..ctx
            });

            if break_count == 0 {
                first_location_height = size.height;
            } else {
                first_location_height = Some(ctx.first_height);
            }

            if let Some(width) = size.width {
                for i in 0..break_count {
                    let height = if i == break_count - 1 {
                        let Some(height) = size.height else {
                            break;
                        };

                        height
                    } else {
                        breakable.full_height
                    };

                    let location = (breakable.do_break)(
                        ctx.pdf,
                        i,
                        Some(if i == 0 {
                            ctx.first_height
                        } else {
                            breakable.full_height
                        }),
                    );

                    draw_box(location, (width, height), color);
                }
            }
        } else {
            size = self.element.draw(ctx);
            first_location_height = size.height;
        }

        if let (Some(width), Some(height)) = (size.width, first_location_height) {
            draw_box(location, (width, height), color);
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

fn draw_box(location: Location, size: (f64, f64), color: [f64; 3]) {
    let points = calculate_points_for_rect(
        Mm(size.0),
        Mm(size.1),
        Mm(location.pos.0 + size.0 / 2.0),
        Mm(location.pos.1 - size.1 / 2.0),
    );

    location.layer.save_graphics_state();

    location.layer.set_outline_thickness(0.);

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
