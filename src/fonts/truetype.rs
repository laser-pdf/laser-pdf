use crate::*;
use std::{
    cell::Cell,
    collections::{BTreeMap, HashMap},
    mem::ManuallyDrop,
    rc::Rc,
};

use fonts::{EncodedGlyph, GeneralMetrics};
use pdf_writer::{
    Chunk, Filter, Name, Str,
    types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap},
    writers::{FontDescriptor, WMode},
};
use rustybuzz::{Face, GlyphBuffer, ShapePlan, UnicodeBuffer, shape_with_plan};
// use elements::padding::Padding;
// use fonts::Font;
// use printpdf::{CurTransMat, Mm, PdfDocumentReference, PdfLayerReference};
use subsetter::GlyphRemapper;
use ttf_parser::GlyphId;

use super::{Font, ShapedGlyph};

pub struct TruetypeFont {
    pub index: usize,
    pub name: Vec<u8>,
    pub face: Face<'static>,
    pub plan: ShapePlan,
    // pub remapper: GlyphRemapper,
    // pub font: Face<'a>,
}

impl TruetypeFont {
    pub fn new(pdf: &mut Pdf, bytes: &'static [u8]) -> Self {
        // let font_reader = std::io::Cursor::new(&bytes);
        // let pdf_font = doc.add_external_font(font_reader).unwrap();
        // let font_info = FontInfo::new(bytes, 0).unwrap();
        //
        //

        // TruetypeFont {
        //     font_ref: pdf_font,
        //     font: font_info,
        // }
        // todo!()

        let face = Face::from_slice(bytes, 0).unwrap();

        let id = pdf.alloc();

        let idx = pdf.fonts.len();
        pdf.fonts.push(id);

        let resource_name = format!("F{}", idx);

        let index = pdf.truetype_fonts.len();
        pdf.truetype_fonts.push(TruetypeFontState {
            glyph_remapper: GlyphRemapper::new(),
            face: face.clone(),
            data: bytes,
            id,
            glyph_set: BTreeMap::new(),
        });

        let plan = ShapePlan::new(
            &face,
            rustybuzz::Direction::LeftToRight,
            Some(rustybuzz::script::LATIN),
            None,
            &[],
        );

        TruetypeFont {
            index,
            name: resource_name.into_bytes(), // face.names().get(0).unwrap().name.to_vec(),
            face,
            plan,
        }
    }
}

thread_local! {
    static UNICODE_BUFFER: Cell<UnicodeBuffer> = Cell::new(UnicodeBuffer::new());
}

impl Font for TruetypeFont {
    type Shaped<'b>
        = Shaped<'b>
    where
        Self: 'b;

    fn shape<'b>(&'b self, text: &'b str) -> Self::Shaped<'b> {
        // In basically all real cases we should end up always taking and returnung the same buffer
        // here. But even in the worst case this should still be better than allocating a new buffer
        // every time.
        let mut buffer = UNICODE_BUFFER.take();

        buffer.push_str(text);

        buffer.set_script(rustybuzz::script::LATIN);
        buffer.set_direction(rustybuzz::Direction::LeftToRight);

        let shaped = shape_with_plan(&self.face, &self.plan, buffer);

        Shaped {
            text,
            buffer: Rc::new(Buffer(ManuallyDrop::new(shaped))),
            i: 0,
        }
    }

    fn encode(&self, pdf: &mut Pdf, glyph_id: u32, text: &str) -> EncodedGlyph {
        let cid = pdf.truetype_fonts[self.index]
            .glyph_remapper
            .remap(glyph_id as u16);

        pdf.truetype_fonts[self.index]
            .glyph_set
            .entry(glyph_id as u16)
            .or_insert_with(|| text.to_string());

        EncodedGlyph::TwoBytes(cid.to_be_bytes())

        // write
        //     .write_all(&[(cid >> 8) as u8, (cid & 0xff) as u8])
        //     .unwrap();
    }

    fn resource_name(&self) -> Name {
        Name(&self.name)
    }

    fn general_metrics(&self) -> GeneralMetrics {
        let ascent = self.face.ascender();
        // let

        // super::GeneralMetrics {
        //     ascent: v_metrics.ascent as f64,
        //     line_height: (v_metrics.ascent + v_metrics.descent.abs() + v_metrics.line_gap) as f64,
        // };
        GeneralMetrics {
            ascent: ascent as u32,

            // It seems that descent is positive in some fonts and negative in others.
            line_height: (ascent + self.face.descender().abs() + self.face.line_gap()) as u32,
        }
    }

    fn units_per_em(&self) -> u16 {
        self.face.units_per_em() as u16
    }
}

struct Buffer(ManuallyDrop<GlyphBuffer>);

impl Drop for Buffer {
    fn drop(&mut self) {
        // Safety: Since we're in drop self.0 can not be used after this point.
        let unicode_buffer = unsafe { ManuallyDrop::take(&mut self.0) }.clear();

        UNICODE_BUFFER.set(unicode_buffer);
    }
}

