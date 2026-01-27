pub mod elements;
pub mod flex;
pub mod fonts;
pub mod image;
pub mod serde_elements;
pub mod test_utils;
mod text;
pub mod utils;

use chrono::{Datelike, Timelike, Utc};
use elements::padding::Padding;
use fonts::Font;
use pdf_writer::{Content, Date, Name, Rect, Ref, TextStr};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use xmp_writer::{DateTime, LangId, Timezone, XmpWriter};

pub use crate::text::TextPiecesCache;

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

impl Into<pdf_writer::types::LineCapStyle> for LineCapStyle {
    fn into(self) -> pdf_writer::types::LineCapStyle {
        match self {
            LineCapStyle::Butt => pdf_writer::types::LineCapStyle::ButtCap,
            LineCapStyle::Round => pdf_writer::types::LineCapStyle::RoundCap,
            LineCapStyle::ProjectingSquare => pdf_writer::types::LineCapStyle::ProjectingSquareCap,
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

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct LineStyle {
    pub thickness: f32,
    pub color: Color,
    pub dash_pattern: Option<LineDashPattern>,
    pub cap_style: LineCapStyle,
}

pub struct Layer {
    pub content: Content,
    pub graphics_state_restore_required: bool,
}

pub struct Page {
    pub ext_g_states: Vec<Ref>, // all objects must be indirect for now
    pub x_objects: Vec<Ref>,
    pub layers: Vec<Layer>,
    pub size: (f32, f32),
}

impl Page {
    pub fn add_ext_g_state(&mut self, resource: Ref) -> usize {
        self.ext_g_states.push(resource);
        self.ext_g_states.len() - 1
    }

    pub fn add_x_object(&mut self, resource: Ref) -> String {
        self.x_objects.push(resource);
        (self.x_objects.len() - 1).to_string()
    }
}

/// See ISO 19005 6.6.3 Table 7
#[derive(Clone)]
pub struct Metadata {
    pub title: String,
    // RFC 3306 compliant language identifier
    pub language: String,
    pub keywords: Option<String>,
    pub producer: String,
    pub creation_date: chrono::DateTime<Utc>,
    // ISO 19005 6.6.5
    pub identifier: String,
}

pub struct Pdf {
    pub alloc: Ref,
    pub pdf: pdf_writer::Pdf,
    pub pages: Vec<Page>,
    pub fonts: Vec<Ref>,
    pub metadata: Option<Metadata>,
    truetype_fonts: Vec<fonts::truetype::TruetypeFontState>,
}

pub struct Identifier {}

impl Identifier {
    pub fn new() -> String {
        Uuid::new_v4().to_string()
    }

    // For testing
    pub fn fixed() -> &'static str {
        "00000000-0000-0000-0000-000000000000"
    }
}

impl Pdf {
    pub fn new() -> Self {
        let pdf = pdf_writer::Pdf::new();

        Pdf {
            alloc: pdf_writer::Ref::new(1),
            pdf,
            pages: Vec::new(),
            fonts: Vec::new(),
            metadata: None,
            truetype_fonts: Vec::new(),
        }
    }

    // ISO 19005 6.6.3
    // It states in the PDF/A specificationsthat the fields of the
    // document information dictionary (if present) must be consistent with the
    // values in the document's metadata.
    // Since we aim to support readers and interpreters as well, we should keep the
    // document info. This method tries to ensure that the values are kept in sync.
    pub fn set_metadata(&mut self, metadata: Metadata) -> () {
        self.metadata = Some(metadata);
    }

    pub fn alloc(&mut self) -> Ref {
        self.alloc.bump()
    }

    pub fn add_page(&mut self, size: (f32, f32)) -> Location {
        self.pages.push(Page {
            ext_g_states: Vec::new(),
            x_objects: Vec::new(),
            layers: vec![Layer {
                content: Content::new(),
                graphics_state_restore_required: false,
            }],
            size,
        });

        Location {
            page_idx: self.pages.len() - 1,
            layer_idx: 0,
            pos: (0., size.1),
            scale_factor: 1.,
        }
    }

    /// Add an element to the PDF. A new page with the given size is added initially and additional
    /// pages of the same size are added when the element requests them during drawing.
    pub fn add_element(&mut self, page_size: (f32, f32), element: impl Element) {
        let text_pieces_cache = TextPiecesCache::new();

        self.add_element_with_text_pieces_cache(page_size, &text_pieces_cache, element);
    }

    /// The same as [Pdf::add_element], but with a [TextPiecesCache] parameter. This is useful when
    /// adding multiple elements to a PDF that share some fonts and text.
    pub fn add_element_with_text_pieces_cache(
        &mut self,
        page_size: (f32, f32),
        text_pieces_cache: &TextPiecesCache,
        element: impl Element,
    ) {
        let mut page_idx = self.pages.len() as u32;

        let location = self.add_page((page_size.0, page_size.1));

        let entry_page = page_idx;

        let do_break = &mut |pdf: &mut Pdf, location_idx, _height| {
            while page_idx <= entry_page + location_idx {
                pdf.add_page((page_size.0, page_size.1));
                page_idx += 1;
            }

            Location {
                page_idx: (entry_page + location_idx + 1) as usize,
                layer_idx: 0,
                pos: (0., page_size.1),
                scale_factor: 1.,
            }
        };

        let ctx = DrawCtx {
            pdf: self,
            text_pieces_cache,
            width: WidthConstraint {
                max: page_size.0,
                expand: true,
            },
            location,

            first_height: page_size.1,
            preferred_height: None,

            breakable: Some(BreakableDraw {
                full_height: page_size.1,
                preferred_height_break_count: 0,
                do_break,
            }),
        };

        element.draw(ctx);
    }

    pub fn finish(mut self) -> Vec<u8> {
        let catalog_ref = self.alloc();
        let page_tree_ref = self.alloc();

        // Write document Info and metadata object
        if let Some(metadata) = self.metadata.clone() {
            // The XMP writer is used to create the file metadata object.
            // The schema of it can be seen in ISO 19005 6.6.2.3.3
            // but it's also represented in the API of the xmp-writer crate.
            let mut writer = XmpWriter::new();

            // ISO 32000 14.4
            // ISO 32000 7.5.5 Table 15
            // ISO 19005 6.1.3
            let identifier: Vec<u8> = metadata.identifier.clone().into();
            self.pdf.set_file_id((identifier.clone(), identifier));

            {
                let id = self.alloc();
                let mut document_info = self.pdf.document_info(id);
                document_info.title(TextStr(metadata.title.clone().as_str()));
                if let Some(ref keywords) = metadata.keywords {
                    document_info.keywords(TextStr(keywords));
                }
                document_info.producer(TextStr(&metadata.producer));
                document_info.creation_date(
                    Date::new(metadata.creation_date.year() as u16)
                        .month(metadata.creation_date.month() as u8)
                        .day(metadata.creation_date.day() as u8)
                        .hour(metadata.creation_date.hour() as u8)
                        .minute(metadata.creation_date.minute() as u8)
                        .second(metadata.creation_date.second() as u8),
                );
            }
            writer.title([
                (
                    Some(LangId(&metadata.language.as_str())),
                    metadata.title.as_str(),
                ),
                (None, metadata.title.as_str()),
            ]);

            writer.language([LangId(&metadata.language.as_str())]);

            if let Some(ref keywords) = metadata.keywords {
                writer.pdf_keywords(keywords);
            }

            writer.producer(&metadata.producer);

            writer.create_date(DateTime::new(
                metadata.creation_date.year() as u16,
                metadata.creation_date.month() as u8,
                metadata.creation_date.day() as u8,
                metadata.creation_date.hour() as u8,
                metadata.creation_date.minute() as u8,
                metadata.creation_date.second() as u8,
                Timezone::Utc,
            ));

            writer.xmp_identifier([metadata.identifier.as_str()]);

            writer.pdfa_part(2);
            // ISO 19005 5.2-4
            writer.pdfa_conformance("A");
            writer.pdf_version("1.7");

            let finished = writer.finish(None);
            let id = self.alloc();
            self.pdf.metadata(id, finished.as_bytes());
            let mut catalog = self.pdf.catalog(catalog_ref);
            catalog.metadata(id).pages(page_tree_ref);
            // ISO 19005 6.7.3.3
            // ISO 32000 14.7.2
            // ISO 32000 14.8.4
            // Simon:TODO: to what extent are we trying to represent the document structure here?
            catalog.struct_tree_root();
            // ISO 19005 6.7.2.2
            catalog.mark_info().marked(true);
        } else {
            self.pdf.catalog(catalog_ref).pages(page_tree_ref);
        }

        for mut truetype_font in self.truetype_fonts {
            truetype_font.finish(&mut self.pdf, &mut self.alloc);
        }

        let pages = self
            .pages
            .iter()
            .scan(self.alloc, |state, _| Some(state.bump()));

        self.pdf
            .pages(page_tree_ref)
            .kids(pages)
            .count(self.pages.len() as i32);

        let mut page_alloc = self.alloc;
        self.alloc = Ref::new(self.alloc.get() + self.pages.len() as i32);

        for page in self.pages {
            let mut page_writer = self.pdf.page(page_alloc.bump());

            page_writer
                .parent(page_tree_ref)
                .media_box(Rect::new(
                    0.,
                    0.,
                    (page.size.0 * 72. / 25.4) as f32,
                    (page.size.1 * 72. / 25.4) as f32,
                ))
                .contents_array(
                    page.layers
                        .iter()
                        .scan(self.alloc, |state, _| Some(state.bump())),
                );

            let mut resources = page_writer.resources();

            let mut ext_g_states = resources.ext_g_states();
            for (i, ext_g_state) in page.ext_g_states.iter().enumerate() {
                ext_g_states.pair(Name(format!("{i}").as_bytes()), ext_g_state);
            }
            drop(ext_g_states);

            if !page.x_objects.is_empty() {
                let mut x_objects = resources.x_objects();
                for (i, x_object) in page.x_objects.iter().enumerate() {
                    x_objects.pair(Name(format!("{i}").as_bytes()), x_object);
                }
            }

            let mut fonts = resources.fonts();

            for (i, &font) in self.fonts.iter().enumerate() {
                // TODO: inherit or make an indirect object
                fonts.pair(Name(&format!("F{}", i).as_bytes()), font);
            }

            drop(fonts);
            drop(resources);
            drop(page_writer);

            for mut layer in page.layers {
                if layer.graphics_state_restore_required {
                    layer.content.restore_state();
                }

                // This adds up as long as it's not bumped between the contents_array call and here.
                self.pdf.stream(self.alloc.bump(), &layer.content.finish());
            }
        }

        self.pdf.finish()
    }
}

/// A position for an element to render at.
/// This doesn't include the width at the moment, as this would make things much more complicated.
/// The line breaking iterator wouldn't work in its current form for example.
/// Things are much easier if an element can make width related calculations in the beginning an
/// doesn't have to recalculate them on a page break.
#[derive(Clone, Debug)]
pub struct Location {
    pub page_idx: usize,
    pub layer_idx: usize,
    pub pos: (f32, f32),
    pub scale_factor: f32,
}

impl Location {
    pub fn layer<'a>(&self, pdf: &'a mut Pdf) -> &'a mut Content {
        &mut pdf.pages[self.page_idx].layers[self.layer_idx].content
    }

