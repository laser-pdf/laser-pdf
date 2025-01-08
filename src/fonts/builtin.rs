use std::collections::HashMap;

use afm::{CharMetric, FontMetrics, Ligature};
use pdf_core_14_font_afms::*;
use pdf_writer::{Name, Str};

use super::Font;
use crate::{
    utils::{mm_to_pt, pt_to_mm},
    Pdf,
};

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

impl Font for BuiltinFont {
    fn break_text_into_lines<'a>(
        &self,
        text: &'a str,
        max_width: f32,
        size: f32,
        character_spacing: f32,
        word_spacing: f32,
    ) -> impl Iterator<Item = &'a str> + Clone {
        crate::text::break_text_into_lines(text, mm_to_pt(max_width), move |text| {
            crate::text::text_width(
                text,
                size,
                1000.,
                |c| self.char_metrics_by_codepoint[&c].wx as f32,
                character_spacing,
                word_spacing,
            )
        })
    }

    fn line_width(&self, line: &str, size: f32, character_spacing: f32, word_spacing: f32) -> f32 {
        pt_to_mm(crate::text::text_width(
            line,
            size,
            1000.,
            |c| self.char_metrics_by_codepoint[&c].wx as f32,
            character_spacing,
            word_spacing,
        ))
    }

    fn render_line(
        &self,
        layer: &mut pdf_writer::Content,
        line: &str,
        size: f32,
        character_spacing: f32,
        word_spacing: f32,
        underline: bool,
        x: f32,
        y: f32,
    ) {
        if character_spacing != 0. {
            layer.set_char_spacing(character_spacing as f32);
        }

        layer.set_font(Name(self.resource_name.as_bytes()), size as f32);

        layer.begin_text().next_line(mm_to_pt(x), mm_to_pt(y));

        if word_spacing != 0. {
            let word_spacing = word_spacing * 1000. / size;

            let mut show_positioned = layer.show_positioned();
            let mut items = show_positioned.items();

            for s in line.split_inclusive(" ") {
                items.show(Str(s.as_bytes()));

                if s.ends_with(" ") {
                    items.adjust(word_spacing as f32);
                }
            }
        } else {
            layer.show(Str(line.as_bytes()));
        }

        layer.end_text();

        if underline {
            let width = self.line_width(line, size, character_spacing, word_spacing);
            crate::utils::line(layer, (x, y - 1.0), width as f32, pt_to_mm(2.0) as f32);
        }
    }
    // fn indirect_font_ref(&self) -> &IndirectFontRef {
    //     &self.font_ref
    // }

    // fn codepoint_h_metrics(&self, codepoint: u32) -> super::HMetrics {
    //     let metrics = self.char_metrics_by_codepoint.get(&codepoint).unwrap();

    //     super::HMetrics {
    //         advance_width: metrics.wx,
    //     }
    // }

    // fn units_per_em(&self) -> u16 {
    //     1000
    // }

    // TODO: This API needs some very serious thought.
    fn general_metrics(&self, size: f32) -> super::GeneralMetrics {
        let bbox = self.metrics.font_bbox;

        // This should be bbox.ymax - bbox.ymin, but it seems that the afm is parsed incorrectly.
        let line_height = bbox.ymax - bbox.xmax;

        let ascent = line_height + self.metrics.descender;

        // TODO: pt to mm???
        super::GeneralMetrics {
            ascent: ascent as f32 * size / 1000.,
            line_height: line_height as f32 * size / 1000.,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_panic() {
        let mut pdf = Pdf::new((12., 12.));

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
