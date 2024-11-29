use printpdf::*;

use crate::utils::*;
use crate::*;

pub struct Svg<'a> {
    pub tree: &'a usvg::Tree,
    // pub data: &'a str,
}

impl<'a> Widget for Svg<'a> {
    fn widget(&self, width: Option<f32>, draw: Option<DrawCtx>) -> [f32; 2] {
        if let Some(context) = draw {
            let pos = context.location.pos;
            context
                .location
                .layer
                .set_ctm(CurTransMat::Translate(Mm(pos[0]), Mm(pos[1])));
            context
                .location
                .layer
                .set_ctm(CurTransMat::Scale(0.25, -0.25));
            context.location.layer.add_svg(self.tree);
            // let root = &self.tree.root();
            // render_node(context.pdf, &context.location, root);
        }

        [0.0; 2]
    }
}

// fn render_node(
//     pdf: &mut Pdf,
//     location: &DrawPos,
//     node: &usvg::Node,
// ) -> Option<[f32; 2]> {
//     match *node.borrow() {
//         usvg::NodeKind::Svg(_) => render_group(pdf, location, node),
//         usvg::NodeKind::Path(ref path) => render_path(pdf, location, path),
//         usvg::NodeKind::Group(ref g) => {
//             location.layer.save_graphics_state();
//             apply_transform(&location.layer, g.transform);
//             let ret = render_group(pdf, location, node);
//             location.layer.restore_graphics_state();
//             ret
//         }
//         _ => None,
//     }
// }

// fn render_group(
//     pdf: &mut Pdf,
//     location: &DrawPos,
//     parent: &usvg::Node,
// ) -> Option<[f32; 2]> {
//     for node in parent.children() {
//         render_node(pdf, location, &node);
//     }
//     None
// }

// fn apply_transform(layer: &PdfLayerReference, transform: svgtypes::Transform) {
//     use lopdf::Object::Real;

//     layer.add_op(Operation::new("cm", vec![
//         Real(transform.a),
//         Real(transform.b),
//         Real(transform.c),
//         Real(transform.d),
//         Real(transform.e),
//         Real(transform.f),
//     ]));
// }

// fn render_path(pdf: &mut Pdf, location: &DrawPos, path: &usvg::Path) -> Option<[f32; 2]> {
//     location.layer.save_graphics_state();
//     if let Some(usvg::Fill { paint: usvg::Paint::Color(color), ref opacity, ref rule }) = path.fill {
//         location.layer.set_fill_color(printpdf::Color::Rgb(printpdf::Rgb::new(
//             color.red as f32 / 255.0,
//             color.green as f32 / 255.0,
//             color.blue as f32 / 255.0,
//             None,
//         )));

//         // location.layer.set_fill_alpha(opacity.value());
//     }

//     if let Some(ref stroke) = path.stroke {
//         let dash_array = stroke.dasharray.as_deref().unwrap_or(&[]);
//         let dash_phase = stroke.dashoffset;
//         location.layer.add_op(Operation::new("d", vec![
//             Object::Array(dash_array.iter().map(|d| Object::Integer(*d as i64)).collect()),
//             Object::Integer(dash_phase as i64),
//         ]));
//         location.layer.set_outline_thickness(stroke.width.value());

//         location.layer.set_line_cap_style(match stroke.linecap {
//             usvg::LineCap::Butt => LineCapStyle::Butt,
//             usvg::LineCap::Round => LineCapStyle::Round,
//             usvg::LineCap::Square => LineCapStyle::ProjectingSquare,
//         });
//     }

//     let mut ops: Vec<lopdf::content::Operation> = Vec::new();

//     let mut closed = false;

//     apply_transform(&location.layer, path.transform);

//     for s in path.data.iter() {
//         match s {
//             &PathSegment::MoveTo { x, y } => ops.push(Operation::new("m", vec![x.into(), y.into()])),
//             &PathSegment::LineTo { x, y } => ops.push(Operation::new("l", vec![x.into(), y.into()])),
//             &PathSegment::CurveTo { x1, y1, x2, y2, x, y } => ops.push(Operation::new("c", vec![
//                 x1.into(), y1.into(),
//                 x2.into(), y2.into(),
//                 x.into(), y.into(),
//             ])),
//             &PathSegment::ClosePath => closed = true,
//         }
//     }

//     // TODO: Check fill and stroke combination
//     match (path.stroke.is_some(), path.fill.is_some(), closed) {
//         (true, true, true) => ops.push(Operation::new("b", Vec::new())),
//         (true, true, false) => ops.push(Operation::new("f", Vec::new())),
//         (true, false, true) => ops.push(Operation::new("s", Vec::new())),
//         (true, false, false) => ops.push(Operation::new("S", Vec::new())),
//         (false, true, _) => ops.push(Operation::new("f", Vec::new())),
//         _ => ops.push(Operation::new("n", Vec::new())),
//     }

//     location.layer.add_ops(ops);
//     location.layer.restore_graphics_state();

//     // let ops = path.data.iter().map(|s: &PathSegment| match s {
//     //     &PathSegment::MoveTo { x, y } => Operation::new("m", vec![x.into(), y.into()]),
//     //     &PathSegment::LineTo { x, y } => Operation::new("l", vec![x.into(), y.into()]),
//     //     &PathSegment::CurveTo { x1, y1, x2, y2, x, y } => Operation::new("c", vec![
//     //         x1.into(), y1.into(),
//     //         x2.into(), y2.into(),
//     //         x.into(), y.into(),
//     //     ]),
//     //     &PathSegment::ClosePath => Operation::new(if let Some(ref stroke) = path.stroke {
//     //         if stroke.
//     //         ""
//     //     } else {
//     //         ""
//     //     }, vec![]),
//     // });

//     None
// }
