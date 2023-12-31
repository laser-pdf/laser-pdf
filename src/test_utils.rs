use printpdf::{
    indices::{PdfLayerIndex, PdfPageIndex},
    PdfDocument,
};

use crate::*;

pub struct DrawStats {
    break_count: u32,
    breaks: Vec<u32>,
    size: Option<ElementSize>,
}

impl DrawStats {
    pub fn assert_pages(&self, pages: u32) -> &Self {
        assert_eq!(self.break_count + 1, pages);
        self
    }

    pub fn assert_linear(&self) -> &Self {
        self.assert_breaks((1..(self.breaks.len() as u32 + 1)).collect::<Vec<_>>())
    }

    pub fn assert_breaks(&self, breaks: impl IntoIterator<Item = u32>) -> &Self {
        assert!(breaks.into_iter().eq(self.breaks.iter().copied()));
        self
    }

    pub fn assert_size(&self, size: Option<ElementSize>) -> &Self {
        assert_eq!(self.size, size);
        self
    }
}

struct BreakableDrawConfig {
    full_height: f64,
    preferred_height_break_count: u32,
}

fn draw_element<E: Element>(
    element: &E,
    width: WidthConstraint,
    first_height: f64,
    preferred_height: f64,
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
    size: Option<ElementSize>,
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

fn test_measure_draw_compatibility<E: Element>(
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
        0.,
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
        measure.size.map_or(0., |s| s.height.unwrap_or(0.)),
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

impl ElementTestParams {
    pub fn run<'a, E: Element>(
        self,
        element: &'a E,
    ) -> impl Iterator<Item = ElementTestOutput> + 'a {
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
        .map(move |(use_first_height, breakable, expand_width)| {
            let width = WidthConstraint {
                max: self.width,
                expand: expand_width,
            };

            let first_height = if use_first_height {
                self.first_height
            } else {
                self.full_height
            };

            let full_height = if breakable {
                Some(self.full_height)
            } else {
                None
            };

            test_measure_draw_compatibility(
                element,
                width,
                first_height,
                full_height,
                self.pos,
                self.page_size,
            )
        })
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

    pub size: Option<ElementSize>,

    pub breakable: Option<ElementTestOutputBreakable>,
}

impl ElementTestOutput {
    pub fn assert_size(&self, size: Option<ElementSize>) -> &Self {
        assert_eq!(self.size, size);
        self
    }
}

#[non_exhaustive]
pub struct ElementProxy<'a, E: Element> {
    pub element: E,
    pub before_draw: &'a dyn Fn(&mut DrawCtx),
    pub after_break: &'a dyn Fn(u32, &Location, WidthConstraint, f64),
}

impl<'a, E: Element> ElementProxy<'a, E> {
    pub fn new(element: E) -> Self {
        ElementProxy {
            element,
            before_draw: &|_| {},
            after_break: &|_, _, _, _| {},
        }
    }
}

impl<'a, E: Element> Element for ElementProxy<'a, E> {
    fn insufficient_first_height(&self, ctx: InsufficientFirstHeightCtx) -> bool {
        self.element.insufficient_first_height(ctx)
    }

    fn measure(&self, ctx: MeasureCtx) -> Option<ElementSize> {
        self.element.measure(ctx)
    }

    fn draw(&self, mut ctx: DrawCtx) -> Option<ElementSize> {
        (self.before_draw)(&mut ctx);

        if let Some(breakable) = ctx.breakable {
            self.element.draw(DrawCtx {
                breakable: Some(BreakableDraw {
                    get_location: &mut |pdf, location_idx| {
                        let location = (breakable.get_location)(pdf, location_idx);

                        (self.after_break)(location_idx, &location, ctx.width, ctx.first_height);

                        location
                    },
                    ..breakable
                }),
                ..ctx
            })
        } else {
            self.element.draw(ctx)
        }
    }
}

pub struct BuildElementReturnToken(());

// Is just here to ensure the callback can't be used more than once.
pub struct BuildElementCallback<'a>(&'a mut dyn FnMut(&dyn Element));

impl<'a> BuildElementCallback<'a> {
    pub fn call(self, element: impl Element) -> BuildElementReturnToken {
        self.0(&element);
        BuildElementReturnToken(())
    }
}

pub struct BuildElementCtx {
    pub width: WidthConstraint,
    pub first_height: f64,
    pub full_height: Option<f64>,
}

pub struct BuildElement<F: Fn(BuildElementCtx, BuildElementCallback) -> BuildElementReturnToken>(
    pub F,
);

