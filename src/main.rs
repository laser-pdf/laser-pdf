use std::{
    collections::HashMap,
    io::{BufReader, BufWriter, Write},
    path::PathBuf,
    rc::Rc,
};

use chrono::{DateTime, Utc};
use laser_pdf::{
    Metadata, Pdf, TextPiecesCache, Timestamp, XmpIdentifier,
    fonts::truetype::TruetypeFont,
    serde_elements::{ElementValue, SerdeElementElement},
};
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
    lang: String,
    producer: String,
    entries: Vec<Entry>,
}

fn main() {
    let input = serde_json::from_reader::<_, Input>(BufReader::new(std::io::stdin())).unwrap();

    let mut fonts: HashMap<PathBuf, Rc<TruetypeFont>> = HashMap::new();

    let mut pdf = Pdf::new();

    {
        pdf.set_metadata(Metadata {
            title: input.title,
            language: input.lang,
            keywords: input.keywords,
            producer: input.producer,
            creation_date: Timestamp(Utc::now()),
            identifier: XmpIdentifier::new(),
        });
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

    let text_pieces_cache = TextPiecesCache::new();

    for (entry, font_map) in input.entries.iter().zip(font_maps.iter()) {
        pdf.add_element_with_text_pieces_cache(
            entry.size,
            &text_pieces_cache,
            SerdeElementElement {
                element: &entry.element,
                fonts: font_map,
            },
        );
    }

    BufWriter::new(std::io::stdout())
        .write_all(&pdf.finish())
        .unwrap();
}
