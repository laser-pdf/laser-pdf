use crate::{widgets::break_whole, *};

pub struct MeasureLayout {
    width: Option<f64>,
    gap: f64,
    no_expand_width: f64,
    total_flex: u8,
    count: u8,
    expand_count: u8,
}

impl MeasureLayout {
    pub fn new(width: Option<f64>, gap: f64) -> Self {
        MeasureLayout {
            width,
            gap,
            no_expand_width: 0.,
            total_flex: 0,
            count: 0,
            expand_count: 0,
        }
    }

    pub fn add_fixed(&mut self, width: f64) {
        self.count += 1;
        self.no_expand_width += width;
    }

    pub fn add_expand(&mut self, fraction: u8) {
        self.count += 1;
        self.expand_count += 1;
        self.total_flex += fraction;
    }

    pub fn build(self) -> DrawLayout {
        let expand_gap_sub = self.gap * self.count as f64 / self.expand_count as f64;

        let remaining_width = self
            .width
            .map(|w| (w - self.no_expand_width + self.gap).max(0.))
            .unwrap_or(0.);

        DrawLayout {
            total_flex: self.total_flex,
            expand_gap_sub,
            remaining_width,
        }
    }
}

#[derive(Copy, Clone)]
pub struct DrawLayout {
    total_flex: u8,
    expand_gap_sub: f64,
    remaining_width: f64,
}

impl DrawLayout {
    pub fn expand_width(&self, fraction: u8) -> f64 {
        (self.remaining_width * fraction as f64 / self.total_flex as f64 - self.expand_gap_sub)
            .max(0.)
    }
}

enum Pass<'a, 'b, 'c> {
    Measure {
        layout: &'a mut MeasureLayout,
    },
    Render {
        gap: f64,
        layout: DrawLayout,
        width: f64,

        // The height on the last draw pos.
        height: &'a mut f64,

        // None means it's still the first element. This is needed for things like not adding a gap
        // before the first element.
        offset: Option<f64>,

        last_draw_rect: u32,

        draw: Option<DrawCtx<'b, 'c>>,
    },
}

pub struct FlexHandler<'a, 'b, 'c>(Pass<'a, 'b, 'c>);

impl<'a, 'b, 'c> FlexHandler<'a, 'b, 'c> {
    pub fn element<W: Element>(&mut self, widget: &W, flex: Flex) {
        match self.0 {
            Pass::Measure {
                layout: &mut ref mut layout,
            } => match flex {
                Flex::Expand(factor) => layout.add_expand(factor),
                Flex::SelfSized => layout.add_fixed(widget.draw(None, None)[0]),
                Flex::Fixed(width) => layout.add_fixed(width),
            },
            Pass::Render {
                ref layout,
                gap,
                width,
                height: &mut ref mut height,
                ref mut offset,
                ref mut last_draw_rect,
                ref mut draw,
            } => {
                let expand = match flex {
                    Flex::Expand(flex) => Some(layout.expand_width(flex)),
                    Flex::Fixed(width) => Some(width),
                    Flex::SelfSized => None,
                };

                let offset = if let Some(offset) = offset {
                    *offset += gap;
                    if let Some(context) = draw {
                        context.location.pos[0] += gap;
                    }
                    offset
                } else {
                    *offset = Some(0.);
                    offset.as_mut().unwrap()
                };

                let mut draw_rect = 0;

                let widget_size = if let Some(context) = draw {
                    if let Some(ref mut next_location) = context.next_location {
                        let mut height_available = context.location.height_available;

                        widget.draw(
                            expand,
                            // if expand { Some(expand_width) } else { None },
                            Some(DrawCtx {
                                pdf: context.pdf,
                                location: context.location.clone(),
                                full_height: 0.0,
                                breakable: Some(BreakableDraw {
                                    get_location: &mut |pdf, draw_rect_id, _size| {
                                        draw_rect = draw_rect_id + 1;

                                        if draw_rect > *last_draw_rect {
                                            *last_draw_rect = draw_rect;
                                            *height = 0.;
                                        }

                                        let mut new_location = next_location(
                                            pdf,
                                            draw_rect_id,
                                            // We have to use the full height_available here, because
                                            // otherwise the answer would change from element to
                                            // element.
                                            [width, height_available],
                                        );

                                        height_available = new_location.height_available;

                                        new_location.pos[0] += *offset;

                                        new_location
                                    },
                                    ..break_ctx
                                }),
                            }),
                        )
                    } else {
                        widget.draw(
                            expand,
                            // if expand { Some(expand_width) } else { None },
                            Some(DrawCtx {
                                pdf: context.pdf,
                                location: context.location.clone(),
                                full_height: 0.0,
                                next_location: None,
                            }),
                        )
                    }
                } else {
                    widget.draw(expand, None)
                };

                let elem_width = if let Some(expand) = expand {
                    expand.max(widget_size[0])
                } else {
                    widget_size[0]
                };

                if let Some(context) = draw {
                    context.location.pos[0] += elem_width;
                }

                *offset += elem_width;

                if *last_draw_rect == draw_rect {
                    *height = height.max(widget_size[1]);
                }
            }
        }
    }

    pub fn flex_gap(&mut self, flex: u8) {
        match self.0 {
            Pass::Measure {
                layout: &mut ref mut layout,
            } => layout.add_expand(flex),
            Pass::Render {
                ref layout,
                gap,
                width: _,
                height: _,
                ref mut offset,
                last_draw_rect: _,
                ref mut draw,
            } => {
                let offset = if let Some(offset) = offset {
                    *offset += gap;
                    if let Some(context) = draw {
                        context.location.pos[0] += gap;
                    }
                    offset
                } else {
                    *offset = Some(0.);
                    offset.as_mut().unwrap()
                };

                let elem_width = layout.expand_width(flex);

                if let Some(context) = draw {
                    context.location.pos[0] += elem_width;
                }

                *offset += elem_width;
            }
        }
    }
}

