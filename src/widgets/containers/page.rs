use printpdf::indices::{PdfLayerIndex, PdfPageIndex};

use printpdf::*;

use crate::widgets::*;

pub struct RepeatingContentHandler<'a> {
    pdf: &'a mut Pdf,
    layer: &'a PdfLayerReference,
}

impl<'a> RepeatingContentHandler<'a> {
    pub fn el<W: Element>(&mut self, widget: W, pos: [f64; 2], width: Option<f64>, height: f64) {
        widget.element(
            width,
            Some(DrawContext {
                pdf: self.pdf,
                draw_pos: DrawPos {
                    layer: self.layer.clone(),
                    pos,
                    preferred_height: None,
                    height_available: height,
                },
                full_height: 0.0,
                next_draw_pos: None,
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
    fn element(&self, _width: Option<f64>, draw: Option<DrawContext>) -> [f64; 2] {
        if let Some(context) = draw {
            let pdf = context.pdf;

            let content_layer = pdf.next_layer(&context.draw_pos);

            // let draw_pos = &mut context.draw_pos;

            if let Some(next_draw_pos) = context.next_draw_pos {
                let first_page: usize = context.draw_pos.layer.page.0;
                let mut last_page: u32 = first_page as u32;

                self.primary.element(
                    Some(self.primary_width),
                    Some(DrawContext {
                        pdf,
                        draw_pos: DrawPos {
                            layer: content_layer,
                            pos: self.primary_pos,
                            preferred_height: None,
                            height_available: self.primary_height,
                        },
                        full_height: self.primary_height,
                        next_draw_pos: Some(&mut |pdf, draw_rect_id, _| {
                            let mut new_draw_pos = next_draw_pos(pdf, draw_rect_id, self.size);
                            // *draw_pos = new_draw_pos.clone();

                            // (self.repeating_content)(&mut RepeatingContentHandler {
                            //     pdf,
                            //     layer: &new_draw_pos.layer
                            // }, 0, 1);

                            // last_page += 1;
                            last_page = last_page.max(first_page as u32 + draw_rect_id + 1);

                            new_draw_pos.pos = self.primary_pos;
                            new_draw_pos.height_available = self.primary_height;
                            new_draw_pos.layer = pdf.next_layer(&new_draw_pos);

                            new_draw_pos
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
                self.primary.element(
                    Some(self.primary_width),
                    Some(DrawContext {
                        pdf,
                        draw_pos: DrawPos {
                            layer: content_layer,
                            pos: self.primary_pos,
                            preferred_height: None,
                            height_available: self.primary_height,
                        },
                        full_height: self.primary_height,
                        next_draw_pos: None,
                    }),
                );

                (self.repeating_content)(
                    &mut RepeatingContentHandler {
                        pdf,
                        layer: &context.draw_pos.layer,
                    },
                    0,
                    1,
                );
            }
        }

        self.size
    }
}
