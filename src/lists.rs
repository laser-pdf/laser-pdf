use crate::*;

pub struct VListHandler<'a, 'b> {
    max_width: Option<f64>,
    render: Option<DrawContext<'a, 'b>>,
    width: f64,
    height: f64,
    break_page: bool,
    line: Option<f64>,
    gap: f64,
}

impl<'a, 'b> VListHandler<'a, 'b> {
    pub fn element<W: Widget>(&mut self, widget: &W, break_page: bool) {
        let size = match self.render {
            Some(DrawContext {
                ref mut pdf,
                ref mut draw_pos,
                full_height,
                next_draw_pos: Some(ref mut next_draw_pos),
            }) => {
                let size = widget.widget(self.max_width, None);

                if self.break_page && break_page && size[1] > draw_pos.height_available {
                    *draw_pos = next_draw_pos(pdf);

                    self.height = 2.0 * self.gap;

                    draw_pos.pos[1] -= self.gap;
                    if let Some(line) = self.line {
                        crate::utils::line(
                            &mut draw_pos.layer,
                            draw_pos.pos,
                            self.max_width.unwrap_or(1.0),
                            line,
                        );
                    }
                    draw_pos.pos[1] -= self.gap;
                    draw_pos.height_available =
                        (draw_pos.height_available - 4.0 * self.gap).max(0.0);
                }

                let height = &mut self.height;
                let gap = self.gap;
                let max_width = self.max_width;
                let line = self.line;

                widget.widget(
                    self.max_width,
                    Some(DrawContext {
                        pdf,
                        draw_pos: *draw_pos,
                        full_height,
                        next_draw_pos: Some(&mut |pdf| {
                            *draw_pos = next_draw_pos(pdf);
                            *height = 0.0;

                            draw_pos.height_available =
                                (draw_pos.height_available - 2.0 * gap).max(0.0);

                            *draw_pos
                        }),
                    }),
                )
            }
            Some(DrawContext {
                ref mut pdf,
                draw_pos,
                full_height,
                next_draw_pos: None,
            }) => widget.widget(
                self.max_width,
                Some(DrawContext {
                    pdf,
                    draw_pos,
                    full_height,
                    next_draw_pos: None,
                }),
            ),
            None => widget.widget(self.max_width, None),
        };

        self.width = self.width.max(size[0]);
        self.height += size[1];

        self.height += 2.0 * self.gap;
        if let Some(ref mut context) = self.render {
            context.draw_pos.pos[1] -= size[1];
            context.draw_pos.height_available -= size[1];

            context.draw_pos.pos[1] -= self.gap;
            if let Some(line) = self.line {
                // println!("{:?} {:?}", context.draw_pos.pos, size);
                crate::utils::line(
                    &mut context.draw_pos.layer,
                    context.draw_pos.pos,
                    self.max_width.unwrap_or(1.0),
                    line,
                );
            }
            context.draw_pos.pos[1] -= self.gap;
            context.draw_pos.height_available -= 2.0 * self.gap;
        }
    }
}

pub struct VList<F: Fn(&mut VListHandler)> {
    pub list: F,
    pub break_page: bool,
    pub line: Option<f64>,
    pub gap: f64,
}

impl<F: Fn(&mut VListHandler)> VList<F> {
    pub fn new(list: F, break_page: bool, line: Option<f64>, gap: f64) -> Self {
        VList {
            list,
            break_page,
            line,
            gap,
        }
    }
}

