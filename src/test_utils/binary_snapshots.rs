use std::{fs::File, io::BufWriter};

use printpdf::{
    indices::{PdfLayerIndex, PdfPageIndex},
    OffsetDateTime, PdfDocument,
};

use crate::{utils::max_optional_size, *};

pub const LOREM_IPSUM: &str =
    "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut \
    labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco \
    laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in \
    voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat \
    non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.";

#[derive(Clone, Copy)]
pub struct TestElementParams {
    pub width: WidthConstraint,
    pub first_height: f64,
    pub preferred_height: Option<f64>,
    pub breakable: Option<TestElementParamsBreakable>,
    pub pos: (f64, f64),
    pub page_size: (f64, f64),
}

#[derive(Clone, Copy)]
pub struct TestElementParamsBreakable {
    pub preferred_height_break_count: u32,
    pub full_height: f64,
}

impl TestElementParams {
    pub const DEFAULT_MAX_WIDTH: f64 = 210. - 2. * 8.;
    pub const DEFAULT_FULL_HEIGHT: f64 = 297. - 2. * 16.;
    pub const DEFAULT_REDUCED_HEIGHT: f64 = 100.;

    pub fn unbreakable() -> Self {
        TestElementParams {
            width: WidthConstraint {
                max: Self::DEFAULT_MAX_WIDTH,
                expand: true,
            },
            first_height: Self::DEFAULT_FULL_HEIGHT,
            preferred_height: None,
            breakable: None,
            pos: (8., 297. - 16.),
            page_size: (210., 297.),
        }
    }

    pub fn breakable() -> Self {
        TestElementParams {
            width: WidthConstraint {
                max: Self::DEFAULT_MAX_WIDTH,
                expand: true,
            },
            first_height: Self::DEFAULT_REDUCED_HEIGHT,
            preferred_height: None,
            breakable: Some(TestElementParamsBreakable {
                preferred_height_break_count: 0,
                full_height: Self::DEFAULT_FULL_HEIGHT,
            }),
            pos: (8., 297. - 16.),
            page_size: (210., 297.),
        }
    }
}

#[derive(Debug)]
struct MeasureStats {
    break_count: u32,
    extra_location_min_height: Option<f64>,
    size: ElementSize,
}

#[derive(Debug)]
pub struct DrawStats {
    break_count: u32,
    size: ElementSize,
}

struct Doc {
    params: TestElementParams,
    pdf: Pdf,
}

impl Doc {
    fn new(params: TestElementParams) -> Self {
        let (document, ..) = PdfDocument::new(
            "test",
            Mm(params.page_size.0),
            Mm(params.page_size.1),
            "Layer 0",
        );

        let document = document
            .with_document_id("0000".to_string())
            .with_instance_id("0000".to_string())
            .with_xmp_document_id("0000".to_string())
            .with_xmp_instance_id("0000".to_string())
            .with_creation_date(OffsetDateTime::unix_epoch())
            .with_mod_date(OffsetDateTime::unix_epoch())
            .with_metadata_date(OffsetDateTime::unix_epoch());

        let pdf = Pdf {
            document,
            page_size: params.page_size,
        };

        Doc { params, pdf }
    }

    fn first_location_usage(&mut self, build: impl Fn(Callback)) -> FirstLocationUsage {
        let mut first_location_usage = None;

        let callback = Callback {
            doc: self,
            pass: CallbackPass::FirstLocationUsage {
                out: &mut first_location_usage,
            },
        };

        build(callback);

        first_location_usage.unwrap()
    }

    fn measure(&mut self, build: impl Fn(Callback)) -> MeasureStats {
        let mut stats = None;

        let callback = Callback {
            doc: self,
            pass: CallbackPass::Measure { out: &mut stats },
        };

        build(callback);

        stats.unwrap()
    }

    fn draw(&mut self, build: impl Fn(Callback)) -> DrawStats {
        let mut stats = None;

        let callback = Callback {
            doc: self,
            pass: CallbackPass::Draw { out: &mut stats },
        };

        build(callback);

        stats.unwrap()
    }
}

enum CallbackPass<'a> {
    FirstLocationUsage {
        out: &'a mut Option<FirstLocationUsage>,
    },
    Measure {
        out: &'a mut Option<MeasureStats>,
    },
    Draw {
        out: &'a mut Option<DrawStats>,
    },
}

pub struct Callback<'a> {
    doc: &'a mut Doc,
    pass: CallbackPass<'a>,
}

impl<'a> Callback<'a> {
    pub fn document(&self) -> &PdfDocumentReference {
        &self.doc.pdf.document
    }

