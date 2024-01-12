use crate::*;

pub struct FranticJumper {
    pub jumps: Vec<u32>,
    pub size: ElementSize,
}

impl Element for FranticJumper {
    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        if let Some(b) = ctx.breakable {
            *b.break_count = self.jumps.iter().cloned().max().map(|m| m + 1).unwrap_or(0);
        }

        self.size
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        if let Some(b) = ctx.breakable {
            let mut previous: Vec<Option<Location>> = vec![
                None;
                self.jumps
                    .iter()
                    .cloned()
                    .max()
                    .map(|m| m as usize + 1)
                    .unwrap_or(0)
            ];

            for &jump in &self.jumps {
                let location = (b.get_location)(ctx.pdf, jump);

                if let Some(prev) = &previous[jump as usize] {
                    assert_eq!(prev.pos, location.pos);
                    assert_eq!(prev.layer.page, location.layer.page);
                    assert_eq!(prev.layer.layer, location.layer.layer);
                } else {
                    previous[jump as usize] = Some(location);
                }
            }
        }

        self.size
    }
}