impl<F: Fn(&mut VListHandler)> Widget for VList<F> {
    fn widget(&self, width: Option<f64>, mut render: Option<DrawContext>) -> [f64; 2] {
        if let Some(context) = &mut render {
            context.draw_pos.pos[1] -= self.gap;
            if let Some(line) = self.line {
                crate::utils::line(
                    &mut context.draw_pos.layer,
                    context.draw_pos.pos,
                    width.unwrap_or(1.0),
                    line,
                );
            }
            context.draw_pos.pos[1] -= self.gap;
            context.draw_pos.height_available =
                (context.draw_pos.height_available - 4.0 * self.gap).max(0.0);
        }

        let mut handler = VListHandler {
            max_width: width,
            render,
            width: 0.0,
            height: 2.0 * self.gap,
            break_page: self.break_page,
            line: self.line,
            gap: self.gap,
        };

        (self.list)(&mut handler);

        [handler.width, handler.height]
    }
}

pub struct HListHandler<'a, 'b> {
    draw: Option<DrawContext<'a, 'b>>,
    width: f64,
    height: f64,
}

impl<'a, 'b> HListHandler<'a, 'b> {
    pub fn element<W: Widget>(&mut self, widget: &W) {
        let size = {
            let draw = if let Some(DrawContext {
                ref mut pdf,
                draw_pos,
                full_height,
                next_draw_pos: _,
            }) = self.draw
            {
                Some(DrawContext {
                    pdf,
                    draw_pos,
                    full_height,
                    next_draw_pos: None,
                })
            } else {
                None
            };
            widget.widget(None, draw)
        };

        self.width += size[0];
        self.height = self.height.max(size[1]);
        if let Some(ref mut context) = self.draw {
            context.draw_pos.pos[0] += size[0];
        }
    }
}

pub struct HList<F: Fn(&mut HListHandler)>(pub F);

impl<F: Fn(&mut HListHandler)> Widget for HList<F> {
    fn widget(&self, _width: Option<f64>, draw: Option<DrawContext>) -> [f64; 2] {
        let mut handler = HListHandler {
            draw,
            width: 0.0,
            height: 0.0,
        };

        self.0(&mut handler);

        [handler.width, handler.height]
    }
}

// pub enum FlexMode {
//     Expand,
//     SelfSized,
//     Fixed(f64),
// }

// pub struct FlexList<F: Fn(&mut dyn FnMut(&dyn Widget, FlexMode))>(pub F);

// impl<F: Fn(&mut dyn FnMut(&dyn Widget, FlexMode))> Widget for FlexList<F> {
//     fn widget(&self, width: Option<f64>, mut render: Option<DrawContext>) -> [f64; 2] {
//         use FlexMode::*;

//         let mut no_expand_width = 0.0;
//         let mut expand_count = 0.0;

//         self.0(&mut |w, expand| {
//             match expand {
//                 Expand => expand_count += 1.0,
//                 SelfSized => {
//                     let size = w.widget(None, None);
//                     no_expand_width += size[0];
//                 },
//                 Fixed(width) => no_expand_width += width,
//             }
//             // if expand {
//             //     expand_count += 1.0;
//             // } else {
//             //     let size = w.widget(None, None);
//             //     no_expand_width += size[0];
//             // }
//         });

//         let remaining_width = width.map(|w| (w - no_expand_width).max(0.0)).unwrap_or(0.0);
//         let expand_width = remaining_width / expand_count;

//         let mut width: f64 = 0.0;
//         let mut height: f64 = 0.0;

//         self.0(&mut |w, expand| {
//             let expand = match expand { Expand => Some(expand_width), Fixed(width) => Some(width), _ => None };

//             let size = w.widget(
//                 expand,
//                 // if expand { Some(expand_width) } else { None },
//                 if let Some(r) = &mut render {
//                     Some(DrawContext {
//                         pos: r.pos,
//                         height_available: r.height_available,
//                         pdf: r.pdf,
//                         next_draw_pos: None,
//                     })
//                 } else {
//                     None
//                 },
//             );

//             let elem_width = if let Some(expand) = expand { expand.max(size[0]) } else { size[0] };

//             if let Some(r) = &mut render {
//                 r.pos[0] += elem_width;
//             }

//             width += elem_width;
//             height = height.max(size[1]);
//         });

//         [width, height]
//     }
// }
