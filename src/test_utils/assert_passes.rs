use std::cell::Cell;

use crate::*;

#[derive(PartialEq, Debug)]
pub struct Break {
    pub page: usize,
    pub layer: usize,
    pub pos: (f32, f32),
}

#[derive(PartialEq, Debug)]
pub struct BreakableDraw {
    pub full_height: f32,
    pub preferred_height_break_count: u32,
    pub breaks: Vec<Break>,
}

#[derive(PartialEq, Debug)]
pub enum Pass {
    FirstLocationUsage {
        width: WidthConstraint,
        first_height: f32,
        full_height: f32,
    },
    Measure {
        width: WidthConstraint,
        first_height: f32,

        /// Some implies a breakable context.
        full_height: Option<f32>,
    },
    Draw {
        width: WidthConstraint,
        first_height: f32,
        preferred_height: Option<f32>,
        page: usize,
        layer: usize,
        pos: (f32, f32),
        breakable: Option<BreakableDraw>,
    },
}

/// This element can be used to test the methods that get called on an element. This is mostly
/// useful for testing containers. It also asserts that the two mutable references passed to a
/// breakable measure start at zero.
pub struct AssertPasses<E: Element> {
    element: E,
    passes: Vec<Pass>,
    current: Cell<usize>,
}

impl<E: Element> AssertPasses<E> {
    pub fn new(element: E, passes: Vec<Pass>) -> Self {
        AssertPasses {
            element,
            passes,
            current: Cell::new(0),
        }
    }
}

impl<E: Element> Drop for AssertPasses<E> {
    fn drop(&mut self) {
        // If this throws, make sure there isn't another error above, because this also gets dropped
        // while unwinding the stack.
        //
        // TODO: Maybe we can check if an unwind is already in progress here?
        assert_eq!(self.current.get(), self.passes.len());
    }
}

impl<E: Element> Element for AssertPasses<E> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        let idx = self.current.get();
        self.current.set(idx + 1);

        let current = &self.passes[idx];

        assert_eq!(
            &Pass::FirstLocationUsage {
                width: ctx.width,
                first_height: ctx.first_height,
                full_height: ctx.full_height,
            },
            current,
        );

        self.element.first_location_usage(ctx)
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        let idx = self.current.get();
        self.current.set(idx + 1);

        let current = &self.passes[idx];

        if let Some(ref b) = ctx.breakable {
            assert_eq!(*b.break_count, 0);
            assert_eq!(*b.extra_location_min_height, None);
        }

        assert_eq!(
            &Pass::Measure {
                width: ctx.width,
                first_height: ctx.first_height,
                full_height: ctx.breakable.as_ref().map(|b| b.full_height),
            },
            current,
        );

        self.element.measure(ctx)
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        let idx = self.current.get();
        self.current.set(idx + 1);

        let current = &self.passes[idx];

        let width = ctx.width;
        let first_height = ctx.first_height;
        let preferred_height = ctx.preferred_height;

        let page = ctx.location.page_idx;
        let layer = ctx.location.layer_idx;
        let pos = ctx.location.pos;

        let result;

        let breakable = if let Some(breakable) = ctx.breakable {
            let full_height = breakable.full_height;
            let preferred_height_break_count = breakable.preferred_height_break_count;

            let mut breaks = Vec::new();

            result = self.element.draw(DrawCtx {
                breakable: Some(crate::BreakableDraw {
                    do_break: &mut |pdf, location_idx, height| {
                        let location = (breakable.do_break)(pdf, location_idx, height);

                        breaks.push(Break {
                            page: location.page_idx,
                            layer: location.layer_idx,
                            pos: location.pos,
                        });

                        location
                    },
                    ..breakable
                }),
                ..ctx
            });

            Some(BreakableDraw {
                full_height,
                preferred_height_break_count,
                breaks,
            })
        } else {
            result = self.element.draw(ctx);
            None
        };

        assert_eq!(
            &Pass::Draw {
                width,
                first_height,
                preferred_height,
                page,
                layer,
                pos,
                breakable,
            },
            current,
        );

        result
    }
}
