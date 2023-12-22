use crate::*;

pub struct ElementStats {
    pub pages: u32,
    pub document: PdfDocument,
}

impl ElementStats {
    pub fn assert_pages(&self, pages: u32) -> &Self {
        assert_eq!(self.pages, pages);
        self
    }

    pub fn assert_linear(&self) -> &Self {
        todo!()
    }

    pub fn assert_empty(&self) -> &Self {
        todo!()
    }
}

pub fn run_element<E: Element>(element: E) -> ElementStats {
    todo!()
}
