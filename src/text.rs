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

    let glyphs: Vec<_> = line
        // we don't want to filter out all unknown glyphs
        .filter(|glyph| !["\n", "\r", "\r\n"].contains(&&text[glyph.text_range.clone()]))
        .map(|glyph| {
            let encoded = font.encode(pdf, glyph.glyph_id, &text[glyph.text_range]);

            (encoded, glyph.x_offset)
        })
        .collect();

    let layer = location.layer(pdf);
    let mut positioned = layer.show_positioned();
    let mut items = positioned.items();

    for (encoded, x_offset) in glyphs {
        if x_offset != 0 {
            if !run.is_empty() {
                draw_run(&mut items, &mut run);
                run.clear();
            }

            items.adjust(-(x_offset as f32));
        }

        match encoded {
            EncodedGlyph::OneByte(byte) => run.push(byte),
            EncodedGlyph::TwoBytes(ref bytes) => run.extend_from_slice(bytes),
        }
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
    mandatory_break_after: bool,
    glyph_count: usize,
}

struct Pieces<'a, 'b, S> {
    current: Option<usize>,
    text: &'a str,
    shaped: S,
    segments: Peekable<LineBreakIteratorUtf8<'b, 'a>>,
}

impl<'a, S> Pieces<'a, 'static, S> {
    fn new<F: 'a, R>(
        font: &'a F,
        text: &'a str,
        f: impl for<'b> FnOnce(Pieces<'a, 'b, S>) -> R,
    ) -> R
    where
        F: Font<Shaped<'a> = S>,
    {
        LINE_SEGMENTER.with(|segmenter| {
            let shaped = font.shape(text);
            let segments = segmenter.segment_str(text).peekable();

            f(Pieces {
                current: Some(0),
                text,
                shaped,
                segments,
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

        while let Some(glyph) = iter.next() {
            glyph_count += 1;

            // A space at the end of a line doesn't count towards the width.
            if matches!(&self.text[glyph.text_range], " " | "\u{00A0}" | "　") {
                whitespace_width += glyph.x_advance as u32;
            } else {
                width += whitespace_width;
                whitespace_width = 0;
                width += glyph.x_advance as u32;
            }
        }

        let last_char = self.text[..segment].chars().next_back();

        let mandatory = match last_char
            .filter(|_| self.current.is_some())
            .map(|l| LINE_BREAK_MAP.get(l))
        {
            Some(
                LineBreak::MandatoryBreak
                | LineBreak::CarriageReturn
                | LineBreak::LineFeed
                | LineBreak::NextLine,
            ) => true,
            _ => false,
        };

        let piece = Piece {
            text: &self.text[current..segment],
            shaped_start: self.shaped.clone(),
            width,
            trailing_whitespace_width: whitespace_width,
            mandatory_break_after: mandatory,
            glyph_count,
        };

        self.current = self.current.and(Some(segment));
        self.shaped = shaped;

        if self.segments.peek().is_none() && !mandatory {
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
    iter: std::iter::Take<I>,
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
    max_width: u32,
    text: &'a str,
    f: impl for<'b> FnOnce(Lines<'a, 'b, F>) -> R,
) -> R {
    LineGenerator::new(font, text, |generator| {
        f(Lines {
            max_width,
            generator,
        })
    })
}

impl<'a, 'b, F: Font> Iterator for Lines<'a, 'b, F> {
    type Item = Line<F::Shaped<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.generator.next(self.max_width, false)
    }
}

pub struct LineGenerator<'a, 'b, F: Font> {
    font: &'a F,
    // text: &'a str,
    pieces: Peekable<Pieces<'a, 'b, F::Shaped<'a>>>,
}

impl<'a, F: Font> LineGenerator<'a, 'static, F> {
    pub fn new<R>(
        font: &'a F,
        text: &'a str,
        f: impl for<'b> FnOnce(LineGenerator<'a, 'b, F>) -> R,
    ) -> R {
        Pieces::new(font, text, |pieces| {
            f(LineGenerator {
                font,
                // text,
                pieces: pieces.peekable(),
            })
        })
    }
}

impl<'a, 'b, F: Font> LineGenerator<'a, 'b, F> {
    pub fn next(&mut self, max_width: u32, incomplete: bool) -> Option<Line<F::Shaped<'a>>> {
        let Some(start) = self.pieces.peek().map(|p| p.shaped_start.clone()) else {
            return None;
        };

        let mut glyph_count = 0;
        let mut current_width = 0;
        let mut current_width_whitespace = 0;

        while let Some(piece) = self.pieces.peek() {
            // If current_width is zero we have to place the piece on this line, because adding
            // another line would not help.
            if (current_width > 0 || incomplete)
                && current_width + current_width_whitespace + piece.width > max_width
            {
                break;
            }

            glyph_count += piece.glyph_count;

            current_width += current_width_whitespace + piece.width;
            current_width_whitespace = piece.trailing_whitespace_width;

            let mandatory_break_after = piece.mandatory_break_after;

            self.pieces.next();

            if mandatory_break_after {
                break;
            }
        }

        Some(Line {
            empty: glyph_count == 0,
            width: current_width,
            trailing_whitespace_width: current_width_whitespace,
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
                    x_advance: if matches!(c, '\n' | '\u{00ad}') { 0 } else { 1 },
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

        fn shape<'a>(&'a self, text: &'a str) -> Self::Shaped<'a> {
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

        Pieces::new(&FakeFont, text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(&pieces, &[("", 0, 0, false)]);
        });
    }

    #[test]
    fn test_pieces_one() {
        let text = "abcde";

        Pieces::new(&FakeFont, text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(&pieces, &[("abcde", 5, 0, false)]);
        });
    }

    #[test]
    fn test_pieces_two() {
        let text = "deadbeef defaced";

        Pieces::new(&FakeFont, text, |pieces| {
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

        Pieces::new(&FakeFont, text, |pieces| {
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

        Pieces::new(&FakeFont, text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(&pieces, &[("\n", 0, 0, true), ("", 0, 0, false)]);
        });
    }

    #[test]
    fn test_pieces_surrounded_newline() {
        let text = "abc\ndef";

        Pieces::new(&FakeFont, text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(&pieces, &[("abc\n", 3, 0, true), ("def", 3, 0, false)]);
        });
    }

    #[test]
    fn test_pieces_newline_at_start() {
        let text = "\nabc def";

        Pieces::new(&FakeFont, text, |pieces| {
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

        Pieces::new(&FakeFont, text, |pieces| {
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

        Pieces::new(&FakeFont, text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(&pieces, &[("        ", 0, 8, false)]);
        });
    }

    #[test]
    fn test_pieces_mixed_whitespace() {
        let text = "    abc    \ndef  the\tjflkdsa";

        Pieces::new(&FakeFont, text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(
                &pieces,
                &[
                    ("    ", 0, 4, false),
                    ("abc    \n", 7, 0, true),
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

        LineGenerator::new(&FakeFont, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next(16, false).map(&collect), Some(""));
            assert_eq!(generator.next(16, false).map(&collect), None);
        });
    }

    #[test]
    fn test_text_flow() {
        let text = "Amet consequatur facilis necessitatibus sed quia numquam reiciendis. \
                Id impedit quo quaerat enim amet. ";

        LineGenerator::new(&FakeFont, text, |mut generator| {
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

        LineGenerator::new(&FakeFont, text, |mut generator| {
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

        LineGenerator::new(&FakeFont, text, |mut generator| {
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
        LineGenerator::new(&FakeFont, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(
                generator.next(16, false).map(&collect),
                Some("Id impedit quo \n")
            );
            assert_eq!(generator.next(16, false).map(&collect), Some("quaerat "));

            // The old line-breaker used to not count the spaces before the newline. It's sort of
            // unclear what's better here.
            assert_eq!(
                generator.next(16, false).map(&collect),
                Some("enimmmmm    \n")
            );
            assert_eq!(generator.next(16, false).map(&collect), Some("amet."));
            assert_eq!(generator.next(16, false).map(&collect), None);
        });
    }

    #[test]
    fn test_newline() {
        let text = "\n";

        LineGenerator::new(&FakeFont, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next(16, false).map(&collect), Some("\n"));
            assert_eq!(generator.next(16, false).map(&collect), Some(""));
            assert_eq!(generator.next(16, false).map(&collect), None);
        });
    }

    #[test]
    fn test_just_spaces() {
        let text = "  ";

        LineGenerator::new(&FakeFont, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next(16, false).map(&collect), Some("  "));
            assert_eq!(generator.next(16, false).map(&collect), None);
        });
    }

    #[test]
    fn test_word_longer_than_line() {
        let text = "Averylongword";

        LineGenerator::new(&FakeFont, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(
                generator.next(8, false).map(&collect),
                Some("Averylongword")
            );
            assert_eq!(generator.next(8, false).map(&collect), None);
        });

        let text = "Averylongword test.";

        LineGenerator::new(&FakeFont, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(
                generator.next(8, false).map(&collect),
                Some("Averylongword ")
            );
            assert_eq!(generator.next(8, false).map(&collect), Some("test."));
            assert_eq!(generator.next(8, false).map(&collect), None);
        });

        let text = "A verylongword test.";

        LineGenerator::new(&FakeFont, text, |mut generator| {
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

        LineGenerator::new(&FakeFont, text, |mut generator| {
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

        LineGenerator::new(&FakeFont, text, |mut generator| {
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

        LineGenerator::new(&FakeFont, text, |mut generator| {
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

        LineGenerator::new(&FakeFont, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next(7, false).map(&collect), Some("A-very-"));
            assert_eq!(generator.next(7, false).map(&collect), Some("long-"));
            assert_eq!(generator.next(7, false).map(&collect), Some("word"));
            assert_eq!(generator.next(7, false).map(&collect), None);
        });

        let text = "A-very -long-word";

        LineGenerator::new(&FakeFont, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next(7, false).map(&collect), Some("A-very "));
            assert_eq!(generator.next(7, false).map(&collect), Some("-long-"));
            assert_eq!(generator.next(7, false).map(&collect), Some("word"));
            assert_eq!(generator.next(7, false).map(&collect), None);
        });

        let text = "A-very--long-word";

        LineGenerator::new(&FakeFont, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next(7, false).map(&collect), Some("A-"));
            assert_eq!(generator.next(7, false).map(&collect), Some("very--"));
            assert_eq!(generator.next(7, false).map(&collect), Some("long-"));
            assert_eq!(generator.next(7, false).map(&collect), Some("word"));
            assert_eq!(generator.next(7, false).map(&collect), None);
        });
    }
}
