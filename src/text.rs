use std::{iter::Peekable, sync::LazyLock};

use icu_properties::LineBreak;
use icu_segmenter::LineBreakIteratorUtf8;
use pdf_writer::{Str, writers::PositionedItems};

use crate::{
    Location, Pdf,
    fonts::{EncodedGlyph, Font, ShapedGlyph},
};

thread_local! {
    static LINE_SEGMENTER: icu_segmenter::LineSegmenter = icu_segmenter::LineSegmenter::new_auto();
}

static LINE_BREAK_MAP: LazyLock<
    icu_properties::maps::CodePointMapDataBorrowed<'static, icu_properties::LineBreak>,
> = LazyLock::new(icu_properties::maps::line_break);

// TODO: Move somewhere else?
const PDFA_LIMIT: usize = 32767;

// Some fonts don't map the U+2010 glyph right, so we use hyphen-minus here.
const HYPHEN: &str = "-";

pub fn draw_line<'a, F: Font>(
    pdf: &mut Pdf,
    location: &Location,
    font: &'a F,
    text: &str,
    line: Line<F::Shaped<'a>>,
) {
    let mut run = Vec::with_capacity(line.size_hint().0);

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
        .filter(|glyph| !["\n", "\r", "\r\n"].contains(&&text[glyph.text_range.clone()]))
        .map(|glyph| {
            let encoded = font.encode(pdf, glyph.glyph_id, &text[glyph.text_range]);

            (
                encoded,
                glyph.x_advance_font,
                glyph.x_advance,
                glyph.x_offset,
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

struct Piece<'a, S> {
    text: &'a str,
    shaped_start: S,
    width: u32,
    trailing_whitespace_width: u32,
    /// Only applies at the end of the line, otherwise, it should not be counted towards the width.
    trailing_hyphen_width: u32,
    mandatory_break_after: bool,
    glyph_count: usize,
    empty: bool,
}

struct Pieces<'a, 'b, S> {
    current: Option<usize>,
    text: &'a str,
    shaped: S,
    segments: Peekable<LineBreakIteratorUtf8<'b, 'a>>,
    shaped_hyphen: ShapedGlyph,
}

impl<'a, S: Iterator<Item = ShapedGlyph>> Pieces<'a, 'static, S> {
    fn new<F: 'a, R>(
        font: &'a F,
        character_spacing: i32,
        word_spacing: i32,
        text: &'a str,
        f: impl for<'b> FnOnce(Pieces<'a, 'b, S>) -> R,
    ) -> R
    where
        F: Font<Shaped<'a> = S>,
    {
        LINE_SEGMENTER.with(|segmenter| {
            let shaped_hyphen = font.shape(HYPHEN, 0, 0).next().unwrap();
            let shaped = font.shape(text, character_spacing, word_spacing);
            let segments = segmenter.segment_str(text).peekable();

            f(Pieces {
                current: Some(0),
                text,
                shaped,
                segments,
                shaped_hyphen,
            })
        })
    }
}

impl<'a, 'b, S: Clone + Iterator<Item = ShapedGlyph>> Iterator for Pieces<'a, 'b, S> {
    type Item = Piece<'a, S>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut shaped = self.shaped.clone();

        let Some(current) = self.current else {
            return None;
        };

        // TODO: Handle unsafe_to_break somewhere here. If unsafe_to_break is true when we would
        // otherwise split pieces we should probably fuse them into one piece because that seems
        // like the only reasonable thing to do.

        let segment = self.segments.find(|&s| s != 0).unwrap_or_else(|| {
            self.current = None;
            self.text.len()
        });

        let mut iter = std::iter::from_fn({
            let mut done = false;
            let shaped = &mut shaped;
            move || {
                if done {
                    return None;
                }

                let next = shaped.next()?;

                if next.text_range.end >= segment {
                    done = true;
                }

                Some(next)
            }
        })
        .peekable();

        let mut width = 0;
        let mut whitespace_width = 0;
        let mut glyph_count = 0;
        let mut mandatory_break_after = false;

        while let Some(glyph) = iter.next() {
            glyph_count += 1;

            // A space at the end of a line doesn't count towards the width.
            if matches!(
                &self.text[glyph.text_range.clone()],
                " " | "\u{00A0}" | "　"
            ) {
                whitespace_width += glyph.x_advance as u32;
            } else if matches!(
                self.text[glyph.text_range]
                    .chars()
                    .next()
                    .map(|c| LINE_BREAK_MAP.get(c)),
                Some(
                    LineBreak::MandatoryBreak
                        | LineBreak::CarriageReturn
                        | LineBreak::LineFeed
                        | LineBreak::NextLine,
                )
            ) {
                // We probably can't break here because the font might generate two missing glyphs
                // for a \r\n here.
                mandatory_break_after = true;
            } else {
                width += whitespace_width;
                whitespace_width = 0;
                width += glyph.x_advance as u32;
            }
        }

        let text = &self.text[current..segment];

        let trailing_hyphen_width = text
            .ends_with('\u{00AD}')
            .then_some(self.shaped_hyphen.x_advance as u32)
            .unwrap_or(0);

        let piece = Piece {
            text,
            shaped_start: self.shaped.clone(),
            width,
            trailing_whitespace_width: whitespace_width,
            trailing_hyphen_width,
            mandatory_break_after,
            glyph_count,

            // TODO: This might not work for \r\n, but that depends on the shaping. We should
            // proabably find a way to filter out newlines entirely so that they don't show up after
            // line breaking (and maybe also don't get shaped?).
            empty: glyph_count == 0 || (glyph_count == 1 && mandatory_break_after),
        };

        self.current = self.current.and(Some(segment));
        self.shaped = shaped;

        if self.segments.peek().is_none() && !mandatory_break_after {
            self.current = None;
        }

        Some(piece)
    }
}

