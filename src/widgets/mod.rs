use lopdf::content::Operation;
use printpdf::utils::calculate_points_for_rect;
use printpdf::*;

use crate::utils::*;
use crate::LineStyle;
use crate::*;

pub mod containers;

pub use containers::*;

pub struct Line {
    pub thickness: f64,
    pub spacing: f64,
    pub color: u32,
}

pub fn line(thickness: f64, spacing: f64) -> impl Element {
    Line {
        thickness,
        spacing,
        color: 0x00_00_00_FF,
    }
}

impl Element for Line {
    fn element(&self, width: Option<f64>, render: Option<DrawContext>) -> [f64; 2] {
        let width = width.unwrap_or(1.0);
        if let Some(context) = render {
            let line_y = context.draw_pos.pos[1] - self.thickness / 2.0 - self.spacing;
            context.draw_pos.layer.save_graphics_state();
            context
                .draw_pos
                .layer
                .set_outline_color(u32_to_color_and_alpha(self.color).0);
            context
                .draw_pos
                .layer
                .set_outline_thickness(mm_to_pt(self.thickness));
            context.draw_pos.layer.add_shape(printpdf::Line {
                points: vec![
                    (Point::new(Mm(context.draw_pos.pos[0]), Mm(line_y)), false),
                    (
                        Point::new(Mm(context.draw_pos.pos[0] + width), Mm(line_y)),
                        false,
                    ),
                ],
                is_closed: false,
                has_fill: false,
                has_stroke: true,
                is_clipping_path: false,
            });
            context.draw_pos.layer.restore_graphics_state();
        }
        [width, self.thickness + 2.0 * self.spacing]
    }
}

pub fn styled_line(style: LineStyle) -> impl Element {
    move |width: Option<f64>, draw: Option<DrawContext>| {
        if let Some(draw) = draw {
            if let Some(width) = width {
                draw.draw_pos.layer.save_graphics_state();

                let (color, _alpha) = u32_to_color_and_alpha(style.color);
                draw.draw_pos.layer.set_outline_color(color);
                draw.draw_pos
                    .layer
                    .set_outline_thickness(mm_to_pt(style.thickness));
                draw.draw_pos
                    .layer
                    .set_line_cap_style(style.cap_style.into());
                draw.draw_pos.layer.set_line_dash_pattern(
                    if let Some(pattern) = style.dash_pattern {
                        pattern.into()
                    } else {
                        LineDashPattern::default()
                    },
                );

                let line_y = draw.draw_pos.pos[1] - style.thickness / 2.0;

                draw.draw_pos.layer.add_shape(printpdf::Line {
                    points: vec![
                        (Point::new(Mm(draw.draw_pos.pos[0]), Mm(line_y)), false),
                        (
                            Point::new(Mm(draw.draw_pos.pos[0] + width), Mm(line_y)),
                            false,
                        ),
                    ],
                    is_closed: false,
                    has_fill: false,
                    has_stroke: true,
                    is_clipping_path: false,
                });

                draw.draw_pos.layer.restore_graphics_state();
            }
        }

        [width.unwrap_or(0.), style.thickness]
    }
}

pub struct Gap(pub f64);

impl Element for Gap {
    fn element(&self, width: Option<f64>, draw: Option<DrawContext>) -> [f64; 2] {
        let height_available = if let Some(ctx) = draw {
            ctx.draw_pos.height_available
        } else {
            f64::INFINITY
        };

        [width.unwrap_or(0.0), self.0.min(height_available)]
    }
}

pub struct HGap(pub f64);

impl Element for HGap {
    fn element(&self, _width: Option<f64>, _render: Option<DrawContext>) -> [f64; 2] {
        [self.0, 0.0]
    }
}

pub struct Border<W: Element> {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
    pub vanish_if_empty: bool,
    pub widget: W,
}

impl<W: Element> Border<W> {
    pub fn top(size: f64, vanish_if_empty: bool, widget: W) -> Self {
        Border {
            left: 0.,
            right: 0.,
            top: size,
            bottom: 0.,
            vanish_if_empty,
            widget,
        }
    }
}