pub enum Flex {
    Expand(u8),
    SelfSized,
    Fixed(f64),
}

/// 0: List callback, 1: Gap
pub struct FlexList<F: Fn(&mut FlexHandler)>(pub F, pub f64);

impl<F: Fn(&mut FlexHandler)> Element for FlexList<F> {
    fn insufficient_first_height(&self, ctx: InsufficientFirstHeightCtx) -> bool {
        false
    }

    fn measure(&self, ctx: MeasureCtx) -> Option<ElementSize> {
        let mut layout = MeasureLayout::new(width, self.1);

        let mut handler = FlexHandler(Pass::Measure {
            layout: &mut layout,
        });

        self.0(&mut handler);

        let min_width = layout.no_expand_width + layout.count.saturating_sub(1) as f64 * layout.gap;

        let width = if let Some(w) = width {
            w.max(min_width)
        } else {
            min_width
        };

        let mut height = 0.0;

        let mut handler = FlexHandler(Pass::Render {
            layout: layout.build(),
            gap: self.1,
            width,
            height: &mut height,
            offset: None,
            last_draw_rect: 0,
            draw,
        });

        self.0(&mut handler);

        Some(ElementSize {
            width: width,
            height: Some(height),
        })
    }

    fn draw(&self, ctx: DrawCtx) -> Option<ElementSize> {
        let mut layout = MeasureLayout::new(width, self.1);

        let mut handler = FlexHandler(Pass::Measure {
            layout: &mut layout,
        });

        self.0(&mut handler);

        let min_width = layout.no_expand_width + layout.count.saturating_sub(1) as f64 * layout.gap;

        let width = if let Some(w) = width {
            w.max(min_width)
        } else {
            min_width
        };

        let mut height = 0.0;

        let mut handler = FlexHandler(Pass::Render {
            layout: layout.build(),
            gap: self.1,
            width,
            height: &mut height,
            offset: None,
            last_draw_rect: 0,
            draw,
        });

        self.0(&mut handler);

        Some(ElementSize {
            width: width,
            height: Some(height),
        })
    }
}

pub fn flex_list<F: Fn(&mut FlexHandler)>(content: F, gap: f64) -> impl Element {
    break_whole(FlexList(content, gap))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout() {
        {
            let mut layout = MeasureLayout::new(Some(100.), 4.);
            layout.add_expand(1);
            layout.add_expand(1);
            layout.add_expand(1);

            let draw_layout = layout.build();

            assert_eq!(
                draw_layout.expand_width(1)
                    + draw_layout.expand_width(1)
                    + draw_layout.expand_width(1)
                    + 2. * 4.,
                100.,
            );
        }

        {
            let mut layout = MeasureLayout::new(Some(100.), 4.);
            layout.add_expand(1);
            layout.add_fixed(50.);
            layout.add_expand(1);

            let draw_layout = layout.build();

            assert_eq!(
                draw_layout.expand_width(1) + 50. + draw_layout.expand_width(1) + 2. * 4.,
                100.,
            );

            assert_eq!(draw_layout.expand_width(1), 21.);
        }

        {
            let mut layout = MeasureLayout::new(Some(100.), 4.);
            layout.add_expand(1);
            layout.add_fixed(25.);
            layout.add_expand(1);
            layout.add_fixed(25.);

            let draw_layout = layout.build();

            assert_eq!(
                draw_layout.expand_width(1) + 25. + draw_layout.expand_width(1) + 25. + 3. * 4.,
                100.,
            );

            assert_eq!(draw_layout.expand_width(1), 19.);
        }

        {
            let mut layout = MeasureLayout::new(Some(100.), 4.);
            layout.add_fixed(25.);
            layout.add_expand(1);
            layout.add_fixed(25.);
            layout.add_expand(1);
            layout.add_fixed(25.);

            let draw_layout = layout.build();

            assert_eq!(
                25. + draw_layout.expand_width(1)
                    + 25.
                    + draw_layout.expand_width(1)
                    + 25.
                    + 4. * 4.,
                100.,
            );
        }

        {
            let mut layout = MeasureLayout::new(Some(100.), 3.);
            layout.add_fixed(25.);
            layout.add_expand(2);
            layout.add_fixed(25.);
            layout.add_expand(1);
            layout.add_fixed(25.);

            let draw_layout = layout.build();

            assert_eq!(
                25. + draw_layout.expand_width(2)
                    + 25.
                    + draw_layout.expand_width(1)
                    + 25.
                    + 3. * 4.,
                100.,
            );
        }

        {
            let mut layout = MeasureLayout::new(Some(100.), 4.);
            layout.add_fixed(25.);
            layout.add_expand(2);
            layout.add_fixed(25.);
            layout.add_expand(1);
            layout.add_fixed(25.);

            let draw_layout = layout.build();

            // not enough space
            assert_eq!(draw_layout.expand_width(1), 0.);

            assert!(
                (25. + draw_layout.expand_width(2)
                    + 25.
                    + draw_layout.expand_width(1)
                    + 25.
                    + 4. * 4.)
                    > 100.,
            );
        }

        {
            let mut layout = MeasureLayout::new(Some(22.), 2.);
            layout.add_expand(2);
            layout.add_fixed(14.);
            layout.add_expand(1);

            let draw_layout = layout.build();

            assert_eq!(
                draw_layout.expand_width(2) + 14. + draw_layout.expand_width(1) + 2. * 2.,
                22.,
            );
        }
    }
}
