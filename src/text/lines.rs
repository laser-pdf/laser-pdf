use std::iter::Peekable;

use crate::{
    fonts::{Font, ShapedGlyph},
    text::pieces::{Piece, pieces},
};

// TODO: remove function
pub fn lines<'a, R, F: Font>(
    font: &'a F,
    character_spacing: f32,
    word_spacing: f32,
    max_width: f32,
    text: &'a str,
    size: f32,
    color: u32,
    f: impl for<'b> FnOnce(Lines<'b, F, std::slice::Iter<'b, Piece<'b, F>>>) -> R,
) -> R {
    pieces(
        font,
        character_spacing,
        word_spacing,
        0.,
        text,
        size,
        color,
        |pieces| {
            let pieces: Vec<_> = pieces.collect();
            let pieces = pieces.iter().peekable();

            f(Lines {
                max_width,
                consider_last_line_trailing_whitespace: true, // TODO: does this make any sense?
                pieces: PiecesCursor {
                    iter: pieces,
                    current: None,
                },
            })
        },
    )
}

pub fn lines_from_pieces<'a, F: Font>(
    pieces: &'a [Piece<'a, F>],
    max_width: f32,
) -> Lines<'a, F, std::slice::Iter<'a, Piece<'a, F>>> {
    Lines {
        max_width,
        consider_last_line_trailing_whitespace: true,
        pieces: PiecesCursor {
            iter: pieces.iter().peekable(),
            current: None,
        },
    }
}

pub struct LineGlyph<'a, F> {
    pub font: &'a F,
    pub text: &'a str,
    pub shaped_glyph: ShapedGlyph,
    pub size: f32,
    pub color: u32,
}

pub struct Line<'a, F, P: Iterator<Item = &'a Piece<'a, F>>> {
    pub empty: bool,
    pub width: f32,
    pub trailing_whitespace_width: f32,
    pub height_above_baseline: f32,
    pub height_below_baseline: f32,
    iter: std::iter::Chain<
        std::iter::FlatMap<
            std::iter::Take<PiecesCursor<'a, Piece<'a, F>, P>>,
            std::iter::Map<
                std::iter::Zip<
                    std::slice::Iter<'a, (&'a F, ShapedGlyph)>,
                    std::iter::Repeat<(&'a str, f32, u32)>,
                >,
                for<'c> fn((&'c (&'c F, ShapedGlyph), (&'c str, f32, u32))) -> LineGlyph<'c, F>,
            >,
            for<'b> fn(
                &'b Piece<'b, F>,
            ) -> std::iter::Map<
                std::iter::Zip<
                    std::slice::Iter<'b, (&'b F, ShapedGlyph)>,
                    std::iter::Repeat<(&'b str, f32, u32)>,
                >,
                for<'c> fn((&'c (&'c F, ShapedGlyph), (&'c str, f32, u32))) -> LineGlyph<'c, F>,
            >,
        >,
        std::option::IntoIter<LineGlyph<'a, F>>,
    >,
}

impl<'a, F, P: Iterator<Item = &'a Piece<'a, F>>> std::fmt::Debug for Line<'a, F, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Line")
            .field("empty", &self.empty)
            .field("width", &self.width)
            .field("trailing_whitespace_width", &self.trailing_whitespace_width)
            .field(
                "iter",
                &(), // &self.iter.clone().map(|a| a.1.clone()).collect::<Vec<_>>(),
            )
            .finish()
    }
}

// impl<P: Iterator + Clone> Clone for Line<P> {
//     fn clone(&self) -> Self {
//         Self {
//             empty: self.empty.clone(),
//             width: self.width.clone(),
//             trailing_whitespace_width: self.trailing_whitespace_width.clone(),
//             height_above_baseline: self.height_above_baseline.clone(),
//             height_below_baseline: self.height_below_baseline.clone(),
//             shaped_hyphen: self.shaped_hyphen.clone(),
//             iter: self.iter.clone(),
//             trailing_hyphen_width: self.trailing_hyphen_width.clone(),
//         }
//     }
// }

impl<'a, F, P: Iterator<Item = &'a Piece<'a, F>>> Iterator for Line<'a, F, P> {
    type Item = LineGlyph<'a, F>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

struct PiecesCursor<'a, T, I: Iterator<Item = &'a T>> {
    iter: Peekable<I>,
    current: Option<&'a T>,
}

/// A manual impl of `Clone` because `T` doesn't need to be `Clone`.
impl<'a, T, I: Iterator<Item = &'a T> + Clone> Clone for PiecesCursor<'a, T, I> {
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            current: self.current.clone(),
        }
    }
}

