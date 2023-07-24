use printpdf::*;

pub fn circle(layer: &PdfLayerReference, pos: [f64; 2], radius: f64) {
    let circle = printpdf::utils::calculate_points_for_circle(Pt(radius), Pt(pos[0]), Pt(pos[1]));

    layer.add_shape(Line {
        points: circle,
        is_closed: true,
        has_fill: true,
        has_stroke: false,
        is_clipping_path: false,
    });
}

pub fn line(layer: &PdfLayerReference, pos: [f64; 2], width: f64, thickness: f64) {
    layer.set_outline_thickness(mm_to_pt(thickness));
    layer.add_shape(printpdf::Line {
        points: vec![
            (Point::new(Mm(pos[0]), Mm(pos[1])), false),
            (Point::new(Mm(pos[0] + width), Mm(pos[1])), false),
        ],
        is_closed: false,
        has_fill: false,
        has_stroke: true,
        is_clipping_path: false,
    });
}

pub fn mm_to_pt(mm: f64) -> f64 {
    Into::<Pt>::into(Mm(mm)).0
}

pub fn pt_to_mm(pt: f64) -> f64 {
    Into::<Mm>::into(Pt(pt)).0
}

pub fn u32_to_color_and_alpha(color: u32) -> (Color, f64) {
    (
        Color::Rgb(Rgb::new(
            ((color & 0xff_00_00_00) >> 24) as f64 / 255.0,
            ((color & 0x00_ff_00_00) >> 16) as f64 / 255.0,
            ((color & 0x00_00_ff_00) >> 8) as f64 / 255.0,
            None,
        )),
        (color & 0x00_00_00_ff) as f64 / 255.0,
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