#[derive(Clone)]
pub struct Shaped<'a> {
    text: &'a str,
    buffer: Rc<Buffer>,
    i: usize,
}

impl<'a> Iterator for Shaped<'a> {
    type Item = ShapedGlyph;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.buffer.0.len() {
            return None;
        }

        let infos = self.buffer.0.glyph_infos();

        let info = infos[self.i];
        let position = self.buffer.0.glyph_positions()[self.i];

        let start = info.cluster as usize;

        // TODO: RTL?
        let mut e = self.i.checked_add(1);
        loop {
            if let Some(index) = e {
                if let Some(end_info) = infos.get(index) {
                    if end_info.cluster == info.cluster {
                        e = index.checked_add(1);
                        continue;
                    }
                }
            }

            break;
        }

        let end = e
            .and_then(|last| infos.get(last))
            .map_or(self.text.len(), |info| info.cluster as usize);

        self.i += 1;

        Some(ShapedGlyph {
            unsafe_to_break: info.unsafe_to_break(),
            glyph_id: info.glyph_id,
            text_range: start..(end as usize),
            x_advance: position.x_advance as u16,
            x_offset: position.x_offset as u16,
            y_offset: position.y_offset as u16,
            y_advance: position.y_advance as u16,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.buffer.0.len() - self.i;
        (len, Some(len))
    }
}

pub(crate) struct TruetypeFontState {
    glyph_remapper: GlyphRemapper,
    face: Face<'static>,
    data: &'static [u8],
    id: Ref,
    glyph_set: BTreeMap<u16, String>,
}

impl TruetypeFontState {
    pub(crate) fn finish(&mut self, pdf: &mut pdf_writer::Pdf, alloc: &mut Ref) {
        let type0_ref = self.id;
        let cid_ref = alloc.bump();
        let descriptor_ref = alloc.bump();
        let cmap_ref = alloc.bump();
        let data_ref = alloc.bump();

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

        let units_per_em = self.face.units_per_em() as f32;

        // Extract the widths of all glyphs.
        // `remapped_gids` returns an iterator over the old GIDs in their new sorted
        // order, so we can append the widths as is.
        let widths = self
            .glyph_remapper
            .remapped_gids()
            .map(|gid| {
                let width = self.face.glyph_hor_advance(GlyphId(gid)).unwrap_or(0);

                (width as f32 / units_per_em * 1000.) as f32
            })
            .collect::<Vec<_>>();

        // Write all non-zero glyph widths.
        let mut first = 0;
        let mut width_writer = cid.widths();
        for group in widths.chunk_by(|&a, &b| a == b) {
            let w = group[0];
            let end = first + group.len();
            if w != 0.0 {
                let last = end - 1;
                width_writer.same(first as u16, last as u16, w);
            }
            first = end;
        }

        drop(width_writer);
        drop(cid);

        let cmap = create_cmap(&self.glyph_set, &self.glyph_remapper);
        pdf.cmap(cmap_ref, &cmap)
            .writing_mode(WMode::Horizontal)
            .filter(Filter::FlateDecode);

        let subset = subset_font(&self.data, &self.glyph_remapper).unwrap();

        let mut stream = pdf.stream(data_ref, &subset);
        stream.filter(Filter::FlateDecode);
        drop(stream);

        let mut font_descriptor = write_font_descriptor(pdf, descriptor_ref, &self.face, "todo");
        font_descriptor.font_file2(data_ref);

        drop(font_descriptor);
    }
}

fn create_cmap(glyph_set: &BTreeMap<u16, String>, glyph_remapper: &GlyphRemapper) -> Vec<u8> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Range;

    use rustybuzz::{Direction, UnicodeBuffer};

    #[test]
    fn test() {
        const FONT: &[u8] = include_bytes!("Kenney Bold.ttf");

        let mut pdf = Pdf::new();

        let font = TruetypeFont::new(&mut pdf, &FONT);

        let text = "Rewriting software in\nRust.";

        let shaped = font.shape(text);
        let shaped = shaped.clone();

        let shaped_vec: Vec<_> = shaped.collect();

        insta::assert_debug_snapshot!(shaped_vec);
    }

    #[test]
    fn test_trailing_space() {
        const FONT: &[u8] = include_bytes!("Kenney Bold.ttf");

        let mut pdf = Pdf::new();

        let font = TruetypeFont::new(&mut pdf, &FONT);

        let text = "Rewriting ";

        let shaped = font.shape(text);
        let shaped = shaped.clone();

        let shaped_vec: Vec<_> = shaped.collect();

        insta::assert_debug_snapshot!(shaped_vec);
    }

    // #[test]
    // fn make_a_pdf() {
    //     use pdf_writer::{
    //         Name, Pdf, Rect, Ref, Str,
    //         types::{CidFontType, SystemInfo},
    //     };
    //     // use ttf_parser::;

    //     let font_data = std::fs::read("../sws/inc/font/nunito/nunito_regular.ttf").unwrap();

