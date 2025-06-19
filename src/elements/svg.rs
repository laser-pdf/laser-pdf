use utils::mm_to_pt;

use crate::{utils::pt_to_mm, *};

/// An SVG graphics element that renders vector graphics from a usvg tree.
///
/// The SVG is automatically scaled to fit the available width while maintaining its aspect ratio.
/// Uses the `svg2pdf` crate for rendering the parsed `usvg` tree.
pub struct Svg<'a> {
    /// Reference to the parsed SVG tree
    pub data: &'a usvg::Tree,
}

impl<'a> Element for Svg<'a> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        let (height, _) = calculate_size(self.data, ctx.width);

        if ctx.break_appropriate_for_min_height(height) {
            FirstLocationUsage::WillSkip
        } else {
            FirstLocationUsage::WillUse
        }
    }

    fn measure(&self, mut ctx: MeasureCtx) -> ElementSize {
        let (height, element_size) = calculate_size(self.data, ctx.width);

        ctx.break_if_appropriate_for_min_height(height);

        element_size
    }

    fn draw(&self, mut ctx: DrawCtx) -> ElementSize {
        let (height, element_size) = calculate_size(self.data, ctx.width);

        ctx.break_if_appropriate_for_min_height(height);

        let pos = ctx.location.pos;

        let (svg_chunk, svg_id) = svg2pdf::to_chunk(
            self.data,
            svg2pdf::ConversionOptions {
                compress: false,
                raster_scale: 1.5,
                embed_text: false,
                pdfa: true,
            },
        )
        .unwrap();

        let offset = ctx.pdf.alloc.get();
        let mut max = offset;

        // We want to avoid using a hashmap here for the mapping. This assumes, of course, that
        // there aren't any huge gaps in the id space of the chunk. They'd have to be so huge that
        // we'd be very close to the end of the i32 space though, so this being an actual problem
        // is extremely unlikely. Also, as far as I can tell, svg2pdf seems to just use a bumping id
        // allocator internally as well, so this should not be a problem at all.
        svg_chunk.renumber_into(&mut ctx.pdf.pdf, |old| {
            let val = offset + old.get();
            max = max.max(val);
            pdf_writer::Ref::new(val)
        });

        let svg_id = pdf_writer::Ref::new(offset + svg_id.get());
        ctx.pdf.alloc = pdf_writer::Ref::new(max + 1);

        let x_object = ctx.pdf.pages[ctx.location.page_idx].add_x_object(svg_id);

        let layer = ctx.location.layer(ctx.pdf);

        layer
            .save_state()
            .transform([
                mm_to_pt(element_size.width.unwrap()),
                0.,
                0.,
                mm_to_pt(element_size.height.unwrap()),
                mm_to_pt(pos.0),
                mm_to_pt(pos.1 - element_size.height.unwrap()),
            ])
            .x_object(Name(x_object.as_bytes()))
            .restore_state();

        element_size
    }
}

#[inline]
fn calculate_size(data: &usvg::Tree, width: WidthConstraint) -> (f32, ElementSize) {
    let svg = data;
    let svg_size = svg.size();
    let svg_width = pt_to_mm(svg_size.width() as f32);
    let svg_height = pt_to_mm(svg_size.height() as f32);

    let width = width.constrain(svg_width);
    let scale_factor = width / svg_width;
    let height = svg_height * scale_factor;

    (
        height,
        ElementSize {
            width: Some(width),
            height: Some(height),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::*;
    use test_utils::binary_snapshots::*;

    #[test]
    fn test() {
        const SVG: &str = "\
            <svg
               width=\"512\"
               height=\"512\"
               viewBox=\"0 0 135.46666 135.46667\"
               version=\"1.1\"
               id=\"svg1\"
               xmlns=\"http://www.w3.org/2000/svg\"
               xmlns:svg=\"http://www.w3.org/2000/svg\">
              <defs
                 id=\"defs1\" />
              <g
                 id=\"layer1\">
                <rect
                   style=\"fill:#000f80;fill-opacity:1;stroke:none;stroke-width:1.3386\"
                   id=\"rect1\"
                   width=\"108.92857\"
                   height=\"72.85714\"
                   x=\"18.571426\"
                   y=\"11.428572\" />
                <ellipse
                   style=\"fill:#008080;fill-opacity:1;stroke:none;stroke-width:2.07092\"
                   id=\"path1\"
                   cx=\"84.107147\"
                   cy=\"84.107132\"
                   rx=\"51.964283\"
                   ry=\"46.250004\" />
              </g>
            </svg>
        ";

        let tree = usvg::Tree::from_str(
            SVG,
            &usvg::Options {
                ..Default::default()
            },
        )
        .unwrap();

        let bytes = test_element_bytes(TestElementParams::breakable(), |callback| {
            callback.call(
                &Svg { data: &tree }
                    .debug(0)
                    .show_max_width()
                    .show_last_location_max_height(),
            );
        });
        assert_binary_snapshot!(".pdf", bytes);
    }
}
