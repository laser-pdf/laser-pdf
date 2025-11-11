use std::ops::Index;

use elements::rotate::Rotation;

use crate::{
    elements::{h_align::HorizontalAlignment, row::Flex, text::TextAlign},
    *,
};

use super::{Font, SerdeElement, SerdeElementElement};

const fn default_false() -> bool {
    false
}

const fn default_0u8() -> u8 {
    0
}

#[derive(Clone, Serialize, Deserialize)]
pub struct None;

impl SerdeElement for None {
    fn element(
        &self,
        _: &impl for<'a> Index<&'a str, Output = Font>,
        _: impl CompositeElementCallback,
    ) {
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Debug<E> {
    pub element: Box<E>,

    #[serde(default = "default_0u8")]
    pub color: u8,

    #[serde(default = "default_false")]
    pub show_max_width: bool,

    #[serde(default = "default_false")]
    pub show_last_location_max_height: bool,
}

impl<E: SerdeElement> SerdeElement for Debug<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::debug::Debug {
            element: SerdeElementElement {
                element: &*self.element,
                fonts,
            },
            color: self.color,
            show_max_width: self.show_max_width,
            show_last_location_max_height: self.show_last_location_max_height,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Text {
    pub text: String,
    pub font: String,
    pub size: f32,
    pub color: u32,
    pub underline: bool,
    pub extra_character_spacing: f32,
    pub extra_word_spacing: f32,
    pub extra_line_height: f32,
    pub align: TextAlign,
}

impl SerdeElement for Text {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::text::Text {
            text: &self.text,
            font: &*fonts[&self.font],
            size: self.size,
            color: self.color,
            underline: self.underline,
            extra_character_spacing: self.extra_character_spacing,
            extra_word_spacing: self.extra_word_spacing,
            extra_line_height: self.extra_line_height,
            align: self.align,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RichTextSpan {
    /// The text content to render
    pub text: String,
    /// Font reference
    pub font: String,
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
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RichText {
    pub spans: Vec<RichTextSpan>,
    pub align: TextAlign,
}

impl SerdeElement for RichText {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::rich_text::RichText {
            spans: self.spans.iter().map(|s| elements::rich_text::Span {
                text: &s.text,
                font: &*fonts[&s.font],
                size: s.size,
                color: s.color,
                underline: s.underline,
                extra_character_spacing: s.extra_character_spacing,
                extra_word_spacing: s.extra_word_spacing,
                extra_line_height: s.extra_line_height,
            }),
            align: self.align,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct VGap {
    pub gap: f32,
}

impl SerdeElement for VGap {
    fn element(
        &self,
        _: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::v_gap::VGap(self.gap));
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct HAlign<E> {
    pub alignment: HorizontalAlignment,
    pub element: Box<E>,
}

impl<E: SerdeElement> SerdeElement for HAlign<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::h_align::HAlign(
            self.alignment,
            SerdeElementElement {
                element: &*self.element,
                fonts,
            },
        ));
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Padding<E> {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub element: Box<E>,
}

impl<E: SerdeElement> SerdeElement for Padding<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::padding::Padding {
            left: self.left,
            right: self.right,
            top: self.top,
            bottom: self.bottom,
            element: SerdeElementElement {
                element: &*self.element,
                fonts,
            },
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct StyledBox<E> {
    pub element: Box<E>,
    pub padding_left: f32,
    pub padding_right: f32,
    pub padding_top: f32,
    pub padding_bottom: f32,
    pub border_radius: f32,
    pub fill: Option<u32>,
    pub outline: Option<LineStyle>,
}

impl<E: SerdeElement> SerdeElement for StyledBox<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::styled_box::StyledBox {
            element: SerdeElementElement {
                element: &*self.element,
                fonts,
            },
            padding_left: self.padding_left,
            padding_right: self.padding_right,
            padding_top: self.padding_top,
            padding_bottom: self.padding_bottom,
            border_radius: self.border_radius,
            fill: self.fill,
            outline: self.outline,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Line {
    pub style: LineStyle,
}

impl SerdeElement for Line {
    fn element(
        &self,
        _: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::line::Line { style: self.style });
    }
}

#[derive(Clone, Deserialize)]
pub struct Image {
    #[serde(rename = "path", deserialize_with = "crate::image::deserialize_image")]
    pub image: crate::image::Image,
}

impl SerdeElement for Image {
    fn element(
        &self,
        _: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::image::ImageElement { image: &self.image });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Rectangle {
    pub size: (f32, f32),
    pub fill: Option<u32>,
    pub outline: Option<(f32, u32)>,
}

impl SerdeElement for Rectangle {
    fn element(
        &self,
        _: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::rectangle::Rectangle {
            size: self.size,
            fill: self.fill,
            outline: self.outline,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Circle {
    pub radius: f32,
    pub fill: Option<u32>,
    pub outline: Option<(f32, u32)>,
}

impl SerdeElement for Circle {
    fn element(
        &self,
        _: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::circle::Circle {
            radius: self.radius,
            fill: self.fill,
            outline: self.outline,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Column<E> {
    pub content: Vec<E>,
    pub gap: f32,

    #[serde(default = "default_false")]
    pub collapse: bool,
}

impl<E: SerdeElement> SerdeElement for Column<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::column::Column {
            content: |mut content| {
                for element in &self.content {
                    content = content.add(&SerdeElementElement { element, fonts })?;
                }

                Option::None
            },
            gap: self.gap,
            collapse: self.collapse,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RowElement<E> {
    pub element: E,
    pub flex: Flex,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Row<E> {
    pub content: Vec<RowElement<E>>,
    pub gap: f32,
    pub expand: bool,
    pub collapse: bool,
}

impl<E: SerdeElement> SerdeElement for Row<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::row::Row {
            content: |content| {
                for RowElement { element, flex } in &self.content {
                    content.add(&SerdeElementElement { element, fonts }, *flex);
                }
            },
            gap: self.gap,
            expand: self.expand,
            collapse: self.collapse,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BreakList<E> {
    pub content: Vec<E>,
    pub gap: f32,
}

impl<E: SerdeElement> SerdeElement for BreakList<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::break_list::BreakList {
            content: |mut content| {
                for element in &self.content {
                    content = content.add(&SerdeElementElement { element, fonts })?;
                }

                Option::None
            },
            gap: self.gap,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Stack<E> {
    pub content: Vec<E>,
    pub expand: bool,
}

impl<E: SerdeElement> SerdeElement for Stack<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::stack::Stack {
            content: |content| {
                for element in &self.content {
                    content.add(&SerdeElementElement { element, fonts });
                }
            },
            expand: self.expand,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TableRowElement<E> {
    pub element: E,
    pub flex: elements::table_row::Flex,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TableRow<E> {
    pub content: Vec<TableRowElement<E>>,
    pub line_style: LineStyle,

    #[serde(alias = "y_expand")]
    pub expand: bool,
}

impl<E: SerdeElement> SerdeElement for TableRow<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::table_row::TableRow {
            content: |content| {
                for TableRowElement { element, flex } in &self.content {
                    content.add(&SerdeElementElement { element, fonts }, *flex);
                }
            },
            line_style: self.line_style,
            expand: self.expand,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Titled<E> {
    pub title: Box<E>,
    pub content: Box<E>,
    pub gap: f32,

    #[serde(default = "default_false")]
    pub collapse_on_empty_content: bool,
}

impl<E: SerdeElement> SerdeElement for Titled<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::titled::Titled {
            title: SerdeElementElement {
                element: &*self.title,
                fonts,
            },
            content: SerdeElementElement {
                element: &*self.content,
                fonts,
            },
            gap: self.gap,
            collapse_on_empty_content: self.collapse_on_empty_content,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TitleOrBreak<E> {
    pub title: Box<E>,
    pub content: Box<E>,
    pub gap: f32,

    #[serde(default = "default_false")]
    pub collapse_on_empty_content: bool,
}

impl<E: SerdeElement> SerdeElement for TitleOrBreak<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::title_or_break::TitleOrBreak {
            title: &SerdeElementElement {
                element: &*self.title,
                fonts,
            },
            content: &SerdeElementElement {
                element: &*self.content,
                fonts,
            },
            gap: self.gap,
            collapse_on_empty_content: self.collapse_on_empty_content,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChangingTitle<E> {
    pub first_title: Box<E>,

    #[serde(alias = "second_title")]
    pub remaining_title: Box<E>,

    pub content: Box<E>,
    pub gap: f32,

    #[serde(default = "default_false")]
    pub collapse: bool,
}

impl<E: SerdeElement> SerdeElement for ChangingTitle<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::changing_title::ChangingTitle {
            first_title: SerdeElementElement {
                element: &*self.first_title,
                fonts,
            },
            remaining_title: SerdeElementElement {
                element: &*self.remaining_title,
                fonts,
            },
            content: SerdeElementElement {
                element: &*self.content,
                fonts,
            },
            gap: self.gap,
            collapse: self.collapse,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RepeatAfterBreak<E> {
    pub title: Box<E>,
    pub content: Box<E>,
    pub gap: f32,

    #[serde(default = "default_false")]
    pub collapse_on_empty_content: bool,
}

impl<E: SerdeElement> SerdeElement for RepeatAfterBreak<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::repeat_after_break::RepeatAfterBreak {
            title: &SerdeElementElement {
                element: &*self.title,
                fonts,
            },
            content: &SerdeElementElement {
                element: &*self.content,
                fonts,
            },
            gap: self.gap,
            collapse_on_empty_content: self.collapse_on_empty_content,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RepeatBottom<E> {
    pub content: Box<E>,
    pub bottom: Box<E>,
    pub gap: f32,

    #[serde(default = "default_false")]
    pub collapse: bool,
}

impl<E: SerdeElement> SerdeElement for RepeatBottom<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::repeat_bottom::RepeatBottom {
            content: SerdeElementElement {
                element: &*self.content,
                fonts,
            },
            bottom: SerdeElementElement {
                element: &*self.bottom,
                fonts,
            },
            gap: self.gap,
            collapse: self.collapse,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PinBelow<E> {
    pub content: Box<E>,
    pub pinned_element: Box<E>,
    pub gap: f32,

    #[serde(default = "default_false")]
    pub collapse: bool,
}

impl<E: SerdeElement> SerdeElement for PinBelow<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::pin_below::PinBelow {
            content: SerdeElementElement {
                element: &*self.content,
                fonts,
            },
            pinned_element: SerdeElementElement {
                element: &*self.pinned_element,
                fonts,
            },
            gap: self.gap,
            collapse: self.collapse,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ForceBreak;

impl SerdeElement for ForceBreak {
    fn element(
        &self,
        _: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::force_break::ForceBreak);
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BreakWhole<E> {
    pub element: Box<E>,
}

impl<E: SerdeElement> SerdeElement for BreakWhole<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::break_whole::BreakWhole(&SerdeElementElement {
            element: &*self.element,
            fonts,
        }));
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MinFirstHeight<E> {
    pub element: Box<E>,
    pub min_first_height: f32,
}

impl<E: SerdeElement> SerdeElement for MinFirstHeight<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::min_first_height::MinFirstHeight {
            element: SerdeElementElement {
                element: &*self.element,
                fonts,
            },
            min_first_height: self.min_first_height,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AlignLocationBottom<E> {
    pub element: Box<E>,
}

impl<E: SerdeElement> SerdeElement for AlignLocationBottom<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::align_location_bottom::AlignLocationBottom(
            SerdeElementElement {
                element: &*self.element,
                fonts,
            },
        ));
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AlignPreferredHeightBottom<E> {
    pub element: Box<E>,
}

impl<E: SerdeElement> SerdeElement for AlignPreferredHeightBottom<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(
            &elements::align_preferred_height_bottom::AlignPreferredHeightBottom(
                SerdeElementElement {
                    element: &*self.element,
                    fonts,
                },
            ),
        );
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ExpandToPreferredHeight<E> {
    pub element: Box<E>,
}

impl<E: SerdeElement> SerdeElement for ExpandToPreferredHeight<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(
            &elements::expand_to_preferred_height::ExpandToPreferredHeight(SerdeElementElement {
                element: &*self.element,
                fonts,
            }),
        );
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ShrinkToFit<E> {
    pub element: Box<E>,
    pub min_height: f32,
}

impl<E: SerdeElement> SerdeElement for ShrinkToFit<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::shrink_to_fit::ShrinkToFit {
            element: SerdeElementElement {
                element: &*self.element,
                fonts,
            },
            min_height: self.min_height,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Rotate<E> {
    pub element: Box<E>,
    pub rotation: Rotation,
}

impl<E: SerdeElement> SerdeElement for Rotate<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::rotate::Rotate {
            element: SerdeElementElement {
                element: &*self.element,
                fonts,
            },
            rotation: self.rotation,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MaxWidth<E> {
    pub element: Box<E>,
    pub max_width: f32,
}

impl<E: SerdeElement> SerdeElement for MaxWidth<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::max_width::MaxWidth {
            element: SerdeElementElement {
                element: &*self.element,
                fonts,
            },
            max_width: self.max_width,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum PageNumberText {
    Current {
        before: String,
        after: String,
    },
    Total {
        before: String,
        after: String,
    },
    CurrentAndTotal {
        before: String,
        between: String,
        after: String,
    },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PageNumber {
    pub skip_pages: usize,
    pub pos: (elements::page::X, elements::page::Y),
    pub text: PageNumberText,
    pub font: String,
    pub size: f32,
    pub color: u32,
    pub underline: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DecorationElement<E> {
    pub element: E,
    pub pos: (elements::page::X, elements::page::Y),
    pub width: Option<f32>,
    pub skip_pages: usize,
    pub repeat: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Page<E> {
    pub primary: Box<E>,
    pub border_left: f32,
    pub border_right: f32,
    pub border_top: f32,
    pub border_bottom: f32,
    pub decoration_elements: Vec<DecorationElement<E>>,
    pub page_numbers: Vec<PageNumber>,
}

impl<E: SerdeElement> SerdeElement for Page<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::page::Page {
            primary: SerdeElementElement {
                element: &*self.primary,
                fonts,
            },
            border_left: self.border_left,
            border_right: self.border_right,
            border_top: self.border_top,
            border_bottom: self.border_bottom,
            decoration_elements: |content, page, page_count| {
                for decoration_element in &self.decoration_elements {
                    if page == decoration_element.skip_pages
                        || decoration_element.repeat && page > decoration_element.skip_pages
                    {
                        content.add(
                            &SerdeElementElement {
                                element: &decoration_element.element,
                                fonts,
                            },
                            decoration_element.pos,
                            decoration_element.width,
                        );
                    }
                }

                for page_number in &self.page_numbers {
                    if page >= page_number.skip_pages {
                        content.add(
                            &elements::text::Text {
                                underline: page_number.underline,
                                color: page_number.color,
                                ..elements::text::Text::basic(
                                    // Since the decoration_elements callback is only called when
                                    // drawing it shouldn't be a problem to be allocating a string here,
                                    // but it could potentially be optimized by reusing the buffer.
                                    &match page_number.text {
                                        PageNumberText::Current {
                                            ref before,
                                            ref after,
                                        } => {
                                            format!("{before}{}{after}", page + 1)
                                        }
                                        PageNumberText::Total {
                                            ref before,
                                            ref after,
                                        } => format!("{before}{page_count}{after}"),
                                        PageNumberText::CurrentAndTotal {
                                            ref before,
                                            ref between,
                                            ref after,
                                        } => {
                                            format!(
                                                "{before}{}{between}{page_count}{after}",
                                                page + 1
                                            )
                                        }
                                    },
                                    &*fonts[&page_number.font],
                                    page_number.size,
                                )
                            },
                            page_number.pos,
                            Option::None,
                        );
                    }
                }
            },
        });
    }
}
