use printpdf::{PdfDocumentReference, PdfLayerReference};

pub struct Pdf {
    pub document: PdfDocumentReference,
    pub page_size: [f64; 2],
}

/// A position for an element to render at.
/// This doesn't include the width at the moment, as this would make things much more complicated.
/// The line breaking iterator wouldn't work in its current form for example.
/// Things are much easier if an element can make width related calculations in the beginning an
/// doesn't have to recalculate them on a page break.
#[derive(Clone, Debug)]
pub struct DrawPos {
    pub layer: PdfLayerReference,
    pub pos: [f64; 2],
    pub height_available: f64,
}

/// The position is in millimeters and in the pdf coordinate system (meaning the origin is on the
/// bottom left corner).
pub struct DrawContext<'a, 'b> {
    // pub pos: [f64; 2],
    // pub height_available: f64,
    pub pdf: &'a mut Pdf,

    pub draw_pos: DrawPos,

    /// The full height of the current drawing rectangle, usually this is the page height minus some
    /// amount of border. It is also the height you're expected to get after a break unless there's
    /// a special [Element] around it like titled.
    pub full_height: f64,

    // /// The height_available after a call to `next_draw_pos`
    // /// This is fine for the moment. It might change in the future to enable variations in page or
    // /// column height. Meaning a call to `next_draw_pos` will need to be able to change this or
    // /// return a new one.
    // pub next_draw_pos_height: f64,

    /// This returns a new [DrawPos] because some collection elements need to keep multiple
    /// [DrawPos]s at once (e.g. for page breaking inside of a horizontal list)
    ///
    /// The second parameter is which drawing rectangle the break is occurring from. This number
    /// must be counted up for sequential page breaks. This allows the same page break to be
    /// performed twice in a row. A new `draw_rect_id` will be returned from the call to
    /// `next_draw_pos`, so if you store the current draw pos, you can just pass the one from there.
    ///
    /// The third parameter is the size of the [Element] on the specified draw rect. If the same
    /// page break is performed multiple times, the largest value on each axis should be used by the
    /// container.
    ///
    /// Note: For correctness we might have to change the size to an option, because
    /// right now `titled` assumes that if the height is zero that means nothing was drawn which
    /// might not be correct for a zero height line or something. Same goes for `widget_or_break`.
    /// Or maybe we can say that if width is also zero then there's nothing, but I'm also not sure
    /// that's correct. We could also just say that zero height must mean there's nothing.
    pub next_draw_pos: Option<&'b mut dyn FnMut(&mut Pdf, u32, [f64; 2]) -> DrawPos>,
}

pub trait Element {
    /// A [None] on width means the element should take the width it needs.
    fn element(&self, width: Option<f64>, draw: Option<DrawContext>) -> [f64; 2];
}

impl<F: Fn(Option<f64>, Option<DrawContext>) -> [f64; 2]> Element for F {
    #[inline]
    fn element(&self, width: Option<f64>, draw: Option<DrawContext>) -> [f64; 2] {
        self(width, draw)
    }
}
