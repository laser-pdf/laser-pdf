use std::{iter::Peekable, sync::LazyLock};

use icu_properties::LineBreak;
use icu_segmenter::LineBreakIteratorUtf8;

use crate::fonts::{Font, GeneralMetrics, ShapedGlyph};

pub fn pieces<'a, F: Font, R>(
    font: &'a F,
    character_spacing: f32,
    word_spacing: f32,
    text: &'a str,
    size: f32,
    color: u32,
    f: impl for<'b, 'c> FnOnce(Pieces<'a, 'b, 'c, F>) -> R,
) -> R {
    LINE_SEGMENTER.with(|segmenter| {
        let shaped_hyphen = font.shape(super::HYPHEN, 0., 0.).next().unwrap();

        let shaped = super::shaping::shape(
            font,
            font.fallback_fonts(),
            text,
            character_spacing / size,
            word_spacing / size,
        );

        // let shaped = font.shape(text, character_spacing, word_spacing);
        let segments = segmenter.segment_str(text).peekable();

        f(Pieces {
            current: Some(0),
            text,
            shaped: shaped.iter(),
            segments,
            shaped_hyphen,
            size,
            color,
            main_font: font,
            main_font_metrics: font.general_metrics(),
        })
    })
}

thread_local! {
    static LINE_SEGMENTER: icu_segmenter::LineSegmenter = icu_segmenter::LineSegmenter::new_auto();
}

static LINE_BREAK_MAP: LazyLock<
    icu_properties::maps::CodePointMapDataBorrowed<'static, icu_properties::LineBreak>,
> = LazyLock::new(icu_properties::maps::line_break);

pub struct Piece<'a, F> {
    pub text: &'a str,
    pub shaped: Vec<(&'a F, ShapedGlyph)>,
    pub width: f32,
    pub height_above_baseline: f32,
    pub height_below_baseline: f32,
    pub trailing_whitespace_width: f32,

    /// Only applies when the piece is at the end of the line. Otherwise, it will not be counted
    /// towards the width and not displayed.
    pub trailing_hyphen: Option<(f32, &'a F, ShapedGlyph)>,
    pub mandatory_break_after: bool,
    pub glyph_count: usize,
    pub empty: bool,
    pub size: f32,
    pub color: u32,
}

pub struct Pieces<'a, 'b, 'c, F> {
    current: Option<usize>,
    text: &'a str,
    // current_index: usize,
    // shaped: &'c [(&'a F, ShapedGlyph)],
    shaped: std::slice::Iter<'c, (&'a F, ShapedGlyph)>,
    segments: Peekable<LineBreakIteratorUtf8<'b, 'a>>,
    main_font: &'a F,
    main_font_metrics: GeneralMetrics,
    shaped_hyphen: ShapedGlyph,
    size: f32,
    color: u32,
}

impl<'a, 'b, 'c, F: Font> Iterator for Pieces<'a, 'b, 'c, F> {
    type Item = Piece<'a, F>;

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

                if next.1.text_range.end >= segment {
                    done = true;
                }

                Some(next)
            }
        })
        .peekable();

        let mut width = 0.;
        let mut whitespace_width = 0.;
        let mut glyph_count = 0;
        let mut mandatory_break_after = false;

        // A line and its the pieces is always at least as high as the main font. Otherwise empty
        // lines pieces would have no height. We could special case the empty line case, but that
        // would lead to the the possibility of an empty line being higher than a line that only has
        // glyphs from a fallback font.
        let mut height_above_baseline = self.main_font_metrics.height_above_baseline;
        let mut height_below_baseline = self.main_font_metrics.height_below_baseline;

        while let Some(glyph) = iter.next() {
            glyph_count += 1;

            // A space at the end of a line doesn't count towards the width.
            if matches!(
                &self.text[glyph.1.text_range.clone()],
                " " | "\u{00A0}" | "　"
            ) {
                whitespace_width += glyph.1.x_advance;
            } else if matches!(
                self.text[glyph.1.text_range.clone()]
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
                whitespace_width = 0.;
                width += glyph.1.x_advance;
            }

            let metrics = glyph.0.general_metrics();

            height_above_baseline = height_above_baseline.max(metrics.height_above_baseline);
            height_below_baseline = height_below_baseline.max(metrics.height_below_baseline);
        }

        let text = &self.text[current..segment];

        let trailing_hyphen = text
            .ends_with('\u{00AD}')
            .then_some((self.main_font, self.shaped_hyphen.clone()));

        let piece = Piece {
            text,
            shaped: self
                .shaped
                .by_ref()
                .take(glyph_count)
                .map(|&(f, ref g)| {
                    (
                        f,
                        ShapedGlyph {
                            text_range: (g.text_range.start - current)
                                ..(g.text_range.end - current),
                            ..g.clone()
                        },
                    )
                })
                .collect(),
            width: width * self.size,
            height_above_baseline: height_above_baseline * self.size,
            height_below_baseline: height_below_baseline * self.size,
            trailing_whitespace_width: whitespace_width * self.size,
            trailing_hyphen: trailing_hyphen
                .map(|(font, glyph)| (glyph.x_advance * self.size, font, glyph)),
            mandatory_break_after,
            glyph_count,

            // TODO: This might not work for \r\n, but that depends on the shaping. We should
            // proabably find a way to filter out newlines entirely so that they don't show up after
            // line breaking (and maybe also don't get shaped?).
            empty: glyph_count == 0 || (glyph_count == 1 && mandatory_break_after),

            size: self.size,
            color: self.color,
        };

        self.current = self.current.and(Some(segment));
        self.shaped = shaped;

        if self.segments.peek().is_none() && !mandatory_break_after {
            self.current = None;
        }

        Some(piece)
    }
}