impl<'a, T, I: Iterator<Item = &'a T>> PiecesCursor<'a, T, I> {
    // Needs to be one call to avoid lifetime problems.
    fn current(&mut self) -> Option<(&'a T, bool)> {
        if self.current.is_none() {
            self.current = self.iter.next();
        }

        self.current.map(|c| (c, self.iter.peek().is_some()))
    }

    fn advance(&mut self) {
        if self.current.is_some() {
            self.current = None;
        } else {
            self.current = self.iter.next();
        }
    }
}

impl<'a, T, I: Iterator<Item = &'a T>> Iterator for PiecesCursor<'a, T, I> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_some() {
            self.current.take()
        } else {
            self.iter.next()
        }
    }
}

pub struct Lines<'a, F: Font + 'a, P: Iterator<Item = &'a Piece<'a, F>>> {
    max_width: f32,
    consider_last_line_trailing_whitespace: bool,
    pieces: PiecesCursor<'a, Piece<'a, F>, P>,
}

impl<'a, F: Font + 'a, P: Iterator<Item = &'a Piece<'a, F>> + Clone> Iterator for Lines<'a, F, P> {
    type Item = Line<'a, F, P>;

    fn next(&mut self) -> Option<Line<'a, F, P>> {
        // No more pieces, no more lines.
        if self.pieces.current().is_none() {
            return None;
        }

        let start = self.pieces.clone();

        let max_width = self.max_width;
        let consider_last_line_trailing_whitespace = self.consider_last_line_trailing_whitespace;

        let mut empty = true;
        let mut piece_count = 0;
        let mut current_width = 0.;
        let mut current_width_whitespace = 0.;

        let mut trailing_hyphen = None;
        let mut last_piece_size = 0.;
        let mut last_piece_color = 0;

        let mut height_above_baseline: f32 = 0.;
        let mut height_below_baseline: f32 = 0.;

        while let Some((piece, has_next)) = self.pieces.current() {
            // If current_width is zero we have to place the piece on this line, because adding
            // another line would not help.
            if (current_width > 0.)
                && current_width
                    + current_width_whitespace
                    + piece.width
                    + piece.trailing_hyphen.as_ref().map_or(0., |h| h.0)
                    + (!has_next && consider_last_line_trailing_whitespace)
                        .then_some(piece.trailing_whitespace_width)
                        .unwrap_or(0.)
                    > max_width
            {
                break;
            }

            empty = empty && piece.empty;
            piece_count += 1;

            current_width += current_width_whitespace + piece.width;
            current_width_whitespace = piece.trailing_whitespace_width;
            trailing_hyphen = piece.trailing_hyphen.clone();
            last_piece_size = piece.size;
            last_piece_color = piece.color;

            height_above_baseline = height_above_baseline.max(piece.height_above_baseline);
            height_below_baseline = height_below_baseline.max(piece.height_below_baseline);

            let mandatory_break_after = piece.mandatory_break_after;

            self.pieces.advance();

            if mandatory_break_after {
                break;
            }
        }

        fn map<'c, F: Font>(
            (&(font, ref glyph), (text, size, color)): (
                &'c (&'c F, ShapedGlyph),
                (&'c str, f32, u32),
            ),
        ) -> LineGlyph<'c, F> {
            LineGlyph {
                font,
                text: &text[glyph.text_range.clone()],
                shaped_glyph: glyph.clone(),
                size,
                color,
            }
        }

        fn piece_glyphs<'a, F: Font>(
            piece: &'a Piece<'a, F>,
        ) -> std::iter::Map<
            std::iter::Zip<
                std::slice::Iter<'a, (&'a F, ShapedGlyph)>,
                std::iter::Repeat<(&'a str, f32, u32)>,
            >,
            for<'c> fn((&'c (&'c F, ShapedGlyph), (&'c str, f32, u32))) -> LineGlyph<'c, F>,
        > {
            piece
                .shaped
                .iter()
                .zip(std::iter::repeat((piece.text, piece.size, piece.color)))
                .map(map)
        }

        Some(Line {
            empty,
            width: current_width + trailing_hyphen.as_ref().map_or(0., |h| h.0),
            trailing_whitespace_width: current_width_whitespace,
            height_above_baseline,
            height_below_baseline,
            iter: start
                .take(piece_count)
                .flat_map(
                    piece_glyphs
                        as for<'b> fn(
                            &'b Piece<'b, F>,
                        ) -> std::iter::Map<
                            std::iter::Zip<
                                std::slice::Iter<'b, (&'b F, ShapedGlyph)>,
                                std::iter::Repeat<(&'b str, f32, u32)>,
                            >,
                            for<'c> fn(
                                (&'c (&'c F, ShapedGlyph), (&'c str, f32, u32)),
                            ) -> LineGlyph<'c, F>,
                        >,
                )
                .chain(
                    trailing_hyphen
                        .map(|h| LineGlyph {
                            font: h.1,
                            text: super::HYPHEN,
                            shaped_glyph: h.2,
                            size: last_piece_size,
                            color: last_piece_color,
                        })
                        .into_iter(),
                ),
        })
    }
}

