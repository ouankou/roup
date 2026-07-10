//! Source locations with checked UTF-8 byte boundaries.
//!
//! Parser-facing offsets are byte offsets because Rust strings and diagnostic
//! slices are byte indexed. Human-facing lines and columns are one based;
//! columns count Unicode scalar values rather than bytes. Newlines are delimited
//! by `\n`, so CRLF input has the same line numbering as LF input.

use std::error::Error;
use std::fmt;
use std::ops::Range;

/// A checked position in a UTF-8 source string.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourcePosition {
    source_id: SourceId,
    byte_offset: usize,
    line: usize,
    column: usize,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct SourceLocation {
    byte_offset: usize,
    line: usize,
    column: usize,
}

/// Content identity carried by every position and span.
///
/// Offsets plus line/column values are insufficient to distinguish two
/// different sources with the same shape. Two independent byte hashes and the
/// byte length make accidental cross-source span reuse detectable without
/// retaining or leaking the source text.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct SourceId {
    byte_len: usize,
    hash_a: u64,
    hash_b: u64,
}

impl SourcePosition {
    /// Computes a source position at `byte_offset`.
    ///
    /// The offset may point one byte past the source (the end-of-input
    /// position), but it must not exceed the source length or split a UTF-8
    /// code point.
    pub fn new(source: &str, byte_offset: usize) -> Result<Self, SourceError> {
        validate_offset(source, byte_offset)?;

        let prefix = &source[..byte_offset];
        let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let column = prefix
            .rsplit_once('\n')
            .map_or(prefix, |(_, current_line)| current_line)
            .chars()
            .count()
            + 1;

        Ok(Self {
            source_id: source_id(source),
            byte_offset,
            line,
            column,
        })
    }

    /// Returns the zero-based UTF-8 byte offset.
    #[must_use]
    pub const fn byte_offset(self) -> usize {
        self.byte_offset
    }

    /// Returns the one-based source line.
    #[must_use]
    pub const fn line(self) -> usize {
        self.line
    }

    /// Returns the one-based Unicode-scalar column.
    #[must_use]
    pub const fn column(self) -> usize {
        self.column
    }
}

impl fmt::Display for SourcePosition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.line, self.column)
    }
}

/// A checked half-open byte range in one source string.
///
/// `start` is inclusive and `end` is exclusive. Both endpoints are valid UTF-8
/// boundaries and retain their human-facing line and column. A `Span` must be
/// used with the source from which it was constructed; [`Span::slice`] checks
/// that its endpoint positions still agree with the supplied source.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Span {
    source_id: SourceId,
    start: SourceLocation,
    end: SourceLocation,
}

impl Span {
    /// Constructs a span covering the complete source string.
    ///
    /// Both endpoints are constructive UTF-8 invariants: zero and
    /// `source.len()` are always character boundaries. This infallible helper is
    /// intended for diagnostics emitted by parsers that cannot yet identify a
    /// narrower token range.
    #[must_use]
    pub fn entire(source: &str) -> Self {
        let source_id = source_id(source);
        let end = position_at_end(source);
        Self {
            source_id,
            start: SourceLocation {
                byte_offset: 0,
                line: 1,
                column: 1,
            },
            end: SourceLocation {
                byte_offset: end.byte_offset,
                line: end.line,
                column: end.column,
            },
        }
    }

    /// Constructs an empty span at the beginning of `source`.
    #[must_use]
    pub fn start_of(source: &str) -> Self {
        let source_id = source_id(source);
        let start = SourceLocation {
            byte_offset: 0,
            line: 1,
            column: 1,
        };
        Self {
            source_id,
            start,
            end: start,
        }
    }

    /// Constructs a checked span for `start..end`.
    pub fn new(source: &str, start: usize, end: usize) -> Result<Self, SourceError> {
        if start > end {
            return Err(SourceError::ReversedRange { start, end });
        }

        Self::from_positions(
            SourcePosition::new(source, start)?,
            SourcePosition::new(source, end)?,
        )
    }