#[cfg(test)]
mod tests {
    use crate::{fonts::ShapedGlyph, text::pieces::Piece};

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
                    x_advance_font: if matches!(c, '\u{00ad}') { 0. } else { 1. },
                    x_advance: if matches!(c, '\u{00ad}') { 0. } else { 1. },
                    x_offset: 0.,
                    y_offset: 0.,
                    y_advance: 0.,
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

        fn shape<'a>(&'a self, text: &'a str, _: f32, _: f32) -> Self::Shaped<'a> {
            FakeShaped {
                inner: text.char_indices(),
            }
        }

        fn encode(&self, _: &mut crate::Pdf, _: u32, _: &str) -> crate::fonts::EncodedGlyph {
            unimplemented!()
        }

        fn resource_name(&self) -> pdf_writer::Name<'_> {
            unimplemented!()
        }

        fn general_metrics(&self) -> crate::fonts::GeneralMetrics {
            crate::fonts::GeneralMetrics {
                height_above_baseline: 0.5,
                height_below_baseline: 0.5,
            }
        }

        fn fallback_fonts(&self) -> &[Self] {
            &[]
        }
    }

    fn collect_piece<'a>(text: &'a str, piece: Piece<'a, FakeFont>) -> (&'a str, f32, f32, bool) {
        let line = piece.shaped.into_iter();

        let by_range = {
            let mut line = line.clone();
            if let Some(first) = line.next() {
                let last = line
                    .last()
                    .map(|l| l.1.text_range.end)
                    .unwrap_or(first.1.text_range.end);

                &text[first.1.text_range.start..last]
            } else {
                ""
            }
        };

        let mut text = String::new();

        for glyph in line {
            text.push(glyph.1.glyph_id as u8 as char);
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
    fn test_empty() {
        let text = "";

        pieces(&FakeFont, 0., 0., text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(&pieces, &[("", 0., 0., false)]);
        });
    }

    #[test]
    fn test_one() {
        let text = "abcde";

        pieces(&FakeFont, 0., 0., text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(&pieces, &[("abcde", 5., 0., false)]);
        });
    }

    #[test]
    fn test_two() {
        let text = "deadbeef defaced";

        pieces(&FakeFont, 0., 0., text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(
                &pieces,
                &[("deadbeef ", 8., 1., false), ("defaced", 7., 0., false)]
            );
        });
    }

    #[test]
    fn test_three() {
        let text = "deadbeef defaced fart";

        pieces(&FakeFont, 0., 0., text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(
                &pieces,
                &[
                    ("deadbeef ", 8., 1., false),
                    ("defaced ", 7., 1., false),
                    ("fart", 4., 0., false)
                ],
            );
        });
    }

    #[test]
    fn test_just_newline() {
        let text = "\n";

        pieces(&FakeFont, 0., 0., text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(&pieces, &[("\n", 0., 0., true), ("", 0., 0., false)]);
        });
    }

    #[test]
    fn test_surrounded_newline() {
        let text = "abc\ndef";

        pieces(&FakeFont, 0., 0., text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(&pieces, &[("abc\n", 3., 0., true), ("def", 3., 0., false)]);
        });
    }

    #[test]
    fn test_newline_at_start() {
        let text = "\nabc def";

        pieces(&FakeFont, 0., 0., text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(
                &pieces,
                &[
                    ("\n", 0., 0., true),
                    ("abc ", 3., 1., false),
                    ("def", 3., 0., false),
                ]
            );
        });
    }

    #[test]
    fn test_trailing_newline() {
        let text = "abc def\n";

        pieces(&FakeFont, 0., 0., text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(
                &pieces,
                &[
                    ("abc ", 3., 1., false),
                    ("def\n", 3., 0., true),
                    ("", 0., 0., false),
                ]
            );
        });
    }

    #[test]
    fn test_just_spaces() {
        let text = "        ";

        pieces(&FakeFont, 0., 0., text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(&pieces, &[("        ", 0., 8., false)]);
        });
    }

    #[test]
    fn test_mixed_whitespace() {
        let text = "    abc    \ndef  the\tjflkdsa";

        pieces(&FakeFont, 0., 0., text, |pieces| {
            let pieces: Vec<_> = pieces.map(|p| collect_piece(text, p)).collect();

            assert_eq!(
                &pieces,
                &[
                    ("    ", 0., 4., false),
                    // It's somewhat unclear whether the trailing spaces should count toward the
                    // width here.
                    ("abc    \n", 3., 4., true),
                    ("def  ", 3., 2., false),
                    ("the\t", 4., 0., false),
                    ("jflkdsa", 7., 0., false),
                ],
            );
        });
    }
}