impl<W: Element> Element for Border<W> {
    fn element(&self, width: Option<f64>, render: Option<DrawContext>) -> [f64; 2] {
        let width = width.map(|w| (w - self.left - self.right).max(0.0));

        let size = match render {
            Some(DrawContext {
                pdf,
                draw_pos,
                full_height,
                next_draw_pos: Some(next_draw_pos),
            }) => self.widget.element(
                width,
                Some(DrawContext {
                    pdf,
                    draw_pos: DrawPos {
                        layer: draw_pos.layer,
                        pos: [draw_pos.pos[0] + self.left, draw_pos.pos[1] - self.top],
                        preferred_height: draw_pos
                            .preferred_height
                            .map(|h| h - self.top - self.bottom),
                        height_available: draw_pos.height_available - self.top - self.bottom,
                    },
                    full_height: full_height - self.top - self.bottom,
                    next_draw_pos: Some(&mut |pdf, draw_rect_id, size| {
                        let size = if self.vanish_if_empty && size[1] <= 0. {
                            [0.; 2]
                        } else {
                            [
                                size[0] + self.left + self.right,
                                size[1] + self.top + self.bottom,
                            ]
                        };

                        let mut new_draw_pos = next_draw_pos(pdf, draw_rect_id, size);
                        new_draw_pos.pos[0] += self.left;
                        new_draw_pos.pos[1] -= self.top;
                        if let Some(ref mut ph) = new_draw_pos.preferred_height {
                            *ph -= self.top + self.bottom;
                        }
                        new_draw_pos.height_available -= self.top + self.bottom;
                        new_draw_pos
                    }),
                }),
            ),
            Some(DrawContext {
                pdf,
                draw_pos,
                full_height,
                next_draw_pos: None,
            }) => self.widget.element(
                width,
                Some(DrawContext {
                    pdf,
                    draw_pos: DrawPos {
                        layer: draw_pos.layer,
                        pos: [draw_pos.pos[0] + self.left, draw_pos.pos[1] - self.top],
                        preferred_height: draw_pos
                            .preferred_height
                            .map(|h| h - self.top - self.bottom),
                        height_available: draw_pos.height_available - self.top - self.bottom,
                    },
                    full_height: full_height - self.top - self.bottom,
                    next_draw_pos: None,
                }),
            ),
            None => self.widget.element(width, None),
        };

        // self.widget.widget(
        //     width,
        //     render.map(|r| DrawContext {
        //         pos: [r.pos[0] + self.left, r.pos[1] - self.top],
        //         height_available: height_available - self.top - self.bottom,
        //         pdf: r.pdf,
        //         next_draw_pos: if let Some(next_draw_pos) = r.next_draw_pos {
        //             Some(&mut |pdf| {
        //                 let (pos, height_available) = next_draw_pos(pdf);
        //                 ([pos[0] + self.left, pos[1] - self.top], height_available - self.top - self.bottom)
        //             })
        //         } else { None },
        //     }),
        // );

        if self.vanish_if_empty && size[1] <= 0. {
            [0.; 2]
        } else {
            [
                size[0] + self.left + self.right,
                size[1] + self.top + self.bottom,
            ]
        }
    }
}

pub struct Page<W: Element, F: Element> {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
    pub widget: W,
    pub footer_gap: f64,
    pub footer: F,
}