#[derive(Clone)]
pub struct Line<I> {
    pub empty: bool,
    pub width: u32,
    pub trailing_whitespace_width: u32,
    shaped_hyphen: ShapedGlyph,
    iter: std::iter::Take<I>,
    trailing_hyphen_width: u32,
}

impl<I: Iterator> Iterator for Line<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub struct Lines<'a, 'b, F: Font> {
    max_width: u32,
    generator: LineGenerator<'a, 'b, F>,
}

pub fn lines<'a, R, F: Font>(
    font: &'a F,
    character_spacing: i32,
    word_spacing: i32,
    max_width: u32,
    text: &'a str,
    f: impl for<'b> FnOnce(Lines<'a, 'b, F>) -> R,
) -> R {
    LineGenerator::new(
        font,
        character_spacing,
        word_spacing,
        true,
        text,
        |generator| {
            f(Lines {
                max_width,
                generator,
            })
        },
    )
}

impl<'a, 'b, F: Font> Iterator for Lines<'a, 'b, F> {
    type Item = Line<F::Shaped<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.generator.next(self.max_width, false)
    }
}

pub struct LineGenerator<'a, 'b, F: Font> {
    font: &'a F,
    consider_last_line_trailing_whitespace: bool,
    pieces: Peekable<Pieces<'a, 'b, F::Shaped<'a>>>,
    current: Option<Piece<'a, F::Shaped<'a>>>,
    shaped_hyphen: ShapedGlyph,
}

impl<'a, F: Font> LineGenerator<'a, 'static, F> {
    pub fn new<R>(
        font: &'a F,
        character_spacing: i32,
        word_spacing: i32,
        consider_last_line_trailing_whitespace: bool,
        text: &'a str,
        f: impl for<'b> FnOnce(LineGenerator<'a, 'b, F>) -> R,
    ) -> R {
        Pieces::new(font, character_spacing, word_spacing, text, |pieces| {
            f(LineGenerator {
                font,
                consider_last_line_trailing_whitespace,
                shaped_hyphen: pieces.shaped_hyphen.clone(),
                pieces: pieces.peekable(),
                current: None,
            })
        })
    }
}

