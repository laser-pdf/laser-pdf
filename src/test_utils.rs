use printpdf::indices::{PdfLayerIndex, PdfPageIndex};

use crate::*;

pub struct ElementStats {
    pages: u32,
    breaks: Vec<u32>,
    document: PdfDocumentReference,
}

impl ElementStats {
    pub fn assert_pages(&self, pages: u32) -> &Self {
        assert_eq!(self.pages, pages);
        self
    }

    pub fn assert_linear(&self) -> &Self {
        self.assert_breaks((1..(self.breaks.len() as u32 + 1)).collect::<Vec<_>>())
    }

    pub fn assert_breaks(&self, breaks: impl IntoIterator<Item = u32>) -> &Self {
        assert!(breaks.into_iter().eq(self.breaks.iter().copied()));
        self
    }
}

pub fn run_element<E: Element>(width: f64, height: f64, element: E) -> ElementStats {
    let (doc, page, layer) = PdfDocument::new("test", Mm(height), Mm(height), "Layer 0");
    let mut page_idx = 0;

    let mut pdf = Pdf {
        document: doc,
        page_size: (width, height),
    };

    let mut breaks = vec![];

    let next_draw_pos = &mut |pdf: &mut Pdf, draw_rect| {
        breaks.push(draw_rect);

        while page_idx <= draw_rect {
            pdf.document.add_page(Mm(width), Mm(height), "Layer 0");
            page_idx += 1;
        }

        let layer = pdf
            .document
            .get_page(PdfPageIndex((draw_rect + 1) as usize))
            .get_layer(PdfLayerIndex(0));

        Location {
            layer,
            pos: (0.0, 297.0),
        }
    };

    let layer = pdf.document.get_page(page).get_layer(layer);

    let ctx = DrawCtx {
        pdf: &mut pdf,
        width: Some(width),
        location: Location {
            layer,
            pos: (0., height),
        },

        first_height: height,

        breakable: Some(BreakableDraw {
            full_height: height,
            max_breaks: None,
            get_location: next_draw_pos,
        }),
    };

    element.draw(ctx);

    ElementStats {
        pages: page_idx + 1,
        breaks,
        document: pdf.document,
    }
}
