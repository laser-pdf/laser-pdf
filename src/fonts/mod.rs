// use pdf_writer::Name;

pub mod builtin;
pub mod truetype;

use pdf_writer::Content;

// pub struct HMetrics {
//     pub advance_width: f64,
// }

pub struct GeneralMetrics {
    pub ascent: f64,
    pub line_height: f64,
}

pub trait Font {
    fn break_text_into_lines<'a>(
        &self,
        text: &'a str,
        max_width: f64,
        size: f64,
        character_spacing: f64,
        word_spacing: f64,
    ) -> impl Iterator<Item = &'a str> + Clone;

    fn general_metrics(&self, size: f64) -> GeneralMetrics;

    fn line_width(&self, line: &str, size: f64, character_spacing: f64, word_spacing: f64) -> f64;

    fn render_line(
        &self,
        layer: &mut Content,
        line: &str,
        size: f64,
        character_spacing: f64,
        word_spacing: f64,
        underline: bool,
        x: f32,
        y: f32,
    );

    // fn resource_name<'a>(&'a self) -> Name<'a>;

    // fn codepoint_h_metrics(&self, codepoint: u32) -> HMetrics;

    // fn units_per_em(&self) -> u16;

    // fn general_metrics(&self) -> GeneralMetrics;
}
