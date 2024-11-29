use crate::*;

pub struct FranticJumper {
    pub jumps: Vec<(u32, Option<f32>)>,
    pub size: ElementSize,
}

impl Element for FranticJumper {
    fn measure(&self, ctx: MeasureCtx) -> ElementSize {
        if let Some(b) = ctx.breakable {
            *b.break_count = self
                .jumps
                .iter()
                .map(|j| j.0)
                .max()
                .map(|m| m + 1)
                .unwrap_or(0);
        }

        self.size
    }

    fn draw(&self, ctx: DrawCtx) -> ElementSize {
        if let Some(b) = ctx.breakable {
            let mut previous: Vec<Option<Location>> = vec![
                None;
                self.jumps
                    .iter()
                    .map(|j| j.0)
                    .max()
                    .map(|m| m as usize + 1)
                    .unwrap_or(0)
            ];

            for &jump in &self.jumps {
                let location = (b.do_break)(ctx.pdf, jump.0, jump.1);

                if let Some(prev) = &previous[jump.0 as usize] {
                    assert_eq!(prev.pos, location.pos);
                    assert_eq!(prev.page_idx, location.page_idx);
                    assert_eq!(prev.layer_idx, location.layer_idx);
                } else {
                    previous[jump.0 as usize] = Some(location);
                }
            }
        }

        self.size
    }
}
