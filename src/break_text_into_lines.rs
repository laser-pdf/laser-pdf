pub struct BreakTextIntoLines<'a, F: Fn(&str) -> f64> {
    line_generator: LineGenerator<'a, F>,
    max_width: f64,
}

impl<'a, F: Fn(&str) -> f64> Iterator for BreakTextIntoLines<'a, F> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.line_generator.next(self.max_width, false) //.map(|t| t.trim_end())
    }
}

pub fn break_text_into_lines<'a, F: Fn(&str) -> f64>(
    text: &'a str,
    max_width: f64,
    text_width: F,
) -> BreakTextIntoLines<'a, F> {
    BreakTextIntoLines {
        line_generator: LineGenerator::new(text, text_width),
        max_width,
    }
}

pub struct LineGenerator<'a, F: Fn(&str) -> f64> {
    text: Option<&'a str>,
    text_width: F,
    soft_hyphen_width: f64,
}

impl<'a, F: Fn(&str) -> f64> LineGenerator<'a, F> {
    pub fn new(text: &'a str, text_width: F) -> Self {
        let soft_hyphen_width = text_width("\u{00ad}");

        LineGenerator {
            text: Some(text),
            text_width,
            soft_hyphen_width,
        }
    }

    pub fn next_unconstrained(&mut self) -> Option<&'a str> {
        if let Some(slice) = self.text {
            for (i, c) in slice.char_indices() {
                if c == '\n' {
                    self.text = Some(&slice[i + 1..]);
                    return Some(&slice[..i]);
                }
            }

            self.text = None;
            Some(slice)
        } else {
            None
        }
    }

    pub fn done(&self) -> bool {
        self.text.is_none()
    }

    pub fn next(&mut self, max_width: f64, incomplete: bool) -> Option<&'a str> {
        if let Some(slice) = self.text {
            let mut current_width = 0.0;
            let mut last_break = 0;
            let mut end_break = 0;
            let mut not_start = incomplete;

            let mut in_whitespace: Option<usize> = None;

            for (i, c) in slice.char_indices() {
                if c == '\n' {
                    if in_whitespace == None {
                        current_width += (self.text_width)(&slice[last_break..i]);
                    }

                    if current_width > max_width && not_start {
                        self.text = Some(&slice[end_break..]);
                        return Some(&slice[..last_break]);
                    } else {
                        self.text = Some(&slice[i + 1..]);
                        return Some(&slice[..i]);
                    }
                } else if c.is_whitespace() {
                    if in_whitespace == None {
                        current_width += (self.text_width)(&slice[last_break..i]);
                        in_whitespace = Some(i);
                    }
                } else if c == '\u{00ad}' && in_whitespace == None {
                    let end = i + c.len_utf8();

                    current_width += (self.text_width)(&slice[last_break..i]);

                    // While we don't add the soft hyphen to `current_width` we
                    // check here if the line would be too long with it such
                    // that if the code doesn't return here, but returns later
                    // we know that the line will produce will fit within the
                    // max width.
                    if not_start && current_width + self.soft_hyphen_width > max_width {
                        self.text = Some(&slice[end_break..]);
                        return Some(&slice[..last_break]);
                    }

                    last_break = end;
                    end_break = end;

                    in_whitespace = Some(end);
                } else if (c == '-' || c == '\u{2010}') && in_whitespace == None {
                    // \u{2010} is the Unicode hyphen

                    let end = i + c.len_utf8();

                    current_width += (self.text_width)(&slice[last_break..end]);

                    if not_start && current_width > max_width {
                        self.text = Some(&slice[end_break..]);
                        return Some(&slice[..last_break]);
                    }

                    last_break = end;
                    end_break = end;

                    in_whitespace = Some(end);
                } else {
                    if let Some(start_whitespace) = in_whitespace {
                        in_whitespace = None;

                        if current_width > max_width {
                            return Some(
                                &slice[..if !not_start {
                                    self.text = Some(&slice[i..]);
                                    start_whitespace
                                } else {
                                    self.text = Some(&slice[end_break..]);
                                    last_break
                                }],
                            );
                        }

                        not_start = true;
                        last_break = start_whitespace;
                        end_break = i;
                    }
                }
            }

            if in_whitespace == None {
                current_width += (self.text_width)(&slice[last_break..]);
            }

            if current_width > max_width && not_start {
                self.text = Some(&slice[end_break..]);
                Some(&slice[..last_break])
            } else {
                self.text = None;
                Some(slice)
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_flow() {
        let mut generator = LineGenerator::new(
            "Amet consequatur facilis necessitatibus sed quia numquam reiciendis. \
                Id impedit quo quaerat enim amet. ",
            |s| s.len() as f64,
        );

        assert_eq!(generator.next(16., false), Some("Amet consequatur"));
        assert_eq!(generator.next(16., false), Some("facilis"));
        assert_eq!(generator.next(16., false), Some("necessitatibus"));
        assert_eq!(generator.next(16., false), Some("sed quia numquam"));
        assert_eq!(generator.next(16., false), Some("reiciendis. Id"));
        assert_eq!(generator.next(16., false), Some("impedit quo"));
        assert_eq!(generator.next(16., false), Some("quaerat enim"));
        assert_eq!(generator.next(16., false), Some("amet. "));
        assert_eq!(generator.next(16., false), None);
    }

    #[test]
    fn test_trailing_whitespace() {
        let mut generator =
            LineGenerator::new("Id impedit quo quaerat enim amet.                  ", |s| {
                s.len() as f64
            });

        assert_eq!(generator.next(16., false), Some("Id impedit quo"));
        assert_eq!(generator.next(16., false), Some("quaerat enim"));

        // it's unclear whether any other behavior would be better here
        assert_eq!(generator.next(16., false), Some("amet.                  "));
        assert_eq!(generator.next(16., false), None);
    }

    #[test]
    fn test_pre_newline_whitespace() {
        let mut generator =
            LineGenerator::new("Id impedit quo \nquaerat enimmmmm    \namet.", |s| {
                s.len() as f64
            });

        assert_eq!(generator.next(16., false), Some("Id impedit quo "));
        assert_eq!(generator.next(16., false), Some("quaerat enimmmmm    "));
        assert_eq!(generator.next(16., false), Some("amet."));
        assert_eq!(generator.next(16., false), None);
    }

    #[test]
    fn test_newline() {
        let mut generator = LineGenerator::new("\n", |s| s.len() as f64);

        assert_eq!(generator.next(16., false), Some(""));
        assert_eq!(generator.next(16., false), Some(""));
        assert_eq!(generator.next(16., false), None);
    }

    #[test]
    fn test_empty_str() {
        let mut generator = LineGenerator::new("", |s| s.len() as f64);

        assert_eq!(generator.next(16., false), Some(""));
        assert_eq!(generator.next(16., false), None);
    }

    #[test]
    fn test_space() {
        let mut generator = LineGenerator::new("  ", |s| s.len() as f64);

        assert_eq!(generator.next(16., false), Some("  "));
        assert_eq!(generator.next(16., false), None);
    }

    #[test]
    fn test_word_longer_than_line() {
        let mut generator = LineGenerator::new("Averylongword", |s| s.len() as f64);

        assert_eq!(generator.next(8., false), Some("Averylongword"));
        assert_eq!(generator.next(8., false), None);

        let mut generator = LineGenerator::new("Averylongword test.", |s| s.len() as f64);

        assert_eq!(generator.next(8., false), Some("Averylongword"));
        assert_eq!(generator.next(8., false), Some("test."));
        assert_eq!(generator.next(8., false), None);

        let mut generator = LineGenerator::new("A verylongword test.", |s| s.len() as f64);

        assert_eq!(generator.next(8., false), Some("A"));
        assert_eq!(generator.next(8., false), Some("verylongword"));
        assert_eq!(generator.next(8., false), Some("test."));
        assert_eq!(generator.next(8., false), None);
    }

    fn len_without_soft_hyphens(s: &str) -> f64 {
        use itertools::{Itertools, Position};

        s.chars()
            .with_position()
            .filter(|&(p, c)| c != '\u{00ad}' || matches!(p, Position::Last | Position::Only))
            .count() as f64
    }

    #[test]
    fn test_soft_hyphens() {
        let mut generator = LineGenerator::new(
            "A\u{00ad}very\u{00ad}long\u{00ad}word",
            len_without_soft_hyphens,
        );

        assert_eq!(generator.next(7., false), Some("A\u{00ad}very\u{00ad}"));
        assert_eq!(generator.next(7., false), Some("long\u{00ad}"));
        assert_eq!(generator.next(7., false), Some("word"));
        assert_eq!(generator.next(7., false), None);

        let mut generator = LineGenerator::new(
            "A\u{00ad}very \u{00ad}long\u{00ad}word",
            len_without_soft_hyphens,
        );

        assert_eq!(generator.next(7., false), Some("A\u{00ad}very"));
        assert_eq!(generator.next(7., false), Some("\u{00ad}long\u{00ad}"));
        assert_eq!(generator.next(7., false), Some("word"));
        assert_eq!(generator.next(7., false), None);

        let mut generator = LineGenerator::new(
            "A\u{00ad}very\u{00ad}\u{00ad}long\u{00ad}word",
            len_without_soft_hyphens,
        );

        assert_eq!(generator.next(7., false), Some("A\u{00ad}very\u{00ad}"));
        assert_eq!(generator.next(7., false), Some("\u{00ad}long\u{00ad}"));
        assert_eq!(generator.next(7., false), Some("word"));
        assert_eq!(generator.next(7., false), None);
    }

    #[test]
    fn test_soft_hyphen_length() {
        let mut generator =
            LineGenerator::new("A\u{00ad}very long\u{00ad}word", len_without_soft_hyphens);

        assert_eq!(generator.next(5., false), Some("A\u{00ad}very"));
        assert_eq!(generator.next(5., false), Some("long\u{00ad}"));
        assert_eq!(generator.next(5., false), Some("word"));
        assert_eq!(generator.next(5., false), None);

        let mut generator = LineGenerator::new(
            "A\u{00ad}very\u{00ad}long\u{00ad}word",
            len_without_soft_hyphens,
        );

        assert_eq!(generator.next(5., false), Some("A\u{00ad}"));
        assert_eq!(generator.next(5., false), Some("very\u{00ad}"));
        assert_eq!(generator.next(5., false), Some("long\u{00ad}"));
        assert_eq!(generator.next(5., false), Some("word"));
        assert_eq!(generator.next(5., false), None);
    }

    #[test]
    fn test_hard_hyphens() {
        let mut generator = LineGenerator::new("A-very-long-word", len_without_soft_hyphens);

        assert_eq!(generator.next(7., false), Some("A-very-"));
        assert_eq!(generator.next(7., false), Some("long-"));
        assert_eq!(generator.next(7., false), Some("word"));
        assert_eq!(generator.next(7., false), None);

        let mut generator = LineGenerator::new("A-very -long-word", len_without_soft_hyphens);

        assert_eq!(generator.next(7., false), Some("A-very"));
        assert_eq!(generator.next(7., false), Some("-long-"));
        assert_eq!(generator.next(7., false), Some("word"));
        assert_eq!(generator.next(7., false), None);

        let mut generator = LineGenerator::new("A‐very--long-word", len_without_soft_hyphens);

        assert_eq!(generator.next(7., false), Some("A‐very-"));
        assert_eq!(generator.next(7., false), Some("-long-"));
        assert_eq!(generator.next(7., false), Some("word"));
        assert_eq!(generator.next(7., false), None);
    }

    #[test]
    fn test_hard_hyphen_length() {
        let mut generator = LineGenerator::new("A\u{2010}very long-word", len_without_soft_hyphens);

        assert_eq!(generator.next(5., false), Some("A‐"));
        assert_eq!(generator.next(5., false), Some("very"));
        assert_eq!(generator.next(5., false), Some("long-"));
        assert_eq!(generator.next(5., false), Some("word"));
        assert_eq!(generator.next(5., false), None);

        let mut generator = LineGenerator::new("A-very-long-word", len_without_soft_hyphens);

        assert_eq!(generator.next(5., false), Some("A-"));
        assert_eq!(generator.next(5., false), Some("very-"));
        assert_eq!(generator.next(5., false), Some("long-"));
        assert_eq!(generator.next(5., false), Some("word"));
        assert_eq!(generator.next(5., false), None);
    }
}
