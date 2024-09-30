use std::ops::Deref;

use printpdf::{IndirectFontRef, PdfDocumentReference};
use stb_truetype::FontInfo;

use super::Font;

#[derive(Debug)]
pub struct TruetypeFont<D: Deref<Target = [u8]>> {
    pub font_ref: IndirectFontRef,
    pub font: FontInfo<D>,
}

impl<D: AsRef<[u8]> + Deref<Target = [u8]>> TruetypeFont<D> {
    pub fn new(doc: &PdfDocumentReference, bytes: D) -> Self {
        let font_reader = std::io::Cursor::new(&bytes);
        let pdf_font = doc.add_external_font(font_reader).unwrap();
        let font_info = FontInfo::new(bytes, 0).unwrap();

        TruetypeFont {
            font_ref: pdf_font,
            font: font_info,
        }
    }
}

impl<D: Deref<Target = [u8]>> Font for TruetypeFont<D> {
    fn indirect_font_ref(&self) -> &printpdf::IndirectFontRef {
        &self.font_ref
    }

    fn codepoint_h_metrics(&self, codepoint: u32) -> super::HMetrics {
        let h_metrics = self.font.get_codepoint_h_metrics(codepoint);

        super::HMetrics {
            advance_width: h_metrics.advance_width as f64,
        }
    }

    fn units_per_em(&self) -> u16 {
        self.font.units_per_em()
    }

    fn general_metrics(&self) -> super::GeneralMetrics {
        let v_metrics = self.font.get_v_metrics();

        super::GeneralMetrics {
            ascent: v_metrics.ascent as f64,

            // It seems that descent is positive in some fonts and negative in others.
            line_height: (v_metrics.ascent + v_metrics.descent.abs() + v_metrics.line_gap) as f64,
        }
    }
}
