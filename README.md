# laser-pdf

A Rust library for programmatic PDF generation with precise, predictable layout control.

**laser-pdf** is designed for applications that need to reproduce layouts with pixel-perfect accuracy, providing fine-grained control over page breaking and element positioning. Built for [Escola](https://www.escola.ch) to generate complex PDFs with predictable behavior.

## Key Features

- **Predictable Layout System**: Inspired by Flutter's layout protocol for consistent, understandable behavior
- **Precise Page Breaking Control**: Elements like `Titled`, `RepeatAfterBreak`, and `PinBelow` for exact control over multi-page layouts  
- **Composable Element Architecture**: Simple trait-based system for building complex layouts from primitive components
- **Axis-Independent Collapse**: Elements can collapse separately on horizontal and vertical axes, with per-location vertical collapse
- **Multiple Font Systems**: Support for built-in PDF fonts and TrueType fonts with advanced text shaping
- **JSON Serialization**: Complete serde support for language-agnostic PDF generation
- **SVG and Image Support**: Render SVG graphics and raster images with precise positioning

## Design Philosophy

Unlike CSS, which can sometimes produce unexpected layouts, laser-pdf prioritizes **predictability**. Once you understand the relatively simple `Element` trait, you can easily predict how layouts will behave. Elements follow clear composition rules, and the collapse behavior is well-defined and consistent.

## Quick Start

(These instructions might need some work. Don't fully trust them yet.)

### Rust API

(A simple function convenient multi-page generation is still missing.)

```rust
use laser_pdf::*;
use laser_pdf::elements::{text::Text, column::Column, padding::Padding};
use laser_pdf::fonts::builtin::BuiltinFont;

// Create a PDF document
let mut pdf = Pdf::new();

// Add a page
let location = pdf.add_page((210., 297.)); // A4 size in mm

// Create fonts
let font = BuiltinFont::helvetica(&mut pdf);

// Build elements
let title = Text::basic("Document Title", &font, 16.0);
let body = Text::basic("This is the document body content.", &font, 12.0);

// Compose layout
let content = Column {
    gap: 10.0,
    collapse: true,
    content: |content| {
        content
          .add(&Padding::all(title, 5.0))?
          .add(&body)?;

        // Returns an option to support short-circuiting.
        None
    }
};

// Draw to PDF
let ctx = DrawCtx {
    pdf: &mut pdf,
    width: WidthConstraint { max: 200.0, expand: true },
    location,
    first_height: 280.0,
    preferred_height: None,
    breakable: None, // For single-page layout
};

content.draw(ctx);

// Save PDF
std::fs::write("output.pdf", pdf.finish())?;
```

### CLI Usage (JSON Interface)

```bash
echo '{
  "title": "My Document",
  "entries": [{
    "size": [210, 297],
    "fonts": {
      "regular": "./fonts/regular.ttf"
    },
    "element": {
      "type": "Column",
      "gap": 10,
      "content": [
        {
          "type": "Text",
          "text": "Hello, World!",
          "font": "regular",
          "size": 16
        }
      ]
    }
  }]
}' | cargo run > output.pdf
```

## Core Concepts

### Element Trait

All layout components implement the `Element` trait with three key methods:

- `first_location_usage()`: Determines if the element will use the first location on a page
- `measure()`: Calculates dimensions given width constraints  
- `draw()`: Renders the element to the PDF

### Page Breaking

Elements can intelligently break across pages using the breakable draw context. Advanced elements like `Titled` ensure titles stay with their content, while `RepeatAfterBreak` can repeat headers on each page.

### Collapse Behavior

Elements can collapse on each axis independently. When an element collapses in the direction of a container (like a `Column`), gaps around it are eliminated - similar to how `display: none` interacts with CSS Grid's `gap` property.

## Available Elements

### Layout Elements
- `Column` / `Row`: Vertical/horizontal stacking with gaps
- `Stack`: Layered positioning
- `Padding`: Add space around elements
- `HAlign`: Horizontal alignment control

### Content Elements  
- `Text`: Simple text rendering with built-in fonts
- `RichText`: Advanced text with TrueType fonts and text shaping
- `Image`: Raster image embedding
- `SVG`: Vector graphics rendering
- `Rectangle` / `Circle`: Basic shapes

### Page Flow Elements
- `Page`: Force page boundary
- `ForceBreak`: Insert page breaks
- `BreakWhole`: Keep element groups together
- `Titled`: Keep titles with content
- `RepeatAfterBreak`: Repeat elements after page breaks
- `PinBelow`: Pin elements to page bottom

### Advanced Elements
- `ShrinkToFit`: Scale content to fit constraints
- `ExpandToPreferredHeight`: Grow to fill available space
- `ChangingTitle`: Dynamic page titles

## Current Status

⚠️ **Pre-1.0**: The library is under active development. While used in production at Escola, the API may change before the 1.0 release. Element names and some APIs are still being refined.

**Roadmap to 1.0:**
- Finalize element naming conventions
- Publish to crates.io
- Release PHP bindings as open source
- Complete documentation

## Development

### Building
```bash
cargo build --verbose
```

### Testing
```bash
# Run all tests
cargo test --verbose

# Run specific test
cargo test test_name

# Update snapshots
cargo insta review
```

### Contributing

The project uses [insta](https://insta.rs/) for snapshot testing, including binary PDF snapshots for visual regression testing.

## License

Licensed under the [MIT License](LICENSE).

## Used By

- [Escola](https://www.escola.ch) - Production PDF generation for educational documents

---

*Built with ❤️ for precise, predictable PDF layouts*
