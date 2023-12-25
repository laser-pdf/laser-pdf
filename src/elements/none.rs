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
        run_element(1., 1., NoneElement)
            .assert_pages(1)
            .assert_linear();

        assert_eq!(
            NoneElement.measure(MeasureCtx {
                width: Some(1.),
                first_height: 1.,
                breakable: None
            }),
            None,
        );
    }
}
