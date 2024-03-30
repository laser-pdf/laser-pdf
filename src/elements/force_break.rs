use crate::*;

pub struct ForceBreak;

impl Element for ForceBreak {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        FirstLocationUsage::WillUse
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        if let Some(breakable) = ctx.breakable {
            *breakable.break_count = 1;
        }

        ElementSize {
            width: None,
            height: None,
        }
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        if let Some(breakable) = ctx.breakable {
            // Note: This passes None as the height even though it always returns WillUse from
            // first_location_usage. Just because the first location has None height doesn't mean
            // it was skipped. In fact it would be incorrect if this element were to return
            // WillSkip. WillSkip implies that if the element is drawn with a full first height it
            // has to then look the same.
            (breakable.do_break)(ctx.pdf, 0, None);
        }

        ElementSize {
            width: None,
            height: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    #[test]
    fn test_force_break() {
        for output in ElementTestParams::default().run(&ForceBreak) {
            output.assert_size(ElementSize {
                width: None,
                height: None,
            });

            if let Some(b) = output.breakable {
                b.assert_break_count(1);
                b.assert_extra_location_min_height(None);
            }
        }
    }
}
