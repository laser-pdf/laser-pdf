use std::cell::RefCell;

use crate::*;

#[derive(PartialEq, Debug)]
pub struct Break {
    pub page: usize,
    pub layer: usize,
    pub pos: (f64, f64),
}

#[derive(PartialEq, Debug)]
pub struct BreakableDraw {
    pub full_height: f64,
    pub preferred_height_break_count: u32,
    pub breaks: Vec<Break>,
}

#[derive(PartialEq, Debug)]
pub enum Pass {
    FirstLocationUsage {
        width: WidthConstraint,
        first_height: f64,
        full_height: f64,
    },
    Measure {
        width: WidthConstraint,
        first_height: f64,

        /// Some implies a breakable context.
        full_height: Option<f64>,
    },
    Draw(DrawPass),
}

#[derive(PartialEq, Debug)]
pub struct DrawPass {
    pub width: WidthConstraint,
    pub first_height: f64,
    pub preferred_height: Option<f64>,
    pub page: usize,
    pub layer: usize,
    pub pos: (f64, f64),
    pub breakable: Option<BreakableDraw>,
}

/// This element can be used to test the methods that get called on an element. This is mostly
/// useful for testing containers. It also asserts that the two mutable references passed to a
/// breakable measure start at zero.
pub struct RecordPasses<E: Element> {
    element: E,
    passes: RefCell<Vec<Pass>>,
}

impl<E: Element> RecordPasses<E> {
    pub fn new(element: E) -> Self {
        RecordPasses {
            element,
            passes: RefCell::new(Vec::new()),
        }
    }

    pub fn assert_draw(&self, pass: DrawPass) {
        let passes = self.passes.borrow();
        let mut draw_passes = passes.iter().filter(|p| matches!(p, Pass::Draw { .. }));

        let draw_pass = draw_passes.next().unwrap();
        assert_eq!(draw_pass, &Pass::Draw(pass));
        assert_eq!(draw_passes.next(), None);
    }

    pub fn assert_draw_count(&self, count: usize) {
        let passes = self.passes.borrow();
        let draw_passes = passes.iter().filter(|p| matches!(p, Pass::Draw { .. }));
        assert_eq!(draw_passes.count(), count);
    }

    pub fn assert_measure_count(&self, count: usize) {
        let passes = self.passes.borrow();
        let measure_passes = passes.iter().filter(|p| matches!(p, Pass::Measure { .. }));
        assert_eq!(measure_passes.count(), count);
    }

    pub fn assert_first_location_usage_count(&self, count: usize) {
        let passes = self.passes.borrow();
        let first_location_usage_passes = passes
            .iter()
            .filter(|p| matches!(p, Pass::FirstLocationUsage { .. }));
        assert_eq!(first_location_usage_passes.count(), count);
    }
}

impl<E: Element> Element for RecordPasses<E> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        self.passes.borrow_mut().push(Pass::FirstLocationUsage {
            width: ctx.width,
            first_height: ctx.first_height,
            full_height: ctx.full_height,
        });

        self.element.first_location_usage(ctx)
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        if let Some(ref b) = ctx.breakable {
            assert_eq!(*b.break_count, 0);
            assert_eq!(*b.extra_location_min_height, 0.);
        }

        self.passes.borrow_mut().push(Pass::Measure {
            width: ctx.width,
            first_height: ctx.first_height,
            full_height: ctx.breakable.as_ref().map(|b| b.full_height),
        });

        self.element.measure(ctx)
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        let width = ctx.width;
        let first_height = ctx.first_height;
        let preferred_height = ctx.preferred_height;

        let page = ctx.location.layer.page.0;
        let layer = ctx.location.layer.layer.0;
        let pos = ctx.location.pos;

        let result;

        let breakable = if let Some(breakable) = ctx.breakable {
            let full_height = breakable.full_height;
            let preferred_height_break_count = breakable.preferred_height_break_count;

            let mut breaks = Vec::new();

            result = self.element.draw(DrawCtx {
                breakable: Some(crate::BreakableDraw {
                    get_location: &mut |pdf, location_idx| {
                        let location = (breakable.get_location)(pdf, location_idx);

                        breaks.push(Break {
                            page: location.layer.page.0,
                            layer: location.layer.layer.0,
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

        self.passes.borrow_mut().push(Pass::Draw(DrawPass {
            width,
            first_height,
            preferred_height,
            page,
            layer,
            pos,
            breakable,
        }));

        result
    }
}
