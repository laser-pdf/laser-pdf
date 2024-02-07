pub mod assert_passes;
pub mod build_element;
pub mod element_proxy;
pub mod fake_image;
pub mod fake_text;
pub mod frantic_jumper;
pub mod record_passes;

pub use build_element::BuildElement;
pub use element_proxy::ElementProxy;
pub use fake_image::FakeImage;
pub use fake_text::FakeText;
pub use frantic_jumper::FranticJumper;

use printpdf::{
    indices::{PdfLayerIndex, PdfPageIndex},
    PdfDocument,
};

use crate::*;

pub struct DrawStats {
    break_count: u32,
    breaks: Vec<u32>,
    size: ElementSize,
}

struct BreakableDrawConfig {
    full_height: f64,
    preferred_height_break_count: u32,
}

fn draw_element<E: Element>(
    element: &E,
    width: WidthConstraint,
    first_height: f64,
    preferred_height: Option<f64>,
    pos: (f64, f64),
    page_size: (f64, f64),
    breakable: Option<BreakableDrawConfig>,
) -> DrawStats {
    let (doc, page, layer) = PdfDocument::new("test", Mm(page_size.0), Mm(page_size.1), "Layer 0");
    let mut page_idx = 0;

    let mut pdf = Pdf {
        document: doc,
        page_size,
    };

    let mut breaks = vec![];

    let next_draw_pos = &mut |pdf: &mut Pdf, location_idx| {
        breaks.push(location_idx);

        while page_idx <= location_idx {
            pdf.document
                .add_page(Mm(page_size.0), Mm(page_size.1), "Layer 0");
            page_idx += 1;
        }

        let layer = pdf
            .document
            .get_page(PdfPageIndex((location_idx + 1) as usize))
            .get_layer(PdfLayerIndex(0));

        Location { layer, pos }
    };

    let layer = pdf.document.get_page(page).get_layer(layer);

    let ctx = DrawCtx {
        pdf: &mut pdf,
        width,
        location: Location { layer, pos },

        first_height,
        preferred_height,

        breakable: breakable.map(|b| BreakableDraw {
            full_height: b.full_height,
            preferred_height_break_count: b.preferred_height_break_count,
            get_location: next_draw_pos,
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
    extra_location_min_height: f64,
    size: ElementSize,
}

pub fn measure_element<E: Element>(
    element: &E,
    width: WidthConstraint,
    first_height: f64,
    full_height: Option<f64>,
) -> MeasureStats {
    let mut break_count = 0;
    let mut extra_location_min_height = 0.;

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

pub fn test_measure_draw_compatibility<E: Element>(
    element: &E,
    width: WidthConstraint,
    first_height: f64,
    full_height: Option<f64>,
    pos: (f64, f64),
    page_size: (f64, f64),
) -> ElementTestOutput {
    let measure = measure_element(element, width, first_height, full_height);
    let draw = draw_element(
        element,
        width,
        first_height,
        None,
        pos,
        page_size,
        full_height.map(|f| BreakableDrawConfig {
            full_height: f,
            preferred_height_break_count: 0,
        }),
    );
    let restricted_draw = draw_element(
        element,
        width,
        first_height,
        measure.size.height,
        pos,
        page_size,
        full_height.map(|f| BreakableDrawConfig {
            full_height: f,
            preferred_height_break_count: measure.break_count,
        }),
    );

    assert_eq!(measure.break_count, draw.break_count);
    assert_eq!(measure.break_count, restricted_draw.break_count);

    assert_eq!(measure.size, draw.size);
    assert_eq!(measure.size, restricted_draw.size);

    ElementTestOutput {
        width,
        first_height,
        pos,
        page_size,
        size: measure.size,
        breakable: full_height.map(|f| ElementTestOutputBreakable {
            full_height: f,
            break_count: measure.break_count,
            extra_location_min_height: measure.extra_location_min_height,
        }),
    }
}

pub struct ElementTestParams {
    /// Will be tested with None and this.
    pub width: f64,

    pub first_height: f64,
    pub full_height: f64,

    pub pos: (f64, f64),
    pub page_size: (f64, f64),
}

impl Default for ElementTestParams {
    fn default() -> Self {
        Self {
            width: 186.,
            first_height: 136.5,
            full_height: 273.,

            pos: (12., 297. - 12.),
            page_size: (210., 297.),
        }
    }
}

pub struct TestConfiguration<'a> {
    pub use_first_height: bool,
    pub breakable: bool,
    pub expand_width: bool,
    pub params: &'a ElementTestParams,
}

impl<'a> TestConfiguration<'a> {
    pub fn run(&self, element: &impl Element) -> ElementTestOutput {
        let width = WidthConstraint {
            max: self.params.width,
            expand: self.expand_width,
        };

        let first_height = if self.use_first_height {
            self.params.first_height
        } else {
            self.params.full_height
        };

        let full_height = if self.breakable {
            Some(self.params.full_height)
        } else {
            None
        };

        test_measure_draw_compatibility(
            element,
            width,
            first_height,
            full_height,
            self.params.pos,
            self.params.page_size,
        )
    }
}

impl ElementTestParams {
    pub fn configurations(&self) -> impl Iterator<Item = TestConfiguration> {
        [
            (false, false, false),
            (false, false, true),
            (false, true, false),
            (false, true, true),
            (true, false, false),
            (true, false, true),
            (true, true, false),
            (true, true, true),
        ]
        .into_iter()
        .map(
            move |(use_first_height, breakable, expand_width)| TestConfiguration {
                use_first_height,
                breakable,
                expand_width,
                params: self,
            },
        )
    }

    pub fn run<'a, E: Element>(
        &'a self,
        element: &'a E,
    ) -> impl Iterator<Item = ElementTestOutput> + 'a {
        self.configurations().map(|c| c.run(element))
    }
}

pub struct ElementTestOutputBreakable {
    pub full_height: f64,

    pub break_count: u32,
    pub extra_location_min_height: f64,
}

impl ElementTestOutputBreakable {
    pub fn assert_break_count(&self, break_count: u32) -> &Self {
        assert_eq!(self.break_count, break_count);
        self
    }

    pub fn assert_extra_location_min_height(&self, extra_location_min_height: f64) -> &Self {
        assert_eq!(self.extra_location_min_height, extra_location_min_height);
        self
    }
}

pub struct ElementTestOutput {
    pub width: WidthConstraint,
    pub first_height: f64,

    pub pos: (f64, f64),
    pub page_size: (f64, f64),

    pub size: ElementSize,

    pub breakable: Option<ElementTestOutputBreakable>,
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
