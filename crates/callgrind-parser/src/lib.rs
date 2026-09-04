//! Parsing primitives for Callgrind profile data.
//!
//! [`parse_header`] is the small piece implemented by the initial scaffold.
//! [`parse_profile`] and the normalized model define the contract exercised by
//! the ignored, test-first conformance suite.

/// A parsed Callgrind profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    /// Whether the optional `# callgrind format` marker was present.
    pub has_format_marker: bool,
    /// Format version, defaulting to version 1 when omitted.
    pub version: u64,
    /// Optional producer description from `creator:`.
    pub creator: Option<String>,
    /// Independently headed profile parts.
    pub parts: Vec<Part>,
}

/// One headed part of a profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    /// Process, thread, command, and descriptive metadata.
    pub metadata: PartMetadata,
    /// Ordered subposition columns used by cost and association rows.
    pub positions: Vec<PositionKind>,
    /// Ordered raw event names used by cost columns.
    pub events: Vec<String>,
    /// Optional long-name and inherited-event declarations.
    pub event_definitions: Vec<EventDefinition>,
    /// Optional profile-wide cost summary for this part.
    pub summary: Option<Vec<u64>>,
    /// Parsed body records in source order.
    pub records: Vec<Record>,
    /// Optional final consistency totals.
    pub totals: Option<Vec<u64>>,
}

/// Metadata declared in a part header.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PartMetadata {
    pub pid: Option<u64>,
    pub thread: Option<u64>,
    pub part: Option<u64>,
    pub command: Option<String>,
    pub descriptions: Vec<Description>,
}

/// A `desc: kind: value` header entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Description {
    pub kind: String,
    pub value: String,
}

/// The meaning of one subposition column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionKind {
    Instruction,
    BasicBlock,
    Line,
}

/// Optional metadata declared by an `event:` header line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventDefinition {
    pub name: String,
    /// The inherited expression without surrounding whitespace.
    pub formula: Option<String>,
    pub long_name: Option<String>,
}

/// Resolved object, source file, and function context for a body record.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Context {
    pub object: Option<String>,
    pub file: Option<String>,
    pub function: Option<String>,
}

/// A cost or association record from a profile body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Record {
    Cost(CostRecord),
    Call(CallRecord),
    Jump(JumpRecord),
}

/// Exclusive costs at one resolved source position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostRecord {
    pub context: Context,
    pub positions: Vec<u64>,
    /// One value per part event, with omitted trailing values filled by zero.
    pub costs: Vec<u64>,
}

/// Inclusive costs for one call association.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallRecord {
    pub caller: Context,
    pub callee: Context,
    pub count: u64,
    pub target_positions: Vec<u64>,
    pub source_positions: Vec<u64>,
    /// One value per part event, with omitted trailing values filled by zero.
    pub costs: Vec<u64>,
}

/// An unconditional or conditional jump association.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JumpRecord {
    pub context: Context,
    pub executed: u64,
    /// Taken count for `jcnd`; absent for an unconditional `jump`.
    pub taken: Option<u64>,
    pub target_positions: Vec<u64>,
}

/// A stable high-level category for invalid profile input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseErrorKind {
    MissingEvents,
    UnsupportedVersion,
    InvalidNumber,
    NumberOverflow,
    UnknownNameId,
    InvalidCostWidth,
    MissingAssociationCost,
    InvalidPositionOrder,
}

/// A parse failure with a one-based source line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub line: usize,
}

/// Parse an entire Callgrind profile into a normalized model.
///
/// This is intentionally left as the implementation seam for the conformance
/// tests in `tests/conformance.rs`.
pub fn parse_profile(_input: &str) -> Result<Profile, ParseError> {
    todo!("implement the grammar slices specified by the conformance tests")
}

/// The recognized portion of a Callgrind profile header.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileHeader {
    /// Callgrind format version, when the profile declares one.
    pub version: Option<String>,
    /// Event names, in the same order as the counters in each cost line.
    pub events: Vec<String>,
}

/// Parse the two header fields supported by the initial scaffold.
#[must_use]
pub fn parse_header(input: &str) -> ProfileHeader {
    let mut header = ProfileHeader::default();

    for line in input.lines() {
        if let Some(value) = line.strip_prefix("version:") {
            header.version = Some(value.trim().to_owned());
        } else if let Some(value) = line.strip_prefix("events:") {
            header.events = value.split_whitespace().map(ToOwned::to_owned).collect();
        }
    }

    header
}

#[cfg(test)]
mod tests {
    use super::{ProfileHeader, parse_header};

    #[test]
    fn parses_version_and_ordered_events() {
        let profile = "# callgrind format\nversion: 1\nevents: Ir Dr Dw\nsummary: 42 3 2\n";

        assert_eq!(
            parse_header(profile),
            ProfileHeader {
                version: Some("1".to_owned()),
                events: vec!["Ir".to_owned(), "Dr".to_owned(), "Dw".to_owned()],
            }
        );
    }

    #[test]
    fn accepts_an_omitted_version_and_header_whitespace() {
        let profile = "# callgrind format\nevents:\t Ir   Dr\tDw \n";

        assert_eq!(
            parse_header(profile),
            ProfileHeader {
                version: None,
                events: vec!["Ir".to_owned(), "Dr".to_owned(), "Dw".to_owned()],
            }
        );
    }
}