    pub fn next_layer(&self, pdf: &mut Pdf) -> Location {
        let page = &mut pdf.pages[self.page_idx];

        let mut content = Content::new();

        let graphics_state_restore_required = if self.scale_factor != 1. {
            content
                .save_state()
                .transform(utils::scale(self.scale_factor));
            true
        } else {
            false
        };

        // The issue is some of the layers are scaled. That's why we currently can't reuse them.
        // TODO: Find a better solution that doesn't require adding so many layers, but also doesn't
        // lead to unbalances saves/restores (which is not allowed by the spec).
        page.layers.push(Layer {
            content,
            graphics_state_restore_required,
        });

        Location {
            layer_idx: page.layers.len() - 1,
            ..*self
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidthConstraint {
    pub max: f32,
    pub expand: bool,
}

impl WidthConstraint {
    pub fn constrain(&self, width: f32) -> f32 {
        if self.expand {
            self.max
        } else {
            width.min(self.max)
        }
    }

    pub fn max(&self, width: f32) -> f32 {
        if self.expand {
            width.max(self.max)
        } else {
            width
        }
    }
}

pub type Pos = (f32, f32);
pub type Size = (f32, f32);

/// This returns a new [Location] because some collection elements need to keep multiple
/// [Location]s at once (e.g. for page breaking inside of a horizontal list)
///
/// The second parameter is which location the break is occurring from. This number
/// must be counted up for sequential page breaks. This allows the same page break to be
/// performed twice in a row.
///
/// The third parameter is the height of the location.
pub type Break<'a> = &'a mut dyn FnMut(&mut Pdf, u32, Option<f32>) -> Location;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FirstLocationUsage {
    /// This means the element has no height at all. Meaning it doesn't break either. If the element
    /// breaks, but has a height of None for the first location it should use
    /// [FirstLocationUsage::WillUse] or [FirstLocationUsage::WillSkip] if appropriate.
    NoneHeight,
    WillUse,
    WillSkip,
}

pub struct FirstLocationUsageCtx<'a> {
    pub text_pieces_cache: &'a TextPiecesCache,
    pub width: WidthConstraint,
    pub first_height: f32,

    // is this needed?
    // one could argue that the parent should know to not even ask if full height isn't more
    // on the other hand a text element could have a behavior of printing one line at a time if
    // full-height is less than the height needed, but available-height might still be even less
    // than that and in that case text might still use the first one (though the correctness of that
    // is also questionable)
    pub full_height: f32,
}

impl<'a> FirstLocationUsageCtx<'a> {
    pub fn break_appropriate_for_min_height(&self, height: f32) -> bool {
        height > self.first_height && self.full_height > self.first_height
    }
}

pub struct BreakableMeasure<'a> {
    pub full_height: f32,
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
    /// `Option<Option<f32>>`.
    pub extra_location_min_height: &'a mut Option<f32>,
}

pub struct MeasureCtx<'a> {
    pub text_pieces_cache: &'a TextPiecesCache,
    pub width: WidthConstraint,
    pub first_height: f32,
    pub breakable: Option<BreakableMeasure<'a>>,
}