#[cfg(test)]
mod tests {
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

    fn collect<'a>(text: &'a str) -> impl Fn(Line<FakeFont>) -> &'a str {
        |line| {
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
            by_range
        }
    }

    fn assert_width(width: f32) -> impl Fn(&Line<FakeFont>) {
        move |l| {
            assert_eq!(l.width, width);
        }
    }

    #[test]
    fn test_empty_string() {
        let text = "";

        lines(&FakeFont, 0., 0., 16., text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next().map(&collect), Some(""));
            assert_eq!(generator.next().map(&collect), None);
        });
    }

    #[test]
    fn test_text_flow() {
        let text = "Amet consequatur facilis necessitatibus sed quia numquam reiciendis. \
                Id impedit quo quaerat enim amet. ";

        lines(&FakeFont, 0., 0., 16., text, |mut generator| {
            let collect = collect(text);

            assert_eq!(
                generator.next().inspect(assert_width(16.)).map(&collect),
                Some("Amet consequatur ")
            );

            assert_eq!(
                generator.next().inspect(assert_width(7.)).map(&collect),
                Some("facilis ")
            );
            assert_eq!(generator.next().map(&collect), Some("necessitatibus "));
            assert_eq!(generator.next().map(&collect), Some("sed quia numquam "));
            assert_eq!(generator.next().map(&collect), Some("reiciendis. Id "));
            assert_eq!(generator.next().map(&collect), Some("impedit quo "));
            assert_eq!(generator.next().map(&collect), Some("quaerat enim "));
            assert_eq!(
                generator.next().inspect(assert_width(5.)).map(&collect),
                Some("amet. ")
            );
            assert_eq!(generator.next().map(&collect), None);

            // Make sure it's sealed.
            assert_eq!(generator.next().map(&collect), None);
            assert_eq!(generator.next().map(&collect), None);
        });
    }

    #[test]
    fn test_text_after_newline() {
        let text = "\nthe the the";

        lines(&FakeFont, 0., 0., 4., text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next().map(&collect), Some("\n"));
            assert_eq!(generator.next().map(&collect), Some("the "));
            assert_eq!(generator.next().map(&collect), Some("the "));
            assert_eq!(generator.next().map(&collect), Some("the"));
            assert_eq!(generator.next().map(&collect), None);
        });
    }

    #[test]
    fn test_trailing_whitespace() {
        let text = "Id impedit quo quaerat enim amet.                  ";

        lines(&FakeFont, 0., 0., 16., text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next().map(&collect), Some("Id impedit quo "));
            assert_eq!(generator.next().map(&collect), Some("quaerat enim "));

            // it's unclear whether any other behavior would be better here
            assert_eq!(
                generator.next().inspect(assert_width(5.)).map(&collect),
                Some("amet.                  ")
            );
            assert_eq!(generator.next().map(&collect), None);
        });
    }

    #[test]
    fn test_pre_newline_whitespace() {
        let text = "Id impedit quo \nquaerat enimmmmm    \namet.";
        lines(&FakeFont, 0., 0., 16., text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next().map(&collect), Some("Id impedit quo \n"));
            // It seems unclear what the intent would be in such a case.
            assert_eq!(
                generator.next().map(&collect),
                Some("quaerat enimmmmm    \n")
            );
            assert_eq!(generator.next().map(&collect), Some("amet."));
            assert_eq!(generator.next().map(&collect), None);
        });
    }

    #[test]
    fn test_newline() {
        let text = "\n";

        lines(&FakeFont, 0., 0., 16., text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next().map(&collect), Some("\n"));
            assert_eq!(generator.next().map(&collect), Some(""));
            assert_eq!(generator.next().map(&collect), None);
        });
    }

    #[test]
    fn test_just_spaces() {
        let text = "  ";

        lines(&FakeFont, 0., 0., 16., text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next().map(&collect), Some("  "));
            assert_eq!(generator.next().map(&collect), None);
        });
    }

    #[test]
    fn test_word_longer_than_line() {
        let text = "Averylongword";

        lines(&FakeFont, 0., 0., 8., text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next().map(&collect), Some("Averylongword"));
            assert_eq!(generator.next().map(&collect), None);
        });

        let text = "Averylongword test.";

        lines(&FakeFont, 0., 0., 8., text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next().map(&collect), Some("Averylongword "));
            assert_eq!(generator.next().map(&collect), Some("test."));
            assert_eq!(generator.next().map(&collect), None);
        });

        let text = "A verylongword test.";

        lines(&FakeFont, 0., 0., 8., text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next().map(&collect), Some("A "));
            assert_eq!(generator.next().map(&collect), Some("verylongword "));
            assert_eq!(generator.next().map(&collect), Some("test."));
            assert_eq!(generator.next().map(&collect), None);
        });
    }

    #[test]
    fn test_soft_hyphens() {
        let text = "A\u{00ad}very\u{00ad}long\u{00ad}word";

        lines(&FakeFont, 0., 0., 7., text, |mut generator| {
            let collect = collect(text);

            assert_eq!(
                generator.next().map(&collect),
                Some("A\u{00ad}very\u{00ad}"),
            );
            assert_eq!(generator.next().map(&collect), Some("long\u{00ad}"),);
            assert_eq!(generator.next().map(&collect), Some("word"));
            assert_eq!(generator.next().map(&collect), None);
        });

        let text = "A\u{00ad}very \u{00ad}long\u{00ad}word";

        lines(&FakeFont, 0., 0., 7., text, |mut generator| {
            let collect = collect(text);

            assert_eq!(
                generator.next().map(&collect),
                // The old line breaker used to not split at a soft hypen that was at the start of
                // a word. But since the segmenter splits there we treat it as a separate piece now.
                Some("A\u{00ad}very \u{00ad}"),
            );
            assert_eq!(generator.next().map(&collect), Some("long\u{00ad}"),);
            assert_eq!(generator.next().map(&collect), Some("word"));
            assert_eq!(generator.next().map(&collect), None);
        });

        let text = "A\u{00ad}very\u{00ad}\u{00ad}long\u{00ad}word";

        lines(&FakeFont, 0., 0., 7., text, |mut generator| {
            let collect = collect(text);

            assert_eq!(
                generator.next().map(&collect),
                Some("A\u{00ad}very\u{00ad}\u{00ad}"),
            );
            assert_eq!(generator.next().map(&collect), Some("long\u{00ad}"),);
            assert_eq!(generator.next().map(&collect), Some("word"));
            assert_eq!(generator.next().map(&collect), None);
        });
    }

    #[test]
    fn test_hard_hyphens() {
        let text = "A-very-long-word";

        lines(&FakeFont, 0., 0., 7., text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next().map(&collect), Some("A-very-"));
            assert_eq!(generator.next().map(&collect), Some("long-"));
            assert_eq!(generator.next().map(&collect), Some("word"));
            assert_eq!(generator.next().map(&collect), None);
        });

        let text = "A-very -long-word";

        lines(&FakeFont, 0., 0., 7., text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next().map(&collect), Some("A-very "));
            assert_eq!(generator.next().map(&collect), Some("-long-"));
            assert_eq!(generator.next().map(&collect), Some("word"));
            assert_eq!(generator.next().map(&collect), None);
        });

        let text = "A-very--long-word";

        lines(&FakeFont, 0., 0., 7., text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next().map(&collect), Some("A-"));
            assert_eq!(generator.next().map(&collect), Some("very--"));
            assert_eq!(generator.next().map(&collect), Some("long-"));
            assert_eq!(generator.next().map(&collect), Some("word"));
            assert_eq!(generator.next().map(&collect), None);
        });
    }
        });
    }
}
