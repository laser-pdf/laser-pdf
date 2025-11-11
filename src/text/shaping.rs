use itertools::Itertools;

use crate::fonts::{Font, ShapedGlyph};

pub fn shape<'a, F: Font>(
    font: &'a F,
    fallback_fonts: &'a [F],
    fallback_font_index: Option<usize>,
    text: &'a str,
    character_spacing: f32,
    word_spacing: f32,
    buffer: &mut Vec<(Option<usize>, ShapedGlyph)>,
    text_range_offset: usize,
) {
    let mut shaped = font.shape(text, character_spacing, word_spacing).peekable();

    buffer.reserve(shaped.size_hint().0);

    let next_font = fallback_fonts.first();

    while let Some(glyph) = shaped.next() {
        if glyph.glyph_id == 0
            && let Some(next_font) = next_font
        {
            let others = shaped.peeking_take_while(|g| g.glyph_id == 0);

            let text_range = glyph.text_range.start
                ..others
                    .last()
                    .map_or(glyph.text_range.end, |g| g.text_range.end);

            shape(
                next_font,
                &fallback_fonts[1..],
                Some(fallback_font_index.map_or(0, |i| i + 1)),
                &text[text_range.clone()],
                character_spacing,
                word_spacing,
                buffer,
                text_range_offset + text_range.start,
            )
        } else {
            buffer.push((
                fallback_font_index,
                ShapedGlyph {
                    text_range: (glyph.text_range.start + text_range_offset)
                        ..(glyph.text_range.end + text_range_offset),
                    ..glyph
                },
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Copy, Clone)]
    enum FakeFont {
        A,
        B,
        C,
    }

    #[derive(Clone)]
    struct FakeShaped<'a> {
        font: FakeFont,
        // last: usize,
        inner: std::str::CharIndices<'a>,
    }

    impl<'a> Iterator for FakeShaped<'a> {
        type Item = ShapedGlyph;

        fn next(&mut self) -> Option<Self::Item> {
            if let Some((i, c)) = self.inner.next() {
                let found = match self.font {
                    FakeFont::A => c.is_ascii_uppercase(),
                    FakeFont::B => c.is_ascii_lowercase(),
                    FakeFont::C => c.is_ascii_digit(),
                };

                Some(ShapedGlyph {
                    unsafe_to_break: false,
                    glyph_id: if found { c as u32 } else { 0 },
                    text_range: i..i + c.len_utf8(),
                    x_advance_font: 1.,
                    x_advance: 1.,
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
                font: *self,
                inner: text.char_indices(),
            }
        }

        fn encode(&self, _: &mut crate::Pdf, _: u32, _: &str) -> crate::fonts::EncodedGlyph {
            unimplemented!()
        }

        fn index(&self) -> usize {
            unimplemented!()
        }

        fn resource_name(&self) -> pdf_writer::Name<'_> {
            unimplemented!()
        }

        fn general_metrics(&self) -> crate::fonts::GeneralMetrics {
            unimplemented!()
        }

        fn fallback_fonts(&self) -> &[Self] {
            &[FakeFont::B, FakeFont::C]
        }
    }

    #[test]
    fn test_fallback() {
        let font = FakeFont::A;

        let text = "ABCabc123ABC";

        let mut shaped = Vec::new();
        shape(
            &font,
            font.fallback_fonts(),
            None,
            text,
            0.,
            0.,
            &mut shaped,
            0,
        );

        insta::assert_debug_snapshot!(shaped, @r"
        [
            (
                None,
                ShapedGlyph {
                    unsafe_to_break: false,
                    glyph_id: 65,
                    text_range: 0..1,
                    x_advance_font: 1.0,
                    x_advance: 1.0,
                    x_offset: 0.0,
                    y_advance: 0.0,
                    y_offset: 0.0,
                },
            ),
            (
                None,
                ShapedGlyph {
                    unsafe_to_break: false,
                    glyph_id: 66,
                    text_range: 1..2,
                    x_advance_font: 1.0,
                    x_advance: 1.0,
                    x_offset: 0.0,
                    y_advance: 0.0,
                    y_offset: 0.0,
                },
            ),
            (
                None,
                ShapedGlyph {
                    unsafe_to_break: false,
                    glyph_id: 67,
                    text_range: 2..3,
                    x_advance_font: 1.0,
                    x_advance: 1.0,
                    x_offset: 0.0,
                    y_advance: 0.0,
                    y_offset: 0.0,
                },
            ),
            (
                Some(
                    0,
                ),
                ShapedGlyph {
                    unsafe_to_break: false,
                    glyph_id: 97,
                    text_range: 3..4,
                    x_advance_font: 1.0,
                    x_advance: 1.0,
                    x_offset: 0.0,
                    y_advance: 0.0,
                    y_offset: 0.0,
                },
            ),
            (
                Some(
                    0,
                ),
                ShapedGlyph {
                    unsafe_to_break: false,
                    glyph_id: 98,
                    text_range: 4..5,
                    x_advance_font: 1.0,
                    x_advance: 1.0,
                    x_offset: 0.0,
                    y_advance: 0.0,
                    y_offset: 0.0,
                },
            ),
            (
                Some(
                    0,
                ),
                ShapedGlyph {
                    unsafe_to_break: false,
                    glyph_id: 99,
                    text_range: 5..6,
                    x_advance_font: 1.0,
                    x_advance: 1.0,
                    x_offset: 0.0,
                    y_advance: 0.0,
                    y_offset: 0.0,
                },
            ),
            (
                Some(
                    1,
                ),
                ShapedGlyph {
                    unsafe_to_break: false,
                    glyph_id: 49,
                    text_range: 6..7,
                    x_advance_font: 1.0,
                    x_advance: 1.0,
                    x_offset: 0.0,
                    y_advance: 0.0,
                    y_offset: 0.0,
                },
            ),
            (
                Some(
                    1,
                ),
                ShapedGlyph {
                    unsafe_to_break: false,
                    glyph_id: 50,
                    text_range: 7..8,
                    x_advance_font: 1.0,
                    x_advance: 1.0,
                    x_offset: 0.0,
                    y_advance: 0.0,
                    y_offset: 0.0,
                },
            ),
            (
                Some(
                    1,
                ),
                ShapedGlyph {
                    unsafe_to_break: false,
                    glyph_id: 51,
                    text_range: 8..9,
                    x_advance_font: 1.0,
                    x_advance: 1.0,
                    x_offset: 0.0,
                    y_advance: 0.0,
                    y_offset: 0.0,
                },
            ),
            (
                None,
                ShapedGlyph {
                    unsafe_to_break: false,
                    glyph_id: 65,
                    text_range: 9..10,
                    x_advance_font: 1.0,
                    x_advance: 1.0,
                    x_offset: 0.0,
                    y_advance: 0.0,
                    y_offset: 0.0,
                },
            ),
            (
                None,
                ShapedGlyph {
                    unsafe_to_break: false,
                    glyph_id: 66,
                    text_range: 10..11,
                    x_advance_font: 1.0,
                    x_advance: 1.0,
                    x_offset: 0.0,
                    y_advance: 0.0,
                    y_offset: 0.0,
                },
            ),
            (
                None,
                ShapedGlyph {
                    unsafe_to_break: false,
                    glyph_id: 67,
                    text_range: 11..12,
                    x_advance_font: 1.0,
                    x_advance: 1.0,
                    x_offset: 0.0,
                    y_advance: 0.0,
                    y_offset: 0.0,
                },
            ),
        ]
        ");
    }
}
