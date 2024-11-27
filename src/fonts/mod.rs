// use pdf_writer::Name;

// pub mod builtin;
pub mod truetype;

use pdf_writer::Pdf;

// pub struct HMetrics {
//     pub advance_width: f64,
// }

// pub struct GeneralMetrics {
//     pub ascent: f64,
//     pub line_height: f64,
// }

pub trait Font {
    fn break_text_into_lines<'a>(
        &self,
        text: &'a str,
        max_width: f64,
    ) -> impl Iterator<Item = &'a str>;

    fn line_height(&self) -> f64;

    fn line_width(&self, line: &str) -> f64;

    fn render_line(&self, pdf: &mut Pdf, line: &str) -> f64;

    // fn resource_name<'a>(&'a self) -> Name<'a>;

    // fn codepoint_h_metrics(&self, codepoint: u32) -> HMetrics;

    // fn units_per_em(&self) -> u16;

    // fn general_metrics(&self) -> GeneralMetrics;
}
