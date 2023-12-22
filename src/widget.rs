use std::ops::Deref;

use crate::*;
use printpdf::{utils::calculate_points_for_rect, *};
use stb_truetype as tt;

use crate::utils::u32_to_color_and_alpha;

// not sure if it makes sense to make this generic
#[derive(Debug)]
pub struct Font<D: Deref<Target = [u8]>> {
    pub font_ref: IndirectFontRef,
    pub font: tt::FontInfo<D>,
}

#[derive(Debug)]
pub struct FontSet<'a, D: Deref<Target = [u8]>> {
    pub regular: &'a Font<D>,
    pub bold: &'a Font<D>,
    pub italic: &'a Font<D>,
    pub bold_italic: &'a Font<D>,
}

impl<'a, D: Deref<Target = [u8]>> Copy for FontSet<'a, D> {}

impl<'a, D: Deref<Target = [u8]>> Clone for FontSet<'a, D> {
    fn clone(&self) -> Self {
        *self
    }
}

pub struct NoneElement;

impl Element for NoneElement {
    fn measure(&self, ctx: MeasureCtx) -> Option<ElementSize> {
        None
    }

    fn draw(&self, ctx: DrawCtx) -> Option<ElementSize> {
        None
    }
}

pub fn none_element() -> impl Element {
    move |_: Option<f64>, _: Option<DrawCtx>| [0.; 2]
}

pub struct Debug<W> {
    element: W,
    color: u32,
}

impl<W: Element> Element for Debug<W> {
    fn measure(&self, ctx: MeasureCtx) -> Option<ElementSize> {
        self.element.measure(ctx)
    }

    fn draw(&self, ctx: DrawCtx) -> Option<ElementSize> {
        let color = u32_to_color_and_alpha(self.color).0;

        let size = if let Some(break_ctx) = ctx.breakable {
            let mut last_draw_rect = 0;

            let size = self.element.draw(DrawCtx {
                pdf: ctx.pdf,
                location: ctx.location.clone(),
                width: ctx.width,
                breakable: Some(BreakableDraw {
                    get_location: &mut |pdf, draw_rect_id| {
                        last_draw_rect = last_draw_rect.max(draw_rect_id);

                        (break_ctx.get_location)(pdf, draw_rect_id)
                    },
                    ..break_ctx
                }),
                ..ctx
            });

            for i in 0..last_draw_rect {
                let location = (break_ctx.get_location)(ctx.pdf, i + 1);

                let points = calculate_points_for_rect(
                    Mm(size[0]),
                    Mm(size[1]),
                    Mm(location.pos[0] + size[0] / 2.0),
                    Mm(location.pos[1] - size[1] / 2.0),
                );

                location.layer.save_graphics_state();

                location.layer.set_outline_thickness(0.);

                location.layer.set_outline_color(color);

                location.layer.add_shape(Line {
                    points,
                    is_closed: true,
                    has_fill: false,
                    has_stroke: true,
                    is_clipping_path: false,
                });

                location.layer.restore_graphics_state();
            }

            size
        } else {
            self.element.element(DrawCtx {
                location: ctx.location.clone(),
                breakable: None,
                ..ctx
            })
        };

        let points = calculate_points_for_rect(
            Mm(size[0]),
            Mm(size[1]),
            Mm(ctx.location.pos[0] + size[0] / 2.0),
            Mm(ctx.location.pos[1] - size[1] / 2.0),
        );

        ctx.location.layer.save_graphics_state();

        ctx.location.layer.set_outline_thickness(0.);

        ctx.location.layer.set_outline_color(color);

        ctx.location.layer.add_shape(Line {
            points,
            is_closed: true,
            has_fill: false,
            has_stroke: true,
            is_clipping_path: false,
        });

        ctx.location.layer.restore_graphics_state();

        size
    }
}

// TODO
// pub fn debug_available_space() -> impl Element {
//     debug(
//         |width: Option<f64>, draw: Option<DrawCtx>| {
//             Some(ElementSize {
//                 width: width.unwrap_or(0.),
//                 height: Some(draw.map(|c| c.location.height_available).unwrap_or(0.)),
//             })
//         },
//         0x00_FF_00_FF,
//     )
// }
