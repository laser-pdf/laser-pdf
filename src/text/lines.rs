use std::iter::Peekable;

use crate::{
    fonts::{Font, ShapedGlyph},
    text::pieces::{Piece, Pieces, pieces},
};

pub fn lines<'a, R, F: Font>(
    font: &'a F,
    character_spacing: i32,
    word_spacing: i32,
    max_width: u32,
    text: &'a str,
    f: impl for<'b, 'c> FnOnce(Lines<'a, 'b, 'c, F>) -> R,
) -> R {
    line_generator(
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

pub fn line_generator<'a, F: Font, R>(
    font: &'a F,
    character_spacing: i32,
    word_spacing: i32,
    consider_last_line_trailing_whitespace: bool,
    text: &'a str,
    f: impl for<'b, 'c> FnOnce(LineGenerator<'a, 'b, 'c, F>) -> R,
) -> R {
    pieces(font, character_spacing, word_spacing, text, |pieces| {
        f(LineGenerator {
            consider_last_line_trailing_whitespace,
            shaped_hyphen: pieces.shaped_hyphen.clone(),
            pieces: pieces.peekable(),
            current: None,
        })
    })
}

pub struct Line<'a, 'c, F> {
    pub empty: bool,
    pub width: u32,
    pub trailing_whitespace_width: u32,
    pub shaped_hyphen: ShapedGlyph,
    iter: std::iter::Take<std::slice::Iter<'c, (&'a F, ShapedGlyph)>>,
    pub trailing_hyphen_width: u32,
}

impl<'a, 'c, F> Clone for Line<'a, 'c, F> {
    fn clone(&self) -> Self {
        Self {
            empty: self.empty.clone(),
            width: self.width.clone(),
            trailing_whitespace_width: self.trailing_whitespace_width.clone(),
            shaped_hyphen: self.shaped_hyphen.clone(),
            iter: self.iter.clone(),
            trailing_hyphen_width: self.trailing_hyphen_width.clone(),
        }
    }
}

impl<'a, 'c, F: Font> Iterator for Line<'a, 'c, F> {
    type Item = &'c (&'a F, ShapedGlyph);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub struct Lines<'a, 'b, 'c, F: Font> {
    max_width: u32,
    generator: LineGenerator<'a, 'b, 'c, F>,
}

impl<'a, 'b, 'c, F: Font> Iterator for Lines<'a, 'b, 'c, F> {
    type Item = Line<'a, 'c, F>;

    fn next(&mut self) -> Option<Self::Item> {
        self.generator.next(self.max_width, false)
    }
}

pub struct LineGenerator<'a, 'b, 'c, F: Font + 'a> {
    consider_last_line_trailing_whitespace: bool,
    pieces: Peekable<Pieces<'a, 'b, 'c, F>>,
    current: Option<Piece<'a, 'c, F>>,
    shaped_hyphen: ShapedGlyph,
}

impl<'a, 'b, 'c, F: Font> LineGenerator<'a, 'b, 'c, F> {
    // Needs to be one call to avoid lifetime problems.
    fn current(&mut self) -> Option<(&Piece<'a, 'c, F>, bool)> {
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

    pub fn next(&mut self, max_width: u32, incomplete: bool) -> Option<Line<'a, 'c, F>> {
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

        fn resource_name(&self) -> pdf_writer::Name<'_> {
            unimplemented!()
        }

        fn general_metrics(&self) -> crate::fonts::GeneralMetrics {
            unimplemented!()
        }

        fn units_per_em(&self) -> u16 {
            1
        }

        fn fallback_fonts(&self) -> impl Iterator<Item = &Self> + Clone {
            std::iter::empty()
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

    fn assert_width(width: u32) -> impl Fn(&Line<FakeFont>) {
        move |l| {
            assert_eq!(l.width, width);
        }
    }

    #[test]
    fn test_empty_string() {
        let text = "";

        line_generator(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next(16, false).map(&collect), Some(""));
            assert_eq!(generator.next(16, false).map(&collect), None);
        });
    }

    #[test]
    fn test_text_flow() {
        let text = "Amet consequatur facilis necessitatibus sed quia numquam reiciendis. \
                Id impedit quo quaerat enim amet. ";

        line_generator(&FakeFont, 0, 0, true, text, |mut generator| {
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

        line_generator(&FakeFont, 0, 0, true, text, |mut generator| {
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

        line_generator(&FakeFont, 0, 0, true, text, |mut generator| {
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
        line_generator(&FakeFont, 0, 0, true, text, |mut generator| {
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

        line_generator(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next(16, false).map(&collect), Some("\n"));
            assert_eq!(generator.next(16, false).map(&collect), Some(""));
            assert_eq!(generator.next(16, false).map(&collect), None);
        });
    }

    #[test]
    fn test_just_spaces() {
        let text = "  ";

        line_generator(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next(16, false).map(&collect), Some("  "));
            assert_eq!(generator.next(16, false).map(&collect), None);
        });
    }

    #[test]
    fn test_word_longer_than_line() {
        let text = "Averylongword";

        line_generator(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(
                generator.next(8, false).map(&collect),
                Some("Averylongword")
            );
            assert_eq!(generator.next(8, false).map(&collect), None);
        });

        let text = "Averylongword test.";

        line_generator(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(
                generator.next(8, false).map(&collect),
                Some("Averylongword ")
            );
            assert_eq!(generator.next(8, false).map(&collect), Some("test."));
            assert_eq!(generator.next(8, false).map(&collect), None);
        });

        let text = "A verylongword test.";

        line_generator(&FakeFont, 0, 0, true, text, |mut generator| {
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

        line_generator(&FakeFont, 0, 0, true, text, |mut generator| {
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

        line_generator(&FakeFont, 0, 0, true, text, |mut generator| {
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

        line_generator(&FakeFont, 0, 0, true, text, |mut generator| {
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

        line_generator(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next(7, false).map(&collect), Some("A-very-"));
            assert_eq!(generator.next(7, false).map(&collect), Some("long-"));
            assert_eq!(generator.next(7, false).map(&collect), Some("word"));
            assert_eq!(generator.next(7, false).map(&collect), None);
        });

        let text = "A-very -long-word";

        line_generator(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next(7, false).map(&collect), Some("A-very "));
            assert_eq!(generator.next(7, false).map(&collect), Some("-long-"));
            assert_eq!(generator.next(7, false).map(&collect), Some("word"));
            assert_eq!(generator.next(7, false).map(&collect), None);
        });

        let text = "A-very--long-word";

        line_generator(&FakeFont, 0, 0, true, text, |mut generator| {
            let collect = collect(text);

            assert_eq!(generator.next(7, false).map(&collect), Some("A-"));
            assert_eq!(generator.next(7, false).map(&collect), Some("very--"));
            assert_eq!(generator.next(7, false).map(&collect), Some("long-"));
            assert_eq!(generator.next(7, false).map(&collect), Some("word"));
            assert_eq!(generator.next(7, false).map(&collect), None);
        });
    }
}
