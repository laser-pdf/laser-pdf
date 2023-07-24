use crate::{widgets::break_whole, *};

pub struct VListHandler<'a, 'b> {
    max_width: Option<f64>,
    render: Option<DrawContext<'a, 'b>>,
    width: f64,
    height: f64,
    break_page: bool,
    // line: Option<f64>,
    gap: f64,
    collapse_empty: bool,
    first: bool,
    draw_rect: u32,
}

impl<'a, 'b> VListHandler<'a, 'b> {
    pub fn el<W: Element>(&mut self, widget: &W) {
        let size = match self.render {
            Some(DrawContext {
                pdf: &mut ref mut pdf,
                ref mut draw_pos,
                full_height,
                next_draw_pos: Some(ref mut next_draw_pos),
            }) => {
                let start_draw_rect = self.draw_rect;

                let size = widget.element(
                    self.max_width,
                    Some(DrawContext {
                        pdf,
                        draw_pos: if self.first {
                            draw_pos.clone()
                        } else {
                            DrawPos {
                                pos: [draw_pos.pos[0], draw_pos.pos[1] - self.gap],
                                preferred_height: None,
                                height_available: draw_pos.height_available - self.gap,
                                layer: draw_pos.layer.clone(),
                            }
                        },
                        full_height,
                        next_draw_pos: Some(&mut |pdf, draw_rect_id, size| {
                            if !self.first {
                                self.height += self.gap;
                                self.first = true;
                            }

                            let new_draw_pos = next_draw_pos(
                                pdf,
                                start_draw_rect + draw_rect_id,
                                if draw_rect_id == 0 {
                                    [self.width.max(size[0]), self.height + size[1]]
                                } else {
                                    size
                                },
                            );

                            let new_draw_rect = start_draw_rect + draw_rect_id + 1;

                            if new_draw_rect > self.draw_rect {
                                self.draw_rect = new_draw_rect;
                                *draw_pos = new_draw_pos.clone();
                            }

                            self.height = 0.0;

                            new_draw_pos
                        }),
                    }),
                );

                if !self.first && (!self.collapse_empty || size[1] > 0.) {
                    draw_pos.pos[1] -= self.gap;
                    draw_pos.height_available -= self.gap;
                }

                draw_pos.pos[1] -= size[1];
                draw_pos.height_available -= size[1];

                size
            }
            Some(DrawContext {
                ref mut pdf,
                ref mut draw_pos,
                full_height,
                next_draw_pos: None,
            }) => {
                let size = widget.element(
                    self.max_width,
                    Some(DrawContext {
                        pdf,
                        draw_pos: if self.first {
                            draw_pos.clone()
                        } else {
                            DrawPos {
                                pos: [draw_pos.pos[0], draw_pos.pos[1] - self.gap],
                                preferred_height: None,
                                height_available: draw_pos.height_available - self.gap,
                                layer: draw_pos.layer.clone(),
                            }
                        },
                        full_height,
                        next_draw_pos: None,
                    }),
                );

                if !self.first && (!self.collapse_empty || size[1] > 0.) {
                    draw_pos.pos[1] -= self.gap;
                    draw_pos.height_available -= self.gap;
                }

                draw_pos.pos[1] -= size[1];
                draw_pos.height_available -= size[1];

                size
            }
            None => widget.element(self.max_width, None),
        };

        if self.collapse_empty && size[1] <= 0. {
        } else if self.first {
            self.first = false;
        } else {
            self.height += self.gap;
        }

        self.width = self.width.max(size[0]);
        self.height += size[1];
    }

    pub fn element<W: Element>(&mut self, widget: &W, break_page: bool) {
        if break_page && self.break_page {
            self.el(&break_whole(|w: Option<f64>, d: Option<DrawContext>| {
                widget.element(w, d)
            }));
        } else {
            self.el(widget);
        }
    }

    pub fn next_draw_pos(&mut self) {
        if let Some(DrawContext {
            pdf: &mut ref mut pdf,
            ref mut draw_pos,
            next_draw_pos: Some(ref mut next_draw_pos),
            ..
        }) = self.render
        {
            // this seems wrong
            *draw_pos = next_draw_pos(pdf, self.draw_rect, [self.width, self.height]);
            self.draw_rect += 1;
        }
    }
}

pub struct VList<F: Fn(&mut VListHandler)> {
    pub list: F,
    pub break_page: bool,
    pub gap: f64,
    pub collapse_empty: bool,
}

impl<F: Fn(&mut VListHandler)> VList<F> {
    pub fn new(list: F, break_page: bool, gap: f64) -> Self {
        VList {
            list,
            break_page,
            gap,
            collapse_empty: false,
        }
    }

    pub fn plain(content: F) -> Self {
        VList {
            list: content,
            break_page: true,
            gap: 0.,

            // doesn't matter
            collapse_empty: true,
        }
    }

    pub fn with_gap(gap: f64, collapse_empty: bool, content: F) -> Self {
        VList {
            list: content,
            gap,
            collapse_empty,
            break_page: true,
        }
    }
}

impl<F: Fn(&mut VListHandler)> Element for VList<F> {
    fn element(&self, width: Option<f64>, render: Option<DrawContext>) -> [f64; 2] {
        // if let Some(context) = &mut render {
        //     context.draw_pos.pos[1] -= self.gap;
        //     // if let Some(line) = self.line {
        //     //     crate::utils::line(
        //     //         &mut context.draw_pos.layer,
        //     //         context.draw_pos.pos,
        //     //         width.unwrap_or(1.0),
        //     //         line,
        //     //     );
        //     // }
        //     context.draw_pos.pos[1] -= self.gap;
        //     context.draw_pos.height_available = (context.draw_pos.height_available - 4.0 * self.gap)
        //         .max(0.0);
        // }

        let mut handler = VListHandler {
            max_width: width,
            render,
            width: 0.0,
            height: 0.0,
            break_page: self.break_page,
            // line: self.line,
            gap: self.gap,
            collapse_empty: self.collapse_empty,
            first: true,
            draw_rect: 0,
        };

        (self.list)(&mut handler);

        [handler.width, handler.height]
    }
}
