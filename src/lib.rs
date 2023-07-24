pub mod break_text_into_lines;
pub mod image;
pub mod markup;
pub mod shapes;
pub mod text;
pub mod utils;
pub mod widget;
pub mod widgets;

use image::Image;
use printpdf::indices::PdfLayerIndex;
use printpdf::*;
use serde::{Deserialize, Serialize};
use stb_truetype as tt;

use std::ops::Deref;

pub const EMPTY_FIELD: &str = "—";

pub fn make_font<D: AsRef<[u8]> + Deref<Target = [u8]>>(
    doc: &PdfDocumentReference,
    bytes: D,
) -> widget::Font<D> {
    let font_reader = std::io::Cursor::new(&bytes);
    let pdf_font = doc.add_external_font(font_reader).unwrap();
    let font_info = tt::FontInfo::new(bytes, 0).unwrap();

    widget::Font {
        font_ref: pdf_font,
        font: font_info,
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum VAlign {
    Top,
    Center,
    Bottom,
}

pub type Color = u32;

/// ISO 32000-1:2008 8.4.3.3
///
/// The line cap style shall specify the shape that shall be used at the ends of
/// open subpaths (and dashes, if any) when they are stroked.
#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum LineCapStyle {
    /// 0: Butt cap. The stroke shall be squared off at the endpoint of the
    /// path. There shall be no projection beyond the end of the path.
    Butt,

    /// 1: Round cap. A semicircular arc with a diameter equal to the line width
    /// shall be drawn around the endpoint and shall be filled in.
    Round,

    /// 2: Projecting square cap. The stroke shall continue beyond the endpoint
    /// of the path for a distance equal to half the line width and shall be
    /// squared off.
    ProjectingSquare,
}

impl Into<printpdf::LineCapStyle> for LineCapStyle {
    fn into(self) -> printpdf::LineCapStyle {
        match self {
            LineCapStyle::Butt => printpdf::LineCapStyle::Butt,
            LineCapStyle::Round => printpdf::LineCapStyle::Round,
            LineCapStyle::ProjectingSquare => printpdf::LineCapStyle::ProjectingSquare,
        }
    }
}

/// ISO 32000-1:2008 8.4.3.6
///
/// The line dash pattern shall control the pattern of dashes and gaps used to
/// stroke paths.
#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct LineDashPattern {
    /// The dash phase shall specify the distance into the dash pattern at which
    /// to start the dash.
    pub offset: u16,

    /// The dash array’s elements shall be numbers that specify the lengths of
    /// alternating dashes and gaps; the numbers shall be nonnegative and not
    /// all zero.
    pub dashes: [u16; 2],
}

impl Into<printpdf::LineDashPattern> for LineDashPattern {
    fn into(self) -> printpdf::LineDashPattern {
        printpdf::LineDashPattern {
            offset: self.offset as i64,
            dash_1: Some(self.dashes[0] as i64),
            gap_1: Some(self.dashes[1] as i64),
            dash_2: None,
            gap_2: None,
            dash_3: None,
            gap_3: None,
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct LineStyle {
    pub thickness: f64,
    pub color: Color,
    pub dash_pattern: Option<LineDashPattern>,
    pub cap_style: LineCapStyle,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SerdeImage {
    pub path: String,
    pub image: Image,
}

impl Into<String> for SerdeImage {
    fn into(self) -> String {
        self.path
    }
}

impl TryFrom<String> for SerdeImage {
    type Error = std::io::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let path: &std::path::Path = value.as_ref();
        let image = if path.extension().map_or(false, |e| e == "svg") {
            Image::Svg(
                usvg::Tree::from_file(path, &Default::default())
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
            )
        } else {
            Image::Pixel(
                printpdf::image::open(path)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
            )
        };

        Ok(SerdeImage { path: value, image })
    }
}

pub struct Pdf {
    pub document: PdfDocumentReference,
    pub page_size: [f64; 2],
}

/// A position for an element to render at.
/// This doesn't include the width at the moment, as this would make things much more complicated.
/// The line breaking iterator wouldn't work in its current form for example.
/// Things are much easier if an element can make width related calculations in the beginning an
/// doesn't have to recalculate them on a page break.
#[derive(Clone, Debug)]
pub struct DrawPos {
    pub layer: PdfLayerReference,
    pub pos: [f64; 2],
    pub height_available: f64,
    pub preferred_height: Option<f64>,
}

// impl DrawPos {
//     pub fn next_layer(&self) -> PdfLayerReference {
//         self.layer.page
//     }
// }

/// The position is in millimeters and in the pdf coordinate system (meaning the origin is on the
/// bottom left corner).
pub struct DrawContext<'a, 'b> {
    // pub pos: [f64; 2],
    // pub height_available: f64,
    pub pdf: &'a mut Pdf,

    pub draw_pos: DrawPos,

    /// The full height of the current drawing rectangle, usually this is the page height minus some
    /// amount of border. It is also the height you're expected to get after a break unless there's
    /// a special [Element] around it like titled.
    pub full_height: f64,

    // /// The height_available after a call to `next_draw_pos`
    // /// This is fine for the moment. It might change in the future to enable variations in page or
    // /// column height. Meaning a call to `next_draw_pos` will need to be able to change this or
    // /// return a new one.
    // pub next_draw_pos_height: f64,
    /// This returns a new [DrawPos] because some collection elements need to keep multiple
    /// [DrawPos]s at once (e.g. for page breaking inside of a horizontal list)
    ///
    /// The second parameter is which drawing rectangle the break is occurring from. This number
    /// must be counted up for sequential page breaks. This allows the same page break to be
    /// performed twice in a row. A new `draw_rect_id` will be returned from the call to
    /// `next_draw_pos`, so if you store the current draw pos, you can just pass the one from there.
    ///
    /// The third parameter is the size of the [Element] on the specified draw rect. If the same
    /// page break is performed multiple times, the largest value on each axis should be used by the
    /// container.
    ///
    /// Note: For correctness we might have to change the size to an option, because
    /// right now `titled` assumes that if the height is zero that means nothing was drawn which
    /// might not be correct for a zero height line or something. Same goes for `widget_or_break`.
    /// Or maybe we can say that if width is also zero then there's nothing, but I'm also not sure
    /// that's correct. We could also just say that zero height must mean there's nothing.
    pub next_draw_pos: Option<&'b mut dyn FnMut(&mut Pdf, u32, [f64; 2]) -> DrawPos>,
}

impl Pdf {
    pub fn next_layer(&mut self, draw_pos: &DrawPos) -> PdfLayerReference {
        let layer = draw_pos.layer.layer;

        let page = self.document.get_page(draw_pos.layer.page);

        if page.layers_len() > layer.0 + 1 {
            page.get_layer(PdfLayerIndex(layer.0 + 1))
        } else {
            page.add_layer(format!("Layer {}", layer.0 + 1))
        }
    }
}

pub trait Element {
    /// A [None] on width means the element should take the width it needs.
    fn element(&self, width: Option<f64>, draw: Option<DrawContext>) -> [f64; 2];
}

impl<F: Fn(Option<f64>, Option<DrawContext>) -> [f64; 2]> Element for F {
    #[inline]
    fn element(&self, width: Option<f64>, draw: Option<DrawContext>) -> [f64; 2] {
        self(width, draw)
    }
}
