use std::ops::Index;

use crate::{
    elements::{h_align::HorizontalAlignment, rich_text::Span, row::Flex, text::TextAlign},
    *,
};

use super::{Font, SerdeElement, SerdeElementElement};

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
pub struct Text {
    pub text: String,
    pub font: String,
    pub size: f64,
    pub color: u32,
    pub underline: bool,
    pub extra_character_spacing: f64,
    pub extra_word_spacing: f64,
    pub extra_line_height: f64,
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
pub struct RichText {
    pub spans: Vec<Span>,
    pub size: f64,
    pub small_size: f64,
    pub extra_line_height: f64,
    pub regular: String,
    pub bold: String,
    pub italic: String,
    pub bold_italic: String,
}

impl SerdeElement for RichText {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::rich_text::RichText {
            spans: &self.spans,
            size: self.size,
            small_size: self.small_size,
            extra_line_height: self.extra_line_height,
            fonts: FontSet {
                regular: &*fonts[&self.regular],
                bold: &*fonts[&self.bold],
                italic: &*fonts[&self.italic],
                bold_italic: &*fonts[&self.bold_italic],
            },
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct VGap {
    pub gap: f64,
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
            &SerdeElementElement {
                element: &*self.element,
                fonts,
            },
        ));
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Padding<E> {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
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
            element: &SerdeElementElement {
                element: &*self.element,
                fonts,
            },
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct StyledBox<E> {
    pub element: Box<E>,
    pub padding_left: f64,
    pub padding_right: f64,
    pub padding_top: f64,
    pub padding_bottom: f64,
    pub border_radius: f64,
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
            element: &SerdeElementElement {
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
    pub size: (f64, f64),
    pub fill: Option<u32>,
    pub outline: Option<(f64, u32)>,
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
    pub radius: f64,
    pub fill: Option<u32>,
    pub outline: Option<(f64, u32)>,
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
    pub gap: f64,
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
    pub gap: f64,
    pub expand: bool,
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
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BreakList<E> {
    pub content: Vec<E>,
    pub gap: f64,
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
}

impl<E: SerdeElement> SerdeElement for Stack<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::stack::Stack(|content| {
            for element in &self.content {
                content.add(&SerdeElementElement { element, fonts });
            }
        }));
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
    pub gap: f64,
    pub collapse_on_empty_content: bool,
}

impl<E: SerdeElement> SerdeElement for Titled<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::titled::Titled {
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
pub struct TitleOrBreak<E> {
    pub title: Box<E>,
    pub content: Box<E>,
    pub gap: f64,
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
pub struct RepeatAfterBreak<E> {
    pub title: Box<E>,
    pub content: Box<E>,
    pub gap: f64,
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
    pub min_first_height: f64,
}

impl<E: SerdeElement> SerdeElement for MinFirstHeight<E> {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    ) {
        callback.call(&elements::min_first_height::MinFirstHeight {
            element: &SerdeElementElement {
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
            &SerdeElementElement {
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
                &SerdeElementElement {
                    element: &*self.element,
                    fonts,
                },
            ),
        );
    }
}
