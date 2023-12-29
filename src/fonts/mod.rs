use printpdf::IndirectFontRef;

pub mod builtin;
pub mod truetype;

pub struct HMetrics {
    pub advance_width: f64,
}

pub struct GeneralMetrics {
    pub ascent: f64,
    pub line_height: f64,
}

pub trait Font {
    fn indirect_font_ref(&self) -> &IndirectFontRef;

    fn codepoint_h_metrics(&self, codepoint: u32) -> HMetrics;

    fn units_per_em(&self) -> u16;

    fn general_metrics(&self) -> GeneralMetrics;
}
