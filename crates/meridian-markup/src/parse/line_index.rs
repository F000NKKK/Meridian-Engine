//! [`LineIndex`]: line starts of one source, for computing [`Span`]s.

use super::span::Span;

/// The byte offset of every line start in one source, built in one pass
/// so a [`Span`] can be computed for any offset without rescanning.
pub(crate) struct LineIndex<'a> {
    source: &'a str,
    line_starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
    pub(crate) fn new(source: &'a str) -> Self {
        let line_starts = std::iter::once(0)
            .chain(source.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        Self {
            source,
            line_starts,
        }
    }

    /// The span of byte `offset`; an offset past the end clamps to it.
    pub(crate) fn span(&self, offset: usize) -> Span {
        let offset = offset.min(self.source.len());
        let line = self.line_starts.partition_point(|&start| start <= offset) - 1;
        let line_start = self.line_starts[line];
        let column = self
            .source
            .get(line_start..offset)
            .map_or(offset - line_start, |text| text.chars().count());
        Span {
            offset,
            line: to_u32(line + 1),
            column: to_u32(column + 1),
        }
    }
}

fn to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_byte_is_line_one_column_one() {
        let index = LineIndex::new("<A />");
        assert_eq!(
            index.span(0),
            Span {
                offset: 0,
                line: 1,
                column: 1
            }
        );
    }

    #[test]
    fn offsets_after_newlines_count_lines_and_chars() {
        let index = LineIndex::new("ab\nцd\n");
        let d = "ab\nц".len();
        assert_eq!(
            index.span(d),
            Span {
                offset: d,
                line: 2,
                column: 2
            }
        );
        assert_eq!(index.span(3).line, 2);
        assert_eq!(index.span(3).column, 1);
    }

    #[test]
    fn offset_past_the_end_clamps() {
        let index = LineIndex::new("ab");
        assert_eq!(index.span(99).offset, 2);
    }
}
