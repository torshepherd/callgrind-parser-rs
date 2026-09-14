use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseErrorKind {
    MissingEvents,
    DuplicateEvents,
    UnsupportedVersion,
    InvalidNumber,
    NumberOverflow,
    PositionUnderflow,
    UnknownNameId,
    InvalidName,
    InvalidCostWidth,
    InvalidPositionWidth,
    MissingAssociationCost,
    InvalidPositionOrder,
    InvalidEventDefinition,
    InvalidJumpCounts,
    MissingCalledFunction,
    UnknownBodyLine,
    UnsupportedExtension,
    InvalidHeader,
    UnexpectedRecord,
    ResourceLimit,
    InvalidUtf8,
    Io,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    /// One-based input line. EOF association failures identify the opener.
    pub line: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Callgrind parse error on line {}: {:?}",
            self.line, self.kind
        )
    }
}

impl std::error::Error for ParseError {}
