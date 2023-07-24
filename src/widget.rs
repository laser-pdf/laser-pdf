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

pub fn none_element() -> impl Element {
    move |_: Option<f64>, _: Option<DrawContext>| [0.; 2]
}

pub fn debug<W: Element>(el: W, color: u32) -> impl Element {
    move |width: Option<f64>, mut draw: Option<DrawContext>| {
        if let Some(DrawContext {
            pdf: &mut ref mut pdf,
            ref mut draw_pos,
            full_height,
            ref mut next_draw_pos,
        }) = draw
        {
            let color = u32_to_color_and_alpha(color).0;

            let size = if let Some(next_draw_pos) = next_draw_pos {
                let mut last_draw_rect = 0;

                el.element(
                    width,
                    Some(DrawContext {
                        pdf,
                        draw_pos: draw_pos.clone(),
                        full_height,
                        next_draw_pos: Some(&mut |pdf, draw_rect_id, size| {
                            if draw_rect_id >= last_draw_rect {
                                last_draw_rect = draw_rect_id;

                                let points = calculate_points_for_rect(
                                    Mm(size[0]),
                                    Mm(size[1]),
                                    Mm(draw_pos.pos[0] + size[0] / 2.0),
                                    Mm(draw_pos.pos[1] - size[1] / 2.0),
                                );

                                draw_pos.layer.save_graphics_state();

                                draw_pos.layer.set_outline_color(color.clone());

                                draw_pos.layer.add_shape(Line {
                                    points,
                                    is_closed: true,
                                    has_fill: false,
                                    has_stroke: true,
                                    is_clipping_path: false,
                                });

                                draw_pos.layer.restore_graphics_state();
                            }

                            *draw_pos = next_draw_pos(pdf, draw_rect_id, size);

                            draw_pos.clone()
                        }),
                    }),
                )
            } else {
                el.element(
                    width,
                    Some(DrawContext {
                        pdf,
                        draw_pos: draw_pos.clone(),
                        full_height,
                        next_draw_pos: None,
                    }),
                )
            };

            let points = calculate_points_for_rect(
                Mm(size[0]),
                Mm(size[1]),
                Mm(draw_pos.pos[0] + size[0] / 2.0),
                Mm(draw_pos.pos[1] - size[1] / 2.0),
            );

            draw_pos.layer.save_graphics_state();

            draw_pos.layer.set_outline_thickness(0.);

            draw_pos.layer.set_outline_color(color);

            draw_pos.layer.add_shape(Line {
                points,
                is_closed: true,
                has_fill: false,
                has_stroke: true,
                is_clipping_path: false,
            });

            draw_pos.layer.restore_graphics_state();

            size
        } else {
            el.element(width, draw)
        }
    }
}

pub fn debug_available_space() -> impl Element {
    debug(
        |width: Option<f64>, draw: Option<DrawContext>| {
            [
                width.unwrap_or(0.),
                draw.map(|c| c.draw_pos.height_available).unwrap_or(0.),
            ]
        },
        0x00_FF_00_FF,
    )
}
