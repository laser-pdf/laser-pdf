use printpdf::image::{DynamicImage, GenericImageView};

use crate::{image::Image, *};

use super::svg::Svg;

const INCH_TO_MM: f64 = 25.4;

pub struct ImageElement<'a> {
    image: &'a Image,
}

impl<'a> Element for ImageElement<'a> {
    fn insufficient_first_height(&self, ctx: InsufficientFirstHeightCtx) -> bool {
        match self.image {
            Image::Svg(svg) => Svg { data: svg }.insufficient_first_height(ctx),
            Image::Pixel(image) => {
                let (height, _, _) = calculate_size(image, ctx.width);

                ctx.break_appropriate_for_min_height(height)
            }
        }
    }

    fn measure(&self, mut ctx: MeasureCtx) -> Option<ElementSize> {
        match self.image {
            Image::Svg(svg) => Svg { data: svg }.measure(ctx),
            Image::Pixel(image) => {
                let (height, _, element_size) = calculate_size(image, ctx.width);

                ctx.break_if_appropriate_for_min_height(height);

                Some(element_size)
            }
        }
    }

    fn draw(&self, mut ctx: DrawCtx) -> Option<ElementSize> {
        match self.image {
            Image::Svg(svg) => Svg { data: svg }.draw(ctx),
            Image::Pixel(image) => {
                let (height, scale, element_size) = calculate_size(image, ctx.width);

                ctx.break_if_appropriate_for_min_height(height);

                let image = printpdf::Image::from_dynamic_image(image);

                image.add_to_layer(
                    ctx.location.layer,
                    Some(Mm(ctx.location.pos.0)),
                    Some(Mm(ctx.location.pos.1 - height)),
                    None,
                    Some(scale),
                    Some(scale),
                    Some(1.0),
                );

                Some(element_size)
            }
        }
    }
}

#[inline]
fn calculate_size(image: &DynamicImage, width: Option<f64>) -> (f64, f64, ElementSize) {
    let dimensions = {
        let (x, y) = image.dimensions();
        (x as f64 * INCH_TO_MM, y as f64 * INCH_TO_MM)
    };

    let (size, scale) = if let Some(width) = width {
        (
            (width, dimensions.1 * width / dimensions.0),
            width / dimensions.0,
        )
    } else {
        (dimensions, 1.0)
    };

    (
        size.1,
        scale,
        ElementSize {
            width: size.0,
            height: Some(size.1),
        },
    )
}
