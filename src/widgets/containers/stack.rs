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
    fn insufficient_first_height(&self, ctx: InsufficientFirstHeightCtx) -> bool {
        false
    }

    fn measure(&self, ctx: MeasureCtx) -> Option<ElementSize> {
        // measure pass
        let mut handler = StackHandler {
            width,
            draw: None,
            size: [0.; 2],
        };
        (self.content)(&mut handler);

        // draw pass
        if let Some(mut draw) = draw {
            if let Some(ref mut next_location) = draw.next_location {
                if ctx.location.height_available < handler.size[1] {
                    ctx.location = next_location(draw.pdf, 0, [0.; 2]);
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

    fn draw(&self, ctx: DrawCtx) -> Option<ElementSize> {
        // measure pass
        let mut handler = StackHandler {
            width,
            draw: None,
            size: [0.; 2],
        };
        (self.content)(&mut handler);

        // draw pass
        if let Some(mut draw) = draw {
            if let Some(ref mut next_location) = draw.next_location {
                if ctx.location.height_available < handler.size[1] {
                    ctx.location = next_location(draw.pdf, 0, [0.; 2]);
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
    draw: Option<&'a mut DrawCtx<'b, 'c>>,
    size: [f64; 2],
}

impl<'a, 'b, 'c> StackHandler<'a, 'b, 'c> {
    pub fn el<W: Element>(&mut self, widget: &W, v_align: VAlign) {
        if let Some(&mut ref mut draw) = self.draw {
            widget.draw(
                self.width,
                Some(DrawCtx {
                    pdf: draw.pdf,
                    location: {
                        let location = &draw.location;

                        match v_align {
                            VAlign::Top => location.clone(),
                            VAlign::Center => {
                                let size = widget.draw(self.width, None);
                                let y_offset = (self.size[1] - size[1]) / 2.;
                                Location {
                                    layer: location.layer.clone(),
                                    pos: [location.pos[0], location.pos[1] - y_offset],
                                    preferred_height: None,
                                    height_available: location.height_available - y_offset,
                                }
                            }
                            VAlign::Bottom => {
                                let size = widget.draw(self.width, None);
                                let y_offset = self.size[1] - size[1];
                                Location {
                                    layer: location.layer.clone(),
                                    pos: [location.pos[0], location.pos[1] - y_offset],
                                    preferred_height: None,
                                    height_available: location.height_available - y_offset,
                                }
                            }
                        }
                    },
                    full_height: 0.,
                    next_location: None,
                }),
            );
        } else {
            let size = widget.draw(self.width, None);

            self.size[0] = self.size[0].max(size[0]);
            self.size[1] = self.size[1].max(size[1]);
        }
    }
}
