// pub mod elements;
pub mod fonts;
// pub mod image;
// pub mod serde_elements;
// pub mod text;
// pub mod utils;

// #[cfg(test)]
// pub mod test_utils;

pub(crate) mod flex;

use std::{collections::HashMap, ops::Range};

use pdf_writer::{
    types::{FontFlags, SystemInfo, UnicodeCmap},
    writers::{FontDescriptor, WMode},
    Chunk, Content, Filter, Name, Ref, Str,
};
use rustybuzz::{Direction, UnicodeBuffer};
// use elements::padding::Padding;
// use fonts::Font;
// use printpdf::{CurTransMat, Mm, PdfDocumentReference, PdfLayerReference};
use serde::{Deserialize, Serialize};
use subsetter::GlyphRemapper;
use ttf_parser::{Face, GlyphId};
use typst_utils::SliceExt;

pub const EMPTY_FIELD: &str = "—";

// #[derive(Debug)]
// pub struct FontSet<'a, F: Font> {
//     pub regular: &'a F,
//     pub bold: &'a F,
//     pub italic: &'a F,
//     pub bold_italic: &'a F,
// }

// impl<'a, F: Font> Clone for FontSet<'a, F> {
//     fn clone(&self) -> Self {
//         *self
//     }
// }

// impl<'a, F: Font> Copy for FontSet<'a, F> {}

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

// impl Into<printpdf::LineCapStyle> for LineCapStyle {
//     fn into(self) -> printpdf::LineCapStyle {
//         match self {
//             LineCapStyle::Butt => printpdf::LineCapStyle::Butt,
//             LineCapStyle::Round => printpdf::LineCapStyle::Round,
//             LineCapStyle::ProjectingSquare => printpdf::LineCapStyle::ProjectingSquare,
//         }
//     }
// }

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

pub struct Pdf {
    // pub document: PdfDocumentReference,
    pub page_size: (f64, f64),
}

/// A position for an element to render at.
/// This doesn't include the width at the moment, as this would make things much more complicated.
/// The line breaking iterator wouldn't work in its current form for example.
/// Things are much easier if an element can make width related calculations in the beginning an
/// doesn't have to recalculate them on a page break.
#[derive(Clone, Debug)]
pub struct Location {
    // pub layer: PdfLayerReference,
    pub pos: (f64, f64),
    pub scale_factor: f64,
}

impl Location {
    pub fn next_layer(&self, pdf: &mut Pdf) -> Location {
        // let page = pdf.document.get_page(self.layer.page);

        // // The issue is some of the layers are scaled. That's why we currently can't reuse them.
        // // TODO: Find a better solution that doesn't require adding so many layers, but also doesn't
        // // lead to unbalances saves/restores (which is not allowed by the spec).
        // let layer = page.add_layer(format!("Layer {}", page.layers_len()));

        // if self.scale_factor != 1. {
        //     layer.set_ctm(CurTransMat::Scale(self.scale_factor, self.scale_factor));
        // }

        // Location { layer, ..*self }
        todo!()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidthConstraint {
    pub max: f64,
    pub expand: bool,
}

impl WidthConstraint {
    pub fn constrain(&self, width: f64) -> f64 {
        if self.expand {
            self.max
        } else {
            width.min(self.max)
        }
    }
}

pub type Pos = (f64, f64);
pub type Size = (f64, f64);

/// This returns a new [Location] because some collection elements need to keep multiple
/// [Location]s at once (e.g. for page breaking inside of a horizontal list)
///
/// The second parameter is which location the break is occurring from. This number
/// must be counted up for sequential page breaks. This allows the same page break to be
/// performed twice in a row.
///
/// The third parameter is the height of the location.
pub type Break<'a> = &'a mut dyn FnMut(&mut Pdf, u32, Option<f64>) -> Location;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FirstLocationUsage {
    /// This means the element has no height at all. Meaning it doesn't break either. If the element
    /// breaks, but has a height of None for the first location it should use
    /// [FirstLocationUsage::WillUse] or [FirstLocationUsage::WillSkip] if appropriate.
    NoneHeight,
    WillUse,
    WillSkip,
}

pub struct FirstLocationUsageCtx {
    pub width: WidthConstraint,
    pub first_height: f64,

    // is this needed?
    // one could argue that the parent should know to not even ask if full height isn't more
    // on the other hand a text element could have a behavior of printing one line at a time if
    // full-height is less than the height needed, but available-height might still be even less
    // than that and in that case text might still use the first one (though the correctness of that
    // is also questionable)
    pub full_height: f64,
}

impl FirstLocationUsageCtx {
    pub fn break_appropriate_for_min_height(&self, height: f64) -> bool {
        height > self.first_height && self.full_height > self.first_height
    }
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
    /// `None` here means the element does not use extra locations. This means it is not possible
    /// to have an element that does use extra locations, but returns a `None` height on the last
    /// one. Should that ever become necessary we'll probably have to change this to an
    /// `Option<Option<f64>>`.
    pub extra_location_min_height: &'a mut Option<f64>,
}

pub struct MeasureCtx<'a> {
    pub width: WidthConstraint,
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
    pub do_break: Break<'a>,
}

pub struct DrawCtx<'a, 'b> {
    pub pdf: &'a mut Pdf,
    pub location: Location,

