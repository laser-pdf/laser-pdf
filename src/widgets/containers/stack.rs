use crate::widgets::*;
use crate::VAlign;

pub struct Stack<C> {
    content: C,
}

impl<C: Fn(&mut StackHandler)> Stack<C> {
    pub fn new(content: C) -> Self {
        Stack { content }
    }
}

impl<C: Fn(&mut StackHandler)> Element for Stack<C> {
    fn element(&self, width: Option<f64>, draw: Option<DrawContext>) -> [f64; 2] {
        // measure pass
        let mut handler = StackHandler {
            width,
            draw: None,
            size: [0.; 2],
        };
        (self.content)(&mut handler);

        // draw pass
        if let Some(mut draw) = draw {
            if let Some(ref mut next_draw_pos) = draw.next_draw_pos {
                if draw.draw_pos.height_available < handler.size[1] {
                    draw.draw_pos = next_draw_pos(draw.pdf, 0, [0.; 2]);
                }
            }

            let mut handler = StackHandler {
                draw: Some(&mut draw),
                ..handler
            };

            (self.content)(&mut handler);
        }

        handler.size
    }
}

pub struct StackHandler<'a, 'b, 'c> {
    width: Option<f64>,
    draw: Option<&'a mut DrawContext<'b, 'c>>,
    size: [f64; 2],
}

impl<'a, 'b, 'c> StackHandler<'a, 'b, 'c> {
    pub fn el<W: Element>(&mut self, widget: &W, v_align: VAlign) {
        if let Some(&mut ref mut draw) = self.draw {
            widget.element(
                self.width,
                Some(DrawContext {
                    pdf: draw.pdf,
                    draw_pos: {
                        let draw_pos = &draw.draw_pos;

                        match v_align {
                            VAlign::Top => draw_pos.clone(),
                            VAlign::Center => {
                                let size = widget.element(self.width, None);
                                let y_offset = (self.size[1] - size[1]) / 2.;
                                DrawPos {
                                    layer: draw_pos.layer.clone(),
                                    pos: [draw_pos.pos[0], draw_pos.pos[1] - y_offset],
                                    preferred_height: None,
                                    height_available: draw_pos.height_available - y_offset,
                                }
                            }
                            VAlign::Bottom => {
                                let size = widget.element(self.width, None);
                                let y_offset = self.size[1] - size[1];
                                DrawPos {
                                    layer: draw_pos.layer.clone(),
                                    pos: [draw_pos.pos[0], draw_pos.pos[1] - y_offset],
                                    preferred_height: None,
                                    height_available: draw_pos.height_available - y_offset,
                                }
                            }
                        }
                    },
                    full_height: 0.,
                    next_draw_pos: None,
                }),
            );
        } else {
            let size = widget.element(self.width, None);

            self.size[0] = self.size[0].max(size[0]);
            self.size[1] = self.size[1].max(size[1]);
        }
    }
}