impl<W: Element, F: Element> Element for Page<W, F> {
    fn element(&self, width: Option<f64>, mut draw: Option<DrawContext>) -> [f64; 2] {
        let width = width.map(|w| (w - self.left - self.right).max(0.0));

        let size = match draw {
            Some(DrawContext {
                pdf: &mut ref mut pdf,
                ref draw_pos,
                full_height,
                next_draw_pos: Some(ref mut next_draw_pos),
            }) => self.widget.element(
                width,
                Some(DrawContext {
                    pdf,
                    draw_pos: DrawPos {
                        layer: draw_pos.layer.clone(),
                        pos: [draw_pos.pos[0] + self.left, draw_pos.pos[1] - self.top],
                        preferred_height: draw_pos
                            .preferred_height
                            .map(|h| h - self.top - self.bottom),
                        height_available: draw_pos.height_available - self.top - self.bottom,
                    },
                    full_height: full_height - self.top - self.bottom,
                    next_draw_pos: Some(&mut |pdf, draw_rect_id, size| {
                        let mut new_draw_pos = next_draw_pos(
                            pdf,
                            draw_rect_id,
                            [
                                size[0] + self.left + self.right,
                                size[1] + self.top + self.bottom,
                            ],
                        );
                        let new_y = new_draw_pos.pos[1];
                        let new_height_available = new_draw_pos.height_available;

                        new_draw_pos.pos[0] += self.left;
                        new_draw_pos.pos[1] -= self.top;
                        new_draw_pos.height_available -= self.top + self.bottom;

                        self.footer.element(
                            width,
                            Some(DrawContext {
                                pdf,
                                draw_pos: DrawPos {
                                    layer: new_draw_pos.layer.clone(),
                                    pos: [
                                        new_draw_pos.pos[0],
                                        new_y - new_height_available + self.bottom
                                            - self.footer_gap,
                                    ],
                                    preferred_height: None,
                                    height_available: self.bottom - self.footer_gap,
                                },
                                full_height: 0.0,
                                next_draw_pos: None,
                            }),
                        );

                        new_draw_pos
                    }),
                }),
            ),
            Some(DrawContext {
                pdf: &mut ref mut pdf,
                ref draw_pos,
                full_height,
                next_draw_pos: None,
            }) => self.widget.element(
                width,
                Some(DrawContext {
                    pdf,
                    draw_pos: DrawPos {
                        layer: draw_pos.layer.clone(),
                        pos: [draw_pos.pos[0] + self.left, draw_pos.pos[1] - self.top],
                        preferred_height: draw_pos
                            .preferred_height
                            .map(|h| h - self.top - self.bottom),
                        height_available: draw_pos.height_available - self.top - self.bottom,
                    },
                    full_height: full_height - self.top - self.bottom,
                    next_draw_pos: None,
                }),
            ),
            None => self.widget.element(width, None),
        };

        // footer
        if let Some(DrawContext { pdf, draw_pos, .. }) = draw {
            self.footer.element(
                width,
                Some(DrawContext {
                    pdf,
                    draw_pos: DrawPos {
                        layer: draw_pos.layer,
                        pos: [
                            draw_pos.pos[0] + self.left,
                            draw_pos.pos[1] - draw_pos.height_available + self.bottom
                                - self.footer_gap,
                        ],
                        preferred_height: None,
                        height_available: self.bottom - self.footer_gap,
                    },
                    full_height: 0.0,
                    next_draw_pos: None,
                }),
            );
        }

        // self.widget.widget(
        //     width,
        //     render.map(|r| DrawContext {
        //         pos: [r.pos[0] + self.left, r.pos[1] - self.top],
        //         height_available: height_available - self.top - self.bottom,
        //         pdf: r.pdf,
        //         next_draw_pos: if let Some(next_draw_pos) = r.next_draw_pos {
        //             Some(&mut |pdf| {
        //                 let (pos, height_available) = next_draw_pos(pdf);
        //                 ([pos[0] + self.left, pos[1] - self.top], height_available - self.top - self.bottom)
        //             })
        //         } else { None },
        //     }),
        // );

        [
            size[0] + self.left + self.right,
            size[1] + self.top + self.bottom,
        ]
    }
}

pub struct Rectangle(pub [f64; 2]);

impl Element for Rectangle {
    fn element(&self, _width: Option<f64>, draw: Option<DrawContext>) -> [f64; 2] {
        if let Some(context) = draw {
            let points = calculate_points_for_rect(
                Pt(mm_to_pt(self.0[0])),
                Pt(mm_to_pt(self.0[1])),
                Pt(mm_to_pt(context.draw_pos.pos[0] + self.0[0] / 2.0)),
                Pt(mm_to_pt(context.draw_pos.pos[1] - self.0[1] / 2.0)),
            );

            context.draw_pos.layer.add_shape(printpdf::Line {
                points,
                is_closed: true,
                has_fill: true,
                has_stroke: false,
                is_clipping_path: false,
            });
        }
        self.0
    }
}

pub struct HCenter<W: Element>(pub W);

impl<W: Element> Element for HCenter<W> {
    fn element(&self, width: Option<f64>, draw: Option<DrawContext>) -> [f64; 2] {
        match (draw, width) {
            (Some(mut context), Some(width)) => {
                let [widget_width, _] = self.0.element(None, None);

                context.draw_pos.pos[0] += (width - widget_width).max(0.0) / 2.0;

                let size = self.0.element(Some(width), Some(context));

                [width, size[1]]
            }
            (draw, width) => self.0.element(width, draw),
        }
    }
}

pub struct HRight<W: Element>(pub W);

impl<W: Element> Element for HRight<W> {
    fn element(&self, width: Option<f64>, draw: Option<DrawContext>) -> [f64; 2] {
        match (draw, width) {
            (Some(mut context), Some(width)) => {
                let [widget_width, _] = self.0.element(None, None);

                context.draw_pos.pos[0] += (width - widget_width).max(0.0);

                let size = self.0.element(Some(width), Some(context));

                [width, size[1]]
            }
            (draw, width) => self.0.element(width, draw),
        }
    }
}

