use crate::{
    elements::new_rich_text::{RichText, Span},
    fonts::Font,
    *,
};

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

/// A text element that renders text content with various styling options.
pub struct Text<'a, F: Font> {
    /// The text content to render
    pub text: &'a str,
    /// Font reference
    pub font: &'a F,
    /// Font size in points
    pub size: f32,
    /// Text color as RGBA (default: black 0x00_00_00_FF)
    pub color: u32,
    /// Whether to underline the text
    pub underline: bool,
    /// Additional spacing between characters
    pub extra_character_spacing: f32,
    /// Additional spacing between words
    pub extra_word_spacing: f32,
    /// Additional line height
    pub extra_line_height: f32,
    /// Text alignment
    pub align: TextAlign,
}

impl<'a, F: Font> Text<'a, F> {
    pub fn basic(text: &'a str, font: &'a F, size: f32) -> Self {
        Text {
            text,
            font,
            size,
            color: 0x00_00_00_FF,
            underline: false,
            extra_character_spacing: 0.,
            extra_word_spacing: 0.,
            extra_line_height: 0.,
            align: TextAlign::Left,
        }
    }

    fn as_rich_text(&self) -> RichText<std::iter::Once<Span<'a, F>>> {
        RichText {
            spans: std::iter::once(Span {
                text: self.text,
                font: self.font,
                size: self.size,
                color: self.color,
                underline: self.underline,
                extra_character_spacing: self.extra_character_spacing,
                extra_word_spacing: self.extra_word_spacing,
                extra_line_height: self.extra_line_height, // TODO: thread this through to the pieces
            }),
            align: match self.align {
                TextAlign::Left => elements::new_rich_text::TextAlign::Left,
                TextAlign::Center => elements::new_rich_text::TextAlign::Center,
                TextAlign::Right => elements::new_rich_text::TextAlign::Right,
            },
        }
    }
}

impl<'a, F: Font + 'a> Element for Text<'a, F> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        self.as_rich_text().first_location_usage(ctx)
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        self.as_rich_text().measure(ctx)
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        self.as_rich_text().draw(ctx)
    }
}

#[cfg(test)]
mod tests {
    use elements::column::Column;
    use fonts::truetype::TruetypeFont;
    use insta::*;

    use crate::fonts::builtin::BuiltinFont;
    use crate::test_utils::binary_snapshots::*;

    use super::*;

    const FONT: &[u8] = include_bytes!("../fonts/Kenney Bold.ttf");

    #[test]
    fn test_multi_page() {
        let bytes = test_element_bytes(TestElementParams::breakable(), |mut callback| {
            let font = BuiltinFont::courier(callback.pdf());

            let content = Text::basic(LOREM_IPSUM, &font, 32.);
            let content = content.debug(0);

            callback.call(&content);
        });
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_truetype() {
        let bytes = test_element_bytes(TestElementParams::breakable(), |mut callback| {
            let font = TruetypeFont::new(callback.pdf(), FONT);

            let content = Text::basic(LOREM_IPSUM, &font, 32.);
            let content = content.debug(0);

            callback.call(&content);
        });
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_truetype_trailing_whitespace() {
        let mut params = TestElementParams::breakable();
        params.width.expand = false;

        let bytes = test_element_bytes(params, |mut callback| {
            let font = TruetypeFont::new(callback.pdf(), FONT);

            let content = Text::basic("Whitespace ", &font, 32.);
            let content = content.debug(0);

            callback.call(&content);
        });
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_truetype_extra_spacing() {
        let mut params = TestElementParams::breakable();
        params.width.expand = false;

        let bytes = test_element_bytes(params, |mut callback| {
            let font = TruetypeFont::new(callback.pdf(), FONT);

            callback.call(&Column {
                gap: 12.,
                collapse: false,
                content: |content| {
                    let normal = Text::basic("Hello, World", &font, 32.);

                    let character_spacing = Text {
                        extra_character_spacing: 16.,
                        ..Text::basic("Hello, World", &font, 32.)
                    };

                    let word_spacing = Text {
                        extra_word_spacing: 16.,
                        ..Text::basic("Hello, World", &font, 32.)
                    };

                    let both = Text {
                        extra_character_spacing: 16.,
                        extra_word_spacing: 16.,
                        ..Text::basic("Hello, World", &font, 32.)
                    };

                    content
                        .add(&normal.debug(0).show_max_width())?
                        .add(&character_spacing.debug(1).show_max_width())?
                        .add(&word_spacing.debug(2).show_max_width())?
                        .add(&both.debug(3).show_max_width())?;

                    None
                },
            });
        });
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_truetype_soft_hyphen() {
        let mut params = TestElementParams::breakable();
        params.width.expand = false;

        let bytes = test_element_bytes(params, |mut callback| {
            let font = TruetypeFont::new(callback.pdf(), FONT);

            callback.call(&Column {
                gap: 12.,
                collapse: false,
                content: |content| {
                    let a = Text::basic("Hello\u{00AD}Wrld", &font, 32.);
                    let b = Text::basic("A Hello\u{00AD}Wrld", &font, 32.);
                    let c = Text::basic("A\u{00A0}Hello\u{00AD}Wrld", &font, 32.);
                    let d = Text::basic("Hello\u{00AD}Wrld\u{00AD}", &font, 32.);

                    content
                        .add(&Padding::right(100., a.debug(0).show_max_width()))?
                        .add(&Padding::right(120., b.debug(0).show_max_width()))?
                        .add(&Padding::right(120., c.debug(0).show_max_width()))?
                        .add(&Padding::right(20., d.debug(0).show_max_width()))?;

                    None
                },
            });
        });
        assert_binary_snapshot!(".pdf", bytes);
    }
}
