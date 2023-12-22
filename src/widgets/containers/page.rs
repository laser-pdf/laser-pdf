use printpdf::indices::{PdfLayerIndex, PdfPageIndex};

use printpdf::*;

use crate::widgets::*;

pub struct RepeatingContentHandler<'a> {
    pdf: &'a mut Pdf,
    layer: &'a PdfLayerReference,
}

impl<'a> RepeatingContentHandler<'a> {
    pub fn el<W: Element>(&mut self, widget: W, pos: [f64; 2], width: Option<f64>, height: f64) {
        widget.draw(
            width,
            Some(DrawCtx {
                pdf: self.pdf,
                location: Location {
                    layer: self.layer.clone(),
                    pos,
                    preferred_height: None,
                    height_available: height,
                },
                full_height: 0.0,
                next_location: None,
            }),
        );
    }
}

/// This widgets assumes that it gets the whole page.
pub struct Page<W: Element, F: Fn(&mut RepeatingContentHandler, usize, usize)> {
    pub primary: W,
    pub primary_pos: [f64; 2],
    pub primary_width: f64,
    pub primary_height: f64,
    pub repeating_content: F,
    pub size: [f64; 2],
}

impl<W: Element, F: Fn(&mut RepeatingContentHandler, usize, usize)> Element for Page<W, F> {
    fn draw(&self, _width: Option<f64>, draw: Option<DrawCtx>) -> [f64; 2] {
        if let Some(context) = draw {
            let pdf = context.pdf;

            let content_layer = pdf.next_layer(&context.location);

            // let location = &mut context.location;

            if let Some(next_location) = context.next_location {
                let first_page: usize = context.location.layer.page.0;
                let mut last_page: u32 = first_page as u32;

                self.primary.draw(
                    Some(self.primary_width),
                    Some(DrawCtx {
                        pdf,
                        location: Location {
                            layer: content_layer,
                            pos: self.primary_pos,
                            preferred_height: None,
                            height_available: self.primary_height,
                        },
                        full_height: self.primary_height,
                        breakable: Some(BreakableDraw {
                            get_location: &mut |pdf, draw_rect_id, _| {
                                let mut new_location = next_location(pdf, draw_rect_id, self.size);
                                // *location = new_location.clone();

                                // (self.repeating_content)(&mut RepeatingContentHandler {
                                //     pdf,
                                //     layer: &new_location.layer
                                // }, 0, 1);

                                // last_page += 1;
                                last_page = last_page.max(first_page as u32 + draw_rect_id + 1);

                                new_location.pos = self.primary_pos;
                                new_location.height_available = self.primary_height;
                                new_location.layer = pdf.next_layer(&new_location);

                                new_location
                            },
                            ..break_ctx
                        }),
                    }),
                );

                let last_page = last_page as usize;
                let page_count = last_page - first_page + 1;

                for (page, pdf_page) in (first_page..=last_page).enumerate() {
                    // TODO: Always using layer 0 here seems incorrect.
                    // Repeatable page breaks could be used to get the correct
                    // base layer.
                    let layer = pdf
                        .document
                        .get_page(PdfPageIndex(pdf_page))
                        .get_layer(PdfLayerIndex(0));

                    (self.repeating_content)(
                        &mut RepeatingContentHandler { pdf, layer: &layer },
                        page,
                        page_count,
                    );
                }
            } else {
                self.primary.draw(
                    Some(self.primary_width),
                    Some(DrawCtx {
                        pdf,
                        location: Location {
                            layer: content_layer,
                            pos: self.primary_pos,
                            preferred_height: None,
                            height_available: self.primary_height,
                        },
                        full_height: self.primary_height,
                        next_location: None,
                    }),
                );

                (self.repeating_content)(
                    &mut RepeatingContentHandler {
                        pdf,
                        layer: &context.location.layer,
                    },
                    0,
                    1,
                );
            }
        }

        self.size
    }
}
