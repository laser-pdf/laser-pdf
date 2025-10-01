mod lines;
mod pieces;
mod shaping;

use itertools::Itertools;
use pdf_writer::{Str, writers::PositionedItems};

use crate::{
    Location, Pdf,
    fonts::{EncodedGlyph, Font},
    utils::set_fill_color,
};

pub use lines::*;
pub use pieces::*;

// TODO: Move somewhere else?
const PDFA_LIMIT: usize = 32767;

// Some fonts don't map the U+2010 glyph right, so we use hyphen-minus here.
const HYPHEN: &str = "-";

pub fn draw_line<'a, F: Font>(
    pdf: &mut Pdf,
    location: &Location,
    line: Line<'a, F, impl Iterator<Item = &'a Piece<'a, F>>>,
) {
    fn draw_run(items: &mut PositionedItems, run: &mut Vec<u8>) {
        for chunk in run.chunks(PDFA_LIMIT) {
            items.show(Str(chunk));
        }
    }

    let mut line = line.peekable();

    // TODO: does that capacity make sense?
    let mut glyphs = Vec::with_capacity(line.size_hint().0);

    let mut run = Vec::with_capacity(line.size_hint().0.min(PDFA_LIMIT));

    let mut adjustment = 0.;

    let mut current_font = None;
    let mut current_size = None;
    let mut current_color = None;

    while let Some(next) = line.peek() {
        let font = next.font;
        let size = next.size;
        let color = next.color;

        let same_state = line.peeking_take_while(|x| {
            std::ptr::eq(x.font, font) && x.size == size && x.color == color
        });

        // Should always be empty at this point because we drain it.
        assert!(glyphs.is_empty());

        glyphs.extend(
            same_state
                // we don't want to filter out all unknown glyphs
                .filter(|glyph| !["\n", "\r", "\r\n"].contains(&glyph.text))
                .map(|glyph| {
                    let encoded = font.encode(pdf, glyph.shaped_glyph.glyph_id, glyph.text);

                    (
                        encoded,
                        glyph.shaped_glyph.x_advance_font,
                        glyph.shaped_glyph.x_advance,
                        glyph.shaped_glyph.x_offset,
                    )
                }),
        );

        run.clear();

        let layer = location.layer(pdf);

        if !current_font.map(|f| std::ptr::eq(f, font)).unwrap_or(false)
            || current_size != Some(size)
        {
            layer.set_font(font.resource_name(), size);
        }

        if current_color != Some(color) {
            set_fill_color(layer, color);
        }

        current_font = Some(font);
        current_size = Some(size);
        current_color = Some(color);

        let mut positioned = layer.show_positioned();
        let mut items = positioned.items();

        for (encoded, x_advance_font, x_advance, x_offset) in glyphs.drain(..) {
            adjustment += x_offset as f32;

            if adjustment != 0. {
                if !run.is_empty() {
                    draw_run(&mut items, &mut run);
                    run.clear();
                }

                // For some absurd reason the PDF spec specifies these to be thousandths instead of
                // being in glyph space, which would be the value from the font.
                items.adjust(-(adjustment * 1000.));
                adjustment = 0.;
            }

            match encoded {
                EncodedGlyph::OneByte(byte) => run.push(byte),
                EncodedGlyph::TwoBytes(ref bytes) => run.extend_from_slice(bytes),
            }

            adjustment += x_advance - x_advance_font;
            adjustment -= x_offset;
        }

        if !run.is_empty() {
            draw_run(&mut items, &mut run);
        }
    }
}