pub fn title<T: Element, C: Element>(title: T, content: C, gap: f64) -> impl Element {
    // TODO: Add behavior to move title to second page asswell if none of the content makes it to
    // the first page.
    VList::new(
        move |h| {
            h.element(&title, true);
            h.element(&content, false);
        },
        true,
        gap,
    )
}

pub fn break_whole<W: Element>(widget: W) -> impl Element {
    move |width: Option<f64>, draw: Option<DrawContext>| {
        if let Some(DrawContext {
            pdf,
            mut draw_pos,
            full_height,
            next_draw_pos: Some(next_draw_pos),
        }) = draw
        {
            let widget_size = widget.element(width, None);
            let draw_rect_offset;

            if widget_size[1] > draw_pos.height_available {
                draw_pos = next_draw_pos(pdf, 0, [widget_size[0], 0.]);
                draw_rect_offset = 1;
            } else {
                draw_rect_offset = 0;
            }

            widget.element(
                width,
                Some(DrawContext {
                    pdf,
                    draw_pos: DrawPos {
                        preferred_height: Some(widget_size[1]),
                        ..draw_pos
                    },
                    full_height,
                    next_draw_pos: Some(&mut |pdf, draw_rect_id, size| {
                        next_draw_pos(pdf, draw_rect_id + draw_rect_offset, size)
                    }),
                }),
            )
        } else {
            widget.element(width, draw)
        }
    }
}

pub fn force_break() -> impl Element {
    move |_width: Option<f64>, draw: Option<DrawContext>| {
        if let Some(DrawContext {
            pdf,
            draw_pos,
            next_draw_pos: Some(next_draw_pos),
            ..
        }) = draw
        {
            let draw_rect = if draw_pos.height_available < 0. {
                next_draw_pos(pdf, 0, [0.; 2]);
                1
            } else {
                0
            };

            next_draw_pos(pdf, draw_rect, [0.; 2]);
        }

        [0.; 2]
    }
}

pub struct StyledBox<W> {
    pub widget: W,
    pub padding_top: f64,
    pub padding_bottom: f64,
    pub padding_left: f64,
    pub padding_right: f64,
    pub border_radius: f64,
    pub fill: Option<u32>,
    pub outline: Option<LineStyle>,
    pub vanish_if_empty: bool,
}

