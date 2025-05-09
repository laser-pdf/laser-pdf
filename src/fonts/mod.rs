// use pdf_writer::Name;

pub mod builtin;
pub mod truetype;

use std::ops::Range;

use pdf_writer::writers::ShowPositioned;

use crate::{Location, Pdf};

// use pdf_writer::Content;

// pub struct HMetrics {
//     pub advance_width: f32,
// }

pub struct GeneralMetrics {
    pub ascent: u32,
    pub line_height: u32,
}

#[derive(Debug, Clone)]
pub struct ShapedGlyph {
    pub unsafe_to_break: bool,
    pub glyph_id: u32,
    pub text_range: Range<usize>,
    /// without kerning
    pub x_advance_font: i32,
    pub x_advance: i32,
    pub x_offset: i32,
    pub y_advance: i32,
    pub y_offset: i32,
}

// TODO: different representation?
pub enum EncodedGlyph {
    OneByte(u8),
    TwoBytes([u8; 2]),
}

pub trait Font {
    type Shaped<'a>: Iterator<Item = ShapedGlyph> + Clone + 'a
    where
        Self: 'a;

    fn shape<'a>(
        &'a self,
        text: &'a str,
        character_spacing: i32,
        word_spacing: i32,
    ) -> Self::Shaped<'a>;

    // fn remap(&self, pdf: &mut crate::Pdf, glyph_id: u16) -> u16;

    fn encode(&self, pdf: &mut crate::Pdf, glyph_id: u32, text: &str) -> EncodedGlyph;

    fn resource_name(&self) -> pdf_writer::Name;

    fn general_metrics(&self) -> GeneralMetrics;

    fn units_per_em(&self) -> u16;

    // fn resource_name<'a>(&'a self) -> Name<'a>;

    // fn codepoint_h_metrics(&self, codepoint: u32) -> HMetrics;

    // fn general_metrics(&self) -> GeneralMetrics;
}
