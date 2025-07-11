use crate::*;

pub struct MaxWidth<E: Element> {
    pub element: E,
    pub max_width: f32,
}

impl<E: Element> Element for MaxWidth<E> {
    fn first_location_usage(&self, ctx: FirstLocationUsageCtx) -> FirstLocationUsage {
        self.element.first_location_usage(FirstLocationUsageCtx {
            width: self.width(ctx.width),
            ..ctx
        })
    }

    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        self.element.measure(MeasureCtx {
            width: self.width(ctx.width),
            ..ctx
        })
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        self.element.draw(DrawCtx {
            width: self.width(ctx.width),
            ..ctx
        })
    }
}

impl<E: Element> MaxWidth<E> {
    fn width(&self, width: WidthConstraint) -> WidthConstraint {
        WidthConstraint {
            max: width.max.min(self.max_width),
            expand: width.expand,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        elements::{svg::Svg, text::Text},
        fonts::builtin::BuiltinFont,
        test_utils::binary_snapshots::*,
    };
    use insta::*;

    #[test]
    fn test_text() {
        let bytes = test_element_bytes(
            TestElementParams {
                first_height: 10.,
                ..TestElementParams::breakable()
            },
            |mut callback| {
                let font = BuiltinFont::courier(callback.pdf());
                let element = MaxWidth {
                    element: Text::basic("Abcd Abcd Abcd", &font, 12.).debug(1),
                    max_width: 12.,
                };

                callback.call(&element.debug(0));
            },
        );
        assert_binary_snapshot!(".pdf", bytes);
    }

    #[test]
    fn test_image() {
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
            let element = MaxWidth {
                element: Svg { data: &tree },
                max_width: 105.,
            };

            callback.call(
                &element
                    .debug(0)
                    .show_max_width()
                    .show_last_location_max_height(),
            );
        });
        assert_binary_snapshot!(".pdf", bytes);
    }
}
