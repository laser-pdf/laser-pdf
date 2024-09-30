use std::collections::HashMap;

use afm::{CharMetric, FontMetrics, Ligature};
use pdf_core_14_font_afms::*;
use printpdf::{BuiltinFont::*, IndirectFontRef, PdfDocumentReference};

use super::Font;

pub struct BuiltinFont {
    font_ref: IndirectFontRef,
    metrics: FontMetrics,
    char_metrics_by_codepoint: HashMap<u32, CharMetric>,
}

impl BuiltinFont {
    fn add(document: &PdfDocumentReference, font: printpdf::BuiltinFont, afm: &str) -> Self {
        let parser = afm::afm();

        let mut input = pom::DataInput::new(afm.as_bytes());

        let metrics = parser.parse(&mut input).unwrap();

        let mut char_metrics_by_codepoint = HashMap::new();

        for char_metric in &metrics.char_metrics {
            char_metrics_by_codepoint.insert(
                char_metric.character_code as u32,
                // some manual cloning
                CharMetric {
                    name: char_metric.name.clone(),
                    ligatures: char_metric
                        .ligatures
                        .iter()
                        .map(|l| Ligature {
                            ligature: l.ligature.clone(),
                            successor: l.successor.clone(),
                            ..*l
                        })
                        .collect(),
                    ..*char_metric
                },
            );
        }

        BuiltinFont {
            font_ref: document.add_builtin_font(font).unwrap(),
            metrics,
            char_metrics_by_codepoint,
        }
    }

    pub fn courier(document: &PdfDocumentReference) -> Self {
        Self::add(document, Courier, COURIER)
    }

    pub fn courier_bold(document: &PdfDocumentReference) -> Self {
        Self::add(document, CourierBold, COURIER_BOLD)
    }

    pub fn courier_oblique(document: &PdfDocumentReference) -> Self {
        Self::add(document, CourierOblique, COURIER_OBLIQUE)
    }

    pub fn courier_bold_oblique(document: &PdfDocumentReference) -> Self {
        Self::add(document, CourierBoldOblique, COURIER_BOLD_OBLIQUE)
    }

    pub fn helvetica(document: &PdfDocumentReference) -> Self {
        Self::add(document, Helvetica, HELVETICA)
    }

    pub fn helvetica_bold(document: &PdfDocumentReference) -> Self {
        Self::add(document, HelveticaBold, HELVETICA_BOLD)
    }

    pub fn helvetica_oblique(document: &PdfDocumentReference) -> Self {
        Self::add(document, HelveticaOblique, HELVETICA_OBLIQUE)
    }

    pub fn helvetica_bold_oblique(document: &PdfDocumentReference) -> Self {
        Self::add(document, HelveticaBoldOblique, HELVETICA_BOLD_OBLIQUE)
    }

    pub fn times_roman(document: &PdfDocumentReference) -> Self {
        Self::add(document, TimesRoman, TIMES_ROMAN)
    }

    pub fn times_bold(document: &PdfDocumentReference) -> Self {
        Self::add(document, TimesBold, HELVETICA_BOLD)
    }

    pub fn times_italic(document: &PdfDocumentReference) -> Self {
        Self::add(document, TimesItalic, TIMES_ITALIC)
    }

    pub fn times_bold_italic(document: &PdfDocumentReference) -> Self {
        Self::add(document, TimesBoldItalic, TIMES_BOLD_ITALIC)
    }

    pub fn symbol(document: &PdfDocumentReference) -> Self {
        Self::add(document, Symbol, SYMBOL)
    }

    pub fn zapf_dingbats(document: &PdfDocumentReference) -> Self {
        Self::add(document, ZapfDingbats, ZAPF_DINGBATS)
    }
}

impl Font for BuiltinFont {
    fn indirect_font_ref(&self) -> &IndirectFontRef {
        &self.font_ref
    }

    fn codepoint_h_metrics(&self, codepoint: u32) -> super::HMetrics {
        let metrics = self.char_metrics_by_codepoint.get(&codepoint).unwrap();

        super::HMetrics {
            advance_width: metrics.wx,
        }
    }

    fn units_per_em(&self) -> u16 {
        1000
    }

    fn general_metrics(&self) -> super::GeneralMetrics {
        let bbox = self.metrics.font_bbox;

        // This should be bbox.ymax - bbox.ymin, but it seems that the afm is parsed incorrectly.
        let line_height = bbox.ymax - bbox.xmax;

        let ascent = line_height + self.metrics.descender;

        super::GeneralMetrics {
            ascent,
            line_height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use printpdf::PdfDocument;

    #[test]
    fn test_no_panic() {
        let doc = PdfDocument::empty("");

        BuiltinFont::courier(&doc);
        BuiltinFont::courier_bold(&doc);
        BuiltinFont::courier_oblique(&doc);
        BuiltinFont::courier_bold_oblique(&doc);

        BuiltinFont::helvetica(&doc);
        BuiltinFont::helvetica_bold(&doc);
        BuiltinFont::helvetica_oblique(&doc);
        BuiltinFont::helvetica_bold_oblique(&doc);

        BuiltinFont::times_roman(&doc);
        BuiltinFont::times_bold(&doc);
        BuiltinFont::times_italic(&doc);
        BuiltinFont::times_bold_italic(&doc);

        BuiltinFont::symbol(&doc);
        BuiltinFont::zapf_dingbats(&doc);
    }
}
