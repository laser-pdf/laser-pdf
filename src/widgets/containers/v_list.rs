use crate::{widgets::break_whole, *};

pub struct VListHandler<'a, 'b> {
    max_width: Option<f64>,
    render: Option<DrawCtx<'a, 'b>>,
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
            Some(DrawCtx {
                pdf: &mut ref mut pdf,
                ref mut location,
                full_height,
                next_location: Some(ref mut next_location),
            }) => {
                let start_draw_rect = self.draw_rect;

                let size = widget.draw(
                    self.max_width,
                    Some(DrawCtx {
                        pdf,
                        location: if self.first {
                            location.clone()
                        } else {
                            Location {
                                pos: [location.pos[0], location.pos[1] - self.gap],
                                preferred_height: None,
                                height_available: location.height_available - self.gap,
                                layer: location.layer.clone(),
                            }
                        },
                        full_height,
                        breakable: Some(BreakableDraw {
                            get_location: &mut |pdf, draw_rect_id| {
                                if !self.first {
                                    self.height += self.gap;
                                    self.first = true;
                                }

                                let new_location = next_location(
                                    pdf,
                                    start_draw_rect + draw_rect_id,
                                    if draw_rect_id == 0 {
                                        Some(ElementSize {
                                            width: self.width.max(size[0]),
                                            height: Some(self.height + size[1]),
                                        })
                                    } else {
                                        size
                                    },
                                );

                                let new_draw_rect = start_draw_rect + draw_rect_id + 1;

                                if new_draw_rect > self.draw_rect {
                                    self.draw_rect = new_draw_rect;
                                    *location = new_location.clone();
                                }

                                self.height = 0.0;

                                new_location
                            },
                            ..break_ctx
                        }),
                    }),
                );

                if !self.first && (!self.collapse_empty || size[1] > 0.) {
                    location.pos[1] -= self.gap;
                    location.height_available -= self.gap;
                }

                location.pos[1] -= size[1];
                location.height_available -= size[1];

                size
            }
            Some(DrawCtx {
                ref mut pdf,
                ref mut location,
                full_height,
                next_location: None,
            }) => {
                let size = widget.draw(
                    self.max_width,
                    Some(DrawCtx {
                        pdf,
                        location: if self.first {
                            location.clone()
                        } else {
                            Location {
                                pos: [location.pos[0], location.pos[1] - self.gap],
                                preferred_height: None,
                                height_available: location.height_available - self.gap,
                                layer: location.layer.clone(),
                            }
                        },
                        full_height,
                        next_location: None,
                    }),
                );

                if !self.first && (!self.collapse_empty || size[1] > 0.) {
                    location.pos[1] -= self.gap;
                    location.height_available -= self.gap;
                }

                location.pos[1] -= size[1];
                location.height_available -= size[1];

                size
            }
            None => widget.draw(self.max_width, None),
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
            self.el(&break_whole(|w: Option<f64>, d: Option<DrawCtx>| {
                widget.draw(w, d)
            }));
        } else {
            self.el(widget);
        }
    }

    pub fn next_location(&mut self) {
        if let Some(DrawCtx {
            pdf: &mut ref mut pdf,
            ref mut location,
            next_location: Some(ref mut next_location),
            ..
        }) = self.render
        {
            // this seems wrong
            *location = next_location(pdf, self.draw_rect, [self.width, self.height]);
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
    fn draw(&self, width: Option<f64>, render: Option<DrawCtx>) -> [f64; 2] {
        // if let Some(context) = &mut render {
        //     context.location.pos[1] -= self.gap;
        //     // if let Some(line) = self.line {
        //     //     crate::utils::line(
        //     //         &mut context.location.layer,
        //     //         context.location.pos,
        //     //         width.unwrap_or(1.0),
        //     //         line,
        //     //     );
        //     // }
        //     context.location.pos[1] -= self.gap;
        //     context.location.height_available = (context.location.height_available - 4.0 * self.gap)
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

        Some(ElementSize {
            width: handler.width,
            height: Some(handler.height),
        })
    }
}
