use crate::{widgets::break_whole, *};

pub struct BreakListHandler<'a, 'b> {
    width: Option<f64>,
    draw: Option<DrawCtx<'a, 'b>>,
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
                let [el_width, el_height] = el.draw(None, None);

                self.x_offset += el_width;
                self.line_height = self.line_height.max(el_height);
            }
            (None, Some(_)) => (),
            (Some(width), None) => {
                let [el_width, el_height] = el.draw(None, None);
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
                    let [el_width, ..] = el.draw(None, None);

                    if self.x_offset + el_width > width {
                        self.x_offset = 0.;
                        self.y_offset += self.line_height + self.gap;
                        self.line_height = 0.;
                    }
                }
            }
        }

        if let Some(DrawCtx {
            pdf: &mut ref mut pdf,
            ref mut location,
            full_height: _,
            next_location: _,
        }) = self.draw
        {
            let location = Location {
                layer: location.layer.clone(),
                pos: [
                    location.pos[0] + self.x_offset,
                    location.pos[1] - self.y_offset,
                ],
                preferred_height: None,
                height_available: location.height_available - self.y_offset,
            };

            let [el_width, el_height] = el.draw(
                None,
                Some(DrawCtx {
                    pdf,
                    location,
                    full_height: 0.,
                    next_location: None,
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
    break_whole(move |width: Option<f64>, draw: Option<DrawCtx>| {
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

        Some(ElementSize {
            width: handler.max_width,
            height: Some(handler.y_offset + handler.line_height),
        })
    })
}
