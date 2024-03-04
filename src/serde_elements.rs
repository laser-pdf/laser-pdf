pub mod elements;

use std::{ops::Index, rc::Rc};

use crate::fonts::truetype::TruetypeFont;
use elements::*;

pub type Font = Rc<TruetypeFont<Vec<u8>>>;

pub struct SerdeElement<'a, E, F: Index<&'a str, Output = Font>> {
    element: &'a E,
    fonts: &'a F,
}

macro_rules! element_value {
    ($enum_name:ident => {$($type:ident $(<$($rest:ident),*>)*),*,}) => {
        #[derive(Clone, serde::Deserialize)]
        pub enum $enum_name {
            $($type ($type $(<$($rest)*>)*)),*
        }

        impl<'a, F: Index<&'a str, Output = Font>> $crate::CompositeElement for
            SerdeElement<'a, $enum_name, F>
        {
            fn element(&self, callback: impl $crate::CompositeElementCallback) {
                match self.element {
                    $($enum_name::$type(ref val) => callback.call(&SerdeElement {
                        element: val,
                        fonts: self.fonts,
                    })),*
                }
            }
        }
    };
}

element_value!(ElementValue => {
    None,
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
    ForceBreak,
    BreakWhole<ElementValue>,
    MinFirstHeight<ElementValue>,
    AlignLocationBottom<ElementValue>,
    AlignPreferredHeightBottom<ElementValue>,
});