    /// Constructs an empty span at `byte_offset`.
    pub fn point(source: &str, byte_offset: usize) -> Result<Self, SourceError> {
        let position = SourcePosition::new(source, byte_offset)?;
        let location = SourceLocation {
            byte_offset: position.byte_offset,
            line: position.line,
            column: position.column,
        };
        Ok(Self {
            source_id: position.source_id,
            start: location,
            end: location,
        })
    }

    /// Constructs a span from positions already checked against a source.
    ///
    /// This avoids rescanning source text when a lexer already carries checked
    /// positions. Positions from different source strings must not be combined;
    /// [`Span::validate_for`] detects such a mismatch before slicing.
    pub fn from_positions(start: SourcePosition, end: SourcePosition) -> Result<Self, SourceError> {
        if start.source_id != end.source_id {
            return Err(SourceError::DifferentSources);
        }
        if start.byte_offset > end.byte_offset {
            return Err(SourceError::ReversedRange {
                start: start.byte_offset,
                end: end.byte_offset,
            });
        }

        let logical_order_is_valid = if start.byte_offset == end.byte_offset {
            start.line == end.line && start.column == end.column
        } else {
            start.line < end.line || (start.line == end.line && start.column <= end.column)
        };
        if !logical_order_is_valid {
            return Err(SourceError::InconsistentPositions {
                start: Box::new(start),
                end: Box::new(end),
            });
        }

        Ok(Self {
            source_id: start.source_id,
            start: SourceLocation {
                byte_offset: start.byte_offset,
                line: start.line,
                column: start.column,
            },
            end: SourceLocation {
                byte_offset: end.byte_offset,
                line: end.line,
                column: end.column,
            },
        })
    }

    /// Returns the inclusive start position.
    #[must_use]
    pub const fn start(self) -> SourcePosition {
        SourcePosition {
            source_id: self.source_id,
            byte_offset: self.start.byte_offset,
            line: self.start.line,
            column: self.start.column,
        }
    }

    /// Returns the exclusive end position.
    #[must_use]
    pub const fn end(self) -> SourcePosition {
        SourcePosition {
            source_id: self.source_id,
            byte_offset: self.end.byte_offset,
            line: self.end.line,
            column: self.end.column,
        }
    }

    /// Returns the inclusive start byte offset.
    #[must_use]
    pub const fn start_byte(self) -> usize {
        self.start.byte_offset
    }

    /// Returns the exclusive end byte offset.
    #[must_use]
    pub const fn end_byte(self) -> usize {
        self.end.byte_offset
    }

    /// Returns the half-open UTF-8 byte range.
    #[must_use]
    pub fn byte_range(self) -> Range<usize> {
        self.start.byte_offset..self.end.byte_offset
    }

    /// Reports whether this span contains no source bytes.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start.byte_offset == self.end.byte_offset
    }

    /// Reports whether `byte_offset` lies in the half-open span.
    #[must_use]
    pub const fn contains(self, byte_offset: usize) -> bool {
        self.start.byte_offset <= byte_offset && byte_offset < self.end.byte_offset
    }

    /// Reports whether `byte_offset` lies in the closed endpoint range.
    ///
    /// This is useful for cursor and insertion diagnostics, where an offset at
    /// the exclusive end still belongs to the diagnostic location.
    #[must_use]
    pub const fn contains_endpoint(self, byte_offset: usize) -> bool {
        self.start.byte_offset <= byte_offset && byte_offset <= self.end.byte_offset
    }

    /// Checks that this span's offsets and positions describe `source`.
    pub fn validate_for(self, source: &str) -> Result<(), SourceError> {
        let actual_source_id = source_id(source);
        if self.source_id != actual_source_id {
            return Err(SourceError::DifferentSources);
        }
        let actual_start = SourcePosition::new(source, self.start.byte_offset)?;
        let expected_start = self.start();
        if actual_start != expected_start {
            return Err(SourceError::PositionDoesNotMatchSource {
                expected: Box::new(expected_start),
                actual: Box::new(actual_start),
            });
        }

        let actual_end = SourcePosition::new(source, self.end.byte_offset)?;
        let expected_end = self.end();
        if actual_end != expected_end {
            return Err(SourceError::PositionDoesNotMatchSource {
                expected: Box::new(expected_end),
                actual: Box::new(actual_end),
            });
        }

        Ok(())
    }

    /// Returns the exact source slice covered by this span.
    pub fn slice(self, source: &str) -> Result<&str, SourceError> {
        self.validate_for(source)?;
        Ok(&source[self.byte_range()])
    }

    /// Returns the smallest checked span covering `self` and `other`.
    ///
    /// Both spans are validated against `source`, preventing accidental joins
    /// between different source buffers.
    pub fn cover(self, source: &str, other: Self) -> Result<Self, SourceError> {
        self.validate_for(source)?;
        other.validate_for(source)?;
        Self::new(
            source,
            self.start_byte().min(other.start_byte()),
            self.end_byte().max(other.end_byte()),
        )
    }
}

