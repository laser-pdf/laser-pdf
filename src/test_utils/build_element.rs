use crate::*;

pub struct BuildElementReturnToken(());

// Is just here to ensure the callback can't be used more than once.
pub struct BuildElementCallback<'a>(&'a mut dyn FnMut(&dyn Element));

impl<'a> BuildElementCallback<'a> {
    pub fn call(self, element: impl Element) -> BuildElementReturnToken {
        self.0(&element);
        BuildElementReturnToken(())
    }
}

pub struct BreakableDraw {
    pub full_height: f32,
    pub preferred_height_break_count: u32,
}

pub enum Pass {
    FirstLocationUsage {
        full_height: f32,
    },
    Measure {
        full_height: Option<f32>,
    },
    Draw {
        preferred_height: Option<f32>,
        breakable: Option<BreakableDraw>,
    },
}

pub struct BuildElementCtx {
    pub width: WidthConstraint,
    pub first_height: f32,
    pub pass: Pass,
}

impl BuildElementCtx {
    pub fn is_breakable(&self) -> bool {
        match self.pass {
            Pass::FirstLocationUsage { .. } => true,
            Pass::Measure { full_height } => full_height.is_some(),
            Pass::Draw { ref breakable, .. } => breakable.is_some(),
        }
    }
}

pub struct BuildElement<F: Fn(BuildElementCtx, BuildElementCallback) -> BuildElementReturnToken>(
    pub F,
);

impl<F: Fn(BuildElementCtx, BuildElementCallback) -> BuildElementReturnToken> Element
    for BuildElement<F>
{
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        let mut ret = FirstLocationUsage::NoneHeight;

        let build_ctx = BuildElementCtx {
            width: ctx.width,
            first_height: ctx.first_height,
            pass: Pass::FirstLocationUsage {
                full_height: ctx.full_height,
            },
        };

        let mut ctx = Some(ctx);

        (self.0)(
            build_ctx,
            BuildElementCallback(&mut |e| ret = e.first_location_usage(ctx.take().unwrap())),
        );
        ret
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        let mut ret = ElementSize {
            width: None,
            height: None,
        };

        let build_ctx = BuildElementCtx {
            width: ctx.width,
            first_height: ctx.first_height,
            pass: Pass::Measure {
                full_height: ctx.breakable.as_ref().map(|b| b.full_height),
            },
        };

        let mut ctx = Some(ctx);

        (self.0)(
            build_ctx,
            BuildElementCallback(&mut |e| ret = e.measure(ctx.take().unwrap())),
        );
        ret
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        let mut ret = ElementSize {
            width: None,
            height: None,
        };

        let build_ctx = BuildElementCtx {
            width: ctx.width,
            first_height: ctx.first_height,
            pass: Pass::Draw {
                preferred_height: ctx.preferred_height,
                breakable: ctx.breakable.as_ref().map(|b| BreakableDraw {
                    full_height: b.full_height,
                    preferred_height_break_count: b.preferred_height_break_count,
                }),
            },
        };

        let mut ctx = Some(ctx);

        (self.0)(
            build_ctx,
            BuildElementCallback(&mut |e| ret = e.draw(ctx.take().unwrap())),
        );
        ret
    }
}