    pub width: WidthConstraint,
    pub first_height: f64,

    pub preferred_height: Option<f64>,

    pub breakable: Option<BreakableDraw<'b>>,
}

impl<'a, 'b> DrawCtx<'a, 'b> {
    pub fn break_if_appropriate_for_min_height(&mut self, height: f64) -> bool {
        if let Some(ref mut breakable) = self.breakable {
            if height > self.first_height && breakable.full_height > self.first_height {
                // TODO: Make sure this is correct. Maybe this function needs to be renamed to make
                // clear what this actually does.
                self.location = (breakable.do_break)(self.pdf, 0, None);
                return true;
            }
        }

        false
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ElementSize {
    pub width: Option<f64>,

    /// None here means that this element doesn't need any space on it's last page. This is useful
    /// for things like collapsing gaps after a forced break. This in combination with no breaks
    /// means the element is completely hidden. This can be used to trigger collapsing of gaps even
    /// hiding certain parent containers, like titled, in turn.
    pub height: Option<f64>,
}

impl ElementSize {
    pub fn new(width: Option<f64>, height: Option<f64>) -> Self {
        ElementSize { width, height }
    }
}

/// Rules:
/// Width returned from measure has to be matched in draw given the same
/// constraint (even if there's some preferred height).
pub trait Element {
    #[allow(unused_variables)]
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        FirstLocationUsage::WillUse
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize;

    fn draw(&self, ctx: DrawCtx) -> ElementSize;

    // fn with_padding_top(&self, padding: f64) -> Padding<Self>
    // where
    //     Self: Sized,
    // {
    //     Padding {
    //         left: 0.,
    //         right: 0.,
    //         top: padding,
    //         bottom: 0.,
    //         element: self,
    //     }
    // }

    // fn with_vertical_padding(&self, padding: f64) -> Padding<Self>
    // where
    //     Self: Sized,
    // {
    //     Padding {
    //         left: 0.,
    //         right: 0.,
    //         top: padding,
    //         bottom: padding,
    //         element: self,
    //     }
    // }

    // fn debug(&self, color: u8) -> elements::debug::Debug<Self>
    // where
    //     Self: Sized,
    // {
    //     elements::debug::Debug {
    //         element: self,
    //         color,
    //         show_max_width: false,
    //         show_last_location_max_height: false,
    //     }
    // }
}

pub trait CompositeElementCallback {
    fn call(self, element: &impl Element);
}

pub trait CompositeElement {
    fn element(&self, callback: impl CompositeElementCallback);
}

impl<C: CompositeElement> Element for C {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        struct Callback<'a> {
            ctx: FirstLocationUsageCtx,
            ret: &'a mut FirstLocationUsage,
        }

        impl<'a> CompositeElementCallback for Callback<'a> {
            fn call(self, element: &impl Element) {
                *self.ret = element.first_location_usage(self.ctx);
            }
        }

        let mut ret = FirstLocationUsage::NoneHeight;

        self.element(Callback { ctx, ret: &mut ret });

        ret
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        struct Callback<'a> {
            ctx: MeasureCtx<'a>,
            ret: &'a mut ElementSize,
        }

        impl<'a> CompositeElementCallback for Callback<'a> {
            fn call(self, element: &impl Element) {
                *self.ret = element.measure(self.ctx);
            }
        }

        let mut ret = ElementSize {
            width: None,
            height: None,
        };

        self.element(Callback { ctx, ret: &mut ret });

        ret
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        struct Callback<'pdf, 'a, 'r> {
            ctx: DrawCtx<'pdf, 'a>,
            ret: &'r mut ElementSize,
        }