impl fmt::Display for Span {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            write!(formatter, "{}", self.start())
        } else {
            write!(formatter, "{}..{}", self.start(), self.end())
        }
    }
}

/// Failure to construct or use a checked source location.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceError {
    /// Positions or spans were combined with a different source buffer.
    DifferentSources,
    /// A byte offset exceeds the source length.
    ByteOffsetOutOfBounds { offset: usize, source_len: usize },
    /// A byte offset splits a UTF-8 code point.
    NotCharBoundary { offset: usize },
    /// A range starts after it ends.
    ReversedRange { start: usize, end: usize },
    /// Two positions have compatible byte order but contradictory line/column
    /// order, normally because they came from different sources.
    InconsistentPositions {
        start: Box<SourcePosition>,
        end: Box<SourcePosition>,
    },
    /// A stored position does not describe the source supplied by the caller.
    PositionDoesNotMatchSource {
        expected: Box<SourcePosition>,
        actual: Box<SourcePosition>,
    },
}

impl fmt::Display for SourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DifferentSources => {
                formatter.write_str("source position belongs to a different source")
            }
            Self::ByteOffsetOutOfBounds { offset, source_len } => write!(
                formatter,
                "byte offset {offset} exceeds source length {source_len}"
            ),
            Self::NotCharBoundary { offset } => {
                write!(formatter, "byte offset {offset} splits a UTF-8 code point")
            }
            Self::ReversedRange { start, end } => {
                write!(
                    formatter,
                    "source range starts at {start} after ending at {end}"
                )
            }
            Self::InconsistentPositions { start, end } => write!(
                formatter,
                "source positions {start} and {end} have inconsistent byte and line order"
            ),
            Self::PositionDoesNotMatchSource { expected, actual } => write!(
                formatter,
                "stored source position {expected} does not match actual position {actual}"
            ),
        }
    }
}

impl Error for SourceError {}

fn validate_offset(source: &str, offset: usize) -> Result<(), SourceError> {
    if offset > source.len() {
        return Err(SourceError::ByteOffsetOutOfBounds {
            offset,
            source_len: source.len(),
        });
    }
    if !source.is_char_boundary(offset) {
        return Err(SourceError::NotCharBoundary { offset });
    }
    Ok(())
}

fn position_at_end(source: &str) -> SourcePosition {
    let line = source.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = source
        .rsplit_once('\n')
        .map_or(source, |(_, current_line)| current_line)
        .chars()
        .count()
        + 1;
    SourcePosition {
        source_id: source_id(source),
        byte_offset: source.len(),
        line,
        column,
    }
}

