use crate::*;

struct ForceBreak;

impl Element for ForceBreak {
    fn measure(&self, ctx: MeasureCtx) -> Option<ElementSize> {
        if let Some(breakable) = ctx.breakable {
            *breakable.break_count = 1;
        }

        None
    }

    fn draw(&self, ctx: DrawCtx) -> Option<ElementSize> {
        if let Some(breakable) = ctx.breakable {
            (breakable.get_location)(ctx.pdf, 0);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    #[test]
    fn test_force_break() {
        for output in ElementTestParams::default().run(&ForceBreak) {
            output.assert_size(None);

            if let Some(b) = output.breakable {
                b.assert_break_count(1);
                b.assert_extra_location_min_height(0.);
            }
        }
    }
}
