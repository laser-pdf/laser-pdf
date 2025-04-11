use pdf_writer::Content;
// use printpdf::*;

// pub fn circle(layer: &PdfLayerReference, pos: [f32; 2], radius: f32) {
//     let circle = printpdf::utils::calculate_points_for_circle(Pt(radius), Pt(pos[0]), Pt(pos[1]));

//     layer.add_shape(Line {
//         points: circle,
//         is_closed: true,
//         has_fill: true,
//         has_stroke: false,
//         is_clipping_path: false,
//     });
// }

pub fn line(layer: &mut Content, pos: (f32, f32), width: f32, thickness: f32) {
    // TODO: mm_to_pt
    layer
        .set_line_width(thickness)
        .move_to(mm_to_pt(pos.0), mm_to_pt(pos.1))
        .line_to(mm_to_pt(pos.0 + width), mm_to_pt(pos.1))
        .stroke();
}

pub fn mm_to_pt(mm: f32) -> f32 {
    // (mm as f64 * 2.834_646) as f32
    // (mm as f64 * 72. / 25.4) as f32
    mm * 72. / 25.4
}

pub fn pt_to_mm(pt: f32) -> f32 {
    // (pt as f64 * 0.352_778) as f32
    pt * 25.4 / 72.
}

pub fn u32_to_color_and_alpha(color: u32) -> ([f32; 3], f32) {
    (
        [
            ((color & 0xff_00_00_00) >> 24) as f32 / 255.0,
            ((color & 0x00_ff_00_00) >> 16) as f32 / 255.0,
            ((color & 0x00_00_ff_00) >> 8) as f32 / 255.0,
        ],
        (color & 0x00_00_00_ff) as f32 / 255.0,
    )
}

pub fn u32_to_rgb_color_array(color: u32) -> [u8; 3] {
    [
        ((color & 0xff_00_00_00) >> 24) as u8,
        ((color & 0x00_ff_00_00) >> 16) as u8,
        ((color & 0x00_00_ff_00) >> 8) as u8,
    ]
}

pub fn rgb_color_array_to_u32(color: [u8; 3]) -> u32 {
    ((color[0] as u32) << 24) | ((color[1] as u32) << 16) | ((color[2] as u32) << 8) | 0xFF
}

pub fn max_optional_size(a: Option<f32>, b: Option<f32>) -> Option<f32> {
    match (a, b) {
        (None, None) => None,
        (None, Some(x)) | (Some(x), None) => Some(x),
        (Some(a), Some(b)) => Some(a.max(b)),
    }
}

pub fn add_optional_size(a: Option<f32>, b: Option<f32>) -> Option<f32> {
    match (a, b) {
        (None, None) => None,
        (None, Some(x)) | (Some(x), None) => Some(x),
        (Some(a), Some(b)) => Some(a + b),
    }
}

pub fn add_optional_size_with_gap(a: Option<f32>, b: Option<f32>, gap: f32) -> Option<f32> {
    match (a, b) {
        (None, None) => None,
        (None, Some(x)) | (Some(x), None) => Some(x),
        (Some(a), Some(b)) => Some(a + gap + b),
    }
}

pub fn set_fill_color(layer: &mut Content, color: u32) {
    let (color, _) = u32_to_color_and_alpha(color);
    layer.set_fill_rgb(color[0], color[1], color[2]);
}

pub fn set_stroke_color(layer: &mut Content, color: u32) {
    let (color, _) = u32_to_color_and_alpha(color);
    layer.set_stroke_rgb(color[0], color[1], color[2]);
}

pub fn scale(scale: f32) -> [f32; 6] {
    [scale, 0., 0., scale, 0., 0.]
}
