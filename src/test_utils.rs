pub mod assert_passes;
pub mod binary_snapshots;
pub mod build_element;
pub mod element_proxy;
pub mod fake_image;
pub mod fake_text;
pub mod frantic_jumper;
pub mod old;
pub mod record_passes;

pub use build_element::BuildElement;
pub use element_proxy::ElementProxy;
pub use fake_image::FakeImage;
pub use fake_text::FakeText;
pub use frantic_jumper::FranticJumper;
pub use old::*;

use crate::{utils::max_optional_size, *};

use self::build_element::{BuildElementCallback, BuildElementReturnToken};

pub struct TestElementParams {
    pub width: WidthConstraint,
    pub first_height: f32,
    pub preferred_height: Option<f32>,
    pub breakable: Option<TestElementParamsBreakable>,
    pub pos: (f32, f32),
    pub page_size: (f32, f32),
}

pub struct TestElementParamsBreakable {
    pub preferred_height_break_count: u32,
    pub full_height: f32,
}

impl Default for TestElementParams {
    fn default() -> Self {
        TestElementParams {
            width: WidthConstraint {
                max: 10.,
                expand: true,
            },
            first_height: 10.,
            preferred_height: None,
            breakable: None,
            pos: (0., 10.),
            page_size: (10., 10.),
        }
    }
}

impl Default for TestElementParamsBreakable {
    fn default() -> Self {
        TestElementParamsBreakable {
            preferred_height_break_count: 0,
            full_height: 10.,
        }
    }
}

#[derive(Debug)]
pub struct ElementTestOutput {
    pub size: ElementSize,
    pub breakable: Option<ElementTestOutputBreakable>,
}

#[derive(Debug)]
pub struct ElementTestOutputBreakable {
    pub break_count: u32,
    pub extra_location_min_height: Option<f32>,

    pub first_location_usage: FirstLocationUsage,
}

impl ElementTestOutput {
    pub fn assert_no_breaks(&self) -> &Self {
        if let Some(b) = &self.breakable {
            b.assert_break_count(0);
        }

        self
    }

    pub fn assert_size(&self, size: ElementSize) -> &Self {
        assert_eq!(self.size, size);
        self
    }
}

impl ElementTestOutputBreakable {
    pub fn assert_break_count(&self, break_count: u32) -> &Self {
        assert_eq!(self.break_count, break_count);
        self
    }

    pub fn assert_extra_location_min_height(
        &self,
        extra_location_min_height: Option<f32>,
    ) -> &Self {
        assert_eq!(self.extra_location_min_height, extra_location_min_height);
        self
    }

    pub fn assert_first_location_usage(&self, expected: FirstLocationUsage) -> &Self {
        assert_eq!(self.first_location_usage, expected);
        self
    }
}

