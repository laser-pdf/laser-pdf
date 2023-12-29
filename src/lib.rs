// pub mod image;
// pub mod markup;
// pub mod shapes;
// pub mod text;
// pub mod widget;
// pub mod widgets;

pub mod elements;
pub mod fonts;
pub mod text;
pub mod utils;

#[cfg(test)]
pub mod test_utils;

use fonts::Font;
use printpdf::{Mm, PdfDocumentReference, PdfLayerReference};
use serde::{Deserialize, Serialize};

pub const EMPTY_FIELD: &str = "—";

#[derive(Debug)]
pub struct FontSet<'a, F: Font> {
    pub regular: &'a F,
    pub bold: &'a F,
    pub italic: &'a F,
    pub bold_italic: &'a F,
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

// #[derive(Clone, Serialize, Deserialize)]
// #[serde(try_from = "String", into = "String")]
// pub struct SerdeImage {
//     pub path: String,
//     pub image: Image,
// }

// impl Into<String> for SerdeImage {
//     fn into(self) -> String {
//         self.path
//     }
// }

// impl TryFrom<String> for SerdeImage {
//     type Error = std::io::Error;

//     fn try_from(value: String) -> Result<Self, Self::Error> {
//         let path: &std::path::Path = value.as_ref();
//         let image = if path.extension().map_or(false, |e| e == "svg") {
//             Image::Svg(
//                 usvg::Tree::from_file(path, &Default::default())
//                     .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
//             )
//         } else {
//             Image::Pixel(
//                 printpdf::image::open(path)
//                     .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
//             )
//         };

//         Ok(SerdeImage { path: value, image })
//     }
// }

pub struct Pdf {
    pub document: PdfDocumentReference,
    pub page_size: (f64, f64),
}

/// A position for an element to render at.
/// This doesn't include the width at the moment, as this would make things much more complicated.
/// The line breaking iterator wouldn't work in its current form for example.
/// Things are much easier if an element can make width related calculations in the beginning an
/// doesn't have to recalculate them on a page break.
#[derive(Clone, Debug)]
pub struct Location {
    pub layer: PdfLayerReference,
    pub pos: (f64, f64),
}

/// This returns a new [Location] because some collection elements need to keep multiple
/// [Location]s at once (e.g. for page breaking inside of a horizontal list)
///
/// The second parameter is which drawing rectangle the break is occurring from. This number
/// must be counted up for sequential page breaks. This allows the same page break to be
/// performed twice in a row. A new `draw_rect_id` will be returned from the call to
/// `next_location`, so if you store the current draw pos, you can just pass the one from there.
pub type GetLocation<'a> = &'a mut dyn FnMut(&mut Pdf, u32) -> Location;

pub struct InsufficientFirstHeightCtx {
    pub width: Option<f64>,
    pub first_height: f64,
    // is this needed?
    // one could argue that the parent should know to not even ask if full height isn't more
    // on the other hand a text element could have a behavior of printing one line at a time if
    // full-height is less than the height needed, but available-height might still be even less
    // than that and in that case text might still use the first one (though the correctness of that
    // is also questionable)
    // pub full_height: f64,
}

pub struct BreakableMeasure<'a> {
    pub full_height: f64,
    pub break_count: &'a mut u32,

    /// The minimum height required for any extra locations added to the end. If, for example,
    /// there's a flex with a text element that gets repeated for each location and other flex
    /// elements use more locations than this one, the text element will still be drawn on the last
    /// location via `preferred_break_count` and `preferred_height`. The flex needs to be able to
    /// predict the height of the last page so that there isn't a single element that is higher than
    /// the other ones.
    pub extra_location_min_height: &'a mut f64,
}

pub struct MeasureCtx<'a> {
    pub width: Option<f64>,
    pub first_height: f64,
    pub breakable: Option<BreakableMeasure<'a>>,
}

impl<'a> MeasureCtx<'a> {
    pub fn break_if_appropriate_for_min_height(&mut self, height: f64) -> bool {
        if let Some(ref mut breakable) = self.breakable {
            if height > self.first_height && breakable.full_height > self.first_height {
                *breakable.break_count = 1;
                return true;
            }
        }

        false
    }
}

pub struct BreakableDraw<'a> {
    pub full_height: f64,
    pub preferred_height_break_count: u32,
    pub get_location: GetLocation<'a>,
}

pub struct DrawCtx<'a, 'b> {
    pub pdf: &'a mut Pdf,
    pub location: Location,

    pub width: Option<f64>,
    pub first_height: f64,

    pub preferred_height: f64,

    pub breakable: Option<BreakableDraw<'b>>,
}

impl<'a, 'b> DrawCtx<'a, 'b> {
    pub fn break_if_appropriate_for_min_height(&mut self, height: f64) -> bool {
        if let Some(ref mut breakable) = self.breakable {
            if height > self.first_height && breakable.full_height > self.first_height {
                self.location = (breakable.get_location)(self.pdf, 0);
                return true;
            }
        }

        false
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ElementSize {
    pub width: f64,

    /// None here means that this element doesn't need any space on it's last page. This is useful
    /// for things like collapsing gaps after a forced break. This in combination with no breaks
    /// means the element is completely hidden. This can be used to trigger collapsing of gaps even
    /// hiding certain parent containers, like titled, in turn.
    pub height: Option<f64>,
}

/// Rules:
/// Width returned from measure has to be matched in draw given the same
/// constraint (even if there's some preferred height).
pub trait Element {
    // will_break_immediately
    // skip_first_place
    // skip_first
    // instant_break
    // insufficient_height
    // insufficient_first_height
    #[allow(unused_variables)]
    fn insufficient_first_height(&self, ctx: InsufficientFirstHeightCtx) -> bool {
        false
    }

    fn measure(&self, ctx: MeasureCtx) -> Option<ElementSize>;

    fn draw(&self, ctx: DrawCtx) -> Option<ElementSize>;
}