impl<F: Fn(BuildElementCtx, BuildElementCallback) -> BuildElementReturnToken> Element
    for BuildElement<F>
{
    fn insufficient_first_height(&self, ctx: InsufficientFirstHeightCtx) -> bool {
        let mut ret = false;

        let build_ctx = BuildElementCtx {
            width: ctx.width,
            first_height: ctx.first_height,
            full_height: Some(ctx.full_height),
        };

        let mut ctx = Some(ctx);

        (self.0)(
            build_ctx,
            BuildElementCallback(&mut |e| ret = e.insufficient_first_height(ctx.take().unwrap())),
        );
        ret
    }

    fn measure(&self, ctx: MeasureCtx) -> Option<ElementSize> {
        let mut ret = None;

        let build_ctx = BuildElementCtx {
            width: ctx.width,
            first_height: ctx.first_height,
            full_height: ctx.breakable.as_ref().map(|b| b.full_height),
        };

        let mut ctx = Some(ctx);

        (self.0)(
            build_ctx,
            BuildElementCallback(&mut |e| ret = e.measure(ctx.take().unwrap())),
        );
        ret
    }

    fn draw(&self, ctx: DrawCtx) -> Option<ElementSize> {
        let mut ret = None;

        let build_ctx = BuildElementCtx {
            width: ctx.width,
            first_height: ctx.first_height,
            full_height: ctx.breakable.as_ref().map(|b| b.full_height),
        };

        let mut ctx = Some(ctx);

        (self.0)(
            build_ctx,
            BuildElementCallback(&mut |e| ret = e.draw(ctx.take().unwrap())),
        );
        ret
    }
}

/// A predictable element for testing containers. It's a bit simpler than actual text in that it
/// doesn't vary it's height based on input width. It just either returns the width from the
/// constraint or [Self::width] if unconstrained.
pub struct FakeText {
    pub lines: u32,
    pub line_height: f64,
    pub width: f64,
}

impl FakeText {
    fn lines_and_breaks(&self, first_height: f64, full_height: f64) -> (u32, u32) {
        let first_lines = (first_height / self.line_height).floor() as u32;

        if self.lines <= first_lines {
            (self.lines, 0)
        } else {
            let remaining_lines = self.lines - first_lines;
            let lines_per_page = (full_height / self.line_height).floor() as u32;
            let full_pages = remaining_lines / lines_per_page;
            let last_page_lines = remaining_lines % lines_per_page;

            (
                if last_page_lines == 0 {
                    lines_per_page
                } else {
                    last_page_lines
                },
                full_pages + if last_page_lines == 0 { 0 } else { 1 },
            )
        }
    }
}

impl Element for FakeText {
    fn insufficient_first_height(&self, ctx: InsufficientFirstHeightCtx) -> bool {
        ctx.first_height < self.line_height
    }

    fn measure(&self, ctx: MeasureCtx) -> Option<ElementSize> {
        let lines = if let Some(breakable) = ctx.breakable {
            let (lines, breaks) = self.lines_and_breaks(ctx.first_height, breakable.full_height);

            *breakable.break_count = breaks;
            lines
        } else {
            self.lines
        };

        Some(ElementSize {
            width: ctx.width.constrain(self.width),
            height: Some(lines as f64 * self.line_height),
        })
    }

    fn draw(&self, ctx: DrawCtx) -> Option<ElementSize> {
        let lines = if let Some(breakable) = ctx.breakable {
            let (lines, breaks) = self.lines_and_breaks(ctx.first_height, breakable.full_height);

            for i in 0..breaks {
                (breakable.get_location)(ctx.pdf, i);
            }

            lines
        } else {
            self.lines
        };

        Some(ElementSize {
            width: ctx.width.constrain(self.width),
            height: Some(lines as f64 * self.line_height),
        })
    }
}

pub struct FakeImage {
    pub width: f64,
    pub height: f64,
}

impl FakeImage {
    fn size(&self, width: WidthConstraint) -> (f64, ElementSize) {
        let width = width.constrain(self.width);

        let scale = width / self.width;
        let size = (width, self.height * scale);

        (
            size.1,
            ElementSize {
                width: size.0,
                height: Some(size.1),
            },
        )
    }
}

impl Element for FakeImage {
    fn insufficient_first_height(&self, ctx: InsufficientFirstHeightCtx) -> bool {
        ctx.break_appropriate_for_min_height(self.size(ctx.width).0)
    }

    fn measure(&self, mut ctx: MeasureCtx) -> Option<ElementSize> {
        let (height, size) = self.size(ctx.width);
        ctx.break_if_appropriate_for_min_height(height);
        Some(size)
    }

    fn draw(&self, mut ctx: DrawCtx) -> Option<ElementSize> {
        let (height, size) = self.size(ctx.width);
        ctx.break_if_appropriate_for_min_height(height);
        Some(size)
    }
}

// someone's gotta test the tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fake_text() {
        let element = FakeText {
            line_height: 1.,
            lines: 11,
            width: 5.,
        };

        for mut output in (ElementTestParams {
            first_height: 1.999,
            full_height: 3.3,
            ..Default::default()
        })
        .run(&element)
        {
            if let Some(ref mut b) = output.breakable {
                b.assert_break_count(if output.first_height == 1.999 { 4 } else { 3 });
            }

            output.assert_size(Some(ElementSize {
                width: if output.width.expand {
                    output.width.max
                } else {
                    5.
                },

                height: Some(if output.breakable.is_some() {
                    if output.first_height == 1.999 {
                        1.
                    } else {
                        2.
                    }
                } else {
                    11.
                }),
            }));
        }
    }
}
