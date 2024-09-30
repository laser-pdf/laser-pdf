use crate::*;

pub struct FakeImage {
    pub width: f64,
    pub height: f64,
}

impl FakeImage {
    fn size(&self, width: WidthConstraint) -> (f64, ElementSize) {
        let width = width.constrain(self.width);

        let scale = width / self.width;
        let size = (width, self.height * scale);

        (
            size.1,
            ElementSize {
                width: Some(size.0),
                height: Some(size.1),
            },
        )
    }
}

impl Element for FakeImage {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        if ctx.break_appropriate_for_min_height(self.size(ctx.width).0) {
            FirstLocationUsage::WillSkip
        } else {
            FirstLocationUsage::WillUse
        }
    }

    fn measure(&self, mut ctx: MeasureCtx) -> ElementSize {
        let (height, size) = self.size(ctx.width);
        ctx.break_if_appropriate_for_min_height(height);
        size
    }

    fn draw(&self, mut ctx: DrawCtx) -> ElementSize {
        let (height, size) = self.size(ctx.width);
        ctx.break_if_appropriate_for_min_height(height);
        size
    }
}
