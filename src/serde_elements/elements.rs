use std::ops::Index;

use crate::{
    elements::{h_align::HorizontalAlignment, rich_text::Span, row::Flex, text::TextAlign},
    *,
};

use super::{Font, SerdeElement};

#[derive(Clone, Serialize, Deserialize)]
pub struct None;

impl<'a, F: Index<&'a str, Output = Font>> CompositeElement for SerdeElement<'a, None, F> {
    fn element(&self, _: impl CompositeElementCallback) {}
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

impl<'a, F: Index<&'a str, Output = Font>> CompositeElement for SerdeElement<'a, Text, F> {
    fn element(&self, callback: impl CompositeElementCallback) {
        callback.call(&elements::text::Text {
            text: &self.element.text,
            font: &*self.fonts[&self.element.font],
            size: self.element.size,
            color: self.element.color,
            underline: self.element.underline,
            extra_character_spacing: self.element.extra_character_spacing,
            extra_word_spacing: self.element.extra_word_spacing,
            extra_line_height: self.element.extra_line_height,
            align: self.element.align,
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

impl<'a, F: Index<&'a str, Output = Font>> CompositeElement for SerdeElement<'a, RichText, F> {
    fn element(&self, callback: impl CompositeElementCallback) {
        callback.call(&elements::rich_text::RichText {
            spans: &self.element.spans,
            size: self.element.size,
            small_size: self.element.small_size,
            extra_line_height: self.element.extra_line_height,
            fonts: FontSet {
                regular: &*self.fonts[&self.element.regular],
                bold: &*self.fonts[&self.element.bold],
                italic: &*self.fonts[&self.element.italic],
                bold_italic: &*self.fonts[&self.element.bold_italic],
            },
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct VGap {
    pub gap: f64,
}

impl<'a, F: Index<&'a str, Output = Font>> CompositeElement for SerdeElement<'a, VGap, F> {
    fn element(&self, callback: impl CompositeElementCallback) {
        callback.call(&elements::v_gap::VGap(self.element.gap));
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct HAlign<E> {
    pub alignment: HorizontalAlignment,
    pub element: Box<E>,
}

impl<'a, E, F: Index<&'a str, Output = Font>> CompositeElement for SerdeElement<'a, HAlign<E>, F>
where
    SerdeElement<'a, E, F>: Element,
{
    fn element(&self, callback: impl CompositeElementCallback) {
        callback.call(&elements::h_align::HAlign(
            self.element.alignment,
            &SerdeElement {
                element: &*self.element.element,
                fonts: self.fonts,
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

impl<'a, E, F: Index<&'a str, Output = Font>> CompositeElement for SerdeElement<'a, Padding<E>, F>
where
    SerdeElement<'a, E, F>: Element,
{
    fn element(&self, callback: impl CompositeElementCallback) {
        callback.call(&elements::padding::Padding {
            left: self.element.left,
            right: self.element.right,
            top: self.element.top,
            bottom: self.element.bottom,
            element: &SerdeElement {
                element: &*self.element.element,
                fonts: self.fonts,
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

impl<'a, E, F: Index<&'a str, Output = Font>> CompositeElement for SerdeElement<'a, StyledBox<E>, F>
where
    SerdeElement<'a, E, F>: Element,
{
    fn element(&self, callback: impl CompositeElementCallback) {
        callback.call(&elements::styled_box::StyledBox {
            element: &SerdeElement {
                element: &*self.element.element,
                fonts: self.fonts,
            },
            padding_left: self.element.padding_left,
            padding_right: self.element.padding_right,
            padding_top: self.element.padding_top,
            padding_bottom: self.element.padding_bottom,
            border_radius: self.element.border_radius,
            fill: self.element.fill,
            outline: self.element.outline,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Line {
    pub style: LineStyle,
}

impl<'a, F: Index<&'a str, Output = Font>> CompositeElement for SerdeElement<'a, Line, F> {
    fn element(&self, callback: impl CompositeElementCallback) {
        callback.call(&elements::line::Line {
            style: self.element.style,
        });
    }
}

#[derive(Clone, Deserialize)]
pub struct Image {
    #[serde(rename = "path", deserialize_with = "crate::image::deserialize_image")]
    pub image: crate::image::Image,
}

impl<'a, F: Index<&'a str, Output = Font>> CompositeElement for SerdeElement<'a, Image, F> {
    fn element(&self, callback: impl CompositeElementCallback) {
        callback.call(&elements::image::ImageElement {
            image: &self.element.image,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Rectangle {
    pub size: (f64, f64),
    pub fill: Option<u32>,
    pub outline: Option<(f64, u32)>,
}

impl<'a, F: Index<&'a str, Output = Font>> CompositeElement for SerdeElement<'a, Rectangle, F> {
    fn element(&self, callback: impl CompositeElementCallback) {
        callback.call(&elements::rectangle::Rectangle {
            size: self.element.size,
            fill: self.element.fill,
            outline: self.element.outline,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Circle {
    pub radius: f64,
    pub fill: Option<u32>,
    pub outline: Option<(f64, u32)>,
}

impl<'a, F: Index<&'a str, Output = Font>> CompositeElement for SerdeElement<'a, Circle, F> {
    fn element(&self, callback: impl CompositeElementCallback) {
        callback.call(&elements::circle::Circle {
            radius: self.element.radius,
            fill: self.element.fill,
            outline: self.element.outline,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Column<E> {
    pub content: Vec<E>,
    pub gap: f64,
}

impl<'a, E, F: Index<&'a str, Output = Font>> CompositeElement for SerdeElement<'a, Column<E>, F>
where
    SerdeElement<'a, E, F>: Element,
{
    fn element(&self, callback: impl CompositeElementCallback) {
        callback.call(&elements::column::Column {
            content: |mut content| {
                for element in &self.element.content {
                    content = content.add(&SerdeElement {
                        element,
                        fonts: self.fonts,
                    })?;
                }

                Option::None
            },
            gap: self.element.gap,
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

impl<'a, E, F: Index<&'a str, Output = Font>> CompositeElement for SerdeElement<'a, Row<E>, F>
where
    SerdeElement<'a, E, F>: Element,
{
    fn element(&self, callback: impl CompositeElementCallback) {
        callback.call(&elements::row::Row {
            content: |content| {
                for RowElement { element, flex } in &self.element.content {
                    content.add(
                        &SerdeElement {
                            element,
                            fonts: self.fonts,
                        },
                        *flex,
                    );
                }
            },
            gap: self.element.gap,
            expand: self.element.expand,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BreakList<E> {
    pub content: Vec<E>,
    pub gap: f64,
}

impl<'a, E, F: Index<&'a str, Output = Font>> CompositeElement for SerdeElement<'a, BreakList<E>, F>
where
    SerdeElement<'a, E, F>: Element,
{
    fn element(&self, callback: impl CompositeElementCallback) {
        callback.call(&elements::break_list::BreakList {
            content: |mut content| {
                for element in &self.element.content {
                    content = content.add(&SerdeElement {
                        element,
                        fonts: self.fonts,
                    })?;
                }

                Option::None
            },
            gap: self.element.gap,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Stack<E> {
    pub content: Vec<E>,
}

impl<'a, E, F: Index<&'a str, Output = Font>> CompositeElement for SerdeElement<'a, Stack<E>, F>
where
    SerdeElement<'a, E, F>: Element,
{
    fn element(&self, callback: impl CompositeElementCallback) {
        callback.call(&elements::stack::Stack(|content| {
            for element in &self.element.content {
                content.add(&SerdeElement {
                    element,
                    fonts: self.fonts,
                });
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

impl<'a, E, F: Index<&'a str, Output = Font>> CompositeElement for SerdeElement<'a, TableRow<E>, F>
where
    SerdeElement<'a, E, F>: Element,
{
    fn element(&self, callback: impl CompositeElementCallback) {
        callback.call(&elements::table_row::TableRow {
            content: |content| {
                for TableRowElement { element, flex } in &self.element.content {
                    content.add(
                        &SerdeElement {
                            element,
                            fonts: self.fonts,
                        },
                        *flex,
                    );
                }
            },
            line_style: self.element.line_style,
            expand: self.element.expand,
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

impl<'a, E, F: Index<&'a str, Output = Font>> CompositeElement for SerdeElement<'a, Titled<E>, F>
where
    SerdeElement<'a, E, F>: Element,
{
    fn element(&self, callback: impl CompositeElementCallback) {
        callback.call(&elements::titled::Titled {
            title: &SerdeElement {
                element: &*self.element.title,
                fonts: self.fonts,
            },
            content: &SerdeElement {
                element: &*self.element.content,
                fonts: self.fonts,
            },
            gap: self.element.gap,
            collapse_on_empty_content: self.element.collapse_on_empty_content,
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

impl<'a, E, F: Index<&'a str, Output = Font>> CompositeElement
    for SerdeElement<'a, TitleOrBreak<E>, F>
where
    SerdeElement<'a, E, F>: Element,
{
    fn element(&self, callback: impl CompositeElementCallback) {
        callback.call(&elements::title_or_break::TitleOrBreak {
            title: &SerdeElement {
                element: &*self.element.title,
                fonts: self.fonts,
            },
            content: &SerdeElement {
                element: &*self.element.content,
                fonts: self.fonts,
            },
            gap: self.element.gap,
            collapse_on_empty_content: self.element.collapse_on_empty_content,
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

impl<'a, E, F: Index<&'a str, Output = Font>> CompositeElement
    for SerdeElement<'a, RepeatAfterBreak<E>, F>
where
    SerdeElement<'a, E, F>: Element,
{
    fn element(&self, callback: impl CompositeElementCallback) {
        callback.call(&elements::repeat_after_break::RepeatAfterBreak {
            title: &SerdeElement {
                element: &*self.element.title,
                fonts: self.fonts,
            },
            content: &SerdeElement {
                element: &*self.element.content,
                fonts: self.fonts,
            },
            gap: self.element.gap,
            collapse_on_empty_content: self.element.collapse_on_empty_content,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ForceBreak;

impl<'a, F: Index<&'a str, Output = Font>> CompositeElement for SerdeElement<'a, ForceBreak, F> {
    fn element(&self, callback: impl CompositeElementCallback) {
        callback.call(&elements::force_break::ForceBreak);
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BreakWhole<E> {
    pub element: Box<E>,
}

impl<'a, E, F: Index<&'a str, Output = Font>> CompositeElement
    for SerdeElement<'a, BreakWhole<E>, F>
where
    SerdeElement<'a, E, F>: Element,
{
    fn element(&self, callback: impl CompositeElementCallback) {
        callback.call(&elements::break_whole::BreakWhole(&SerdeElement {
            element: &*self.element.element,
            fonts: self.fonts,
        }));
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MinFirstHeight<E> {
    pub element: Box<E>,
    pub min_first_height: f64,
}

impl<'a, E, F: Index<&'a str, Output = Font>> CompositeElement
    for SerdeElement<'a, MinFirstHeight<E>, F>
where
    SerdeElement<'a, E, F>: Element,
{
    fn element(&self, callback: impl CompositeElementCallback) {
        callback.call(&elements::min_first_height::MinFirstHeight {
            element: &SerdeElement {
                element: &*self.element.element,
                fonts: self.fonts,
            },
            min_first_height: self.element.min_first_height,
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AlignLocationBottom<E> {
    pub element: Box<E>,
}

impl<'a, E, F: Index<&'a str, Output = Font>> CompositeElement
    for SerdeElement<'a, AlignLocationBottom<E>, F>
where
    SerdeElement<'a, E, F>: Element,
{
    fn element(&self, callback: impl CompositeElementCallback) {
        callback.call(&elements::align_location_bottom::AlignLocationBottom(
            &SerdeElement {
                element: &*self.element.element,
                fonts: self.fonts,
            },
        ));
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AlignPreferredHeightBottom<E> {
    pub element: Box<E>,
}

impl<'a, E, F: Index<&'a str, Output = Font>> CompositeElement
    for SerdeElement<'a, AlignPreferredHeightBottom<E>, F>
where
    SerdeElement<'a, E, F>: Element,
{
    fn element(&self, callback: impl CompositeElementCallback) {
        callback.call(
            &elements::align_preferred_height_bottom::AlignPreferredHeightBottom(&SerdeElement {
                element: &*self.element.element,
                fonts: self.fonts,
            }),
        );
    }
}