fn source_id(source: &str) -> SourceId {
    // FNV-1a and djb2 use unrelated recurrences. This is not a security
    // boundary; the pair is an exactness guard against accidental span/source
    // mixups while keeping Span small and Copy.
    let mut hash_a = 0xcbf2_9ce4_8422_2325_u64;
    let mut hash_b = 5381_u64;
    for byte in source.bytes() {
        hash_a ^= u64::from(byte);
        hash_a = hash_a.wrapping_mul(0x0000_0100_0000_01b3);
        hash_b = hash_b
            .wrapping_mul(33)
            .wrapping_add(u64::from(byte).wrapping_add(1));
    }
    SourceId {
        byte_len: source.len(),
        hash_a,
        hash_b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_ascii_lines_and_columns() {
        let source = "alpha\nbeta\ngamma";
        let span = Span::new(source, 6, 10).expect("valid beta span");

        assert_eq!(span.start().byte_offset(), 6);
        assert_eq!(span.start().line(), 2);
        assert_eq!(span.start().column(), 1);
        assert_eq!(span.end().line(), 2);
        assert_eq!(span.end().column(), 5);
        assert_eq!(span.slice(source), Ok("beta"));
        assert_eq!(span.to_string(), "2:1..2:5");
    }

    #[test]
    fn infallible_constructor_covers_complete_unicode_source() {
        let source = "alpha\nλ🦀";
        let entire = Span::entire(source);
        let start = Span::start_of(source);

        assert_eq!(entire.byte_range(), 0..source.len());
        assert_eq!((entire.end().line(), entire.end().column()), (2, 3));
        assert_eq!(entire.slice(source), Ok(source));
        assert_eq!(start.byte_range(), 0..0);
        assert_eq!(start.slice(source), Ok(""));
    }

    #[test]
    fn unicode_columns_count_scalars_and_offsets_remain_bytes() {
        let source = "aλ🦀\nβ";
        let crab_end = "aλ🦀".len();
        let crab = Span::new(source, "aλ".len(), crab_end).expect("valid crab span");

        assert_eq!(crab.start().column(), 3);
        assert_eq!(crab.end().column(), 4);
        assert_eq!(crab.slice(source), Ok("🦀"));

        let beta = SourcePosition::new(source, crab_end + 1).expect("valid beta position");
        assert_eq!((beta.line(), beta.column()), (2, 1));
    }

    #[test]
    fn rejects_offsets_inside_unicode_code_points() {
        let source = "λ";
        assert_eq!(
            SourcePosition::new(source, 1),
            Err(SourceError::NotCharBoundary { offset: 1 })
        );
        assert_eq!(
            Span::new(source, 0, 3),
            Err(SourceError::ByteOffsetOutOfBounds {
                offset: 3,
                source_len: 2,
            })
        );
    }

    #[test]
    fn rejects_reversed_ranges_before_resolving_positions() {
        assert_eq!(
            Span::new("abc", 3, 1),
            Err(SourceError::ReversedRange { start: 3, end: 1 })
        );
    }

    #[test]
    fn point_spans_are_valid_at_end_of_input() {
        let span = Span::point("abc", 3).expect("valid end point");
        assert!(span.is_empty());
        assert!(!span.contains(3));
        assert!(span.contains_endpoint(3));
        assert_eq!(span.slice("abc"), Ok(""));
        assert_eq!(span.to_string(), "1:4");
    }

    #[test]
    fn rejects_a_different_source_even_when_offsets_are_valid() {
        let span = Span::new("a\nb", 2, 3).expect("valid span");
        let error = span
            .slice("abc")
            .expect_err("different line layout must be rejected");

        assert_eq!(error, SourceError::DifferentSources);
    }

    #[test]
    fn rejects_different_text_with_the_same_source_shape() {
        let span = Span::new("abc", 1, 2).expect("valid span");
        assert_eq!(span.slice("xyz"), Err(SourceError::DifferentSources));
    }

    #[test]
    fn rejects_positions_combined_from_distinct_sources() {
        let start = SourcePosition::new("abc", 0).expect("valid start");
        let end = SourcePosition::new("xyz", 1).expect("valid end");
        assert_eq!(
            Span::from_positions(start, end),
            Err(SourceError::DifferentSources)
        );
    }

    #[test]
    fn joins_only_spans_from_the_supplied_source() {
        let source = "first second third";
        let first = Span::new(source, 0, 5).expect("valid first span");
        let third = Span::new(source, 13, 18).expect("valid third span");
        let covered = first.cover(source, third).expect("compatible spans");

        assert_eq!(covered.slice(source), Ok(source));
    }

    #[test]
    fn constructs_from_prevalidated_positions() {
        let source = "one\ntwo";
        let start = SourcePosition::new(source, 4).expect("valid start");
        let end = SourcePosition::new(source, 7).expect("valid end");
        let span = Span::from_positions(start, end).expect("ordered positions");

        assert_eq!(span.slice(source), Ok("two"));
    }
}
