//! Incremental Callgrind decoding and an owned, interned semantic model.
//!
//! Use [`parse_reader`] for files, [`parse_profile`] for strings, or [`Decoder`]
//! to consume records without retaining a complete profile. All paths share
//! the same grammar and state machine. See NOTES.md for scope and dialects.

mod decoder;
mod error;
mod lex;
mod model;

pub use decoder::{Decoder, parse_profile, parse_reader};
pub use error::{ParseError, ParseErrorKind};
pub use model::*;

/// Legacy scaffold field scanner, not validation. Applications should use
/// parse_reader instead. Retained only while the frontend stubs use it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileHeader {
    pub version: Option<String>,
    pub events: Vec<String>,
}

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
    use super::*;

    #[test]
    fn legacy_header_scanner() {
        assert_eq!(
            parse_header("version: 1\nevents:\t Ir Dr\n"),
            ProfileHeader {
                version: Some("1".into()),
                events: vec!["Ir".into(), "Dr".into()],
            }
        );
        assert_eq!(parse_header("events: Ir\n").version, None);
    }
}
