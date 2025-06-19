use crate::*;

/// A null element that takes up no space.
///
/// It uses `None` as the size on both axes, meaning it will trigger collapsing in containers that
/// support it, such as a `Column` with `collapse: true`. Collapsing means that for example the gaps
/// before and after the element will be combined into one and if all elements in a container are
/// collapsed, the container itself will also have a `None` size on the relevant axis.
///
/// This element is useful for conditional layouts where you may want to
/// include an element or nothing at all based on some condition.
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
