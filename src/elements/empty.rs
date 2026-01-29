use crate::*;

/// An element with a zero size.
///
/// Compared to [crate::elements::none::NoneElement] it does not trigger collapse behavior.
pub struct Empty;

impl Element for Empty {
    fn first_location_usage(&self, _ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        FirstLocationUsage::WillUse
    }

    fn measure(&self, _ctx: MeasureCtx) -> ElementSize {
        ElementSize {
            width: Some(0.),
            height: Some(0.),
        }
    }

    fn draw(&self, _ctx: DrawCtx) -> ElementSize {
        ElementSize {
            width: Some(0.),
            height: Some(0.),
        }
    }
}
