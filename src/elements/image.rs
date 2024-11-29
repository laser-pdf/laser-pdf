use miniz_oxide::deflate::{compress_to_vec_zlib, CompressionLevel};
use pdf_writer::Filter;
use printpdf::image::{DynamicImage, GenericImageView};
use utils::mm_to_pt;

use crate::{image::Image, *};

use super::svg::Svg;

const INCH_TO_MM: f32 = 25.4;

pub struct ImageElement<'a> {
    pub image: &'a Image,
}

impl<'a> Element for ImageElement<'a> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        match self.image {
            Image::Svg(svg) => Svg { data: svg }.first_location_usage(ctx),
            Image::Pixel(image) => {
                let (height, _) = calculate_size(image, ctx.width);

                if ctx.break_appropriate_for_min_height(height) {
                    FirstLocationUsage::WillSkip
                } else {
                    FirstLocationUsage::WillUse
                }
            }
        }
    }

    fn measure(&self, mut ctx: MeasureCtx) -> ElementSize {
        match self.image {
            Image::Svg(svg) => Svg { data: svg }.measure(ctx),
            Image::Pixel(image) => {
                let (height, element_size) = calculate_size(image, ctx.width);

                ctx.break_if_appropriate_for_min_height(height);

                element_size
            }
        }
    }

    fn draw(&self, mut ctx: DrawCtx) -> ElementSize {
        match self.image {
            Image::Svg(svg) => Svg { data: svg }.draw(ctx),
            Image::Pixel(image) => {
                let (height, element_size) = calculate_size(image, ctx.width);

                ctx.break_if_appropriate_for_min_height(height);

                // a bit of a copy-paste from
                // https://github.com/typst/pdf-writer/blob/main/examples/image.rs

                // Define some indirect reference ids we'll use.
                let image_id = ctx.pdf.alloc();
                let s_mask_id = ctx.pdf.alloc();
                let image_name = ctx.pdf.pages[ctx.location.page_idx].add_x_object(image_id);

                let dynamic = image;

                // Now, there are multiple considerations:
                // - Writing an XObject with just the raw samples would work, but lead to
                //   huge file sizes since the image would be embedded without any
                //   compression.
                // - We can encode the samples with a filter. However, which filter is best
                //   depends on the file format. For example, for JPEGs you should use
                //   DCT-Decode and for PNGs you should use Deflate.
                // - When the image has transparency, we need to provide that separately
                //   through an extra linked SMask image.
                let level = CompressionLevel::DefaultLevel as u8;
                let encoded = compress_to_vec_zlib(dynamic.to_rgb8().as_raw(), level);

                // If there's an alpha channel, extract the pixel alpha values.
                let mask = dynamic.color().has_alpha().then(|| {
                    let alphas: Vec<_> = dynamic.pixels().map(|p| (p.2).0[3]).collect();
                    compress_to_vec_zlib(&alphas, level)
                });
                let (filter, encoded, mask) = (Filter::FlateDecode, encoded, mask);

                // Write the stream for the image we want to embed.
                let mut image = ctx.pdf.pdf.image_xobject(image_id, &encoded);
                image.filter(filter);
                image.width(dynamic.width() as i32);
                image.height(dynamic.height() as i32);
                image.color_space().device_rgb();
                image.bits_per_component(8);
                if mask.is_some() {
                    image.s_mask(s_mask_id);
                }
                drop(image);

                // Add SMask if the image has transparency.
                if let Some(encoded) = &mask {
                    let mut s_mask = ctx.pdf.pdf.image_xobject(s_mask_id, encoded);
                    s_mask.filter(filter);
                    s_mask.width(dynamic.width() as i32);
                    s_mask.height(dynamic.height() as i32);
                    s_mask.color_space().device_gray();
                    s_mask.bits_per_component(8);
                }

                ctx.location
                    .layer(ctx.pdf)
                    .save_state()
                    .transform([
                        mm_to_pt(element_size.width.unwrap()),
                        0.,
                        0.,
                        mm_to_pt(element_size.height.unwrap()),
                        mm_to_pt(ctx.location.pos.0),
                        mm_to_pt(ctx.location.pos.1 - element_size.height.unwrap()),
                    ])
                    .x_object(Name(image_name.as_bytes()))
                    .restore_state();

                element_size
            }
        }
    }
}

#[inline]
fn calculate_size(image: &DynamicImage, width: WidthConstraint) -> (f32, ElementSize) {
    let dimensions = {
        let (x, y) = image.dimensions();
        (x as f32 * INCH_TO_MM, y as f32 * INCH_TO_MM)
    };

    let width = width.constrain(dimensions.0);

    let size = (width, dimensions.1 * width / dimensions.0);

    (
        size.1,
        ElementSize {
            width: Some(size.0),
            height: Some(size.1),
        },
    )
}
