use crate::{widgets::break_whole, *};

pub struct BreakListHandler<'a, 'b> {
    width: Option<f64>,
    draw: Option<DrawContext<'a, 'b>>,
    gap: f64,
    start: bool,
    max_width: f64,
    x_offset: f64,
    y_offset: f64,
    line_height: f64,
}

impl<'a, 'b> BreakListHandler<'a, 'b> {
    pub fn el<W: Element>(&mut self, el: &W) {
        if self.start {
            self.start = false;
        } else {
            self.x_offset += self.gap;
        }

        match (self.width, &mut self.draw) {
            (None, None) => {
                let [el_width, el_height] = el.element(None, None);

                self.x_offset += el_width;
                self.line_height = self.line_height.max(el_height);
            }
            (None, Some(_)) => (),
            (Some(width), None) => {
                let [el_width, el_height] = el.element(None, None);
                if self.x_offset > 0. && self.x_offset + el_width > width {
                    self.x_offset = el_width;
                    self.y_offset += self.line_height + self.gap;
                    self.line_height = el_height;
                } else {
                    self.x_offset += el_width;
                    self.line_height = self.line_height.max(el_height);
                }
            }
            (Some(width), Some(_)) => {
                if self.x_offset > 0. {
                    let [el_width, ..] = el.element(None, None);

                    if self.x_offset + el_width > width {
                        self.x_offset = 0.;
                        self.y_offset += self.line_height + self.gap;
                        self.line_height = 0.;
                    }
                }
            }
        }

        if let Some(DrawContext {
            pdf: &mut ref mut pdf,
            ref mut draw_pos,
            full_height: _,
            next_draw_pos: _,
        }) = self.draw
        {
            let draw_pos = DrawPos {
                layer: draw_pos.layer.clone(),
                pos: [
                    draw_pos.pos[0] + self.x_offset,
                    draw_pos.pos[1] - self.y_offset,
                ],
                preferred_height: None,
                height_available: draw_pos.height_available - self.y_offset,
            };

            let [el_width, el_height] = el.element(
                None,
                Some(DrawContext {
                    pdf,
                    draw_pos,
                    full_height: 0.,
                    next_draw_pos: None,
                }),
            );

            self.x_offset += el_width;
            self.line_height = self.line_height.max(el_height);
        }

        // This is correct as long as breaking only ever done before placing an element,
        // and not after.
        self.max_width = self.max_width.max(self.x_offset);
    }
}

/// This behaves a bit like flexbox wrapping.
pub fn break_list<F: Fn(&mut BreakListHandler)>(content: F, gap: f64) -> impl Element {
    break_whole(move |width: Option<f64>, draw: Option<DrawContext>| {
        let mut handler = BreakListHandler {
            width,
            draw,
            gap,
            start: true,
            max_width: width.unwrap_or(0.),
            x_offset: 0.,
            y_offset: 0.,
            line_height: 0.,
        };

        content(&mut handler);

        [handler.max_width, handler.y_offset + handler.line_height]
    })
}
