use crate::*;

pub struct VListHandler<'a, 'b> {
    max_width: Option<f32>,
    render: Option<DrawCtx<'a, 'b>>,
    width: f32,
    height: f32,
    break_page: bool,
    line: Option<f32>,
    gap: f32,
}

impl<'a, 'b> VListHandler<'a, 'b> {
    pub fn element<W: Widget>(&mut self, widget: &W, break_page: bool) {
        let size = match self.render {
            Some(DrawCtx {
                ref mut pdf,
                ref mut location,
                full_height,
                next_location: Some(ref mut next_location),
            }) => {
                let size = widget.widget(self.max_width, None);

                if self.break_page && break_page && size[1] > location.height_available {
                    *location = next_location(pdf);

                    self.height = 2.0 * self.gap;

                    location.pos[1] -= self.gap;
                    if let Some(line) = self.line {
                        crate::utils::line(
                            &mut location.layer,
                            location.pos,
                            self.max_width.unwrap_or(1.0),
                            line,
                        );
                    }
                    location.pos[1] -= self.gap;
                    location.height_available =
                        (location.height_available - 4.0 * self.gap).max(0.0);
                }

                let height = &mut self.height;
                let gap = self.gap;
                let max_width = self.max_width;
                let line = self.line;

                widget.widget(
                    self.max_width,
                    Some(DrawCtx {
                        pdf,
                        location: *location,
                        full_height,
                        breakable: Some(BreakableDraw {
                            get_location: &mut |pdf| {
                                *location = next_location(pdf);
                                *height = 0.0;

                                location.height_available =
                                    (location.height_available - 2.0 * gap).max(0.0);

                                *location
                            },
                            ..break_ctx
                        }),
                    }),
                )
            }
            Some(DrawCtx {
                ref mut pdf,
                location,
                full_height,
                next_location: None,
            }) => widget.widget(
                self.max_width,
                Some(DrawCtx {
                    pdf,
                    location,
                    full_height,
                    next_location: None,
                }),
            ),
            None => widget.widget(self.max_width, None),
        };

        self.width = self.width.max(size[0]);
        self.height += size[1];

        self.height += 2.0 * self.gap;
        if let Some(ref mut context) = self.render {
            context.location.pos[1] -= size[1];
            context.location.height_available -= size[1];

            context.location.pos[1] -= self.gap;
            if let Some(line) = self.line {
                // println!("{:?} {:?}", context.location.pos, size);
                crate::utils::line(
                    &mut context.location.layer,
                    context.location.pos,
                    self.max_width.unwrap_or(1.0),
                    line,
                );
            }
            context.location.pos[1] -= self.gap;
            context.location.height_available -= 2.0 * self.gap;
        }
    }
}

pub struct VList<F: Fn(&mut VListHandler)> {
    pub list: F,
    pub break_page: bool,
    pub line: Option<f32>,
    pub gap: f32,
}

impl<F: Fn(&mut VListHandler)> VList<F> {
    pub fn new(list: F, break_page: bool, line: Option<f32>, gap: f32) -> Self {
        VList {
            list,
            break_page,
            line,
            gap,
        }
    }
}

impl<F: Fn(&mut VListHandler)> Widget for VList<F> {
    fn widget(&self, width: Option<f32>, mut render: Option<DrawCtx>) -> [f32; 2] {
        if let Some(context) = &mut render {
            context.location.pos[1] -= self.gap;
            if let Some(line) = self.line {
                crate::utils::line(
                    &mut context.location.layer,
                    context.location.pos,
                    width.unwrap_or(1.0),
                    line,
                );
            }
            context.location.pos[1] -= self.gap;
            context.location.height_available =
                (context.location.height_available - 4.0 * self.gap).max(0.0);
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

        Some(ElementSize {
            width: handler.width,
            height: Some(handler.height),
        })
    }
}

pub struct HListHandler<'a, 'b> {
    draw: Option<DrawCtx<'a, 'b>>,
    width: f32,
    height: f32,
}

impl<'a, 'b> HListHandler<'a, 'b> {
    pub fn element<W: Widget>(&mut self, widget: &W) {
        let size = {
            let draw = if let Some(DrawCtx {
                ref mut pdf,
                location,
                full_height,
                next_location: _,
            }) = self.draw
            {
                Some(DrawCtx {
                    pdf,
                    location,
                    full_height,
                    next_location: None,
                })
            } else {
                None
            };
            widget.widget(None, draw)
        };

        self.width += size[0];
        self.height = self.height.max(size[1]);
        if let Some(ref mut context) = self.draw {
            context.location.pos[0] += size[0];
        }
    }
}

pub struct HList<F: Fn(&mut HListHandler)>(pub F);

impl<F: Fn(&mut HListHandler)> Widget for HList<F> {
    fn widget(&self, _width: Option<f32>, draw: Option<DrawCtx>) -> [f32; 2] {
        let mut handler = HListHandler {
            draw,
            width: 0.0,
            height: 0.0,
        };

        self.0(&mut handler);

        Some(ElementSize {
            width: handler.width,
            height: Some(handler.height),
        })
    }
}

// pub enum FlexMode {
//     Expand,
//     SelfSized,
//     Fixed(f32),
// }

// pub struct FlexList<F: Fn(&mut dyn FnMut(&dyn Widget, FlexMode))>(pub F);

// impl<F: Fn(&mut dyn FnMut(&dyn Widget, FlexMode))> Widget for FlexList<F> {
//     fn widget(&self, width: Option<f32>, mut render: Option<DrawCtx>) -> [f32; 2] {
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

//         let mut width: f32 = 0.0;
//         let mut height: f32 = 0.0;

//         self.0(&mut |w, expand| {
//             let expand = match expand { Expand => Some(expand_width), Fixed(width) => Some(width), _ => None };

//             let size = w.widget(
//                 expand,
//                 // if expand { Some(expand_width) } else { None },
//                 if let Some(r) = &mut render {
//                     Some(DrawCtx {
//                         pos: r.pos,
//                         height_available: r.height_available,
//                         pdf: r.pdf,
//                         next_location: None,
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
