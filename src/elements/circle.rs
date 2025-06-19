use kurbo::Shape;

use crate::{utils::*, *};

/// A circular shape element with optional fill and outline.
/// 
/// The circle is rendered with the specified radius and can have both
/// a fill color and an outline with configurable thickness and color.
pub struct Circle {
    /// Radius of the circle in millimeters
    pub radius: f32,
    /// Optional fill color as RGBA (None for transparent)
    pub fill: Option<u32>,
    /// Optional outline as (thickness_mm, color_rgba)
    pub outline: Option<(f32, u32)>,
}

impl Element for Circle {
    fn measure(&self, mut ctx: MeasureCtx) -> ElementSize {
        let outline_thickness = outline_thickness(self);
        ctx.break_if_appropriate_for_min_height(self.radius * 2. + outline_thickness);

        size(self)
    }

    fn draw(&self, mut ctx: DrawCtx) -> ElementSize {
        let outline_thickness = outline_thickness(self);
        ctx.break_if_appropriate_for_min_height(self.radius * 2. + outline_thickness);

        let extra_outline_offset = outline_thickness / 2.0;

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
            set_fill_color(layer, color);
        }

        if let Some((thickness, color)) = self.outline {
            layer.set_line_width(mm_to_pt(thickness) as f32);

            set_stroke_color(layer, color);
        }

        if let Some(ext_graphics) = resource_id {
            layer.set_parameters(Name(format!("{}", ext_graphics).as_bytes()));
        }

        let shape = kurbo::Circle::new(
            (
                mm_to_pt(ctx.location.pos.0 + self.radius + extra_outline_offset) as f64,
                mm_to_pt(ctx.location.pos.1 - self.radius - extra_outline_offset) as f64,
            ),
            mm_to_pt(self.radius) as f64,
        );

        let els = shape.path_elements(0.1);

        let mut closed = false;

        for el in els {
            use kurbo::PathEl::*;

            match el {
                MoveTo(point) => {
                    layer.move_to(point.x as f32, point.y as f32);
                }
                LineTo(point) => {
                    layer.line_to(point.x as f32, point.y as f32);
                }
                QuadTo(a, b) => {
                    layer.cubic_to_initial(a.x as f32, a.y as f32, b.x as f32, b.y as f32);
                }
                CurveTo(a, b, c) => {
                    layer.cubic_to(
                        a.x as f32, a.y as f32, b.x as f32, b.y as f32, c.x as f32, c.y as f32,
                    );
                }
                ClosePath => closed = true,
            };
        }

        assert!(closed);

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

fn outline_thickness(circle: &Circle) -> f32 {
    circle.outline.map(|o| o.0).unwrap_or(0.0)
}

fn size(circle: &Circle) -> ElementSize {
    let outline_thickness = outline_thickness(circle);

    let size = circle.radius * 2. + outline_thickness;

    ElementSize {
        width: Some(size),
        height: Some(size),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::*;
    use test_utils::binary_snapshots::*;

    #[test]
    fn test_circle() {
        use crate::test_utils::*;

        for output in (ElementTestParams {
            first_height: 11.,
            ..Default::default()
        })
        .run(&Circle {
            radius: 5.5,
            fill: None,
            outline: Some((1., 0)),
        }) {
            output.assert_size(ElementSize {
                width: Some(12.),
                height: Some(12.),
            });

            if let Some(b) = output.breakable {
                if output.first_height == 11. {
                    b.assert_break_count(1);
                } else {
                    b.assert_break_count(0);
                }

                b.assert_extra_location_min_height(None);
            }
        }
    }

    #[test]
    fn test() {
        let bytes = test_element_bytes(TestElementParams::breakable(), |callback| {
            callback.call(
                &Circle {
                    radius: 52.5,
                    fill: Some(0x00_FF_00_77),
                    outline: Some((12., 0x00_00_FF_44)),
                }
                .debug(0)
                .show_max_width()
                .show_last_location_max_height(),
            );
        });
        assert_binary_snapshot!(".pdf", bytes);
    }
}
