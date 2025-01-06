// use pdf_writer::Name;

pub mod builtin;
pub mod truetype;

use pdf_writer::Content;

// pub struct HMetrics {
//     pub advance_width: f32,
// }

pub struct GeneralMetrics {
    pub ascent: f32,
    pub line_height: f32,
}

pub trait Font {
    fn break_text_into_lines<'a>(
        &self,
        text: &'a str,
        max_width: f32,
        size: f32,
        character_spacing: f32,
        word_spacing: f32,
    ) -> impl Iterator<Item = &'a str> + Clone;

    fn general_metrics(&self, size: f32) -> GeneralMetrics;

    fn line_width(&self, line: &str, size: f32, character_spacing: f32, word_spacing: f32) -> f32;

    fn render_line(
        &self,
        layer: &mut Content,
        line: &str,
        size: f32,
        character_spacing: f32,
        word_spacing: f32,
        underline: bool,
        x: f32,
        y: f32,
    );

    // fn resource_name<'a>(&'a self) -> Name<'a>;

    // fn codepoint_h_metrics(&self, codepoint: u32) -> HMetrics;

    // fn units_per_em(&self) -> u16;

    // fn general_metrics(&self) -> GeneralMetrics;
}
