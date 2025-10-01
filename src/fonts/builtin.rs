use std::{collections::HashMap, str::CharIndices};

use afm::{CharMetric, FontMetrics, Ligature};
use pdf_core_14_font_afms::*;
use pdf_writer::Name;

use super::{EncodedGlyph, Font, ShapedGlyph};
use crate::Pdf;

pub struct BuiltinFont {
    resource_name: String,
    metrics: FontMetrics,
    char_metrics_by_codepoint: HashMap<u32, CharMetric>,
}

impl BuiltinFont {
    fn add(pdf: &mut Pdf, font_name: &str, afm: &str) -> Self {
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

        let id = pdf.alloc();

        pdf.pdf
            .type1_font(id)
            .base_font(pdf_writer::Name(font_name.as_bytes()));

        let idx = pdf.fonts.len();
        pdf.fonts.push(id);

        let resource_name = format!("F{}", idx);

        BuiltinFont {
            resource_name,
            metrics,
            char_metrics_by_codepoint,
        }
    }

    pub fn courier(pdf: &mut Pdf) -> Self {
        Self::add(pdf, "Courier", COURIER)
    }

    pub fn courier_bold(pdf: &mut Pdf) -> Self {
        Self::add(pdf, "Courier-Bold", COURIER_BOLD)
    }

    pub fn courier_oblique(pdf: &mut Pdf) -> Self {
        Self::add(pdf, "Courier-Oblique", COURIER_OBLIQUE)
    }

    pub fn courier_bold_oblique(pdf: &mut Pdf) -> Self {
        Self::add(pdf, "Courier-BoldOblique", COURIER_BOLD_OBLIQUE)
    }

    pub fn helvetica(pdf: &mut Pdf) -> Self {
        Self::add(pdf, "Helvetica", HELVETICA)
    }

    pub fn helvetica_bold(pdf: &mut Pdf) -> Self {
        Self::add(pdf, "Helvetica-Bold", HELVETICA_BOLD)
    }

    pub fn helvetica_oblique(pdf: &mut Pdf) -> Self {
        Self::add(pdf, "Helvetica-Oblique", HELVETICA_OBLIQUE)
    }

    pub fn helvetica_bold_oblique(pdf: &mut Pdf) -> Self {
        Self::add(pdf, "Helvetica-BoldOblique", HELVETICA_BOLD_OBLIQUE)
    }

    pub fn times_roman(pdf: &mut Pdf) -> Self {
        Self::add(pdf, "Times-Roman", TIMES_ROMAN)
    }

    pub fn times_bold(pdf: &mut Pdf) -> Self {
        Self::add(pdf, "Times-Bold", HELVETICA_BOLD)
    }

    pub fn times_italic(pdf: &mut Pdf) -> Self {
        Self::add(pdf, "Times-Italic", TIMES_ITALIC)
    }

    pub fn times_bold_italic(pdf: &mut Pdf) -> Self {
        Self::add(pdf, "Times-BoldItalic", TIMES_BOLD_ITALIC)
    }

    pub fn symbol(pdf: &mut Pdf) -> Self {
        Self::add(pdf, "Symbol", SYMBOL)
    }

    pub fn zapf_dingbats(pdf: &mut Pdf) -> Self {
        Self::add(pdf, "ZapfDingbats", ZAPF_DINGBATS)
    }
}

#[derive(Clone)]
pub struct Shaped<'a> {
    font: &'a BuiltinFont,
    chars: CharIndices<'a>,
}

impl<'a> Iterator for Shaped<'a> {
    type Item = ShapedGlyph;

    fn next(&mut self) -> Option<Self::Item> {
        self.chars.next().map(|(i, c)| {
            let metrics = self.font.char_metrics_by_codepoint.get(&(c as u32));

            let advance = metrics.map_or(0., |m| m.wx as f32 / 1000.);

            ShapedGlyph {
                unsafe_to_break: false,
                glyph_id: c as u32, // i guess
                text_range: i..i + c.len_utf8(),
                x_advance_font: advance,
                x_advance: advance,
                x_offset: 0.,
                y_offset: 0.,
                y_advance: 0.,
            }
        })
    }
}

impl Font for BuiltinFont {
    type Shaped<'a>
        = Shaped<'a>
    where
        Self: 'a;

    fn shape<'a>(&'a self, text: &'a str, _: f32, _: f32) -> Self::Shaped<'a> {
        Shaped {
            font: self,
            chars: text.char_indices(),
        }
    }

    fn encode(&self, _: &mut crate::Pdf, glyph_id: u32, _: &str) -> EncodedGlyph {
        EncodedGlyph::OneByte(glyph_id as u8)
    }

    fn resource_name(&self) -> pdf_writer::Name<'_> {
        Name(self.resource_name.as_bytes())
    }

    fn general_metrics(&self) -> super::GeneralMetrics {
        let bbox = self.metrics.font_bbox;

        // This should be bbox.ymax - bbox.ymin, but it seems that the afm is parsed incorrectly.
        let line_height = bbox.ymax - bbox.xmax;

        let ascent = line_height + self.metrics.descender;

        super::GeneralMetrics {
            height_above_baseline: (ascent / 1000.) as f32,
            height_below_baseline: (-self.metrics.descender / 1000.) as f32,
        }
    }

    fn fallback_fonts(&self) -> &[Self] {
        &[]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_panic() {
        let mut pdf = Pdf::new();

        BuiltinFont::courier(&mut pdf);
        BuiltinFont::courier_bold(&mut pdf);
        BuiltinFont::courier_oblique(&mut pdf);
        BuiltinFont::courier_bold_oblique(&mut pdf);

        BuiltinFont::helvetica(&mut pdf);
        BuiltinFont::helvetica_bold(&mut pdf);
        BuiltinFont::helvetica_oblique(&mut pdf);
        BuiltinFont::helvetica_bold_oblique(&mut pdf);

        BuiltinFont::times_roman(&mut pdf);
        BuiltinFont::times_bold(&mut pdf);
        BuiltinFont::times_italic(&mut pdf);
        BuiltinFont::times_bold_italic(&mut pdf);

        BuiltinFont::symbol(&mut pdf);
        BuiltinFont::zapf_dingbats(&mut pdf);
    }
}
