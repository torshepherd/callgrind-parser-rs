//! Parsing primitives for Callgrind profile data.
//!
//! The complete parser is still to be implemented. This initial API parses the
//! two header fields needed by the workspace smoke test and gives downstream
//! crates a real dependency to build against.

/// The recognized portion of a Callgrind profile header.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileHeader {
    /// Callgrind format version, when the profile declares one.
    pub version: Option<String>,
    /// Event names, in the same order as the counters in each cost line.
    pub events: Vec<String>,
}

/// Parse the recognized header fields from a Callgrind profile.
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
}