impl<W: Element> Element for StyledBox<W> {
    fn element(&self, width: Option<f64>, mut render: Option<DrawContext>) -> [f64; 2] {
        let outline_thickness = self.outline.map(|o| o.thickness).unwrap_or(0.0);
        let extra_outline_offset = outline_thickness / 2.0;

        let top = self.padding_top + extra_outline_offset;
        let bottom = self.padding_bottom + extra_outline_offset;
        let left = self.padding_left + extra_outline_offset;
        let right = self.padding_right + extra_outline_offset;

        let width = width.map(|w| (w - left - right).max(0.0));

        fn draw_box<W: Element>(this: &StyledBox<W>, draw_pos: &DrawPos, size: [f64; 2]) {
            use kurbo::{PathEl, RoundedRect, Shape};

            let size = [
                size[0] + this.padding_left + this.padding_right,
                size[1] + this.padding_top + this.padding_bottom,
            ];

            let shape = RoundedRect::new(
                mm_to_pt(draw_pos.pos[0]),
                mm_to_pt(draw_pos.pos[1]),
                mm_to_pt(draw_pos.pos[0] + size[0]),
                mm_to_pt(draw_pos.pos[1] - size[1]),
                mm_to_pt(this.border_radius),
            );

            // let points = utils::calculate_points_for_rect(
            //     Mm(size[0]),
            //     Mm(size[1]),
            //     Mm(context.draw_pos.pos[0] + size[0] / 2.0 + extra_outline_offset),
            //     Mm(context.draw_pos.pos[1] - size[1] / 2.0 - extra_outline_offset),
            // );

            let layer = &draw_pos.layer;

            layer.save_graphics_state();

            if let Some(color) = this.fill {
                let (color, alpha) = u32_to_color_and_alpha(color);
                layer.set_fill_color(color);
                layer.set_fill_alpha(alpha);
            }

            if let Some(line_style) = this.outline {
                // No outline alpha?
                let (color, _alpha) = u32_to_color_and_alpha(line_style.color);
                layer.set_outline_color(color);
                layer.set_outline_thickness(mm_to_pt(line_style.thickness));
                layer.set_line_cap_style(line_style.cap_style.into());
                layer.set_line_dash_pattern(if let Some(pattern) = line_style.dash_pattern {
                    pattern.into()
                } else {
                    LineDashPattern::default()
                });
            }

            let els = shape.path_elements(0.1);

            let mut closed = false;

            for el in els {
                use PathEl::*;

                match el {
                    MoveTo(point) => {
                        layer.add_op(Operation::new("m", vec![point.x.into(), point.y.into()]))
                    }
                    LineTo(point) => {
                        layer.add_op(Operation::new("l", vec![point.x.into(), point.y.into()]))
                    }
                    QuadTo(a, b) => layer.add_op(
                        // i dunno
                        Operation::new("v", vec![a.x.into(), a.y.into(), b.x.into(), b.y.into()]),
                    ),
                    CurveTo(a, b, c) => layer.add_op(Operation::new(
                        "c",
                        vec![
                            a.x.into(),
                            a.y.into(),
                            b.x.into(),
                            b.y.into(),
                            c.x.into(),
                            c.y.into(),
                        ],
                    )),
                    ClosePath => closed = true,
                };
            }

            match (this.outline.is_some(), this.fill.is_some(), closed) {
                (true, true, true) => layer.add_op(Operation::new("b", Vec::new())),
                (true, true, false) => layer.add_op(Operation::new("f", Vec::new())),
                (true, false, true) => layer.add_op(Operation::new("s", Vec::new())),
                (true, false, false) => layer.add_op(Operation::new("S", Vec::new())),
                (false, true, _) => layer.add_op(Operation::new("f", Vec::new())),
                _ => layer.add_op(Operation::new("n", Vec::new())),
            }

            // layer.add_shape(printpdf::Line {
            //     points,
            //     is_closed: true,
            //     has_fill: self.fill.is_some(),
            //     has_stroke: self.outline.is_some(),
            //     is_clipping_path: false,
            // });

            draw_pos.layer.restore_graphics_state();
        }

        let size = match render {
            Some(DrawContext {
                pdf: &mut ref mut pdf,
                ref mut draw_pos,
                full_height,
                next_draw_pos: Some(ref mut next_draw_pos),
            }) => {
                let layer = pdf
                    .document
                    .get_page(draw_pos.layer.page)
                    .add_layer("StyledBox Content");

                let mut last_draw_rect = 0;

                self.widget.element(
                    width,
                    Some(DrawContext {
                        pdf,
                        draw_pos: DrawPos {
                            layer,
                            pos: [draw_pos.pos[0] + left, draw_pos.pos[1] - top],
                            preferred_height: None,
                            height_available: draw_pos.height_available - top - bottom,
                            // clear: true,
                        },
                        full_height: full_height - top - bottom,
                        next_draw_pos: Some(&mut |pdf, draw_rect_id, size| {
                            let size = if self.vanish_if_empty && size[1] <= 0. {
                                [0.; 2]
                            } else {
                                if last_draw_rect <= draw_rect_id {
                                    draw_box(self, draw_pos, size);
                                }

                                [size[0] + left + right, size[1] + top + bottom]
                            };

                            let mut new_draw_pos = next_draw_pos(pdf, draw_rect_id, size);
                            if last_draw_rect <= draw_rect_id {
                                *draw_pos = new_draw_pos.clone();
                            }
                            last_draw_rect = draw_rect_id + 1;

                            new_draw_pos.pos[0] += left;
                            new_draw_pos.pos[1] -= top;
                            new_draw_pos.height_available -= top + bottom;
                            new_draw_pos
                        }),
                    }),
                )
            }
            Some(DrawContext {
                pdf: &mut ref mut pdf,
                ref draw_pos,
                full_height,
                next_draw_pos: None,
            }) => {
                let layer = pdf
                    .document
                    .get_page(draw_pos.layer.page)
                    .add_layer("StyledBox Content");

                self.widget.element(
                    width,
                    Some(DrawContext {
                        pdf,
                        draw_pos: DrawPos {
                            layer,
                            pos: [draw_pos.pos[0] + left, draw_pos.pos[1] - top],
                            preferred_height: None,
                            height_available: draw_pos.height_available - top - bottom,
                            // clear: true,
                        },
                        full_height: full_height - top - bottom,
                        next_draw_pos: None,
                    }),
                )
            }
            None => self.widget.element(width, None),
        };

        if self.vanish_if_empty && size[1] <= 0. {
            [0.; 2]
        } else {
            if let Some(ref context) = render {
                draw_box(self, &context.draw_pos, size);
            }

            [size[0] + left + right, size[1] + top + bottom]
        }
    }
}
