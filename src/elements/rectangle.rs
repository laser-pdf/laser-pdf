use pdf_writer::Name;

use crate::{utils::*, *};

pub struct Rectangle {
    pub size: (f32, f32),
    pub fill: Option<u32>,
    pub outline: Option<(f32, u32)>,
}

impl Element for Rectangle {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        let outline_thickness = outline_thickness(self);
        if ctx.break_appropriate_for_min_height(self.size.1 + outline_thickness) {
            FirstLocationUsage::WillSkip
        } else {
            FirstLocationUsage::WillUse
        }
    }

    fn measure(&self, mut ctx: MeasureCtx) -> ElementSize {
        let outline_thickness = outline_thickness(self);
        ctx.break_if_appropriate_for_min_height(self.size.1 + outline_thickness);

        size(self)
    }

    fn draw(&self, mut ctx: DrawCtx) -> ElementSize {
        let outline_thickness = outline_thickness(self);
        ctx.break_if_appropriate_for_min_height(self.size.1 + outline_thickness);

        let extra_outline_offset = outline_thickness as f32 / 2.0;

        let resource_id;

        let fill_alpha = self
            .fill
            .map(|c| u32_to_color_and_alpha(c).1)
            .filter(|&a| a != 1.);

        let outline_alpha = self
            .outline
            .map(|(_, c)| u32_to_color_and_alpha(c).1)
            .filter(|&a| a != 1.);

        if fill_alpha.is_some() || outline_alpha.is_some() {
            let ext_graphics_ref = ctx.pdf.alloc();

            let mut ext_graphics = ctx.pdf.pdf.ext_graphics(ext_graphics_ref);
            fill_alpha.inspect(|&a| {
                ext_graphics.non_stroking_alpha(a);
            });
            outline_alpha.inspect(|&a| {
                ext_graphics.stroking_alpha(a);
            });

            resource_id =
                Some(ctx.pdf.pages[ctx.location.page_idx].add_ext_g_state(ext_graphics_ref));
        } else {
            resource_id = None;
        }

        let layer = ctx.location.layer(ctx.pdf);

        layer.save_state();

        if let Some(color) = self.fill {
            let (color, _) = u32_to_color_and_alpha(color);

            layer.set_fill_rgb(color[0], color[1], color[2]);
        }

        if let Some((thickness, color)) = self.outline {
            layer.set_line_width(mm_to_pt(thickness) as f32);

            set_fill_color(layer, color);
        }

        if let Some(ext_graphics) = resource_id {
            layer.set_parameters(Name(format!("{}", ext_graphics).as_bytes()));
        }

        layer.rect(
            mm_to_pt(ctx.location.pos.0 as f32 + extra_outline_offset),
            mm_to_pt((ctx.location.pos.1 - self.size.1) as f32 - extra_outline_offset),
            mm_to_pt(self.size.0 as f32),
            mm_to_pt(self.size.1 as f32),
        );

        match (self.fill.is_some(), self.outline.is_some()) {
            (true, true) => layer.fill_nonzero_and_stroke(),
            (true, false) => layer.fill_nonzero(),
            (false, true) => layer.stroke(),
            (false, false) => layer,
        };

        layer.restore_state();

        size(self)
    }
}

fn outline_thickness(rectangle: &Rectangle) -> f32 {
    rectangle.outline.map(|o| o.0).unwrap_or(0.0)
}

fn size(rectangle: &Rectangle) -> ElementSize {
    let outline_thickness = outline_thickness(rectangle);

    ElementSize {
        width: Some(rectangle.size.0 + outline_thickness),
        height: Some(rectangle.size.1 + outline_thickness),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    #[test]
    fn test_rectangle() {
        for output in (ElementTestParams {
            first_height: 12.,
            ..Default::default()
        })
        .run(&Rectangle {
            size: (11., 12.),
            fill: None,
            outline: Some((1., 0)),
        }) {
            output.assert_size(ElementSize {
                width: Some(12.),
                height: Some(13.),
            });

            if let Some(b) = output.breakable {
                if output.first_height == 12. {
                    b.assert_break_count(1);
                } else {
                    b.assert_break_count(0);
                }

                b.assert_extra_location_min_height(None);
            }
        }
    }
}