    //     let ttf = ttf_parser::Face::parse(&font_data, 0).unwrap();
    //     let mut glyph_remapper: subsetter::GlyphRemapper = Default::default();
    //     let mut glyph_set = HashMap::new();

    //     let mut alloc = Ref::new(1);

    //     // Define some indirect reference ids we'll use.
    //     let catalog_id = alloc.bump();
    //     let page_tree_id = alloc.bump();
    //     let page_id = alloc.bump();
    //     let content_id = alloc.bump();

    //     let font_name = Name(b"F1");

    //     let type0_ref = alloc.bump();
    //     let cid_ref = alloc.bump();
    //     let descriptor_ref = alloc.bump();
    //     let cmap_ref = alloc.bump();
    //     let data_ref = alloc.bump();

    //     // Write a document catalog and a page tree with one A4 page that uses no resources.
    //     let mut pdf = Pdf::new();
    //     pdf.catalog(catalog_id).pages(page_tree_id);
    //     pdf.pages(page_tree_id).kids([page_id]).count(1);
    //     pdf.page(page_id)
    //         .parent(page_tree_id)
    //         .media_box(Rect::new(0.0, 0.0, 595.0, 842.0))
    //         .contents(content_id)
    //         .resources()
    //         .fonts()
    //         .pair(font_name, type0_ref);

    //     let mut content = Content::new();
    //     content.begin_text();
    //     content.set_font(font_name, 14.0);
    //     content.next_line(108.0, 734.0);
    //     // content.show(Str(b"Hello World from Rust!"));

    //     let mut positioned = content.show_positioned();
    //     let mut items = positioned.items();

    //     let mut encoded = Vec::new();

    //     {
    //         let text = "Here in my terminal, just installed this new crate here.";

    //         let glyphs = shape(text, &font_data, 12.);

    //         for glyph in glyphs {
    //             glyph_set
    //                 .entry(glyph.glyph_id as u16)
    //                 .or_insert_with(|| text[glyph.text_range].to_string());

    //             let cid = glyph_remapper.remap(glyph.glyph_id as u16);
    //             // ????
    //             encoded.push((cid >> 8) as u8);
    //             encoded.push((cid & 0xff) as u8);
    //         }

    //         // something about pdf/a???
    //         for chunk in encoded.chunks(0x7FFF) {
    //             items.show(Str(chunk));
    //         }
    //     }

    //     drop(items);
    //     drop(positioned);

    //     content.end_text();
    //     pdf.stream(content_id, &content.finish());

    //     // Write the base font object referencing the CID font.
    //     pdf.type0_font(type0_ref)
    //         .base_font(Name(b"test"))
    //         .encoding_predefined(Name(b"Identity-H")) // TODO: what does this mean??????????
    //         .descendant_font(cid_ref)
    //         .to_unicode(cmap_ref);

    //     // Write the CID font referencing the font descriptor.
    //     let mut cid = pdf.cid_font(cid_ref);
    //     cid.subtype(CidFontType::Type2);
    //     cid.base_font(Name(b"test"));
    //     cid.system_info(SystemInfo {
    //         registry: Str(b"Adobe"), // whyyyy????
    //         ordering: Str(b"Identity"),
    //         supplement: 0,
    //     });
    //     cid.font_descriptor(descriptor_ref);
    //     cid.default_width(0.0);

    //     let units_per_em = ttf.units_per_em() as f32;

    //     // Extract the widths of all glyphs.
    //     // `remapped_gids` returns an iterator over the old GIDs in their new sorted
    //     // order, so we can append the widths as is.
    //     let widths = glyph_remapper
    //         .remapped_gids()
    //         .map(|gid| {
    //             let width = ttf.glyph_hor_advance(GlyphId(gid)).unwrap_or(0);

    //             (width as f32 / units_per_em * 1000.) as f32
    //         })
    //         .collect::<Vec<_>>();

    //     // Write all non-zero glyph widths.
    //     let mut first = 0;
    //     let mut width_writer = cid.widths();
    //     for (w, group) in widths.group_by_key(|&w| w) {
    //         let end = first + group.len();
    //         if w != 0.0 {
    //             let last = end - 1;
    //             width_writer.same(first as u16, last as u16, w);
    //         }
    //         first = end;
    //     }

    //     drop(width_writer);
    //     drop(cid);

    //     let cmap = create_cmap(&glyph_set, &glyph_remapper);
    //     pdf.cmap(cmap_ref, &cmap)
    //         .writing_mode(WMode::Horizontal)
    //         .filter(Filter::FlateDecode);

    //     let subset = subset_font(&font_data, &glyph_remapper).unwrap();

    //     let mut stream = pdf.stream(data_ref, &subset);
    //     stream.filter(Filter::FlateDecode);
    //     drop(stream);

    //     // let mut font_descriptor = write_font_descriptor(&mut pdf, descriptor_ref, &ttf, "todo");
    //     // font_descriptor.font_file2(data_ref);

    //     // drop(font_descriptor);

    //     // Finish with cross-reference table and trailer and write to file.
    //     // std::fs::write("test.pdf", pdf.finish()).unwrap();
    // }

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
}
