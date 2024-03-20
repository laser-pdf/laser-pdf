use printpdf::{utils::calculate_points_for_rect, Line, Rgb};

use crate::*;

pub struct Debug<'a, E: Element>(pub &'a E);

impl<'a, E: Element> Element for Debug<'a, E> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        self.0.first_location_usage(ctx)
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        self.0.measure(ctx)
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        let size;
        let first_location_height;
        let location = ctx.location.clone();

        if let Some(breakable) = ctx.breakable {
            let mut break_count = 0;

            size = self.0.draw(DrawCtx {
                pdf: ctx.pdf,
                breakable: Some(BreakableDraw {
                    get_location: &mut |pdf, location_idx| {
                        break_count = break_count.max(location_idx + 1);
                        (breakable.get_location)(pdf, location_idx)
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

                    let location = (breakable.get_location)(ctx.pdf, i);

                    draw_box(location, (width, height));
                }
            }
        } else {
            size = self.0.draw(ctx);
            first_location_height = size.height;
        }

        if let (Some(width), Some(height)) = (size.width, first_location_height) {
            draw_box(location, (width, height));
        }

        size
    }
}

fn draw_box(location: Location, size: (f64, f64)) {
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
        .set_outline_color(printpdf::Color::Rgb(Rgb::new(0., 1., 0., None)));

    location.layer.add_shape(Line {
        points,
        is_closed: true,
        has_fill: false,
        has_stroke: true,
        is_clipping_path: false,
    });

    location.layer.restore_graphics_state();
}
