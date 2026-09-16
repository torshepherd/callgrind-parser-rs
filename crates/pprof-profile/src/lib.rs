//! Shared pprof schema and bounded, validated protobuf/gzip I/O. No protoc.
pub mod proto;

use flate2::{Compression, GzBuilder, bufread::GzDecoder};
use prost::Message;
use std::{
    collections::BTreeSet,
    fmt,
    io::{self, Cursor, Read, Write},
};

pub const DEFAULT_MAX_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug)]
pub struct Error(pub String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self(e.to_string())
    }
}

/// Resolve an index without silently accepting negative/out-of-range strings.
pub fn string(profile: &proto::Profile, index: i64) -> Result<&str, Error> {
    usize::try_from(index)
        .ok()
        .and_then(|i| profile.string_table.get(i))
        .map(String::as_str)
        .ok_or_else(|| Error(format!("invalid string-table index {index}")))
}

fn ids(values: impl Iterator<Item = u64>, kind: &str) -> Result<BTreeSet<u64>, Error> {
    let mut out = BTreeSet::new();
    for id in values {
        if id == 0 || !out.insert(id) {
            return Err(Error(format!("zero or duplicate {kind} ID {id}")));
        }
    }
    Ok(out)
}
fn reference(values: &BTreeSet<u64>, id: u64, kind: &str) -> Result<(), Error> {
    if !values.contains(&id) {
        return Err(Error(format!("unknown {kind} ID {id}")));
    }
    Ok(())
}

/// Structural validation. Signed sample values are valid pprof; consumers set
/// their own policy for negative values and unknown/negative source positions.
pub fn validate(p: &proto::Profile) -> Result<(), Error> {
    if p.string_table.first().map(String::as_str) != Some("") {
        return Err(Error("string_table[0] must be empty".into()));
    }
    if p.sample_type.is_empty() {
        return Err(Error("profile has no sample types".into()));
    }
    for t in p.sample_type.iter().chain(p.period_type.iter()) {
        string(p, t.r#type)?;
        string(p, t.unit)?;
    }
    for i in [
        p.drop_frames,
        p.keep_frames,
        p.default_sample_type,
        p.doc_url,
    ]
    .into_iter()
    .chain(p.comment.iter().copied())
    {
        string(p, i)?;
    }
    let mappings = ids(p.mapping.iter().map(|m| m.id), "mapping")?;
    let functions = ids(p.function.iter().map(|f| f.id), "function")?;
    let locations = ids(p.location.iter().map(|l| l.id), "location")?;
    for m in &p.mapping {
        string(p, m.filename)?;
        string(p, m.build_id)?;
    }
    for f in &p.function {
        string(p, f.name)?;
        string(p, f.system_name)?;
        string(p, f.filename)?;
    }
    for l in &p.location {
        if l.mapping_id != 0 {
            reference(&mappings, l.mapping_id, "mapping")?;
        }
        for line in &l.line {
            reference(&functions, line.function_id, "function")?;
        }
    }
    for (i, s) in p.sample.iter().enumerate() {
        if s.value.len() != p.sample_type.len() {
            return Err(Error(format!(
                "sample {i}: value width differs from sample types"
            )));
        }
        for id in &s.location_id {
            reference(&locations, *id, "location")?;
        }
        for label in &s.label {
            string(p, label.key)?;
            string(p, label.str)?;
            string(p, label.num_unit)?;
            if label.str != 0 && (label.num != 0 || label.num_unit != 0) {
                return Err(Error(format!(
                    "sample {i}: label has both string and numeric values"
                )));
            }
        }
    }
    Ok(())
}

fn bounded_read(reader: impl Read, limit: u64) -> Result<Vec<u8>, Error> {
    let bound = limit
        .checked_add(1)
        .filter(|_| limit > 0)
        .ok_or_else(|| Error("invalid byte limit".into()))?;
    let mut bytes = Vec::new();
    reader.take(bound).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(Error(format!("input exceeds {limit}-byte limit")));
    }
    Ok(bytes)
}

/// Accept one gzip member or raw protobuf. Both compressed and uncompressed
/// byte counts are bounded. CRC/truncation, trailing gzip data and malformed
/// protobuf are errors; legacy text formats are deliberately unsupported.
pub fn read(reader: impl Read, max_bytes: u64) -> Result<proto::Profile, Error> {
    let bytes = bounded_read(reader, max_bytes)?;
    let decoded = if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut gzip = GzDecoder::new(Cursor::new(&bytes));
        let out = bounded_read(&mut gzip, max_bytes)?;
        if gzip.into_inner().position() != bytes.len() as u64 {
            return Err(Error("trailing or concatenated gzip data".into()));
        }
        out
    } else {
        bytes
    };
    let profile = proto::Profile::decode(decoded.as_slice())
        .map_err(|e| Error(format!("invalid pprof protobuf: {e}")))?;
    validate(&profile)?;
    Ok(profile)
}

/// Deterministic gzip header; explicit finalization propagates output failures.
pub fn write(profile: &proto::Profile, writer: impl Write) -> Result<(), Error> {
    validate(profile)?;
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(writer, Compression::default());
    encoder.write_all(&profile.encode_to_vec())?;
    encoder.finish()?;
    Ok(())
}