impl<'a> MeasureCtx<'a> {
    pub fn break_if_appropriate_for_min_height(&mut self, height: f32) -> bool {
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
    pub full_height: f32,
    pub preferred_height_break_count: u32,
    pub do_break: Break<'a>,
}

pub struct DrawCtx<'a, 'b> {
    pub pdf: &'a mut Pdf,
    pub text_pieces_cache: &'a TextPiecesCache,
    pub location: Location,

    pub width: WidthConstraint,
    pub first_height: f32,

    pub preferred_height: Option<f32>,

    pub breakable: Option<BreakableDraw<'b>>,
}

impl<'a, 'b> DrawCtx<'a, 'b> {
    pub fn break_if_appropriate_for_min_height(&mut self, height: f32) -> bool {
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
    pub width: Option<f32>,

    /// None here means that this element doesn't need any space on it's last location. This is
    /// useful for things like collapsing gaps after a forced break. This in combination with no
    /// breaks means the element is completely hidden. This can be used to trigger collapsing of
    /// gaps even hiding certain parent containers, like titled, in turn.
    pub height: Option<f32>,
}

impl ElementSize {
    pub fn new(width: Option<f32>, height: Option<f32>) -> Self {
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

    fn with_padding_top(self, padding: f32) -> Padding<Self>
    where
        Self: Sized,
    {
        Padding {
            left: 0.,
            right: 0.,
            top: padding,
            bottom: 0.,
            element: self,
        }
    }

    fn with_vertical_padding(self, padding: f32) -> Padding<Self>
    where
        Self: Sized,
    {
        Padding {
            left: 0.,
            right: 0.,
            top: padding,
            bottom: padding,
            element: self,
        }
    }

    fn debug(self, color: u8) -> elements::debug::Debug<Self>
    where
        Self: Sized,
    {
        elements::debug::Debug {
            element: self,
            color,
            show_max_width: false,
            show_last_location_max_height: false,
        }
    }
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
            ctx: FirstLocationUsageCtx<'a>,
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
