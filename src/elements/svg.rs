use printpdf::CurTransMat;

use crate::{utils::pt_to_mm, *};

pub struct Svg<'a> {
    pub data: &'a usvg::Tree,
}

impl<'a> Element for Svg<'a> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        let (height, _, _) = calculate_size(self.data, ctx.width);

        if ctx.break_appropriate_for_min_height(height) {
            FirstLocationUsage::WillSkip
        } else {
            FirstLocationUsage::WillUse
        }
    }

    fn measure(&self, mut ctx: MeasureCtx) -> Option<ElementSize> {
        let (height, _, element_size) = calculate_size(self.data, ctx.width);

        ctx.break_if_appropriate_for_min_height(height);

        Some(element_size)
    }

    fn draw(&self, mut ctx: DrawCtx) -> Option<ElementSize> {
        let (height, scale_factor, element_size) = calculate_size(self.data, ctx.width);

        ctx.break_if_appropriate_for_min_height(height);

        let svg = self.data.svg_node();
        let svg_size = svg.size;

        let view_box_scale = {
            let rect = svg.view_box.rect;
            [
                svg_size.width() / rect.width(),
                svg_size.height() / rect.height(),
            ]
        };

        let pos = ctx.location.pos;
        let layer = &ctx.location.layer;

        layer.save_graphics_state();
        layer.set_ctm(CurTransMat::Translate(Mm(pos.0), Mm(pos.1)));

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

        Some(element_size)
    }
}

#[inline]
fn calculate_size(data: &usvg::Tree, width: WidthConstraint) -> (f64, f64, ElementSize) {
    let svg = data.svg_node();
    let svg_size = svg.size;
    let svg_width = pt_to_mm(svg_size.width());
    let svg_height = pt_to_mm(svg_size.height());

    let width = width.constrain(svg_width);
    let scale_factor = width / svg_width;
    let height = svg_height * scale_factor;

    (
        height,
        scale_factor,
        ElementSize {
            width,
            height: Some(height),
        },
    )
}
