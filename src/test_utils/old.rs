use super::*;

pub fn test_measure_draw_compatibility<E: Element>(
    element: &E,
    width: WidthConstraint,
    first_height: f32,
    full_height: Option<f32>,
    pos: (f32, f32),
    page_size: (f32, f32),
) -> ElementMeasureDrawCompatibilityOutput {
    let measure = measure_element(element, width, first_height, full_height);
    let draw = draw_element(
        element,
        width,
        first_height,
        None,
        pos,
        page_size,
        full_height.map(|f| BreakableDrawConfig {
            pos,
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
            pos,
            full_height: f,
            preferred_height_break_count: measure.break_count,
        }),
    );

    assert_eq!(measure.break_count, draw.break_count);
    assert_eq!(measure.break_count, restricted_draw.break_count);

    assert_eq!(measure.size, draw.size);
    assert_eq!(measure.size, restricted_draw.size);

    ElementMeasureDrawCompatibilityOutput {
        width,
        first_height,
        pos,
        page_size,
        size: measure.size,
        breakable: full_height.map(|f| ElementMeasureDrawCompatibilityOutputBreakable {
            first_location_usage: FirstLocationUsage::NoneHeight, // TODO
            full_height: f,
            break_count: measure.break_count,
            extra_location_min_height: measure.extra_location_min_height,
        }),
    }
}

pub struct ElementTestParams {
    /// Will be tested with None and this.
    pub width: f32,

    pub first_height: f32,
    pub full_height: f32,

    pub pos: (f32, f32),
    pub page_size: (f32, f32),
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
    pub fn run(&self, element: &impl Element) -> ElementMeasureDrawCompatibilityOutput {
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
    ) -> impl Iterator<Item = ElementMeasureDrawCompatibilityOutput> + 'a {
        self.configurations().map(|c| c.run(element))
    }
}

pub struct ElementMeasureDrawCompatibilityOutputBreakable {
    pub full_height: f32,

    pub break_count: u32,
    pub extra_location_min_height: Option<f32>,

    pub first_location_usage: FirstLocationUsage,
}

impl ElementMeasureDrawCompatibilityOutputBreakable {
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

pub struct ElementMeasureDrawCompatibilityOutput {
    pub width: WidthConstraint,
    pub first_height: f32,

    pub pos: (f32, f32),
    pub page_size: (f32, f32),

    pub size: ElementSize,

    pub breakable: Option<ElementMeasureDrawCompatibilityOutputBreakable>,
}

impl ElementMeasureDrawCompatibilityOutput {
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
