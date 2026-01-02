# laser-pdf

laser-pdf is a PDF generation library written in Rust.

## Example

```rust
use laser_pdf::elements::{column::Column, text::Text};
use laser_pdf::fonts::builtin::BuiltinFont;
use laser_pdf::*;

fn main() -> std::io::Result<()> {
    // Create a PDF document
    let mut pdf = Pdf::new();

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
            content.add(&title)?.add(&body)?;

            // Returns an option to support short-circuiting.
            None
        },
    };

    // Draw to PDF on an A4 sized page
    pdf.add_element((210., 297.), content);

    // Save PDF
    std::fs::write("output.pdf", pdf.finish())?;

    Ok(())
}
```

## Goals

**Precision:** The main goal is to be able to precisely replicate complicated document designs
with straightforward, understandable code. This includes page breaking, which can be a pain in other
systems like when generating PDFs from HTML.

**Understandability:** All the interactions between containers and their children go through a
relatively simple element trait. The goal is for layout behavior to be as understandable as
possible, avoiding a process of trial and error.

**Performance:** It should always be trivially possible to generate even a large document as
a response to a HTTP request without a slow response time. At Escola we use a 50 page document
generated from real layout code as a benchmark. This can be generated in about 60ms on an 8th
generation i7.

## Non-Goals

laser-pdf does not aim to be a general purpose typesetting system. It currently doesn't even
implement justified text alignment.

## Design

Layouts are built by composing elements. Elements interact through the
[`Element`](https://docs.rs/laser-pdf/latest/laser_pdf/trait.Element.html) trait. Only three
operations are possible on an element:

**`measure`:** This returns the size an element would use with the given constraint. This includes the
number of breaks the element would perform (if in a breakable context). 

**`first_location_usage`:** This is a more specialized version of measure that is used for example
to determine whether a title that belongs to the element should be pulled to the next page because
none of the element fits on the first page. The
[`Titled`](https://docs.rs/laser-pdf/latest/laser_pdf/elements/titled/struct.Titled.html) element
is used for this, preventing stranded titles.

**`draw`:** This draws the element to the PDF, starting at the provided page and position. An element
does not need to have been measured before being drawn. In fact a lot of care is taken to avoid
unnecessarily measuring elements before drawing them.

Elements are fundamentally stateless, meaning that measuring and then drawing an element will mean
some redundant computation. This enables a simpler design, but means some possible inefficiencies
with deeply nested layout. A lot of care is taken to minimize the possible performance downsides
that come with this design:

Most containers do not need to measure their children before drawing them. The main exception is
the `Row` element, which acts like a horizontal flexbox. It needs to measure self-sized children
because those can determine their own width and that can influence how other elements before it are
layed out. Other children are not measured before drawing unless the `expand` option is used, which
for example makes it possible to align elements at the bottom of the whole row.

The measure operation on most elements avoids doing any memory allocation at all.  The `Text`
element is the exception to this. It allocates memory during shaping and unicode segmentation. A
cache is used to ensure that shaping and segmentation only happens once for each piece of text
(including the string, font, size, color, etc.). This means that a text element will only allocate
the first time it is measured and the cached result of shaping and segmentation can then also be
used for a subsequent draw.

The children of container elements like `Column` and `Row` are provided as closures. These closures
can be called multiple times if the element is measured before it is drawn. It is up to the user of
the library to make sure that expensive computations happen outside of these closures.
