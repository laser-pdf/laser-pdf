pub mod builtin;
pub mod truetype;

use std::ops::Range;

pub struct GeneralMetrics {
    pub height_above_baseline: f32,
    pub height_below_baseline: f32,
}

#[derive(Debug, Clone)]
pub struct ShapedGlyph {
    pub unsafe_to_break: bool,
    /// Zero is reserved for glyphs not found in the font.
    pub glyph_id: u32,
    pub text_range: Range<usize>,
    /// without kerning
    pub x_advance_font: f32,
    pub x_advance: f32,
    pub x_offset: f32,
    pub y_advance: f32,
    pub y_offset: f32,
}

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
        character_spacing: f32,
        word_spacing: f32,
    ) -> Self::Shaped<'a>;

    fn encode(&self, pdf: &mut crate::Pdf, glyph_id: u32, text: &str) -> EncodedGlyph;

    fn resource_name(&self) -> pdf_writer::Name<'_>;

    fn general_metrics(&self) -> GeneralMetrics;

    fn fallback_fonts(&self) -> &[Self]
    where
        Self: Sized;
}