impl<'a, 'b, F: Font> LineGenerator<'a, 'b, F> {
    // Needs to be one call to avoid lifetime problems.
    fn current(&mut self) -> Option<(&Piece<'a, F::Shaped<'a>>, bool)> {
        if self.current.is_none() {
            self.current = self.pieces.next();
        }

        self.current
            .as_ref()
            .map(|c| (c, self.pieces.peek().is_some()))
    }

    fn advance(&mut self) {
        self.current = None;
    }
}

impl<'a, 'b, F: Font> LineGenerator<'a, 'b, F> {
    pub fn next(&mut self, max_width: u32, incomplete: bool) -> Option<Line<F::Shaped<'a>>> {
        let Some(start) = self.current().map(|(p, _)| p.shaped_start.clone()) else {
            return None;
        };

        let consider_last_line_trailing_whitespace = self.consider_last_line_trailing_whitespace;

        let mut empty = true;
        let mut glyph_count = 0;
        let mut current_width = 0;
        let mut current_width_whitespace = 0;
        let mut trailing_hyphen_width = 0;

        while let Some((piece, has_next)) = self.current() {
            // If current_width is zero we have to place the piece on this line, because adding
            // another line would not help.
            if (current_width > 0 || incomplete)
                && current_width
                    + current_width_whitespace
                    + piece.width
                    + piece.trailing_hyphen_width
                    + (!has_next && consider_last_line_trailing_whitespace)
                        .then_some(piece.trailing_whitespace_width)
                        .unwrap_or(0)
                    > max_width
            {
                break;
            }

            empty = empty && piece.empty;
            glyph_count += piece.glyph_count;

            current_width += current_width_whitespace + piece.width;
            current_width_whitespace = piece.trailing_whitespace_width;
            trailing_hyphen_width = piece.trailing_hyphen_width;

            let mandatory_break_after = piece.mandatory_break_after;

            self.advance();

            if mandatory_break_after {
                break;
            }
        }