        impl<'pdf, 'a, 'r> CompositeElementCallback for Callback<'pdf, 'a, 'r> {
            fn call(self, element: &impl Element) {
                *self.ret = element.draw(self.ctx);
            }
        }

        let mut ret = ElementSize {
            width: None,
            height: None,
        };

        self.element(Callback { ctx, ret: &mut ret });

        ret
    }
}

// -------------------------------------------------------------------------------------------------

// pub trait BuildElement<'a, F: 'static> {
//     type R: Element + 'a;

//     fn call(self, fonts: &'a F) -> Self::R;
// }

// impl<'a, F: 'static, R: Element + 'a, O: FnOnce(&'a F) -> R> BuildElement<'a, F> for O {
//     type R = R;

//     #[inline]
//     fn call(self, fonts: &'a F) -> Self::R {
//         self(fonts)
//     }
// }

// pub fn build_pdf<F: 'static>(
//     name: &str,
//     page_size: (f64, f64),
//     build_fonts: impl FnOnce(&PdfDocumentReference) -> F,
//     build_element: impl for<'a> BuildElement<'a, F>,
// ) -> printpdf::PdfDocumentReference {
//     use printpdf::{
//         indices::{PdfLayerIndex, PdfPageIndex},
//         PdfDocument,
//     };

//     let (doc, page, layer) = PdfDocument::new(name, Mm(page_size.0), Mm(page_size.1), "Layer 0");
//     let mut page_idx = 0;

//     let mut pdf = Pdf {
//         document: doc,
//         page_size,
//     };

//     let do_break = &mut |pdf: &mut Pdf, location_idx, size| {
//         while page_idx <= location_idx {
//             pdf.document
//                 .add_page(Mm(page_size.0), Mm(page_size.1), "Layer 0");
//             page_idx += 1;
//         }

//         let layer = pdf
//             .document
//             .get_page(PdfPageIndex((location_idx + 1) as usize))
//             .get_layer(PdfLayerIndex(0));

//         Location {
//             layer,
//             pos: (0., page_size.1),
//             scale_factor: 1.,
//         }
//     };

//     let layer = pdf.document.get_page(page).get_layer(layer);

//     let fonts = build_fonts(&pdf.document);

//     let element = build_element.call(&fonts);

//     let ctx = DrawCtx {
//         pdf: &mut pdf,
//         width: WidthConstraint {
//             max: page_size.0,
//             expand: true,
//         },
//         location: Location {
//             layer,
//             pos: (0., page_size.1),
//             scale_factor: 1.,
//         },

//         first_height: page_size.1,
//         preferred_height: None,

//         breakable: Some(BreakableDraw {
//             full_height: page_size.1,
//             preferred_height_break_count: 0,
//             do_break,
//         }),
//     };

//     element.draw(ctx);

//     pdf.document
// }

