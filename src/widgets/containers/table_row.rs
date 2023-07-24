use printpdf::{LineDashPattern, Mm, Point};
use serde::Deserialize;

use crate::{
    utils::{mm_to_pt, u32_to_color_and_alpha},
    LineStyle, *,
};

use super::flex_list::{DrawLayout, MeasureLayout};

enum Pass<'a, 'b, 'c> {
    MeasureWidth {
        layout: &'a mut MeasureLayout,
        // max_height: &'a mut f64,
    },
    MeasureSize {
        layout: DrawLayout,
        // height: &'a mut f64,
        size: &'a mut [f64; 2],
    },
    Draw {
        layout: DrawLayout,
        // size: &'a mut [f64; 2],
        height: f64,
        draw: DrawContext<'b, 'c>,
    },

    /// Currently this could be done in the draw pos, but when we introduce page breaking it'll need
    /// to be separate.
    DrawContainer {
        layout: DrawLayout,
        height: f64,
        draw_pos: DrawPos,
        // draw: DrawContext<'b, 'c>,
    },
}

pub struct TableRowHandler<'a, 'b, 'c>(Pass<'a, 'b, 'c>, bool, LineStyle);

impl<'a, 'b, 'c> TableRowHandler<'a, 'b, 'c> {
    pub fn el<W: Element>(&mut self, widget: &W, flex: ColWidth) {
        let first = self.1;
        let line_style = self.2;

        match self.0 {
            Pass::MeasureWidth {
                layout: &mut ref mut layout,
                // max_height: &mut ref mut max_height,
            } => match flex {
                ColWidth::Expand(factor) => layout.add_expand(factor),
                ColWidth::Fixed(width) => layout.add_fixed(width),
            },
            Pass::MeasureSize {
                ref layout,
                // height,
                size: &mut ref mut size,
            } => {
                let width = match flex {
                    ColWidth::Expand(flex) => layout.expand_width(flex),
                    ColWidth::Fixed(width) => width,
                };

                if !first {
                    size[0] += line_style.thickness;
                }

                let widget_size = widget.element(Some(width), None);

                size[0] += widget_size[0].max(width);
                size[1] = size[1].max(widget_size[1]);
            }
            Pass::Draw {
                ref layout,
                // size: &mut ref mut size,
                height,
                ref mut draw,
            } => {
                let width = match flex {
                    ColWidth::Expand(flex) => layout.expand_width(flex),
                    ColWidth::Fixed(width) => width,
                };

                if !first {
                    draw.draw_pos.pos[0] += line_style.thickness;
                }

                let widget_size = widget.element(
                    Some(width),
                    Some(DrawContext {
                        pdf: draw.pdf,
                        draw_pos: DrawPos {
                            height_available: height,
                            ..draw.draw_pos.clone()
                        },
                        full_height: 0.0,
                        next_draw_pos: None,
                    }),
                );

                debug_assert!(widget_size[1] <= height);

                draw.draw_pos.pos[0] += widget_size[0].max(width);
            }
            Pass::DrawContainer {
                ref layout,
                height,
                ref mut draw_pos,
            } => {
                if !first {
                    // draw_pos.pos[0] += padding;

                    draw_pos.layer.save_graphics_state();
                    let layer = &draw_pos.layer;

                    let (color, _alpha) = u32_to_color_and_alpha(line_style.color);
                    layer.set_outline_color(color);
                    layer.set_outline_thickness(mm_to_pt(line_style.thickness));
                    layer.set_line_cap_style(line_style.cap_style.into());
                    layer.set_line_dash_pattern(if let Some(pattern) = line_style.dash_pattern {
                        pattern.into()
                    } else {
                        LineDashPattern::default()
                    });

                    let line_x = draw_pos.pos[0] + line_style.thickness / 2.;

                    draw_pos.layer.add_shape(printpdf::Line {
                        points: vec![
                            (Point::new(Mm(line_x), Mm(draw_pos.pos[1])), false),
                            (Point::new(Mm(line_x), Mm(draw_pos.pos[1] - height)), false),
                        ],
                        is_closed: false,
                        has_fill: false,
                        has_stroke: true,
                        is_clipping_path: false,
                    });

                    draw_pos.layer.restore_graphics_state();

                    draw_pos.pos[0] += line_style.thickness;
                }

                let width = match flex {
                    ColWidth::Expand(flex) => layout.expand_width(flex),
                    ColWidth::Fixed(width) => width,
                };

                draw_pos.pos[0] += width;
            }
        }

        if self.1 {
            self.1 = false;
        }
    }
}

#[derive(Copy, Clone, Deserialize)]
pub enum ColWidth {
    Expand(u8),
    Fixed(f64),
}

/// 0: List callback, 1: Gap
pub struct TableRow<F: Fn(&mut TableRowHandler)> {
    pub content: F,
    pub line_style: LineStyle,
    pub y_expand: bool,
}

impl<F: Fn(&mut TableRowHandler)> Element for TableRow<F> {
    fn element(&self, width: Option<f64>, draw: Option<DrawContext>) -> [f64; 2] {
        let mut layout = MeasureLayout::new(width, self.line_style.thickness);

        let mut handler = TableRowHandler(
            Pass::MeasureWidth {
                layout: &mut layout,
            },
            true,
            self.line_style,
        );

        (self.content)(&mut handler);

        let draw_layout = layout.build();

        let mut size = [0.; 2];
        let mut handler = TableRowHandler(
            Pass::MeasureSize {
                layout: draw_layout,
                size: &mut size,
            },
            true,
            self.line_style,
        );

        (self.content)(&mut handler);

        // make immutable
        let size = size;

        // let draw_pos = draw.as_ref().map(|c| c.draw_pos.clone());

        // size[0] += self.padding;
        // size[1] += 2. * self.padding;

        if let Some(ctx) = draw {
            let draw_pos = ctx.draw_pos.clone();
            let height = if self.y_expand {
                draw_pos.height_available
            } else {
                size[1]
            };

            let mut handler = TableRowHandler(
                Pass::Draw {
                    layout: draw_layout.clone(),
                    // padding: self.padding,
                    // size: &mut size,
                    height,
                    draw: ctx,
                },
                true,
                self.line_style,
            );

            (self.content)(&mut handler);

            let mut handler = TableRowHandler(
                Pass::DrawContainer {
                    layout: draw_layout,
                    // padding: self.padding,
                    height,
                    draw_pos,
                },
                true,
                self.line_style,
            );

            (self.content)(&mut handler);

            // draw_pos.pos[0] += self.padding;

            // draw_pos.layer.save_graphics_state();

            // draw_pos
            //     .layer
            //     .set_outline_color(u32_to_color_and_alpha(0x000000_FF).0);

            // draw_pos.layer.set_outline_thickness(mm_to_pt(0.));
            // draw_pos.layer.add_shape(printpdf::Line {
            //     points: vec![
            //         (Point::new(Mm(draw_pos.pos[0]), Mm(draw_pos.pos[1])), false),
            //         (
            //             Point::new(Mm(draw_pos.pos[0]), Mm(draw_pos.pos[1] - size[1])),
            //             false,
            //         ),
            //     ],
            //     is_closed: false,
            //     has_fill: false,
            //     has_stroke: true,
            //     is_clipping_path: false,
            // });

            // draw_pos.layer.restore_graphics_state();

            [if let Some(w) = width { w } else { size[0] }, height]
        } else {
            size
        }
    }
}