        Some(Line {
            empty,
            width: current_width + trailing_hyphen_width,
            trailing_whitespace_width: current_width_whitespace,
            trailing_hyphen_width,
            shaped_hyphen: self.shaped_hyphen.clone(),
            iter: start.take(glyph_count),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::fonts::ShapedGlyph;

    use super::*;

    #[derive(Debug)]
    struct FakeFont;

    #[derive(Clone, Debug)]
    struct FakeShaped<'a> {
        // last: usize,
        inner: std::str::CharIndices<'a>,
    }

    impl<'a> Iterator for FakeShaped<'a> {
        type Item = ShapedGlyph;

        fn next(&mut self) -> Option<Self::Item> {
            if let Some((i, c)) = self.inner.next() {
                Some(ShapedGlyph {
                    unsafe_to_break: false,
                    glyph_id: c as u32,
                    text_range: i..i + c.len_utf8(),
                    // we don't match newlines here because they produce the missing glyph which has
                    // a non-zero width.
                    x_advance_font: if matches!(c, '\u{00ad}') { 0 } else { 1 },
                    x_advance: if matches!(c, '\u{00ad}') { 0 } else { 1 },
                    x_offset: 0,
                    y_offset: 0,
                    y_advance: 0,
                })
            } else {
                None
            }
        }
    }

    impl Font for FakeFont {
        type Shaped<'a>
            = FakeShaped<'a>
        where
            Self: 'a;

        fn shape<'a>(&'a self, text: &'a str, _: i32, _: i32) -> Self::Shaped<'a> {
            FakeShaped {
                inner: text.char_indices(),
            }
        }

        fn encode(&self, _: &mut crate::Pdf, _: u32, _: &str) -> crate::fonts::EncodedGlyph {
            unimplemented!()
        }

        fn resource_name(&self) -> pdf_writer::Name {
            unimplemented!()
        }

        fn general_metrics(&self) -> crate::fonts::GeneralMetrics {
            unimplemented!()
        }

        fn units_per_em(&self) -> u16 {
            1
        }
    }

    fn collect<'a>(text: &'a str) -> impl Fn(Line<FakeShaped>) -> &'a str {
        |line| {
            let by_range = {
                let mut line = line.clone();
                if let Some(first) = line.next() {
                    let last = line
                        .last()
                        .map(|l| l.text_range.end)
                        .unwrap_or(first.text_range.end);

                    &text[first.text_range.start..last]
                } else {
                    ""
                }
            };

            let mut text = String::new();

            for glyph in line {
                text.push(glyph.glyph_id as u8 as char);
            }

            assert_eq!(by_range, text);
            by_range
        }
    }

    fn collect_piece<'a>(
        text: &'a str,
        piece: Piece<'a, FakeShaped<'a>>,
    ) -> (&'a str, u32, u32, bool) {
        let line = piece.shaped_start.take(piece.text.len());

        let by_range = {
            let mut line = line.clone();
            if let Some(first) = line.next() {
                let last = line
                    .last()
                    .map(|l| l.text_range.end)
                    .unwrap_or(first.text_range.end);

                &text[first.text_range.start..last]
            } else {
                ""
            }
        };

        let mut text = String::new();

        for glyph in line {
            text.push(glyph.glyph_id as u8 as char);
        }

        assert_eq!(by_range, text);
        assert_eq!(text, piece.text);

        (
            piece.text,
            piece.width,
            piece.trailing_whitespace_width,
            piece.mandatory_break_after,
        )
    }

    #[test]
    fn test_pieces_empty() {
        let text = "";

        Pieces::new(&FakeFont, 0, 0, text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(&pieces, &[("", 0, 0, false)]);
        });
    }

    #[test]
    fn test_pieces_one() {
        let text = "abcde";

        Pieces::new(&FakeFont, 0, 0, text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(&pieces, &[("abcde", 5, 0, false)]);
        });
    }

    #[test]
    fn test_pieces_two() {
        let text = "deadbeef defaced";

        Pieces::new(&FakeFont, 0, 0, text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(
                &pieces,
                &[("deadbeef ", 8, 1, false), ("defaced", 7, 0, false)]
            );
        });
    }

    #[test]
    fn test_pieces_three() {
        let text = "deadbeef defaced fart";

        Pieces::new(&FakeFont, 0, 0, text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(
                &pieces,
                &[
                    ("deadbeef ", 8, 1, false),
                    ("defaced ", 7, 1, false),
                    ("fart", 4, 0, false)
                ],
            );
        });
    }

    #[test]
    fn test_pieces_just_newline() {
        let text = "\n";

        Pieces::new(&FakeFont, 0, 0, text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(&pieces, &[("\n", 0, 0, true), ("", 0, 0, false)]);
        });
    }

    #[test]
    fn test_pieces_surrounded_newline() {
        let text = "abc\ndef";

        Pieces::new(&FakeFont, 0, 0, text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(&pieces, &[("abc\n", 3, 0, true), ("def", 3, 0, false)]);
        });
    }

    #[test]
    fn test_pieces_newline_at_start() {
        let text = "\nabc def";

        Pieces::new(&FakeFont, 0, 0, text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(
                &pieces,
                &[
                    ("\n", 0, 0, true),
                    ("abc ", 3, 1, false),
                    ("def", 3, 0, false),
                ]
            );
        });
    }

    #[test]
    fn test_pieces_trailing_newline() {
        let text = "abc def\n";

        Pieces::new(&FakeFont, 0, 0, text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(
                &pieces,
                &[
                    ("abc ", 3, 1, false),
                    ("def\n", 3, 0, true),
                    ("", 0, 0, false),
                ]
            );
        });
    }

    #[test]
    fn test_pieces_just_spaces() {
        let text = "        ";

        Pieces::new(&FakeFont, 0, 0, text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(&pieces, &[("        ", 0, 8, false)]);
        });
    }

    #[test]
    fn test_pieces_mixed_whitespace() {
        let text = "    abc    \ndef  the\tjflkdsa";

        Pieces::new(&FakeFont, 0, 0, text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(
                &pieces,
                &[
                    ("    ", 0, 4, false),
                    // It's somewhat unclear whether the trailing spaces should count toward the
                    // width here.
                    ("abc    \n", 3, 4, true),
                    ("def  ", 3, 2, false),
                    ("the\t", 4, 0, false),
                    ("jflkdsa", 7, 0, false),
                ],
            );
        });
    }

    fn assert_width(width: u32) -> impl Fn(&Line<FakeShaped>) {
        move |l| {
            assert_eq!(l.width, width);
        }
    }

    #[test]
    fn test_empty_string() {
        let text = "";

        LineGenerator::new(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next(16, false).map(&collect), Some(""));
            assert_eq!(generator.next(16, false).map(&collect), None);
        });
    }

    #[test]
    fn test_text_flow() {
        let text = "Amet consequatur facilis necessitatibus sed quia numquam reiciendis. \
                Id impedit quo quaerat enim amet. ";

        LineGenerator::new(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(
                generator
                    .next(16, false)
                    .inspect(assert_width(16))
                    .map(&collect),
                Some("Amet consequatur ")
            );

            assert_eq!(
                generator
                    .next(16, false)
                    .inspect(assert_width(7))
                    .map(&collect),
                Some("facilis ")
            );
            assert_eq!(
                generator.next(16, false).map(&collect),
                Some("necessitatibus ")
            );
            assert_eq!(
                generator.next(16, false).map(&collect),
                Some("sed quia numquam ")
            );
            assert_eq!(
                generator.next(16, false).map(&collect),
                Some("reiciendis. Id ")
            );
            assert_eq!(
                generator.next(16, false).map(&collect),
                Some("impedit quo ")
            );
            assert_eq!(
                generator.next(16, false).map(&collect),
                Some("quaerat enim ")
            );
            assert_eq!(
                generator
                    .next(16, false)
                    .inspect(assert_width(5))
                    .map(&collect),
                Some("amet. ")
            );
            assert_eq!(generator.next(16, false).map(&collect), None);

            // Make sure it's sealed.
            assert_eq!(generator.next(16, false).map(&collect), None);
            assert_eq!(generator.next(16, false).map(&collect), None);
        });
    }

    #[test]
    fn test_text_after_newline() {
        let text = "\nthe the the";

        LineGenerator::new(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next(4, false).map(&collect), Some("\n"));
            assert_eq!(generator.next(4, false).map(&collect), Some("the "));
            assert_eq!(generator.next(4, false).map(&collect), Some("the "));
            assert_eq!(generator.next(4, false).map(&collect), Some("the"));
            assert_eq!(generator.next(4, false).map(&collect), None);
        });
    }

    #[test]
    fn test_trailing_whitespace() {
        let text = "Id impedit quo quaerat enim amet.                  ";

        LineGenerator::new(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(
                generator.next(16, false).map(&collect),
                Some("Id impedit quo ")
            );
            assert_eq!(
                generator.next(16, false).map(&collect),
                Some("quaerat enim ")
            );

            // it's unclear whether any other behavior would be better here
            assert_eq!(
                generator
                    .next(16, false)
                    .inspect(assert_width(5))
                    .map(&collect),
                Some("amet.                  ")
            );
            assert_eq!(generator.next(16, false).map(&collect), None);
        });
    }

    #[test]
    fn test_pre_newline_whitespace() {
        let text = "Id impedit quo \nquaerat enimmmmm    \namet.";
        LineGenerator::new(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(
                generator.next(16, false).map(&collect),
                Some("Id impedit quo \n")
            );
            // It seems unclear what the intent would be in such a case.
            assert_eq!(
                generator.next(16, false).map(&collect),
                Some("quaerat enimmmmm    \n")
            );
            assert_eq!(generator.next(16, false).map(&collect), Some("amet."));
            assert_eq!(generator.next(16, false).map(&collect), None);
        });
    }

    #[test]
    fn test_newline() {
        let text = "\n";

        LineGenerator::new(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next(16, false).map(&collect), Some("\n"));
            assert_eq!(generator.next(16, false).map(&collect), Some(""));
            assert_eq!(generator.next(16, false).map(&collect), None);
        });
    }

    #[test]
    fn test_just_spaces() {
        let text = "  ";

        LineGenerator::new(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next(16, false).map(&collect), Some("  "));
            assert_eq!(generator.next(16, false).map(&collect), None);
        });
    }

    #[test]
    fn test_word_longer_than_line() {
        let text = "Averylongword";

        LineGenerator::new(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(
                generator.next(8, false).map(&collect),
                Some("Averylongword")
            );
            assert_eq!(generator.next(8, false).map(&collect), None);
        });

        let text = "Averylongword test.";

        LineGenerator::new(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(
                generator.next(8, false).map(&collect),
                Some("Averylongword ")
            );
            assert_eq!(generator.next(8, false).map(&collect), Some("test."));
            assert_eq!(generator.next(8, false).map(&collect), None);
        });

        let text = "A verylongword test.";

        LineGenerator::new(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next(8, false).map(&collect), Some("A "));
            assert_eq!(
                generator.next(8, false).map(&collect),
                Some("verylongword ")
            );
            assert_eq!(generator.next(8, false).map(&collect), Some("test."));
            assert_eq!(generator.next(8, false).map(&collect), None);
        });
    }

    #[test]
    fn test_soft_hyphens() {
        let text = "A\u{00ad}very\u{00ad}long\u{00ad}word";

        LineGenerator::new(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(
                generator.next(7, false).map(&collect),
                Some("A\u{00ad}very\u{00ad}"),
            );
            assert_eq!(generator.next(7, false).map(&collect), Some("long\u{00ad}"),);
            assert_eq!(generator.next(7, false).map(&collect), Some("word"));
            assert_eq!(generator.next(7, false).map(&collect), None);
        });

        let text = "A\u{00ad}very \u{00ad}long\u{00ad}word";

        LineGenerator::new(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(
                generator.next(7, false).map(&collect),
                // The old line breaker used to not split at a soft hypen that was at the start of
                // a word. But since the segmenter splits there we treat it as a separate piece now.
                Some("A\u{00ad}very \u{00ad}"),
            );
            assert_eq!(generator.next(7, false).map(&collect), Some("long\u{00ad}"),);
            assert_eq!(generator.next(7, false).map(&collect), Some("word"));
            assert_eq!(generator.next(7, false).map(&collect), None);
        });

        let text = "A\u{00ad}very\u{00ad}\u{00ad}long\u{00ad}word";

        LineGenerator::new(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(
                generator.next(7, false).map(&collect),
                Some("A\u{00ad}very\u{00ad}\u{00ad}"),
            );
            assert_eq!(generator.next(7, false).map(&collect), Some("long\u{00ad}"),);
            assert_eq!(generator.next(7, false).map(&collect), Some("word"));
            assert_eq!(generator.next(7, false).map(&collect), None);
        });
    }

    #[test]
    fn test_hard_hyphens() {
        let text = "A-very-long-word";

        LineGenerator::new(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next(7, false).map(&collect), Some("A-very-"));
            assert_eq!(generator.next(7, false).map(&collect), Some("long-"));
            assert_eq!(generator.next(7, false).map(&collect), Some("word"));
            assert_eq!(generator.next(7, false).map(&collect), None);
        });

        let text = "A-very -long-word";

        LineGenerator::new(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next(7, false).map(&collect), Some("A-very "));
            assert_eq!(generator.next(7, false).map(&collect), Some("-long-"));
            assert_eq!(generator.next(7, false).map(&collect), Some("word"));
            assert_eq!(generator.next(7, false).map(&collect), None);
        });

        let text = "A-very--long-word";

        LineGenerator::new(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next(7, false).map(&collect), Some("A-"));
            assert_eq!(generator.next(7, false).map(&collect), Some("very--"));
            assert_eq!(generator.next(7, false).map(&collect), Some("long-"));
            assert_eq!(generator.next(7, false).map(&collect), Some("word"));
            assert_eq!(generator.next(7, false).map(&collect), None);
        });
    }
}