#[test]
fn make_a_pdf() {
    use pdf_writer::{
        types::{CidFontType, SystemInfo},
        Name, Pdf, Rect, Ref, Str,
    };
    // use ttf_parser::;

    let font_data = std::fs::read("../sws/inc/font/nunito/nunito_regular.ttf").unwrap();

    let ttf = ttf_parser::Face::parse(&font_data, 0).unwrap();
    let mut glyph_remapper: subsetter::GlyphRemapper = Default::default();
    let mut glyph_set = HashMap::new();

    let mut alloc = Ref::new(1);

    // Define some indirect reference ids we'll use.
    let catalog_id = alloc.bump();
    let page_tree_id = alloc.bump();
    let page_id = alloc.bump();
    let content_id = alloc.bump();

    let font_name = Name(b"F1");

    let type0_ref = alloc.bump();
    let cid_ref = alloc.bump();
    let descriptor_ref = alloc.bump();
    let cmap_ref = alloc.bump();
    let data_ref = alloc.bump();

    // Write a document catalog and a page tree with one A4 page that uses no resources.
    let mut pdf = Pdf::new();
    pdf.catalog(catalog_id).pages(page_tree_id);
    pdf.pages(page_tree_id).kids([page_id]).count(1);
    pdf.page(page_id)
        .parent(page_tree_id)
        .media_box(Rect::new(0.0, 0.0, 595.0, 842.0))
        .contents(content_id)
        .resources()
        .fonts()
        .pair(font_name, type0_ref);

    let mut content = Content::new();
    content.begin_text();
    content.set_font(font_name, 14.0);
    content.next_line(108.0, 734.0);
    // content.show(Str(b"Hello World from Rust!"));

    let mut positioned = content.show_positioned();
    let mut items = positioned.items();

    let mut encoded = Vec::new();

    {
        let text = "Here in my terminal, just installed this new crate here.";

        let glyphs = shape(text, &font_data, 12.);

        for glyph in glyphs {
            glyph_set
                .entry(glyph.glyph_id as u16)
                .or_insert_with(|| text[glyph.text_range].to_string());

            let cid = glyph_remapper.remap(glyph.glyph_id as u16);
            // ????
            encoded.push((cid >> 8) as u8);
            encoded.push((cid & 0xff) as u8);
        }

        // something about pdf/a???
        for chunk in encoded.chunks(0x7FFF) {
            items.show(Str(chunk));
        }
    }

    drop(items);
    drop(positioned);

    content.end_text();
    pdf.stream(content_id, &content.finish());

    // Write the base font object referencing the CID font.
    pdf.type0_font(type0_ref)
        .base_font(Name(b"test"))
        .encoding_predefined(Name(b"Identity-H")) // TODO: what does this mean??????????
        .descendant_font(cid_ref)
        .to_unicode(cmap_ref);

    // Write the CID font referencing the font descriptor.
    let mut cid = pdf.cid_font(cid_ref);
    cid.subtype(CidFontType::Type2);
    cid.base_font(Name(b"test"));
    cid.system_info(SystemInfo {
        registry: Str(b"Adobe"), // whyyyy????
        ordering: Str(b"Identity"),
        supplement: 0,
    });
    cid.font_descriptor(descriptor_ref);
    cid.default_width(0.0);

    let units_per_em = ttf.units_per_em() as f64;

    // Extract the widths of all glyphs.
    // `remapped_gids` returns an iterator over the old GIDs in their new sorted
    // order, so we can append the widths as is.
    let widths = glyph_remapper
        .remapped_gids()
        .map(|gid| {
            let width = ttf.glyph_hor_advance(GlyphId(gid)).unwrap_or(0);

            (width as f64 / units_per_em * 1000.) as f32
        })
        .collect::<Vec<_>>();

    // Write all non-zero glyph widths.
    let mut first = 0;
    let mut width_writer = cid.widths();
    for (w, group) in widths.group_by_key(|&w| w) {
        let end = first + group.len();
        if w != 0.0 {
            let last = end - 1;
            width_writer.same(first as u16, last as u16, w);
        }
        first = end;
    }

    drop(width_writer);
    drop(cid);

    let cmap = create_cmap(&glyph_set, &glyph_remapper);
    pdf.cmap(cmap_ref, &cmap)
        .writing_mode(WMode::Horizontal)
        .filter(Filter::FlateDecode);

    let subset = subset_font(&font_data, &glyph_remapper).unwrap();

    let mut stream = pdf.stream(data_ref, &subset);
    stream.filter(Filter::FlateDecode);
    drop(stream);

    let mut font_descriptor = write_font_descriptor(&mut pdf, descriptor_ref, &ttf, "todo");
    font_descriptor.font_file2(data_ref);

    drop(font_descriptor);

    // Finish with cross-reference table and trailer and write to file.
    std::fs::write("test.pdf", pdf.finish()).unwrap();
}

fn create_cmap(glyph_set: &HashMap<u16, String>, glyph_remapper: &GlyphRemapper) -> Vec<u8> {
    // Produce a reverse mapping from glyphs' CIDs to unicode strings.
    let mut cmap = UnicodeCmap::new(
        Name(b"Custom"),
        SystemInfo {
            registry: Str(b"Adobe"), // whyyyy????
            ordering: Str(b"Identity"),
            supplement: 0,
        },
    );
    for (&g, text) in glyph_set.iter() {
        // See commend in `write_normal_text` for why we can choose the CID this way.
        let cid = glyph_remapper.get(g).unwrap();
        if !text.is_empty() {
            cmap.pair_with_multiple(cid, text.chars());
        }
    }
    deflate(&cmap.finish())
}

fn subset_font(font: &[u8], glyph_remapper: &GlyphRemapper) -> Result<Vec<u8>, subsetter::Error> {
    let subset = subsetter::subset(font, 0, glyph_remapper)?;
    let data = subset.as_ref();

    Ok(deflate(data))
}

fn deflate(data: &[u8]) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec_zlib(data, 9)
}

