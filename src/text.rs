mod lines;
mod pieces;
mod shaping;

use pdf_writer::{Str, writers::PositionedItems};

use crate::{
    Location, Pdf,
    fonts::{EncodedGlyph, Font},
};

pub use lines::*;

// TODO: Move somewhere else?
const PDFA_LIMIT: usize = 32767;

// Some fonts don't map the U+2010 glyph right, so we use hyphen-minus here.
const HYPHEN: &str = "-";

pub fn draw_line<'a, F: Font>(
    pdf: &mut Pdf,
    location: &Location,
    font: &'a F,
    text: &str,
    line: Line<'a, '_, F>,
) {
    let mut run = Vec::with_capacity(line.size_hint().0.min(PDFA_LIMIT));

    fn draw_run(items: &mut PositionedItems, run: &mut Vec<u8>) {
        for chunk in run.chunks(PDFA_LIMIT) {
            items.show(Str(chunk));
        }
    }

    let hyphen = (line.trailing_hyphen_width != 0).then(|| {
        (
            font.encode(pdf, line.shaped_hyphen.glyph_id, HYPHEN),
            line.shaped_hyphen.x_advance_font,
            line.shaped_hyphen.x_advance,
            line.shaped_hyphen.x_offset,
        )
    });

    let glyphs: Vec<_> = line
        // we don't want to filter out all unknown glyphs
        .filter(|glyph| !["\n", "\r", "\r\n"].contains(&&text[glyph.1.text_range.clone()]))
        .map(|glyph| {
            // TODO: use the right fonts and stuff!!!!!
            let encoded = glyph
                .0
                .encode(pdf, glyph.1.glyph_id, &text[glyph.1.text_range.clone()]);

            (
                encoded,
                glyph.1.x_advance_font,
                glyph.1.x_advance,
                glyph.1.x_offset,
            )
        })
        .chain(hyphen)
        .collect();

    let layer = location.layer(pdf);
    let mut positioned = layer.show_positioned();
    let mut items = positioned.items();

    let mut adjustment = 0.;

    for (encoded, x_advance_font, x_advance, x_offset) in glyphs {
        adjustment += x_offset as f32;

        if adjustment != 0. {
            if !run.is_empty() {
                draw_run(&mut items, &mut run);
                run.clear();
            }

            // For some absurd reason the PDF spec specifies these to be thousandths instead of
            // being in glyph space, which would be the value from the font.
            items.adjust(-(adjustment * 1000. / font.units_per_em() as f32));
            adjustment = 0.;
        }

        match encoded {
            EncodedGlyph::OneByte(byte) => run.push(byte),
            EncodedGlyph::TwoBytes(ref bytes) => run.extend_from_slice(bytes),
        }

        adjustment += (x_advance - x_advance_font) as f32;
        adjustment -= x_offset as f32;
    }

    if !run.is_empty() {
        draw_run(&mut items, &mut run);
    }
}
