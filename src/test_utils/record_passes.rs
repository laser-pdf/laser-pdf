use std::cell::RefCell;

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
    Draw(DrawPass),
}

#[derive(PartialEq, Debug)]
pub struct DrawPass {
    pub width: WidthConstraint,
    pub first_height: f32,
    pub preferred_height: Option<f32>,
    pub page: usize,
    pub layer: usize,
    pub pos: (f32, f32),
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

    pub fn into_passes(self) -> Vec<Pass> {
        self.passes.into_inner()
    }

    pub fn assert_draw(&self, pass: DrawPass) {
        self.assert_draws(&[pass]);
    }

    pub fn assert_draws(&self, expected_draws: &[DrawPass]) {
        let passes = self.passes.borrow();
        let actual_draws: Vec<_> = passes
            .iter()
            .filter_map(|p| if let Pass::Draw(d) = p { Some(d) } else { None })
            .collect();

        assert!(
            expected_draws.iter().eq(actual_draws.iter().map(|d| *d)),
            "assertion `actual_draws == expected_draws` failed\nactual_draws: {:#?}\nexpected_draws: {:#?}",
            actual_draws,
            expected_draws,
        );
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
            assert_eq!(*b.extra_location_min_height, None);
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
