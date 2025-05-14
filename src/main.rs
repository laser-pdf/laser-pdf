use std::{
    collections::HashMap,
    io::{BufReader, BufWriter, Write},
    path::PathBuf,
    rc::Rc,
};

use laser_pdf::{
    BreakableDraw, DrawCtx, Element, Location, Pdf, WidthConstraint,
    fonts::truetype::TruetypeFont,
    serde_elements::{ElementValue, SerdeElementElement},
};
use pdf_writer::TextStr;
use serde::Deserialize;

#[derive(Deserialize)]
struct Entry {
    size: (f32, f32),
    fonts: HashMap<String, String>,
    element: ElementValue,
}

#[derive(Deserialize)]
struct Input {
    title: String,
    keywords: Option<String>,
    entries: Vec<Entry>,
}

fn main() {
    let input = serde_json::from_reader::<_, Input>(BufReader::new(std::io::stdin())).unwrap();

    let mut fonts: HashMap<PathBuf, Rc<TruetypeFont>> = HashMap::new();

    let mut pdf = Pdf::new();

    {
        let id = pdf.alloc();
        let mut document_info = pdf.pdf.document_info(id);
        document_info.title(TextStr(&input.title));

        if let Some(ref keywords) = input.keywords {
            document_info.keywords(TextStr(keywords));
        }
    }

    let mut load_font = |path: &str| {
        let path = std::fs::canonicalize(path).unwrap();

        if let Some(content) = fonts.get(&path) {
            content.clone()
        } else {
            // It's fine if these stay around till the end of the process.
            let bytes = std::fs::read(&path).unwrap().leak();
            let font = Rc::new(TruetypeFont::new(&mut pdf, bytes));

            fonts.insert(path, font.clone());
            font
        }
    };

    let font_maps: Vec<HashMap<&str, Rc<TruetypeFont>>> = input
        .entries
        .iter()
        .map(|entry| {
            let mut map: HashMap<&str, Rc<TruetypeFont>> = HashMap::new();
            for (k, v) in &entry.fonts {
                map.insert(k, load_font(&v));
            }

            map
        })
        .collect();

    let mut page_idx = 0;

    for (entry, font_map) in input.entries.iter().zip(font_maps.iter()) {
        let page_size = entry.size;

        let location = pdf.add_page((page_size.0, page_size.1));

        let entry_page = page_idx;

        let do_break = &mut |pdf: &mut Pdf, location_idx, _height| {
            while page_idx <= entry_page + location_idx {
                pdf.add_page((page_size.0, page_size.1));
                page_idx += 1;
            }

            Location {
                page_idx: (entry_page + location_idx + 1) as usize,
                layer_idx: 0,
                pos: (0., page_size.1),
                scale_factor: 1.,
            }
        };

        let ctx = DrawCtx {
            pdf: &mut pdf,
            width: WidthConstraint {
                max: page_size.0,
                expand: true,
            },
            location,

            first_height: page_size.1,
            preferred_height: None,

            breakable: Some(BreakableDraw {
                full_height: page_size.1,
                preferred_height_break_count: 0,
                do_break,
            }),
        };

        SerdeElementElement {
            element: &entry.element,
            fonts: font_map,
        }
        .draw(ctx);

        page_idx += 1;
    }

    BufWriter::new(std::io::stdout())
        .write_all(&pdf.finish())
        .unwrap();
}
