pub mod elements;

use std::{ops::Index, rc::Rc};

use crate::{fonts::truetype::TruetypeFont, CompositeElement, CompositeElementCallback};
use elements::*;

pub type Font = Rc<TruetypeFont<'static>>;

pub trait SerdeElement {
    fn element(
        &self,
        fonts: &impl for<'a> Index<&'a str, Output = Font>,
        callback: impl CompositeElementCallback,
    );
}

pub struct SerdeElementElement<'a, E: SerdeElement, F: for<'b> Index<&'b str, Output = Font>> {
    pub element: &'a E,
    pub fonts: &'a F,
}

impl<'a, E: SerdeElement, F: for<'b> Index<&'b str, Output = Font>> CompositeElement
    for SerdeElementElement<'a, E, F>
{
    fn element(&self, callback: impl CompositeElementCallback) {
        self.element.element(self.fonts, callback);
    }
}

#[macro_export]
macro_rules! define_serde_element_value {
    ($enum_name:ident {$($type:ident $(<$($rest:ident),*>)*),*,}) => {
        #[derive(Clone, serde::Deserialize)]
        pub enum $enum_name {
            $($type ($type $(<$($rest)*>)*)),*
        }

        impl $crate::serde_elements::SerdeElement for $enum_name {
            fn element(
                &self,
                fonts: &impl for<'a> core::ops::Index<&'a str, Output = $crate::serde_elements::Font>,
                callback: impl $crate::CompositeElementCallback,
            ) {
                match self {
                    $($enum_name::$type(ref val) => $crate::serde_elements::SerdeElement
                        ::element(val, fonts, callback)),*
                }
            }
        }
    };
}

define_serde_element_value!(ElementValue {
    None,
    Debug<ElementValue>,
    Text,
    RichText,
    VGap,
    HAlign<ElementValue>,
    Padding<ElementValue>,
    StyledBox<ElementValue>,
    Line,
    Image,
    Rectangle,
    Circle,
    Column<ElementValue>,
    Row<ElementValue>,
    BreakList<ElementValue>,
    Stack<ElementValue>,
    TableRow<ElementValue>,
    Titled<ElementValue>,
    TitleOrBreak<ElementValue>,
    RepeatAfterBreak<ElementValue>,
    RepeatBottom<ElementValue>,
    PinBelow<ElementValue>,
    ForceBreak,
    BreakWhole<ElementValue>,
    MinFirstHeight<ElementValue>,
    AlignLocationBottom<ElementValue>,
    AlignPreferredHeightBottom<ElementValue>,
    ExpandToPreferredHeight<ElementValue>,
    ShrinkToFit<ElementValue>,
    Rotate<ElementValue>,
});