/// Writes a FontDescriptor dictionary.
pub fn write_font_descriptor<'a>(
    pdf: &'a mut Chunk,
    descriptor_ref: Ref,
    // font: &'a Font,
    font: &'a Face,
    base_font: &str,
) -> FontDescriptor<'a> {
    let ttf = font;
    // let metrics = font.metrics();
    // let serif = font
    //     .find_name(name_id::POST_SCRIPT_NAME)
    //     .is_some_and(|name| name.contains("Serif"));
    let serif = false; // TODO

    let mut flags = FontFlags::empty();
    flags.set(FontFlags::SERIF, serif);
    flags.set(FontFlags::FIXED_PITCH, ttf.is_monospaced());
    flags.set(FontFlags::ITALIC, ttf.is_italic());
    flags.insert(FontFlags::SYMBOLIC);
    flags.insert(FontFlags::SMALL_CAP);

    let units_per_em = ttf.units_per_em() as f32;

    let global_bbox = ttf.global_bounding_box();
    let bbox = pdf_writer::Rect::new(
        f32::from(global_bbox.x_min) / units_per_em * 1000.,
        f32::from(global_bbox.y_min) / units_per_em * 1000.,
        f32::from(global_bbox.x_max) / units_per_em * 1000.,
        f32::from(global_bbox.y_min) / units_per_em * 1000.,
    );

    let italic_angle = ttf.italic_angle();
    let ascender =
        f32::from(ttf.typographic_ascender().unwrap_or(ttf.ascender())) / units_per_em * 1000.;
    let descender =
        f32::from(ttf.typographic_descender().unwrap_or(ttf.descender())) / units_per_em * 1000.;
    let cap_height = ttf
        .capital_height()
        .filter(|&h| h > 0)
        .map_or(ascender, |h| f32::from(h) / units_per_em * 1000.);
    let stem_v = 10.0 + 0.244 * (f32::from(ttf.weight().to_number()) - 50.0);

    // Write the font descriptor (contains metrics about the font).
    let mut font_descriptor = pdf.font_descriptor(descriptor_ref);
    font_descriptor
        .name(Name(base_font.as_bytes()))
        .flags(flags)
        .bbox(bbox)
        .italic_angle(italic_angle)
        .ascent(ascender)
        .descent(descender)
        .cap_height(cap_height)
        .stem_v(stem_v);

    font_descriptor
}

struct Glyph {
    /// The glyph ID of the glyph.
    pub glyph_id: u32,
    /// The range in the original text that corresponds to the
    /// cluster of the glyph.
    pub text_range: Range<usize>,
    /// The advance of the glyph.
    pub x_advance: f32,
    /// The x offset of the glyph.
    pub x_offset: f32,
    /// The y offset of the glyph.
    pub y_offset: f32,
    /// The y advance of the glyph.
    pub y_advance: f32,
}

fn shape(text: &str, font: &[u8], size: f32) -> Vec<Glyph> {
    let data = font;
    let rb_font = rustybuzz::Face::from_slice(data.as_ref().as_ref(), 0).unwrap();

    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();

    buffer.set_direction(Direction::LeftToRight);

    let dir = buffer.direction();

    let output = rustybuzz::shape(&rb_font, &[], buffer);

    let positions = output.glyph_positions();
    let infos = output.glyph_infos();

    let mut glyphs = vec![];

    for i in 0..output.len() {
        let pos = positions[i];
        let start_info = infos[i];

        let start = start_info.cluster as usize;

        let end = if dir == Direction::LeftToRight || dir == Direction::TopToBottom {
            let mut e = i.checked_add(1);
            loop {
                if let Some(index) = e {
                    if let Some(end_info) = infos.get(index) {
                        if end_info.cluster == start_info.cluster {
                            e = index.checked_add(1);
                            continue;
                        }
                    }
                }

                break;
            }

            e
        } else {
            let mut e = i.checked_sub(1);
            while let Some(index) = e {
                if let Some(end_info) = infos.get(index) {
                    if end_info.cluster == start_info.cluster {
                        e = index.checked_sub(1);
                    } else {
                        break;
                    }
                }
            }

            e
        }
        .and_then(|last| infos.get(last))
        .map_or(text.len(), |info| info.cluster as usize);

        glyphs.push(Glyph {
            glyph_id: start_info.glyph_id,
            text_range: start..end,
            x_advance: (pos.x_advance as f32 / rb_font.units_per_em() as f32) * size,
            x_offset: (pos.x_offset as f32 / rb_font.units_per_em() as f32) * size,
            y_offset: (pos.y_offset as f32 / rb_font.units_per_em() as f32) * size,
            y_advance: (pos.y_advance as f32 / rb_font.units_per_em() as f32) * size,
        });
    }

    glyphs
}
