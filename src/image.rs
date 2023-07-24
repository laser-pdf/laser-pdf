use serde::{de::Visitor, Deserializer};

use printpdf::*;

use printpdf::image::{png::PngDecoder, ImageDecoder};

use crate::utils::*;
use crate::*;

const INCH_TO_MM: f64 = 25.4;

pub fn deserialize_buffer<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    struct FileVisitor;

    impl<'de> Visitor<'de> for FileVisitor {
        type Value = Vec<u8>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid path")
        }

        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
            std::fs::read(v).map_err(|e| E::custom(e))
        }

        fn visit_borrowed_str<E: serde::de::Error>(self, v: &'de str) -> Result<Self::Value, E> {
            std::fs::read(v).map_err(|e| E::custom(e))
        }

        fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
            std::fs::read(v).map_err(|e| E::custom(e))
        }
    }

    Ok(deserializer.deserialize_str(FileVisitor)?)
}

pub fn deserialize_svg<'de, D: Deserializer<'de>>(deserializer: D) -> Result<usvg::Tree, D::Error> {
    struct SvgVisitor;

    impl<'de> Visitor<'de> for SvgVisitor {
        type Value = usvg::Tree;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid svg")
        }

        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
            usvg::Tree::from_str(v, &Default::default()).map_err(|e| E::custom(e))
        }

        fn visit_borrowed_str<E: serde::de::Error>(self, v: &'de str) -> Result<Self::Value, E> {
            usvg::Tree::from_str(v, &Default::default()).map_err(|e| E::custom(e))
        }

        fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
            usvg::Tree::from_str(&v, &Default::default()).map_err(|e| E::custom(e))
        }
    }

    Ok(deserializer.deserialize_str(SvgVisitor)?)
}

pub fn deserialize_svg_from_path<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<usvg::Tree, D::Error> {
    struct SvgVisitor;

    impl<'de> Visitor<'de> for SvgVisitor {
        type Value = usvg::Tree;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid svg")
        }

        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
            usvg::Tree::from_file(v, &Default::default()).map_err(|e| E::custom(e))
        }

        fn visit_borrowed_str<E: serde::de::Error>(self, v: &'de str) -> Result<Self::Value, E> {
            usvg::Tree::from_file(v, &Default::default()).map_err(|e| E::custom(e))
        }

        fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
            usvg::Tree::from_file(&v, &Default::default()).map_err(|e| E::custom(e))
        }
    }

    Ok(deserializer.deserialize_str(SvgVisitor)?)
}

#[derive(Clone)]
pub enum Image {
    Svg(usvg::Tree),
    Pixel(printpdf::image::DynamicImage),
}

pub fn deserialize_image<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Image, D::Error> {
    struct ImageVisitor;

    fn visit<E: serde::de::Error>(path: impl AsRef<std::path::Path>) -> Result<Image, E> {
        if path.as_ref().extension().map_or(false, |e| e == "svg") {
            Ok(Image::Svg(
                usvg::Tree::from_file(path, &Default::default()).map_err(E::custom)?,
            ))
        } else {
            Ok(Image::Pixel(
                printpdf::image::open(path).map_err(E::custom)?,
            ))
        }
    }

    impl<'de> Visitor<'de> for ImageVisitor {
        type Value = Image;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid image")
        }

        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
            visit(v)
        }

        fn visit_borrowed_str<E: serde::de::Error>(self, v: &'de str) -> Result<Self::Value, E> {
            visit(v)
        }

        fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
            visit(v)
        }
    }

    Ok(deserializer.deserialize_str(ImageVisitor)?)
}

pub fn image<'a>(image: &'a Image) -> impl Element + 'a {
    move |width: Option<f64>, draw: Option<DrawContext>| match image {
        Image::Svg(svg) => SvgWidget { data: svg }.element(width, draw),
        Image::Pixel(image) => {
            use printpdf::image::GenericImageView;

            let dimensions = {
                let (x, y) = image.dimensions();
                [x as f64 * INCH_TO_MM, y as f64 * INCH_TO_MM]
            };

            let (size, scale) = if let Some(width) = width {
                (
                    [width, dimensions[1] * width / dimensions[0]],
                    width / dimensions[0],
                )
            } else {
                (dimensions, 1.0)
            };

            if let Some(context) = draw {
                let image = printpdf::Image::from_dynamic_image(image);

                image.add_to_layer(
                    context.draw_pos.layer,
                    Some(Mm(context.draw_pos.pos[0])),
                    Some(Mm(context.draw_pos.pos[1] - size[1])),
                    None,
                    Some(scale),
                    Some(scale),
                    Some(1.0),
                );
            }

            size
        }
    }
}

pub struct PngWidget<'a> {
    pub bytes: &'a [u8],
}

impl<'a> Element for PngWidget<'a> {
    fn element(&self, width: Option<f64>, draw: Option<DrawContext>) -> [f64; 2] {
        let decoder = PngDecoder::new(std::io::Cursor::new(self.bytes)).unwrap();
        let dimensions = {
            let (x, y) = decoder.dimensions();
            [x as f64 * INCH_TO_MM, y as f64 * INCH_TO_MM]
        };

        let (size, scale) = if let Some(width) = width {
            (
                [width, dimensions[1] * width / dimensions[0]],
                width / dimensions[0],
            )
        } else {
            (dimensions, 1.0)
        };

        if let Some(context) = draw {
            let image = printpdf::Image::try_from(decoder).unwrap();

            image.add_to_layer(
                context.draw_pos.layer,
                Some(Mm(context.draw_pos.pos[0])),
                Some(Mm(context.draw_pos.pos[1] - size[1])),
                None,
                Some(scale),
                Some(scale),
                Some(1.0),
            );
        }

        size
    }
}

pub struct SvgWidget<'a> {
    pub data: &'a usvg::Tree,
}

impl<'a> Element for SvgWidget<'a> {
    fn element(&self, width: Option<f64>, draw: Option<DrawContext>) -> [f64; 2] {
        let svg = self.data.svg_node();
        let svg_size = svg.size;
        let svg_width = pt_to_mm(svg_size.width());
        let svg_height = pt_to_mm(svg_size.height());

        let (width, height, scale_factor) = if let Some(width) = width {
            let scale_factor = width / svg_width;

            (width, svg_height * scale_factor, scale_factor)
        } else {
            (svg_width, svg_height, 1.0)
        };

        if let Some(context) = draw {
            let view_box_scale = {
                let rect = svg.view_box.rect;
                [
                    svg_size.width() / rect.width(),
                    svg_size.height() / rect.height(),
                ]
            };

            let pos = context.draw_pos.pos;
            let layer = &context.draw_pos.layer;

            layer.save_graphics_state();
            layer.set_ctm(CurTransMat::Translate(Mm(pos[0]), Mm(pos[1])));

            // invert coordinate space and apply scale
            // the reason this isn't just one call is that lopdf is rounding real numbers to two
            // decimal digits so calling `set_ctm` twice will me more precise
            layer.set_ctm(CurTransMat::Scale(
                scale_factor,  // * view_box_scale[0],
                -scale_factor, // * view_box_scale[1],
            ));
            layer.set_ctm(CurTransMat::Scale(view_box_scale[0], view_box_scale[1]));

            layer.add_svg(&self.data);

            layer.restore_graphics_state();
        }

        [width, height]
    }
}
