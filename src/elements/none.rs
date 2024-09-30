use crate::*;

pub struct NoneElement;

impl Element for NoneElement {
    fn first_location_usage(&self, _ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        FirstLocationUsage::NoneHeight
    }

    fn measure(&self, _ctx: MeasureCtx) -> ElementSize {
        ElementSize {
            width: None,
            height: None,
        }
    }

    fn draw(&self, _ctx: DrawCtx) -> ElementSize {
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
    fn none_element() {
        for output in ElementTestParams::default().run(&NoneElement) {
            output.assert_size(ElementSize {
                width: None,
                height: None,
            });

            if let Some(b) = output.breakable {
                b.assert_break_count(0)
                    .assert_extra_location_min_height(None);
            }
        }
    }
}