    pub fn call(self, element: &impl Element) {
        match self.pass {
            CallbackPass::FirstLocationUsage { out } => {
                let params = &self.doc.params;

                *out = Some(element.first_location_usage(FirstLocationUsageCtx {
                    width: params.width,
                    first_height: params.first_height,
                    full_height: params.breakable.as_ref().unwrap().full_height,
                }));
            }
            CallbackPass::Measure { out } => {
                let mut break_count = 0;
                let mut extra_location_min_height = None;

                let ctx = MeasureCtx {
                    width: self.doc.params.width,
                    first_height: self.doc.params.first_height,
                    breakable: self
                        .doc
                        .params
                        .breakable
                        .as_ref()
                        .map(|b| BreakableMeasure {
                            full_height: b.full_height,
                            break_count: &mut break_count,
                            extra_location_min_height: &mut extra_location_min_height,
                        }),
                };

                let size = element.measure(ctx);

                *out = Some(MeasureStats {
                    break_count,
                    extra_location_min_height,
                    size,
                })
            }
            CallbackPass::Draw { out } => {
                let params = &self.doc.params;
                let pdf = &mut self.doc.pdf;

                let mut page_idx = 0;

                let next_draw_pos = &mut |pdf: &mut Pdf, location_idx, _height| {
                    while page_idx <= location_idx {
                        pdf.document.add_page(
                            Mm(params.page_size.0),
                            Mm(params.page_size.1),
                            "Layer 0",
                        );
                        page_idx += 1;
                    }

                    let layer = pdf
                        .document
                        .get_page(PdfPageIndex((location_idx + 1) as usize))
                        .get_layer(PdfLayerIndex(0));

                    Location {
                        layer,
                        pos: params.pos,
                        scale_factor: 1.,
                    }
                };

                let layer = pdf
                    .document
                    .get_page(PdfPageIndex(0))
                    .get_layer(PdfLayerIndex(0));

                let first_pos = (
                    params.pos.0,
                    params.breakable.as_ref().map_or(params.pos.1, |b| {
                        params.pos.1 - (b.full_height - params.first_height)
                    }),
                );

                let ctx = DrawCtx {
                    pdf,
                    width: params.width,
                    location: Location {
                        layer,
                        pos: first_pos,
                        scale_factor: 1.,
                    },

                    first_height: params.first_height,
                    preferred_height: params.preferred_height,

                    breakable: params.breakable.as_ref().map(|b| BreakableDraw {
                        full_height: b.full_height,
                        preferred_height_break_count: b.preferred_height_break_count,
                        do_break: next_draw_pos,
                    }),
                };

                let size = element.draw(ctx);

                *out = Some(DrawStats {
                    break_count: page_idx,
                    size,
                });
            }
        }
    }
}

pub fn test_element_file(
    params: TestElementParams,
    build_element: impl Fn(Callback),
    file: &mut File,
) {
    let measure = Doc::new(params).measure(&build_element);

    let mut draw_doc = Doc::new(params);

    let draw = draw_doc.draw(&build_element);

    assert_eq!(measure.size.width, draw.size.width);

    let preferred_break_count = params
        .breakable
        .as_ref()
        .map(|b| b.preferred_height_break_count)
        .unwrap_or(0);

    if measure.extra_location_min_height.is_some() && preferred_break_count > measure.break_count {
        assert_eq!(draw.break_count, preferred_break_count);
        assert!(draw.size.height >= measure.extra_location_min_height);
        assert!(
            draw.size.height
                <= max_optional_size(measure.extra_location_min_height, params.preferred_height)
        );
    } else {
        let preferred = (preferred_break_count, params.preferred_height);
        let measured = (measure.break_count, measure.size.height);
        let drawn = (draw.break_count, draw.size.height);

        type Thing = (u32, Option<f64>);

        fn max(a: Thing, b: Thing) -> Thing {
            // Beware of wild NaNs, they bite!
            if a > b {
                a
            } else {
                b
            }
        }

        assert!(drawn >= measured);
        assert!(drawn <= max(preferred, measured));
    }

    let restricted_draw = Doc::new(TestElementParams {
        preferred_height: measure.size.height,
        breakable: params
            .breakable
            .as_ref()
            .map(|b| TestElementParamsBreakable {
                preferred_height_break_count: measure.break_count,
                ..*b
            }),
        ..params
    })
    .draw(&build_element);

    assert_eq!(measure.break_count, restricted_draw.break_count);
    assert_eq!(measure.size, restricted_draw.size);

    if let Some(breakable) = params.breakable {
        let full_height = breakable.full_height;
        let first_location_usage = Doc::new(params).first_location_usage(&build_element);

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

                let skipped_measure = Doc::new(TestElementParams {
                    first_height: full_height,
                    ..params
                })
                .measure(&build_element);

                // TODO: insert draw here

                assert_eq!(skipped_measure.break_count + 1, measure.break_count);
                assert_ne!(params.first_height, full_height);
            }
        }
    }

    draw_doc
        .pdf
        .document
        .save(&mut BufWriter::new(file))
        .unwrap();
}
