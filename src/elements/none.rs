use crate::*;

pub struct NoneElement;

impl Element for NoneElement {
    fn measure(&self, _ctx: MeasureCtx) -> Option<ElementSize> {
        None
    }

    fn draw(&self, _ctx: DrawCtx) -> Option<ElementSize> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    #[test]
    fn none_element() {
        for output in ElementTestParams::default().run(&NoneElement) {
            output.assert_size(None);

            if let Some(b) = output.breakable {
                b.assert_break_count(0).assert_extra_location_min_height(0.);
            }
        }
    }
}
