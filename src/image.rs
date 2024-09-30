use serde::{de::Visitor, Deserializer};

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