pub fn test_element(
    TestElementParams {
        width,
        first_height,
        preferred_height,
        breakable,
        pos,
        page_size,
    }: TestElementParams,
    build_element: impl Fn(bool, BuildElementCallback) -> BuildElementReturnToken,
) -> ElementTestOutput {
    let first_pos = (
        pos.0,
        breakable
            .as_ref()
            .map_or(pos.1, |b| pos.1 - (b.full_height - first_height)),
    );

    let element = BuildElement(|_, callback| build_element(false, callback));
    let element_with_asserts = BuildElement(|_, callback| build_element(true, callback));

    let measure = measure_element(
        &element,
        width,
        first_height,
        breakable.as_ref().map(|b| b.full_height),
    );
    let draw = draw_element(
        &element_with_asserts,
        width,
        first_height,
        preferred_height,
        first_pos,
        page_size,
        breakable.as_ref().map(|b| BreakableDrawConfig {
            pos,
            full_height: b.full_height,
            preferred_height_break_count: b.preferred_height_break_count,
        }),
    );

    assert_eq!(measure.size.width, draw.size.width);

    let preferred_break_count = breakable
        .as_ref()
        .map(|b| b.preferred_height_break_count)
        .unwrap_or(0);

    if measure.extra_location_min_height.is_some() && preferred_break_count > measure.break_count {
        assert_eq!(draw.break_count, preferred_break_count);
        assert!(draw.size.height >= measure.extra_location_min_height);
        assert!(
            draw.size.height
                <= max_optional_size(measure.extra_location_min_height, preferred_height)
        );
    } else {
        let preferred = (preferred_break_count, preferred_height);
        let measured = (measure.break_count, measure.size.height);
        let drawn = (draw.break_count, draw.size.height);

        type Thing = (u32, Option<f32>);

        fn max(a: Thing, b: Thing) -> Thing {
            // Beware of wild NaNs, they bite!
            if a > b { a } else { b }
        }

        assert!(drawn >= measured);
        assert!(drawn <= max(preferred, measured));
    }

    let restricted_draw = draw_element(
        &element,
        width,
        first_height,
        measure.size.height,
        first_pos,
        page_size,
        breakable.as_ref().map(|b| BreakableDrawConfig {
            pos,
            full_height: b.full_height,
            preferred_height_break_count: measure.break_count,
        }),
    );

    assert_eq!(measure.break_count, restricted_draw.break_count);
    assert_eq!(measure.size, restricted_draw.size);

    ElementTestOutput {
        size: draw.size,
        breakable: breakable.map(|breakable| {
            let full_height = breakable.full_height;
            let first_location_usage = element.first_location_usage(FirstLocationUsageCtx {
                width,
                first_height,
                full_height,
            });

            match first_location_usage {
                FirstLocationUsage::NoneHeight => {
                    assert!(measure.size.height.is_none());
                    assert_eq!(measure.break_count, 0);
                }
                FirstLocationUsage::WillUse => {
                    assert!(measure.size.height.is_some() || measure.break_count >= 1);
                }
                FirstLocationUsage::WillSkip => {
                    assert!(measure.break_count >= 1);

                    let skipped_measure =
                        measure_element(&element, width, full_height, Some(full_height));

                    // TODO: insert draw here

                    assert_eq!(skipped_measure.break_count + 1, measure.break_count);
                    assert_ne!(first_height, full_height);
                }
            }

            ElementTestOutputBreakable {
                break_count: draw.break_count,
                extra_location_min_height: measure.extra_location_min_height,
                first_location_usage,
            }
        }),
    }
}

pub struct DrawStats {
    break_count: u32,
    breaks: Vec<u32>,
    size: ElementSize,
}

struct BreakableDrawConfig {
    pos: (f32, f32),
    full_height: f32,
    preferred_height_break_count: u32,
}

fn draw_element<E: Element>(
    element: &E,
    width: WidthConstraint,
    first_height: f32,
    preferred_height: Option<f32>,
    first_pos: (f32, f32),
    page_size: (f32, f32),
    breakable: Option<BreakableDrawConfig>,
) -> DrawStats {
    let mut page_idx = 0;

    let mut pdf = Pdf::new();
    pdf.add_page(page_size);

    let mut breaks = vec![];

    let next_draw_pos = &mut |pdf: &mut Pdf, location_idx, _height| {
        breaks.push(location_idx);

        while page_idx <= location_idx {
            pdf.add_page(page_size);
            page_idx += 1;
        }

        Location {
            page_idx: location_idx as usize + 1,
            layer_idx: 0,
            pos: breakable.as_ref().unwrap().pos,
            scale_factor: 1.,
        }
    };

    let ctx = DrawCtx {
        pdf: &mut pdf,
        width,
        location: Location {
            page_idx: 0,
            layer_idx: 0,
            pos: first_pos,
            scale_factor: 1.,
        },

        first_height,
        preferred_height,

        breakable: breakable.as_ref().map(|b| BreakableDraw {
            full_height: b.full_height,
            preferred_height_break_count: b.preferred_height_break_count,
            do_break: next_draw_pos,
        }),
    };

    let size = element.draw(ctx);

    DrawStats {
        break_count: page_idx,
        breaks,
        size,
    }
}

pub struct MeasureStats {
    break_count: u32,
    extra_location_min_height: Option<f32>,
    size: ElementSize,
}

pub fn measure_element<E: Element>(
    element: &E,
    width: WidthConstraint,
    first_height: f32,
    full_height: Option<f32>,
) -> MeasureStats {
    let mut break_count = 0;
    let mut extra_location_min_height = None;

    let ctx = MeasureCtx {
        width,
        first_height,
        breakable: full_height.map(|full_height| BreakableMeasure {
            full_height,
            break_count: &mut break_count,
            extra_location_min_height: &mut extra_location_min_height,
        }),
    };

    let size = element.measure(ctx);

    MeasureStats {
        break_count,
        extra_location_min_height,
        size,
    }
}
