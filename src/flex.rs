pub struct MeasureLayout {
    width: f64,
    gap: f64,
    total_flex: u8,
    no_expand_count: u8,
    no_expand_width: f64,
}

impl MeasureLayout {
    pub fn new(width: f64, gap: f64) -> Self {
        MeasureLayout {
            width,
            gap,
            total_flex: 0,
            no_expand_count: 0,
            no_expand_width: 0.,
        }
    }

    pub fn add_fixed(&mut self, width: f64) {
        self.no_expand_count += 1;
        self.no_expand_width += width;
    }

    pub fn add_expand(&mut self, fraction: u8) {
        // self.count += 1;
        self.total_flex += fraction;
    }

    pub fn no_expand_width(&self) -> Option<f64> {
        if self.no_expand_count == 0 {
            None
        } else {
            Some(self.no_expand_width + self.gap * (self.no_expand_count - 1) as f64)
        }
    }

    pub fn build(self) -> DrawLayout {
        // The goal here is to divide the space in a way that, even with a gap, we divide the space
        // such that for expample if you have a flex with 1fr + 1fr above a flex with 2fr + 1fr +
        // 1fr the size of the first cell in each will actually be the same.
        //
        // Imagine each element has half a gap on either side. We can then divide the space
        // including this and then subtract one gap for each element. Of course the input width
        // needs to get one gap added to it at the beginning, otherwise the gaps wouldn't add up.
        //
        // For non-expanded elements we first subtract all of the non-expanded elements plus their
        // gaps and then we do the math normally.

        let remaining_width =
            (self.width + self.gap - self.no_expand_width - self.gap * self.no_expand_count as f64)
                .max(0.);

        DrawLayout {
            total_flex: self.total_flex,
            gap: self.gap,
            remaining_width,
        }
    }
}

#[derive(Copy, Clone)]
pub struct DrawLayout {
    total_flex: u8,
    gap: f64,
    remaining_width: f64,
}

impl DrawLayout {
    pub fn expand_width(&self, fraction: u8) -> f64 {
        (self.remaining_width * fraction as f64 / self.total_flex as f64 - self.gap).max(0.)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alignment() {
        let mut layout = MeasureLayout::new(15., 2.);
        layout.add_expand(1);
        layout.add_fixed(3.);
        layout.add_expand(1);

        let a = layout.build();

        let mut layout = MeasureLayout::new(15., 2.);
        layout.add_expand(1);
        layout.add_expand(1);
        layout.add_fixed(3.);
        layout.add_expand(2);

        let b = layout.build();

        // the fixed element in the middle should stay in the same position in both cases
        assert_eq!(
            a.expand_width(1),
            b.expand_width(1) + 2. + b.expand_width(1),
        );
        assert_eq!(a.expand_width(1), 4.,);

        // while we're at it we can also test the total width
        assert_eq!(a.expand_width(1) + 2. + 3. + 2. + a.expand_width(1), 15.);
        assert_eq!(
            b.expand_width(1) + 2. + b.expand_width(1) + 2. + 3. + 2. + b.expand_width(2),
            15.
        );
    }

    #[test]
    fn test_total_width() {
        {
            let mut layout = MeasureLayout::new(100., 4.);
            layout.add_expand(1);
            layout.add_expand(1);
            layout.add_expand(1);

            let draw_layout = layout.build();

            assert_eq!(
                draw_layout.expand_width(1)
                    + draw_layout.expand_width(1)
                    + draw_layout.expand_width(1)
                    + 2. * 4.,
                100.,
            );
        }

        {
            let mut layout = MeasureLayout::new(100., 4.);
            layout.add_expand(1);
            layout.add_fixed(50.);
            layout.add_expand(1);

            let draw_layout = layout.build();

            assert_eq!(
                draw_layout.expand_width(1) + 50. + draw_layout.expand_width(1) + 2. * 4.,
                100.,
            );

            assert_eq!(draw_layout.expand_width(1), 21.);
        }

        {
            let mut layout = MeasureLayout::new(100., 4.);
            layout.add_expand(1);
            layout.add_fixed(25.);
            layout.add_expand(1);
            layout.add_fixed(25.);

            let draw_layout = layout.build();

            assert_eq!(
                draw_layout.expand_width(1) + 25. + draw_layout.expand_width(1) + 25. + 3. * 4.,
                100.,
            );

            assert_eq!(draw_layout.expand_width(1), 19.);
        }

        {
            let mut layout = MeasureLayout::new(100., 4.);
            layout.add_fixed(25.);
            layout.add_expand(1);
            layout.add_fixed(25.);
            layout.add_expand(1);
            layout.add_fixed(25.);

            let draw_layout = layout.build();

            assert_eq!(
                25. + draw_layout.expand_width(1)
                    + 25.
                    + draw_layout.expand_width(1)
                    + 25.
                    + 4. * 4.,
                100.,
            );
        }

        {
            let mut layout = MeasureLayout::new(100., 3.);
            layout.add_fixed(25.);
            layout.add_expand(2);
            layout.add_fixed(25.);
            layout.add_expand(1);
            layout.add_fixed(25.);

            let draw_layout = layout.build();

            assert_eq!(
                25. + draw_layout.expand_width(2)
                    + 25.
                    + draw_layout.expand_width(1)
                    + 25.
                    + 3. * 4.,
                100.,
            );
        }

        {
            let mut layout = MeasureLayout::new(100., 4.);
            layout.add_fixed(25.);
            layout.add_expand(2);
            layout.add_fixed(25.);
            layout.add_expand(1);
            layout.add_fixed(25.);

            let draw_layout = layout.build();

            assert_eq!(
                (25. + draw_layout.expand_width(2)
                    + 25.
                    + draw_layout.expand_width(1)
                    + 25.
                    + 4. * 4.),
                100.,
            );
        }

        {
            let mut layout = MeasureLayout::new(22., 2.);
            layout.add_expand(2);
            layout.add_fixed(14.);
            layout.add_expand(1);

            let draw_layout = layout.build();

            assert_eq!(
                draw_layout.expand_width(2) + 14. + draw_layout.expand_width(1) + 2. * 2.,
                22.,
            );
        }
    }
}
