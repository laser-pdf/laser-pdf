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

pub enum Pass {
    FirstLocationUsage,
    Measure,
    Draw,
}

pub struct BuildElementCtx {
    pub width: WidthConstraint,
    pub first_height: f64,
    pub full_height: Option<f64>,
    pub pass: Pass,
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
            full_height: Some(ctx.full_height),
            pass: Pass::FirstLocationUsage,
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
            full_height: ctx.breakable.as_ref().map(|b| b.full_height),
            pass: Pass::Measure,
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
            full_height: ctx.breakable.as_ref().map(|b| b.full_height),
            pass: Pass::Draw,
        };

        let mut ctx = Some(ctx);

        (self.0)(
            build_ctx,
            BuildElementCallback(&mut |e| ret = e.draw(ctx.take().unwrap())),
        );
        ret
    }
}
